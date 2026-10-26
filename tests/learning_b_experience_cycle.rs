use std::collections::BTreeMap;

use transformer_memory_engine::{
    experience::EvidenceSource,
    learning_b_events::{ExecutionEvent, ExecutionOutcome},
    Memory, StateToken,
};

fn event(task: &str, id: &str, state: &str) -> ExecutionEvent {
    ExecutionEvent {
        id: id.into(),
        task_id: task.into(),
        state_id: state.into(),
        outcome: ExecutionOutcome::Success,
        source: EvidenceSource::VerifiedToolResult,
        reference: format!("execution/{task}/{id}"),
    }
}

fn trace(task: &str) -> Vec<ExecutionEvent> {
    vec![
        event(task, "e1", "api.select"),
        event(task, "e2", "api.call"),
        event(task, "e3", "response.verify"),
    ]
}

#[test]
fn experiences_from_two_tasks_can_be_reactivated() {
    let mut memory = Memory::new();

    for id in ["api.select", "api.call", "response.verify"] {
        memory
            .insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
    }

    let first = trace("bitcoin-status");

    memory
        .learning_b_record_execution_evidence(
            &first,
            &BTreeMap::from([("e1".into(), 100), ("e2".into(), 200), ("e3".into(), 300)]),
            400,
        )
        .unwrap();

    // Experience from the first task is now available.
    let after_first = memory
        .learning_b_experience_activation("api.call", 400)
        .unwrap();

    assert_eq!(after_first.neighbors.len(), 1);
    assert_eq!(after_first.neighbors[0].state_id, "response.verify");
    assert_eq!(after_first.neighbors[0].task_count, 1);

    // A different task reuses the same states.
    let second = trace("bce-status");

    memory
        .learning_b_record_execution_evidence(
            &second,
            &BTreeMap::from([("e1".into(), 500), ("e2".into(), 600), ("e3".into(), 700)]),
            800,
        )
        .unwrap();

    // A query at time 400 must not see future evidence.
    let historical = memory
        .learning_b_experience_activation("api.call", 400)
        .unwrap();

    assert_eq!(historical.neighbors[0].task_count, 1);

    // Both tasks now contribute to the same observed relation.
    let current = memory
        .learning_b_experience_activation("api.call", 800)
        .unwrap();

    assert_eq!(current.neighbors.len(), 1);
    assert_eq!(current.neighbors[0].state_id, "response.verify");
    assert_eq!(current.neighbors[0].observation_count, 2);
    assert_eq!(current.neighbors[0].task_count, 2);
    assert_eq!(current.neighbors[0].evidence_ids.len(), 2);

    // No fabricated graph weights or skill validation.
    assert!(memory.relations.is_empty());
    assert!(memory.transitions.is_empty());

    // The experience survives a persisted-memory round trip.
    let snapshot = serde_json::to_vec(&memory).unwrap();
    let restored: Memory = serde_json::from_slice(&snapshot).unwrap();

    let recovered = restored
        .learning_b_experience_activation("api.call", 800)
        .unwrap();

    assert_eq!(recovered, current);
}
