use crate::Memory;

fn valid_unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

impl Memory {
    /// Validate persisted token and relation metadata.
    ///
    /// This checks structural consistency, not semantic truth.
    /// Legacy states without these maps remain supported.
    pub fn validate_state_metadata(&self) -> Result<(), String> {
        for id in self.token_kinds.keys() {
            if !self.states.contains_key(id) {
                return Err(format!("Token kind references unknown token: {id}"));
            }
        }

        for (id, inertia) in &self.token_inertia {
            if !self.states.contains_key(id) {
                return Err(format!("Token inertia references unknown token: {id}"));
            }
            inertia.validate()?;
        }

        for (id, estimate) in &self.retention_estimates {
            if !self.states.contains_key(id) {
                return Err(format!("Retention estimate references unknown token: {id}"));
            }
            if !valid_unit(*estimate) {
                return Err(format!("Invalid retention estimate: {id}"));
            }
        }

        for (id, dynamics) in &self.token_dynamics {
            if !self.states.contains_key(id) {
                return Err(format!("Dynamics references unknown token: {id}"));
            }
            if !valid_unit(dynamics.activation)
                || !valid_unit(dynamics.stability)
                || dynamics.last_reflected_operations > dynamics.operations
            {
                return Err(format!("Invalid token dynamics: {id}"));
            }
        }

        for (key, inertia) in &self.relation_inertia {
            if !valid_unit(*inertia) {
                return Err(format!("Invalid relation inertia: {key}"));
            }
            if !self.relations.contains_key(key) {
                return Err(format!("Inertia references unknown relation: {key}"));
            }
        }

        for (key, relation) in &self.relations {
            if !self.states.contains_key(&relation.from) || !self.states.contains_key(&relation.to)
            {
                return Err(format!("Relation references unknown token: {key}"));
            }

            let expected = serde_json::to_string(&(relation.from.as_str(), relation.to.as_str()))
                .map_err(|e| e.to_string())?;

            if *key != expected {
                return Err(format!("Relation key mismatch: {key}"));
            }

            if !valid_unit(relation.weight) {
                return Err(format!("Invalid relation weight: {key}"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        token_architecture::{TokenInertia, TokenKind},
        StateToken,
    };

    fn fixture() -> Memory {
        let mut memory = Memory::new();

        for id in ["a", "b"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        memory
    }

    #[test]
    fn valid_metadata_passes() {
        let mut m = fixture();
        m.set_token_kind("a", TokenKind::Memory).unwrap();

        m.set_token_inertia(
            "a",
            TokenInertia {
                activation: 0.2,
                stability: 0.8,
                retention: 0.9,
            },
        )
        .unwrap();

        m.reinforce("a", "b", 0.5).unwrap();
        m.set_relation_inertia("a", "b", 0.7).unwrap();

        assert!(m.validate_state_metadata().is_ok());
    }

    #[test]
    fn legacy_metadata_defaults_pass() {
        let m = fixture();
        let mut json = serde_json::to_value(&m).unwrap();

        for field in ["token_kinds", "token_inertia", "relation_inertia"] {
            json.as_object_mut().unwrap().remove(field);
        }

        let restored: Memory = serde_json::from_value(json).unwrap();

        assert!(restored.validate_state_metadata().is_ok());
        assert_eq!(restored.token_kind("a"), Some(TokenKind::State));
    }

    #[test]
    fn orphan_token_metadata_is_rejected() {
        let mut m = fixture();
        m.token_kinds.insert("missing".into(), TokenKind::Skill);

        assert!(m.validate_state_metadata().is_err());

        m.token_kinds.clear();
        m.token_inertia
            .insert("missing".into(), TokenInertia::default());

        assert!(m.validate_state_metadata().is_err());
    }

    #[test]
    fn invalid_inertia_is_rejected() {
        let mut m = fixture();

        m.token_inertia.insert(
            "a".into(),
            TokenInertia {
                activation: 1.5,
                stability: 0.0,
                retention: 0.0,
            },
        );

        assert!(m.validate_state_metadata().is_err());

        m.token_inertia.clear();
        m.reinforce("a", "b", 0.5).unwrap();

        let key = serde_json::to_string(&("a", "b")).unwrap();

        m.relation_inertia.insert(key, -0.1);

        assert!(m.validate_state_metadata().is_err());
    }

    #[test]
    fn invalid_relations_and_dynamics_are_rejected() {
        let mut m = fixture();
        m.reinforce("a", "b", 0.5).unwrap();

        let key = serde_json::to_string(&("a", "b")).unwrap();

        m.relations.get_mut(&key).unwrap().weight = 1.5;
        assert!(m.validate_state_metadata().is_err());

        m.relations.get_mut(&key).unwrap().weight = 0.5;
        m.token_dynamics.get_mut("a").unwrap().activation = -0.1;

        assert!(m.validate_state_metadata().is_err());
    }
}
