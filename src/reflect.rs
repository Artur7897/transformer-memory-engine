use crate::{metrics::MemoryMetrics, Memory};
use serde::{Deserialize, Serialize};

/// A candidate for later consolidation, not a conclusion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenReflection {
    pub state_id: String,
    pub pending_operations: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionReflection {
    pub from: String,
    pub to: String,
    pub observed: u64,
    pub missed: u64,
}

/// Deterministic read-only inspection of memory.
///
/// No hypotheses are promoted to facts.
/// No state, evidence, weight or counter is modified.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectReport {
    pub metrics: MemoryMetrics,
    pub pending_tokens: Vec<TokenReflection>,
    pub mixed_transitions: Vec<TransitionReflection>,
    pub pending_experience_ids: Vec<String>,
}

impl Memory {
    pub fn reflect_preview(&self) -> Result<ReflectReport, String> {
        let mut pending_tokens = Vec::new();

        for id in self.states.keys() {
            let dynamics = self.dynamics(id).ok_or("Stored state missing dynamics")?;

            if dynamics.pending_operations() > 0 {
                pending_tokens.push(TokenReflection {
                    state_id: id.clone(),
                    pending_operations: dynamics.pending_operations(),
                });
            }
        }

        let mut mixed_transitions = Vec::new();

        for (key, evidence) in &self.transitions {
            if evidence.observed == 0 || evidence.missed == 0 {
                continue;
            }

            let (from, to): (String, String) =
                serde_json::from_str(key).map_err(|_| format!("Invalid transition key: {key}"))?;

            mixed_transitions.push(TransitionReflection {
                from,
                to,
                observed: evidence.observed,
                missed: evidence.missed,
            });
        }

        mixed_transitions.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));

        let pending_experience_ids = self
            .experiences
            .values()
            .filter(|e| e.observed.is_none())
            .map(|e| e.id.clone())
            .collect();

        Ok(ReflectReport {
            metrics: self.metrics(),
            pending_tokens,
            mixed_transitions,
            pending_experience_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn memory() -> Memory {
        let mut m = Memory::new();

        for id in ["a", "b"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![0.5, 0.2],
            })
            .unwrap();
        }

        m
    }

    #[test]
    fn reflect_is_deterministic_and_read_only() {
        let mut m = memory();

        m.record_token_operation("a").unwrap();
        m.record_token_operation("a").unwrap();
        m.observe_transition("a", "b", true).unwrap();
        m.observe_transition("a", "b", false).unwrap();
        m.begin_experience("exp-1", "a", "UP").unwrap();

        let before = serde_json::to_value(&m).unwrap();

        let first = m.reflect_preview().unwrap();
        let second = m.reflect_preview().unwrap();

        assert_eq!(first, second);
        assert_eq!(serde_json::to_value(&m).unwrap(), before);

        assert_eq!(first.pending_tokens.len(), 1);
        assert_eq!(first.pending_tokens[0].state_id, "a");
        assert_eq!(first.pending_tokens[0].pending_operations, 2);

        assert_eq!(first.mixed_transitions.len(), 1);
        assert_eq!(first.mixed_transitions[0].observed, 1);
        assert_eq!(first.mixed_transitions[0].missed, 1);

        assert_eq!(first.pending_experience_ids, vec!["exp-1"]);
    }

    #[test]
    fn mixed_outcomes_are_not_invented() {
        let mut m = memory();

        m.observe_transition("a", "b", true).unwrap();
        m.observe_transition("a", "b", true).unwrap();

        let report = m.reflect_preview().unwrap();

        assert!(report.mixed_transitions.is_empty());
        assert_eq!(report.metrics.observed_transitions, 2);
        assert_eq!(report.metrics.missed_transitions, 0);
    }

    #[test]
    fn reflection_does_not_consume_pending_operations() {
        let mut m = memory();

        m.record_token_operation("a").unwrap();

        let first = m.reflect_preview().unwrap();
        let second = m.reflect_preview().unwrap();

        assert_eq!(first, second);
        assert_eq!(m.dynamics("a").unwrap().pending_operations(), 1);
        assert_eq!(m.dynamics("a").unwrap().last_reflected_operations, 0);
    }

    #[test]
    fn legacy_memory_can_be_reflected() {
        let legacy = r#"{
            "states": {
                "a": {"id": "a", "vector": [1.0, 0.0]}
            },
            "relations": {},
            "cycles": 0
        }"#;

        let m: Memory = serde_json::from_str(legacy).unwrap();
        let report = m.reflect_preview().unwrap();

        assert_eq!(report.metrics.state_count, 1);
        assert!(report.pending_tokens.is_empty());
        assert!(report.mixed_transitions.is_empty());
        assert!(report.pending_experience_ids.is_empty());
    }
}
