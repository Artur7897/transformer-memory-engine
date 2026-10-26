use crate::learning_b::ActivationTier;
use crate::Memory;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct WeightedActivation {
    pub token_id: String,
    pub tier: ActivationTier,
    pub depth: usize,
    pub path_weight: f32,
    pub parent: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeightedActivationView {
    pub anchor: String,
    pub states: Vec<WeightedActivation>,
    pub truncated: bool,
}

impl Memory {
    /// Read-only weighted graph exploration.
    ///
    /// Candidates are selected by graph depth first, then by
    /// descending path weight. This is an experimental baseline,
    /// not yet a calibrated semantic Attention mechanism.
    pub fn learning_b_weighted_preview(
        &self,
        anchor: &str,
        max_depth: usize,
        max_states: usize,
    ) -> Result<WeightedActivationView, String> {
        if !self.states.contains_key(anchor) {
            return Err("Unknown task anchor".into());
        }
        if max_states == 0 {
            return Err("max_states must be positive".into());
        }

        let mut adjacency: BTreeMap<&str, Vec<(&str, f32)>> = BTreeMap::new();

        for relation in self.relations.values() {
            if !relation.weight.is_finite() || !(0.0..=1.0).contains(&relation.weight) {
                return Err("Invalid relation weight".into());
            }
            if !self.states.contains_key(&relation.from) || !self.states.contains_key(&relation.to)
            {
                return Err("Relation references unknown state".into());
            }
            if relation.weight > 0.0 {
                adjacency
                    .entry(relation.from.as_str())
                    .or_default()
                    .push((relation.to.as_str(), relation.weight));
            }
        }

        let mut visited = BTreeSet::new();
        visited.insert(anchor.to_string());

        let root = WeightedActivation {
            token_id: anchor.to_string(),
            tier: ActivationTier::L1,
            depth: 0,
            path_weight: 1.0,
            parent: None,
        };

        let mut result = vec![root.clone()];
        let mut frontier = vec![root];
        let mut truncated = false;

        for depth in 1..=max_depth {
            let mut candidates: BTreeMap<String, WeightedActivation> = BTreeMap::new();

            for parent in &frontier {
                if let Some(edges) = adjacency.get(parent.token_id.as_str()) {
                    for (target, weight) in edges {
                        if visited.contains(*target) {
                            continue;
                        }

                        let score = parent.path_weight * weight;
                        let candidate = WeightedActivation {
                            token_id: (*target).to_string(),
                            tier: match depth {
                                1 => ActivationTier::L1,
                                2 => ActivationTier::L2,
                                _ => ActivationTier::L3,
                            },
                            depth,
                            path_weight: score,
                            parent: Some(parent.token_id.clone()),
                        };

                        let replace = candidates.get(*target).is_none_or(|old| {
                            score > old.path_weight
                                || (score == old.path_weight && candidate.parent < old.parent)
                        });

                        if replace {
                            candidates.insert((*target).to_string(), candidate);
                        }
                    }
                }
            }

            let mut ordered: Vec<_> = candidates.into_values().collect();
            ordered.sort_by(|a, b| {
                b.path_weight
                    .total_cmp(&a.path_weight)
                    .then_with(|| a.token_id.cmp(&b.token_id))
            });

            let remaining = max_states - result.len();
            if ordered.len() > remaining {
                truncated = true;
                ordered.truncate(remaining);
            }

            if ordered.is_empty() {
                break;
            }

            for state in &ordered {
                visited.insert(state.token_id.clone());
            }

            result.extend(ordered.iter().cloned());
            frontier = ordered;

            if result.len() >= max_states {
                // An unexplored frontier may still exist.
                // This is reported conservatively as budget-limited.
                if depth < max_depth {
                    truncated = true;
                }
                break;
            }
        }

        Ok(WeightedActivationView {
            anchor: anchor.to_string(),
            states: result,
            truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture() -> Memory {
        let mut memory = Memory::new();

        for id in ["task_a", "task_b", "strong", "weak", "bridge", "shared"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        memory.reinforce("task_a", "weak", 0.2).unwrap();
        memory.reinforce("task_a", "strong", 0.9).unwrap();
        memory.reinforce("strong", "shared", 0.8).unwrap();

        memory.reinforce("task_b", "bridge", 0.9).unwrap();
        memory.reinforce("bridge", "weak", 0.8).unwrap();
        memory.reinforce("weak", "shared", 0.7).unwrap();

        memory
    }

    #[test]
    fn stronger_relation_wins_limited_budget() {
        let memory = fixture();
        let view = memory.learning_b_weighted_preview("task_a", 1, 2).unwrap();

        assert_eq!(view.states.len(), 2);
        assert_eq!(view.states[1].token_id, "strong");
        assert_eq!(view.states[1].tier, ActivationTier::L1);
        assert!(view.truncated);
    }

    #[test]
    fn shared_token_is_task_relative() {
        let memory = fixture();

        let a = memory.learning_b_weighted_preview("task_a", 4, 20).unwrap();
        let b = memory.learning_b_weighted_preview("task_b", 4, 20).unwrap();

        let shared_a = a.states.iter().find(|s| s.token_id == "shared").unwrap();
        let shared_b = b.states.iter().find(|s| s.token_id == "shared").unwrap();

        assert_eq!(shared_a.tier, ActivationTier::L2);
        assert_eq!(shared_b.tier, ActivationTier::L3);
        assert_eq!(memory.states.len(), 6);
    }

    #[test]
    fn deterministic_read_only_and_validation() {
        let memory = fixture();
        let before = serde_json::to_value(&memory).unwrap();

        let first = memory.learning_b_weighted_preview("task_a", 4, 20).unwrap();
        let second = memory.learning_b_weighted_preview("task_a", 4, 20).unwrap();

        assert_eq!(first, second);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());

        assert!(memory
            .learning_b_weighted_preview("missing", 4, 20)
            .is_err());
        assert!(memory.learning_b_weighted_preview("task_a", 4, 0).is_err());
    }
}
