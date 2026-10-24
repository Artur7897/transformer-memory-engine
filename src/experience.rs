use crate::Memory;
use serde::{Deserialize, Serialize};

/// Describes how an outcome was established.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceSource {
    ExternalObservation,
    VerifiedToolResult,
    ModelStatement,
    HumanReport,
}

/// A recorded experience with a pre-existing expectation.
///
/// `observed` is None until a real outcome becomes available.
/// A model statement is not independently verified evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceRecord {
    pub id: String,
    pub state_id: String,
    pub expected: String,
    pub observed: Option<String>,
    pub source: Option<EvidenceSource>,
    pub reference: Option<String>,
}

impl Memory {
    /// Store an expectation before observing the outcome.
    pub fn begin_experience(
        &mut self,
        id: &str,
        state_id: &str,
        expected: &str,
    ) -> Result<(), String> {
        if id.trim().is_empty() || expected.trim().is_empty() {
            return Err("Experience ID and expectation must not be empty".into());
        }
        if !self.states.contains_key(state_id) {
            return Err("Referenced state token does not exist".into());
        }
        if self.experiences.contains_key(id) {
            return Err("Experience ID already exists".into());
        }

        self.experiences.insert(
            id.into(),
            ExperienceRecord {
                id: id.into(),
                state_id: state_id.into(),
                expected: expected.into(),
                observed: None,
                source: None,
                reference: None,
            },
        );
        Ok(())
    }

    /// Attach one outcome. Existing outcomes cannot be silently overwritten.
    pub fn complete_experience(
        &mut self,
        id: &str,
        observed: &str,
        source: EvidenceSource,
        reference: Option<&str>,
    ) -> Result<(), String> {
        if observed.trim().is_empty() {
            return Err("Observed outcome must not be empty".into());
        }
        if reference.is_some_and(|r| r.trim().is_empty()) {
            return Err("Evidence reference must not be empty".into());
        }

        let record = self
            .experiences
            .get_mut(id)
            .ok_or("Experience does not exist")?;

        if record.observed.is_some() {
            return Err("Experience outcome already recorded".into());
        }

        record.observed = Some(observed.into());
        record.source = Some(source);
        record.reference = reference.map(str::to_owned);

        Ok(())
    }

    pub fn pending_experience_count(&self) -> usize {
        self.experiences
            .values()
            .filter(|e| e.observed.is_none())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn memory() -> Memory {
        let mut m = Memory::new();
        m.insert(StateToken {
            id: "market".into(),
            vector: vec![0.5, 0.25],
        })
        .unwrap();
        m
    }

    #[test]
    fn expectation_precedes_observation() {
        let mut m = memory();
        m.begin_experience("case-1", "market", "UP").unwrap();

        assert_eq!(m.pending_experience_count(), 1);
        assert_eq!(m.experiences["case-1"].observed, None);

        m.complete_experience(
            "case-1",
            "DOWN",
            EvidenceSource::ExternalObservation,
            Some("historical-case-1"),
        )
        .unwrap();

        assert_eq!(m.pending_experience_count(), 0);
        assert_eq!(m.experiences["case-1"].expected, "UP");
        assert_eq!(m.experiences["case-1"].observed.as_deref(), Some("DOWN"));
    }

    #[test]
    fn observation_cannot_be_overwritten() {
        let mut m = memory();
        m.begin_experience("case-1", "market", "NO_TRADE").unwrap();

        m.complete_experience(
            "case-1",
            "NO_TRADE",
            EvidenceSource::VerifiedToolResult,
            None,
        )
        .unwrap();

        assert!(m
            .complete_experience("case-1", "TRADE", EvidenceSource::ModelStatement, None,)
            .is_err());
    }

    #[test]
    fn rejects_unknown_states_and_duplicate_ids() {
        let mut m = memory();
        assert!(m.begin_experience("x", "unknown", "UP").is_err());

        m.begin_experience("x", "market", "UP").unwrap();
        assert!(m.begin_experience("x", "market", "DOWN").is_err());
        assert!(m
            .complete_experience("missing", "UP", EvidenceSource::HumanReport, None)
            .is_err());
    }

    #[test]
    fn experience_persists_and_legacy_memory_loads() {
        let mut m = memory();
        m.begin_experience("case-1", "market", "UP").unwrap();
        m.complete_experience(
            "case-1",
            "UP",
            EvidenceSource::ExternalObservation,
            Some("case-1-outcome"),
        )
        .unwrap();

        let json = serde_json::to_string(&m).unwrap();
        let restored: Memory = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.experiences, m.experiences);

        let mut old: serde_json::Value = serde_json::from_str(&json).unwrap();
        old.as_object_mut().unwrap().remove("experiences");

        let legacy: Memory = serde_json::from_value(old).unwrap();
        assert!(legacy.experiences.is_empty());
    }
}
