use std::collections::BTreeMap;

use transformer_memory_engine::{
    contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
    experience::EvidenceSource,
    Memory, StateToken,
};

#[test]
fn execution_adjacency_is_preserved_but_not_evaluated() {
    let mut memory = Memory::new();

    for id in ["api.select", "api.call"] {
        memory
            .insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
    }

    let raw = ContextualEvidenceRecord {
        schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
        id: "execution-1".into(),
        from: "api.select".into(),
        to: "api.call".into(),
        occurred: true,
        observed_at_unix_ms: 100,
        available_at_unix_ms: 200,
        source: EvidenceSource::VerifiedToolResult,
        reference: "task-1/events".into(),
        context: BTreeMap::from([
            ("observation_kind".into(), "execution_adjacency".into()),
            ("task_id".into(), "task-1".into()),
        ]),
    };

    memory.record_contextual_evidence(raw).unwrap();

    // Original observation remains available.
    assert_eq!(memory.contextual_evidence_as_of(200).len(), 1);

    // But it is not evaluated as a confirmed transition.
    let analysis = memory
        .analyze_context_as_of("api.select", "api.call", "observation_kind", 200)
        .unwrap();

    assert!(analysis.groups.is_empty());

    let quality = memory
        .analyze_evidence_quality_as_of("api.select", "api.call", 200)
        .unwrap();

    assert_eq!(quality.total_records, 0);

    // Historical Memory serialization must preserve the record.
    let bytes = serde_json::to_vec(&memory).unwrap();
    let restored: Memory = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(restored.contextual_evidence_as_of(200).len(), 1);
}

#[test]
fn legacy_evaluated_evidence_remains_counted() {
    let mut memory = Memory::new();

    for id in ["api.select", "api.call"] {
        memory
            .insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
    }

    memory
        .record_contextual_evidence(ContextualEvidenceRecord {
            schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
            id: "evaluated-1".into(),
            from: "api.select".into(),
            to: "api.call".into(),
            occurred: true,
            observed_at_unix_ms: 100,
            available_at_unix_ms: 200,
            source: EvidenceSource::ExternalObservation,
            reference: "verified/result".into(),
            context: BTreeMap::from([("mode".into(), "test".into())]),
        })
        .unwrap();

    let analysis = memory
        .analyze_context_as_of("api.select", "api.call", "mode", 200)
        .unwrap();

    assert_eq!(analysis.groups.len(), 1);
    assert_eq!(analysis.groups[0].observed, 1);

    let quality = memory
        .analyze_evidence_quality_as_of("api.select", "api.call", 200)
        .unwrap();

    assert_eq!(quality.total_records, 1);
}
