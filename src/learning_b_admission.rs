use crate::{learning_b_resolution::ResolutionReport, Memory, StateToken};
use std::collections::{BTreeMap, BTreeSet};

/// A proposed semantic resolution supplied by an external resolver.
/// This module does not generate semantic representations.
#[derive(Clone, Debug, PartialEq)]
pub enum AdmissionDecision {
    Reuse { existing_id: String },
    Create { vector: Vec<f32> },
    Defer,
    Reject { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdmissionProposal {
    pub candidate_id: String,
    pub decision: AdmissionDecision,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AdmissionDisposition {
    Reused { existing_id: String },
    Prepared { token: StateToken },
    Deferred,
    Rejected { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdmissionEntry {
    pub candidate_id: String,
    pub disposition: AdmissionDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdmissionPlan {
    pub entries: Vec<AdmissionEntry>,
}

impl Memory {
    /// Validate external resolution proposals against the current memory.
    ///
    /// Every unresolved candidate must have exactly one proposal.
    /// No changes are made to Memory A.
    /// Prepared tokens are NOT automatically admitted or validated skills.
    pub fn learning_b_admission_preview(
        &self,
        report: &ResolutionReport,
        proposals: &[AdmissionProposal],
    ) -> Result<AdmissionPlan, String> {
        let known: BTreeSet<&str> = report
            .known
            .iter()
            .map(|entry| entry.state_id.as_str())
            .collect();

        let unresolved: BTreeSet<&str> = report
            .unresolved
            .iter()
            .map(|entry| entry.state_id.as_str())
            .collect();

        if known.len() != report.known.len()
            || unresolved.len() != report.unresolved.len()
            || !known.is_disjoint(&unresolved)
        {
            return Err("Invalid resolution report identifiers".into());
        }

        for id in &known {
            if !self.states.contains_key(*id) {
                return Err("Known state is absent from memory".into());
            }
        }

        for id in &unresolved {
            if id.trim().is_empty() || self.states.contains_key(*id) {
                return Err("Invalid unresolved candidate".into());
            }
        }

        let mut decisions = BTreeMap::new();

        for proposal in proposals {
            let id = proposal.candidate_id.as_str();

            if !unresolved.contains(id) {
                return Err(format!("Unexpected candidate: {id}"));
            }

            if decisions.insert(id, &proposal.decision).is_some() {
                return Err(format!("Duplicate proposal: {id}"));
            }
        }

        if decisions.len() != unresolved.len() {
            return Err("Missing admission proposal".into());
        }

        let mut entries = Vec::new();

        for candidate_id in unresolved {
            let decision = decisions
                .get(candidate_id)
                .ok_or("Missing candidate resolution")?;

            let disposition = match decision {
                AdmissionDecision::Reuse { existing_id } => {
                    if !self.states.contains_key(existing_id) {
                        return Err(format!("Reuse target does not exist: {existing_id}"));
                    }

                    AdmissionDisposition::Reused {
                        existing_id: existing_id.clone(),
                    }
                }

                AdmissionDecision::Create { vector } => {
                    if vector.is_empty()
                        || !vector.iter().all(|v| v.is_finite())
                        || !vector.iter().any(|v| *v != 0.0)
                    {
                        return Err(format!("Invalid semantic vector: {candidate_id}"));
                    }

                    AdmissionDisposition::Prepared {
                        token: StateToken {
                            id: candidate_id.to_owned(),
                            vector: vector.clone(),
                        },
                    }
                }

                AdmissionDecision::Defer => AdmissionDisposition::Deferred,

                AdmissionDecision::Reject { reason } => {
                    if reason.trim().is_empty() {
                        return Err("Rejection reason is empty".into());
                    }

                    AdmissionDisposition::Rejected {
                        reason: reason.clone(),
                    }
                }
            };

            entries.push(AdmissionEntry {
                candidate_id: candidate_id.to_owned(),
                disposition,
            });
        }

        Ok(AdmissionPlan { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        experience::EvidenceSource,
        learning_b_events::{ExecutionEvent, ExecutionOutcome},
    };

    fn event(task: &str, id: &str, state: &str) -> ExecutionEvent {
        ExecutionEvent {
            id: id.into(),
            task_id: task.into(),
            state_id: state.into(),
            outcome: ExecutionOutcome::Success,
            source: EvidenceSource::VerifiedToolResult,
            reference: format!("test/{task}/{id}"),
        }
    }

    fn fixture() -> (Memory, ResolutionReport) {
        let mut memory = Memory::new();

        memory
            .insert(StateToken {
                id: "api.call".into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();

        let report = memory
            .learning_b_resolve_events(&[vec![
                event("bitcoin", "e1", "api.call"),
                event("bitcoin", "e2", "http.request"),
                event("bitcoin", "e3", "response.check"),
                event("bitcoin", "e4", "unclear.action"),
            ]])
            .unwrap();

        (memory, report)
    }

    #[test]
    fn prepares_reuse_create_and_defer_without_mutation() {
        let (memory, report) = fixture();
        let before = serde_json::to_string(&memory).unwrap();

        let plan = memory
            .learning_b_admission_preview(
                &report,
                &[
                    AdmissionProposal {
                        candidate_id: "http.request".into(),
                        decision: AdmissionDecision::Reuse {
                            existing_id: "api.call".into(),
                        },
                    },
                    AdmissionProposal {
                        candidate_id: "response.check".into(),
                        decision: AdmissionDecision::Create {
                            vector: vec![0.0, 1.0],
                        },
                    },
                    AdmissionProposal {
                        candidate_id: "unclear.action".into(),
                        decision: AdmissionDecision::Defer,
                    },
                ],
            )
            .unwrap();

        assert_eq!(plan.entries.len(), 3);

        assert!(matches!(
            &plan.entries[0].disposition,
            AdmissionDisposition::Reused { existing_id }
                if existing_id == "api.call"
        ));

        assert!(matches!(
            &plan.entries[1].disposition,
            AdmissionDisposition::Prepared { token }
                if token.id == "response.check"
        ));

        assert!(matches!(
            &plan.entries[2].disposition,
            AdmissionDisposition::Deferred
        ));

        assert_eq!(before, serde_json::to_string(&memory).unwrap());
    }

    #[test]
    fn rejects_missing_and_duplicate_proposals() {
        let (memory, report) = fixture();

        assert!(memory.learning_b_admission_preview(&report, &[]).is_err());

        let duplicate = AdmissionProposal {
            candidate_id: "http.request".into(),
            decision: AdmissionDecision::Defer,
        };

        assert!(memory
            .learning_b_admission_preview(&report, &[duplicate.clone(), duplicate],)
            .is_err());
    }

    #[test]
    fn rejects_invalid_reuse_target() {
        let (memory, report) = fixture();

        let proposals = [
            AdmissionProposal {
                candidate_id: "http.request".into(),
                decision: AdmissionDecision::Reuse {
                    existing_id: "missing".into(),
                },
            },
            AdmissionProposal {
                candidate_id: "response.check".into(),
                decision: AdmissionDecision::Defer,
            },
            AdmissionProposal {
                candidate_id: "unclear.action".into(),
                decision: AdmissionDecision::Defer,
            },
        ];

        assert!(memory
            .learning_b_admission_preview(&report, &proposals)
            .is_err());
    }

    #[test]
    fn rejects_invalid_vectors() {
        let (memory, report) = fixture();

        for vector in [
            vec![],
            vec![0.0, 0.0],
            vec![f32::NAN, 1.0],
            vec![f32::INFINITY, 1.0],
        ] {
            let proposals = [
                AdmissionProposal {
                    candidate_id: "http.request".into(),
                    decision: AdmissionDecision::Create { vector },
                },
                AdmissionProposal {
                    candidate_id: "response.check".into(),
                    decision: AdmissionDecision::Defer,
                },
                AdmissionProposal {
                    candidate_id: "unclear.action".into(),
                    decision: AdmissionDecision::Defer,
                },
            ];

            assert!(memory
                .learning_b_admission_preview(&report, &proposals)
                .is_err());
        }
    }

    #[test]
    fn records_explicit_rejection_without_admission() {
        let (memory, report) = fixture();

        let plan = memory
            .learning_b_admission_preview(
                &report,
                &[
                    AdmissionProposal {
                        candidate_id: "http.request".into(),
                        decision: AdmissionDecision::Defer,
                    },
                    AdmissionProposal {
                        candidate_id: "response.check".into(),
                        decision: AdmissionDecision::Defer,
                    },
                    AdmissionProposal {
                        candidate_id: "unclear.action".into(),
                        decision: AdmissionDecision::Reject {
                            reason: "Insufficient evidence".into(),
                        },
                    },
                ],
            )
            .unwrap();

        assert!(matches!(
            &plan.entries[2].disposition,
            AdmissionDisposition::Rejected { reason }
                if reason == "Insufficient evidence"
        ));
    }
}

/// Result of an explicitly approved token admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionCommitReport {
    pub inserted: Vec<String>,
    pub reused: Vec<(String, String)>,
}

impl Memory {
    /// Commit a previously reviewed admission plan.
    ///
    /// All prepared token IDs must be explicitly approved.
    /// Validate the entire plan before changing Memory A.
    ///
    /// This is structural validation, not semantic verification.
    pub fn learning_b_commit_admission(
        &mut self,
        plan: &AdmissionPlan,
        approved_ids: &[String],
    ) -> Result<AdmissionCommitReport, String> {
        let approved: BTreeSet<&str> = approved_ids.iter().map(String::as_str).collect();

        if approved.len() != approved_ids.len() {
            return Err("Duplicate approval ID".into());
        }

        let mut seen = BTreeSet::new();
        let mut prepared = Vec::new();
        let mut reused = Vec::new();

        // Phase 1: validate the entire plan.
        for entry in &plan.entries {
            let id = entry.candidate_id.as_str();

            if id.trim().is_empty() || !seen.insert(id) {
                return Err("Empty or duplicate candidate ID".into());
            }

            match &entry.disposition {
                AdmissionDisposition::Prepared { token } => {
                    if token.id != id
                        || self.states.contains_key(id)
                        || token.vector.is_empty()
                        || !token.vector.iter().all(|v| v.is_finite())
                        || !token.vector.iter().any(|v| *v != 0.0)
                    {
                        return Err(format!("Invalid prepared token: {id}"));
                    }

                    if !approved.contains(id) {
                        return Err(format!("Token not approved: {id}"));
                    }

                    prepared.push(token.clone());
                }

                AdmissionDisposition::Reused { existing_id } => {
                    if !self.states.contains_key(existing_id) {
                        return Err(format!("Missing reuse target: {existing_id}"));
                    }

                    if approved.contains(id) {
                        return Err("Reuse cannot be approved as creation".into());
                    }

                    reused.push((id.to_owned(), existing_id.clone()));
                }

                AdmissionDisposition::Deferred | AdmissionDisposition::Rejected { .. } => {
                    if approved.contains(id) {
                        return Err("Non-prepared candidate was approved".into());
                    }
                }
            }
        }

        if approved.iter().any(|id| !seen.contains(*id)) {
            return Err("Approval references an unknown candidate".into());
        }

        // Phase 2: stage updates in the affected maps.
        // Nothing in the live memory changes until staging succeeds.
        let mut staged_states = self.states.clone();
        let mut staged_dynamics = self.token_dynamics.clone();
        let mut inserted = Vec::new();

        for token in prepared {
            let id = token.id.clone();

            if staged_states.insert(id.clone(), token).is_some() {
                return Err("Duplicate token during staging".into());
            }

            staged_dynamics.entry(id.clone()).or_default();

            inserted.push(id);
        }

        // Phase 3: commit both maps.
        self.states = staged_states;
        self.token_dynamics = staged_dynamics;

        Ok(AdmissionCommitReport { inserted, reused })
    }
}

#[cfg(test)]
mod admission_commit_tests {
    use super::*;

    fn prepared(id: &str, vector: Vec<f32>) -> AdmissionEntry {
        AdmissionEntry {
            candidate_id: id.into(),
            disposition: AdmissionDisposition::Prepared {
                token: StateToken {
                    id: id.into(),
                    vector,
                },
            },
        }
    }

    #[test]
    fn commits_approved_token_with_dynamics() {
        let mut memory = Memory::new();

        let plan = AdmissionPlan {
            entries: vec![prepared("api.call", vec![1.0, 0.0])],
        };

        let result = memory
            .learning_b_commit_admission(&plan, &["api.call".to_owned()])
            .unwrap();

        assert_eq!(result.inserted, vec!["api.call"]);
        assert!(memory.states.contains_key("api.call"));
        assert!(memory.token_dynamics.contains_key("api.call"));
    }

    #[test]
    fn rejects_partial_approval_without_mutation() {
        let mut memory = Memory::new();

        let plan = AdmissionPlan {
            entries: vec![
                prepared("api.call", vec![1.0, 0.0]),
                prepared("response.check", vec![0.0, 1.0]),
            ],
        };

        let before = serde_json::to_string(&memory).unwrap();

        assert!(memory
            .learning_b_commit_admission(&plan, &["api.call".to_owned()],)
            .is_err());

        assert_eq!(serde_json::to_string(&memory).unwrap(), before);
    }

    #[test]
    fn rejects_duplicate_candidates_atomically() {
        let mut memory = Memory::new();

        let plan = AdmissionPlan {
            entries: vec![prepared("same", vec![1.0]), prepared("same", vec![0.5])],
        };

        let before = serde_json::to_string(&memory).unwrap();

        assert!(memory
            .learning_b_commit_admission(&plan, &["same".to_owned()],)
            .is_err());

        assert_eq!(serde_json::to_string(&memory).unwrap(), before);
    }

    #[test]
    fn refuses_overwrite_of_existing_memory() {
        let mut memory = Memory::new();

        memory
            .insert(StateToken {
                id: "existing".into(),
                vector: vec![0.2, 0.8],
            })
            .unwrap();

        let before = serde_json::to_string(&memory).unwrap();

        let plan = AdmissionPlan {
            entries: vec![prepared("existing", vec![1.0, 0.0])],
        };

        assert!(memory
            .learning_b_commit_admission(&plan, &["existing".to_owned()],)
            .is_err());

        assert_eq!(serde_json::to_string(&memory).unwrap(), before);
    }
}
