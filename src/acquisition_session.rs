//! Persistent acquisition feedback ledger.
//! One session is owned by Memory and can be saved using state_container::update.
//! This module performs no network activity and does not claim that LLM feedback
//! proves an answer correct.
use crate::{acquisition::{AcquisitionBatch, AcquisitionDisposition, AcquisitionFeedback}, lifecycle::TmePhase, Memory};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcquisitionSession {
    /// Sources that supplied new *hints* to retrieval. Never treated as proof.
    #[serde(default)]
    pub external_hints: Vec<ExternalAcquisitionHint>,
    pub id: String,
    pub anchor: String,
    pub query_vector: Vec<f32>,
    pub max_depth: usize,
    pub batch_size: usize,
    pub seen: BTreeSet<String>,
    pub rounds: u32,
    pub closed: bool,
    pub last_feedback: AcquisitionFeedback,
    pub last_disposition: Option<AcquisitionDisposition>,
}

/// Audit record for an external clue, without copying untrusted content into
/// Memory or endorsing its claims. The embedder and tool gateway run outside TME.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalAcquisitionHint {
    pub source_reference: String,
    pub reason: String,
}

impl Memory {
    fn require_active_acquisition(&self) -> Result<(), String> {
        if self.lifecycle.phase != TmePhase::Active {
            return Err("Acquisition mutations require ACTIVE phase".into());
        }
        Ok(())
    }

    pub fn acquisition_session_start(
        &mut self, id: &str, anchor: &str, query_vector: Vec<f32>,
        max_depth: usize, batch_size: usize,
    ) -> Result<AcquisitionBatch, String> {
        self.require_active_acquisition()?;
        if id.trim().is_empty() || id.len() > 128 || self.acquisition_sessions.contains_key(id) {
            return Err("Invalid or duplicate acquisition session ID".into());
        }
        if max_depth == 0 || max_depth > 8 || batch_size == 0 || batch_size > 20 {
            return Err("Invalid acquisition session bounds".into());
        }
        // Check vectors, graph and bounds before mutating the state.
        let batch = self.acquisition_preview(
            anchor, &query_vector, AcquisitionFeedback::Initial,
            &BTreeSet::new(), max_depth, batch_size,
        )?;
        let seen: BTreeSet<_> = batch.results.iter().map(|item| item.token_id.clone()).collect();
        self.acquisition_sessions.insert(id.into(), AcquisitionSession {
            id: id.into(), anchor: anchor.into(), query_vector,
            external_hints: Vec::new(),
            max_depth, batch_size, seen, rounds: 1, closed: false,
            last_feedback: AcquisitionFeedback::Initial,
            last_disposition: Some(batch.disposition),
        });
        Ok(batch)
    }

    pub fn acquisition_session_feedback(
        &mut self, id: &str, feedback: AcquisitionFeedback,
    ) -> Result<AcquisitionBatch, String> {
        self.require_active_acquisition()?;
        if feedback == AcquisitionFeedback::Initial {
            return Err("Initial is reserved for session start".into());
        }
        let session = self.acquisition_sessions.get(id)
            .ok_or("Unknown acquisition session")?.clone();
        if session.closed { return Err("Acquisition session is closed".into()); }
        if session.rounds >= 64 { return Err("Acquisition session round limit reached".into()); }
        let batch = self.acquisition_preview(
            &session.anchor, &session.query_vector, feedback,
            &session.seen, session.max_depth, session.batch_size,
        )?;
        let current = self.acquisition_sessions.get_mut(id)
            .ok_or("Acquisition session disappeared")?;
        for item in &batch.results { current.seen.insert(item.token_id.clone()); }
        current.rounds += 1;
        current.last_feedback = feedback;
        current.last_disposition = Some(batch.disposition);
        // ExternalSourceCandidate is a hand-off, not a terminal outcome:
        // an authorized external hint must still be able to reactivate
        // this very session. Only explicit completion closes it.
        if batch.disposition == AcquisitionDisposition::Complete {
            current.closed = true;
        }
        Ok(batch)
    }

