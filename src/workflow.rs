//! Small transactional coordinator for feedback and maintenance.
//!
//! Acquisition remains Learning-B owned, Reflection/SLEEP share maintenance,
//! and semantic Abstraction remains a separate reviewed action. No I/O, LLM
//! invocation, tool execution, or semantic truth judgment happens here.
use crate::{
    acquisition::{AcquisitionBatch, AcquisitionDisposition, AcquisitionFeedback},
    maintenance::{MaintenanceAssessment, MaintenanceMode, MaintenanceSignal,
        MaintenanceCommitReport, MaintenanceFindingKind},
    lifecycle::TmePhase,
    Memory,
};
use serde::{Serialize, Deserialize};


#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowFeedbackResult {
    pub acquisition: AcquisitionBatch,
    pub assessment: MaintenanceAssessment,
    /// A suggestion for the external Soul/ToolGateway, not an authorization.
    pub external_lookup_candidate: bool,
    /// Exact duplicates are only candidates; no automatic semantic merge.
    pub duplicate_candidates: Vec<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SleepPassResult {
    pub maintenance: MaintenanceCommitReport,
    pub sleep_pass_number: u64,
    pub review_still_pending: bool,
}

/// Read-only handoff to the semantic Abstraction process.
/// A candidate is NOT an approved merge and does not generate summary text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbstractionHandoff {
    pub source_token_ids: Vec<String>,
    pub source_records: Vec<(String, String, String)>,
    pub reason: String,
    pub requires_semantic_review: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbstractionHandoffView {
    pub handoffs: Vec<AbstractionHandoff>,
    pub findings_total: usize,
    pub findings_truncated: bool,
}

impl Memory {
    /// Reflection and SLEEP share the same read-only candidate selection.
    /// Exact text matches are possible compression candidates, not an assertion
    /// that independent sources corroborate one another. Non-text findings
    /// stay with their specialized owner rather than being summarized here.
    pub fn workflow_abstraction_handoffs(
        &self,
        mode: MaintenanceMode,
        signal: MaintenanceSignal,
        budget: usize,
    ) -> Result<AbstractionHandoffView, String> {
        let assessment = self.maintenance_assess(mode, signal, budget)?;
        let mut handoffs = Vec::new();
        for finding in &assessment.findings {
            if finding.kind != MaintenanceFindingKind::ExactTextDuplicate {
                continue;
            }
            let mut records = Vec::new();
            for id in &finding.token_ids {
                let entry = self.text_memories.get(id)
                    .ok_or_else(|| format!("Missing candidate text: {id}"))?;
                records.push((id.clone(), entry.text.clone(), entry.source.clone()));
            }
            if records.len() >= 2 {
                handoffs.push(AbstractionHandoff {
                    source_token_ids: finding.token_ids.clone(),
                    source_records: records,
                    reason: "Exact text and source; review whether compression is useful".into(),
                    requires_semantic_review: true,
                });
            }
        }
        Ok(AbstractionHandoffView {
            handoffs,
            findings_total: assessment.findings_total,
            findings_truncated: assessment.truncated,
        })
    }

    /// Process feedback and examine current maintenance findings in one
    /// validated state transaction. The caller alone decides whether external
    /// tools should be invoked; the TME never executes them here.
    pub fn workflow_feedback(
        &mut self,
        session_id: &str,
        feedback: AcquisitionFeedback,
        signal: MaintenanceSignal,
        assessment_budget: usize,
    ) -> Result<WorkflowFeedbackResult, String> {
        if assessment_budget == 0 {
            return Err("Workflow assessment budget must be positive".into());
        }
        if self.lifecycle.phase != TmePhase::Active {
            return Err("Feedback workflow requires ACTIVE phase".into());
        }
        let mut staged = self.clone();
        let acquisition = staged.acquisition_session_feedback(session_id, feedback)?;
        let assessment = staged.maintenance_assess(
            MaintenanceMode::Reflection, signal, assessment_budget,
        )?;
        let duplicate_candidates = assessment.findings.iter()
            .filter(|f| f.kind == MaintenanceFindingKind::ExactTextDuplicate)
            .map(|f| f.token_ids.clone()).collect();
        // Do not lose work due to a bounded assessment: it may have been truncated.
        if assessment.findings_total > 0 {
            staged.lifecycle_request_maintenance();
        }
        staged.validate_state_metadata()?;
        let external_lookup_candidate =
            acquisition.disposition == AcquisitionDisposition::ExternalSourceCandidate;
        *self = staged;
        Ok(WorkflowFeedbackResult {
            acquisition, assessment, external_lookup_candidate, duplicate_candidates,
        })
    }

