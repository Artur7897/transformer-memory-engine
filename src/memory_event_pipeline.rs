use crate::{
    adaptive_commit::AdaptiveCommit,
    adaptive_dynamics::{AdaptiveEvent, AdaptivePolicy},
    retention::{RecallOutcome, RetentionEvent},
    retention_dynamics::{RetentionCommit, RetentionPolicy},
    Memory,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecallPolicy {
    pub retention: RetentionPolicy,
    pub activation: AdaptivePolicy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecallCommit {
    pub token_id: String,
    pub retention: RetentionCommit,
    pub activation: Option<AdaptiveCommit>,
}

/// Explicitly reported outcomes must be verified by the caller.
/// The TME itself cannot establish whether an LLM actually recalled
/// the correct information.
impl Memory {
    pub fn apply_recall(
        &mut self,
        event: RetentionEvent,
        policy: RecallPolicy,
    ) -> Result<RecallCommit, String> {
        policy.retention.validate()?;
        policy.activation.validate()?;

        let token_id = event.token_id.clone();
        let successful = event.outcome == RecallOutcome::Success;

        // Stage every change in an independent in-memory copy.
        // No partial mutation is exposed if any step fails.
        let mut staged = self.clone();

        let retention = staged.apply_retention_event(event, policy.retention)?;

        let activation = if successful {
            Some(staged.apply_adaptive_event(&token_id, AdaptiveEvent::Use, policy.activation)?)
        } else {
            None
        };

        staged.validate_state_metadata()?;
        staged.validate_retention_history()?;

        // Commit only after all operations succeeded.
        *self = staged;

        Ok(RecallCommit {
            token_id,
            retention,
            activation,
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

    const STATE_ID: &str = "e89b0c3d-1443-4f9a-9800-44c820607bdd";

    fn memory() -> Memory {
        let mut m = Memory::new();

        m.insert(StateToken {
            id: "door-memory".into(),
            vector: vec![1.0, 0.0],
        })
        .unwrap();

        m.set_token_kind("door-memory", TokenKind::Memory).unwrap();

        m.set_token_dynamics("door-memory", 0.2, 0.6).unwrap();

        m.set_token_inertia(
            "door-memory",
            TokenInertia {
                activation: 0.5,
                stability: 0.7,
                retention: 0.5,
            },
        )
        .unwrap();

        m.set_retention_estimate("door-memory", 0.4).unwrap();

        m
    }

    fn policy() -> RecallPolicy {
        RecallPolicy {
            retention: RetentionPolicy {
                minimum_gap_ms: 1000,
                adjustment_rate: 0.5,
            },
            activation: AdaptivePolicy {
                use_gain: 0.5,
                review_decay: 0.5,
            },
        }
    }

    fn event(id: &str, time: i64, outcome: RecallOutcome) -> RetentionEvent {
        RetentionEvent {
            id: id.into(),
            token_id: "door-memory".into(),
            attempted_at_unix_ms: time,
            outcome,
            reference: format!("test/{id}"),
        }
    }

    #[test]
    fn success_changes_activation_but_first_event_not_retention() {
        let mut m = memory();

        let report = m
            .apply_recall(event("r1", 100, RecallOutcome::Success), policy())
            .unwrap();

        assert!(!report.retention.qualified);
        assert!(report.activation.is_some());

        // Activation: 0.2 -> target 0.6 -> inertia 0.5 -> 0.4.
        assert!((m.dynamics("door-memory").unwrap().activation - 0.4).abs() < 1e-6);

        assert_eq!(m.retention_estimate("door-memory"), Some(0.4));
    }

    #[test]
    fn delayed_success_changes_two_dimensions() {
        let mut m = memory();

        m.apply_recall(event("r1", 100, RecallOutcome::Success), policy())
            .unwrap();

        let report = m
            .apply_recall(event("r2", 2100, RecallOutcome::Success), policy())
            .unwrap();

        assert!(report.retention.qualified);

        assert!((m.retention_estimate("door-memory").unwrap() - 0.55).abs() < 1e-6);

        assert!((m.dynamics("door-memory").unwrap().activation - 0.55).abs() < 1e-6);

        assert_eq!(m.dynamics("door-memory").unwrap().stability, 0.6);
    }

    #[test]
    fn failed_recall_changes_retention_not_activation() {
        let mut m = memory();

        m.apply_recall(event("r1", 100, RecallOutcome::Failure), policy())
            .unwrap();

        let report = m
            .apply_recall(event("r2", 2100, RecallOutcome::Failure), policy())
            .unwrap();

        assert!(report.activation.is_none());

        assert!((m.retention_estimate("door-memory").unwrap() - 0.3).abs() < 1e-6);

        assert_eq!(m.dynamics("door-memory").unwrap().activation, 0.2);
    }

    #[test]
    fn pipeline_is_atomic_and_survives_chimera_reload() {
        let mut m = memory();

        let before = serde_json::to_value(&m).unwrap();

        assert!(m
            .apply_recall(
                event("invalid", 100, RecallOutcome::Success),
                RecallPolicy {
                    retention: RetentionPolicy {
                        minimum_gap_ms: 0,
                        adjustment_rate: 0.5,
                    },
                    activation: policy().activation,
                },
            )
            .is_err());

        assert_eq!(serde_json::to_value(&m).unwrap(), before);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{STATE_ID}.tme"));

        state_container::create(&path, STATE_ID, "QWENTEST", &m).unwrap();

        state_container::update(&path, STATE_ID, "QWENTEST", |memory| {
            memory
                .apply_recall(event("r1", 100, RecallOutcome::Success), policy())
                .map(|_| ())
                .map_err(std::io::Error::other)
        })
        .unwrap();

        let restored = state_container::load(&path, STATE_ID, "QWENTEST").unwrap();

        assert!((restored.dynamics("door-memory").unwrap().activation - 0.4).abs() < 1e-6);

        assert_eq!(
            restored.retention_events_for("door-memory").unwrap().len(),
            1
        );
    }
}
