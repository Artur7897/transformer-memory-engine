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
            id: "scaling_concept".into(),
            vector: vec![1.0, 0.0],
        })
        .unwrap();

    let start = Instant::now();

    for index in 0..count {
        memory
            .record_retention_event(RetentionEvent {
                id: format!("event-{index}"),
                token_id: "scaling_concept".into(),
                attempted_at_unix_ms: index as i64,
                outcome: RecallOutcome::Success,
                reference: format!("benchmark/{index}"),
            })
            .unwrap();
    }

    let ingestion_ms = start.elapsed().as_secs_f64() * 1000.0;

    let start = Instant::now();

    for _ in 0..3 {
        let report = memory
            .analyze_retention_as_of("scaling_concept", count as i64, 100)
            .unwrap();

        assert_eq!(report.attempts, count);
        black_box(report);
    }

    let query_ms = start.elapsed().as_secs_f64() * 1000.0 / 3.0;

    println!("{count:>9} | {ingestion_ms:>13.3} | {query_ms:>12.3}");
}

fn main() {
    println!("TME Retention Scaling Test");
    println!("Release build; single token; derived ID index");
    println!();
    println!("   Events |  Ingestion ms | Query avg ms");
    println!("----------+---------------+-------------");

    for count in [10_000, 50_000, 100_000] {
        run(count);
    }
}
