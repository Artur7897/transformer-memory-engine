use crate::{experience::EvidenceSource, Memory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Repeated provenance does not establish dependency.
/// Distinct provenance does not establish independence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceGroup {
    pub source: String,
    pub reference: String,
    pub evidence_ids: Vec<String>,
    pub repeated_reference: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceQualityReport {
    pub from: String,
    pub to: String,
    pub cutoff_unix_ms: i64,
    pub total_records: usize,
    pub provenance_groups: Vec<ProvenanceGroup>,
    pub repeated_reference_groups: usize,
    pub records_in_repeated_groups: usize,
    pub independence_established: bool,
}

fn source_name(source: &EvidenceSource) -> &'static str {
    match source {
        EvidenceSource::ExternalObservation => "external_observation",
        EvidenceSource::VerifiedToolResult => "verified_tool_result",
        EvidenceSource::ModelStatement => "model_statement",
        EvidenceSource::HumanReport => "human_report",
    }
}

impl Memory {
    /// Descriptive provenance audit for a transition as of a cutoff.
    ///
    /// Equal source + reference is a potential shared-origin signal.
    /// No independent-sample count or truth score is inferred.
    pub fn analyze_evidence_quality_as_of(
        &self,
        from: &str,
        to: &str,
        cutoff_unix_ms: i64,
    ) -> Result<EvidenceQualityReport, String> {
        if !self.states.contains_key(from) || !self.states.contains_key(to) {
            return Err("Unknown transition state".into());
        }

        let mut groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
        let mut total_records = 0usize;

        for record in self.contextual_evidence.values() {
            if record.is_execution_adjacency() {
                continue;
            }

            if record.from != from
                || record.to != to
                || record.available_at_unix_ms > cutoff_unix_ms
            {
                continue;
            }

            total_records = total_records
                .checked_add(1)
                .ok_or("Record counter overflow")?;

            let key = (
                source_name(&record.source).to_owned(),
                record.reference.clone(),
            );

            groups.entry(key).or_default().push(record.id.clone());
        }

        let mut provenance_groups = Vec::new();
        let mut repeated_reference_groups = 0usize;
        let mut records_in_repeated_groups = 0usize;

        for ((source, reference), mut evidence_ids) in groups {
            evidence_ids.sort();

            let repeated_reference = evidence_ids.len() > 1;

            if repeated_reference {
                repeated_reference_groups += 1;
                records_in_repeated_groups += evidence_ids.len();
            }

            provenance_groups.push(ProvenanceGroup {
                source,
                reference,
                evidence_ids,
                repeated_reference,
            });
        }

        Ok(EvidenceQualityReport {
            from: from.into(),
            to: to.into(),
            cutoff_unix_ms,
            total_records,
            provenance_groups,
            repeated_reference_groups,
            records_in_repeated_groups,
            // Provenance metadata alone cannot prove independence.
            independence_established: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
        StateToken,
    };

    fn fixture() -> Memory {
        let mut memory = Memory::new();

        for id in ["input", "output"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        for (id, reference, available) in [
            ("e1", "dataset/a", 10),
            ("e2", "dataset/a", 20),
            ("e3", "dataset/b", 30),
            ("e4", "dataset/a", 100),
        ] {
            memory
                .record_contextual_evidence(ContextualEvidenceRecord {
                    schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                    id: id.into(),
                    from: "input".into(),
                    to: "output".into(),
                    occurred: true,
                    observed_at_unix_ms: 0,
                    available_at_unix_ms: available,
                    source: EvidenceSource::ExternalObservation,
                    reference: reference.into(),
                    context: BTreeMap::new(),
                })
                .unwrap();
        }

        memory
    }

    #[test]
    fn detects_reused_reference() {
        let memory = fixture();
        let report = memory
            .analyze_evidence_quality_as_of("input", "output", 40)
            .unwrap();

        assert_eq!(report.total_records, 3);
        assert_eq!(report.provenance_groups.len(), 2);
        assert_eq!(report.repeated_reference_groups, 1);
        assert_eq!(report.records_in_repeated_groups, 2);
        assert_eq!(report.provenance_groups[0].evidence_ids, vec!["e1", "e2"]);
        assert!(!report.independence_established);
    }

    #[test]
    fn excludes_evidence_unavailable_at_cutoff() {
        let memory = fixture();
        let report = memory
            .analyze_evidence_quality_as_of("input", "output", 15)
            .unwrap();

        assert_eq!(report.total_records, 1);
        assert_eq!(report.repeated_reference_groups, 0);
        assert!(!report.independence_established);
    }

    #[test]
    fn provenance_analysis_is_read_only_and_deterministic() {
        let memory = fixture();
        let before = serde_json::to_value(&memory).unwrap();

        let first = memory
            .analyze_evidence_quality_as_of("input", "output", 200)
            .unwrap();
        let second = memory
            .analyze_evidence_quality_as_of("input", "output", 200)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
        assert_eq!(first.records_in_repeated_groups, 3);
    }

    #[test]
    fn legacy_memory_json_remains_compatible() {
        let memory = fixture();
        let json = serde_json::to_string(&memory).unwrap();
        let restored: Memory = serde_json::from_str(&json).unwrap();

        assert_eq!(
            memory
                .analyze_evidence_quality_as_of("input", "output", 200)
                .unwrap(),
            restored
                .analyze_evidence_quality_as_of("input", "output", 200)
                .unwrap()
        );

        assert!(restored
            .analyze_evidence_quality_as_of("missing", "output", 200)
            .is_err());
    }
}
