use crate::Memory;
use serde::{Deserialize, Serialize};

/// Descriptive evidence only, not epistemic truth or
/// a calibrated prediction of the next transition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransitionAssessment {
    pub from: String,
    pub to: String,
    pub observed: u64,
    pub missed: u64,
    pub total: u128,
    pub empirical_rate: Option<f64>,
    pub laplace_rate: f64,
    pub mixed_outcomes: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvidenceAnalysis {
    pub transitions: Vec<TransitionAssessment>,
    pub unevaluated_pairs: usize,
    pub mixed_pairs: usize,
}

impl Memory {
    /// Deterministic, read-only analysis of recorded transitions.
    ///
    /// Rates describe the supplied evidence. They do not
    /// establish independence, causation or reliability.
    pub fn reflect_evidence(&self) -> Result<EvidenceAnalysis, String> {
        let mut transitions = Vec::new();
        let mut unevaluated_pairs = 0usize;
        let mut mixed_pairs = 0usize;

        for (key, evidence) in &self.transitions {
            let (from, to): (String, String) =
                serde_json::from_str(key).map_err(|_| format!("Invalid transition key: {key}"))?;

            if !self.states.contains_key(&from) || !self.states.contains_key(&to) {
                return Err("Transition references an unknown state".into());
            }

            let total = u128::from(evidence.observed) + u128::from(evidence.missed);

            let empirical_rate = if total == 0 {
                unevaluated_pairs += 1;
                None
            } else {
                Some(evidence.observed as f64 / total as f64)
            };

            let mixed = evidence.observed > 0 && evidence.missed > 0;

            if mixed {
                mixed_pairs += 1;
            }

            transitions.push(TransitionAssessment {
                from,
                to,
                observed: evidence.observed,
                missed: evidence.missed,
                total,
                empirical_rate,
                laplace_rate: evidence.strength(),
                mixed_outcomes: mixed,
            });
        }

        transitions.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));

        Ok(EvidenceAnalysis {
            transitions,
            unevaluated_pairs,
            mixed_pairs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StateToken, TransitionEvidence};

    fn fixture() -> Memory {
        let mut m = Memory::new();

        for id in ["a", "b"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        }

        m
    }

    #[test]
    fn distinguishes_raw_rate_from_smoothed_rate() {
        let mut m = fixture();
        m.observe_transition("a", "b", true).unwrap();
        m.observe_transition("a", "b", true).unwrap();
        m.observe_transition("a", "b", false).unwrap();

        let report = m.reflect_evidence().unwrap();
        let assessment = &report.transitions[0];

        assert_eq!(assessment.total, 3);
        assert_eq!(assessment.observed, 2);
        assert_eq!(assessment.missed, 1);
        assert_eq!(assessment.empirical_rate, Some(2.0 / 3.0));
        assert_eq!(assessment.laplace_rate, 3.0 / 5.0);
        assert!(assessment.mixed_outcomes);
        assert_eq!(report.mixed_pairs, 1);
    }

    #[test]
    fn zero_observations_do_not_imply_a_measured_rate() {
        let mut m = fixture();

        let key = serde_json::to_string(&("a", "b")).unwrap();
        m.transitions.insert(key, TransitionEvidence::default());

        let report = m.reflect_evidence().unwrap();

        assert_eq!(report.unevaluated_pairs, 1);
        assert_eq!(report.transitions[0].empirical_rate, None);
        assert_eq!(report.transitions[0].laplace_rate, 0.5);
    }

    #[test]
    fn analysis_is_read_only_and_deterministic() {
        let mut m = fixture();
        m.observe_transition("a", "b", true).unwrap();

        let before = serde_json::to_value(&m).unwrap();
        let first = m.reflect_evidence().unwrap();
        let second = m.reflect_evidence().unwrap();

        assert_eq!(first, second);
        assert_eq!(serde_json::to_value(&m).unwrap(), before);
    }

    #[test]
    fn rejects_dangling_transition_references() {
        let mut m = fixture();
        let key = serde_json::to_string(&("a", "missing")).unwrap();

        m.transitions.insert(
            key,
            TransitionEvidence {
                observed: 1,
                missed: 0,
            },
        );

        assert!(m.reflect_evidence().is_err());
    }
}
