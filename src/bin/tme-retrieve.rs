use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use transformer_memory_engine::{Memory, StateToken};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    states: Vec<StateToken>,
    query: Vec<f32>,
    limit: usize,
}

#[derive(Serialize)]
struct Response {
    engine: &'static str,
    state_count: usize,
    results: Vec<Match>,
}

#[derive(Serialize)]
struct Match {
    id: String,
    similarity: f32,
}

fn run() -> Result<(), String> {
    let mut input = String::new();

    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;

    let request: Request = serde_json::from_str(&input).map_err(|e| e.to_string())?;

    if request.query.is_empty() || !request.query.iter().all(|x| x.is_finite()) {
        return Err("Query must contain finite numeric values".into());
    }

    if request.limit == 0 || request.limit > 100 {
        return Err("Limit must be between 1 and 100".into());
    }

    let dimensions = request.query.len();
    let mut memory = Memory::new();

    for state in request.states {
        if state.vector.len() != dimensions {
            return Err(format!("Dimension mismatch for state {}", state.id));
        }

        memory.insert(state)?;
    }

    let results = memory
        .attention(&request.query, request.limit)
        .into_iter()
        .map(|(id, similarity)| Match { id, similarity })
        .collect();

    let response = Response {
        engine: "transformer-memory-engine",
        state_count: memory.states.len(),
        results,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&response).map_err(|e| e.to_string())?
    );

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("TME error: {error}");
        std::process::exit(1);
    }
}
