use crate::Memory;
use serde::{Deserialize, Serialize};

/// Read-only consolidation assessment.
/// This does not authorize structural weight changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsolidationDisposition {
    ReviewRequired,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConsolidationPreview {
    pub token_id: String,
    pub activation: f32,
    pub retention_attempts: usize,
    pub retention_successes: usize,
    pub retention_failures: usize,
    pub long_gap_successes: usize,
    pub retention_claim_established: bool,
    pub semantic_stability_available: bool,
    pub episodic_significance_available: bool,
    pub disposition: ConsolidationDisposition,
    pub reasons: Vec<String>,
    pub memory_changed: bool,
}

impl Memory {
    /// Inspect independent observations without modifying memory.
    ///
    /// No retention reliability score is inferred.
    /// No preservation, decay or deletion is authorized.
    pub fn consolidation_preview(
        &self,
        token_id: &str,
        cutoff_unix_ms: i64,
        minimum_gap_ms: i64,
    ) -> Result<ConsolidationPreview, String> {
        let dynamics = self.dynamics(token_id).ok_or("Unknown state token")?;

        if !dynamics.activation.is_finite() || !(0.0..=1.0).contains(&dynamics.activation) {
            return Err("Invalid activation value".into());
        }

        let retention = self.analyze_retention_as_of(token_id, cutoff_unix_ms, minimum_gap_ms)?;

        let mut reasons = Vec::new();

        if retention.attempts == 0 {
            reasons.push("No observed recall attempts".into());
        } else {
            reasons.push("Recall history is descriptive, not calibrated retention".into());
        }

        if retention.successful_recall_after_gap_count > 0 {
            reasons.push("Successful recalls after separated attempts observed".into());
        }

        reasons.push("Semantic stability is not available as a validated dimension".into());
        reasons.push("Episodic significance is not available as a validated dimension".into());
        reasons.push("No structural action authorized without a validated policy".into());

        Ok(ConsolidationPreview {
            token_id: token_id.into(),
            activation: dynamics.activation,
            retention_attempts: retention.attempts,
            retention_successes: retention.successes,
            retention_failures: retention.failures,
            long_gap_successes: retention.successful_recall_after_gap_count,
            retention_claim_established: false,
            semantic_stability_available: false,
            episodic_significance_available: false,
            disposition: ConsolidationDisposition::ReviewRequired,
            reasons,
            memory_changed: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        retention::{RecallOutcome, RetentionEvent},
        StateToken,
    };

    fn fixture() -> Memory {
        let mut memory = Memory::new();
        memory
            .insert(StateToken {
                id: "event".into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        memory
    }

    #[test]
    fn missing_history_does_not_imply_forgetting() {
        let memory = fixture();
        let result = memory.consolidation_preview("event", 1000, 100).unwrap();

        assert_eq!(result.retention_attempts, 0);
        assert!(!result.retention_claim_established);
        assert_eq!(result.disposition, ConsolidationDisposition::ReviewRequired);
    }

    #[test]
    fn long_gap_success_does_not_fabricate_stability() {
        let mut memory = fixture();

        for (id, timestamp) in [("r1", 10), ("r2", 1000)] {
            memory
                .record_retention_event(RetentionEvent {
                    id: id.into(),
                    token_id: "event".into(),
                    attempted_at_unix_ms: timestamp,
                    outcome: RecallOutcome::Success,
                    reference: format!("test/{id}"),
                })
                .unwrap();
        }

        let result = memory.consolidation_preview("event", 2000, 100).unwrap();

        assert_eq!(result.long_gap_successes, 1);
        assert!(!result.retention_claim_established);
    }

    #[test]
    fn preview_is_deterministic_and_read_only() {
        let mut memory = fixture();
        memory.set_token_dynamics("event", 0.2, 0.9).unwrap();

        let before = serde_json::to_value(&memory).unwrap();
        let first = memory.consolidation_preview("event", 1000, 100).unwrap();
        let second = memory.consolidation_preview("event", 1000, 100).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.activation, 0.2);
        assert!(!first.memory_changed);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn invalid_input_is_rejected() {
        let memory = fixture();
        assert!(memory.consolidation_preview("missing", 1000, 100).is_err());
        assert!(memory.consolidation_preview("event", 1000, 0).is_err());
    }
}
