use crate::Memory;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct BridgeResult {
    pub anchor: String,
    pub expanded: Vec<String>,
    pub discovered: usize,
    pub edges_examined: usize,
    pub budget_exhausted: bool,
}

impl Memory {
    /// Experimental best-first graph traversal.
    ///
    /// Budget counts expanded nodes, not all examined edges.
    /// Building adjacency still reads the entire relation map.
    /// No persisted state is modified.
    pub fn learning_b_bridge_preview(
        &self,
        anchor: &str,
        max_depth: usize,
        max_expansions: usize,
    ) -> Result<BridgeResult, String> {
        if !self.states.contains_key(anchor) {
            return Err("Unknown anchor".into());
        }
        if max_expansions == 0 {
            return Err("Expansion budget must be positive".into());
        }

        let mut adjacency: BTreeMap<&str, Vec<(&str, f32)>> = BTreeMap::new();

        for relation in self.relations.values() {
            if !relation.weight.is_finite() || !(0.0..=1.0).contains(&relation.weight) {
                return Err("Invalid relation weight".into());
            }
            if !self.states.contains_key(&relation.from) || !self.states.contains_key(&relation.to)
            {
                return Err("Unknown relation endpoint".into());
            }
            if relation.weight > 0.0 {
                adjacency
                    .entry(relation.from.as_str())
                    .or_default()
                    .push((&relation.to, relation.weight));
            }
        }

        // (id, depth, path score)
        let mut frontier = vec![(anchor.to_string(), 0usize, 1.0f32)];
        let mut expanded_set = BTreeSet::new();
        let mut expanded = Vec::new();
        let mut discovered = BTreeSet::new();
        discovered.insert(anchor.to_string());
        let mut edges_examined = 0usize;

        while !frontier.is_empty() && expanded.len() < max_expansions {
            frontier.sort_by(|a, b| {
                b.2.total_cmp(&a.2)
                    .then_with(|| a.1.cmp(&b.1))
                    .then_with(|| a.0.cmp(&b.0))
            });

            let (id, depth, score) = frontier.remove(0);

            if !expanded_set.insert(id.clone()) {
                continue;
            }

            expanded.push(id.clone());

            if depth >= max_depth {
                continue;
            }

            if let Some(edges) = adjacency.get(id.as_str()) {
                for (target, weight) in edges {
                    edges_examined += 1;

                    if expanded_set.contains(*target) {
                        continue;
                    }

                    let next_score = score * weight;
                    discovered.insert((*target).to_string());

                    // Update an existing frontier entry if a
                    // stronger route to the same token is found.
                    if let Some(old) = frontier.iter_mut().find(|entry| entry.0 == *target) {
                        if next_score > old.2 {
                            old.1 = depth + 1;
                            old.2 = next_score;
                        }
                    } else {
                        frontier.push(((*target).to_string(), depth + 1, next_score));
                    }
                }
            }
        }

        Ok(BridgeResult {
            anchor: anchor.to_string(),
            expanded,
            discovered: discovered.len(),
            edges_examined,
            budget_exhausted: !frontier.is_empty(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture(bridge_weight: f32) -> Memory {
        let mut memory = Memory::new();

        for id in ["task", "bridge", "target", "noise_a", "noise_b"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        memory.reinforce("task", "bridge", bridge_weight).unwrap();
        memory.reinforce("bridge", "target", 0.95).unwrap();
        memory.reinforce("task", "noise_a", 0.20).unwrap();
        memory.reinforce("task", "noise_b", 0.20).unwrap();

        memory
    }

    #[test]
    fn strong_bridge_reaches_second_level_early() {
        let memory = fixture(0.95);
        let result = memory.learning_b_bridge_preview("task", 2, 3).unwrap();

        assert_eq!(result.expanded, vec!["task", "bridge", "target"]);
    }

    #[test]
    fn weak_bridge_can_be_missed_under_budget() {
        let memory = fixture(0.05);
        let result = memory.learning_b_bridge_preview("task", 2, 3).unwrap();

        assert!(!result.expanded.contains(&"target".to_string()));
        assert!(result.budget_exhausted);
    }

    #[test]
    fn deterministic_read_only_and_bounded() {
        let memory = fixture(0.95);
        let before = serde_json::to_value(&memory).unwrap();

        let a = memory.learning_b_bridge_preview("task", 2, 3).unwrap();
        let b = memory.learning_b_bridge_preview("task", 2, 3).unwrap();

        assert_eq!(a, b);
        assert!(a.expanded.len() <= 3);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());

        assert!(memory.learning_b_bridge_preview("missing", 2, 3).is_err());
        assert!(memory.learning_b_bridge_preview("task", 2, 0).is_err());
    }
}
