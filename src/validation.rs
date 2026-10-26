use crate::{abstraction::AbstractionCandidate, Memory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCounts {
    pub observed: u64,
    pub missed: u64,
    pub evidence_ids: Vec<String>,
}

impl ValidationCounts {
    fn new() -> Self {
        Self {
            observed: 0,
            missed: 0,
            evidence_ids: Vec::new(),
        }
    }

    fn total(&self) -> u64 {
        self.observed + self.missed
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationDirection {
    SameDirection,
    OppositeDirection,
    NoDifference,
    InsufficientNewEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub candidate_id: String,
    pub discovery_cutoff_unix_ms: i64,
    pub validation_cutoff_unix_ms: i64,
    pub left: ValidationCounts,
    pub right: ValidationCounts,
    pub direction: ValidationDirection,
}

impl Memory {
    /// Descriptive out-of-discovery comparison.
    /// Does not establish statistical independence or confirmation.
    pub fn validate_abstraction_as_of(
        &self,
        candidate: &AbstractionCandidate,
        cutoff_unix_ms: i64,
    ) -> Result<ValidationReport, String> {
        if candidate.schema_version != crate::abstraction::ABSTRACTION_SCHEMA_VERSION {
            return Err("Unsupported candidate schema version".into());
        }
        if cutoff_unix_ms <= candidate.cutoff_unix_ms {
            return Err("Validation cutoff must follow discovery".into());
        }
        if candidate.left_value == candidate.right_value {
            return Err("Candidate groups must differ".into());
        }

        // Reject candidates with inconsistent discovery data.
        let discovery = self.abstraction_candidates_as_of(
            &candidate.from,
            &candidate.to,
            &candidate.context_key,
            candidate.cutoff_unix_ms,
        )?;
        if !discovery.contains(candidate) {
            return Err("Candidate does not match discovery evidence".into());
        }

        let excluded: BTreeSet<&str> = candidate.evidence_ids.iter().map(String::as_str).collect();

        let mut left = ValidationCounts::new();
        let mut right = ValidationCounts::new();

        for record in self.contextual_evidence.values() {
            if record.is_execution_adjacency() {
                continue;
            }

            if record.from != candidate.from
                || record.to != candidate.to
                || record.available_at_unix_ms <= candidate.cutoff_unix_ms
                || record.available_at_unix_ms > cutoff_unix_ms
                || excluded.contains(record.id.as_str())
            {
                continue;
            }

            let target = match record.context.get(&candidate.context_key) {
                Some(value) if value == &candidate.left_value => &mut left,
                Some(value) if value == &candidate.right_value => &mut right,
                _ => continue,
            };

            if record.occurred {
                target.observed = target
                    .observed
                    .checked_add(1)
                    .ok_or("Observed counter overflow")?;
            } else {
                target.missed = target
                    .missed
                    .checked_add(1)
                    .ok_or("Missed counter overflow")?;
            }
            target.evidence_ids.push(record.id.clone());
        }

        left.evidence_ids.sort();
        right.evidence_ids.sort();

        let direction = if left.total() == 0 || right.total() == 0 {
            ValidationDirection::InsufficientNewEvidence
        } else {
            let discovery_left = candidate.left_observed as u128
                * (candidate.right_observed + candidate.right_missed) as u128;
            let discovery_right = candidate.right_observed as u128
                * (candidate.left_observed + candidate.left_missed) as u128;

            let validation_left = left.observed as u128 * right.total() as u128;
            let validation_right = right.observed as u128 * left.total() as u128;

            let discovery_order = discovery_left.cmp(&discovery_right);
            let validation_order = validation_left.cmp(&validation_right);

            if validation_order.is_eq() {
                ValidationDirection::NoDifference
            } else if discovery_order == validation_order {
                ValidationDirection::SameDirection
            } else {
                ValidationDirection::OppositeDirection
            }
        };

        Ok(ValidationReport {
            candidate_id: candidate.id.clone(),
            discovery_cutoff_unix_ms: candidate.cutoff_unix_ms,
            validation_cutoff_unix_ms: cutoff_unix_ms,
            left,
            right,
            direction,
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

    fn add(memory: &mut Memory, id: &str, mode: &str, occurred: bool, available: i64) {
        memory
            .record_contextual_evidence(ContextualEvidenceRecord {
                schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                id: id.into(),
                from: "a".into(),
                to: "b".into(),
                occurred,
                observed_at_unix_ms: 0,
                available_at_unix_ms: available,
                source: EvidenceSource::ExternalObservation,
                reference: format!("test/{id}"),
                context: BTreeMap::from([("mode".into(), mode.into())]),
            })
            .unwrap();
    }

    fn fixture() -> (Memory, AbstractionCandidate) {
        let mut m = Memory::new();
        for id in ["a", "b"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        }

        add(&mut m, "d1", "left", true, 10);
        add(&mut m, "d2", "right", false, 20);
        let candidate = m
            .abstraction_candidates_as_of("a", "b", "mode", 30)
            .unwrap()
            .remove(0);

        (m, candidate)
    }

    #[test]
    fn same_direction_uses_only_new_evidence() {
        let (mut m, c) = fixture();
        add(&mut m, "v1", "left", true, 40);
        add(&mut m, "v2", "right", false, 50);
        add(&mut m, "future", "right", true, 100);

        let report = m.validate_abstraction_as_of(&c, 60).unwrap();
        assert_eq!(report.direction, ValidationDirection::SameDirection);
        assert_eq!(report.left.evidence_ids, vec!["v1"]);
        assert_eq!(report.right.evidence_ids, vec!["v2"]);
    }

    #[test]
    fn opposite_direction_is_reported() {
        let (mut m, c) = fixture();
        add(&mut m, "v1", "left", false, 40);
        add(&mut m, "v2", "right", true, 50);

        let report = m.validate_abstraction_as_of(&c, 60).unwrap();
        assert_eq!(report.direction, ValidationDirection::OppositeDirection);
    }

    #[test]
    fn insufficient_evidence_is_not_confirmation() {
        let (mut m, c) = fixture();
        add(&mut m, "v1", "left", true, 40);

        let report = m.validate_abstraction_as_of(&c, 60).unwrap();
        assert_eq!(
            report.direction,
            ValidationDirection::InsufficientNewEvidence
        );
    }

    #[test]
    fn deterministic_read_only_and_rejects_modified_candidate() {
        let (mut m, c) = fixture();
        add(&mut m, "v1", "left", true, 40);
        add(&mut m, "v2", "right", false, 50);

        let before = serde_json::to_value(&m).unwrap();
        let a = m.validate_abstraction_as_of(&c, 60).unwrap();
        let b = m.validate_abstraction_as_of(&c, 60).unwrap();
        assert_eq!(a, b);
        assert_eq!(before, serde_json::to_value(&m).unwrap());

        let mut altered = c.clone();
        altered.left_observed += 1;
        assert!(m.validate_abstraction_as_of(&altered, 60).is_err());
        assert!(m.validate_abstraction_as_of(&c, 30).is_err());
    }
}
