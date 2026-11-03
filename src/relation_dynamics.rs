use crate::{token_architecture::inertia_preview, Memory};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum RelationEvent {
    Reinforce { delta: f32 },
    ReviewDecay { rate: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelationCommit {
    pub from: String,
    pub to: String,
    pub event: RelationEvent,
    pub weight_before: f32,
    pub weight_target: f32,
    pub weight_after: f32,
    pub inertia: f32,
    pub memory_changed: bool,
}

impl Memory {
    /// Apply one explicit relation event with individual inertia.
    ///
    /// Existing relation only. No automatic triggers.
    /// No file writes and no cycle-counter modifications.
    pub fn apply_relation_event(
        &mut self,
        from: &str,
        to: &str,
        event: RelationEvent,
    ) -> Result<RelationCommit, String> {
        let key = serde_json::to_string(&(from, to)).map_err(|e| e.to_string())?;

        let relation = self.relations.get(&key).ok_or("Unknown relation")?;

        if relation.from != from || relation.to != to {
            return Err("Relation identity mismatch".into());
        }

        if !self.states.contains_key(from) || !self.states.contains_key(to) {
            return Err("Relation references unknown token".into());
        }

        let before = relation.weight;

        if !before.is_finite() || !(0.0..=1.0).contains(&before) {
            return Err("Invalid stored relation weight".into());
        }

        let inertia = self
            .relation_inertia(from, to)
            .ok_or("Unknown relation inertia")?;

        if !inertia.is_finite() || !(0.0..=1.0).contains(&inertia) {
            return Err("Invalid stored relation inertia".into());
        }

        let target = match event {
            RelationEvent::Reinforce { delta } => {
                if !delta.is_finite() || delta <= 0.0 {
                    return Err("Invalid reinforcement delta".into());
                }
                (f64::from(before) + f64::from(delta)).min(1.0) as f32
            }
            RelationEvent::ReviewDecay { rate } => {
                if !rate.is_finite() || !(0.0..=1.0).contains(&rate) {
                    return Err("Invalid review decay rate".into());
                }
                before * (1.0 - rate)
            }
        };

        let after = inertia_preview(before, target, inertia)?.clamp(0.0, 1.0);

        // Mutation occurs only after all validation succeeds.
        self.relations
            .get_mut(&key)
            .ok_or("Relation disappeared")?
            .weight = after;

        Ok(RelationCommit {
            from: from.to_owned(),
            to: to.to_owned(),
            event,
            weight_before: before,
            weight_target: target,
            weight_after: after,
            inertia,
            memory_changed: after != before,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture(inertia: f32) -> Memory {
        let mut m = Memory::new();

        for id in ["a", "b"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        }

        m.reinforce("a", "b", 0.4).unwrap();
        m.set_relation_inertia("a", "b", inertia).unwrap();
        m
    }

    #[test]
    fn reinforcement_respects_inertia() {
        let mut m = fixture(0.8);

        let result = m
            .apply_relation_event("a", "b", RelationEvent::Reinforce { delta: 0.5 })
            .unwrap();

        assert!((result.weight_target - 0.9).abs() < 1e-6);
        assert!((result.weight_after - 0.5).abs() < 1e-6);
        assert!(result.memory_changed);
    }

    #[test]
    fn decay_respects_inertia() {
        let mut m = fixture(0.8);

        let result = m
            .apply_relation_event("a", "b", RelationEvent::ReviewDecay { rate: 0.5 })
            .unwrap();

        assert!((result.weight_target - 0.2).abs() < 1e-6);
        assert!((result.weight_after - 0.36).abs() < 1e-6);
    }

    #[test]
    fn maximum_inertia_blocks_both_directions() {
        let mut m = fixture(1.0);

        for event in [
            RelationEvent::Reinforce { delta: 0.5 },
            RelationEvent::ReviewDecay { rate: 0.5 },
        ] {
            let result = m.apply_relation_event("a", "b", event).unwrap();

            assert_eq!(result.weight_after, 0.4);
            assert!(!result.memory_changed);
        }
    }

    #[test]
    fn invalid_event_preserves_memory() {
        let mut m = fixture(0.5);
        let before = serde_json::to_value(&m).unwrap();

        assert!(m
            .apply_relation_event("a", "b", RelationEvent::Reinforce { delta: f32::NAN },)
            .is_err());

        assert_eq!(serde_json::to_value(&m).unwrap(), before);
    }

    #[test]
    fn applied_relation_and_inertia_survive_serialization() {
        let mut m = fixture(0.8);

        m.apply_relation_event("a", "b", RelationEvent::Reinforce { delta: 0.5 })
            .unwrap();

        let restored: Memory = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();

        assert_eq!(restored.relations, m.relations);
        assert_eq!(restored.relation_inertia("a", "b"), Some(0.8));
    }
}
