use crate::Memory;
use serde::{Deserialize, Serialize};

/// Diagnostic retrieval view. No combined ranking is calculated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetrievalCandidate {
    pub token_id: String,
    pub similarity: f32,
    pub activation: f32,
    pub retention_stability_established: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetrievalPriorityPreview {
    pub candidates: Vec<RetrievalCandidate>,
    pub combined_ranking_calibrated: bool,
    pub memory_changed: bool,
}

impl Memory {
    /// Inspect the existing semantic ranking alongside activation.
    ///
    /// Activation does not alter rank. Stored stability is not treated
    /// as a validated retention estimate or evidence of truth.
    pub fn retrieval_priority_preview(
        &self,
        query: &[f32],
        limit: usize,
    ) -> Result<RetrievalPriorityPreview, String> {
        if limit == 0 {
            return Err("Retrieval limit must be positive".into());
        }

        if query.is_empty() || !query.iter().all(|x| x.is_finite()) {
            return Err("Query must contain finite numeric values".into());
        }

        let norm_squared: f64 = query.iter().map(|x| f64::from(*x) * f64::from(*x)).sum();

        if norm_squared == 0.0 {
            return Err("Query vector must be nonzero".into());
        }

        let mut candidates = Vec::new();

        for (token_id, similarity) in self.attention(query, limit) {
            let dynamics = self
                .dynamics(&token_id)
                .ok_or("Attention returned an unknown token")?;

            if !dynamics.activation.is_finite() || !(0.0..=1.0).contains(&dynamics.activation) {
                return Err("Stored activation is invalid".into());
            }

            candidates.push(RetrievalCandidate {
                token_id,
                similarity,
                activation: dynamics.activation,
                retention_stability_established: false,
            });
        }

        Ok(RetrievalPriorityPreview {
            candidates,
            combined_ranking_calibrated: false,
            memory_changed: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture() -> Memory {
        let mut memory = Memory::new();

        memory
            .insert(StateToken {
                id: "relevant".into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();

        memory
            .insert(StateToken {
                id: "active".into(),
                vector: vec![0.6, 0.8],
            })
            .unwrap();

        memory
    }

    #[test]
    fn activation_does_not_change_semantic_ranking() {
        let mut memory = fixture();
        memory.set_token_dynamics("relevant", 0.05, 0.0).unwrap();
        memory.set_token_dynamics("active", 1.0, 0.9).unwrap();

        let preview = memory.retrieval_priority_preview(&[1.0, 0.0], 2).unwrap();

        assert_eq!(preview.candidates[0].token_id, "relevant");
        assert_eq!(preview.candidates[0].activation, 0.05);
        assert_eq!(preview.candidates[1].token_id, "active");
        assert_eq!(preview.candidates[1].activation, 1.0);
        assert!(!preview.combined_ranking_calibrated);
    }

    #[test]
    fn preview_matches_existing_attention() {
        let memory = fixture();
        let expected = memory.attention(&[1.0, 0.0], 2);
        let preview = memory.retrieval_priority_preview(&[1.0, 0.0], 2).unwrap();

        let actual: Vec<_> = preview
            .candidates
            .iter()
            .map(|candidate| (candidate.token_id.clone(), candidate.similarity))
            .collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deterministic_and_read_only() {
        let memory = fixture();
        let before = serde_json::to_value(&memory).unwrap();

        let first = memory.retrieval_priority_preview(&[1.0, 0.0], 2).unwrap();
        let second = memory.retrieval_priority_preview(&[1.0, 0.0], 2).unwrap();

        assert_eq!(first, second);
        assert!(!first.memory_changed);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        let memory = fixture();

        assert!(memory.retrieval_priority_preview(&[], 2).is_err());
        assert!(memory
            .retrieval_priority_preview(&[f32::NAN, 0.0], 2)
            .is_err());
        assert!(memory.retrieval_priority_preview(&[0.0, 0.0], 2).is_err());
        assert!(memory.retrieval_priority_preview(&[1.0, 0.0], 0).is_err());
    }
}
