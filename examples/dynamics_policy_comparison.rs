use transformer_memory_engine::{Memory, StateToken};

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    expected: &'static str,
}

#[derive(Debug, PartialEq)]
struct ResultRow {
    case: String,
    alpha: f32,
    winner: String,
    correct: bool,
}

fn fixture() -> Result<Memory, String> {
    let mut memory = Memory::new();

    for (id, vector, activation) in [
        ("semantic", vec![1.0, 0.0], 0.05),
        ("active", vec![0.6, 0.8], 1.0),
    ] {
        memory.insert(StateToken {
            id: id.into(),
            vector,
        })?;
        memory.set_token_dynamics(id, activation, 0.0)?;
    }

    Ok(memory)
}

fn compare() -> Result<Vec<ResultRow>, String> {
    let memory = fixture()?;
    let query = [1.0, 0.0];

    let cases = [
        Case {
            name: "semantic_is_correct",
            expected: "semantic",
        },
        Case {
            name: "active_is_correct",
            expected: "active",
        },
    ];

    let preview = memory.retrieval_priority_preview(&query, 2)?;
    let mut results = Vec::new();

    for case in cases {
        for alpha in [0.0_f32, 0.2, 0.4, 0.6] {
            let mut ranked: Vec<_> = preview
                .candidates
                .iter()
                .map(|c| {
                    let score = (1.0 - alpha) * c.similarity + alpha * c.activation;
                    (c.token_id.as_str(), score)
                })
                .collect();

            ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(b.0)));

            let winner = ranked.first().ok_or("No retrieval candidates")?.0;

            results.push(ResultRow {
                case: case.name.into(),
                alpha,
                winner: winner.into(),
                correct: winner == case.expected,
            });
        }
    }

    Ok(results)
}

fn main() -> Result<(), String> {
    println!("TME Experimental Policy Comparison v0.2");
    println!("Same candidates; different external ground truth.");
    println!("No production ranking changes.\n");

    for result in compare()? {
        println!(
            "{:<22} alpha={:.1} winner={:<9} correct={}",
            result.case, result.alpha, result.winner, result.correct
        );
    }

    println!("\nConstructed scenarios, not population statistics.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_is_deterministic() {
        assert_eq!(compare().unwrap(), compare().unwrap());
    }

    #[test]
    fn semantic_baseline_and_activation_tradeoff() {
        let rows = compare().unwrap();

        assert_eq!(rows.len(), 8);

        let semantic_baseline = &rows[0];
        let semantic_coupled = &rows[2];
        let active_baseline = &rows[4];
        let active_coupled = &rows[6];

        assert!(semantic_baseline.correct);
        assert!(!semantic_coupled.correct);
        assert!(!active_baseline.correct);
        assert!(active_coupled.correct);
    }

    #[test]
    fn simulation_does_not_mutate_core() {
        let memory = fixture().unwrap();
        let before = serde_json::to_value(&memory).unwrap();

        let _ = compare().unwrap();

        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }
}
