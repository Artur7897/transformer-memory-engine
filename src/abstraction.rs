use crate::Memory;
use serde::{Deserialize, Serialize};

pub const ABSTRACTION_SCHEMA_VERSION: u32 = 1;

/// Descriptive comparison, not a verified relationship or causal claim.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbstractionCandidate {
    pub schema_version: u32,
    pub id: String,
    pub from: String,
    pub to: String,
    pub context_key: String,
    pub left_value: String,
    pub right_value: String,
    pub left_observed: u64,
    pub left_missed: u64,
    pub right_observed: u64,
    pub right_missed: u64,
    pub left_rate: f64,
    pub right_rate: f64,
    pub evidence_ids: Vec<String>,
    pub cutoff_unix_ms: i64,
    pub status: String,
}

impl Memory {
    /// Generate evidence-bound comparisons between context groups.
    ///
    /// Each candidate is a question for further validation.
    /// The original evidence and memory remain unchanged.
    pub fn abstraction_candidates_as_of(
        &self,
        from: &str,
        to: &str,
        context_key: &str,
        cutoff_unix_ms: i64,
    ) -> Result<Vec<AbstractionCandidate>, String> {
        let analysis = self.analyze_context_as_of(from, to, context_key, cutoff_unix_ms)?;

        let mut candidates = Vec::new();

        for (i, left) in analysis.groups.iter().enumerate() {
            for right in analysis.groups.iter().skip(i + 1) {
                // Equal observed rates provide no descriptive difference.
                if left.observed as u128 * right.total as u128
                    == right.observed as u128 * left.total as u128
                {
                    continue;
                }

                let mut evidence_ids = left.evidence_ids.clone();
                evidence_ids.extend(right.evidence_ids.iter().cloned());
                evidence_ids.sort();
                evidence_ids.dedup();

                let identity = serde_json::to_string(&(
                    from,
                    to,
                    context_key,
                    left.value.as_str(),
                    right.value.as_str(),
                    cutoff_unix_ms,
                ))
                .map_err(|e| e.to_string())?;

                candidates.push(AbstractionCandidate {
                    schema_version: ABSTRACTION_SCHEMA_VERSION,
                    id: format!("context-comparison:{identity}"),
                    from: from.into(),
                    to: to.into(),
                    context_key: context_key.into(),
                    left_value: left.value.clone(),
                    right_value: right.value.clone(),
                    left_observed: left.observed,
                    left_missed: left.missed,
                    right_observed: right.observed,
                    right_missed: right.missed,
                    left_rate: left.empirical_rate,
                    right_rate: right.empirical_rate,
                    evidence_ids,
                    cutoff_unix_ms,
                    status: "unverified_candidate".into(),
                });
            }
        }

        Ok(candidates)
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

        for (id, mode, occurred, available) in [
            ("e1", "a", true, 100),
            ("e2", "a", true, 200),
            ("e3", "b", false, 300),
            ("e4", "b", true, 500),
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
                reference: format!("synthetic/{id}"),
                context: BTreeMap::from([("mode".into(), mode.into())]),
            })
            .unwrap();
        }

        m
    }

    #[test]
    fn creates_evidence_bound_candidate() {
        let m = fixture();
        let candidates = m
            .abstraction_candidates_as_of("input", "output", "mode", 1000)
            .unwrap();

        assert_eq!(candidates.len(), 1);
        let c = &candidates[0];
        assert_eq!(c.left_rate, 1.0);
        assert_eq!(c.right_rate, 0.5);
        assert_eq!(c.evidence_ids.len(), 4);
        assert_eq!(c.status, "unverified_candidate");
    }

    #[test]
    fn future_evidence_is_not_referenced() {
        let m = fixture();
        let candidates = m
            .abstraction_candidates_as_of("input", "output", "mode", 300)
            .unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].right_rate, 0.0);
        assert!(!candidates[0].evidence_ids.contains(&"e4".into()));
    }

    #[test]
    fn equal_rates_do_not_generate_difference_candidates() {
        let m = fixture();

        // At 200 only context "a" is available.
        assert!(m
            .abstraction_candidates_as_of("input", "output", "mode", 200)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn abstraction_is_deterministic_and_read_only() {
        let m = fixture();
        let before = serde_json::to_value(&m).unwrap();

        let a = m
            .abstraction_candidates_as_of("input", "output", "mode", 1000)
            .unwrap();
        let b = m
            .abstraction_candidates_as_of("input", "output", "mode", 1000)
            .unwrap();

        assert_eq!(a, b);
        assert_eq!(serde_json::to_value(&m).unwrap(), before);
    }
}
