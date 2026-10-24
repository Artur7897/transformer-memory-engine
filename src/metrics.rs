use crate::Memory;
use serde::{Deserialize, Serialize};

/// Deterministic measurements of persisted memory content.
///
/// These are observations, not autonomous Reflect/Sleep triggers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub state_count: usize,
    pub relation_count: usize,
    pub transition_pair_count: usize,
    pub observed_transitions: u128,
    pub missed_transitions: u128,
    pub mixed_outcome_pairs: usize,
    pub unobserved_transition_pairs: usize,
}

impl MemoryMetrics {
    pub fn total_observations(&self) -> u128 {
        self.observed_transitions + self.missed_transitions
    }
}

impl Memory {
    /// Return a read-only snapshot of currently measurable state.
    ///
    /// A mixed-outcome pair is not necessarily a logical contradiction.
    /// No decay, consolidation or trigger is executed.
    pub fn metrics(&self) -> MemoryMetrics {
        let mut result = MemoryMetrics {
            state_count: self.states.len(),
            relation_count: self.relations.len(),
            transition_pair_count: self.transitions.len(),
            ..MemoryMetrics::default()
        };

        for evidence in self.transitions.values() {
            result.observed_transitions += u128::from(evidence.observed);
            result.missed_transitions += u128::from(evidence.missed);

            if evidence.observed > 0 && evidence.missed > 0 {
                result.mixed_outcome_pairs += 1;
            }

            if evidence.observed == 0 && evidence.missed == 0 {
                result.unobserved_transition_pairs += 1;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn token(id: &str) -> StateToken {
        StateToken {
            id: id.to_string(),
            vector: vec![1.0, 0.0],
        }
    }

    #[test]
    fn empty_memory_has_zero_metrics() {
        assert_eq!(Memory::new().metrics(), MemoryMetrics::default());
    }

    #[test]
    fn metrics_track_real_observations() {
        let mut memory = Memory::new();
        memory.insert(token("a")).unwrap();
        memory.insert(token("b")).unwrap();

        memory.reinforce("a", "b", 0.5).unwrap();

        memory.observe_transition("a", "b", true).unwrap();
        memory.observe_transition("a", "b", true).unwrap();
        memory.observe_transition("a", "b", false).unwrap();

        let m = memory.metrics();

        assert_eq!(m.state_count, 2);
        assert_eq!(m.relation_count, 1);
        assert_eq!(m.transition_pair_count, 1);
        assert_eq!(m.observed_transitions, 2);
        assert_eq!(m.missed_transitions, 1);
        assert_eq!(m.total_observations(), 3);
        assert_eq!(m.mixed_outcome_pairs, 1);
    }

    #[test]
    fn metrics_do_not_change_memory() {
        let mut memory = Memory::new();
        memory.insert(token("a")).unwrap();
        memory.insert(token("b")).unwrap();
        memory.observe_transition("a", "b", true).unwrap();

        let before = serde_json::to_value(&memory).unwrap();

        let first = memory.metrics();
        let second = memory.metrics();

        assert_eq!(first, second);
        assert_eq!(serde_json::to_value(&memory).unwrap(), before);
    }

    #[test]
    fn legacy_memory_remains_compatible() {
        let legacy = r#"{
          "states": {},
          "relations": {},
          "cycles": 0
        }"#;

        let memory: Memory = serde_json::from_str(legacy).unwrap();

        assert_eq!(memory.metrics(), MemoryMetrics::default());
    }
}
