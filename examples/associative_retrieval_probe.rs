use std::collections::{BTreeSet, VecDeque};
use transformer_memory_engine::{Memory, StateToken};

#[derive(Debug, PartialEq, Eq)]
enum Recall {
    Found { id: String, hops: usize },
    Ambiguous { ids: Vec<String>, hops: usize },
    NotFound,
}

fn add(memory: &mut Memory, id: &str) -> Result<(), String> {
    memory.insert(StateToken {
        id: id.into(),
        vector: vec![1.0, 0.0],
    })
}

fn fixture() -> Result<Memory, String> {
    let mut memory = Memory::new();

    for id in [
        "customer_a",
        "customer_b",
        "customer_c",
        "customer_d",
        "context_a",
        "context_b",
        "record_a",
        "record_b",
        "record_c",
        "record_d",
    ] {
        add(&mut memory, id)?;
    }

    // Direct, unique association.
    memory.reinforce("customer_a", "record_a", 1.0)?;

    // Two equally reachable candidates.
    memory.reinforce("customer_b", "record_a", 1.0)?;
    memory.reinforce("customer_b", "record_b", 1.0)?;

    // customer_c intentionally has no association.

    // Multi-hop association.
    memory.reinforce("customer_d", "context_a", 1.0)?;
    memory.reinforce("context_a", "context_b", 1.0)?;
    memory.reinforce("context_b", "record_d", 1.0)?;

    // Unrelated record remains stored.
    let _ = memory.dynamics("record_c");

    Ok(memory)
}

// Experimental graph traversal, not production retrieval.
// All positive links are treated as trusted associations.
// Shortest-path depth is not a confidence score.
fn recall(memory: &Memory, anchor: &str, max_hops: usize) -> Result<Recall, String> {
    if !memory.states.contains_key(anchor) {
        return Err("Unknown anchor".into());
    }

    let mut adjacency = std::collections::BTreeMap::<String, BTreeSet<String>>::new();

    for relation in memory.relations.values() {
        if relation.weight > 0.0 {
            if !memory.states.contains_key(&relation.from)
                || !memory.states.contains_key(&relation.to)
            {
                return Err("Broken relation reference".into());
            }
            adjacency
                .entry(relation.from.clone())
                .or_default()
                .insert(relation.to.clone());
        }
    }

    let mut queue = VecDeque::from([(anchor.to_owned(), 0_usize)]);
    let mut visited = BTreeSet::from([anchor.to_owned()]);
    let mut found_depth = None;
    let mut candidates = BTreeSet::new();

    while let Some((id, depth)) = queue.pop_front() {
        if found_depth.is_some_and(|d| depth > d) {
            break;
        }

        if depth > 0 && id.starts_with("record_") {
            found_depth = Some(depth);
            candidates.insert(id);
            continue;
        }

        if depth >= max_hops {
            continue;
        }

        if let Some(neighbors) = adjacency.get(&id) {
            for next in neighbors {
                if visited.insert(next.clone()) {
                    queue.push_back((next.clone(), depth + 1));
                }
            }
        }
    }

    let Some(hops) = found_depth else {
        return Ok(Recall::NotFound);
    };

    let ids: Vec<_> = candidates.into_iter().collect();

    if ids.len() == 1 {
        Ok(Recall::Found {
            id: ids[0].clone(),
            hops,
        })
    } else {
        Ok(Recall::Ambiguous { ids, hops })
    }
}

fn main() -> Result<(), String> {
    let memory = fixture()?;

    println!("TME Associative Retrieval Probe v0.4");
    println!("Experimental traversal of existing relations.\n");

    for anchor in ["customer_a", "customer_b", "customer_c", "customer_d"] {
        println!("{anchor}: {:?}", recall(&memory, anchor, 3)?);
    }

    println!("\nNo semantic identity verification performed.");
    println!("No new memory weights or production mutations.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_association() {
        assert_eq!(
            recall(&fixture().unwrap(), "customer_a", 3).unwrap(),
            Recall::Found {
                id: "record_a".into(),
                hops: 1,
            }
        );
    }

    #[test]
    fn ambiguity_is_not_silently_resolved() {
        assert_eq!(
            recall(&fixture().unwrap(), "customer_b", 3).unwrap(),
            Recall::Ambiguous {
                ids: vec!["record_a".into(), "record_b".into()],
                hops: 1,
            }
        );
    }

    #[test]
    fn missing_relation_does_not_invent_result() {
        assert_eq!(
            recall(&fixture().unwrap(), "customer_c", 3).unwrap(),
            Recall::NotFound
        );
    }

    #[test]
    fn multi_hop_recall_obeys_depth_limit() {
        let memory = fixture().unwrap();

        assert_eq!(recall(&memory, "customer_d", 2).unwrap(), Recall::NotFound);

        assert_eq!(
            recall(&memory, "customer_d", 3).unwrap(),
            Recall::Found {
                id: "record_d".into(),
                hops: 3,
            }
        );
    }

    #[test]
    fn deterministic_and_read_only() {
        let memory = fixture().unwrap();
        let before = serde_json::to_value(&memory).unwrap();

        for anchor in ["customer_a", "customer_b", "customer_c", "customer_d"] {
            assert_eq!(
                recall(&memory, anchor, 3).unwrap(),
                recall(&memory, anchor, 3).unwrap()
            );
        }

        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn unknown_anchor_is_rejected() {
        assert!(recall(&fixture().unwrap(), "missing", 3).is_err());
    }
}
