use crate::Memory;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecallOutcome {
    Success,
    Failure,
}

/// A reported recall test, not an automatically verified fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionEvent {
    pub id: String,
    pub token_id: String,
    pub attempted_at_unix_ms: i64,
    pub outcome: RecallOutcome,
    pub reference: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionObservation {
    pub event: RetentionEvent,
    pub gap_from_previous_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionReport {
    pub token_id: String,
    pub cutoff_unix_ms: i64,
    pub attempts: usize,
    pub successes: usize,
    pub failures: usize,
    pub observations: Vec<RetentionObservation>,
}

impl Memory {
    /// Record an explicitly reported recall attempt.
    /// Does not change activation, stability or relation weights.
    pub fn record_retention_event(&mut self, event: RetentionEvent) -> Result<(), String> {
        if !self.states.contains_key(&event.token_id) {
            return Err("Unknown token".into());
        }

        if event.id.trim().is_empty() || event.reference.trim().is_empty() {
            return Err("Retention ID and reference are required".into());
        }

        if self
            .retention_events
            .values()
            .flatten()
            .any(|existing| existing.id == event.id)
        {
            return Err("Retention event ID already exists".into());
        }

        let events = self
            .retention_events
            .entry(event.token_id.clone())
            .or_default();

        if let Some(previous) = events.last() {
            if event.attempted_at_unix_ms < previous.attempted_at_unix_ms {
                return Err("Retention event time is out of order".into());
            }
        }

        events.push(event);
        Ok(())
    }

    /// Historical recall report without looking into future attempts.
    pub fn retention_as_of(
        &self,
        token_id: &str,
        cutoff_unix_ms: i64,
    ) -> Result<RetentionReport, String> {
        if !self.states.contains_key(token_id) {
            return Err("Unknown token".into());
        }

        let mut observations = Vec::new();
        let mut previous: Option<i64> = None;
        let mut successes = 0usize;
        let mut failures = 0usize;

        if let Some(events) = self.retention_events.get(token_id) {
            for event in events {
                if event.attempted_at_unix_ms > cutoff_unix_ms {
                    continue;
                }

                let gap = previous
                    .map(|time| {
                        event
                            .attempted_at_unix_ms
                            .checked_sub(time)
                            .ok_or("Retention interval overflow")
                    })
                    .transpose()?;

                match event.outcome {
                    RecallOutcome::Success => successes += 1,
                    RecallOutcome::Failure => failures += 1,
                }

                observations.push(RetentionObservation {
                    event: event.clone(),
                    gap_from_previous_ms: gap,
                });
                previous = Some(event.attempted_at_unix_ms);
            }
        }

        Ok(RetentionReport {
            token_id: token_id.into(),
            cutoff_unix_ms,
            attempts: observations.len(),
            successes,
            failures,
            observations,
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

    fn event(id: &str, time: i64, outcome: RecallOutcome) -> RetentionEvent {
        RetentionEvent {
            id: id.into(),
            token_id: "concept".into(),
            attempted_at_unix_ms: time,
            outcome,
            reference: format!("test/{id}"),
        }
    }

    #[test]
    fn measures_distance_between_attempts() {
        let mut m = fixture();
        m.record_retention_event(event("r1", 1000, RecallOutcome::Success))
            .unwrap();
        m.record_retention_event(event("r2", 2000, RecallOutcome::Success))
            .unwrap();

        let report = m.retention_as_of("concept", 3000).unwrap();
        assert_eq!(report.successes, 2);
        assert_eq!(report.observations[0].gap_from_previous_ms, None);
        assert_eq!(report.observations[1].gap_from_previous_ms, Some(1000));
    }

    #[test]
    fn repeated_immediate_use_is_not_long_retention() {
        let mut m = fixture();
        for i in 0..100 {
            m.record_retention_event(event(&format!("r{i}"), 1000 + i, RecallOutcome::Success))
                .unwrap();
        }

        let report = m.retention_as_of("concept", 2000).unwrap();
        assert_eq!(report.successes, 100);
        assert!(report
            .observations
            .iter()
            .skip(1)
            .all(|item| item.gap_from_previous_ms == Some(1)));
    }

    #[test]
    fn cutoff_excludes_future_recall() {
        let mut m = fixture();
        m.record_retention_event(event("r1", 100, RecallOutcome::Failure))
            .unwrap();
        m.record_retention_event(event("r2", 500, RecallOutcome::Success))
            .unwrap();

        let report = m.retention_as_of("concept", 200).unwrap();
        assert_eq!(report.attempts, 1);
        assert_eq!(report.failures, 1);
        assert_eq!(report.successes, 0);
    }

    #[test]
    fn invalid_and_duplicate_events_are_rejected() {
        let mut m = fixture();
        m.record_retention_event(event("r1", 100, RecallOutcome::Success))
            .unwrap();

        assert!(m
            .record_retention_event(event("r1", 200, RecallOutcome::Success))
            .is_err());

        assert!(m
            .record_retention_event(event("r2", 50, RecallOutcome::Failure))
            .is_err());

        assert_eq!(m.retention_as_of("concept", 1000).unwrap().attempts, 1);
    }

    #[test]
    fn persistence_and_read_only_reporting() {
        let mut m = fixture();
        m.record_retention_event(event("r1", 100, RecallOutcome::Success))
            .unwrap();

        let json = serde_json::to_value(&m).unwrap();
        let restored: Memory = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(
            m.retention_as_of("concept", 200).unwrap(),
            restored.retention_as_of("concept", 200).unwrap()
        );

        let report = m.retention_as_of("concept", 200).unwrap();
        assert_eq!(report.attempts, 1);
        assert_eq!(serde_json::to_value(&m).unwrap(), json);

        let mut legacy = json;
        legacy.as_object_mut().unwrap().remove("retention_events");

        let older: Memory = serde_json::from_value(legacy).unwrap();
        assert_eq!(older.retention_as_of("concept", 200).unwrap().attempts, 0);
    }
}