    /// State-driven SLEEP: assess and acknowledge only deterministic pending
    /// operations, then finish the sleep pass. Unresolved findings survive.
    /// Atomic clone-and-swap prevents half-completed pass counters on error.
    pub fn workflow_sleep_pass(&mut self, budget: usize) -> Result<SleepPassResult, String> {
        if budget == 0 { return Err("Sleep pass budget must be positive".into()); }
        if self.lifecycle.phase != TmePhase::Sleep {
            return Err("Sleep pass requires SLEEP phase".into());
        }
        let mut staged = self.clone();
        let maintenance = staged.maintenance_review_operations(MaintenanceMode::Sleep, budget)?;
        staged.lifecycle_finish_sleep()?;
        staged.validate_state_metadata()?;
        let result = SleepPassResult {
            maintenance,
            sleep_pass_number: staged.lifecycle.completed_sleep_passes,
            review_still_pending: staged.lifecycle.pending_maintenance,
        };
        *self = staged;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn fixture() -> Memory {
        let mut m = Memory::new();
        for (id, v) in [
            ("root", vec![0., 1.]),
            ("near", vec![-1., 0.]),
            ("far", vec![1., 0.]),
        ] {
            m.insert(StateToken {id:id.into(),vector:v}).unwrap();
        }
        m.reinforce("root", "near", 0.5).unwrap();
        m.reinforce("near", "far", 0.5).unwrap();
        m
    }

    #[test]
    fn both_profiles_expose_same_abstraction_candidates_without_mutation() {
        use crate::text_ingestion::{TextDocument, TextSegment};
        let mut m = Memory::new();
        for (anchor, token) in [("doc-a", "t-a"), ("doc-b", "t-b")] {
            m.ingest_text(&TextDocument {
                anchor_id: anchor.into(),
                title: "Document".into(),
                anchor_vector: vec![1.0, 0.0],
                source: "original-source".into(),
                segments: vec![TextSegment {
                    id: token.into(), text: "Same fact".into(),
                    vector: vec![0.0, 1.0],
                }],
            }).unwrap();
        }
        let before = serde_json::to_value(&m).unwrap();
        let a = m.workflow_abstraction_handoffs(
            MaintenanceMode::Reflection, MaintenanceSignal::Routine, 100,
        ).unwrap();
        let b = m.workflow_abstraction_handoffs(
            MaintenanceMode::Sleep, MaintenanceSignal::Routine, 100,
        ).unwrap();
        assert_eq!(a.handoffs, b.handoffs);
        assert_eq!(a.handoffs.len(), 1);
        assert_eq!(a.handoffs[0].source_token_ids, vec!["t-a", "t-b"]);
        assert!(a.handoffs[0].requires_semantic_review);
        assert_eq!(serde_json::to_value(&m).unwrap(), before);
    }

    #[test]
    fn candidate_budget_does_not_misreport_exhaustiveness() {
        let mut m = fixture();
        m.record_token_operation("near").unwrap();
        let view = m.workflow_abstraction_handoffs(
            MaintenanceMode::Reflection, MaintenanceSignal::Routine, 1,
        ).unwrap();
        assert!(view.handoffs.is_empty());
        assert!(view.findings_total >= 1);
        assert!(m.workflow_abstraction_handoffs(
            MaintenanceMode::Sleep, MaintenanceSignal::Routine, 0,
        ).is_err());
    }

    #[test]
    fn feedback_advances_session_and_retains_maintenance() {
        let mut m=fixture();
        m.record_token_operation("near").unwrap();
        m.acquisition_session_start("s", "root", vec![1.,0.], 3, 1).unwrap();
        let result=m.workflow_feedback("s",AcquisitionFeedback::Insufficient,
            MaintenanceSignal::Urgent, 10).unwrap();
        assert_eq!(result.acquisition.results[0].token_id,"far");
        assert!(!result.external_lookup_candidate);
        assert!(result.assessment.urgent);
        assert!(m.lifecycle.pending_maintenance);
        assert_eq!(m.dynamics("near").unwrap().pending_operations(),1);
    }

    #[test]
    fn bad_feedback_is_atomic() {
        let mut m=fixture();
        m.acquisition_session_start("s", "root", vec![1.,0.], 3, 1).unwrap();
        let before=serde_json::to_value(&m).unwrap();
        assert!(m.workflow_feedback("s",AcquisitionFeedback::Initial,
            MaintenanceSignal::Routine,10).is_err());
        assert_eq!(before,serde_json::to_value(&m).unwrap());
    }

    #[test]
    fn sleep_pass_acknowledges_operations_without_erasing_context() {
        let mut m=fixture();
        m.record_token_operation("near").unwrap();
        m.lifecycle_request_maintenance();
        m.lifecycle_enter_standby(0).unwrap();
        m.lifecycle_begin_sleep(true).unwrap();
        let states_before=m.states.clone();
        let result=m.workflow_sleep_pass(10).unwrap();
        assert_eq!(result.maintenance.acknowledged,vec!["near"]);
        assert!(!result.review_still_pending);
        assert_eq!(result.sleep_pass_number,1);
        assert_eq!(m.lifecycle.phase,TmePhase::Standby);
        assert_eq!(m.states,states_before);
    }

    #[test]
    fn sleep_rejects_invalid_phase_and_budget_without_mutation() {
        let mut m=fixture();
        let before=serde_json::to_value(&m).unwrap();
        assert!(m.workflow_sleep_pass(10).is_err());
        assert!(m.workflow_sleep_pass(0).is_err());
        assert_eq!(before,serde_json::to_value(&m).unwrap());
    }
}
