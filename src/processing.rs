use crate::Memory;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingCandidate {
    pub token_id: String,
    pub similarity: f32,
    pub activation: f32,
    pub evidence_records: usize,
    pub repeated_reference_groups: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetrievalDecision {
    Found(ProcessingCandidate),
    Ambiguous(Vec<ProcessingCandidate>),
    NotFound,
}

impl Memory {
    /// Read-only coordination of semantic candidates, a resolved
    /// context anchor, existing relations, and provenance metadata.
    ///
    /// Positive relations are treated as associations, not proof.
    /// Evidence counts are descriptive, not confidence scores.
    pub fn process_retrieval(
        &self,
        query: &[f32],
        anchor: &str,
        cutoff_unix_ms: i64,
    ) -> Result<RetrievalDecision, String> {
        self.process_retrieval_filtered(query, anchor, cutoff_unix_ms, None)
    }

    /// Restrict retrieval to explicitly eligible existing tokens.
    /// This is query scope, not a stored token type or truth claim.
    pub fn process_retrieval_scoped(
        &self,
        query: &[f32],
        anchor: &str,
        cutoff_unix_ms: i64,
        eligible_ids: &[&str],
    ) -> Result<RetrievalDecision, String> {
        if eligible_ids.is_empty() {
            return Err("Eligible token list must not be empty".into());
        }

        let eligible: BTreeSet<String> = eligible_ids.iter().map(|id| (*id).to_owned()).collect();

        if eligible.len() != eligible_ids.len() {
            return Err("Duplicate eligible token ID".into());
        }

        if eligible
            .iter()
            .any(|id| id == anchor || !self.states.contains_key(id))
        {
            return Err("Invalid eligible token ID".into());
        }

        self.process_scoped_candidates(query, anchor, cutoff_unix_ms, &eligible)
    }

    /// Evaluate only eligible tokens with a positive direct relation.
    /// Preserve the original cosine computation and ordering.
    fn process_scoped_candidates(
        &self,
        query: &[f32],
        anchor: &str,
        cutoff_unix_ms: i64,
        eligible: &BTreeSet<String>,
    ) -> Result<RetrievalDecision, String> {
        if !self.states.contains_key(anchor) {
            return Err("Unknown context anchor".into());
        }

        if query.is_empty() || !query.iter().all(|x| x.is_finite()) {
            return Err("Query must contain finite numeric values".into());
        }

        let norm_squared: f64 = query.iter().map(|x| f64::from(*x) * f64::from(*x)).sum();

        if norm_squared == 0.0 {
            return Err("Query vector must be nonzero".into());
        }

        let query_norm = query.iter().map(|x| x * x).sum::<f32>().sqrt();

        if query_norm == 0.0 {
            return Ok(RetrievalDecision::NotFound);
        }

        let mut candidates = Vec::new();

        for id in eligible {
            let key = serde_json::to_string(&(anchor, id.as_str())).map_err(|e| e.to_string())?;

            let linked = self
                .relations
                .get(&key)
                .is_some_and(|r| r.from == anchor && r.to == *id && r.weight > 0.0);

            if !linked {
                continue;
            }

            let state = self.states.get(id).ok_or("Eligible state disappeared")?;

            if state.vector.len() != query.len() {
                continue;
            }

            let norm = state.vector.iter().map(|x| x * x).sum::<f32>().sqrt();

            if norm == 0.0 {
                continue;
            }

            let dot: f32 = state.vector.iter().zip(query).map(|(a, b)| a * b).sum();

            let similarity = dot / (norm * query_norm);

            if !similarity.is_finite() {
                continue;
            }

            let dynamics = self.dynamics(id).ok_or("Eligible state missing dynamics")?;

            if !dynamics.activation.is_finite() || !(0.0..=1.0).contains(&dynamics.activation) {
                return Err("Stored activation is invalid".into());
            }

            let quality = self.analyze_evidence_quality_as_of(anchor, id, cutoff_unix_ms)?;

            candidates.push(ProcessingCandidate {
                token_id: id.clone(),
                similarity,
                activation: dynamics.activation,
                evidence_records: quality.total_records,
                repeated_reference_groups: quality.repeated_reference_groups,
            });
        }

        candidates.sort_by(|a, b| {
            b.similarity
                .total_cmp(&a.similarity)
                .then_with(|| a.token_id.cmp(&b.token_id))
        });

        match candidates.len() {
            0 => Ok(RetrievalDecision::NotFound),
            1 => Ok(RetrievalDecision::Found(candidates.remove(0))),
            _ => Ok(RetrievalDecision::Ambiguous(candidates)),
        }
    }

    fn process_retrieval_filtered(
        &self,
        query: &[f32],
        anchor: &str,
        cutoff_unix_ms: i64,
        eligible: Option<&BTreeSet<String>>,
    ) -> Result<RetrievalDecision, String> {
        if !self.states.contains_key(anchor) {
            return Err("Unknown context anchor".into());
        }

        let preview = self.retrieval_priority_preview(query, self.states.len())?;

        let mut candidates = Vec::new();

        for candidate in preview.candidates {
            if candidate.token_id == anchor {
                continue;
            }

            if eligible.is_some_and(|ids| !ids.contains(&candidate.token_id)) {
                continue;
            }

            let linked = self.relations.values().any(|relation| {
                relation.from == anchor
                    && relation.to == candidate.token_id
                    && relation.weight > 0.0
            });

            if !linked {
                continue;
            }

            let quality =
                self.analyze_evidence_quality_as_of(anchor, &candidate.token_id, cutoff_unix_ms)?;

            candidates.push(ProcessingCandidate {
                token_id: candidate.token_id,
                similarity: candidate.similarity,
                activation: candidate.activation,
                evidence_records: quality.total_records,
                repeated_reference_groups: quality.repeated_reference_groups,
            });
        }

        match candidates.len() {
            0 => Ok(RetrievalDecision::NotFound),
            1 => Ok(RetrievalDecision::Found(candidates.remove(0))),
            _ => Ok(RetrievalDecision::Ambiguous(candidates)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn memory() -> Memory {
        let mut memory = Memory::new();

        for (id, vector) in [
            ("customer", vec![0.0, 1.0]),
            ("record_a", vec![1.0, 0.0]),
            ("record_b", vec![0.6, 0.8]),
        ] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector,
                })
                .unwrap();
        }

        memory
    }

    #[test]
    fn unique_relation_is_found() {
        let mut memory = memory();
        memory.reinforce("customer", "record_b", 1.0).unwrap();

        let result = memory
            .process_retrieval(&[1.0, 0.0], "customer", 100)
            .unwrap();

        match result {
            RetrievalDecision::Found(candidate) => {
                assert_eq!(candidate.token_id, "record_b");
                assert_eq!(candidate.evidence_records, 0);
            }
            _ => panic!("Expected one associated record"),
        }
    }

    #[test]
    fn multiple_relations_remain_ambiguous() {
        let mut memory = memory();
        memory.reinforce("customer", "record_a", 1.0).unwrap();
        memory.reinforce("customer", "record_b", 1.0).unwrap();

        let result = memory
            .process_retrieval(&[1.0, 0.0], "customer", 100)
            .unwrap();

        assert!(matches!(
            result,
            RetrievalDecision::Ambiguous(records) if records.len() == 2
        ));
    }

    #[test]
    fn missing_relation_is_not_guessed() {
        assert_eq!(
            memory()
                .process_retrieval(&[1.0, 0.0], "customer", 100,)
                .unwrap(),
            RetrievalDecision::NotFound
        );
    }

    #[test]
    fn processing_does_not_change_memory() {
        let mut memory = memory();
        memory.reinforce("customer", "record_a", 1.0).unwrap();

        let before = serde_json::to_value(&memory).unwrap();

        let first = memory
            .process_retrieval(&[1.0, 0.0], "customer", 100)
            .unwrap();
        let second = memory
            .process_retrieval(&[1.0, 0.0], "customer", 100)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn explicit_scope_excludes_unrelated_linked_tokens() {
        let mut memory = memory();
        memory.reinforce("customer", "record_a", 1.0).unwrap();
        memory.reinforce("customer", "record_b", 1.0).unwrap();

        let scoped = memory
            .process_retrieval_scoped(&[1.0, 0.0], "customer", 100, &["record_b"])
            .unwrap();

        assert!(matches!(
            scoped,
            RetrievalDecision::Found(candidate)
                if candidate.token_id == "record_b"
        ));

        assert!(matches!(
            memory
                .process_retrieval(&[1.0, 0.0], "customer", 100)
                .unwrap(),
            RetrievalDecision::Ambiguous(_)
        ));
    }

    #[test]
    fn scoped_retrieval_does_not_invent_a_relation() {
        let memory = memory();
        assert_eq!(
            memory
                .process_retrieval_scoped(&[1.0, 0.0], "customer", 100, &["record_a"])
                .unwrap(),
            RetrievalDecision::NotFound
        );
    }

    #[test]
    fn scoped_retrieval_rejects_invalid_candidates() {
        let memory = memory();

        for ids in [
            vec![],
            vec!["missing"],
            vec!["customer"],
            vec!["record_a", "record_a"],
        ] {
            assert!(memory
                .process_retrieval_scoped(&[1.0, 0.0], "customer", 100, &ids)
                .is_err());
        }
    }

    #[test]
    fn scoped_retrieval_is_read_only() {
        let mut memory = memory();
        memory.reinforce("customer", "record_a", 1.0).unwrap();

        let before = serde_json::to_value(&memory).unwrap();

        let first = memory
            .process_retrieval_scoped(&[1.0, 0.0], "customer", 100, &["record_a"])
            .unwrap();

        let second = memory
            .process_retrieval_scoped(&[1.0, 0.0], "customer", 100, &["record_a"])
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn optimized_scope_matches_full_retrieval() {
        let mut memory = memory();
        memory.reinforce("customer", "record_a", 1.0).unwrap();
        memory.reinforce("customer", "record_b", 1.0).unwrap();

        let full = memory
            .process_retrieval(&[1.0, 0.0], "customer", 100)
            .unwrap();

        let scoped = memory
            .process_retrieval_scoped(&[1.0, 0.0], "customer", 100, &["record_a", "record_b"])
            .unwrap();

        assert_eq!(full, scoped);
    }

    #[test]
    fn optimized_scope_ignores_unrelated_tokens() {
        let mut memory = memory();
        memory.reinforce("customer", "record_b", 1.0).unwrap();

        for n in 0..100 {
            memory
                .insert(crate::StateToken {
                    id: format!("distractor_{n}"),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        let result = memory
            .process_retrieval_scoped(&[1.0, 0.0], "customer", 100, &["record_b"])
            .unwrap();

        assert!(matches!(
            result,
            RetrievalDecision::Found(candidate)
                if candidate.token_id == "record_b"
        ));
    }

    #[test]
    fn invalid_anchor_or_query_is_rejected() {
        let memory = memory();

        assert!(memory
            .process_retrieval(&[1.0, 0.0], "missing", 100,)
            .is_err());

        assert!(memory
            .process_retrieval(&[0.0, 0.0], "customer", 100,)
            .is_err());
    }
}
