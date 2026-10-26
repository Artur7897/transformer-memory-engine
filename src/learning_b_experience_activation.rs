use crate::Memory;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperiencedNeighbor {
    pub state_id: String,
    pub observation_count: usize,
    pub task_count: usize,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperienceActivation {
    pub anchor: String,
    pub neighbors: Vec<ExperiencedNeighbor>,
}

impl Memory {
    /// Find states that immediately followed the anchor in
    /// recorded execution observations.
    ///
    /// This is not a confidence ranking, causal inference,
    /// or validated skill recommendation.
    pub fn learning_b_experience_activation(
        &self,
        anchor: &str,
        cutoff_unix_ms: i64,
    ) -> Result<ExperienceActivation, String> {
        if !self.states.contains_key(anchor) {
            return Err("Unknown activation anchor".into());
        }

        let mut grouped: BTreeMap<String, (Vec<String>, BTreeSet<String>)> = BTreeMap::new();

        for record in self.contextual_evidence.values() {
            if !record.is_execution_adjacency()
                || record.from != anchor
                || record.available_at_unix_ms > cutoff_unix_ms
            {
                continue;
            }

            if !self.states.contains_key(&record.to) {
                return Err("Observation references an unknown state".into());
            }

            let task_id = record
                .context
                .get("task_id")
                .ok_or("Execution observation missing task ID")?;

            if task_id.trim().is_empty() {
                return Err("Execution observation has empty task ID".into());
            }

            let (ids, tasks) = grouped.entry(record.to.clone()).or_default();

            ids.push(record.id.clone());
            tasks.insert(task_id.clone());
        }

        let mut neighbors: Vec<ExperiencedNeighbor> = grouped
            .into_iter()
            .map(|(state_id, (mut evidence_ids, tasks))| {
                evidence_ids.sort();

                ExperiencedNeighbor {
                    state_id,
                    observation_count: evidence_ids.len(),
                    task_count: tasks.len(),
                    evidence_ids,
                }
            })
            .collect();

        neighbors.sort_by(|a, b| {
            b.task_count
                .cmp(&a.task_count)
                .then_with(|| b.observation_count.cmp(&a.observation_count))
                .then_with(|| a.state_id.cmp(&b.state_id))
        });

        Ok(ExperienceActivation {
            anchor: anchor.into(),
            neighbors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        contextual_evidence::{ContextualEvidenceRecord, CONTEXTUAL_EVIDENCE_SCHEMA_VERSION},
        experience::EvidenceSource,
        StateToken,
    };

    fn fixture() -> Memory {
        let mut memory = Memory::new();

        for id in ["api.call", "response.verify", "documentation.read"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        for (id, task, to, available) in [
            ("e1", "task-1", "response.verify", 100),
            ("e2", "task-2", "response.verify", 200),
            ("e3", "task-3", "documentation.read", 300),
        ] {
            memory
                .record_contextual_evidence(ContextualEvidenceRecord {
                    schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                    id: id.into(),
                    from: "api.call".into(),
                    to: to.into(),
                    occurred: true,
                    observed_at_unix_ms: available,
                    available_at_unix_ms: available,
                    source: EvidenceSource::VerifiedToolResult,
                    reference: format!("execution/{task}/{id}"),
                    context: BTreeMap::from([
                        ("observation_kind".into(), "execution_adjacency".into()),
                        ("task_id".into(), task.into()),
                    ]),
                })
                .unwrap();
        }

        memory
    }

    #[test]
    fn activates_shared_experience_across_tasks() {
        let memory = fixture();

        let result = memory
            .learning_b_experience_activation("api.call", 300)
            .unwrap();

        assert_eq!(result.neighbors.len(), 2);
        assert_eq!(result.neighbors[0].state_id, "response.verify");
        assert_eq!(result.neighbors[0].observation_count, 2);
        assert_eq!(result.neighbors[0].task_count, 2);
        assert_eq!(result.neighbors[1].state_id, "documentation.read");
    }

    #[test]
    fn respects_availability_cutoff() {
        let memory = fixture();

        let result = memory
            .learning_b_experience_activation("api.call", 150)
            .unwrap();

        assert_eq!(result.neighbors.len(), 1);
        assert_eq!(result.neighbors[0].observation_count, 1);
    }

    #[test]
    fn remains_read_only() {
        let memory = fixture();
        let before = serde_json::to_string(&memory).unwrap();

        memory
            .learning_b_experience_activation("api.call", 300)
            .unwrap();

        assert_eq!(before, serde_json::to_string(&memory).unwrap());
    }

    #[test]
    fn rejects_unknown_anchor() {
        let memory = fixture();

        assert!(memory
            .learning_b_experience_activation("missing", 300)
            .is_err());
    }
}
