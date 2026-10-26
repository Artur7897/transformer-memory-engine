use std::time::Instant;
use transformer_memory_engine::{Memory, StateToken};

fn build_graph(size: usize) -> Memory {
    assert!(size >= 3);

    let mut memory = Memory::new();

    for (id, vector) in [
        ("task", vec![0.0, 1.0]),
        ("z_bridge", vec![0.0, -1.0]),
        ("z_target", vec![1.0, 0.0]),
    ] {
        memory
            .insert(StateToken {
                id: id.to_string(),
                vector,
            })
            .unwrap();
    }

    memory.reinforce("task", "z_bridge", 0.95).unwrap();
    memory.reinforce("z_bridge", "z_target", 0.95).unwrap();

    // Distractors: close in graph, semantically irrelevant.
    for i in 0..(size - 3) {
        let id = format!("a_noise_{i:05}");

        memory
            .insert(StateToken {
                id: id.clone(),
                vector: vec![-1.0, 0.0],
            })
            .unwrap();

        memory.reinforce("task", &id, 0.20).unwrap();
    }

    memory
}

fn position(ids: &[String], target: &str) -> Option<usize> {
    ids.iter().position(|id| id == target).map(|i| i + 1)
}

fn report(name: &str, ids: &[String], visited: usize, elapsed: f64) {
    let rank = position(ids, "z_target");
    let top10 = rank.is_some_and(|n| n <= 10);

    println!(
        "{name:<11} rank={:<7} top10={:<5} candidates={:<7} ms={:.3}",
        rank.map(|n| n.to_string())
            .unwrap_or_else(|| "missing".into()),
        top10,
        visited,
        elapsed,
    );
}

fn run(size: usize) {
    let memory = build_graph(size);

    println!("\n=== CHIMERA / {size} TOKENS ===");

    let start = Instant::now();

    let graph = memory
        .learning_b_activation_preview("task", 2, size)
        .unwrap();

    let graph_ms = start.elapsed().as_secs_f64() * 1000.0;

    let graph_ids: Vec<String> = graph.states.iter().map(|s| s.token_id.clone()).collect();

    report("Graph", &graph_ids, graph.states.len(), graph_ms);

    let start = Instant::now();

    let weighted = memory.learning_b_weighted_preview("task", 2, size).unwrap();

    let weighted_ms = start.elapsed().as_secs_f64() * 1000.0;

    let weighted_ids: Vec<String> = weighted.states.iter().map(|s| s.token_id.clone()).collect();

    report(
        "Weighted",
        &weighted_ids,
        weighted.states.len(),
        weighted_ms,
    );

    let start = Instant::now();

    let semantic = memory
        .learning_b_semantic_preview("task", &[1.0, 0.0], 2, size)
        .unwrap();

    let semantic_ms = start.elapsed().as_secs_f64() * 1000.0;

    let semantic_ids: Vec<String> = semantic
        .candidates
        .iter()
        .map(|s| s.token_id.clone())
        .collect();

    report(
        "Semantic",
        &semantic_ids,
        semantic.candidates_considered,
        semantic_ms,
    );

    assert_eq!(memory.states.len(), size);

    // These are the expected outcomes of this synthetic fixture,
    // not general assertions about retrieval quality.
    assert_eq!(position(&semantic_ids, "z_target"), Some(1));
    assert_eq!(position(&weighted_ids, "z_target"), Some(size));
    assert_eq!(position(&graph_ids, "z_target"), Some(size));

    println!("Synthetic fixture assertions: PASS");
}

fn main() {
    for size in [100, 1_000, 10_000] {
        run(size);
    }
}
