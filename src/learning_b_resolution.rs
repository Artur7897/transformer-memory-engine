use crate::{
    learning_b_events::{ExecutionEvent, ExecutionOutcome},
    Memory,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenOccurrence {
    pub task_id: String,
    pub event_id: String,
    pub outcome: ExecutionOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenResolution {
    pub state_id: String,
    pub known: bool,
    pub occurrence_count: usize,
    pub task_count: usize,
    pub occurrences: Vec<TokenOccurrence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionReport {
    pub task_count: usize,
    pub event_count: usize,
    pub known: Vec<TokenResolution>,
    pub unresolved: Vec<TokenResolution>,
}

impl Memory {
    /// Read-only resolution of explicitly supplied token identifiers.
    ///
    /// Matching IDs are treated as the same declared identifier,
    /// not as proof that separately named concepts are equivalent.
    ///
    /// Repeated observations indicate reuse, not skill mastery.
    pub fn learning_b_resolve_events(
        &self,
        traces: &[Vec<ExecutionEvent>],
    ) -> Result<ResolutionReport, String> {
        let mut tasks = BTreeSet::new();
        let mut occurrences: BTreeMap<String, Vec<TokenOccurrence>> = BTreeMap::new();
        let mut event_count = 0usize;

        for trace in traces {
            // Reuse the existing Learning B trace validation.
            let assessment = self.assess_execution_events(trace)?;

            if !tasks.insert(assessment.task_id.clone()) {
                return Err("Duplicate task ID across traces".into());
            }

            for event in &assessment.events {
                occurrences
                    .entry(event.state_id.clone())
                    .or_default()
                    .push(TokenOccurrence {
                        task_id: assessment.task_id.clone(),
                        event_id: event.event_id.clone(),
                        outcome: event.outcome.clone(),
                    });

                event_count += 1;
            }
        }

        let mut known = Vec::new();
        let mut unresolved = Vec::new();

        for (state_id, token_occurrences) in occurrences {
            let distinct_tasks: BTreeSet<&str> = token_occurrences
                .iter()
                .map(|event| event.task_id.as_str())
                .collect();

            let resolution = TokenResolution {
                known: self.states.contains_key(&state_id),
                state_id,
                occurrence_count: token_occurrences.len(),
                task_count: distinct_tasks.len(),
                occurrences: token_occurrences,
            };

            if resolution.known {
                known.push(resolution);
            } else {
                unresolved.push(resolution);
            }
        }

        Ok(ResolutionReport {
            task_count: tasks.len(),
            event_count,
            known,
            unresolved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{experience::EvidenceSource, StateToken};

    fn event(task: &str, id: &str, state: &str, outcome: ExecutionOutcome) -> ExecutionEvent {
        ExecutionEvent {
            id: id.into(),
            task_id: task.into(),
            state_id: state.into(),
            outcome,
            source: EvidenceSource::VerifiedToolResult,
            reference: format!("test/{task}/{id}"),
        }
    }

    #[test]
    fn shares_known_state_across_different_tasks() {
        let mut memory = Memory::new();
        memory
            .insert(StateToken {
                id: "api.call".into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();

        let before = serde_json::to_string(&memory).unwrap();

        let report = memory
            .learning_b_resolve_events(&[
                vec![
                    event(
                        "bitcoin-status",
                        "e1",
                        "api.call",
                        ExecutionOutcome::Success,
                    ),
                    event(
                        "bitcoin-status",
                        "e2",
                        "tx.verify",
                        ExecutionOutcome::Success,
                    ),
                ],
                vec![
                    event("bce-status", "e1", "api.call", ExecutionOutcome::Success),
                    event("bce-status", "e2", "bce.verify", ExecutionOutcome::Success),
                ],
            ])
            .unwrap();

        assert_eq!(report.task_count, 2);
        assert_eq!(report.event_count, 4);

        assert_eq!(report.known.len(), 1);
        assert_eq!(report.known[0].state_id, "api.call");
        assert_eq!(report.known[0].occurrence_count, 2);
        assert_eq!(report.known[0].task_count, 2);

        assert_eq!(report.unresolved.len(), 2);

        // No new token, relation, or evidence was written.
        assert_eq!(before, serde_json::to_string(&memory).unwrap());
    }

    #[test]
    fn counts_repeated_events_separately_from_tasks() {
        let memory = Memory::new();

        let report = memory
            .learning_b_resolve_events(&[vec![
                event("task-1", "e1", "api.call", ExecutionOutcome::Failure),
                event("task-1", "e2", "api.call", ExecutionOutcome::Success),
            ]])
            .unwrap();

        let token = &report.unresolved[0];

        assert_eq!(token.occurrence_count, 2);
        assert_eq!(token.task_count, 1);
        assert_eq!(token.occurrences[0].outcome, ExecutionOutcome::Failure);
    }

    #[test]
    fn does_not_merge_different_identifiers() {
        let memory = Memory::new();

        let report = memory
            .learning_b_resolve_events(&[
                vec![event("task-1", "e1", "api.call", ExecutionOutcome::Success)],
                vec![event(
                    "task-2",
                    "e1",
                    "http.request",
                    ExecutionOutcome::Success,
                )],
            ])
            .unwrap();

        assert_eq!(report.unresolved.len(), 2);
        assert_ne!(report.unresolved[0].state_id, report.unresolved[1].state_id);
    }

    #[test]
    fn rejects_duplicate_tasks_and_invalid_traces() {
        let memory = Memory::new();

        let trace = vec![event("task-1", "e1", "api.call", ExecutionOutcome::Success)];

        assert!(memory
            .learning_b_resolve_events(&[trace.clone(), trace])
            .is_err());

        assert!(memory.learning_b_resolve_events(&[vec![]]).is_err());
    }

    #[test]
    fn accepts_empty_collection_without_changes() {
        let memory = Memory::new();
        let report = memory.learning_b_resolve_events(&[]).unwrap();

        assert_eq!(report.task_count, 0);
        assert_eq!(report.event_count, 0);
        assert!(report.known.is_empty());
        assert!(report.unresolved.is_empty());
    }
}
