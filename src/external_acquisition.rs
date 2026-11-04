//! Typed external information hand-off. This is not a tool executor.
//! The authorized Soul/ToolGateway chooses actual sources, permissions and
//! freshness requirements. The TME never treats external output as truth.
use crate::{acquisition::AcquisitionDisposition, acquisition_session::AcquisitionSession, Memory};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ExternalSourceKind {
    Library,
    Runbook,
    Api,
    OtherEntity,
    Web,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalInformationRequest {
    pub session_id: String,
    pub anchor_id: String,
    pub allowed_source_kinds: Vec<ExternalSourceKind>,
    pub reason: String,
    /// No tool permission is implied by this request.
    pub requires_gateway_authorization: bool,
}

impl Memory {
    /// Produce a handoff only when internal retrieval explicitly reached a
    /// handoff boundary. Does not query sources or mutate TME state.
    pub fn acquisition_external_handoff(
        &self,
        session_id: &str,
        allowed_sources: &[ExternalSourceKind],
    ) -> Result<ExternalInformationRequest, String> {
        let session: &AcquisitionSession = self.acquisition_session_get(session_id)
            .ok_or("Unknown acquisition session")?;
        if session.closed {
            return Err("Closed acquisition session cannot request external context".into());
        }
        if session.last_disposition != Some(AcquisitionDisposition::ExternalSourceCandidate) {
            return Err("Internal search has not requested external context".into());
        }
        if allowed_sources.is_empty() || allowed_sources.len() > 5 {
            return Err("External source selection must be non-empty and bounded".into());
        }
        let mut kinds = allowed_sources.to_vec();
        kinds.sort_by_key(|v| *v as u8);
        kinds.dedup();
        Ok(ExternalInformationRequest {
            session_id: session_id.to_owned(),
            anchor_id: session.anchor.clone(),
            allowed_source_kinds: kinds,
            reason: "Internal retrieval has no further reachable unseen candidates".into(),
            requires_gateway_authorization: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{acquisition::AcquisitionFeedback, StateToken};
    #[test]
    fn handoff_only_after_internal_exhaustion_and_never_mutates() {
        let mut m=Memory::new();
        m.insert(StateToken{id:"root".into(),vector:vec![1.,0.]}).unwrap();
        m.insert(StateToken{id:"known".into(),vector:vec![1.,0.]}).unwrap();
        m.reinforce("root","known",0.5).unwrap();
        m.acquisition_session_start("session","root",vec![1.,0.],3,1).unwrap();
        assert!(m.acquisition_external_handoff("session", &[ExternalSourceKind::Api]).is_err());
        let batch=m.acquisition_session_feedback("session",AcquisitionFeedback::Insufficient).unwrap();
        assert_eq!(batch.disposition,AcquisitionDisposition::ExternalSourceCandidate);
        let before=serde_json::to_value(&m).unwrap();
        let handoff=m.acquisition_external_handoff("session", &[ExternalSourceKind::Web,ExternalSourceKind::Web]).unwrap();
        assert_eq!(handoff.allowed_source_kinds,vec![ExternalSourceKind::Web]);
        assert!(handoff.requires_gateway_authorization);
        assert_eq!(before,serde_json::to_value(&m).unwrap());
        assert!(m.acquisition_external_handoff("session", &[]).is_err());
    }
}
