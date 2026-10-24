use crate::Memory;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphIntegrityReport {
    pub evidence_count: usize,
    pub dependency_count: usize,
    pub linked_evidence_count: usize,
    pub valid: bool,
}

/// Full structural validation of explicitly recorded evidence links.
/// Does not establish the truth of any dependency assertion.
impl Memory {
    pub fn validate_evidence_graph(&self) -> Result<GraphIntegrityReport, String> {
        let mut incoming: BTreeMap<String, usize> = self
            .contextual_evidence
            .keys()
            .map(|id| (id.clone(), 0))
            .collect();

        let mut dependency_count = 0usize;
        let mut linked_evidence_count = 0usize;

        for (child, links) in &self.evidence_dependencies {
            if !self.contextual_evidence.contains_key(child) {
                return Err(format!("Dependency child is missing: {child}"));
            }

            if !links.is_empty() {
                linked_evidence_count += 1;
            }

            let mut parents = BTreeSet::new();

            for link in links {
                if !self.contextual_evidence.contains_key(&link.parent_id) {
                    return Err(format!("Dependency parent is missing: {}", link.parent_id));
                }

                if let Some(known_at) = link.known_at_unix_ms {
                    let child_record = self
                        .contextual_evidence
                        .get(child)
                        .ok_or("Missing dependency child")?;
                    let parent_record = self
                        .contextual_evidence
                        .get(&link.parent_id)
                        .ok_or("Missing dependency parent")?;

                    if known_at < child_record.available_at_unix_ms
                        || known_at < parent_record.available_at_unix_ms
                    {
                        return Err(format!("Dependency knowledge predates evidence: {child}"));
                    }
                }

                if child == &link.parent_id {
                    return Err(format!("Self dependency: {child}"));
                }

                if !parents.insert(link.parent_id.clone()) {
                    return Err(format!("Duplicate dependency parent for: {child}"));
                }

                dependency_count = dependency_count
                    .checked_add(1)
                    .ok_or("Dependency counter overflow")?;

                let count = incoming
                    .get_mut(&link.parent_id)
                    .ok_or("Missing graph node")?;

                *count = count.checked_add(1).ok_or("Graph degree overflow")?;
            }
        }

        // Kahn traversal detects cycles without recursive traversal.
        let mut ready: BTreeSet<String> = incoming
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut visited = 0usize;

        while let Some(id) = ready.pop_first() {
            visited += 1;

            if let Some(links) = self.evidence_dependencies.get(&id) {
                for link in links {
                    let degree = incoming
                        .get_mut(&link.parent_id)
                        .ok_or("Missing graph node")?;

                    *degree -= 1;

                    if *degree == 0 {
                        ready.insert(link.parent_id.clone());
                    }
                }
            }
        }

        if visited != self.contextual_evidence.len() {
            return Err("Evidence dependency graph contains a cycle".into());
        }

        Ok(GraphIntegrityReport {
            evidence_count: self.contextual_evidence.len(),
            dependency_count,
            linked_evidence_count,
            valid: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
        evidence_dependencies::{DependencyKind, DependencyLink},
        experience::EvidenceSource,
        StateToken,
    };

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

    fn link(parent: &str) -> DependencyLink {
        DependencyLink {
            parent_id: parent.into(),
            kind: DependencyKind::DerivedFrom,
            known_at_unix_ms: None,
        }
    }

    #[test]
    fn validates_multi_step_graph() {
        let mut m = fixture();

        m.record_evidence_dependency("b", "a", DependencyKind::CopiedFrom)
            .unwrap();

        m.record_evidence_dependency("c", "b", DependencyKind::DerivedFrom)
            .unwrap();

        let report = m.validate_evidence_graph().unwrap();

        assert_eq!(report.evidence_count, 3);
        assert_eq!(report.dependency_count, 2);
        assert_eq!(report.linked_evidence_count, 2);
        assert!(report.valid);
    }

    #[test]
    fn rejects_loaded_cycle() {
        let mut m = fixture();

        m.evidence_dependencies.insert("a".into(), vec![link("b")]);
        m.evidence_dependencies.insert("b".into(), vec![link("c")]);
        m.evidence_dependencies.insert("c".into(), vec![link("a")]);

        assert!(m.validate_evidence_graph().is_err());
    }

    #[test]
    fn rejects_missing_nodes() {
        let mut m = fixture();

        m.evidence_dependencies
            .insert("a".into(), vec![link("missing")]);
        assert!(m.validate_evidence_graph().is_err());

        m.evidence_dependencies.clear();
        m.evidence_dependencies
            .insert("missing".into(), vec![link("a")]);
        assert!(m.validate_evidence_graph().is_err());
    }

    #[test]
    fn rejects_duplicate_and_self_dependencies() {
        let mut m = fixture();

        m.evidence_dependencies
            .insert("b".into(), vec![link("a"), link("a")]);
        assert!(m.validate_evidence_graph().is_err());

        m.evidence_dependencies.insert("b".into(), vec![link("b")]);
        assert!(m.validate_evidence_graph().is_err());
    }

    #[test]
    fn deterministic_read_only_and_legacy_compatible() {
        let m = fixture();
        let before = serde_json::to_value(&m).unwrap();

        let a = m.validate_evidence_graph().unwrap();
        let b = m.validate_evidence_graph().unwrap();

        assert_eq!(a, b);
        assert_eq!(before, serde_json::to_value(&m).unwrap());

        let mut legacy = before;
        legacy
            .as_object_mut()
            .unwrap()
            .remove("evidence_dependencies");

        let restored: Memory = serde_json::from_value(legacy).unwrap();

        assert!(restored.validate_evidence_graph().unwrap().valid);
    }
}

#[cfg(test)]
mod load_integrity_tests {
    use crate::{
        contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
        evidence_dependencies::{DependencyKind, DependencyLink},
        experience::EvidenceSource,
        Memory, StateToken,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> Memory {
        let mut m = Memory::new();
        for id in ["input", "output"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        }
        for id in ["a", "b"] {
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

    fn load_test(memory: &Memory) -> std::io::Result<Memory> {
        let serial = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tme-load-integrity-{}-{serial}.json",
            std::process::id()
        ));

        fs::write(&path, serde_json::to_vec(memory)?)?;
        let result = Memory::load(&path);
        let _ = fs::remove_file(&path);
        result
    }

    #[test]
    fn load_accepts_valid_graph() {
        let mut m = fixture();
        m.record_timed_evidence_dependency("b", "a", DependencyKind::CopiedFrom, 20)
            .unwrap();

        let restored = load_test(&m).unwrap();
        assert_eq!(
            restored
                .evidence_dependency_report("b")
                .unwrap()
                .ancestor_ids,
            vec!["a"]
        );
    }

    #[test]
    fn load_rejects_corrupt_graph() {
        let mut m = fixture();

        m.evidence_dependencies.insert(
            "a".into(),
            vec![DependencyLink {
                parent_id: "b".into(),
                kind: DependencyKind::DerivedFrom,
                known_at_unix_ms: None,
            }],
        );
        m.evidence_dependencies.insert(
            "b".into(),
            vec![DependencyLink {
                parent_id: "a".into(),
                kind: DependencyKind::DerivedFrom,
                known_at_unix_ms: None,
            }],
        );

        let error = load_test(&m).err().unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn load_rejects_impossible_dependency_time() {
        let mut m = fixture();

        m.evidence_dependencies.insert(
            "b".into(),
            vec![DependencyLink {
                parent_id: "a".into(),
                kind: DependencyKind::CopiedFrom,
                known_at_unix_ms: Some(5),
            }],
        );

        let error = load_test(&m).err().unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
