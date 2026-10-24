use crate::Memory;
use serde::{Deserialize, Serialize};

pub const HYPOTHESIS_SCHEMA_VERSION: u32 = 1;

/// A question motivated by evidence, not a discovered fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisCandidate {
    pub schema_version: u32,
    pub id: String,
    pub from: String,
    pub to: String,
    pub source_transition_key: String,
    pub observed: u64,
    pub missed: u64,
    pub question_kind: String,
    pub status: String,
}

impl Memory {
    /// Generate deterministic research questions from mixed transitions.
    ///
    /// Does not invent explanatory variables, record predictions,
    /// alter weights, or promote hypotheses to facts.
    pub fn hypothesis_candidates(&self) -> Result<Vec<HypothesisCandidate>, String> {
        let analysis = self.reflect_evidence()?;
        let mut result = Vec::new();

        for assessment in analysis.transitions {
            if !assessment.mixed_outcomes {
                continue;
            }

            let key = serde_json::to_string(&(assessment.from.as_str(), assessment.to.as_str()))
                .map_err(|e| e.to_string())?;

            result.push(HypothesisCandidate {
                schema_version: HYPOTHESIS_SCHEMA_VERSION,
                id: format!("transition-context:{key}"),
                from: assessment.from,
                to: assessment.to,
                source_transition_key: key,
                observed: assessment.observed,
                missed: assessment.missed,
                question_kind: "unexplained_outcome_variation".into(),
                status: "unverified_candidate".into(),
            });
        }

        result.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture() -> Memory {
        let mut m = Memory::new();

        for id in ["a", "b", "c"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![0.5, 0.2],
            })
            .unwrap();
        }

        m
    }

    #[test]
    fn mixed_transition_generates_unverified_candidate() {
        let mut m = fixture();

        m.observe_transition("a", "b", true).unwrap();
        m.observe_transition("a", "b", false).unwrap();

        let result = m.hypothesis_candidates().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].from, "a");
        assert_eq!(result[0].to, "b");
        assert_eq!(result[0].observed, 1);
        assert_eq!(result[0].missed, 1);
        assert_eq!(result[0].status, "unverified_candidate");
    }

    #[test]
    fn consistent_observations_generate_no_candidate() {
        let mut m = fixture();

        m.observe_transition("a", "b", true).unwrap();
        m.observe_transition("a", "b", true).unwrap();

        assert!(m.hypothesis_candidates().unwrap().is_empty());
    }

    #[test]
    fn candidates_are_deterministic_and_read_only() {
        let mut m = fixture();

        m.observe_transition("b", "c", true).unwrap();
        m.observe_transition("b", "c", false).unwrap();
        m.observe_transition("a", "b", true).unwrap();
        m.observe_transition("a", "b", false).unwrap();

        let before = serde_json::to_value(&m).unwrap();
        let first = m.hypothesis_candidates().unwrap();
        let second = m.hypothesis_candidates().unwrap();

        assert_eq!(first, second);
        assert_eq!(first.len(), 2);
        assert_eq!(serde_json::to_value(&m).unwrap(), before);
    }

    #[test]
    fn invalid_transition_evidence_is_rejected() {
        let mut m = fixture();

        let key = serde_json::to_string(&("a", "missing")).unwrap();

        m.transitions.insert(
            key,
            crate::TransitionEvidence {
                observed: 1,
                missed: 1,
            },
        );

        assert!(m.hypothesis_candidates().is_err());
    }
}