    /// An authorized external tool can supply a new retrieval embedding after
    /// encountering a clue. This operation does NOT ingest that clue, validate
    /// its factual accuracy, execute the tool, or erase already delivered tokens.
    /// New evidence must go through existing admission/ingestion separately.
    pub fn acquisition_session_reactivate(
        &mut self,
        id: &str,
        new_query_vector: Vec<f32>,
        source_reference: &str,
        reason: &str,
    ) -> Result<AcquisitionBatch, String> {
        self.require_active_acquisition()?;
        if source_reference.trim().is_empty() || source_reference.len() > 512
            || reason.trim().is_empty() || reason.len() > 512 {
            return Err("External hint requires bounded provenance and reason".into());
        }
        let session = self.acquisition_sessions.get(id)
            .ok_or("Unknown acquisition session")?.clone();
        if session.closed { return Err("Acquisition session is closed".into()); }
        if session.rounds >= 64 { return Err("Acquisition session round limit reached".into()); }
        if new_query_vector.len() != session.query_vector.len() {
            return Err("External hint embedding dimension mismatch".into());
        }
        // Compute a complete read-only preview first. If embedding is invalid,
        // the persistent session remains untouched.
        let batch = self.acquisition_preview(
            &session.anchor, &new_query_vector, AcquisitionFeedback::Insufficient,
            &session.seen, session.max_depth, session.batch_size,
        )?;
        let current = self.acquisition_sessions.get_mut(id)
            .ok_or("Acquisition session disappeared")?;
        current.query_vector = new_query_vector;
        current.external_hints.push(ExternalAcquisitionHint {
            source_reference: source_reference.to_owned(),
            reason: reason.to_owned(),
        });
        current.rounds += 1;
        current.last_feedback = AcquisitionFeedback::Insufficient;
        current.last_disposition = Some(batch.disposition);
        for candidate in &batch.results { current.seen.insert(candidate.token_id.clone()); }
        // ExternalSourceCandidate is a hand-off, not a terminal outcome:
        // an authorized external hint must still be able to reactivate
        // this very session. Only explicit completion closes it.
        if batch.disposition == AcquisitionDisposition::Complete {
            current.closed = true;
        }
        Ok(batch)
    }

    pub fn acquisition_session_get(&self, id: &str) -> Option<&AcquisitionSession> {
        self.acquisition_sessions.get(id)
    }

