use crate::Memory;
use serde::{Deserialize, Serialize};

/// Explicitly requested operation, never inferred from wall-clock time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdaptiveEvent {
    Use,
    ConsolidationReview,
}

/// Experimental policy. Rates are configuration, not learned facts.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdaptivePolicy {
    pub use_gain: f32,
    pub review_decay: f32,
}

impl AdaptivePolicy {
    pub fn validate(&self) -> Result<(), String> {
        for value in [self.use_gain, self.review_decay] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err("Adaptive policy rates must be finite in [0,1]".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdaptivePreview {
    pub token_id: String,
    pub event: AdaptiveEvent,
    pub activation_before: f32,
    pub activation_after: f32,
    pub retention_stability: f32,
    pub pending_operations: u64,
    pub memory_changed: bool,
}

impl Memory {
    /// Preview activation change for an explicit event.
    ///
    /// Stability is a caller-maintained experimental retention
    /// parameter, not semantic validity or evidence quality.
    /// Neither original data nor stored dynamics are modified.
    pub fn adaptive_dynamics_preview(
        &self,
        id: &str,
        event: AdaptiveEvent,
        policy: AdaptivePolicy,
    ) -> Result<AdaptivePreview, String> {
        policy.validate()?;

        let dynamics = self.dynamics(id).ok_or("Unknown state token")?;

        let activation = dynamics.activation;
        let stability = dynamics.stability;

        if !activation.is_finite()
            || !(0.0..=1.0).contains(&activation)
            || !stability.is_finite()
            || !(0.0..=1.0).contains(&stability)
        {
            return Err("Stored token dynamics are invalid".into());
        }

        let after = match event {
            AdaptiveEvent::Use => activation + (1.0 - activation) * policy.use_gain,
            AdaptiveEvent::ConsolidationReview => {
                activation * (1.0 - policy.review_decay * (1.0 - stability))
            }
        }
        .clamp(0.0, 1.0);

        Ok(AdaptivePreview {
            token_id: id.into(),
            event,
            activation_before: activation,
            activation_after: after,
            retention_stability: stability,
            pending_operations: dynamics.pending_operations(),
            memory_changed: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture() -> Memory {
        let mut memory = Memory::new();
        memory
            .insert(StateToken {
                id: "concept".into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        memory
    }

    fn policy() -> AdaptivePolicy {
        AdaptivePolicy {
            use_gain: 0.5,
            review_decay: 0.5,
        }
    }

    #[test]
    fn use_increases_activation() {
        let mut memory = fixture();
        memory.set_token_dynamics("concept", 0.4, 0.6).unwrap();

        let preview = memory
            .adaptive_dynamics_preview("concept", AdaptiveEvent::Use, policy())
            .unwrap();

        assert!((preview.activation_after - 0.7).abs() < 1e-6);
        assert!(!preview.memory_changed);
    }

    #[test]
    fn stability_protects_activation_at_review() {
        let mut memory = fixture();
        memory.set_token_dynamics("concept", 0.8, 0.9).unwrap();

        let preview = memory
            .adaptive_dynamics_preview("concept", AdaptiveEvent::ConsolidationReview, policy())
            .unwrap();

        assert!((preview.activation_after - 0.76).abs() < 1e-6);
        assert_eq!(preview.retention_stability, 0.9);
    }

    #[test]
    fn zero_review_rate_preserves_activation() {
        let mut memory = fixture();
        memory.set_token_dynamics("concept", 0.8, 0.0).unwrap();

        let preview = memory
            .adaptive_dynamics_preview(
                "concept",
                AdaptiveEvent::ConsolidationReview,
                AdaptivePolicy {
                    use_gain: 0.5,
                    review_decay: 0.0,
                },
            )
            .unwrap();

        assert_eq!(preview.activation_after, 0.8);
    }

    #[test]
    fn deterministic_and_read_only() {
        let mut memory = fixture();
        memory.record_token_operation("concept").unwrap();

        let before = serde_json::to_value(&memory).unwrap();

        let first = memory
            .adaptive_dynamics_preview("concept", AdaptiveEvent::Use, policy())
            .unwrap();
        let second = memory
            .adaptive_dynamics_preview("concept", AdaptiveEvent::Use, policy())
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.pending_operations, 1);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn invalid_configuration_and_state_are_rejected() {
        let memory = fixture();

        assert!(memory
            .adaptive_dynamics_preview("missing", AdaptiveEvent::Use, policy())
            .is_err());

        assert!(memory
            .adaptive_dynamics_preview(
                "concept",
                AdaptiveEvent::Use,
                AdaptivePolicy {
                    use_gain: f32::NAN,
                    review_decay: 0.5,
                },
            )
            .is_err());
    }
}
