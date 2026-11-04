//! Standalone text-ingestion bridge for a dedicated test state.
//! Never initializes or updates Lucy/Academy production states implicitly.
use serde::Deserialize;
use std::{env, fs, io, path::PathBuf};
use transformer_memory_engine::{
    state_container,
    text_ingestion::{TextDocument, TextSegment},
    Memory,
    acquisition::{AcquisitionBatch, AcquisitionFeedback},
    external_acquisition::ExternalSourceKind,
    text_abstraction::{TextAbstractionRequest,TextAbstractionCommit},
    maintenance::{MaintenanceMode,MaintenanceSignal},
    lifecycle::TmePhase,
};

#[derive(Deserialize)]
struct Segment {
    id: String,
    text: String,
    vector: Vec<f32>,
}
#[derive(Deserialize)]
struct Input {
    anchor_id: String,
    title: String,
    anchor_vector: Vec<f32>,
    source: String,
    segments: Vec<Segment>,
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        return Err("Usage: text-ingestion-bridge <create|ingest|recall|search|acquire|session-start|session-feedback|session-hint|external-handoff|abstraction-handoffs|abstraction-preview|abstraction-commit|maintenance-assess|reflection-pass|lifecycle-status|maintenance-request|standby|sleep-begin|sleep-pass|wake|workflow-feedback> <state-dir> <state-uuid> <entity-serial> <json-file-or-anchor>".into());
    }
    let command = &args[1];
    let dir = PathBuf::from(&args[2]);
    let id = &args[3];
    let serial = &args[4];
    let file = dir.join(format!("{id}.tme"));
    match command.as_str() {
        "create" | "ingest" => {
            let bytes = fs::read(&args[5])?;
            let input: Input = serde_json::from_slice(&bytes)?;
            let document = TextDocument {
                anchor_id: input.anchor_id,
                title: input.title,
                anchor_vector: input.anchor_vector,
                source: input.source,
                segments: input
                    .segments
                    .into_iter()
                    .map(|s| TextSegment {
                        id: s.id,
                        text: s.text,
                        vector: s.vector,
                    })
                    .collect(),
            };
            if command == "create" {
                fs::create_dir_all(&dir)?;
                let mut memory = Memory::new();
                let report = memory.ingest_text(&document)?;
                state_container::create(&file, id, serial, &memory)?;
                println!(
                    "CREATED inserted={} reused={}",
                    report.inserted.len(),
                    report.reused.len()
                );
            } else {
                state_container::update(&file, id, serial, |memory| {
                    let report = memory.ingest_text(&document).map_err(io::Error::other)?;
                    println!(
                        "UPDATED inserted={} reused={}",
                        report.inserted.len(),
                        report.reused.len()
                    );
                    Ok(())
                })?;
            }
        }
        "search" => {
            // Search payload: {"anchor_id":str,"query_vector":[f32],"max_depth":usize,"limit":usize}.
            // This branch is read-only: no state_container::update and no writes.
            #[derive(Deserialize)]
            struct SearchInput {
                anchor_id: String,
                query_vector: Vec<f32>,
                max_depth: usize,
                limit: usize,
            }
            let query: SearchInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            if query.max_depth == 0 || query.max_depth > 8 || query.limit == 0 || query.limit > 20 {
                return Err("Invalid search limits".into());
            }
            let memory = state_container::load(&file, id, serial)?;
            if memory.is_anchor(&query.anchor_id) != Some(true) {
                return Err("Unknown text anchor".into());
            }
            let view = memory.learning_b_semantic_preview(
                &query.anchor_id,
                &query.query_vector,
                query.max_depth,
                query.limit,
            )?;
            let results: Vec<_> = view
                .candidates
                .into_iter()
                .filter_map(|candidate| {
                    let entry = memory.text_memories.get(&candidate.token_id)?;
                    // No cross-document material in this initial demo.
                    if entry.anchor_id != query.anchor_id || entry.position == 0 {
                        return None;
                    }
                    Some(serde_json::json!({
                        "token_id": candidate.token_id,
                        "position": entry.position,
                        "text": entry.text,
                        "source": entry.source,
                        "similarity": candidate.similarity,
                        "path_weight": candidate.path_weight,
                        "combined_score": candidate.combined_score,
                    }))
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "anchor_id": query.anchor_id,
                    "truncated": view.truncated,
                    "candidates_considered": view.candidates_considered,
                    "results": results,
                }))?
            );
        }
        "acquire" => {
            // A read-only feedback round. The agent retains delivered token IDs.
            #[derive(Deserialize)]
            struct AcquireInput {
                anchor_id: String,
                query_vector: Vec<f32>,
                feedback: AcquisitionFeedback,
                #[serde(default)]
                seen_token_ids: Vec<String>,
                max_depth: usize,
                batch_size: usize,
            }
            let request: AcquireInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            if request.max_depth == 0 || request.max_depth > 8
                || request.batch_size == 0 || request.batch_size > 20
            {
                return Err("Invalid acquisition limits".into());
            }
            let memory = state_container::load(&file, id, serial)?;
            if memory.is_anchor(&request.anchor_id) != Some(true) {
                return Err("Unknown text anchor".into());
            }
            let seen = request.seen_token_ids.into_iter().collect();
            let batch = memory.acquisition_preview(
                &request.anchor_id, &request.query_vector, request.feedback,
                &seen, request.max_depth, request.batch_size,
            )?;
            let results: Vec<_> = batch.results.iter().filter_map(|candidate| {
                let entry = memory.text_memories.get(&candidate.token_id)?;
                if entry.anchor_id != request.anchor_id || entry.position == 0 {
                    return None;
                }
                Some(serde_json::json!({
                    "token_id": candidate.token_id,
                    "depth": candidate.depth,
                    "tier": format!("{:?}", candidate.tier),
                    "text": entry.text,
                    "source": entry.source,
                    "similarity": candidate.similarity,
                    "combined_score": candidate.combined_score,
                }))
            }).collect();
            println!("{}", serde_json::to_string(&serde_json::json!({
                "anchor_id": batch.anchor,
                "depth_reached": batch.depth_reached,
                "total_reachable_candidates": batch.total_reachable_candidates,
                "unseen_reachable_candidates": batch.unseen_reachable_candidates,
                "unseen_within_depth": batch.unseen_within_depth,
                "disposition": format!("{:?}", batch.disposition),
                "results": results,
            }))?);
        }
        "abstraction-handoffs" => {
            // Candidate discovery is strictly read-only. Neither summaries
            // nor token deletions are committed by this endpoint.
            #[derive(Deserialize)]
            struct HandoffInput {
                mode: MaintenanceMode,
                signal: MaintenanceSignal,
                budget: usize,
            }
            let input: HandoffInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            let memory = state_container::load(&file, id, serial)?;
            let expected = match input.mode {
                MaintenanceMode::Reflection => TmePhase::Active,
                MaintenanceMode::Sleep => TmePhase::Sleep,
            };
            if memory.lifecycle.phase != expected {
                return Err("Abstraction handoff mode does not match TME phase".into());
            }
            let handoffs = memory.workflow_abstraction_handoffs(
                input.mode, input.signal, input.budget,
            )?;
            println!("{}", serde_json::to_string(&handoffs)?);
        }
        "abstraction-preview" => {
            // Read-only: a proposed summary is NOT considered semantically valid.
            let proposal: TextAbstractionRequest =
                serde_json::from_slice(&fs::read(&args[5])?)?;
            let memory = state_container::load(&file, id, serial)?;
            let preview = memory.text_abstraction_preview(&proposal)?;
            println!("{}", serde_json::to_string(&preview)?);
        }
        "abstraction-commit" => {
            // This is a caller-reviewed action, not a semantic truth oracle.
            // A review_reference is audit metadata, not proof of verification.
            #[derive(Deserialize)]
            struct AbstractionInput {
                request: TextAbstractionRequest,
                commit: TextAbstractionCommit,
            }
            let input: AbstractionInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            let mut result = None;
            state_container::update(&file, id, serial, |memory| {
                if memory.lifecycle.phase != TmePhase::Active {
                    return Err(io::Error::other("Abstraction commit requires ACTIVE phase"));
                }
                result = Some(memory.text_abstraction_commit(&input.request, &input.commit)
                    .map_err(io::Error::other)?);
                Ok(())
            })?;
            println!("{}", serde_json::to_string(&result.ok_or("Missing abstraction commit result")?)?);
        }
        "maintenance-assess" => {
            // Both Reflection and SLEEP share the same state-analysis function.
            #[derive(Deserialize)]
            struct AssessInput {
                mode: MaintenanceMode,
                signal: MaintenanceSignal,
                budget: usize,
            }
            let input: AssessInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            let memory = state_container::load(&file, id, serial)?;
            let expected = match input.mode {
                MaintenanceMode::Reflection => TmePhase::Active,
                MaintenanceMode::Sleep => TmePhase::Sleep,
            };
            if memory.lifecycle.phase != expected {
                return Err("Assessment mode does not match TME lifecycle phase".into());
            }
            let assessment = memory.maintenance_assess(input.mode, input.signal, input.budget)?;
            println!("{}", serde_json::to_string(&assessment)?);
        }
        "reflection-pass" => {
            // Eindeutige, bereits beobachtete Operationen werden quittiert.
            // Keine semantische Bewertung, kein Löschen von Text oder Evidenz.
            #[derive(Deserialize)]
            struct ReflectInput { budget: usize }
            let input: ReflectInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            let mut result = None;
            state_container::update(&file, id, serial, |memory| {
                if memory.lifecycle.phase != TmePhase::Active {
                    return Err(io::Error::other("Reflection pass requires ACTIVE phase"));
                }
                result = Some(memory.maintenance_review_operations(
                    MaintenanceMode::Reflection, input.budget,
                ).map_err(io::Error::other)?);
                Ok(())
            })?;
            println!("{}", serde_json::to_string(&result.ok_or("Missing reflection report")?)?);
        }
        "lifecycle-status" => {
            // Read-only lifecycle inspection. Last positional argument is unused.
            let memory = state_container::load(&file, id, serial)?;
            println!("{}", serde_json::to_string(&memory.lifecycle)?);
        }
        "maintenance-request" => {
            // Signal maintenance need without entering standby or sleep.
            state_container::update(&file, id, serial, |memory| {
                memory.lifecycle_request_maintenance();
                Ok(())
            })?;
            let memory = state_container::load(&file, id, serial)?;
            println!("{}", serde_json::to_string(&memory.lifecycle)?);
        }
        "standby" => {
            #[derive(Deserialize)]
            struct StandbyInput { active_tasks: usize }
            let input: StandbyInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            state_container::update(&file, id, serial, |memory| {
                memory.lifecycle_enter_standby(input.active_tasks)
                    .map_err(io::Error::other)
            })?;
            let memory = state_container::load(&file, id, serial)?;
            println!("{}", serde_json::to_string(&memory.lifecycle)?);
        }
        "sleep-begin" => {
            // Soul owns LLM management; this is an asserted observation, NOT
            // a process shutdown. Soul must guarantee llm_inactive is accurate.
            #[derive(Deserialize)]
            struct SleepBeginInput { llm_inactive: bool }
            let input: SleepBeginInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            state_container::update(&file, id, serial, |memory| {
                memory.lifecycle_begin_sleep(input.llm_inactive)
                    .map_err(io::Error::other)
            })?;
            let memory = state_container::load(&file, id, serial)?;
            println!("{}", serde_json::to_string(&memory.lifecycle)?);
        }
        "sleep-pass" => {
            #[derive(Deserialize)]
            struct SleepPassInput { budget: usize }
            let input: SleepPassInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            let mut report = None;
            state_container::update(&file, id, serial, |memory| {
                report = Some(memory.workflow_sleep_pass(input.budget)
                    .map_err(io::Error::other)?);
                Ok(())
            })?;
            let result = report.ok_or("Missing committed sleep report")?;
            println!("{}", serde_json::to_string(&serde_json::json!({
                "sleep_pass_number": result.sleep_pass_number,
                "review_still_pending": result.review_still_pending,
                "acknowledged": result.maintenance.acknowledged,
                "outstanding_findings": result.maintenance.outstanding_findings,
                "additional_review_required": result.maintenance.additional_review_required,
                "phase": "standby",
            }))?);
        }
        "wake" => {
            // Waking an already ACTIVE state is harmless; cannot commit a
            // half-finished pass because it only changes the lifecycle phase.
            state_container::update(&file, id, serial, |memory| {
                memory.lifecycle_wake();
                Ok(())
            })?;
            let memory = state_container::load(&file, id, serial)?;
            println!("{}", serde_json::to_string(&memory.lifecycle)?);
        }
        "workflow-feedback" => {
            #[derive(Deserialize)]
            struct FeedbackInput {
                session_id: String,
                feedback: AcquisitionFeedback,
                signal: MaintenanceSignal,
                assessment_budget: usize,
            }
            let input: FeedbackInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            let mut result = None;
            state_container::update(&file, id, serial, |memory| {
                result = Some(memory.workflow_feedback(
                    &input.session_id, input.feedback, input.signal, input.assessment_budget,
                ).map_err(io::Error::other)?);
                Ok(())
            })?;
            let result = result.ok_or("Missing committed feedback result")?;
            let memory = state_container::load(&file, id, serial)?;
            let results: Vec<_> = result.acquisition.results.iter().map(|candidate| {
                let entry = memory.text_memories.get(&candidate.token_id);
                serde_json::json!({
                    "token_id": candidate.token_id,
                    "tier": format!("{:?}", candidate.tier),
                    "depth": candidate.depth,
                    "text": entry.map(|v|v.text.as_str()),
                    "source": entry.map(|v|v.source.as_str()),
                    "combined_score": candidate.combined_score,
                })
            }).collect();
            println!("{}",serde_json::to_string(&serde_json::json!({
                "session_id": input.session_id,
                "disposition": format!("{:?}",result.acquisition.disposition),
                "external_lookup_candidate": result.external_lookup_candidate,
                "depth_reached": result.acquisition.depth_reached,
                "results": results,
                "duplicate_candidates": result.duplicate_candidates,
                "maintenance": result.assessment,
                "maintenance_pending": memory.lifecycle.pending_maintenance,
            }))?);
        }
        "external-handoff" => {
            #[derive(Deserialize)]
            struct HandoffInput {
                session_id: String,
                allowed_source_kinds: Vec<ExternalSourceKind>,
            }
            let input: HandoffInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            let memory = state_container::load(&file, id, serial)?;
            let handoff=memory.acquisition_external_handoff(
                &input.session_id, &input.allowed_source_kinds,
            )?;
            println!("{}", serde_json::to_string(&handoff)?);
        }
        "session-start" | "session-feedback" | "session-hint" => {
            #[derive(Deserialize)]
            struct SessionInput {
                session_id: String,
                anchor_id: Option<String>,
                query_vector: Option<Vec<f32>>,
                feedback: Option<AcquisitionFeedback>,
                max_depth: Option<usize>,
                batch_size: Option<usize>,
                source_reference: Option<String>,
                reason: Option<String>,
            }
            let input: SessionInput = serde_json::from_slice(&fs::read(&args[5])?)?;
            let mut output: Option<AcquisitionBatch> = None;
            state_container::update(&file, id, serial, |memory| {
                let result = match command.as_str() {
                    "session-start" => memory.acquisition_session_start(
                        &input.session_id,
                        input.anchor_id.as_deref().ok_or_else(|| io::Error::other("Missing anchor_id"))?,
                        input.query_vector.clone().ok_or_else(|| io::Error::other("Missing query_vector"))?,
                        input.max_depth.ok_or_else(|| io::Error::other("Missing max_depth"))?,
                        input.batch_size.ok_or_else(|| io::Error::other("Missing batch_size"))?,
                    ),
                    "session-feedback" => memory.acquisition_session_feedback(
                        &input.session_id,
                        input.feedback.ok_or_else(|| io::Error::other("Missing feedback"))?,
                    ),
                    "session-hint" => memory.acquisition_session_reactivate(
                        &input.session_id,
                        input.query_vector.clone().ok_or_else(|| io::Error::other("Missing query_vector"))?,
                        input.source_reference.as_deref().ok_or_else(|| io::Error::other("Missing source_reference"))?,
                        input.reason.as_deref().ok_or_else(|| io::Error::other("Missing reason"))?,
                    ),
                    _ => unreachable!(),
                }.map_err(io::Error::other)?;
                output=Some(result);
                Ok(())
            })?;
            // Emit only after the validated atomic state update succeeded.
            let batch = output.ok_or("Missing committed acquisition batch")?;
            let memory = state_container::load(&file, id, serial)?;
            let results: Vec<_> = batch.results.iter().map(|candidate| {
                let entry = memory.text_memories.get(&candidate.token_id);
                serde_json::json!({
                    "token_id": candidate.token_id,
                    "tier": format!("{:?}", candidate.tier),
                    "depth": candidate.depth,
                    "text": entry.map(|v|v.text.as_str()),
                    "source": entry.map(|v|v.source.as_str()),
                    "combined_score": candidate.combined_score,
                })
            }).collect();
            println!("{}",serde_json::to_string(&serde_json::json!({
                "session_id":input.session_id,
                "disposition":format!("{:?}",batch.disposition),
                "depth_reached":batch.depth_reached,
                "unseen_reachable_candidates":batch.unseen_reachable_candidates,
                "results":results,
            }))?);
        }
        "recall" => {
            let memory = state_container::load(&file, id, serial)?;
            let entries = memory.recall_text_segments(&args[5]);
            if entries.is_empty() {
                return Err("No text segments for anchor".into());
            }
            for entry in entries {
                println!("{}\t{}", entry.position, entry.text);
            }
        }
        _ => return Err("Unknown command".into()),
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("TEXT BRIDGE ERROR: {error}");
        std::process::exit(1);
    }
}
