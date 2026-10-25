use std::collections::BTreeMap;
use transformer_memory_engine::{
    contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
    experience::EvidenceSource,
    Memory, StateToken,
};

#[derive(Clone)]
enum Event {
    Learn(&'static str, [f32; 2]),
    Link(&'static str, &'static str),
    Activation(&'static str, f32),
    Dynamics(&'static str, f32, f32),
    Review(&'static str),
    Use(&'static str),
    Evidence(&'static str, &'static str, bool),
    ContextEvidence(&'static str, &'static str, bool, &'static str),
    ReflectProbe(&'static str, &'static str, &'static str),
    Probe(&'static str, &'static str),
}

#[derive(Clone)]
struct TimedEvent {
    day: u32,
    action: Event,
}

#[derive(Debug, PartialEq, Eq)]
struct Probe {
    day: u32,
    anchor: String,
    expected: String,
    semantic: String,
    contextual: Option<String>,
    evidence_count: usize,
    correct: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct LifecycleReflectProbe {
    day: u32,
    total_records: usize,
    mixed_context_values: Vec<String>,
    review_required: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct Report {
    events: usize,
    recorded_uses: u64,
    probes: Vec<Probe>,
    reflect_probes: Vec<LifecycleReflectProbe>,
}

fn scenario() -> Vec<TimedEvent> {
    use Event::*;

    vec![
        TimedEvent {
            day: 0,
            action: Learn("client_a", [0.0, 1.0]),
        },
        TimedEvent {
            day: 0,
            action: Learn("client_b", [0.0, 1.0]),
        },
        TimedEvent {
            day: 0,
            action: Learn("record_a", [1.0, 0.0]),
        },
        TimedEvent {
            day: 0,
            action: Learn("record_b", [0.6, 0.8]),
        },
        TimedEvent {
            day: 1,
            action: Link("client_a", "record_a"),
        },
        TimedEvent {
            day: 1,
            action: Link("client_b", "record_b"),
        },
        TimedEvent {
            day: 2,
            action: Activation("record_a", 0.05),
        },
        TimedEvent {
            day: 2,
            action: Activation("record_b", 1.0),
        },
        TimedEvent {
            day: 3,
            action: Evidence("client_a", "record_a", true),
        },
        TimedEvent {
            day: 3,
            action: Evidence("client_b", "record_b", true),
        },
        TimedEvent {
            day: 30,
            action: Use("record_b"),
        },
        TimedEvent {
            day: 100,
            action: Probe("client_a", "record_a"),
        },
        TimedEvent {
            day: 365,
            action: Probe("client_b", "record_b"),
        },
    ]
}

fn relation_exists(memory: &Memory, from: &str, to: &str) -> Result<bool, String> {
    let key = serde_json::to_string(&(from, to)).map_err(|e| e.to_string())?;
    Ok(memory.relations.get(&key).is_some_and(|r| r.weight > 0.0))
}

fn replay(events: &[TimedEvent]) -> Result<Report, String> {
    replay_with_mode(events, false).map(|(report, _)| report)
}

fn replay_with_mode(events: &[TimedEvent], adaptive: bool) -> Result<(Report, Memory), String> {
    use transformer_memory_engine::adaptive_dynamics::{AdaptiveEvent, AdaptivePolicy};

    let policy = AdaptivePolicy {
        use_gain: 0.5,
        review_decay: 0.5,
    };

    let mut memory = Memory::new();
    let mut last_day = None;
    let mut recorded_uses = 0;
    let mut probes = Vec::new();
    let mut reflect_probes = Vec::new();

    for (index, timed) in events.iter().enumerate() {
        if last_day.is_some_and(|day| timed.day < day) {
            return Err("Events are not chronologically ordered".into());
        }
        last_day = Some(timed.day);

        match &timed.action {
            Event::Learn(id, vector) => {
                memory.insert(StateToken {
                    id: (*id).into(),
                    vector: vector.to_vec(),
                })?;
            }
            Event::Link(from, to) => {
                memory.reinforce(from, to, 1.0)?;
            }
            Event::Activation(id, activation) => {
                let stability = memory.dynamics(id).ok_or("Unknown token")?.stability;
                memory.set_token_dynamics(id, *activation, stability)?;
            }
            Event::Dynamics(id, activation, stability) => {
                memory.set_token_dynamics(id, *activation, *stability)?;
            }
            Event::Review(id) => {
                if adaptive {
                    let preview = memory.adaptive_dynamics_preview(
                        id,
                        AdaptiveEvent::ConsolidationReview,
                        policy,
                    )?;
                    memory.set_token_dynamics(
                        id,
                        preview.activation_after,
                        preview.retention_stability,
                    )?;
                } else if memory.dynamics(id).is_none() {
                    return Err("Unknown review token".into());
                }
            }
            Event::Use(id) => {
                memory.record_token_operation(id)?;
                recorded_uses += 1;

                if adaptive {
                    let preview =
                        memory.adaptive_dynamics_preview(id, AdaptiveEvent::Use, policy)?;
                    memory.set_token_dynamics(
                        id,
                        preview.activation_after,
                        preview.retention_stability,
                    )?;
                }
            }
            Event::Evidence(from, to, occurred) => {
                memory.record_contextual_evidence(ContextualEvidenceRecord {
                    schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                    id: format!("evidence-{index}"),
                    from: (*from).into(),
                    to: (*to).into(),
                    occurred: *occurred,
                    observed_at_unix_ms: i64::from(timed.day) * 86_400_000,
                    available_at_unix_ms: i64::from(timed.day) * 86_400_000,
                    source: EvidenceSource::ExternalObservation,
                    reference: format!("synthetic/{index}"),
                    context: BTreeMap::new(),
                })?;
            }
            Event::ContextEvidence(from, to, occurred, mode) => {
                let timestamp = i64::from(timed.day) * 86_400_000;
                memory.record_contextual_evidence(ContextualEvidenceRecord {
                    schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                    id: format!("context-evidence-{index}"),
                    from: (*from).into(),
                    to: (*to).into(),
                    occurred: *occurred,
                    observed_at_unix_ms: timestamp,
                    available_at_unix_ms: timestamp,
                    source: EvidenceSource::ExternalObservation,
                    reference: format!("synthetic/context/{index}"),
                    context: BTreeMap::from([("mode".into(), (*mode).into())]),
                })?;
            }
            Event::ReflectProbe(from, to, context_key) => {
                let cutoff = i64::from(timed.day) * 86_400_000;
                let report = memory.reflect_contextual_as_of(from, to, context_key, cutoff)?;

                reflect_probes.push(LifecycleReflectProbe {
                    day: timed.day,
                    total_records: report.evidence_quality.total_records,
                    mixed_context_values: report.mixed_context_values,
                    review_required: report.review_required,
                });
            }
            Event::Probe(anchor, expected) => {
                let query = [1.0, 0.0];
                let preview = memory.retrieval_priority_preview(&query, 4)?;
                let records: Vec<_> = preview
                    .candidates
                    .iter()
                    .filter(|c| c.token_id.starts_with("record_"))
                    .collect();

                let semantic = records
                    .first()
                    .ok_or("No semantic record")?
                    .token_id
                    .clone();

                let mut linked = Vec::new();
                for candidate in records {
                    if relation_exists(&memory, anchor, &candidate.token_id)? {
                        linked.push(candidate.token_id.clone());
                    }
                }

                // Do not guess when zero or multiple records match.
                let contextual = if linked.len() == 1 {
                    linked.into_iter().next()
                } else {
                    None
                };

                let evidence_count = if let Some(id) = &contextual {
                    memory
                        .analyze_evidence_quality_as_of(
                            anchor,
                            id,
                            i64::from(timed.day) * 86_400_000,
                        )?
                        .total_records
                } else {
                    0
                };

                probes.push(Probe {
                    day: timed.day,
                    anchor: (*anchor).into(),
                    expected: (*expected).into(),
                    correct: contextual.as_deref() == Some(*expected),
                    semantic,
                    contextual,
                    evidence_count,
                });
            }
        }
    }

    memory.validate_evidence_graph()?;

    Ok((
        Report {
            events: events.len(),
            recorded_uses,
            probes,
            reflect_probes,
        },
        memory,
    ))
}

fn main() -> Result<(), String> {
    let report = replay(&scenario())?;

    println!("TME Integrated Event Replay v0.6");
    println!("Existing core APIs, synthetic event timeline.");
    println!("No decay, Sleep or calibrated confidence.\n");

    println!("Events: {}", report.events);
    println!("Recorded uses: {}", report.recorded_uses);

    for probe in &report.probes {
        println!("{probe:?}");
    }

    let correct = report.probes.iter().filter(|p| p.correct).count();
    println!("Contextual Top-1: {correct}/{}", report.probes.len());
    println!("This is an integration fixture, not a quality benchmark.");

    println!("\nTME Adaptive Dynamics v0.7");
    for adaptive in [false, true] {
        let (a, b, uses) = adaptive_trace(adaptive)?;
        println!(
            "adaptive={} record_a={:.4} record_b={:.4} uses={}",
            adaptive, a, b, uses
        );
    }
    println!("Explicit simulated updates; no automatic decay or Sleep.");

    println!("\nTME Coupled Retrieval v0.8");
    let (activation_a, activation_b, record_a, record_b) = coupled_retrieval_probe()?;
    println!("client_a={} activation={:.4}", record_a, activation_a);
    println!("client_b={} activation={:.4}", record_b, activation_b);
    println!("Activation is diagnostic; relations select the records.");

    println!("\nTME Unified Event Replay v0.9");
    for adaptive in [false, true] {
        let trace = unified_trace(adaptive)?;
        println!(
            "adaptive={} record_a={:.4} record_b={:.4} uses={} contextual={}/{}",
            adaptive,
            trace.activation_a,
            trace.activation_b,
            trace.recorded_uses,
            trace.contextual_correct,
            trace.probes,
        );
        println!("  client_a={} client_b={}", trace.linked_a, trace.linked_b);
    }
    println!("Same event history; fixed adaptive policy; no automatic Sleep.");

    println!("\nTME Contextual Reflect v1.0");
    for cutoff in [20, 30, 40] {
        let (records, mixed, review) = conflict_reflect_trace(cutoff)?;
        println!(
            "cutoff={} records={} mixed={:?} review_required={}",
            cutoff, records, mixed, review
        );
    }
    println!("Descriptive evidence review; no memory mutation.");

    println!("\nTME Unified Memory Lifecycle v1.1");

    for adaptive in [false, true] {
        let (report, a, b) = lifecycle_trace(adaptive)?;

        println!(
            "adaptive={} record_a={:.4} record_b={:.4} uses={}",
            adaptive, a, b, report.recorded_uses
        );

        for probe in &report.reflect_probes {
            println!(
                "  day={} records={} mixed={:?} review={}",
                probe.day, probe.total_records, probe.mixed_context_values, probe.review_required
            );
        }
    }

    println!("Chronological snapshots at probe execution; no Sleep.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(replay(&scenario()).unwrap(), replay(&scenario()).unwrap());
    }

    #[test]
    fn context_selects_correct_record() {
        let report = replay(&scenario()).unwrap();
        assert_eq!(report.events, 13);
        assert_eq!(report.recorded_uses, 1);
        assert_eq!(report.probes.len(), 2);
        assert!(report.probes.iter().all(|p| p.correct));
        assert!(report.probes.iter().all(|p| p.evidence_count == 1));
    }

    #[test]
    fn semantic_retrieval_alone_misses_customer_b() {
        let report = replay(&scenario()).unwrap();
        assert_eq!(report.probes[0].semantic, "record_a");
        assert_eq!(report.probes[1].semantic, "record_a");
    }

    #[test]
    fn rejects_out_of_order_events() {
        let mut events = scenario();
        events.swap(0, 12);
        assert!(replay(&events).is_err());
    }

    #[test]
    fn missing_link_is_not_reconstructed() {
        let mut events = scenario();
        events.retain(|e| !matches!(&e.action, Event::Link("client_b", "record_b")));
        let report = replay(&events).unwrap();
        assert_eq!(report.probes[1].contextual, None);
        assert!(!report.probes[1].correct);
    }
}

// Experimental extension of the integrated replay.
// Both branches process the same scheduled events.
// The adaptive branch explicitly applies core preview results.
fn adaptive_trace(adaptive: bool) -> Result<(f32, f32, u64), String> {
    use transformer_memory_engine::adaptive_dynamics::{AdaptiveEvent, AdaptivePolicy};

    let policy = AdaptivePolicy {
        use_gain: 0.5,
        review_decay: 0.5,
    };

    let mut memory = Memory::new();

    for (id, activation, stability) in [("record_a", 0.2, 0.8), ("record_b", 0.8, 0.1)] {
        memory.insert(StateToken {
            id: id.into(),
            vector: vec![1.0, 0.0],
        })?;
        memory.set_token_dynamics(id, activation, stability)?;
    }

    let events = [
        (30_u32, "record_a", AdaptiveEvent::Use),
        (100, "record_a", AdaptiveEvent::ConsolidationReview),
        (100, "record_b", AdaptiveEvent::ConsolidationReview),
        (365, "record_a", AdaptiveEvent::Use),
    ];

    let mut uses = 0_u64;

    for (_day, id, event) in events {
        if event == AdaptiveEvent::Use {
            memory.record_token_operation(id)?;
            uses += 1;
        }

        if adaptive {
            let preview = memory.adaptive_dynamics_preview(id, event, policy)?;

            // Simulator-only explicit application.
            // This is NOT an automatic production commit.
            memory.set_token_dynamics(id, preview.activation_after, preview.retention_stability)?;
        }
    }

    let a = memory.dynamics("record_a").ok_or("Missing record_a")?;
    let b = memory.dynamics("record_b").ok_or("Missing record_b")?;

    if a.operations != uses || b.operations != 0 {
        return Err("Operation counts differ from event history".into());
    }

    Ok((a.activation, b.activation, uses))
}

#[cfg(test)]
mod adaptive_replay_tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn baseline_does_not_modify_activation() {
        let (a, b, uses) = adaptive_trace(false).unwrap();
        assert!(close(a, 0.2));
        assert!(close(b, 0.8));
        assert_eq!(uses, 2);
    }

    #[test]
    fn adaptive_policy_changes_activation_as_expected() {
        let (a, b, uses) = adaptive_trace(true).unwrap();
        assert!(close(a, 0.77));
        assert!(close(b, 0.44));
        assert_eq!(uses, 2);
    }

    #[test]
    fn adaptive_trace_is_deterministic() {
        assert_eq!(adaptive_trace(true).unwrap(), adaptive_trace(true).unwrap());
    }

    #[test]
    fn adaptive_and_baseline_use_same_operation_count() {
        assert_eq!(
            adaptive_trace(false).unwrap().2,
            adaptive_trace(true).unwrap().2
        );
    }
}

// v0.8: Dynamics and retrieval share one actual Memory instance.
// Activation is observed, not used as a truth or ranking score.
fn coupled_retrieval_probe() -> Result<(f32, f32, String, String), String> {
    use transformer_memory_engine::{
        adaptive_dynamics::{AdaptiveEvent, AdaptivePolicy},
        processing::RetrievalDecision,
    };

    let mut memory = Memory::new();

    for (id, vector) in [
        ("client_a", vec![0.0, 1.0]),
        ("client_b", vec![0.0, 1.0]),
        ("record_a", vec![1.0, 0.0]),
        ("record_b", vec![0.6, 0.8]),
    ] {
        memory.insert(StateToken {
            id: id.into(),
            vector,
        })?;
    }

    memory.reinforce("client_a", "record_a", 1.0)?;
    memory.reinforce("client_b", "record_b", 1.0)?;

    memory.set_token_dynamics("record_a", 0.2, 0.8)?;
    memory.set_token_dynamics("record_b", 0.8, 0.1)?;

    let policy = AdaptivePolicy {
        use_gain: 0.5,
        review_decay: 0.5,
    };

    // Apply explicit events to the SAME memory used for retrieval.
    for (id, event) in [
        ("record_a", AdaptiveEvent::Use),
        ("record_a", AdaptiveEvent::ConsolidationReview),
        ("record_b", AdaptiveEvent::ConsolidationReview),
        ("record_a", AdaptiveEvent::Use),
    ] {
        if event == AdaptiveEvent::Use {
            memory.record_token_operation(id)?;
        }

        let preview = memory.adaptive_dynamics_preview(id, event, policy)?;
        memory.set_token_dynamics(id, preview.activation_after, preview.retention_stability)?;
    }

    let query = [1.0, 0.0];

    let read = |anchor| -> Result<(String, f32), String> {
        match memory.process_retrieval_linked(&query, anchor, 100)? {
            RetrievalDecision::Found(candidate) => Ok((candidate.token_id, candidate.activation)),
            _ => Err(format!("No unique retrieval for {anchor}")),
        }
    };

    let (id_a, activation_a) = read("client_a")?;
    let (id_b, activation_b) = read("client_b")?;

    Ok((activation_a, activation_b, id_a, id_b))
}

#[cfg(test)]
mod coupled_retrieval_tests {
    use super::*;

    #[test]
    fn adaptive_states_are_visible_during_retrieval() {
        let (a, b, _, _) = coupled_retrieval_probe().unwrap();
        assert!((a - 0.77).abs() < 1e-6);
        assert!((b - 0.44).abs() < 1e-6);
    }

    #[test]
    fn context_association_survives_activation_changes() {
        let (_, _, a, b) = coupled_retrieval_probe().unwrap();
        assert_eq!(a, "record_a");
        assert_eq!(b, "record_b");
    }

    #[test]
    fn combined_processing_is_deterministic() {
        assert_eq!(
            coupled_retrieval_probe().unwrap(),
            coupled_retrieval_probe().unwrap()
        );
    }
}

// v0.9: A single chronological stream drives one Memory per run.
// Baseline and adaptive modes differ only in explicit policy application.
fn unified_scenario() -> Vec<TimedEvent> {
    let mut events = scenario();

    for timed in &mut events {
        timed.action = match timed.action.clone() {
            Event::Activation("record_a", _) => Event::Dynamics("record_a", 0.2, 0.8),
            Event::Activation("record_b", _) => Event::Dynamics("record_b", 0.8, 0.1),
            other => other,
        };
    }

    events.extend([
        TimedEvent {
            day: 90,
            action: Event::Review("record_a"),
        },
        TimedEvent {
            day: 180,
            action: Event::Review("record_b"),
        },
        TimedEvent {
            day: 200,
            action: Event::Use("record_a"),
        },
    ]);

    events.sort_by_key(|event| event.day);
    events
}

#[derive(Debug, PartialEq)]
struct UnifiedTrace {
    activation_a: f32,
    activation_b: f32,
    recorded_uses: u64,
    contextual_correct: usize,
    probes: usize,
    linked_a: String,
    linked_b: String,
}

fn unified_trace(adaptive: bool) -> Result<UnifiedTrace, String> {
    use transformer_memory_engine::processing::RetrievalDecision;

    let (report, memory) = replay_with_mode(&unified_scenario(), adaptive)?;

    let resolve = |anchor| -> Result<String, String> {
        match memory.process_retrieval_linked(&[1.0, 0.0], anchor, 365 * 86_400_000)? {
            RetrievalDecision::Found(candidate) => Ok(candidate.token_id),
            _ => Err(format!("No unique relation for {anchor}")),
        }
    };

    let a = memory.dynamics("record_a").ok_or("Missing record_a")?;
    let b = memory.dynamics("record_b").ok_or("Missing record_b")?;

    Ok(UnifiedTrace {
        activation_a: a.activation,
        activation_b: b.activation,
        recorded_uses: report.recorded_uses,
        contextual_correct: report.probes.iter().filter(|probe| probe.correct).count(),
        probes: report.probes.len(),
        linked_a: resolve("client_a")?,
        linked_b: resolve("client_b")?,
    })
}

#[cfg(test)]
mod unified_replay_tests {
    use super::*;

    #[test]
    fn identical_event_history_for_both_modes() {
        let baseline = unified_trace(false).unwrap();
        let adaptive = unified_trace(true).unwrap();

        assert_eq!(baseline.recorded_uses, 2);
        assert_eq!(baseline.recorded_uses, adaptive.recorded_uses);
        assert_eq!(baseline.probes, adaptive.probes);
    }

    #[test]
    fn baseline_keeps_initial_activation() {
        let trace = unified_trace(false).unwrap();
        assert!((trace.activation_a - 0.2).abs() < 1e-6);
        assert!((trace.activation_b - 0.8).abs() < 1e-6);
    }

    #[test]
    fn adaptive_events_change_activation() {
        let trace = unified_trace(true).unwrap();
        assert!((trace.activation_a - 0.59).abs() < 1e-6);
        assert!((trace.activation_b - 0.495).abs() < 1e-6);
    }

    #[test]
    fn both_modes_keep_correct_context_relations() {
        for adaptive in [false, true] {
            let trace = unified_trace(adaptive).unwrap();
            assert_eq!(trace.linked_a, "record_a");
            assert_eq!(trace.linked_b, "record_b");
            assert_eq!(trace.contextual_correct, 2);
            assert_eq!(trace.probes, 2);
        }
    }

    #[test]
    fn unified_replay_is_deterministic() {
        assert_eq!(unified_trace(true).unwrap(), unified_trace(true).unwrap());
    }

    #[test]
    fn unified_replay_rejects_reversed_timeline() {
        let mut events = unified_scenario();
        events.reverse();
        assert!(replay_with_mode(&events, true).is_err());
    }
}

// v1.0: Contextual conflict review using existing Memory APIs.
// No inferred truth, automatic correction, or consolidation.
fn conflict_reflect_trace(cutoff: i64) -> Result<(usize, Vec<String>, bool), String> {
    use transformer_memory_engine::contextual_evidence::{
        ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
    };
    use transformer_memory_engine::experience::EvidenceSource;

    let mut memory = Memory::new();

    for id in ["event", "outcome"] {
        memory.insert(StateToken {
            id: id.into(),
            vector: vec![1.0, 0.0],
        })?;
    }

    // Explicit event schedule. Availability controls historical reads.
    for (id, occurred, available, context) in [
        ("positive_a", true, 10, "normal"),
        ("positive_b", true, 20, "special"),
        ("negative_a", false, 30, "normal"),
        ("negative_b", false, 40, "special"),
    ] {
        memory.record_contextual_evidence(ContextualEvidenceRecord {
            schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
            id: id.into(),
            from: "event".into(),
            to: "outcome".into(),
            occurred,
            observed_at_unix_ms: available,
            available_at_unix_ms: available,
            source: EvidenceSource::ExternalObservation,
            reference: format!("synthetic/{id}"),
            context: BTreeMap::from([("mode".into(), context.into())]),
        })?;
    }

    let before = serde_json::to_value(&memory).map_err(|error| error.to_string())?;

    let report = memory.reflect_contextual_as_of("event", "outcome", "mode", cutoff)?;

    let after = serde_json::to_value(&memory).map_err(|error| error.to_string())?;

    if before != after || report.memory_changed {
        return Err("Reflect unexpectedly modified memory".into());
    }

    Ok((
        report.evidence_quality.total_records,
        report.mixed_context_values,
        report.review_required,
    ))
}

#[cfg(test)]
mod conflict_reflect_tests {
    use super::*;

    #[test]
    fn positive_evidence_alone_needs_no_mixed_review() {
        assert_eq!(conflict_reflect_trace(20).unwrap(), (2, vec![], false));
    }

    #[test]
    fn opposite_result_in_same_context_requires_review() {
        assert_eq!(
            conflict_reflect_trace(30).unwrap(),
            (3, vec!["normal".into()], true)
        );
    }

    #[test]
    fn both_contexts_can_be_mixed() {
        assert_eq!(
            conflict_reflect_trace(40).unwrap(),
            (4, vec!["normal".into(), "special".into()], true)
        );
    }

    #[test]
    fn unavailable_evidence_is_not_used_early() {
        assert_eq!(conflict_reflect_trace(29).unwrap(), (2, vec![], false));
    }

    #[test]
    fn conflict_reflection_is_deterministic() {
        assert_eq!(
            conflict_reflect_trace(40).unwrap(),
            conflict_reflect_trace(40).unwrap()
        );
    }
}

// v1.1: Reflect and adaptive events use one chronological dispatcher.
fn lifecycle_scenario() -> Vec<TimedEvent> {
    let mut events = unified_scenario();

    events.extend([
        TimedEvent {
            day: 10,
            action: Event::ContextEvidence("client_a", "record_a", true, "normal"),
        },
        TimedEvent {
            day: 20,
            action: Event::ReflectProbe("client_a", "record_a", "mode"),
        },
        TimedEvent {
            day: 30,
            action: Event::ContextEvidence("client_a", "record_a", false, "normal"),
        },
        TimedEvent {
            day: 40,
            action: Event::ReflectProbe("client_a", "record_a", "mode"),
        },
        TimedEvent {
            day: 50,
            action: Event::ContextEvidence("client_a", "record_a", true, "special"),
        },
        TimedEvent {
            day: 60,
            action: Event::ReflectProbe("client_a", "record_a", "mode"),
        },
    ]);

    events.sort_by_key(|event| event.day);
    events
}

fn lifecycle_trace(adaptive: bool) -> Result<(Report, f32, f32), String> {
    let (report, memory) = replay_with_mode(&lifecycle_scenario(), adaptive)?;

    let a = memory.dynamics("record_a").ok_or("Missing record_a")?;
    let b = memory.dynamics("record_b").ok_or("Missing record_b")?;

    Ok((report, a.activation, b.activation))
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[test]
    fn reflect_probes_observe_chronological_state() {
        let (report, _, _) = lifecycle_trace(false).unwrap();
        assert_eq!(report.reflect_probes.len(), 3);

        assert_eq!(
            report
                .reflect_probes
                .iter()
                .map(|p| (p.day, p.total_records, p.review_required))
                .collect::<Vec<_>>(),
            vec![(20, 2, false), (40, 3, true), (60, 4, true)]
        );

        assert_eq!(
            report.reflect_probes[1].mixed_context_values,
            vec!["normal"]
        );
        assert_eq!(
            report.reflect_probes[2].mixed_context_values,
            vec!["normal"]
        );
    }

    #[test]
    fn adaptive_and_control_runs_share_evidence_results() {
        let (control, a, b) = lifecycle_trace(false).unwrap();
        let (adaptive, aa, ab) = lifecycle_trace(true).unwrap();

        assert_eq!(control.reflect_probes, adaptive.reflect_probes);
        assert_eq!(control.recorded_uses, adaptive.recorded_uses);
        assert_eq!(control.probes, adaptive.probes);

        assert!((a - 0.2).abs() < 1e-6);
        assert!((b - 0.8).abs() < 1e-6);
        assert!((aa - 0.59).abs() < 1e-6);
        assert!((ab - 0.495).abs() < 1e-6);
    }

    #[test]
    fn lifecycle_replay_is_deterministic() {
        assert_eq!(
            lifecycle_trace(true).unwrap(),
            lifecycle_trace(true).unwrap()
        );
    }

    #[test]
    fn lifecycle_rejects_out_of_order_events() {
        let mut events = lifecycle_scenario();
        events.reverse();
        assert!(replay_with_mode(&events, false).is_err());
        assert!(replay_with_mode(&events, true).is_err());
    }
}
