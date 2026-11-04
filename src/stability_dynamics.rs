use crate::{token_architecture::inertia_preview, Memory};

#[derive(Clone, Debug, PartialEq)]
pub struct StabilityCommit {
    pub token_id: String,
    pub before: f32,
    pub target: f32,
    pub after: f32,
    pub inertia: f32,
    pub memory_changed: bool,
}

impl Memory {
    /// Apply an explicitly supplied stability target.
    /// This does not infer truth, reliability or retention.
    /// Persistence remains the caller's responsibility.
    pub fn apply_stability_target(
        &mut self,
        id: &str,
        target: f32,
    ) -> Result<StabilityCommit, String> {
        if !target.is_finite() || !(0.0..=1.0).contains(&target) {
            return Err("Invalid stability target".into());
        }

        let dynamics = self.dynamics(id).ok_or("Unknown token")?;
        let inertia = self.token_inertia(id).ok_or("Unknown token")?;

        inertia.validate()?;

        let applied_inertia = self.effective_memory_inertia(id, inertia.stability)?;
        let after = inertia_preview(dynamics.stability, target, applied_inertia)?.clamp(0.0, 1.0);

        self.set_token_dynamics(id, dynamics.activation, after)?;

        Ok(StabilityCommit {
            token_id: id.to_owned(),
            before: dynamics.stability,
            target,
            after,
            inertia: applied_inertia,
            memory_changed: after != dynamics.stability,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        state_container,
        token_architecture::{TokenInertia, TokenKind},
        StateToken,
    };

    const STATE_ID: &str = "b429c7e0-946f-45d2-8c6a-d38575c67eb1";
    const SERIAL: &str = "QWENTEST";

    fn fixture(stability_inertia: f32) -> Memory {
        let mut memory = Memory::new();

        memory
            .insert(StateToken {
                id: "experience".into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();

        memory
            .set_token_kind("experience", TokenKind::Memory)
            .unwrap();

        memory.set_token_dynamics("experience", 0.4, 0.2).unwrap();

        memory
            .set_token_inertia(
                "experience",
                TokenInertia {
                    activation: 0.1,
                    stability: stability_inertia,
                    retention: 0.9,
                },
            )
            .unwrap();

        memory
    }

    #[test]
    fn stability_respects_individual_inertia() {
        let mut memory = fixture(0.75);

        let result = memory.apply_stability_target("experience", 0.8).unwrap();

        assert!((result.after - 0.35).abs() < 1e-6);
        assert!(result.memory_changed);

        let dynamics = memory.dynamics("experience").unwrap();
        assert!((dynamics.stability - 0.35).abs() < 1e-6);
        assert_eq!(dynamics.activation, 0.4);
    }

    #[test]
    fn maximum_inertia_blocks_stability_change() {
        let mut memory = fixture(1.0);

        let result = memory.apply_stability_target("experience", 0.8).unwrap();

        assert_eq!(result.after, 0.2);
        assert!(!result.memory_changed);
    }

    #[test]
    fn invalid_target_preserves_memory() {
        let mut memory = fixture(0.5);
        let before = serde_json::to_value(&memory).unwrap();

        assert!(memory
            .apply_stability_target("experience", f32::NAN)
            .is_err());

        assert_eq!(serde_json::to_value(&memory).unwrap(), before);
    }

    #[test]
    fn stability_and_token_metadata_survive_tme_update() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{STATE_ID}.tme"));

        let memory = fixture(0.75);

        state_container::create(&path, STATE_ID, SERIAL, &memory).unwrap();

        state_container::update(&path, STATE_ID, SERIAL, |memory| {
            memory
                .apply_stability_target("experience", 0.8)
                .map(|_| ())
                .map_err(std::io::Error::other)
        })
        .unwrap();

        let restored = state_container::load(&path, STATE_ID, SERIAL).unwrap();

        assert_eq!(restored.token_kind("experience"), Some(TokenKind::Memory));

        assert!((restored.dynamics("experience").unwrap().stability - 0.35).abs() < 1e-6);

        assert_eq!(restored.token_inertia("experience").unwrap().retention, 0.9);
    }
}
