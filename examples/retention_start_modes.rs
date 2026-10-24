use std::hint::black_box;
use std::time::Instant;

use transformer_memory_engine::{
    retention::{RecallOutcome, RetentionEvent},
    Memory, StateToken,
};

const EVENTS: usize = 100_000;
const TOKENS: usize = 100;

fn event(index: usize) -> RetentionEvent {
    RetentionEvent {
        id: format!("event-{index}"),
        token_id: format!("token-{}", index % TOKENS),
        attempted_at_unix_ms: (index / TOKENS) as i64,
        outcome: RecallOutcome::Success,
        reference: format!("bench/{index}"),
    }
}

fn write_new(memory: &mut Memory) {
    memory.record_retention_event(event(EVENTS)).unwrap();
}

fn main() {
    let mut original = Memory::new();

    for i in 0..TOKENS {
        original
            .insert(StateToken {
                id: format!("token-{i}"),
                vector: vec![1.0],
            })
            .unwrap();
    }

    for i in 0..EVENTS {
        original.record_retention_event(event(i)).unwrap();
    }

    let bytes = serde_json::to_vec(&original).unwrap();

    println!("TME Retention Start Modes");
    println!("Events: {EVENTS}; Tokens: {TOKENS}");
    println!("JSON bytes: {}", bytes.len());
    println!();

    // Read-only: deserialize, then query without creating an ID index.
    let t = Instant::now();
    let readonly: Memory = serde_json::from_slice(&bytes).unwrap();
    let readonly_load = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let report = readonly.retention_as_of("token-0", i64::MAX).unwrap();
    let readonly_query = t.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(report.attempts, EVENTS / TOKENS);
    black_box(report);

    // Lazy: deserialize, then build the index on first insertion.
    let t = Instant::now();
    let mut lazy: Memory = serde_json::from_slice(&bytes).unwrap();
    let lazy_load = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    write_new(&mut lazy);
    let lazy_first = t.elapsed().as_secs_f64() * 1000.0;

    // Prepared: deserialize, explicitly validate and build index,
    // then insert.
    let t = Instant::now();
    let mut prepared: Memory = serde_json::from_slice(&bytes).unwrap();
    let prepared_load = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    prepared.prepare_retention_writes().unwrap();
    let preparation = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    write_new(&mut prepared);
    let prepared_first = t.elapsed().as_secs_f64() * 1000.0;

    assert_eq!(
        serde_json::to_value(&lazy).unwrap(),
        serde_json::to_value(&prepared).unwrap()
    );

    assert!(lazy.record_retention_event(event(EVENTS)).is_err());
    assert!(prepared.record_retention_event(event(EVENTS)).is_err());

    println!("Read-only deserialize: {readonly_load:.3} ms");
    println!("Read-only query:       {readonly_query:.3} ms");
    println!("Lazy deserialize:     {lazy_load:.3} ms");
    println!("Lazy first write:     {lazy_first:.3} ms");
    println!("Prepared deserialize: {prepared_load:.3} ms");
    println!("Explicit preparation: {preparation:.3} ms");
    println!("Prepared first write: {prepared_first:.6} ms");
    println!("Consistency: PASS");
    println!();
    println!("Note: raw Serde loading is measured without");
    println!("Memory::load evidence-graph validation.");
}
