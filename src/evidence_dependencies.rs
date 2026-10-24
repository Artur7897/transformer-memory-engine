use crate::Memory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Explicitly asserted relationship between evidence records.
/// The assertion itself is not proof of the relationship.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DependencyKind {
    CopiedFrom,
    DerivedFrom,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DependencyLink {
    pub parent_id: String,
    pub kind: DependencyKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyReport {
    pub evidence_id: String,
    pub direct_dependencies: Vec<DependencyLink>,
    pub ancestor_ids: Vec<String>,
    pub explicitly_linked: bool,
    /// Absence of links does not establish independence.
    pub independence_established: bool,
}

impl Memory {
    fn evidence_exists(&self, id: &str) -> bool {
        self.contextual_evidence.contains_key(id)
    }

    /// Return whether `start` reaches `target` through parent links.
    fn reaches_evidence(&self, start: &str, target: &str) -> bool {
        let mut pending = vec![start.to_owned()];
        let mut visited = BTreeSet::new();

        while let Some(id) = pending.pop() {
            if id == target {
                return true;
            }
            if !visited.insert(id.clone()) {
                continue;
            }
            if let Some(links) = self.evidence_dependencies.get(&id) {
                for link in links {
                    pending.push(link.parent_id.clone());
                }
            }
        }

        false
    }

    /// Record one declared dependency: child -> parent.
    ///
    /// Reject missing evidence, self-links, duplicate parent links
    /// and directed cycles. Original evidence records are unchanged.
    pub fn record_evidence_dependency(
        &mut self,
        child_id: &str,
        parent_id: &str,
        kind: DependencyKind,
    ) -> Result<(), String> {
        if !self.evidence_exists(child_id) || !self.evidence_exists(parent_id) {
            return Err("Dependency references unknown evidence".into());
        }

        if child_id == parent_id {
            return Err("Evidence cannot depend on itself".into());
        }

        if self
            .evidence_dependencies
            .get(child_id)
            .is_some_and(|links| links.iter().any(|link| link.parent_id == parent_id))
        {
            return Err("Dependency already recorded for this parent".into());
        }

        if self.reaches_evidence(parent_id, child_id) {
            return Err("Dependency would introduce a cycle".into());
        }

        let links = self
            .evidence_dependencies
            .entry(child_id.to_owned())
            .or_default();

        links.push(DependencyLink {
            parent_id: parent_id.to_owned(),
            kind,
        });
        links.sort();
        Ok(())
    }

    /// Audit the explicitly documented ancestry of an event.
    ///
    /// Unknown dependencies remain unknown. Ancestors are not
    /// counted as independent observations.
    pub fn evidence_dependency_report(
        &self,
        evidence_id: &str,
    ) -> Result<DependencyReport, String> {
        if !self.evidence_exists(evidence_id) {
            return Err("Unknown evidence ID".into());
        }

        let direct_dependencies = self
            .evidence_dependencies
            .get(evidence_id)
            .cloned()
            .unwrap_or_default();

        let mut pending: Vec<String> = direct_dependencies
            .iter()
            .map(|link| link.parent_id.clone())
            .collect();

        let mut visited = BTreeSet::new();

        while let Some(id) = pending.pop() {
            if !self.evidence_exists(&id) {
                return Err("Dependency graph references missing evidence".into());
            }
            if id == evidence_id {
                return Err("Dependency graph contains a cycle".into());
            }
            if !visited.insert(id.clone()) {
                continue;
            }
            if let Some(links) = self.evidence_dependencies.get(&id) {
                for link in links {
                    pending.push(link.parent_id.clone());
                }
            }
        }

        Ok(DependencyReport {
            evidence_id: evidence_id.into(),
            explicitly_linked: !direct_dependencies.is_empty(),
            direct_dependencies,
            ancestor_ids: visited.into_iter().collect(),
            independence_established: false,
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

        for id in ["a", "b", "c", "d"] {
            m.record_contextual_evidence(ContextualEvidenceRecord {
                schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                id: id.into(),
                from: "input".into(),
                to: "output".into(),
                occurred: true,
                observed_at_unix_ms: 0,
                available_at_unix_ms: 10,
                source: EvidenceSource::ExternalObservation,
                reference: format!("record/{id}"),
                context: BTreeMap::new(),
            })
            .unwrap();
        }

        m
    }

    #[test]
    fn follows_multi_step_dependencies() {
        let mut m = fixture();

        m.record_evidence_dependency("b", "a", DependencyKind::CopiedFrom)
            .unwrap();
        m.record_evidence_dependency("c", "b", DependencyKind::DerivedFrom)
            .unwrap();

        let report = m.evidence_dependency_report("c").unwrap();

        assert_eq!(report.ancestor_ids, vec!["a", "b"]);
        assert_eq!(report.direct_dependencies.len(), 1);
        assert!(report.explicitly_linked);
        assert!(!report.independence_established);
    }

    #[test]
    fn rejects_cycles_and_self_links() {
        let mut m = fixture();

        m.record_evidence_dependency("b", "a", DependencyKind::CopiedFrom)
            .unwrap();
        m.record_evidence_dependency("c", "b", DependencyKind::DerivedFrom)
            .unwrap();

        assert!(m
            .record_evidence_dependency("a", "c", DependencyKind::DerivedFrom)
            .is_err());

        assert!(m
            .record_evidence_dependency("a", "a", DependencyKind::CopiedFrom)
            .is_err());

        assert_eq!(m.evidence_dependencies.len(), 2);
    }

    #[test]
    fn rejects_missing_or_duplicate_references() {
        let mut m = fixture();

        assert!(m
            .record_evidence_dependency("missing", "a", DependencyKind::CopiedFrom)
            .is_err());

        m.record_evidence_dependency("d", "a", DependencyKind::DerivedFrom)
            .unwrap();

        assert!(m
            .record_evidence_dependency("d", "a", DependencyKind::CopiedFrom)
            .is_err());
    }

    #[test]
    fn multi_parent_graph_is_deterministic() {
        let mut m = fixture();

        m.record_evidence_dependency("d", "b", DependencyKind::DerivedFrom)
            .unwrap();
        m.record_evidence_dependency("d", "a", DependencyKind::DerivedFrom)
            .unwrap();

        let first = m.evidence_dependency_report("d").unwrap();
        let second = m.evidence_dependency_report("d").unwrap();

        assert_eq!(first, second);
        assert_eq!(first.ancestor_ids, vec!["a", "b"]);
    }

    #[test]
    fn persistence_and_legacy_json_are_supported() {
        let mut m = fixture();

        m.record_evidence_dependency("b", "a", DependencyKind::CopiedFrom)
            .unwrap();

        let json = serde_json::to_string(&m).unwrap();
        let restored: Memory = serde_json::from_str(&json).unwrap();

        assert_eq!(
            restored
                .evidence_dependency_report("b")
                .unwrap()
                .ancestor_ids,
            vec!["a"]
        );

        let mut legacy: serde_json::Value = serde_json::from_str(&json).unwrap();

        legacy
            .as_object_mut()
            .unwrap()
            .remove("evidence_dependencies");

        let old: Memory = serde_json::from_value(legacy).unwrap();

        assert!(old.evidence_dependencies.is_empty());
        assert!(
            !old.evidence_dependency_report("a")
                .unwrap()
                .independence_established
        );
    }
}
