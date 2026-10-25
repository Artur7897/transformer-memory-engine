use std::collections::BTreeMap;
use transformer_memory_engine::{
    contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
    evidence_dependencies::DependencyKind,
    experience::EvidenceSource,
    Memory, StateToken,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Assessment {
    record: String,
    observations: usize,
    positive: usize,
    negative: usize,
    repeated_reference_groups: usize,
    dependent_records: usize,
    conflict: bool,
}

fn observe(
    memory: &mut Memory,
    id: &str,
    target: &str,
    occurred: bool,
    reference: &str,
) -> Result<(), String> {
    memory.record_contextual_evidence(ContextualEvidenceRecord {
        schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
        id: id.into(),
        from: "customer".into(),
        to: target.into(),
        occurred,
        observed_at_unix_ms: 10,
        available_at_unix_ms: 20,
        source: EvidenceSource::ExternalObservation,
        reference: reference.into(),
        context: BTreeMap::new(),
    })
}

fn fixture() -> Result<Memory, String> {
    let mut memory = Memory::new();

    for id in ["customer", "documented", "copied", "unknown", "conflicting"] {
        memory.insert(StateToken {
            id: id.into(),
            vector: vec![1.0, 0.0],
        })?;
    }

    for target in ["documented", "copied", "unknown", "conflicting"] {
        memory.reinforce("customer", target, 1.0)?;
    }

    observe(&mut memory, "d1", "documented", true, "original/d1")?;

    observe(&mut memory, "c1", "copied", true, "shared/report")?;
    observe(&mut memory, "c2", "copied", true, "shared/report")?;
    memory.record_evidence_dependency("c2", "c1", DependencyKind::CopiedFrom)?;

    observe(&mut memory, "x1", "conflicting", true, "original/x1")?;
    observe(&mut memory, "x2", "conflicting", false, "original/x2")?;

    Ok(memory)
}

fn assess(memory: &Memory, record: &str, cutoff: i64) -> Result<Assessment, String> {
    let key = serde_json::to_string(&("customer", record)).map_err(|e| e.to_string())?;

    if !memory.relations.contains_key(&key) {
        return Err("No association to requested record".into());
    }

    let quality = memory.analyze_evidence_quality_as_of("customer", record, cutoff)?;

    let evidence: Vec<_> = memory
        .contextual_evidence
        .values()
        .filter(|e| e.from == "customer" && e.to == record && e.available_at_unix_ms <= cutoff)
        .collect();

    let mut dependent_records = 0;
    for entry in &evidence {
        let report = memory.evidence_dependency_report(&entry.id)?;
        if report.explicitly_linked {
            dependent_records += 1;
        }
    }

    let positive = evidence.iter().filter(|e| e.occurred).count();
    let negative = evidence.len() - positive;

    Ok(Assessment {
        record: record.into(),
        observations: evidence.len(),
        positive,
        negative,
        repeated_reference_groups: quality.repeated_reference_groups,
        dependent_records,
        conflict: positive > 0 && negative > 0,
    })
}

fn evaluate() -> Result<Vec<Assessment>, String> {
    let memory = fixture()?;
    memory.validate_evidence_graph()?;

    ["documented", "copied", "unknown", "conflicting"]
        .iter()
        .map(|id| assess(&memory, id, 100))
        .collect()
}

fn main() -> Result<(), String> {
    println!("TME Evidence-Aware Retrieval Probe v0.5");
    println!("All reported properties are descriptive.\n");

    for row in evaluate()? {
        println!("{row:?}");
    }

    println!("\nNo factual verification or confidence score inferred.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_is_deterministic() {
        assert_eq!(evaluate().unwrap(), evaluate().unwrap());
    }

    #[test]
    fn documented_observation_retains_provenance() {
        let rows = evaluate().unwrap();
        assert_eq!(rows[0].observations, 1);
        assert_eq!(rows[0].positive, 1);
        assert!(!rows[0].conflict);
    }

    #[test]
    fn copied_evidence_is_identified() {
        let rows = evaluate().unwrap();
        assert_eq!(rows[1].repeated_reference_groups, 1);
        assert_eq!(rows[1].dependent_records, 1);
    }

    #[test]
    fn missing_evidence_remains_missing() {
        let rows = evaluate().unwrap();
        assert_eq!(rows[2].observations, 0);
    }

    #[test]
    fn conflicting_observations_are_reported() {
        let rows = evaluate().unwrap();
        assert_eq!(rows[3].positive, 1);
        assert_eq!(rows[3].negative, 1);
        assert!(rows[3].conflict);
    }

    #[test]
    fn future_evidence_is_not_visible() {
        let memory = fixture().unwrap();
        let row = assess(&memory, "documented", 15).unwrap();
        assert_eq!(row.observations, 0);
    }

    #[test]
    fn unknown_association_is_rejected() {
        assert!(assess(&fixture().unwrap(), "nonexistent", 100).is_err());
    }
}
