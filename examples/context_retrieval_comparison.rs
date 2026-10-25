use transformer_memory_engine::{Memory, StateToken};

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    anchor: &'static str,
    expected: &'static str,
}

#[derive(Debug, PartialEq)]
struct ResultRow {
    case: String,
    expected: String,
    semantic: String,
    activated: String,
    contextual: String,
}

fn fixture() -> Result<Memory, String> {
    let mut memory = Memory::new();

    for (id, vector, activation) in [
        ("client_a", vec![0.0, 1.0], 0.5),
        ("client_b", vec![0.0, 1.0], 0.5),
        ("record_a", vec![1.0, 0.0], 0.05),
        ("record_b", vec![0.6, 0.8], 1.0),
    ] {
        memory.insert(StateToken {
            id: id.into(),
            vector,
        })?;
        memory.set_token_dynamics(id, activation, 0.0)?;
    }

    memory.reinforce("client_a", "record_a", 1.0)?;
    memory.reinforce("client_b", "record_b", 1.0)?;

    Ok(memory)
}

fn relation_weight(memory: &Memory, from: &str, to: &str) -> Result<f32, String> {
    let key = serde_json::to_string(&(from, to)).map_err(|e| e.to_string())?;
    Ok(memory.relations.get(&key).map_or(0.0, |r| r.weight))
}

fn evaluate() -> Result<Vec<ResultRow>, String> {
    let memory = fixture()?;
    let query = [1.0, 0.0];

    let cases = [
        Case {
            name: "customer_a",
            anchor: "client_a",
            expected: "record_a",
        },
        Case {
            name: "customer_b",
            anchor: "client_b",
            expected: "record_b",
        },
    ];

    let candidates = memory.retrieval_priority_preview(&query, 4)?.candidates;

    let records: Vec<_> = candidates
        .iter()
        .filter(|c| c.token_id.starts_with("record_"))
        .collect();

    if records.len() != 2 {
        return Err("Expected two record candidates".into());
    }

    let mut result = Vec::new();

    for case in cases {
        let semantic = records
            .iter()
            .max_by(|a, b| {
                a.similarity
                    .total_cmp(&b.similarity)
                    .then_with(|| b.token_id.cmp(&a.token_id))
            })
            .ok_or("No semantic candidate")?
            .token_id
            .clone();

        let activated = records
            .iter()
            .max_by(|a, b| {
                let sa = 0.6 * a.similarity + 0.4 * a.activation;
                let sb = 0.6 * b.similarity + 0.4 * b.activation;
                sa.total_cmp(&sb).then_with(|| b.token_id.cmp(&a.token_id))
            })
            .ok_or("No activation candidate")?
            .token_id
            .clone();

        // Experimental policy: only records explicitly linked
        // from the trusted, already resolved context anchor
        // are eligible. No new stored weight is introduced.
        let mut linked = Vec::new();

        for candidate in &records {
            if relation_weight(&memory, case.anchor, &candidate.token_id)? > 0.0 {
                linked.push(*candidate);
            }
        }

        let contextual = linked
            .iter()
            .max_by(|a, b| {
                a.similarity
                    .total_cmp(&b.similarity)
                    .then_with(|| b.token_id.cmp(&a.token_id))
            })
            .ok_or("No matching context relation")?
            .token_id
            .clone();

        result.push(ResultRow {
            case: case.name.into(),
            expected: case.expected.into(),
            semantic,
            activated,
            contextual,
        });
    }

    Ok(result)
}

fn main() -> Result<(), String> {
    println!("TME Context Retrieval Comparison v0.3");
    println!("Experimental context-filter policy.");
    println!("No production retrieval changes.\n");

    for row in evaluate()? {
        println!(
            "{} expected={} semantic={} activated={} contextual={}",
            row.case, row.expected, row.semantic, row.activated, row.contextual
        );
    }

    println!("\nContext anchors and links are trusted fixture inputs.");
    println!("Identity resolution and link correctness are not tested.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(evaluate().unwrap(), evaluate().unwrap());
    }

    #[test]
    fn context_distinguishes_cases() {
        let rows = evaluate().unwrap();
        assert_eq!(rows.len(), 2);

        assert_eq!(rows[0].semantic, "record_a");
        assert_eq!(rows[1].semantic, "record_a");

        assert_eq!(rows[0].activated, "record_b");
        assert_eq!(rows[1].activated, "record_b");

        for row in rows {
            assert_eq!(row.contextual, row.expected);
        }
    }

    #[test]
    fn missing_context_link_is_not_invented() {
        let memory = fixture().unwrap();
        assert_eq!(
            relation_weight(&memory, "client_a", "record_b").unwrap(),
            0.0
        );
    }
}
