use crate::{
    retention::{RecallOutcome, RetentionEvent},
    token_architecture::inertia_preview,
    Memory,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetentionPolicy {
    pub minimum_gap_ms: i64,
    pub adjustment_rate: f32,
}

impl RetentionPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.minimum_gap_ms <= 0 {
            return Err("Minimum retention gap must be positive".into());
        }
        if !self.adjustment_rate.is_finite() || !(0.0..=1.0).contains(&self.adjustment_rate) {
            return Err("Invalid retention adjustment rate".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetentionCommit {
    pub token_id: String,
    pub event_id: String,
    pub gap_ms: Option<i64>,
    pub qualified: bool,
    pub estimate_before: f32,
    pub estimate_after: f32,
    pub applied_inertia: f32,
    pub memory_changed: bool,
}

impl Memory {
    /// Explicit experimental estimate, not verified recall probability.
    pub fn retention_estimate(&self, id: &str) -> Option<f32> {
        if !self.states.contains_key(id) {
            return None;
        }
        self.retention_estimates.get(id).copied()
    }

    /// Set an explicit initial estimate.
    /// No implicit score is derived from repetition.
    pub fn set_retention_estimate(&mut self, id: &str, estimate: f32) -> Result<(), String> {
        if !self.states.contains_key(id) {
            return Err("Unknown token".into());
        }
        if !estimate.is_finite() || !(0.0..=1.0).contains(&estimate) {
            return Err("Invalid retention estimate".into());
        }

        self.retention_estimates.insert(id.into(), estimate);
        Ok(())
    }

    /// Record a verified-by-caller recall outcome and optionally
    /// adjust the experimental retention estimate.
    ///
    /// The first event establishes a time reference only.
    /// Short gaps are recorded but do not adjust the estimate.
    /// No wall-clock trigger, activation change or file write.
    pub fn apply_retention_event(
        &mut self,
        event: RetentionEvent,
        policy: RetentionPolicy,
    ) -> Result<RetentionCommit, String> {
        policy.validate()?;

        let before = self
            .retention_estimate(&event.token_id)
            .ok_or("Token requires an explicit retention estimate")?;

        let inertia = self.token_inertia(&event.token_id).ok_or("Unknown token")?;

        inertia.validate()?;

        let previous = self
            .retention_events_for(&event.token_id)
            .and_then(|events| events.last());

        let gap = previous
            .map(|prior| {
                event
                    .attempted_at_unix_ms
                    .checked_sub(prior.attempted_at_unix_ms)
                    .ok_or("Retention time interval overflow")
            })
            .transpose()?;

        if gap.is_some_and(|value| value < 0) {
            return Err("Retention event time is out of order".into());
        }

        let qualified = gap.is_some_and(|value| value >= policy.minimum_gap_ms);

        let after = if qualified {
            let goal = match event.outcome {
                RecallOutcome::Success => 1.0_f32,
                RecallOutcome::Failure => 0.0_f32,
            };

            let target = before + (goal - before) * policy.adjustment_rate;

            inertia_preview(before, target, inertia.retention)?.clamp(0.0, 1.0)
        } else {
            before
        };

        let token_id = event.token_id.clone();
        let event_id = event.id.clone();

        // Record the event only after the calculations are valid.
        // The existing API checks unique IDs and chronological order.
        self.record_retention_event(event)?;

        self.retention_estimates.insert(token_id.clone(), after);

        Ok(RetentionCommit {
            token_id,
            event_id,
            gap_ms: gap,
            qualified,
            estimate_before: before,
            estimate_after: after,
            applied_inertia: inertia.retention,
            memory_changed: after != before,
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

    const ID: &str = "fd8295a1-f136-4fa1-83b3-9143913dfaaa";

    fn fixture() -> Memory {
        let mut m = Memory::new();

        m.insert(StateToken {
            id: "door-memory".into(),
            vector: vec![1.0, 0.0],
        })
        .unwrap();

        m.set_token_kind("door-memory", TokenKind::Memory).unwrap();

        m.set_token_inertia(
            "door-memory",
            TokenInertia {
                activation: 0.2,
                stability: 0.6,
                retention: 0.5,
            },
        )
        .unwrap();

        m.set_retention_estimate("door-memory", 0.4).unwrap();

        m
    }

    fn policy() -> RetentionPolicy {
        RetentionPolicy {
            minimum_gap_ms: 1000,
            adjustment_rate: 0.5,
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
    fn first_event_only_establishes_reference() {
        let mut m = fixture();

        let result = m
            .apply_retention_event(event("r1", 100, RecallOutcome::Success), policy())
            .unwrap();

        assert!(!result.qualified);
        assert_eq!(result.gap_ms, None);
        assert_eq!(result.estimate_after, 0.4);
    }

    #[test]
    fn immediate_repetition_does_not_increase_estimate() {
        let mut m = fixture();

        m.apply_retention_event(event("r1", 100, RecallOutcome::Success), policy())
            .unwrap();

        let result = m
            .apply_retention_event(event("r2", 101, RecallOutcome::Success), policy())
            .unwrap();

        assert_eq!(result.gap_ms, Some(1));
        assert!(!result.qualified);
        assert_eq!(m.retention_estimate("door-memory"), Some(0.4));
        assert_eq!(m.retention_events_for("door-memory").unwrap().len(), 2);
    }

    #[test]
    fn delayed_success_respects_retention_inertia() {
        let mut m = fixture();

        m.apply_retention_event(event("r1", 100, RecallOutcome::Success), policy())
            .unwrap();

        let result = m
            .apply_retention_event(event("r2", 2100, RecallOutcome::Success), policy())
            .unwrap();

        // Before 0.4, target 0.7, inertia 0.5 => 0.55.
        assert!(result.qualified);
        assert!((result.estimate_after - 0.55).abs() < 1e-6);
        assert!(result.memory_changed);
    }

    #[test]
    fn delayed_failure_reduces_estimate() {
        let mut m = fixture();

        m.apply_retention_event(event("r1", 100, RecallOutcome::Success), policy())
            .unwrap();

        let result = m
            .apply_retention_event(event("r2", 2100, RecallOutcome::Failure), policy())
            .unwrap();

        // Before 0.4, target 0.2, inertia 0.5 => 0.3.
        assert!(result.qualified);
        assert!((result.estimate_after - 0.3).abs() < 1e-6);
    }

    #[test]
    fn invalid_event_does_not_change_memory() {
        let mut m = fixture();

        m.apply_retention_event(event("r1", 100, RecallOutcome::Success), policy())
            .unwrap();

        let before = serde_json::to_value(&m).unwrap();

        assert!(m
            .apply_retention_event(event("r2", 50, RecallOutcome::Success), policy(),)
            .is_err());

        assert_eq!(serde_json::to_value(&m).unwrap(), before);
    }

    #[test]
    fn estimate_survives_chimera_update_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{ID}.tme"));

        state_container::create(&path, ID, "QWENTEST", &fixture()).unwrap();

        state_container::update(&path, ID, "QWENTEST", |m| {
            for (id, time) in [("r1", 100), ("r2", 2100)] {
                m.apply_retention_event(event(id, time, RecallOutcome::Success), policy())
                    .map_err(std::io::Error::other)?;
            }
            Ok(())
        })
        .unwrap();

        let restored = state_container::load(&path, ID, "QWENTEST").unwrap();

        assert!((restored.retention_estimate("door-memory").unwrap() - 0.55).abs() < 1e-6);

        assert_eq!(
            restored.retention_events_for("door-memory").unwrap().len(),
            2
        );

        assert_eq!(restored.token_kind("door-memory"), Some(TokenKind::Memory));
    }
}
