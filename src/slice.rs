use crate::Memory;
use serde::{Deserialize, Serialize};

/// Select an evidence subset without copying the underlying records.
/// Both observation-window bounds are inclusive.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub context_key: Option<String>,
    pub context_value: Option<String>,
    pub observed_from_unix_ms: Option<i64>,
    pub observed_to_unix_ms: Option<i64>,
    pub cutoff_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySlice {
    pub query: SliceQuery,
    pub evidence_ids: Vec<String>,
    pub record_count: usize,
}

impl Memory {
    /// Read-only selection of evidence available by the cutoff.
    /// Output IDs are ordered deterministically.
    pub fn slice_evidence(&self, query: &SliceQuery) -> Result<MemorySlice, String> {
        if query.context_key.is_some() != query.context_value.is_some() {
            return Err("Context key and value must be supplied together".into());
        }

        for value in [
            query.from.as_deref(),
            query.to.as_deref(),
            query.context_key.as_deref(),
            query.context_value.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if value.trim().is_empty() {
                return Err("Slice filter must not be empty".into());
            }
        }

        for state in [query.from.as_ref(), query.to.as_ref()]
            .into_iter()
            .flatten()
        {
            if !self.states.contains_key(state) {
                return Err(format!("Unknown slice state: {state}"));
            }
        }

        if let (Some(start), Some(end)) = (query.observed_from_unix_ms, query.observed_to_unix_ms) {
            if start > end {
                return Err("Observation window is reversed".into());
            }
        }

        let mut evidence_ids = Vec::new();

        for record in self.contextual_evidence.values() {
            if record.available_at_unix_ms > query.cutoff_unix_ms {
                continue;
            }

            if query.from.as_deref().is_some_and(|v| v != record.from) {
                continue;
            }
            if query.to.as_deref().is_some_and(|v| v != record.to) {
                continue;
            }

            if query
                .observed_from_unix_ms
                .is_some_and(|v| record.observed_at_unix_ms < v)
            {
                continue;
            }
            if query
                .observed_to_unix_ms
                .is_some_and(|v| record.observed_at_unix_ms > v)
            {
                continue;
            }

            if let (Some(key), Some(value)) =
                (query.context_key.as_deref(), query.context_value.as_deref())
            {
                if record.context.get(key).map(String::as_str) != Some(value) {
                    continue;
                }
            }

            evidence_ids.push(record.id.clone());
        }

        evidence_ids.sort();

        Ok(MemorySlice {
            query: query.clone(),
            record_count: evidence_ids.len(),
            evidence_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
        experience::EvidenceSource,
        StateToken,
    };
    use std::collections::BTreeMap;

    fn fixture() -> Memory {
        let mut m = Memory::new();

        for id in ["a", "b", "c"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        }

        for (id, from, to, mode, observed, available) in [
            ("e1", "a", "b", "manual", 10, 20),
            ("e2", "a", "b", "automatic", 20, 30),
            ("e3", "a", "b", "manual", 30, 100),
            ("e4", "b", "c", "manual", 40, 50),
        ] {
            m.record_contextual_evidence(ContextualEvidenceRecord {
                schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                id: id.into(),
                from: from.into(),
                to: to.into(),
                occurred: true,
                observed_at_unix_ms: observed,
                available_at_unix_ms: available,
                source: EvidenceSource::ExternalObservation,
                reference: format!("test/{id}"),
                context: BTreeMap::from([("mode".into(), mode.into())]),
            })
            .unwrap();
        }
        m
    }

    #[test]
    fn selects_transition_and_context() {
        let m = fixture();
        let query = SliceQuery {
            from: Some("a".into()),
            to: Some("b".into()),
            context_key: Some("mode".into()),
            context_value: Some("manual".into()),
            cutoff_unix_ms: 200,
            ..Default::default()
        };

        let slice = m.slice_evidence(&query).unwrap();
        assert_eq!(slice.evidence_ids, vec!["e1", "e3"]);
        assert_eq!(slice.record_count, 2);
    }

    #[test]
    fn availability_cutoff_prevents_lookahead() {
        let m = fixture();
        let query = SliceQuery {
            cutoff_unix_ms: 50,
            ..Default::default()
        };

        let slice = m.slice_evidence(&query).unwrap();
        assert_eq!(slice.evidence_ids, vec!["e1", "e2", "e4"]);
        assert!(!slice.evidence_ids.contains(&"e3".into()));
    }

    #[test]
    fn observation_window_is_inclusive() {
        let m = fixture();
        let query = SliceQuery {
            observed_from_unix_ms: Some(20),
            observed_to_unix_ms: Some(30),
            cutoff_unix_ms: 200,
            ..Default::default()
        };

        assert_eq!(
            m.slice_evidence(&query).unwrap().evidence_ids,
            vec!["e2", "e3"]
        );
    }

    #[test]
    fn rejects_invalid_filters() {
        let m = fixture();
        let mut query = SliceQuery {
            context_key: Some("mode".into()),
            cutoff_unix_ms: 200,
            ..Default::default()
        };
        assert!(m.slice_evidence(&query).is_err());

        query.context_value = Some("manual".into());
        query.observed_from_unix_ms = Some(40);
        query.observed_to_unix_ms = Some(20);
        assert!(m.slice_evidence(&query).is_err());

        query.observed_to_unix_ms = Some(50);
        query.from = Some("missing".into());
        assert!(m.slice_evidence(&query).is_err());
    }

    #[test]
    fn deterministic_read_only_and_overlapping_slices() {
        let m = fixture();
        let before = serde_json::to_value(&m).unwrap();

        let all = SliceQuery {
            cutoff_unix_ms: 200,
            ..Default::default()
        };
        let subset = SliceQuery {
            from: Some("a".into()),
            cutoff_unix_ms: 200,
            ..Default::default()
        };

        let first = m.slice_evidence(&all).unwrap();
        let second = m.slice_evidence(&all).unwrap();
        let smaller = m.slice_evidence(&subset).unwrap();

        assert_eq!(first, second);
        assert!(smaller.evidence_ids.contains(&"e1".into()));
        assert!(first.evidence_ids.contains(&"e1".into()));
        assert_eq!(before, serde_json::to_value(&m).unwrap());
    }
}
