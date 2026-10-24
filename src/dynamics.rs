use crate::Memory;
use serde::{Deserialize, Serialize};

/// Dynamic properties are separate from the token vector.
///
/// Activation and stability are experimental state variables,
/// not confidence, correctness or epistemic reliability.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenDynamics {
    pub activation: f32,
    pub stability: f32,
    pub operations: u64,
    pub last_reflected_operations: u64,
}

impl Default for TokenDynamics {
    fn default() -> Self {
        Self {
            activation: 1.0,
            stability: 0.0,
            operations: 0,
            last_reflected_operations: 0,
        }
    }
}

impl TokenDynamics {
    pub fn pending_operations(&self) -> u64 {
        self.operations
            .saturating_sub(self.last_reflected_operations)
    }
}

impl Memory {
    /// Return stored dynamics, or the initial state for a legacy token.
    pub fn dynamics(&self, id: &str) -> Option<TokenDynamics> {
        if !self.states.contains_key(id) {
            return None;
        }
        Some(self.token_dynamics.get(id).cloned().unwrap_or_default())
    }

    /// Record an explicitly reported processing operation.
    ///
    /// This does not activate Reflect, Sleep or Decay.
    pub fn record_token_operation(&mut self, id: &str) -> Result<u64, String> {
        if !self.states.contains_key(id) {
            return Err("State token does not exist".into());
        }

        let state = self.token_dynamics.entry(id.to_string()).or_default();
        let next = state
            .operations
            .checked_add(1)
            .ok_or("Token operation counter overflow")?;
        state.operations = next;
        Ok(next)
    }

    /// Set experimental dynamic values without interpreting them as truth.
    pub fn set_token_dynamics(
        &mut self,
        id: &str,
        activation: f32,
        stability: f32,
    ) -> Result<(), String> {
        if !self.states.contains_key(id) {
            return Err("State token does not exist".into());
        }
        if !activation.is_finite()
            || !(0.0..=1.0).contains(&activation)
            || !stability.is_finite()
            || !(0.0..=1.0).contains(&stability)
        {
            return Err("Activation and stability must be finite values in [0, 1]".into());
        }

        let state = self.token_dynamics.entry(id.to_string()).or_default();
        state.activation = activation;
        state.stability = stability;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn memory_with_token() -> Memory {
        let mut memory = Memory::new();
        memory
            .insert(StateToken {
                id: "btc_state".into(),
                vector: vec![0.5, 0.8],
            })
            .unwrap();
        memory
    }

    #[test]
    fn new_token_has_independent_dynamics() {
        let memory = memory_with_token();
        let state = memory.dynamics("btc_state").unwrap();

        assert_eq!(state.activation, 1.0);
        assert_eq!(state.stability, 0.0);
        assert_eq!(state.operations, 0);
        assert_eq!(memory.states["btc_state"].vector, vec![0.5, 0.8]);
    }

    #[test]
    fn operations_are_counted_without_triggering_decay() {
        let mut memory = memory_with_token();
        for _ in 0..25 {
            memory.record_token_operation("btc_state").unwrap();
        }

        let state = memory.dynamics("btc_state").unwrap();
        assert_eq!(state.operations, 25);
        assert_eq!(state.pending_operations(), 25);
        assert_eq!(memory.cycles, 0);
    }

    #[test]
    fn strength_and_stability_are_independent() {
        let mut memory = memory_with_token();
        memory.set_token_dynamics("btc_state", 1.0, 0.95).unwrap();

        let state = memory.dynamics("btc_state").unwrap();
        assert_eq!(state.activation, 1.0);
        assert_eq!(state.stability, 0.95);
        assert!(memory.set_token_dynamics("btc_state", 1.2, 0.1).is_err());
        assert_eq!(memory.dynamics("btc_state").unwrap(), state);
    }

    #[test]
    fn dynamics_survive_persistence_and_legacy_loading() {
        let mut memory = memory_with_token();
        memory.record_token_operation("btc_state").unwrap();
        memory.set_token_dynamics("btc_state", 0.6, 0.9).unwrap();

        let json = serde_json::to_string(&memory).unwrap();
        let restored: Memory = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.dynamics("btc_state"), memory.dynamics("btc_state"));

        let mut legacy: serde_json::Value = serde_json::from_str(&json).unwrap();
        legacy.as_object_mut().unwrap().remove("token_dynamics");

        let old: Memory = serde_json::from_value(legacy).unwrap();
        assert_eq!(old.dynamics("btc_state"), Some(TokenDynamics::default()));
        assert_eq!(old.record_count_for_test(), 0);
    }
}

#[cfg(test)]
impl Memory {
    fn record_count_for_test(&self) -> usize {
        self.token_dynamics.len()
    }
}
