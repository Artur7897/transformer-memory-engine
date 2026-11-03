use crate::Memory;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    State,
    Memory,
    Skill,
    Llm,
}

/// Inertia is resistance to change, not confidence or truth.
/// Zero means no resistance; one means no change.
/// Actual calibration remains experimental.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenInertia {
    pub activation: f32,
    pub stability: f32,
    pub retention: f32,
}

impl Default for TokenInertia {
    fn default() -> Self {
        Self {
            activation: 0.0,
            stability: 0.0,
            retention: 0.0,
        }
    }
}

fn valid(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

impl TokenInertia {
    pub fn validate(&self) -> Result<(), String> {
        if !valid(self.activation) || !valid(self.stability) || !valid(self.retention) {
            return Err("Token inertia must contain finite values in [0,1]".into());
        }
        Ok(())
    }
}

/// Preview only: no persistent mutation.
pub fn inertia_preview(current: f32, target: f32, inertia: f32) -> Result<f32, String> {
    if !current.is_finite() || !target.is_finite() || !valid(inertia) {
        return Err("Invalid inertia calculation input".into());
    }

    let result =
        f64::from(current) + (f64::from(target) - f64::from(current)) * (1.0 - f64::from(inertia));

    let result = result as f32;

    if !result.is_finite() {
        return Err("Inertia calculation overflow".into());
    }

    Ok(result)
}

impl Memory {
    pub fn token_kind(&self, id: &str) -> Option<TokenKind> {
        if !self.states.contains_key(id) {
            return None;
        }

        Some(
            self.token_kinds
                .get(id)
                .copied()
                .unwrap_or(TokenKind::State),
        )
    }

    pub fn set_token_kind(&mut self, id: &str, kind: TokenKind) -> Result<(), String> {
        if !self.states.contains_key(id) {
            return Err("Unknown token".into());
        }

        self.token_kinds.insert(id.to_owned(), kind);
        Ok(())
    }

    pub fn token_inertia(&self, id: &str) -> Option<TokenInertia> {
        if !self.states.contains_key(id) {
            return None;
        }

        Some(self.token_inertia.get(id).copied().unwrap_or_default())
    }

    pub fn set_token_inertia(&mut self, id: &str, inertia: TokenInertia) -> Result<(), String> {
        if !self.states.contains_key(id) {
            return Err("Unknown token".into());
        }

        inertia.validate()?;
        self.token_inertia.insert(id.to_owned(), inertia);
        Ok(())
    }

    pub fn relation_inertia(&self, from: &str, to: &str) -> Option<f32> {
        let key = serde_json::to_string(&(from, to)).ok()?;

        if !self.relations.contains_key(&key) {
            return None;
        }

        Some(*self.relation_inertia.get(&key).unwrap_or(&0.0))
    }

    pub fn set_relation_inertia(
        &mut self,
        from: &str,
        to: &str,
        inertia: f32,
    ) -> Result<(), String> {
        if !valid(inertia) {
            return Err("Invalid relation inertia".into());
        }

        let key = serde_json::to_string(&(from, to)).map_err(|e| e.to_string())?;

        if !self.relations.contains_key(&key) {
            return Err("Unknown relation".into());
        }

        self.relation_inertia.insert(key, inertia);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture() -> Memory {
        let mut memory = Memory::new();

        for id in ["a", "b"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        memory
    }

    #[test]
    fn four_token_types_survive_serialization() {
        let mut memory = fixture();

        memory.set_token_kind("a", TokenKind::Skill).unwrap();
        memory.set_token_kind("b", TokenKind::Llm).unwrap();

        let serialized = serde_json::to_string(&memory).unwrap();
        let restored: Memory = serde_json::from_str(&serialized).unwrap();

        assert_eq!(restored.token_kind("a"), Some(TokenKind::Skill));
        assert_eq!(restored.token_kind("b"), Some(TokenKind::Llm));
        assert_eq!(restored.token_kind("missing"), None);
    }

    #[test]
    fn legacy_tokens_default_to_state() {
        let memory = fixture();
        assert_eq!(memory.token_kind("a"), Some(TokenKind::State));
    }

    #[test]
    fn independent_inertia_dimensions_persist() {
        let mut memory = fixture();

        let inertia = TokenInertia {
            activation: 0.2,
            stability: 0.8,
            retention: 0.95,
        };

        memory.set_token_inertia("a", inertia).unwrap();

        let serialized = serde_json::to_string(&memory).unwrap();
        let restored: Memory = serde_json::from_str(&serialized).unwrap();

        assert_eq!(restored.token_inertia("a"), Some(inertia));
        assert_eq!(restored.token_inertia("b"), Some(TokenInertia::default()));
    }

    #[test]
    fn relation_inertia_is_independent() {
        let mut memory = fixture();
        memory.reinforce("a", "b", 0.5).unwrap();
        memory.set_relation_inertia("a", "b", 0.7).unwrap();

        assert_eq!(memory.relation_inertia("a", "b"), Some(0.7));
        assert_eq!(memory.relation_inertia("b", "a"), None);
    }

    #[test]
    fn inertia_preview_is_deterministic_and_read_only() {
        assert!((inertia_preview(0.2, 0.8, 0.5).unwrap() - 0.5).abs() < 1e-6);
        assert_eq!(inertia_preview(0.2, 0.8, 1.0).unwrap(), 0.2);
        assert_eq!(inertia_preview(0.2, 0.8, 0.0).unwrap(), 0.8);
        assert!(inertia_preview(0.2, 0.8, f32::NAN).is_err());
    }

    #[test]
    fn invalid_inertia_does_not_mutate_memory() {
        let mut memory = fixture();

        assert!(memory
            .set_token_inertia(
                "a",
                TokenInertia {
                    activation: 1.5,
                    stability: 0.5,
                    retention: 0.5,
                }
            )
            .is_err());

        assert_eq!(memory.token_inertia("a"), Some(TokenInertia::default()));
    }
}
