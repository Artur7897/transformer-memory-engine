use crate::{abstraction::AbstractionCandidate, Memory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyAuditReport {
    pub candidate_id: String,
    pub discovery_cutoff_unix_ms: i64,
    pub validation_cutoff_unix_ms: i64,
    pub discovery_evidence_ids: Vec<String>,
    pub validation_evidence_ids: Vec<String>,
    pub shared_ancestor_ids: Vec<String>,
    pub unknown_time_links: usize,
    pub future_links_excluded: usize,
    pub overlap_detected: bool,
    pub independence_established: bool,
}

#[derive(Default)]
struct Ancestry {
    ids: BTreeSet<String>,
    unknown_time_links: usize,
    future_links_excluded: usize,
}

impl Memory {
    fn ancestry_as_of(&self, seeds: &[String], cutoff: i64) -> Result<Ancestry, String> {
        let mut result = Ancestry::default();
        let mut pending = seeds.to_vec();

        while let Some(id) = pending.pop() {
            let record = self
                .contextual_evidence
                .get(&id)
                .ok_or("Unknown evidence in ancestry")?;

            if record.available_at_unix_ms > cutoff {
                return Err("Seed or ancestor unavailable at cutoff".into());
            }

            if !result.ids.insert(id.clone()) {
                continue;
            }

            for link in self.evidence_dependencies.get(&id).into_iter().flatten() {
                match link.known_at_unix_ms {
                    None => result.unknown_time_links += 1,
                    Some(time) if time > cutoff => {
                        result.future_links_excluded += 1;
                    }
                    Some(_) => pending.push(link.parent_id.clone()),
                }
            }
        }

        Ok(result)
    }

    /// Audit documented ancestry across discovery and validation.
    /// No-overlap does not prove independent observations.
    pub fn audit_abstraction_dependencies_as_of(
        &self,
        candidate: &AbstractionCandidate,
        validation_cutoff_unix_ms: i64,
    ) -> Result<DependencyAuditReport, String> {
        self.validate_evidence_graph()?;

        let validation = self.validate_abstraction_as_of(candidate, validation_cutoff_unix_ms)?;

        let mut validation_ids = validation.left.evidence_ids;
        validation_ids.extend(validation.right.evidence_ids);
        validation_ids.sort();
        validation_ids.dedup();

        let discovery = self.ancestry_as_of(&candidate.evidence_ids, candidate.cutoff_unix_ms)?;

        let validation_ancestry =
            self.ancestry_as_of(&validation_ids, validation_cutoff_unix_ms)?;

        let shared: Vec<String> = discovery
            .ids
            .intersection(&validation_ancestry.ids)
            .cloned()
            .collect();

        Ok(DependencyAuditReport {
            candidate_id: candidate.id.clone(),
            discovery_cutoff_unix_ms: candidate.cutoff_unix_ms,
            validation_cutoff_unix_ms,
            discovery_evidence_ids: candidate.evidence_ids.clone(),
            validation_evidence_ids: validation_ids,
            overlap_detected: !shared.is_empty(),
            shared_ancestor_ids: shared,
            unknown_time_links: discovery.unknown_time_links
                + validation_ancestry.unknown_time_links,
            future_links_excluded: discovery.future_links_excluded
                + validation_ancestry.future_links_excluded,
            independence_established: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
        evidence_dependencies::DependencyKind,
        experience::EvidenceSource,
        StateToken,
    };
    use std::collections::BTreeMap;

    fn fixture() -> (Memory, AbstractionCandidate) {
        let mut m = Memory::new();
        for id in ["input", "output"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        }

        for (id, mode, occurred, available) in [
            ("d1", "left", true, 10),
            ("d2", "right", false, 20),
            ("v1", "left", true, 40),
            ("v2", "right", false, 50),
        ] {
            m.record_contextual_evidence(ContextualEvidenceRecord {
                schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                id: id.into(),
                from: "input".into(),
                to: "output".into(),
                occurred,
                observed_at_unix_ms: 0,
                available_at_unix_ms: available,
                source: EvidenceSource::ExternalObservation,
                reference: format!("fixture/{id}"),
                context: BTreeMap::from([("mode".into(), mode.into())]),
            })
            .unwrap();
        }

        let candidate = m
            .abstraction_candidates_as_of("input", "output", "mode", 30)
            .unwrap()
            .remove(0);

        (m, candidate)
    }

    #[test]
    fn detects_documented_shared_origin() {
        let (mut m, candidate) = fixture();
        m.record_timed_evidence_dependency("v1", "d1", DependencyKind::DerivedFrom, 45)
            .unwrap();

        let report = m
            .audit_abstraction_dependencies_as_of(&candidate, 60)
            .unwrap();

        assert!(report.overlap_detected);
        assert_eq!(report.shared_ancestor_ids, vec!["d1"]);
        assert!(!report.independence_established);
    }

    #[test]
    fn future_link_is_not_used_retroactively() {
        let (mut m, candidate) = fixture();
        m.record_timed_evidence_dependency("v1", "d1", DependencyKind::DerivedFrom, 90)
            .unwrap();

        let report = m
            .audit_abstraction_dependencies_as_of(&candidate, 60)
            .unwrap();

        assert!(!report.overlap_detected);
        assert_eq!(report.future_links_excluded, 1);
        assert!(!report.independence_established);
    }

    #[test]
    fn unknown_time_remains_unknown() {
        let (mut m, candidate) = fixture();
        m.record_evidence_dependency("v1", "d1", DependencyKind::CopiedFrom)
            .unwrap();

        let report = m
            .audit_abstraction_dependencies_as_of(&candidate, 60)
            .unwrap();

        assert!(!report.overlap_detected);
        assert_eq!(report.unknown_time_links, 1);
        assert!(!report.independence_established);
    }

    #[test]
    fn deterministic_and_read_only() {
        let (m, candidate) = fixture();
        let before = serde_json::to_value(&m).unwrap();

        let first = m
            .audit_abstraction_dependencies_as_of(&candidate, 60)
            .unwrap();
        let second = m
            .audit_abstraction_dependencies_as_of(&candidate, 60)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(before, serde_json::to_value(&m).unwrap());
        assert!(!first.overlap_detected);
        assert!(!first.independence_established);
    }
}
