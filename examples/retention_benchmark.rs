use std::hint::black_box;
use std::time::Instant;

use transformer_memory_engine::{
    retention::{RecallOutcome, RetentionEvent},
    Memory, StateToken,
};

fn run(count: usize) {
    let mut memory = Memory::new();

    memory
        .insert(StateToken {
            id: "benchmark_concept".into(),
            vector: vec![1.0, 0.0],
        })
        .unwrap();

    let start = Instant::now();

    for index in 0..count {
        memory
            .record_retention_event(RetentionEvent {
                id: format!("event-{index}"),
                token_id: "benchmark_concept".into(),
                attempted_at_unix_ms: index as i64,
                outcome: RecallOutcome::Success,
                reference: format!("benchmark/{index}"),
            })
            .unwrap();
    }

    let ingestion_ms = start.elapsed().as_secs_f64() * 1000.0;

    let start = Instant::now();
    for _ in 0..100 {
        let report = memory
            .analyze_retention_as_of("benchmark_concept", count as i64, 100)
            .unwrap();

        black_box(report);
    }

    let query_ms = start.elapsed().as_secs_f64() * 1000.0 / 100.0;

    let json = serde_json::to_vec(&memory).unwrap();

    println!(
        "{count:>8} | {ingestion_ms:>12.3} | \
         {query_ms:>12.3} | {:>12}",
        json.len()
    );
}

fn main() {
    println!("TME Retention Performance Baseline");
    println!("Release build, single process");
    println!();
    println!("  Events | Ingestion ms | Query avg ms | JSON bytes");
    println!("---------+--------------+--------------+-------------");

    for count in [1_000, 3_000, 6_000] {
        run(count);
    }
}
