//! Persistent memory-level classification, separate from task-relative ActivationTier.
//! The level is derived from current memory token state and is not stored separately.
use crate::{token_architecture::TokenKind, Memory};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryLevel {
    L1,
    L2,
    L3,
}

impl Memory {
    /// Memory tokens only. Requires an explicit retention estimate.
    /// A Skill, State or Llm token has no memory level.
    pub fn memory_level(&self, id: &str) -> Option<MemoryLevel> {
        if self.token_kind(id)? != TokenKind::Memory {
            return None;
        }
        let stability = self.dynamics(id)?.stability;
        let retention = self.retention_estimate(id)?;
        if !stability.is_finite()
            || !retention.is_finite()
            || !(0.0..=1.0).contains(&stability)
            || !(0.0..=1.0).contains(&retention)
        {
            return None;
        }
        let avg = (f64::from(stability) + f64::from(retention)) / 2.0;
        Some(if avg >= 0.8 {
            MemoryLevel::L1
        } else if avg >= 0.4 {
            MemoryLevel::L2
        } else {
            MemoryLevel::L3
        })
    }

    /// Set experimental L3 inertia bonus. Zero disables L3-specific damping.
    /// This changes neither stored inertia nor stability/retention directly.
    pub fn set_memory_l3_inertia_bonus(&mut self, bonus: f32) -> Result<(), String> {
        if !bonus.is_finite() || !(0.0..=1.0).contains(&bonus) {
            return Err("Invalid L3 memory inertia bonus".into());
        }
        self.memory_l3_inertia_bonus = bonus;
        Ok(())
    }

    /// Effective inertia for one specified dimension, before a state update.
    /// L3 adds resistance *once*: base + (1 - base) * bonus.
    /// L1/L2 and skill tokens are unaffected.
    pub fn effective_memory_inertia(&self, id: &str, base: f32) -> Result<f32, String> {
        if !base.is_finite() || !(0.0..=1.0).contains(&base) {
            return Err("Invalid base inertia".into());
        }
        if !self.states.contains_key(id) {
            return Err("Unknown token".into());
        }
        let bonus = self.memory_l3_inertia_bonus;
        if !bonus.is_finite() || !(0.0..=1.0).contains(&bonus) {
            return Err("Invalid L3 memory inertia bonus".into());
        }
        Ok(if self.memory_level(id) == Some(MemoryLevel::L3) {
            base + (1.0 - base) * bonus
        } else {
            base
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        token_architecture::{TokenInertia, TokenKind},
        StateToken,
    };

    fn fixture(kind: TokenKind, stability: f32, retention: f32) -> Memory {
        let mut m = Memory::new();
        m.insert(StateToken {
            id: "concept".into(),
            vector: vec![1.0, 0.0],
        })
        .unwrap();
        m.set_token_kind("concept", kind).unwrap();
        m.set_token_dynamics("concept", 0.5, stability).unwrap();
        m.set_retention_estimate("concept", retention).unwrap();
        m
    }

    #[test]
    fn memory_levels_are_derived_from_mean() {
        assert_eq!(
            fixture(TokenKind::Memory, 0.50, 0.90).memory_level("concept"),
            Some(MemoryLevel::L2)
        );
        assert_eq!(
            fixture(TokenKind::Memory, 0.01, 0.75).memory_level("concept"),
            Some(MemoryLevel::L3)
        );
        assert_eq!(
            fixture(TokenKind::Memory, 0.90, 0.90).memory_level("concept"),
            Some(MemoryLevel::L1)
        );
        assert_eq!(
            fixture(TokenKind::Memory, 0.20, 0.20).memory_level("missing"),
            None
        );
        assert_eq!(
            fixture(TokenKind::Skill, 0.90, 0.90).memory_level("concept"),
            None
        );
    }

    #[test]
    fn anchor_is_independent_of_kind_and_survives_serialization() {
        let mut m = fixture(TokenKind::Skill, 0.80, 0.80);
        m.set_anchor("concept", true).unwrap();
        m.set_token_inertia(
            "concept",
            TokenInertia {
                activation: 0.2,
                stability: 0.8,
                retention: 0.9,
            },
        )
        .unwrap();
        let restored: Memory = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(restored.is_anchor("concept"), Some(true));
        assert_eq!(restored.token_kind("concept"), Some(TokenKind::Skill));
        assert_eq!(restored.token_inertia("concept").unwrap().retention, 0.9);
        assert_eq!(restored.memory_level("concept"), None);
    }

    #[test]
    fn l3_damping_is_separate_and_does_not_block_reactivation() {
        let mut m = fixture(TokenKind::Memory, 0.01, 0.75);
        m.set_memory_l3_inertia_bonus(0.5).unwrap();
        assert!((m.effective_memory_inertia("concept", 0.2).unwrap() - 0.6).abs() < 1e-5);
        m.set_token_dynamics("concept", 0.5, 0.5).unwrap();
        assert_eq!(m.memory_level("concept"), Some(MemoryLevel::L2));
        assert!((m.effective_memory_inertia("concept", 0.2).unwrap() - 0.2).abs() < 1e-5);
    }

    #[test]
    fn old_states_have_no_anchors_and_no_extra_l3_damping() {
        let m = fixture(TokenKind::Memory, 0.01, 0.75);
        let mut serialized = serde_json::to_value(&m).unwrap();
        serialized.as_object_mut().unwrap().remove("anchor_tokens");
        serialized
            .as_object_mut()
            .unwrap()
            .remove("memory_l3_inertia_bonus");
        let restored: Memory = serde_json::from_value(serialized).unwrap();
        assert_eq!(restored.is_anchor("concept"), Some(false));
        assert_eq!(restored.memory_l3_inertia_bonus, 0.0);
        assert!((restored.effective_memory_inertia("concept", 0.4).unwrap() - 0.4).abs() < 1e-5);
    }
}
