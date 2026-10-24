pub mod abstraction;
pub mod context_analysis;
pub mod contextual_evidence;
pub mod dynamics;
pub mod evidence_analysis;
pub mod experience;
pub mod hypotheses;
pub mod metrics;
pub mod reflect;
pub mod validation;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StateToken {
    pub id: String,
    pub vector: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub weight: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TransitionEvidence {
    pub observed: u64,
    pub missed: u64,
}

impl TransitionEvidence {
    pub fn strength(&self) -> f64 {
        // Laplace-smoothed empirical transition estimate.
        (self.observed as f64 + 1.0) / (self.observed as f64 + self.missed as f64 + 2.0)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Memory {
    pub states: BTreeMap<String, StateToken>,
    pub relations: BTreeMap<String, Relation>,
    pub cycles: u64,
    #[serde(default)]
    pub transitions: BTreeMap<String, TransitionEvidence>,
    #[serde(default)]
    pub token_dynamics: BTreeMap<String, dynamics::TokenDynamics>,
    #[serde(default)]
    pub experiences: BTreeMap<String, experience::ExperienceRecord>,
    #[serde(default)]
    pub contextual_evidence: BTreeMap<String, contextual_evidence::ContextualEvidenceRecord>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, token: StateToken) -> Result<(), String> {
        if token.id.is_empty() || token.vector.is_empty() {
            return Err("State ID and vector must not be empty".into());
        }
        if !token.vector.iter().all(|x| x.is_finite()) {
            return Err("State vector must contain finite numbers".into());
        }
        if self.states.contains_key(&token.id) {
            return Err("State ID already exists".into());
        }

        let id = token.id.clone();
        self.states.insert(id.clone(), token);
        self.token_dynamics
            .insert(id, dynamics::TokenDynamics::default());
        Ok(())
    }

    pub fn reinforce(&mut self, from: &str, to: &str, delta: f32) -> Result<(), String> {
        if !self.states.contains_key(from) || !self.states.contains_key(to) {
            return Err("Both states must exist".into());
        }
        if !delta.is_finite() || delta <= 0.0 {
            return Err("Reinforcement must be positive and finite".into());
        }

        let key = serde_json::to_string(&(from, to)).map_err(|e| e.to_string())?;

        let relation = self.relations.entry(key).or_insert(Relation {
            from: from.into(),
            to: to.into(),
            weight: 0.0,
        });

        relation.weight = (relation.weight + delta).min(1.0);
        Ok(())
    }

    pub fn decay(&mut self, rate: f32) -> Result<(), String> {
        if !rate.is_finite() || !(0.0..=1.0).contains(&rate) {
            return Err("Decay rate must be between 0 and 1".into());
        }

        for relation in self.relations.values_mut() {
            relation.weight *= 1.0 - rate;
        }

        self.cycles += 1;
        Ok(())
    }

    pub fn attention(&self, query: &[f32], limit: usize) -> Vec<(String, f32)> {
        if query.is_empty() || !query.iter().all(|x| x.is_finite()) {
            return vec![];
        }

        let query_norm = query.iter().map(|x| x * x).sum::<f32>().sqrt();
        if query_norm == 0.0 {
            return vec![];
        }

        let mut scores = Vec::new();

        for state in self.states.values() {
            if state.vector.len() != query.len() {
                continue;
            }

            let norm = state.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm == 0.0 {
                continue;
            }

            let dot: f32 = state.vector.iter().zip(query).map(|(a, b)| a * b).sum();

            let score = dot / (norm * query_norm);
            if score.is_finite() {
                scores.push((state.id.clone(), score));
            }
        }

        scores.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        scores.truncate(limit);
        scores
    }

    /// Record one explicitly evaluated opportunity for a transition.
    /// occurred=true: target followed the source within the observation window.
    /// occurred=false: the target did not follow within that same window.
    pub fn observe_transition(
        &mut self,
        from: &str,
        to: &str,
        occurred: bool,
    ) -> Result<f64, String> {
        if !self.states.contains_key(from) || !self.states.contains_key(to) {
            return Err("Both states must exist".into());
        }

        let key = serde_json::to_string(&(from, to)).map_err(|e| e.to_string())?;

        let evidence = self.transitions.entry(key).or_default();

        if occurred {
            evidence.observed = evidence
                .observed
                .checked_add(1)
                .ok_or("Observed counter overflow")?;
        } else {
            evidence.missed = evidence
                .missed
                .checked_add(1)
                .ok_or("Missed counter overflow")?;
        }

        Ok(evidence.strength())
    }

    pub fn transition_strength(&self, from: &str, to: &str) -> Option<f64> {
        let key = serde_json::to_string(&(from, to)).ok()?;
        self.transitions.get(&key).map(|e| e.strength())
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self)?;
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(tmp, path)
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_and_adaptation() {
        let mut memory = Memory::new();

        memory
            .insert(StateToken {
                id: "market_state".into(),
                vector: vec![0.8, -0.2, 0.4],
            })
            .unwrap();

        memory
            .insert(StateToken {
                id: "experience".into(),
                vector: vec![0.7, -0.1, 0.5],
            })
            .unwrap();

        memory.reinforce("market_state", "experience", 0.8).unwrap();
        memory.decay(0.1).unwrap();

        let relation = memory.relations.values().next().unwrap();
        assert!((relation.weight - 0.72).abs() < 0.00001);

        let results = memory.attention(&[0.8, -0.2, 0.4], 1);
        assert_eq!(results[0].0, "market_state");

        let path = std::env::temp_dir().join(format!("tme-test-{}.json", std::process::id()));

        memory.save(&path).unwrap();
        let restored = Memory::load(&path).unwrap();
        fs::remove_file(path).unwrap();

        assert_eq!(restored.states.len(), 2);
        assert_eq!(restored.relations.len(), 1);
        assert_eq!(restored.cycles, 1);
        assert_eq!(restored.relations, memory.relations);
    }

    #[test]
    fn invalid_states_are_rejected() {
        let mut memory = Memory::new();
        assert!(memory
            .insert(StateToken {
                id: "invalid".into(),
                vector: vec![f32::NAN],
            })
            .is_err());
        assert!(memory.decay(-0.1).is_err());
    }

    #[test]
    fn learns_from_positive_and_negative_experience() {
        let mut memory = Memory::new();

        for id in ["event", "response"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        for _ in 0..8 {
            memory
                .observe_transition("event", "response", true)
                .unwrap();
        }

        for _ in 0..2 {
            memory
                .observe_transition("event", "response", false)
                .unwrap();
        }

        let strength = memory.transition_strength("event", "response").unwrap();

        assert!((strength - 0.75).abs() < 1e-10);

        for _ in 0..10 {
            memory
                .observe_transition("event", "response", false)
                .unwrap();
        }

        let reduced = memory.transition_strength("event", "response").unwrap();

        assert!(reduced < strength);
    }

    #[test]
    fn transition_evidence_survives_restart() {
        let mut memory = Memory::new();

        for id in ["a", "b"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![0.5, 0.5],
                })
                .unwrap();
        }

        memory.observe_transition("a", "b", true).unwrap();
        memory.observe_transition("a", "b", false).unwrap();

        let path =
            std::env::temp_dir().join(format!("tme-transition-test-{}.json", std::process::id()));

        memory.save(&path).unwrap();
        let restored = Memory::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(memory.transitions, restored.transitions);
        assert_eq!(restored.transition_strength("a", "b"), Some(0.5));
    }
}
