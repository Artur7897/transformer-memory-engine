use crate::Memory;
use std::collections::{BTreeMap, VecDeque};

/// Task-relative activation, not a persistent storage classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationTier {
    L1,
    L2,
    L3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivatedState {
    pub token_id: String,
    pub tier: ActivationTier,
    pub depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationBridge {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskActivationView {
    pub anchor: String,
    pub states: Vec<ActivatedState>,
    pub bridges: Vec<ActivationBridge>,
    pub truncated: bool,
}

impl Memory {
    /// Read-only, bounded traversal of existing directed relations.
    ///
    /// This prototype uses graph distance as a deterministic tier
    /// definition. It does NOT yet model semantic Attention,
    /// weighted prioritization or learned skills.
    ///
    /// Anchor and direct neighbors = L1
    /// Two hops from anchor     = L2
    /// Three or more hops       = L3
    pub fn learning_b_activation_preview(
        &self,
        anchor: &str,
        max_depth: usize,
        max_states: usize,
    ) -> Result<TaskActivationView, String> {
        if !self.states.contains_key(anchor) {
            return Err("Unknown task anchor".into());
        }
        if max_states == 0 {
            return Err("max_states must be positive".into());
        }

        let mut neighbors: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

        for relation in self.relations.values() {
            if !relation.weight.is_finite() || relation.weight < 0.0 {
                return Err("Invalid relation weight".into());
            }
            if !self.states.contains_key(&relation.from) || !self.states.contains_key(&relation.to)
            {
                return Err("Relation references an unknown state".into());
            }
            if relation.weight > 0.0 {
                neighbors
                    .entry(relation.from.as_str())
                    .or_default()
                    .push(relation.to.as_str());
            }
        }

        for next in neighbors.values_mut() {
            next.sort_unstable();
            next.dedup();
        }

        let mut reached: BTreeMap<String, usize> = BTreeMap::new();
        let mut queue = VecDeque::new();
        let mut bridges = Vec::new();
        let mut truncated = false;

        reached.insert(anchor.to_string(), 0);
        queue.push_back((anchor.to_string(), 0usize));

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            if let Some(next) = neighbors.get(current.as_str()) {
                for target in next {
                    if reached.contains_key(*target) {
                        continue;
                    }

                    if reached.len() >= max_states {
                        truncated = true;
                        break;
                    }

                    let next_depth = depth + 1;
                    reached.insert((*target).to_string(), next_depth);

                    // A runtime bridge opens a deeper region.
                    // It is not a separate persisted relation type.
                    if next_depth >= 2 {
                        bridges.push(ActivationBridge {
                            from: current.clone(),
                            to: (*target).to_string(),
                        });
                    }

                    queue.push_back(((*target).to_string(), next_depth));
                }
            }

            if truncated {
                break;
            }
        }

        let mut states: Vec<ActivatedState> = reached
            .into_iter()
            .map(|(token_id, depth)| ActivatedState {
                token_id,
                depth,
                tier: match depth {
                    0 | 1 => ActivationTier::L1,
                    2 => ActivationTier::L2,
                    _ => ActivationTier::L3,
                },
            })
            .collect();

        states.sort_by(|a, b| {
            a.depth
                .cmp(&b.depth)
                .then_with(|| a.token_id.cmp(&b.token_id))
        });

        Ok(TaskActivationView {
            anchor: anchor.to_string(),
            states,
            bridges,
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

        for id in ["task_a", "task_b", "step_x", "step_y", "shared_api"] {
            memory
                .insert(StateToken {
                    id: id.to_string(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        memory.reinforce("task_a", "shared_api", 0.8).unwrap();

        memory.reinforce("task_b", "step_x", 0.8).unwrap();
        memory.reinforce("step_x", "step_y", 0.8).unwrap();
        memory.reinforce("step_y", "shared_api", 0.8).unwrap();

        memory
    }

    fn tier(view: &TaskActivationView, id: &str) -> Option<ActivationTier> {
        view.states
            .iter()
            .find(|state| state.token_id == id)
            .map(|state| state.tier)
    }

    #[test]
    fn shared_token_changes_tier_without_duplication() {
        let memory = fixture();
        let before = serde_json::to_value(&memory).unwrap();

        let a = memory
            .learning_b_activation_preview("task_a", 4, 20)
            .unwrap();
        let b = memory
            .learning_b_activation_preview("task_b", 4, 20)
            .unwrap();

        assert_eq!(tier(&a, "shared_api"), Some(ActivationTier::L1));
        assert_eq!(tier(&b, "shared_api"), Some(ActivationTier::L3));

        assert_eq!(memory.states.len(), 5);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn deeper_connections_form_runtime_bridges() {
        let memory = fixture();
        let view = memory
            .learning_b_activation_preview("task_b", 4, 20)
            .unwrap();

        assert_eq!(tier(&view, "step_x"), Some(ActivationTier::L1));
        assert_eq!(tier(&view, "step_y"), Some(ActivationTier::L2));
        assert_eq!(tier(&view, "shared_api"), Some(ActivationTier::L3));

        assert!(view
            .bridges
            .iter()
            .any(|edge| edge.from == "step_y" && edge.to == "shared_api"));
        assert!(!view.truncated);
    }

    #[test]
    fn bounded_and_deterministic() {
        let memory = fixture();

        let first = memory
            .learning_b_activation_preview("task_b", 4, 2)
            .unwrap();
        let second = memory
            .learning_b_activation_preview("task_b", 4, 2)
            .unwrap();

        assert_eq!(first, second);
        assert!(first.truncated);
        assert_eq!(first.states.len(), 2);

        assert!(memory
            .learning_b_activation_preview("missing", 4, 20)
            .is_err());
        assert!(memory
            .learning_b_activation_preview("task_b", 4, 0)
            .is_err());
    }
}
