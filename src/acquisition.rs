//! Bounded feedback-driven *internal* information acquisition.
//!
//! This read-only coordinator reuses Learning-B semantic ranking and does not
//! infer correctness from the existence of additional graph connections.
//! An external agent must provide feedback after inspecting the prior batch.
use crate::{learning_b_semantic::SemanticActivation, Memory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionFeedback {
    Initial,
    Sufficient,
    Insufficient,
    Incorrect,
    Contradictory,
    ExecutionError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionDisposition {
    ContextDelivered,
    Complete,
    ExpandAvailable,
    ExternalSourceCandidate,
    BudgetExhausted,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcquisitionBatch {
    pub anchor: String,
    pub depth_reached: usize,
    pub results: Vec<SemanticActivation>,
    pub total_reachable_candidates: usize,
    pub unseen_reachable_candidates: usize,
    pub unseen_within_depth: usize,
    pub disposition: AcquisitionDisposition,
}

impl Memory {
    /// Select a new batch, progressing through task-relative graph depths.
    ///
    /// The caller retains `seen` across calls. It must not assert `Sufficient`
    /// merely because an LLM generated a plausible answer; externally verified
    /// results are preferred where possible.
    ///
    /// The maximum depth is an operational bound (1-3 is typical), not
    /// persistent memory-level classification. No external I/O occurs here.
    pub fn acquisition_preview(
        &self,
        anchor: &str,
        query_vector: &[f32],
        feedback: AcquisitionFeedback,
        seen: &BTreeSet<String>,
        max_depth: usize,
        batch_size: usize,
    ) -> Result<AcquisitionBatch, String> {
        if max_depth == 0 || batch_size == 0 {
            return Err("Acquisition depth and batch size must be positive".into());
        }
        if !self.states.contains_key(anchor) {
            return Err("Unknown acquisition anchor".into());
        }
        if seen.iter().any(|id| !self.states.contains_key(id)) {
            return Err("Unknown previously seen token".into());
        }
        // The existing semantic module computes candidates over all reachable
        // states; this stage only limits which ranked candidates are delivered.
        let full = self.learning_b_semantic_preview(
            anchor, query_vector, max_depth, self.states.len().max(1)
        )?;
        let total = full.candidates.len();
        let remaining: Vec<_> = full.candidates.into_iter()
            .filter(|c| !seen.contains(&c.token_id)).collect();
        let reachable = remaining.len();
        // Do not abandon unserved L1 candidates just because an earlier L1
        // result was insufficient. Exhaust each open graph depth before moving
        // to the next. Depth is a task-relative activation tier, NOT a
        // persistent Memory L1/L2/L3 classification.
        //
        // A first request opens depth 1; subsequent negative feedback may
        // advance to the shallowest still-unseen depth. This avoids a
        // spurious "external lookup" when deeper known memories remain.
        let depth_reached = if feedback == AcquisitionFeedback::Initial {
            1usize
        } else {
            remaining.iter().map(|c| c.depth).min().unwrap_or(max_depth)
        };
        let within: Vec<_> = remaining.iter()
            .filter(|c| c.depth <= depth_reached).cloned().collect();
        let within_count = within.len();
        let results = match feedback {
            AcquisitionFeedback::Sufficient => Vec::new(),
            _ => within.into_iter().take(batch_size).collect(),
        };
        let disposition = if feedback == AcquisitionFeedback::Sufficient {
            AcquisitionDisposition::Complete
        } else if !results.is_empty() {
            AcquisitionDisposition::ContextDelivered
        } else if reachable == 0 {
            AcquisitionDisposition::ExternalSourceCandidate
        } else {
            AcquisitionDisposition::ExpandAvailable
        };
        Ok(AcquisitionBatch {
            anchor: anchor.to_owned(), depth_reached, results,
            total_reachable_candidates: total,
            unseen_reachable_candidates: reachable,
            unseen_within_depth: within_count,
            disposition,
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn graph() -> Memory {
        let mut m = Memory::new();
        for (id, vector) in [
            ("task", vec![0.0, 1.0]),
            ("a", vec![-1.0, 0.0]),
            ("b", vec![0.0, -1.0]),
            ("answer", vec![1.0, 0.0]),
        ] {
            m.insert(StateToken { id: id.into(), vector }).unwrap();
        }
        m.reinforce("task", "a", 0.8).unwrap();
        m.reinforce("a", "b", 0.8).unwrap();
        m.reinforce("b", "answer", 0.8).unwrap();
        m
    }

    #[test]
    fn missing_context_opens_new_depth_without_resending_prior_tokens() {
        let m = graph();
        let mut seen = BTreeSet::new();
        let first = m.acquisition_preview("task", &[1.0, 0.0], AcquisitionFeedback::Initial, &seen, 3, 1).unwrap();
        assert_eq!(first.results[0].token_id, "a");
        seen.insert("a".to_owned());
        let second = m.acquisition_preview("task", &[1.0, 0.0], AcquisitionFeedback::Insufficient, &seen, 3, 1).unwrap();
        assert_eq!(second.results[0].token_id, "b");
        seen.insert("b".to_owned());
        let third = m.acquisition_preview("task", &[1.0, 0.0], AcquisitionFeedback::Incorrect, &seen, 3, 1).unwrap();
        assert_eq!(third.results[0].token_id, "answer");
    }

    #[test]
    fn feedback_exhausts_l1_before_opening_l2() {
        let mut m=Memory::new();
        for (id, vector) in [
            ("task",vec![0.,1.]),
            ("l1a",vec![0.8,0.2]),
            ("l1b",vec![-0.8,0.2]),
            ("l2",vec![1.,0.]),
        ] { m.insert(StateToken{id:id.into(),vector}).unwrap(); }
        m.reinforce("task","l1a",0.5).unwrap();
        m.reinforce("task","l1b",0.5).unwrap();
        m.reinforce("l1a","l2",0.5).unwrap();
        let mut seen=BTreeSet::new();
        let first=m.acquisition_preview("task",&[1.,0.],AcquisitionFeedback::Initial,&seen,3,1).unwrap();
        assert_eq!(first.results.len(),1);
        assert_eq!(first.results[0].depth,1);
        seen.insert(first.results[0].token_id.clone());
        let second=m.acquisition_preview("task",&[1.,0.],AcquisitionFeedback::Insufficient,&seen,3,1).unwrap();
        assert_eq!(second.depth_reached,1);
        assert_eq!(second.results[0].depth,1);
        assert_ne!(second.results[0].token_id,first.results[0].token_id);
        seen.insert(second.results[0].token_id.clone());
        let third=m.acquisition_preview("task",&[1.,0.],AcquisitionFeedback::Incorrect,&seen,3,1).unwrap();
        assert_eq!(third.depth_reached,2);
        assert_eq!(third.results[0].token_id,"l2");
        seen.insert("l2".to_owned());
        let exhausted=m.acquisition_preview("task",&[1.,0.],AcquisitionFeedback::Insufficient,&seen,3,1).unwrap();
        assert_eq!(exhausted.disposition,AcquisitionDisposition::ExternalSourceCandidate);
    }

    #[test]
    fn missing_l1_candidates_are_not_skipped_after_relevance_changes() {
        let mut m=Memory::new();
        for (id,v) in [
            ("root",vec![0.,1.]),
            ("first",vec![1.,0.]),
            ("other",vec![-1.,0.]),
            ("deep",vec![1.,0.]),
        ] { m.insert(StateToken{id:id.into(),vector:v}).unwrap(); }
        m.reinforce("root","first",0.5).unwrap();
        m.reinforce("root","other",0.5).unwrap();
        m.reinforce("other","deep",0.5).unwrap();
        let mut seen=BTreeSet::new();
        seen.insert("first".to_string());
        let r=m.acquisition_preview("root",&[1.,0.],AcquisitionFeedback::Insufficient,&seen,3,1).unwrap();
        assert_eq!(r.results[0].token_id,"other");
        assert_eq!(r.depth_reached,1);
    }

    #[test]
    fn sufficient_feedback_stops_read_only() {
        let m = graph();
        let before = serde_json::to_value(&m).unwrap();
        let result = m.acquisition_preview("task", &[1.0, 0.0], AcquisitionFeedback::Sufficient, &BTreeSet::new(), 3, 2).unwrap();
        assert!(result.results.is_empty());
        assert_eq!(result.disposition, AcquisitionDisposition::Complete);
        assert_eq!(before, serde_json::to_value(&m).unwrap());
    }
}
