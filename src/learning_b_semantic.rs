use crate::learning_b::ActivationTier;
use crate::Memory;

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticActivation {
    pub token_id: String,
    pub tier: ActivationTier,
    pub depth: usize,
    pub similarity: f32,
    pub path_weight: f32,
    pub combined_score: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticActivationView {
    pub anchor: String,
    pub candidates: Vec<SemanticActivation>,
    pub candidates_considered: usize,
    pub truncated: bool,
}

impl Memory {
    /// Experimental semantic ranking over reachable graph states.
    ///
    /// Signals:
    /// - cosine similarity to the task query
    /// - strongest path found by the existing weighted preview
    /// - graph depth, reported separately
    ///
    /// The 0.7/0.3 formula is a test parameter, not a
    /// calibrated CHIMERA Attention rule.
    ///
    /// Read-only. No changes to Memory A.
    pub fn learning_b_semantic_preview(
        &self,
        anchor: &str,
        query: &[f32],
        max_depth: usize,
        limit: usize,
    ) -> Result<SemanticActivationView, String> {
        if limit == 0 {
            return Err("Limit must be positive".into());
        }

        if query.is_empty() || !query.iter().all(|x| x.is_finite()) {
            return Err("Invalid query vector".into());
        }

        let query_norm = query
            .iter()
            .map(|x| f64::from(*x) * f64::from(*x))
            .sum::<f64>()
            .sqrt();

        if !query_norm.is_finite() || query_norm == 0.0 {
            return Err("Query vector must be nonzero".into());
        }

        // Full reachable candidate baseline, not optimized retrieval.
        // Graph exploration itself is bounded by max_depth.
        let graph =
            self.learning_b_weighted_preview(anchor, max_depth, self.states.len().max(1))?;

        let mut candidates = Vec::new();

        for item in graph.states {
            // The anchor defines the task context; do not rank it
            // as its own answer.
            if item.token_id == anchor {
                continue;
            }

            let state = self
                .states
                .get(&item.token_id)
                .ok_or("Graph returned unknown state")?;

            if state.vector.len() != query.len() {
                continue;
            }

            if !state.vector.iter().all(|x| x.is_finite()) {
                return Err("Invalid stored state vector".into());
            }

            let state_norm = state
                .vector
                .iter()
                .map(|x| f64::from(*x) * f64::from(*x))
                .sum::<f64>()
                .sqrt();

            if !state_norm.is_finite() {
                return Err("Invalid stored state norm".into());
            }

            if state_norm == 0.0 {
                continue;
            }

            let dot = state
                .vector
                .iter()
                .zip(query)
                .map(|(a, b)| f64::from(*a) * f64::from(*b))
                .sum::<f64>();

            let cosine = dot / (state_norm * query_norm);

            if !cosine.is_finite() {
                return Err("Invalid cosine similarity".into());
            }

            let similarity = cosine.clamp(-1.0, 1.0) as f32;

            // Map cosine [-1, 1] to [0, 1].
            let semantic = (similarity + 1.0) / 2.0;

            let score = 0.7 * semantic + 0.3 * item.path_weight;

            candidates.push(SemanticActivation {
                token_id: item.token_id,
                tier: item.tier,
                depth: item.depth,
                similarity,
                path_weight: item.path_weight,
                combined_score: score,
            });
        }

        candidates.sort_by(|a, b| {
            b.combined_score
                .total_cmp(&a.combined_score)
                .then_with(|| a.token_id.cmp(&b.token_id))
        });

        let considered = candidates.len();
        let truncated = graph.truncated || considered > limit;

        candidates.truncate(limit);

        Ok(SemanticActivationView {
            anchor: anchor.to_string(),
            candidates,
            candidates_considered: considered,
            truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture() -> Memory {
        let mut memory = Memory::new();

        for (id, vector) in [
            ("task", vec![0.0, 1.0]),
            ("near_irrelevant", vec![-1.0, 0.0]),
            ("bridge", vec![0.0, -1.0]),
            ("far_relevant", vec![1.0, 0.0]),
        ] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector,
                })
                .unwrap();
        }

        memory.reinforce("task", "near_irrelevant", 0.9).unwrap();
        memory.reinforce("task", "bridge", 0.8).unwrap();
        memory.reinforce("bridge", "far_relevant", 0.8).unwrap();

        memory
    }

    #[test]
    fn distant_relevant_token_can_outrank_nearby_token() {
        let memory = fixture();

        let view = memory
            .learning_b_semantic_preview("task", &[1.0, 0.0], 3, 3)
            .unwrap();

        assert_eq!(view.candidates[0].token_id, "far_relevant");
        assert_eq!(view.candidates[0].tier, ActivationTier::L2);
        assert_eq!(view.candidates[0].depth, 2);

        let near = view
            .candidates
            .iter()
            .find(|x| x.token_id == "near_irrelevant")
            .unwrap();

        assert_eq!(near.tier, ActivationTier::L1);
        assert!(view.candidates[0].combined_score > near.combined_score);
    }

    #[test]
    fn ranking_is_deterministic_and_read_only() {
        let memory = fixture();
        let before = serde_json::to_value(&memory).unwrap();

        let a = memory
            .learning_b_semantic_preview("task", &[1.0, 0.0], 3, 3)
            .unwrap();

        let b = memory
            .learning_b_semantic_preview("task", &[1.0, 0.0], 3, 3)
            .unwrap();

        assert_eq!(a, b);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn invalid_queries_and_budget_are_rejected() {
        let memory = fixture();

        assert!(memory
            .learning_b_semantic_preview("task", &[], 3, 3,)
            .is_err());

        assert!(memory
            .learning_b_semantic_preview("task", &[0.0, 0.0], 3, 3,)
            .is_err());

        assert!(memory
            .learning_b_semantic_preview("task", &[f32::NAN, 0.0], 3, 3,)
            .is_err());

        assert!(memory
            .learning_b_semantic_preview("task", &[1.0, 0.0], 3, 0,)
            .is_err());
    }

    #[test]
    fn depth_budget_excludes_distant_state() {
        let memory = fixture();

        let view = memory
            .learning_b_semantic_preview("task", &[1.0, 0.0], 1, 3)
            .unwrap();

        assert!(!view.candidates.iter().any(|x| x.token_id == "far_relevant"));
    }
}
