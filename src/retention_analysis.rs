use crate::{retention::RecallOutcome, Memory};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionAnalysis {
    pub token_id: String,
    pub cutoff_unix_ms: i64,
    pub attempts: usize,
    pub successes: usize,
    pub failures: usize,
    pub maximum_observed_gap_ms: Option<i64>,
    pub maximum_successful_gap_ms: Option<i64>,
    pub successful_recall_after_gap_count: usize,
    pub no_retention_claim: bool,
}

impl Memory {
    /// Descriptive retention analysis, without assigning a score.
    ///
    /// `minimum_gap_ms` defines which consecutive attempts count
    /// as temporally separated. It is a caller-selected threshold,
    /// not a universal biological memory constant.
    pub fn analyze_retention_as_of(
        &self,
        token_id: &str,
        cutoff_unix_ms: i64,
        minimum_gap_ms: i64,
    ) -> Result<RetentionAnalysis, String> {
        if minimum_gap_ms <= 0 {
            return Err("Minimum retention gap must be positive".into());
        }

        let report = self.retention_as_of(token_id, cutoff_unix_ms)?;

        let mut maximum_observed_gap_ms: Option<i64> = None;
        let mut maximum_successful_gap_ms: Option<i64> = None;
        let mut successful_recall_after_gap_count = 0usize;

        for observation in &report.observations {
            let Some(gap) = observation.gap_from_previous_ms else {
                continue;
            };

            maximum_observed_gap_ms = Some(maximum_observed_gap_ms.map_or(gap, |old| old.max(gap)));

            if observation.event.outcome == RecallOutcome::Success {
                maximum_successful_gap_ms =
                    Some(maximum_successful_gap_ms.map_or(gap, |old| old.max(gap)));

                if gap >= minimum_gap_ms {
                    successful_recall_after_gap_count += 1;
                }
            }
        }

        Ok(RetentionAnalysis {
            token_id: token_id.into(),
            cutoff_unix_ms,
            attempts: report.attempts,
            successes: report.successes,
            failures: report.failures,
            maximum_observed_gap_ms,
            maximum_successful_gap_ms,
            successful_recall_after_gap_count,
            no_retention_claim: true,
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

    fn fixture(times: &[i64]) -> Memory {
        let mut memory = Memory::new();

        memory
            .insert(StateToken {
                id: "concept".into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();

        for (index, time) in times.iter().enumerate() {
            memory
                .record_retention_event(RetentionEvent {
                    id: format!("recall-{index}"),
                    token_id: "concept".into(),
                    attempted_at_unix_ms: *time,
                    outcome: RecallOutcome::Success,
                    reference: format!("test/{index}"),
                })
                .unwrap();
        }

        memory
    }

    #[test]
    fn short_repetitions_do_not_count_as_long_gap() {
        let memory = fixture(&[100, 101, 102, 103]);
        let report = memory
            .analyze_retention_as_of("concept", 200, 1000)
            .unwrap();

        assert_eq!(report.successes, 4);
        assert_eq!(report.successful_recall_after_gap_count, 0);
        assert_eq!(report.maximum_successful_gap_ms, Some(1));
    }

    #[test]
    fn later_successful_recall_is_distinguished() {
        let memory = fixture(&[100, 200, 100_000]);
        let report = memory
            .analyze_retention_as_of("concept", 200_000, 1000)
            .unwrap();

        assert_eq!(report.successful_recall_after_gap_count, 1);
        assert_eq!(report.maximum_successful_gap_ms, Some(99_800));
    }

    #[test]
    fn future_recalls_are_excluded() {
        let memory = fixture(&[100, 200, 100_000]);
        let report = memory
            .analyze_retention_as_of("concept", 500, 1000)
            .unwrap();

        assert_eq!(report.attempts, 2);
        assert_eq!(report.maximum_successful_gap_ms, Some(100));
    }

    #[test]
    fn empty_history_and_invalid_threshold() {
        let memory = fixture(&[]);
        let report = memory.analyze_retention_as_of("concept", 500, 100).unwrap();

        assert_eq!(report.attempts, 0);
        assert_eq!(report.maximum_observed_gap_ms, None);
        assert!(report.no_retention_claim);

        assert!(memory.analyze_retention_as_of("concept", 500, 0).is_err());
    }

    #[test]
    fn deterministic_and_read_only() {
        let memory = fixture(&[100, 200, 100_000]);
        let before = serde_json::to_value(&memory).unwrap();

        let first = memory
            .analyze_retention_as_of("concept", 200_000, 1000)
            .unwrap();
        let second = memory
            .analyze_retention_as_of("concept", 200_000, 1000)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }
}
