use std::time::Instant;
use transformer_memory_engine::{Memory, StateToken};

const SIZE: usize = 1_000;
const TARGET: &str = "z_target";

fn build(bridge_weight: f32) -> Memory {
    let mut memory = Memory::new();

    for (id, vector) in [
        ("task", vec![0.0, 1.0]),
        ("z_bridge", vec![0.0, -1.0]),
        (TARGET, vec![1.0, 0.0]),
    ] {
        memory
            .insert(StateToken {
                id: id.into(),
                vector,
            })
            .unwrap();
    }

    memory.reinforce("task", "z_bridge", bridge_weight).unwrap();

    memory.reinforce("z_bridge", TARGET, 0.95).unwrap();

    for i in 0..(SIZE - 3) {
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

fn print_result(method: &str, budget: usize, found: bool, examined: usize, milliseconds: f64) {
    println!(
        "{method:<12} budget={budget:<4} \
         found={found:<5} examined={examined:<6} \
         ms={milliseconds:.3}"
    );
}

fn run(bridge_weight: f32) {
    let memory = build(bridge_weight);

    println!(
        "\n=== {} TOKENS / BRIDGE WEIGHT {:.2} ===",
        memory.states.len(),
        bridge_weight
    );

    for budget in [10, 50, 100] {
        let start = Instant::now();

        let graph = memory
            .learning_b_activation_preview("task", 2, budget)
            .unwrap();

        let graph_time = start.elapsed().as_secs_f64() * 1000.0;

        let graph_found = graph.states.iter().any(|s| s.token_id == TARGET);

        print_result("Graph", budget, graph_found, graph.states.len(), graph_time);

        let start = Instant::now();

        let weighted = memory
            .learning_b_weighted_preview("task", 2, budget)
            .unwrap();

        let weighted_time = start.elapsed().as_secs_f64() * 1000.0;

        let weighted_found = weighted.states.iter().any(|s| s.token_id == TARGET);

        print_result(
            "Weighted",
            budget,
            weighted_found,
            weighted.states.len(),
            weighted_time,
        );

        let start = Instant::now();

        let bridge = memory.learning_b_bridge_preview("task", 2, budget).unwrap();

        let bridge_time = start.elapsed().as_secs_f64() * 1000.0;

        let bridge_found = bridge.expanded.iter().any(|s| s == TARGET);

        print_result(
            "BestFirst",
            budget,
            bridge_found,
            bridge.expanded.len(),
            bridge_time,
        );

        // Strong bridge is explored early; weak bridge may
        // remain unexpanded under the current budget.
        assert_eq!(bridge_found, bridge_weight > 0.20);

        println!(
            "  BestFirst discovery: {} tokens, {} edges examined",
            bridge.discovered, bridge.edges_examined
        );

        // This is a ranking limit, NOT an activation budget.
        // The current Semantic method first explores the graph.
        let start = Instant::now();

        let semantic = memory
            .learning_b_semantic_preview("task", &[1.0, 0.0], 2, budget)
            .unwrap();

        let semantic_time = start.elapsed().as_secs_f64() * 1000.0;

        let semantic_found = semantic.candidates.iter().any(|s| s.token_id == TARGET);

        print_result(
            "SemanticTop",
            budget,
            semantic_found,
            semantic.candidates_considered,
            semantic_time,
        );

        assert!(semantic_found);
        assert!(!graph_found);

        // Current implementation uses breadth-first levels.
        // With many direct neighbors, the second-hop target
        // remains unreachable under this small activation budget.
        assert!(!weighted_found);
    }
}

fn main() {
    run(0.95);
    run(0.05);

    println!("\nAll synthetic budget assertions passed.");
}
