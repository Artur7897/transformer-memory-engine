use crate::{
    adaptive_dynamics::{AdaptiveEvent, AdaptivePolicy},
    token_architecture::inertia_preview,
    Memory,
};

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveCommit {
    pub token_id: String,
    pub event: AdaptiveEvent,
    pub activation_before: f32,
    pub activation_target: f32,
    pub activation_after: f32,
    pub applied_inertia: f32,
    pub memory_changed: bool,
}

impl Memory {
    /// Apply one explicit adaptive event to in-memory activation.
    ///
    /// No automatic clock, operation, reflection or decay trigger.
    /// No file write: persistence remains the caller's responsibility.
    pub fn apply_adaptive_event(
        &mut self,
        id: &str,
        event: AdaptiveEvent,
        policy: AdaptivePolicy,
    ) -> Result<AdaptiveCommit, String> {
        let preview = self.adaptive_dynamics_preview(id, event, policy)?;

        let inertia = self.token_inertia(id).ok_or("Unknown token")?;

        inertia.validate()?;

        let after = inertia_preview(
            preview.activation_before,
            preview.activation_after,
            inertia.activation,
        )?
        .clamp(0.0, 1.0);

        // Validate and write only after all calculations succeeded.
        self.set_token_dynamics(id, after, preview.retention_stability)?;

        Ok(AdaptiveCommit {
            token_id: id.to_owned(),
            event,
            activation_before: preview.activation_before,
            activation_target: preview.activation_after,
            activation_after: after,
            applied_inertia: inertia.activation,
            memory_changed: after != preview.activation_before,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{token_architecture::TokenInertia, StateToken};

    fn fixture(inertia: f32) -> Memory {
        let mut m = Memory::new();

        m.insert(StateToken {
            id: "experience".into(),
            vector: vec![1.0, 0.0],
        })
        .unwrap();

        m.set_token_dynamics("experience", 0.2, 0.6).unwrap();

        m.set_token_inertia(
            "experience",
            TokenInertia {
                activation: inertia,
                stability: 0.0,
                retention: 0.0,
            },
        )
        .unwrap();

        m
    }

    fn policy() -> AdaptivePolicy {
        AdaptivePolicy {
            use_gain: 0.5,
            review_decay: 0.5,
        }
    }

    #[test]
    fn inertia_reduces_activation_change() {
        let mut m = fixture(0.75);

        let result = m
            .apply_adaptive_event("experience", AdaptiveEvent::Use, policy())
            .unwrap();

        assert!((result.activation_target - 0.6).abs() < 1e-6);
        assert!((result.activation_after - 0.3).abs() < 1e-6);
        assert!((m.dynamics("experience").unwrap().activation - 0.3).abs() < 1e-6);
    }

    #[test]
    fn maximum_inertia_prevents_change() {
        let mut m = fixture(1.0);

        let result = m
            .apply_adaptive_event("experience", AdaptiveEvent::Use, policy())
            .unwrap();

        assert_eq!(result.activation_after, 0.2);
        assert!(!result.memory_changed);
    }

    #[test]
    fn applied_activation_survives_serialization() {
        let mut m = fixture(0.5);

        m.apply_adaptive_event("experience", AdaptiveEvent::Use, policy())
            .unwrap();

        let serialized = serde_json::to_string(&m).unwrap();
        let restored: Memory = serde_json::from_str(&serialized).unwrap();

        assert_eq!(m.dynamics("experience"), restored.dynamics("experience"));

        assert_eq!(
            m.token_inertia("experience"),
            restored.token_inertia("experience")
        );
    }
}
