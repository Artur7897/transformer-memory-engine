use std::hint::black_box;
use std::time::{Duration, Instant};
use transformer_memory_engine::{Memory, StateToken};

const DIMENSIONS: usize = 16;
const CANDIDATES: usize = 5;
const RUNS: usize = 7;

fn vector(index: usize) -> Vec<f32> {
    (0..DIMENSIONS)
        .map(|dimension| {
            let value = ((index * 17 + dimension * 13) % 101) as f32;
            (value + 1.0) / 101.0
        })
        .collect()
}

fn fixture(total: usize) -> Result<(Memory, Vec<String>), String> {
    let mut memory = Memory::new();

    memory.insert(StateToken {
        id: "anchor".into(),
        vector: vector(0),
    })?;

    let mut eligible = Vec::new();

    for i in 0..total {
        let id = format!("token_{i:06}");

        memory.insert(StateToken {
            id: id.clone(),
            vector: vector(i + 1),
        })?;

        if i < CANDIDATES {
            memory.reinforce("anchor", &id, 1.0)?;
            eligible.push(id);
        }
    }

    Ok((memory, eligible))
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() / 2]
}

fn measure(
    memory: &Memory,
    eligible: &[&str],
    query: &[f32],
) -> Result<(Duration, Duration), String> {
    let expected = memory.process_retrieval(query, "anchor", 100)?;

    let actual = memory.process_retrieval_scoped(query, "anchor", 100, eligible)?;

    if expected != actual {
        return Err("Retrieval results differ".into());
    }

    let mut full_samples = Vec::new();
    let mut scoped_samples = Vec::new();

    // Alternate execution order to reduce simple order bias.
    for run in 0..RUNS {
        if run % 2 == 0 {
            let start = Instant::now();
            let result = memory.process_retrieval(query, "anchor", 100)?;
            black_box(result);
            full_samples.push(start.elapsed());

            let start = Instant::now();
            let result = memory.process_retrieval_scoped(query, "anchor", 100, eligible)?;
            black_box(result);
            scoped_samples.push(start.elapsed());
        } else {
            let start = Instant::now();
            let result = memory.process_retrieval_scoped(query, "anchor", 100, eligible)?;
            black_box(result);
            scoped_samples.push(start.elapsed());

            let start = Instant::now();
            let result = memory.process_retrieval(query, "anchor", 100)?;
            black_box(result);
            full_samples.push(start.elapsed());
        }
    }

    Ok((median(full_samples), median(scoped_samples)))
}

fn main() -> Result<(), String> {
    println!("TME Processing Scaling Benchmark v0.1");
    println!(
        "dimensions={} linked_candidates={} repetitions={}",
        DIMENSIONS, CANDIDATES, RUNS
    );
    println!("All results must match exactly.\n");

    for total in [1_000, 10_000, 30_000] {
        let setup_start = Instant::now();
        let (memory, ids) = fixture(total)?;
        let setup = setup_start.elapsed();

        let eligible: Vec<&str> = ids.iter().map(String::as_str).collect();

        let query = vector(1);
        let (full, scoped) = measure(&memory, &eligible, &query)?;

        let full_us = full.as_secs_f64() * 1_000_000.0;
        let scoped_us = scoped.as_secs_f64() * 1_000_000.0;

        println!(
            "tokens={total:>6} setup={:.3}s full={:.2}us \
             scoped={:.2}us ratio={:.2}x",
            setup.as_secs_f64(),
            full_us,
            scoped_us,
            full_us / scoped_us
        );
    }

    println!("\nDescriptive single-process benchmark.");
    println!("No memory-size or allocation profiling.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_has_expected_candidate_count() {
        let (memory, ids) = fixture(20).unwrap();
        assert_eq!(memory.states.len(), 21);
        assert_eq!(memory.relations.len(), CANDIDATES);
        assert_eq!(ids.len(), CANDIDATES);
    }

    #[test]
    fn scoped_and_full_results_are_identical() {
        let (memory, ids) = fixture(100).unwrap();
        let eligible: Vec<&str> = ids.iter().map(String::as_str).collect();

        let query = vector(1);

        assert_eq!(
            memory.process_retrieval(&query, "anchor", 100).unwrap(),
            memory
                .process_retrieval_scoped(&query, "anchor", 100, &eligible)
                .unwrap()
        );
    }
}
