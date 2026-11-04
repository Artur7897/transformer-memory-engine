//! Shared Reflection/SLEEP assessment kernel. No semantic truth judgments.
//! Assessment remains read-only; actions must pass their own commit checks.
use crate::{Memory, reflect::ReflectReport, token_architecture::TokenKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceMode { Reflection, Sleep }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceSignal { Routine, Urgent }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceFindingKind { PendingOperations, MixedTransition, UnresolvedExperience, ExactTextDuplicate }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceFinding {
    pub kind: MaintenanceFindingKind,
    pub token_ids: Vec<String>,
    pub note: String,
    pub requires_semantics: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceAssessment {
    pub mode: MaintenanceMode,
    pub urgent: bool,
    pub findings: Vec<MaintenanceFinding>,
    pub findings_total: usize,
    pub truncated: bool,
}

impl Memory {
    /// Both modes share *exactly* this deterministic analysis. Difference is
    /// the caller's inspection budget, not a separate model of memory truth.
    /// An urgent event may be assessed during Reflection immediately.
    pub fn maintenance_assess(
        &self,
        mode: MaintenanceMode,
        signal: MaintenanceSignal,
        finding_budget: usize,
    ) -> Result<MaintenanceAssessment, String> {
        if finding_budget == 0 { return Err("Maintenance budget must be positive".into()); }
        let report: ReflectReport = self.reflect_preview()?;
        let mut findings = Vec::new();
        for token in report.pending_tokens {
            findings.push(MaintenanceFinding {
                kind: MaintenanceFindingKind::PendingOperations,
                token_ids: vec![token.state_id],
                note: format!("{} operations pending inspection", token.pending_operations),
                requires_semantics: false,
            });
        }
        for relation in report.mixed_transitions {
            findings.push(MaintenanceFinding {
                kind: MaintenanceFindingKind::MixedTransition,
                token_ids: vec![relation.from, relation.to],
                note: "Observed and missed transitions coexist; no truth decision".into(),
                requires_semantics: false,
            });
        }
        for id in report.pending_experience_ids {
            findings.push(MaintenanceFinding {
                kind: MaintenanceFindingKind::UnresolvedExperience,
                token_ids: vec![id],
                note: "Experience outcome unresolved".into(),
                requires_semantics: false,
            });
        }
        // A precise duplicate candidate means exact text and same explicit
        // source. Cross-source paraphrases need separate semantic Abstraction.
        // Do not merge or delete: graph links and provenance are still owned
        // by the state validator and the relevant commit mechanism.
        let mut by_identity: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
        for (id, entry) in &self.text_memories {
            if self.token_kind(id) != Some(TokenKind::Memory) || self.is_anchor(id) == Some(true) { continue; }
            by_identity.entry((entry.text.clone(), entry.source.clone()))
                .or_default().push(id.clone());
        }
        for (_, ids) in by_identity {
            if ids.len() > 1 {
                findings.push(MaintenanceFinding {
                    kind: MaintenanceFindingKind::ExactTextDuplicate,
                    token_ids: ids,
                    note: "Exact content and source; candidate for provenance-safe consolidation".into(),
                    requires_semantics: false,
                });
            }
        }
        findings.sort_by(|a, b| a.token_ids.cmp(&b.token_ids).then_with(|| a.note.cmp(&b.note)));
        let total = findings.len();
        findings.truncate(finding_budget);
        Ok(MaintenanceAssessment {
            mode, urgent: signal == MaintenanceSignal::Urgent,
            findings, findings_total: total,
            truncated: total > finding_budget,
        })
    }
}


/// Deterministic maintenance commit: mark only the exact operation counts that
/// were inspected. No token deletion, weight change, or semantic judgment.
/// The expected count prevents a stale inspection from acknowledging later work.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationAcknowledgement {
    pub token_id: String,
    pub expected_operations: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceCommitReport {
    pub acknowledged: Vec<String>,
    pub outstanding_findings: usize,
    pub additional_review_required: bool,
}

impl Memory {
    /// Both Reflection and SLEEP use this exact commit path. A caller must
    /// explicitly approve each inspected counter value. The entire batch is
    /// checked before applying any mutation.
    pub fn maintenance_acknowledge_operations(
        &mut self,
        acknowledgements: &[OperationAcknowledgement],
    ) -> Result<MaintenanceCommitReport, String> {
        use std::collections::BTreeSet;
        let mut unique = BTreeSet::new();
        for entry in acknowledgements {
            if !unique.insert(entry.token_id.as_str()) {
                return Err("Duplicate maintenance acknowledgement".into());
            }
            let dynamics = self.dynamics(&entry.token_id)
                .ok_or("Maintenance references unknown token")?;
            if entry.expected_operations != dynamics.operations
                || entry.expected_operations <= dynamics.last_reflected_operations {
                return Err("Stale or already completed maintenance acknowledgement".into());
            }
        }
        let mut staged = self.clone();
        for entry in acknowledgements {
            let dyn_state = staged.token_dynamics.entry(entry.token_id.clone()).or_default();
            dyn_state.last_reflected_operations = entry.expected_operations;
        }
        staged.validate_state_metadata()?;
        let remaining = staged.maintenance_assess(
            MaintenanceMode::Sleep, MaintenanceSignal::Routine, usize::MAX,
        )?;
        let report = MaintenanceCommitReport {
            acknowledged: acknowledgements.iter().map(|e| e.token_id.clone()).collect(),
            outstanding_findings: remaining.findings_total,
            additional_review_required: remaining.findings_total > 0,
        };
        *self = staged;
        Ok(report)
    }

    /// Convenience method for a bounded, deterministic inspection pass.
    /// Findings such as duplicate text or contradictions are NOT automatically
    /// dismissed. They remain visible for Abstraction or specialist review.
    pub fn maintenance_review_operations(
        &mut self, mode: MaintenanceMode, budget: usize,
    ) -> Result<MaintenanceCommitReport, String> {
        let assessment = self.maintenance_assess(mode, MaintenanceSignal::Routine, budget)?;
        let mut approvals = Vec::new();
        for finding in &assessment.findings {
            if finding.kind == MaintenanceFindingKind::PendingOperations {
                for id in &finding.token_ids {
                    let d = self.dynamics(id).ok_or("Missing token dynamics")?;
                    approvals.push(OperationAcknowledgement {
                        token_id: id.clone(), expected_operations: d.operations,
                    });
                }
            }
        }
        self.maintenance_acknowledge_operations(&approvals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;
    #[test]
    fn modes_share_same_findings_and_remain_read_only() {
        let mut m = Memory::new();
        m.insert(StateToken {id:"a".into(), vector:vec![1.0,0.0]}).unwrap();
        m.record_token_operation("a").unwrap();
        let before = serde_json::to_value(&m).unwrap();
        let r = m.maintenance_assess(MaintenanceMode::Reflection, MaintenanceSignal::Urgent, 1).unwrap();
        let s = m.maintenance_assess(MaintenanceMode::Sleep, MaintenanceSignal::Routine, 20).unwrap();
        assert_eq!(r.findings, s.findings);
        assert!(r.urgent);
        assert!(!s.urgent);
        assert_eq!(before, serde_json::to_value(&m).unwrap());
    }
    #[test]
    fn budgets_are_explicit() {
        let mut m = Memory::new();
        for id in ["a", "b"] {
            m.insert(StateToken {id:id.into(), vector:vec![1.0,0.0]}).unwrap();
            m.record_token_operation(id).unwrap();
        }
        let r = m.maintenance_assess(MaintenanceMode::Reflection, MaintenanceSignal::Routine, 1).unwrap();
        assert_eq!(r.findings_total, 2);
        assert_eq!(r.findings.len(), 1);
        assert!(r.truncated);
        assert!(m.maintenance_assess(MaintenanceMode::Sleep, MaintenanceSignal::Routine, 0).is_err());
    }
}

#[cfg(test)]
mod stage6_tests {
    use super::*;
    use crate::StateToken;
    fn fixture() -> Memory {
        let mut m=Memory::new();
        for id in ["a","b"] {
            m.insert(StateToken {id:id.into(),vector:vec![1.0,0.0]}).unwrap();
            m.record_token_operation(id).unwrap();
        }
        m
    }
    #[test]
    fn bounded_reflection_and_sleep_share_commit_semantics() {
        let mut m=fixture();
        let r=m.maintenance_review_operations(MaintenanceMode::Reflection,1).unwrap();
        assert_eq!(r.acknowledged.len(),1);
        assert!(r.additional_review_required);
        let r=m.maintenance_review_operations(MaintenanceMode::Sleep,10).unwrap();
        assert_eq!(r.acknowledged.len(),1);
        assert!(!r.additional_review_required);
        assert_eq!(m.dynamics("a").unwrap().pending_operations(),0);
        assert_eq!(m.dynamics("b").unwrap().pending_operations(),0);
    }
    #[test]
    fn stale_or_duplicate_commit_is_atomic() {
        let mut m=fixture();
        let a=OperationAcknowledgement{token_id:"a".into(),expected_operations:1};
        let bad=OperationAcknowledgement{token_id:"b".into(),expected_operations:4};
        let before=serde_json::to_value(&m).unwrap();
        assert!(m.maintenance_acknowledge_operations(&[a.clone(),bad]).is_err());
        assert!(m.maintenance_acknowledge_operations(&[a.clone(),a.clone()]).is_err());
        assert_eq!(serde_json::to_value(&m).unwrap(),before);
        m.record_token_operation("a").unwrap();
        assert!(m.maintenance_acknowledge_operations(&[a]).is_err());
        assert_eq!(m.dynamics("a").unwrap().pending_operations(),2);
    }
    #[test]
    fn reconciliation_does_not_change_weights_or_tokens() {
        let mut m=fixture();
        m.set_token_dynamics("a",0.3,0.8).unwrap();
        let tokens=m.states.clone();
        let before=m.dynamics("a").unwrap();
        m.maintenance_review_operations(MaintenanceMode::Sleep,10).unwrap();
        let after=m.dynamics("a").unwrap();
        assert_eq!(m.states,tokens);
        assert_eq!(after.activation,before.activation);
        assert_eq!(after.stability,before.stability);
    }
}
