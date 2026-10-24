use std::fs;
use std::hint::black_box;
use std::time::Instant;

use transformer_memory_engine::{
    retention::{RecallOutcome, RetentionEvent},
    Memory, StateToken,
};

const EVENTS: usize = 100_000;
const TOKENS: usize = 100;

fn event(index: usize, token: usize, time: i64) -> RetentionEvent {
    RetentionEvent {
        id: format!("event-{index}"),
        token_id: format!("token-{token}"),
        attempted_at_unix_ms: time,
        outcome: RecallOutcome::Success,
        reference: format!("benchmark/{index}"),
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut memory = Memory::new();

    for token in 0..TOKENS {
        memory.insert(StateToken {
            id: format!("token-{token}"),
            vector: vec![1.0, 0.0],
        })?;
    }

    for index in 0..EVENTS {
        memory.record_retention_event(event(index, index % TOKENS, (index / TOKENS) as i64))?;
    }

    let path = std::env::temp_dir().join(format!(
        "tme-cold-load-{}-{}.json",
        std::process::id(),
        EVENTS
    ));

    if path.exists() || path.with_extension("tmp").exists() {
        return Err("Temporary benchmark path already exists".into());
    }

    // Cleanup is attempted even if the benchmark fails.
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        memory.save(&path)?;
        let save_ms = start.elapsed().as_secs_f64() * 1000.0;

        let file_bytes = fs::metadata(&path)?.len();

        let start = Instant::now();
        let mut loaded = Memory::load(&path)?;
        let load_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        loaded.validate_retention_history()?;
        let validation_ms = start.elapsed().as_secs_f64() * 1000.0;

        // The first insertion reconstructs the derived ID index.
        let start = Instant::now();
        loaded.record_retention_event(event(EVENTS, 0, (EVENTS / TOKENS) as i64))?;
        let first_write_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        loaded.record_retention_event(event(EVENTS + 1, 0, (EVENTS / TOKENS + 1) as i64))?;
        let next_write_ms = start.elapsed().as_secs_f64() * 1000.0;

        let report = loaded.retention_as_of("token-0", i64::MAX)?;
        assert_eq!(report.attempts, EVENTS / TOKENS + 2);
        black_box(report);

        println!("TME Cold Load Benchmark");
        println!("Events: {EVENTS}");
        println!("Tokens: {TOKENS}");
        println!("JSON bytes: {file_bytes}");
        println!("Save: {save_ms:.3} ms");
        println!("Load incl. validation: {load_ms:.3} ms");
        println!("Separate validation: {validation_ms:.3} ms");
        println!("First write after load: {first_write_ms:.3} ms");
        println!("Next write: {next_write_ms:.6} ms");
        println!("Integrity: PASS");
        Ok(())
    })();

    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("tmp"));
    result
}

fn main() {
    if let Err(error) = run() {
        eprintln!("BENCHMARK FAILED: {error}");
        std::process::exit(1);
    }
}
