use crate::{experience::EvidenceSource, Memory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CONTEXTUAL_EVIDENCE_SCHEMA_VERSION: u32 = 1;

/// An individual evaluated transition opportunity.
/// available_at_unix_ms is when the result could first
/// be known, including a completed observation window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextualEvidenceRecord {
    pub schema_version: u32,
    pub id: String,
    pub from: String,
    pub to: String,
    pub occurred: bool,
    pub observed_at_unix_ms: i64,
    pub available_at_unix_ms: i64,
    pub source: EvidenceSource,
    pub reference: String,
    pub context: BTreeMap<String, String>,
}

impl Memory {
    /// Store original evidence without changing aggregate counters.
    /// Repeated IDs cannot silently overwrite evidence.
    pub fn record_contextual_evidence(
        &mut self,
        record: ContextualEvidenceRecord,
    ) -> Result<(), String> {
        if record.schema_version != CONTEXTUAL_EVIDENCE_SCHEMA_VERSION {
            return Err("Unsupported contextual evidence schema".into());
        }

        if record.id.trim().is_empty()
            || record.from.trim().is_empty()
            || record.to.trim().is_empty()
            || record.reference.trim().is_empty()
        {
            return Err("Required evidence fields are empty".into());
        }

        if !self.states.contains_key(&record.from) || !self.states.contains_key(&record.to) {
            return Err("Contextual evidence references unknown state".into());
        }

        if record.available_at_unix_ms < record.observed_at_unix_ms {
            return Err("Result cannot be available before observation".into());
        }

        if record
            .context
            .iter()
            .any(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
        {
            return Err("Context keys and values must not be empty".into());
        }

        if self.contextual_evidence.contains_key(&record.id) {
            return Err("Contextual evidence ID already exists".into());
        }

        self.contextual_evidence.insert(record.id.clone(), record);
        Ok(())
    }

    /// Only events whose outcomes were available by cutoff.
    /// Results are ordered by BTreeMap record ID.
    pub fn contextual_evidence_as_of(&self, cutoff_unix_ms: i64) -> Vec<ContextualEvidenceRecord> {
        self.contextual_evidence
            .values()
            .filter(|record| record.available_at_unix_ms <= cutoff_unix_ms)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture() -> (Memory, ContextualEvidenceRecord) {
        let mut memory = Memory::new();

        for id in ["market", "response"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        let mut context = BTreeMap::new();
        context.insert("environment".into(), "variable".into());
        context.insert("timeframe".into(), "5m".into());

        let record = ContextualEvidenceRecord {
            schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
            id: "observation-001".into(),
            from: "market".into(),
            to: "response".into(),
            occurred: true,
            observed_at_unix_ms: 1_000_000,
            available_at_unix_ms: 1_300_000,
            source: EvidenceSource::ExternalObservation,
            reference: "synthetic/case-001".into(),
            context,
        };

        (memory, record)
    }

    #[test]
    fn persists_context_and_provenance() {
        let (mut memory, record) = fixture();
        memory.record_contextual_evidence(record.clone()).unwrap();

        let json = serde_json::to_string(&memory).unwrap();
        let restored: Memory = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.contextual_evidence["observation-001"], record);
        assert!(restored.transitions.is_empty());
    }

    #[test]
    fn hides_unavailable_results() {
        let (mut memory, record) = fixture();
        memory.record_contextual_evidence(record).unwrap();

        assert!(memory.contextual_evidence_as_of(1_299_999).is_empty());
        assert_eq!(memory.contextual_evidence_as_of(1_300_000).len(), 1);
    }

    #[test]
    fn prevents_overwrites_and_invalid_references() {
        let (mut memory, record) = fixture();
        memory.record_contextual_evidence(record.clone()).unwrap();

        assert!(memory.record_contextual_evidence(record.clone()).is_err());

        let mut invalid = record;
        invalid.id = "observation-002".into();
        invalid.to = "unknown".into();

        assert!(memory.record_contextual_evidence(invalid).is_err());
        assert_eq!(memory.contextual_evidence.len(), 1);
    }

    #[test]
    fn rejects_results_before_observation() {
        let (mut memory, mut record) = fixture();
        record.available_at_unix_ms = 999_999;

        assert!(memory.record_contextual_evidence(record).is_err());
        assert!(memory.contextual_evidence.is_empty());
    }

    #[test]
    fn legacy_memory_remains_readable() {
        let legacy = r#"{
            "states": {},
            "relations": {},
            "cycles": 0
        }"#;

        let restored: Memory = serde_json::from_str(legacy).unwrap();
        assert!(restored.contextual_evidence.is_empty());
    }
}
