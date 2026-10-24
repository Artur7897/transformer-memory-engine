use crate::{contextual_evidence::ContextualEvidenceRecord, Memory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Exact grouping of records without deleting or modifying evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionGroup {
    pub from: String,
    pub to: String,
    pub occurred: bool,
    pub source: String,
    pub reference: String,
    pub context: BTreeMap<String, String>,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionPreview {
    pub cutoff_unix_ms: i64,
    pub original_record_count: usize,
    pub group_count: usize,
    pub groups: Vec<CompressionGroup>,
    pub original_records_preserved: bool,
}

fn source_key(record: &ContextualEvidenceRecord) -> String {
    serde_json::to_string(&record.source).expect("EvidenceSource is serializable")
}

impl Memory {
    /// Deterministic, exact-match grouping of available evidence.
    ///
    /// This previews a possible compact index, not physical compression.
    /// All underlying evidence remains authoritative.
    pub fn compress_preview_as_of(&self, cutoff_unix_ms: i64) -> CompressionPreview {
        type GroupKey = (String, String, bool, String, String, Vec<(String, String)>);

        let mut grouped: BTreeMap<GroupKey, Vec<String>> = BTreeMap::new();
        let mut count = 0usize;

        for record in self.contextual_evidence.values() {
            if record.available_at_unix_ms > cutoff_unix_ms {
                continue;
            }

            count += 1;

            let key = (
                record.from.clone(),
                record.to.clone(),
                record.occurred,
                source_key(record),
                record.reference.clone(),
                record
                    .context
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            );

            grouped.entry(key).or_default().push(record.id.clone());
        }

        let groups = grouped
            .into_iter()
            .map(
                |((from, to, occurred, source, reference, context), mut evidence_ids)| {
                    evidence_ids.sort();
                    CompressionGroup {
                        from,
                        to,
                        occurred,
                        source,
                        reference,
                        context: context.into_iter().collect(),
                        evidence_ids,
                    }
                },
            )
            .collect::<Vec<_>>();

        CompressionPreview {
            cutoff_unix_ms,
            original_record_count: count,
            group_count: groups.len(),
            groups,
            original_records_preserved: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        contextual_evidence::CONTEXTUAL_EVIDENCE_SCHEMA_VERSION, experience::EvidenceSource,
        StateToken,
    };

    fn fixture() -> Memory {
        let mut m = Memory::new();

        for id in ["a", "b"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        }

        for (id, occurred, reference, available) in [
            ("e1", true, "shared", 10),
            ("e2", true, "shared", 20),
            ("e3", false, "shared", 30),
            ("e4", true, "other", 40),
            ("e5", true, "shared", 100),
        ] {
            m.record_contextual_evidence(ContextualEvidenceRecord {
                schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                id: id.into(),
                from: "a".into(),
                to: "b".into(),
                occurred,
                observed_at_unix_ms: 0,
                available_at_unix_ms: available,
                source: EvidenceSource::ExternalObservation,
                reference: reference.into(),
                context: BTreeMap::from([("mode".into(), "standard".into())]),
            })
            .unwrap();
        }
        m
    }

    #[test]
    fn groups_only_matching_evidence() {
        let report = fixture().compress_preview_as_of(50);

        assert_eq!(report.original_record_count, 4);
        assert_eq!(report.group_count, 3);
        assert!(report
            .groups
            .iter()
            .any(|group| { group.evidence_ids == vec!["e1", "e2"] }));
    }

    #[test]
    fn future_evidence_is_excluded() {
        let report = fixture().compress_preview_as_of(20);

        assert_eq!(report.original_record_count, 2);
        assert_eq!(report.group_count, 1);
        assert_eq!(report.groups[0].evidence_ids, vec!["e1", "e2"]);
    }

    #[test]
    fn compression_is_deterministic_and_read_only() {
        let m = fixture();
        let before = serde_json::to_value(&m).unwrap();

        let first = m.compress_preview_as_of(200);
        let second = m.compress_preview_as_of(200);

        assert_eq!(first, second);
        assert_eq!(serde_json::to_value(&m).unwrap(), before);
        assert!(first.original_records_preserved);
    }

    #[test]
    fn no_evidence_is_not_an_error() {
        let m = fixture();
        let report = m.compress_preview_as_of(0);

        assert_eq!(report.original_record_count, 0);
        assert_eq!(report.group_count, 0);
        assert!(report.groups.is_empty());
    }
}
