use crate::{
    learning_b_events::{ExecutionEvent, ExecutionOutcome},
    Memory,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedTransition {
    pub task_id: String,
    pub from: String,
    pub to: String,
    pub from_event_id: String,
    pub to_event_id: String,
    pub from_outcome: ExecutionOutcome,
    pub to_outcome: ExecutionOutcome,
    pub from_reference: String,
    pub to_reference: String,
    pub states_known: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionPreview {
    pub task_id: String,
    pub transitions: Vec<ObservedTransition>,
}

impl Memory {
    /// Derive consecutive execution transitions without mutation.
    ///
    /// Event order must be supplied by the execution runtime.
    /// Adjacency is temporal, not proof of causality or skill mastery.
    pub fn learning_b_transition_preview(
        &self,
        events: &[ExecutionEvent],
    ) -> Result<TransitionPreview, String> {
        let assessment = self.assess_execution_events(events)?;
        let mut transitions = Vec::new();

        for pair in assessment.events.windows(2) {
            let from = &pair[0];
            let to = &pair[1];

            transitions.push(ObservedTransition {
                task_id: assessment.task_id.clone(),
                from: from.state_id.clone(),
                to: to.state_id.clone(),
                from_event_id: from.event_id.clone(),
                to_event_id: to.event_id.clone(),
                from_outcome: from.outcome.clone(),
                to_outcome: to.outcome.clone(),
                from_reference: from.reference.clone(),
                to_reference: to.reference.clone(),
                states_known: from.known_state && to.known_state,
            });
        }

        Ok(TransitionPreview {
            task_id: assessment.task_id,
            transitions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{experience::EvidenceSource, StateToken};

    fn event(id: &str, state: &str, outcome: ExecutionOutcome) -> ExecutionEvent {
        ExecutionEvent {
            id: id.into(),
            task_id: "task-001".into(),
            state_id: state.into(),
            outcome,
            source: EvidenceSource::VerifiedToolResult,
            reference: format!("test/{id}"),
        }
    }

    #[test]
    fn identifies_consecutive_transitions_without_mutation() {
        let mut memory = Memory::new();

        for id in ["api.select", "api.call", "response.verify"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        let before = serde_json::to_string(&memory).unwrap();

        let result = memory
            .learning_b_transition_preview(&[
                event("e1", "api.select", ExecutionOutcome::Success),
                event("e2", "api.call", ExecutionOutcome::Success),
                event("e3", "response.verify", ExecutionOutcome::Success),
            ])
            .unwrap();

        assert_eq!(result.transitions.len(), 2);
        assert_eq!(result.transitions[0].from, "api.select");
        assert_eq!(result.transitions[0].to, "api.call");
        assert_eq!(result.transitions[1].from, "api.call");
        assert_eq!(result.transitions[1].to, "response.verify");
        assert!(result.transitions.iter().all(|t| t.states_known));
        assert_eq!(before, serde_json::to_string(&memory).unwrap());
    }

    #[test]
    fn preserves_failures_and_unknown_states() {
        let memory = Memory::new();

        let result = memory
            .learning_b_transition_preview(&[
                event("e1", "api.call", ExecutionOutcome::Failure),
                event("e2", "documentation.read", ExecutionOutcome::Unknown),
            ])
            .unwrap();

        assert_eq!(result.transitions.len(), 1);
        assert!(!result.transitions[0].states_known);
        assert_eq!(
            result.transitions[0].from_outcome,
            ExecutionOutcome::Failure
        );
        assert_eq!(result.transitions[0].to_outcome, ExecutionOutcome::Unknown);
    }

    #[test]
    fn single_event_has_no_transition() {
        let memory = Memory::new();

        let result = memory
            .learning_b_transition_preview(&[event("e1", "api.call", ExecutionOutcome::Success)])
            .unwrap();

        assert!(result.transitions.is_empty());
    }

    #[test]
    fn rejects_invalid_traces() {
        let memory = Memory::new();

        assert!(memory.learning_b_transition_preview(&[]).is_err());

        let duplicate = event("e1", "api.call", ExecutionOutcome::Success);

        assert!(memory
            .learning_b_transition_preview(&[duplicate.clone(), duplicate,])
            .is_err());
    }
}