    pub fn acquisition_session_close(&mut self, id: &str) -> Result<(), String> {
        self.require_active_acquisition()?;
        let session = self.acquisition_sessions.get_mut(id)
            .ok_or("Unknown acquisition session")?;
        session.closed = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;
    const UUID: &str = "12345678-1234-1234-1234-123456789abc";
    #[test]
    fn standby_and_sleep_forbid_session_mutation() {
        let mut m=Memory::new();
        m.insert(StateToken{id:"root".into(),vector:vec![1.,0.]}).unwrap();
        m.acquisition_session_start("s","root",vec![1.,0.],3,1).unwrap();
        m.lifecycle_request_maintenance();
        m.lifecycle_enter_standby(0).unwrap();
        let before=serde_json::to_value(&m).unwrap();
        assert!(m.acquisition_session_feedback("s",AcquisitionFeedback::Insufficient).is_err());
        assert!(m.acquisition_session_reactivate("s",vec![1.,0.],"api:x","clue").is_err());
        assert!(m.acquisition_session_close("s").is_err());
        assert_eq!(before,serde_json::to_value(&m).unwrap());
        m.lifecycle_begin_sleep(true).unwrap();
        assert!(m.acquisition_session_start("other","root",vec![1.,0.],3,1).is_err());
        m.lifecycle_wake();
        assert!(m.acquisition_session_feedback("s",AcquisitionFeedback::Insufficient).is_ok());
    }
    #[test]
    fn session_survives_persistence_and_feedback_expands_graph() {
        let mut m = Memory::new();
        for (id, v) in [("root",vec![0.,1.]),("near",vec![-1.,0.]),
            ("far",vec![1.,0.])] {
            m.insert(StateToken{id:id.into(),vector:v}).unwrap();
        }
        m.reinforce("root","near",0.8).unwrap();
        m.reinforce("near","far",0.8).unwrap();
        let a = m.acquisition_session_start("s", "root", vec![1.,0.], 3, 2).unwrap();
        assert_eq!(a.results.len(), 1);
        assert_eq!(a.results[0].token_id, "near");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{UUID}.tme"));
        crate::state_container::create(&path,UUID,"ACADEMY",&m).unwrap();
        let mut loaded=crate::state_container::load(&path,UUID,"ACADEMY").unwrap();
        assert_eq!(loaded.acquisition_session_get("s").unwrap().seen.len(),1);
        let b=loaded.acquisition_session_feedback("s",AcquisitionFeedback::Insufficient).unwrap();
        assert_eq!(b.results[0].token_id,"far");
        let c=loaded.acquisition_session_feedback("s",AcquisitionFeedback::Sufficient).unwrap();
        assert_eq!(c.disposition,AcquisitionDisposition::Complete);
        assert!(loaded.acquisition_session_get("s").unwrap().closed);
    }
    #[test]
    fn rejected_feedback_does_not_mutate_session() {
        let mut m=Memory::new();
        m.insert(StateToken{id:"root".into(),vector:vec![1.,0.]}).unwrap();
        m.acquisition_session_start("s","root",vec![1.,0.],3,2).unwrap();
        let before=serde_json::to_value(&m).unwrap();
        assert!(m.acquisition_session_feedback("s",AcquisitionFeedback::Initial).is_err());
        assert_eq!(before,serde_json::to_value(&m).unwrap());
    }
    #[test]
    fn external_hint_reactivates_existing_memory_without_duplicating_tokens() {
        let mut m=Memory::new();
        for (id,v) in [
            ("root",vec![0.,1.]),
            ("old",vec![-1.,0.]),
            ("recalled",vec![1.,0.]),
        ] {
            m.insert(StateToken{id:id.into(),vector:v}).unwrap();
        }
        m.reinforce("root","old",0.5).unwrap();
        m.reinforce("root","recalled",0.5).unwrap();
        let first=m.acquisition_session_start("s","root",vec![-1.,0.],3,1).unwrap();
        assert_eq!(first.results[0].token_id,"old");
        let states_before=m.states.len();
        let batch=m.acquisition_session_reactivate(
            "s",vec![1.,0.],"library:item-42","New clue: repaired clock",
        ).unwrap();
        assert_eq!(batch.results[0].token_id,"recalled");
        assert_eq!(m.states.len(),states_before);
        assert_eq!(m.acquisition_session_get("s").unwrap().external_hints.len(),1);
        let restored:Memory=serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(restored.acquisition_session_get("s").unwrap().external_hints[0].source_reference,"library:item-42");
    }

    #[test]
    fn invalid_external_hint_preserves_session_exactly() {
        let mut m=Memory::new();
        m.insert(StateToken{id:"root".into(),vector:vec![1.,0.]}).unwrap();
        m.acquisition_session_start("s","root",vec![1.,0.],3,1).unwrap();
        let before=serde_json::to_value(&m).unwrap();
        assert!(m.acquisition_session_reactivate("s",vec![f32::NAN,0.],"x","reason").is_err());
        assert!(m.acquisition_session_reactivate("s",vec![1.],"x","reason").is_err());
        assert!(m.acquisition_session_reactivate("s",vec![1.,0.],"","reason").is_err());
        assert_eq!(before,serde_json::to_value(&m).unwrap());
    }

}

#[cfg(test)]
mod stage7_tests {
    use super::*;
    use crate::StateToken;

    #[test]
    fn external_handoff_keeps_session_open_for_new_clue() {
        let mut memory=Memory::new();
        for (id, v) in [("root",vec![0.,1.]),("a",vec![-1.,0.])] {
            memory.insert(StateToken{id:id.into(),vector:v}).unwrap();
        }
        memory.reinforce("root","a",0.5).unwrap();
        memory.acquisition_session_start("s","root",vec![-1.,0.],3,1).unwrap();
        let handoff=memory.acquisition_session_feedback(
            "s",AcquisitionFeedback::Insufficient).unwrap();
        assert_eq!(handoff.disposition,AcquisitionDisposition::ExternalSourceCandidate);
        assert!(!memory.acquisition_session_get("s").unwrap().closed);
        // The clue itself is not new evidence or a duplicated token.
        let before=memory.states.len();
        let _=memory.acquisition_session_reactivate(
            "s",vec![1.,0.],"library:clue","Alternative search formulation").unwrap();
        assert_eq!(memory.states.len(),before);
        assert_eq!(memory.acquisition_session_get("s").unwrap().external_hints.len(),1);
    }
}
