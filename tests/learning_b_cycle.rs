use transformer_memory_engine::{
    experience::EvidenceSource,
    learning_b_admission::{AdmissionDecision, AdmissionProposal},
    learning_b_events::{ExecutionEvent, ExecutionOutcome},
    Memory,
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

#[test]
fn admitted_state_is_reused_by_a_later_task() {
    let mut memory = Memory::new();

    // First execution: the state does not exist yet.
    let first = memory
        .learning_b_resolve_events(&[vec![event("task-1", "event-1", "response.verify")]])
        .unwrap();

    assert_eq!(first.known.len(), 0);
    assert_eq!(first.unresolved.len(), 1);

    // The semantic representation is supplied externally.
    let plan = memory
        .learning_b_admission_preview(
            &first,
            &[AdmissionProposal {
                candidate_id: "response.verify".into(),
                decision: AdmissionDecision::Create {
                    vector: vec![0.0, 1.0],
                },
            }],
        )
        .unwrap();

    let committed = memory
        .learning_b_commit_admission(&plan, &["response.verify".into()])
        .unwrap();

    assert_eq!(committed.inserted, vec!["response.verify"]);

    // Second execution: same token, different task.
    let second = memory
        .learning_b_resolve_events(&[vec![event("task-2", "event-1", "response.verify")]])
        .unwrap();

    assert_eq!(second.known.len(), 1);
    assert!(second.unresolved.is_empty());
    assert_eq!(second.known[0].state_id, "response.verify");
    assert_eq!(memory.states.len(), 1);
}

#[test]
fn admitted_state_survives_persistence() {
    let mut memory = Memory::new();

    let first = memory
        .learning_b_resolve_events(&[vec![event("task-1", "event-1", "response.verify")]])
        .unwrap();

    let plan = memory
        .learning_b_admission_preview(
            &first,
            &[AdmissionProposal {
                candidate_id: "response.verify".into(),
                decision: AdmissionDecision::Create {
                    vector: vec![0.0, 1.0],
                },
            }],
        )
        .unwrap();

    memory
        .learning_b_commit_admission(&plan, &["response.verify".into()])
        .unwrap();

    let snapshot = serde_json::to_string(&memory).unwrap();
    let restored: Memory = serde_json::from_str(&snapshot).unwrap();

    let second = restored
        .learning_b_resolve_events(&[vec![event("task-2", "event-1", "response.verify")]])
        .unwrap();

    assert_eq!(second.known.len(), 1);
    assert!(second.unresolved.is_empty());
}
