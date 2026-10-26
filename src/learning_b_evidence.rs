use crate::{
    contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
    learning_b_events::{ExecutionEvent, ExecutionOutcome},
    Memory,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionEvidenceReport {
    pub task_id: String,
    pub recorded_ids: Vec<String>,
}

fn outcome_name(outcome: &ExecutionOutcome) -> &'static str {
    match outcome {
        ExecutionOutcome::Success => "success",
        ExecutionOutcome::Failure => "failure",
        ExecutionOutcome::Unknown => "unknown",
    }
}

impl Memory {
    /// Persist observed adjacent execution steps as contextual evidence.
    ///
    /// event_times are supplied by the execution runtime, not inferred.
    /// available_at is when the trace could be evaluated.
    ///
    /// The operation is atomic in memory.
    /// No causal dependency, skill mastery or relation weight is inferred.
    pub fn learning_b_record_execution_evidence(
        &mut self,
        events: &[ExecutionEvent],
        event_times: &BTreeMap<String, i64>,
        available_at_unix_ms: i64,
    ) -> Result<ExecutionEvidenceReport, String> {
        let preview = self.learning_b_transition_preview(events)?;

        let event_ids: BTreeSet<&str> = events.iter().map(|e| e.id.as_str()).collect();

        let supplied_ids: BTreeSet<&str> = event_times.keys().map(String::as_str).collect();

        if event_ids != supplied_ids {
            return Err("Event timestamps do not match trace".into());
        }

        let mut previous_time = None;

        for event in events {
            let time = *event_times
                .get(&event.id)
                .ok_or("Missing event timestamp")?;

            if previous_time.is_some_and(|previous| time < previous) {
                return Err("Event timestamps are not ordered".into());
            }

            if time > available_at_unix_ms {
                return Err("Evidence cannot precede its observation".into());
            }

            previous_time = Some(time);
        }

        // One evidence source cannot faithfully represent two
        // different sources. Preserve the trace without writing
        // a misleading combined observation in that case.
        for pair in events.windows(2) {
            if pair[0].source != pair[1].source {
                return Err("Mixed evidence sources require separate attribution".into());
            }
        }

        // Stage every write. A failure must leave Memory A unchanged.
        let mut staged = self.clone();
        let mut recorded_ids = Vec::new();

        for (transition, pair) in preview.transitions.iter().zip(events.windows(2)) {
            if !transition.states_known {
                return Err("Transition references an unresolved state".into());
            }

            let from_time = *event_times
                .get(&transition.from_event_id)
                .ok_or("Missing source timestamp")?;

            let to_time = *event_times
                .get(&transition.to_event_id)
                .ok_or("Missing target timestamp")?;

            // JSON tuple encoding avoids collisions between IDs
            // containing separators or special characters.
            let identity = serde_json::to_string(&(
                transition.task_id.as_str(),
                transition.from_event_id.as_str(),
                transition.to_event_id.as_str(),
            ))
            .map_err(|e| e.to_string())?;

            let id = format!("learning_b/adjacent/{identity}");

            let reference = serde_json::to_string(&(
                transition.from_reference.as_str(),
                transition.to_reference.as_str(),
            ))
            .map_err(|e| e.to_string())?;

            let context = BTreeMap::from([
                ("observation_kind".into(), "execution_adjacency".into()),
                ("task_id".into(), transition.task_id.clone()),
                ("from_event_id".into(), transition.from_event_id.clone()),
                ("to_event_id".into(), transition.to_event_id.clone()),
                (
                    "from_outcome".into(),
                    outcome_name(&transition.from_outcome).into(),
                ),
                (
                    "to_outcome".into(),
                    outcome_name(&transition.to_outcome).into(),
                ),
                ("from_time_unix_ms".into(), from_time.to_string()),
            ]);

            staged.record_contextual_evidence(ContextualEvidenceRecord {
                schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                id: id.clone(),
                from: transition.from.clone(),
                to: transition.to.clone(),
                occurred: true,
                observed_at_unix_ms: to_time,
                available_at_unix_ms,
                source: pair[0].source.clone(),
                reference,
                context,
            })?;

            recorded_ids.push(id);
        }

        // Commit only after all records pass validation.
        self.contextual_evidence = staged.contextual_evidence;

        Ok(ExecutionEvidenceReport {
            task_id: preview.task_id,
            recorded_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{experience::EvidenceSource, StateToken};

    fn fixture() -> (Memory, Vec<ExecutionEvent>, BTreeMap<String, i64>) {
        let mut memory = Memory::new();

        for id in ["api.select", "api.call", "response.verify"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        let events = ["api.select", "api.call", "response.verify"]
            .iter()
            .enumerate()
            .map(|(i, state)| ExecutionEvent {
                id: format!("e{}", i + 1),
                task_id: "task-001".into(),
                state_id: (*state).into(),
                outcome: ExecutionOutcome::Success,
                source: EvidenceSource::VerifiedToolResult,
                reference: format!("tool/result/{}", i + 1),
            })
            .collect();

        let times = BTreeMap::from([("e1".into(), 100), ("e2".into(), 200), ("e3".into(), 300)]);

        (memory, events, times)
    }

    #[test]
    fn persists_transitions_without_reinforcement() {
        let (mut memory, events, times) = fixture();

        let result = memory
            .learning_b_record_execution_evidence(&events, &times, 400)
            .unwrap();

        assert_eq!(result.recorded_ids.len(), 2);
        assert_eq!(memory.contextual_evidence.len(), 2);
        assert!(memory.relations.is_empty());
        assert!(memory.transitions.is_empty());

        let first = &memory.contextual_evidence[&result.recorded_ids[0]];
        assert_eq!(first.from, "api.select");
        assert_eq!(first.to, "api.call");
        assert_eq!(first.observed_at_unix_ms, 200);
        assert_eq!(first.available_at_unix_ms, 400);
        assert_eq!(first.context["observation_kind"], "execution_adjacency");

        let restored: Memory =
            serde_json::from_str(&serde_json::to_string(&memory).unwrap()).unwrap();

        assert_eq!(restored.contextual_evidence.len(), 2);
    }

    #[test]
    fn duplicate_trace_does_not_add_evidence() {
        let (mut memory, events, times) = fixture();

        memory
            .learning_b_record_execution_evidence(&events, &times, 400)
            .unwrap();

        let before = serde_json::to_string(&memory).unwrap();

        assert!(memory
            .learning_b_record_execution_evidence(&events, &times, 400)
            .is_err());

        assert_eq!(before, serde_json::to_string(&memory).unwrap());
    }

    #[test]
    fn unknown_state_cannot_partially_commit() {
        let (mut memory, mut events, times) = fixture();

        events[2].state_id = "unknown.state".into();
        let before = serde_json::to_string(&memory).unwrap();

        assert!(memory
            .learning_b_record_execution_evidence(&events, &times, 400)
            .is_err());

        assert_eq!(before, serde_json::to_string(&memory).unwrap());
    }

    #[test]
    fn rejects_invalid_timing_and_mixed_sources() {
        let (mut memory, mut events, mut times) = fixture();

        times.insert("e2".into(), 50);
        assert!(memory
            .learning_b_record_execution_evidence(&events, &times, 400)
            .is_err());

        times.insert("e2".into(), 200);
        events[1].source = EvidenceSource::ModelStatement;

        assert!(memory
            .learning_b_record_execution_evidence(&events, &times, 400)
            .is_err());

        assert!(memory.contextual_evidence.is_empty());
    }
}
