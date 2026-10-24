use crate::{
    evidence_dependencies::{DependencyKind, DependencyLink},
    Memory,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalDependencyReport {
    pub evidence_id: String,
    pub cutoff_unix_ms: i64,
    pub known_links: Vec<DependencyLink>,
    pub unknown_time_links: Vec<DependencyLink>,
    pub future_links_excluded: usize,
    pub temporal_completeness_established: bool,
}

impl Memory {
    /// Record a dependency whose assertion was known at a documented time.
    /// Existing un-timed dependency records remain supported.
    pub fn record_timed_evidence_dependency(
        &mut self,
        child_id: &str,
        parent_id: &str,
        kind: DependencyKind,
        known_at_unix_ms: i64,
    ) -> Result<(), String> {
        let child = self
            .contextual_evidence
            .get(child_id)
            .ok_or("Unknown child evidence")?;

        let parent = self
            .contextual_evidence
            .get(parent_id)
            .ok_or("Unknown parent evidence")?;

        if known_at_unix_ms < child.available_at_unix_ms
            || known_at_unix_ms < parent.available_at_unix_ms
        {
            return Err("Dependency knowledge predates available evidence".into());
        }

        self.record_evidence_dependency(child_id, parent_id, kind)?;

        let links = self
            .evidence_dependencies
            .get_mut(child_id)
            .ok_or("Recorded dependency is missing")?;

        let link = links
            .iter_mut()
            .find(|link| link.parent_id == parent_id)
            .ok_or("Recorded parent is missing")?;

        link.known_at_unix_ms = Some(known_at_unix_ms);

        Ok(())
    }

    /// Direct links known by the historical cutoff.
    ///
    /// Unknown-time links are reported separately and are not
    /// considered historically established or independent.
    pub fn temporal_dependency_report_as_of(
        &self,
        evidence_id: &str,
        cutoff_unix_ms: i64,
    ) -> Result<TemporalDependencyReport, String> {
        self.validate_evidence_graph()?;

        let record = self
            .contextual_evidence
            .get(evidence_id)
            .ok_or("Unknown evidence ID")?;

        if record.available_at_unix_ms > cutoff_unix_ms {
            return Err("Evidence was not yet available".into());
        }

        let mut known_links = Vec::new();
        let mut unknown_time_links = Vec::new();
        let mut future_links_excluded = 0;

        if let Some(links) = self.evidence_dependencies.get(evidence_id) {
            for link in links {
                let parent = self
                    .contextual_evidence
                    .get(&link.parent_id)
                    .ok_or("Missing dependency parent")?;

                match link.known_at_unix_ms {
                    None => unknown_time_links.push(link.clone()),
                    Some(time) if time > cutoff_unix_ms => {
                        future_links_excluded += 1;
                    }
                    Some(time)
                        if time < record.available_at_unix_ms
                            || time < parent.available_at_unix_ms =>
                    {
                        return Err("Dependency predates available evidence".into());
                    }
                    Some(_) => known_links.push(link.clone()),
                }
            }
        }

        known_links.sort();
        unknown_time_links.sort();

        Ok(TemporalDependencyReport {
            evidence_id: evidence_id.into(),
            cutoff_unix_ms,
            known_links,
            unknown_time_links,
            future_links_excluded,
            // Missing links and unknown ancestry cannot be ruled out.
            temporal_completeness_established: false,
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

        for id in ["input", "output"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        }

        for id in ["a", "b", "c"] {
            m.record_contextual_evidence(ContextualEvidenceRecord {
                schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                id: id.into(),
                from: "input".into(),
                to: "output".into(),
                occurred: true,
                observed_at_unix_ms: 0,
                available_at_unix_ms: 10,
                source: EvidenceSource::ExternalObservation,
                reference: format!("test/{id}"),
                context: BTreeMap::new(),
            })
            .unwrap();
        }

        m
    }

    #[test]
    fn distinguishes_known_and_future_links() {
        let mut m = fixture();

        m.record_timed_evidence_dependency("c", "a", DependencyKind::DerivedFrom, 20)
            .unwrap();

        m.record_timed_evidence_dependency("c", "b", DependencyKind::DerivedFrom, 50)
            .unwrap();

        let report = m.temporal_dependency_report_as_of("c", 30).unwrap();

        assert_eq!(report.known_links.len(), 1);
        assert_eq!(report.known_links[0].parent_id, "a");
        assert_eq!(report.future_links_excluded, 1);
    }

    #[test]
    fn legacy_links_have_unknown_times() {
        let mut m = fixture();

        m.record_evidence_dependency("b", "a", DependencyKind::CopiedFrom)
            .unwrap();

        let report = m.temporal_dependency_report_as_of("b", 100).unwrap();

        assert!(report.known_links.is_empty());
        assert_eq!(report.unknown_time_links.len(), 1);
        assert!(!report.temporal_completeness_established);
    }

    #[test]
    fn rejects_invalid_knowledge_times() {
        let mut m = fixture();

        assert!(m
            .record_timed_evidence_dependency("b", "a", DependencyKind::CopiedFrom, 5)
            .is_err());

        assert!(m.evidence_dependencies.is_empty());
        assert!(m.temporal_dependency_report_as_of("b", 5).is_err());
    }

    #[test]
    fn persistence_is_backward_compatible() {
        let mut m = fixture();

        m.record_timed_evidence_dependency("b", "a", DependencyKind::CopiedFrom, 20)
            .unwrap();

        let json = serde_json::to_string(&m).unwrap();
        let restored: Memory = serde_json::from_str(&json).unwrap();

        assert_eq!(
            restored
                .temporal_dependency_report_as_of("b", 30)
                .unwrap()
                .known_links
                .len(),
            1
        );

        let old_link = serde_json::json!({
            "parent_id": "a",
            "kind": "CopiedFrom"
        });

        let legacy: DependencyLink = serde_json::from_value(old_link).unwrap();

        assert_eq!(legacy.known_at_unix_ms, None);
    }
}
