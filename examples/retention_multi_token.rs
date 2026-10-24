use std::hint::black_box;
use std::time::Instant;

use transformer_memory_engine::{
    retention::{RecallOutcome, RetentionEvent},
    Memory, StateToken,
};

const TOTAL_EVENTS: usize = 100_000;

fn run(token_count: usize) {
    assert!(token_count > 0);
    assert_eq!(TOTAL_EVENTS % token_count, 0);

    let mut memory = Memory::new();

    for index in 0..token_count {
        memory
            .insert(StateToken {
                id: format!("token-{index}"),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
    }

    let start = Instant::now();

    for index in 0..TOTAL_EVENTS {
        let token = index % token_count;

        memory
            .record_retention_event(RetentionEvent {
                id: format!("event-{index}"),
                token_id: format!("token-{token}"),
                attempted_at_unix_ms: (index / token_count) as i64,
                outcome: RecallOutcome::Success,
                reference: format!("benchmark/{index}"),
            })
            .unwrap();
    }

    let ingestion_ms = start.elapsed().as_secs_f64() * 1000.0;

    let start = Instant::now();
    let mut observed = 0usize;

    for index in 0..token_count {
        let report = memory
            .analyze_retention_as_of(&format!("token-{index}"), i64::MAX, 100)
            .unwrap();

        observed += report.attempts;
        black_box(report);
    }

    assert_eq!(observed, TOTAL_EVENTS);

    let analysis_ms = start.elapsed().as_secs_f64() * 1000.0;

    println!(
        "{token_count:>7} | {TOTAL_EVENTS:>9} | \
         {ingestion_ms:>12.3} | {analysis_ms:>12.3}"
    );
}

fn main() {
    println!("TME Multi-Token Retention Scaling");
    println!("Release build / 100,000 events per scenario");
    println!();
    println!(" Tokens |    Events | Ingestion ms | Analysis ms");
    println!("--------+-----------+--------------+-------------");

    for count in [1, 100, 1_000] {
        run(count);
    }
}
