use crate::{experience::EvidenceSource, Memory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionOutcome {
    Success,
    Failure,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub id: String,
    pub task_id: String,
    pub state_id: String,
    pub outcome: ExecutionOutcome,
    pub source: EvidenceSource,
    pub reference: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventAssessment {
    pub event_id: String,
    pub state_id: String,
    pub known_state: bool,
    pub outcome: ExecutionOutcome,
    pub source: EvidenceSource,
    pub reference: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionAssessment {
    pub task_id: String,
    pub events: Vec<EventAssessment>,
    pub unresolved_states: Vec<String>,
}

impl Memory {
    /// Read-only assessment of an execution trace.
    ///
    /// Events must already be ordered by the producing runtime.
    /// Unknown states are reported, not given fabricated vectors.
    /// No capability, causal relation or success claim is inferred.
    pub fn assess_execution_events(
        &self,
        events: &[ExecutionEvent],
    ) -> Result<ExecutionAssessment, String> {
        let first = events.first().ok_or("Execution trace is empty")?;

        if first.task_id.trim().is_empty() {
            return Err("Task ID is empty".into());
        }

        let mut seen_events = BTreeSet::new();
        let mut unresolved = BTreeSet::new();
        let mut assessments = Vec::with_capacity(events.len());

        for event in events {
            if event.task_id != first.task_id {
                return Err("Mixed task IDs in execution trace".into());
            }

            if event.id.trim().is_empty()
                || event.state_id.trim().is_empty()
                || event.reference.trim().is_empty()
            {
                return Err("Required execution event field is empty".into());
            }

            if !seen_events.insert(event.id.as_str()) {
                return Err("Duplicate execution event ID".into());
            }

            let known = self.states.contains_key(&event.state_id);

            if !known {
                unresolved.insert(event.state_id.clone());
            }

            assessments.push(EventAssessment {
                event_id: event.id.clone(),
                state_id: event.state_id.clone(),
                known_state: known,
                outcome: event.outcome.clone(),
                source: event.source.clone(),
                reference: event.reference.clone(),
            });
        }

        Ok(ExecutionAssessment {
            task_id: first.task_id.clone(),
            events: assessments,
            unresolved_states: unresolved.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn event(id: &str, state_id: &str, outcome: ExecutionOutcome) -> ExecutionEvent {
        ExecutionEvent {
            id: id.into(),
            task_id: "task-001".into(),
            state_id: state_id.into(),
            outcome,
            source: EvidenceSource::VerifiedToolResult,
            reference: format!("test/{id}"),
        }
    }

    #[test]
    fn identifies_known_and_unknown_states_without_mutation() {
        let mut memory = Memory::new();
        memory
            .insert(StateToken {
                id: "api.call".into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();

        let before = serde_json::to_string(&memory).unwrap();

        let result = memory
            .assess_execution_events(&[
                event("e1", "api.call", ExecutionOutcome::Success),
                event("e2", "response.verify", ExecutionOutcome::Unknown),
            ])
            .unwrap();

        assert!(result.events[0].known_state);
        assert!(!result.events[1].known_state);
        assert_eq!(result.unresolved_states, vec!["response.verify"]);
        assert_eq!(before, serde_json::to_string(&memory).unwrap());
    }

    #[test]
    fn preserves_failure_and_evidence_source() {
        let memory = Memory::new();
        let result = memory
            .assess_execution_events(&[event("e1", "api.call", ExecutionOutcome::Failure)])
            .unwrap();

        assert_eq!(result.events[0].outcome, ExecutionOutcome::Failure);
        assert_eq!(result.events[0].source, EvidenceSource::VerifiedToolResult);
    }

    #[test]
    fn rejects_invalid_traces() {
        let memory = Memory::new();
        assert!(memory.assess_execution_events(&[]).is_err());

        let duplicate = event("e1", "api.call", ExecutionOutcome::Success);
        assert!(memory
            .assess_execution_events(&[duplicate.clone(), duplicate,])
            .is_err());

        let mut other_task = event("e2", "api.call", ExecutionOutcome::Success);
        other_task.task_id = "different-task".into();

        assert!(memory
            .assess_execution_events(&[
                event("e1", "api.call", ExecutionOutcome::Success),
                other_task,
            ])
            .is_err());
    }
}
