use std::collections::BTreeMap;
use transformer_memory_engine::{
    contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
    experience::EvidenceSource,
    Memory, StateToken,
};

#[derive(Clone)]
enum Event {
    Learn(&'static str, [f32; 2]),
    Link(&'static str, &'static str),
    Activation(&'static str, f32),
    Use(&'static str),
    Evidence(&'static str, &'static str, bool),
    Probe(&'static str, &'static str),
}

#[derive(Clone)]
struct TimedEvent {
    day: u32,
    action: Event,
}

#[derive(Debug, PartialEq, Eq)]
struct Probe {
    day: u32,
    anchor: String,
    expected: String,
    semantic: String,
    contextual: Option<String>,
    evidence_count: usize,
    correct: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct Report {
    events: usize,
    recorded_uses: u64,
    probes: Vec<Probe>,
}

fn scenario() -> Vec<TimedEvent> {
    use Event::*;

    vec![
        TimedEvent {
            day: 0,
            action: Learn("client_a", [0.0, 1.0]),
        },
        TimedEvent {
            day: 0,
            action: Learn("client_b", [0.0, 1.0]),
        },
        TimedEvent {
            day: 0,
            action: Learn("record_a", [1.0, 0.0]),
        },
        TimedEvent {
            day: 0,
            action: Learn("record_b", [0.6, 0.8]),
        },
        TimedEvent {
            day: 1,
            action: Link("client_a", "record_a"),
        },
        TimedEvent {
            day: 1,
            action: Link("client_b", "record_b"),
        },
        TimedEvent {
            day: 2,
            action: Activation("record_a", 0.05),
        },
        TimedEvent {
            day: 2,
            action: Activation("record_b", 1.0),
        },
        TimedEvent {
            day: 3,
            action: Evidence("client_a", "record_a", true),
        },
        TimedEvent {
            day: 3,
            action: Evidence("client_b", "record_b", true),
        },
        TimedEvent {
            day: 30,
            action: Use("record_b"),
        },
        TimedEvent {
            day: 100,
            action: Probe("client_a", "record_a"),
        },
        TimedEvent {
            day: 365,
            action: Probe("client_b", "record_b"),
        },
    ]
}

fn relation_exists(memory: &Memory, from: &str, to: &str) -> Result<bool, String> {
    let key = serde_json::to_string(&(from, to)).map_err(|e| e.to_string())?;
    Ok(memory.relations.get(&key).is_some_and(|r| r.weight > 0.0))
}

fn replay(events: &[TimedEvent]) -> Result<Report, String> {
    let mut memory = Memory::new();
    let mut last_day = None;
    let mut recorded_uses = 0;
    let mut probes = Vec::new();

    for (index, timed) in events.iter().enumerate() {
        if last_day.is_some_and(|day| timed.day < day) {
            return Err("Events are not chronologically ordered".into());
        }
        last_day = Some(timed.day);

        match &timed.action {
            Event::Learn(id, vector) => {
                memory.insert(StateToken {
                    id: (*id).into(),
                    vector: vector.to_vec(),
                })?;
            }
            Event::Link(from, to) => {
                memory.reinforce(from, to, 1.0)?;
            }
            Event::Activation(id, activation) => {
                let stability = memory.dynamics(id).ok_or("Unknown token")?.stability;
                memory.set_token_dynamics(id, *activation, stability)?;
            }
            Event::Use(id) => {
                memory.record_token_operation(id)?;
                recorded_uses += 1;
            }
            Event::Evidence(from, to, occurred) => {
                memory.record_contextual_evidence(ContextualEvidenceRecord {
                    schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                    id: format!("evidence-{index}"),
                    from: (*from).into(),
                    to: (*to).into(),
                    occurred: *occurred,
                    observed_at_unix_ms: i64::from(timed.day) * 86_400_000,
                    available_at_unix_ms: i64::from(timed.day) * 86_400_000,
                    source: EvidenceSource::ExternalObservation,
                    reference: format!("synthetic/{index}"),
                    context: BTreeMap::new(),
                })?;
            }
            Event::Probe(anchor, expected) => {
                let query = [1.0, 0.0];
                let preview = memory.retrieval_priority_preview(&query, 4)?;
                let records: Vec<_> = preview
                    .candidates
                    .iter()
                    .filter(|c| c.token_id.starts_with("record_"))
                    .collect();

                let semantic = records
                    .first()
                    .ok_or("No semantic record")?
                    .token_id
                    .clone();

                let mut linked = Vec::new();
                for candidate in records {
                    if relation_exists(&memory, anchor, &candidate.token_id)? {
                        linked.push(candidate.token_id.clone());
                    }
                }

                // Do not guess when zero or multiple records match.
                let contextual = if linked.len() == 1 {
                    linked.into_iter().next()
                } else {
                    None
                };

                let evidence_count = if let Some(id) = &contextual {
                    memory
                        .analyze_evidence_quality_as_of(
                            anchor,
                            id,
                            i64::from(timed.day) * 86_400_000,
                        )?
                        .total_records
                } else {
                    0
                };

                probes.push(Probe {
                    day: timed.day,
                    anchor: (*anchor).into(),
                    expected: (*expected).into(),
                    correct: contextual.as_deref() == Some(*expected),
                    semantic,
                    contextual,
                    evidence_count,
                });
            }
        }
    }

    memory.validate_evidence_graph()?;

    Ok(Report {
        events: events.len(),
        recorded_uses,
        probes,
    })
}

fn main() -> Result<(), String> {
    let report = replay(&scenario())?;

    println!("TME Integrated Event Replay v0.6");
    println!("Existing core APIs, synthetic event timeline.");
    println!("No decay, Sleep or calibrated confidence.\n");

    println!("Events: {}", report.events);
    println!("Recorded uses: {}", report.recorded_uses);

    for probe in &report.probes {
        println!("{probe:?}");
    }

    let correct = report.probes.iter().filter(|p| p.correct).count();
    println!("Contextual Top-1: {correct}/{}", report.probes.len());
    println!("This is an integration fixture, not a quality benchmark.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(replay(&scenario()).unwrap(), replay(&scenario()).unwrap());
    }

    #[test]
    fn context_selects_correct_record() {
        let report = replay(&scenario()).unwrap();
        assert_eq!(report.events, 13);
        assert_eq!(report.recorded_uses, 1);
        assert_eq!(report.probes.len(), 2);
        assert!(report.probes.iter().all(|p| p.correct));
        assert!(report.probes.iter().all(|p| p.evidence_count == 1));
    }

    #[test]
    fn semantic_retrieval_alone_misses_customer_b() {
        let report = replay(&scenario()).unwrap();
        assert_eq!(report.probes[0].semantic, "record_a");
        assert_eq!(report.probes[1].semantic, "record_a");
    }

    #[test]
    fn rejects_out_of_order_events() {
        let mut events = scenario();
        events.swap(0, 12);
        assert!(replay(&events).is_err());
    }

    #[test]
    fn missing_link_is_not_reconstructed() {
        let mut events = scenario();
        events.retain(|e| !matches!(&e.action, Event::Link("client_b", "record_b")));
        let report = replay(&events).unwrap();
        assert_eq!(report.probes[1].contextual, None);
        assert!(!report.probes[1].correct);
    }
}
