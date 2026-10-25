use transformer_memory_engine::{Memory, StateToken};

fn main() -> Result<(), String> {
    let mut memory = Memory::new();

    // A: best semantic match, rarely activated.
    // B: weaker semantic match, highly activated.
    for (id, vector, activation) in [
        ("relevant", vec![1.0, 0.0], 0.05),
        ("frequent", vec![0.6, 0.8], 1.0),
    ] {
        memory.insert(StateToken {
            id: id.into(),
            vector,
        })?;
        memory.set_token_dynamics(id, activation, 0.0)?;
    }

    let query = [1.0, 0.0];
    let baseline = memory.retrieval_priority_preview(&query, 2)?;

    println!("TME Retrieval Activation Probe");
    println!("Ground-truth relevant token: relevant");
    println!("Experimental coefficients; not calibrated.");
    println!();

    for alpha in [0.0_f32, 0.1, 0.2, 0.3, 0.4, 0.5, 1.0] {
        let mut ranked: Vec<_> = baseline
            .candidates
            .iter()
            .map(|candidate| {
                let score = (1.0 - alpha) * candidate.similarity + alpha * candidate.activation;
                (candidate.token_id.as_str(), score)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(b.0)));

        let top = ranked[0];
        println!(
            "alpha={alpha:.1}  top={}  score={:.3}  correct={}",
            top.0,
            top.1,
            top.0 == "relevant"
        );
    }

    println!();
    println!("This is one constructed counterexample,");
    println!("not an estimate of retrieval performance.");
    Ok(())
}
