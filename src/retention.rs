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
    /// Verify stored retention history and construct an ID lookup index.
    /// The index is derived; the event history remains authoritative.
    fn build_retention_id_index(&self) -> Result<std::collections::BTreeSet<String>, String> {
        let mut ids = std::collections::BTreeSet::new();

        for (token_id, events) in &self.retention_events {
            if !self.states.contains_key(token_id) {
                return Err("Retention history refers to unknown token".into());
            }

            let mut previous_time: Option<i64> = None;

            for event in events {
                if event.token_id != *token_id {
                    return Err("Retention event token mismatch".into());
                }

                if event.id.trim().is_empty() || event.reference.trim().is_empty() {
                    return Err("Retention ID and reference are required".into());
                }

                if !ids.insert(event.id.clone()) {
                    return Err("Duplicate retention event ID".into());
                }

                if previous_time.is_some_and(|previous| event.attempted_at_unix_ms < previous) {
                    return Err("Retention history is out of order".into());
                }

                previous_time = Some(event.attempted_at_unix_ms);
            }
        }

        Ok(ids)
    }

    /// Prepare the derived ID index for a write-intensive session.
    ///
    /// Performs a full integrity check before installing the index.
    /// Does not alter persistent events or the JSON schema.
    pub fn prepare_retention_writes(&mut self) -> Result<(), String> {
        let index = self.build_retention_id_index()?;
        self.retention_id_index = Some(index);
        Ok(())
    }

    /// Validate persistent retention records without mutating memory.
    pub fn validate_retention_history(&self) -> Result<(), String> {
        self.build_retention_id_index().map(|_| ())
    }

    /// Read-only access; mutation must use record_retention_event.
    pub fn retention_events_for(
        &self,
        token_id: &str,
    ) -> Option<&[crate::retention::RetentionEvent]> {
        self.retention_events.get(token_id).map(Vec::as_slice)
    }

    /// Record an explicitly reported recall attempt.
    /// Does not change activation, stability or relation weights.
    pub fn record_retention_event(&mut self, event: RetentionEvent) -> Result<(), String> {
        if !self.states.contains_key(&event.token_id) {
            return Err("Unknown token".into());
        }

        if event.id.trim().is_empty() || event.reference.trim().is_empty() {
            return Err("Retention ID and reference are required".into());
        }

        // First write after deserialization rebuilds the derived index.
        // This also validates historical records before accepting writes.
        if self.retention_id_index.is_none() {
            self.retention_id_index = Some(self.build_retention_id_index()?);
        }

        if self
            .retention_id_index
            .as_ref()
            .expect("index initialized")
            .contains(&event.id)
        {
            return Err("Retention event ID already exists".into());
        }

        if let Some(previous) = self
            .retention_events
            .get(&event.token_id)
            .and_then(|events| events.last())
        {
            if event.attempted_at_unix_ms < previous.attempted_at_unix_ms {
                return Err("Retention event time is out of order".into());
            }
        }

        let id = event.id.clone();
        self.retention_events
            .entry(event.token_id.clone())
            .or_default()
            .push(event);

        self.retention_id_index
            .as_mut()
            .expect("index initialized")
            .insert(id);

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

#[cfg(test)]
mod index_tests {
    use super::*;
    use crate::StateToken;

    fn fixture() -> Memory {
        let mut memory = Memory::new();
        for id in ["a", "b"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0],
                })
                .unwrap();
        }
        memory
    }

    fn event(id: &str, token_id: &str, time: i64) -> RetentionEvent {
        RetentionEvent {
            id: id.into(),
            token_id: token_id.into(),
            attempted_at_unix_ms: time,
            outcome: RecallOutcome::Success,
            reference: format!("test/{id}"),
        }
    }

    #[test]
    fn index_rejects_duplicates_across_tokens() {
        let mut memory = fixture();
        memory
            .record_retention_event(event("same", "a", 100))
            .unwrap();
        assert!(memory
            .record_retention_event(event("same", "b", 200))
            .is_err());
        assert_eq!(memory.retention_events_for("b"), None);
    }

    #[test]
    fn explicit_preparation_preserves_events_and_uniqueness() {
        let mut memory = fixture();
        memory
            .record_retention_event(event("first", "a", 100))
            .unwrap();

        let json_before = serde_json::to_value(&memory).unwrap();

        memory.prepare_retention_writes().unwrap();

        assert_eq!(json_before, serde_json::to_value(&memory).unwrap());
        assert!(memory
            .record_retention_event(event("first", "b", 200))
            .is_err());

        memory
            .record_retention_event(event("second", "b", 200))
            .unwrap();
        assert_eq!(memory.retention_events_for("b").unwrap().len(), 1);
    }

    #[test]
    fn preparation_rebuilds_index_after_deserialization() {
        let mut memory = fixture();
        memory
            .record_retention_event(event("first", "a", 100))
            .unwrap();

        let json = serde_json::to_string(&memory).unwrap();
        let mut restored: Memory = serde_json::from_str(&json).unwrap();

        assert!(restored.retention_id_index.is_none());

        restored.prepare_retention_writes().unwrap();

        assert!(restored.retention_id_index.is_some());
        assert!(restored
            .record_retention_event(event("first", "b", 200))
            .is_err());
    }

    #[test]
    fn preparation_rejects_corrupt_history() {
        let mut memory = fixture();
        memory
            .record_retention_event(event("first", "a", 100))
            .unwrap();

        let mut json = serde_json::to_value(&memory).unwrap();
        json["retention_events"]["b"] = serde_json::json!([event("first", "b", 200)]);

        let mut restored: Memory = serde_json::from_value(json).unwrap();

        assert!(restored.prepare_retention_writes().is_err());
        assert!(restored.retention_id_index.is_none());
    }

    #[test]
    fn index_rebuilds_after_serde_roundtrip() {
        let mut memory = fixture();
        memory
            .record_retention_event(event("first", "a", 100))
            .unwrap();

        let serialized = serde_json::to_string(&memory).unwrap();
        assert!(!serialized.contains("retention_id_index"));

        let mut restored: Memory = serde_json::from_str(&serialized).unwrap();

        assert!(restored
            .record_retention_event(event("first", "b", 200))
            .is_err());

        restored
            .record_retention_event(event("second", "b", 200))
            .unwrap();
        assert_eq!(restored.retention_events_for("b").unwrap().len(), 1);
    }

    #[test]
    fn file_load_prepares_index_and_preserves_json() {
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT: AtomicU64 = AtomicU64::new(0);

        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tme-index-load-{}-{unique}.json",
            std::process::id()
        ));

        let mut original = fixture();
        original
            .record_retention_event(event("first", "a", 100))
            .unwrap();

        original.save(&path).unwrap();
        let loaded_result = Memory::load(&path);
        let _ = std::fs::remove_file(&path);

        let mut loaded = loaded_result.unwrap();

        assert!(loaded.retention_id_index.is_some());
        assert_eq!(
            serde_json::to_value(&original).unwrap(),
            serde_json::to_value(&loaded).unwrap()
        );

        assert!(loaded
            .record_retention_event(event("first", "b", 200))
            .is_err());

        loaded
            .record_retention_event(event("second", "b", 200))
            .unwrap();
    }

    #[test]
    fn raw_serde_keeps_lazy_index_behavior() {
        let mut original = fixture();
        original
            .record_retention_event(event("first", "a", 100))
            .unwrap();

        let json = serde_json::to_vec(&original).unwrap();
        let restored: Memory = serde_json::from_slice(&json).unwrap();

        assert!(restored.retention_id_index.is_none());
        assert_eq!(
            original.retention_as_of("a", 200).unwrap(),
            restored.retention_as_of("a", 200).unwrap()
        );
    }

    #[test]
    fn cloned_memory_preserves_index_correctness() {
        let mut memory = fixture();
        memory
            .record_retention_event(event("first", "a", 100))
            .unwrap();

        let mut clone = memory.clone();
        assert!(clone
            .record_retention_event(event("first", "b", 200))
            .is_err());

        clone
            .record_retention_event(event("second", "b", 200))
            .unwrap();

        assert!(memory.retention_events_for("b").is_none());
    }

    #[test]
    fn corrupted_deserialized_history_blocks_new_writes() {
        let mut memory = fixture();
        memory
            .record_retention_event(event("first", "a", 100))
            .unwrap();

        let mut value = serde_json::to_value(&memory).unwrap();
        let duplicate = serde_json::to_value(event("first", "b", 200)).unwrap();

        value["retention_events"]["b"] = serde_json::json!([duplicate]);

        let mut restored: Memory = serde_json::from_value(value).unwrap();

        assert!(restored.validate_retention_history().is_err());
        assert!(restored
            .record_retention_event(event("new", "b", 300))
            .is_err());
    }

    #[test]
    fn invalid_insertion_does_not_change_history() {
        let mut memory = fixture();
        memory
            .record_retention_event(event("first", "a", 100))
            .unwrap();
        let before = serde_json::to_value(&memory).unwrap();

        assert!(memory
            .record_retention_event(event("older", "a", 50))
            .is_err());

        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }
}
