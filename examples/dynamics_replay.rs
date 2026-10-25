use transformer_memory_engine::{Memory, StateToken};

#[derive(Clone)]
enum Event {
    Learn {
        id: &'static str,
        vector: [f32; 2],
    },
    SetActivation {
        id: &'static str,
        activation: f32,
    },
    Use {
        id: &'static str,
    },
    RecallProbe {
        query: [f32; 2],
        expected: &'static str,
    },
}

#[derive(Clone)]
struct TimedEvent {
    day: u32,
    event: Event,
}

#[derive(Debug, PartialEq)]
struct ProbeResult {
    day: u32,
    expected: String,
    retrieved: String,
    activation: f32,
    correct: bool,
}

#[derive(Debug, PartialEq)]
struct ReplayReport {
    processed_events: usize,
    operations: u64,
    probes: Vec<ProbeResult>,
}

fn scenario() -> Vec<TimedEvent> {
    vec![
        TimedEvent {
            day: 0,
            event: Event::Learn {
                id: "relevant",
                vector: [1.0, 0.0],
            },
        },
        TimedEvent {
            day: 0,
            event: Event::Learn {
                id: "frequent",
                vector: [0.6, 0.8],
            },
        },
        TimedEvent {
            day: 1,
            event: Event::SetActivation {
                id: "relevant",
                activation: 0.05,
            },
        },
        TimedEvent {
            day: 1,
            event: Event::SetActivation {
                id: "frequent",
                activation: 1.0,
            },
        },
        TimedEvent {
            day: 2,
            event: Event::RecallProbe {
                query: [1.0, 0.0],
                expected: "relevant",
            },
        },
        TimedEvent {
            day: 30,
            event: Event::Use { id: "frequent" },
        },
        TimedEvent {
            day: 100,
            event: Event::RecallProbe {
                query: [1.0, 0.0],
                expected: "relevant",
            },
        },
        TimedEvent {
            day: 365,
            event: Event::Use { id: "relevant" },
        },
        TimedEvent {
            day: 366,
            event: Event::RecallProbe {
                query: [1.0, 0.0],
                expected: "relevant",
            },
        },
    ]
}

fn replay(events: &[TimedEvent]) -> Result<ReplayReport, String> {
    let mut memory = Memory::new();
    let mut last_day = None;
    let mut operations = 0_u64;
    let mut probes = Vec::new();

    for timed in events {
        if last_day.is_some_and(|day| timed.day < day) {
            return Err("Events must be ordered by simulated day".into());
        }
        last_day = Some(timed.day);

        match &timed.event {
            Event::Learn { id, vector } => {
                memory.insert(StateToken {
                    id: (*id).into(),
                    vector: vector.to_vec(),
                })?;
            }
            Event::SetActivation { id, activation } => {
                let old = memory.dynamics(id).ok_or("Unknown token")?;
                memory.set_token_dynamics(id, *activation, old.stability)?;
            }
            Event::Use { id } => {
                memory.record_token_operation(id)?;
                operations += 1;
            }
            Event::RecallProbe { query, expected } => {
                let candidate = memory
                    .retrieval_priority_preview(query, 1)?
                    .candidates
                    .into_iter()
                    .next()
                    .ok_or("No retrieval candidate")?;

                probes.push(ProbeResult {
                    day: timed.day,
                    expected: (*expected).into(),
                    correct: candidate.token_id == *expected,
                    retrieved: candidate.token_id,
                    activation: candidate.activation,
                });
            }
        }
    }

    Ok(ReplayReport {
        processed_events: events.len(),
        operations,
        probes,
    })
}

fn main() -> Result<(), String> {
    let report = replay(&scenario())?;

    println!("TME Dynamics Replay v0.1");
    println!("Semantic-only retrieval; activation is diagnostic.");
    println!("Simulated days do not cause automatic time decay.");
    println!("Events: {}", report.processed_events);
    println!("Recorded uses: {}", report.operations);

    for probe in &report.probes {
        println!(
            "day={} expected={} retrieved={} activation={:.2} correct={}",
            probe.day, probe.expected, probe.retrieved, probe.activation, probe.correct
        );
    }

    let correct = report.probes.iter().filter(|p| p.correct).count();
    println!("Top-1: {correct}/{}", report.probes.len());
    println!("Toy scenario only; not a memory-quality benchmark.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_is_deterministic() {
        let first = replay(&scenario()).unwrap();
        let second = replay(&scenario()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.processed_events, 9);
        assert_eq!(first.operations, 2);
        assert_eq!(first.probes.len(), 3);
        assert!(first.probes.iter().all(|p| p.correct));
    }

    #[test]
    fn rejects_out_of_order_events() {
        let mut events = scenario();
        events.swap(0, 8);
        assert!(replay(&events).is_err());
    }

    #[test]
    fn use_does_not_implicitly_change_activation() {
        let result = replay(&scenario()).unwrap();
        assert_eq!(result.probes[0].activation, 0.05);
        assert_eq!(result.probes[2].activation, 0.05);
    }
}
