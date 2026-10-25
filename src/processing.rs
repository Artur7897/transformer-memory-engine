use crate::Memory;

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingCandidate {
    pub token_id: String,
    pub similarity: f32,
    pub activation: f32,
    pub evidence_records: usize,
    pub repeated_reference_groups: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetrievalDecision {
    Found(ProcessingCandidate),
    Ambiguous(Vec<ProcessingCandidate>),
    NotFound,
}

impl Memory {
    /// Read-only coordination of semantic candidates, a resolved
    /// context anchor, existing relations, and provenance metadata.
    ///
    /// Positive relations are treated as associations, not proof.
    /// Evidence counts are descriptive, not confidence scores.
    pub fn process_retrieval(
        &self,
        query: &[f32],
        anchor: &str,
        cutoff_unix_ms: i64,
    ) -> Result<RetrievalDecision, String> {
        if !self.states.contains_key(anchor) {
            return Err("Unknown context anchor".into());
        }

        let preview = self.retrieval_priority_preview(query, self.states.len())?;

        let mut candidates = Vec::new();

        for candidate in preview.candidates {
            if candidate.token_id == anchor {
                continue;
            }

            let linked = self.relations.values().any(|relation| {
                relation.from == anchor
                    && relation.to == candidate.token_id
                    && relation.weight > 0.0
            });

            if !linked {
                continue;
            }

            let quality =
                self.analyze_evidence_quality_as_of(anchor, &candidate.token_id, cutoff_unix_ms)?;

            candidates.push(ProcessingCandidate {
                token_id: candidate.token_id,
                similarity: candidate.similarity,
                activation: candidate.activation,
                evidence_records: quality.total_records,
                repeated_reference_groups: quality.repeated_reference_groups,
            });
        }

        match candidates.len() {
            0 => Ok(RetrievalDecision::NotFound),
            1 => Ok(RetrievalDecision::Found(candidates.remove(0))),
            _ => Ok(RetrievalDecision::Ambiguous(candidates)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    fn memory() -> Memory {
        let mut memory = Memory::new();

        for (id, vector) in [
            ("customer", vec![0.0, 1.0]),
            ("record_a", vec![1.0, 0.0]),
            ("record_b", vec![0.6, 0.8]),
        ] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector,
                })
                .unwrap();
        }

        memory
    }

    #[test]
    fn unique_relation_is_found() {
        let mut memory = memory();
        memory.reinforce("customer", "record_b", 1.0).unwrap();

        let result = memory
            .process_retrieval(&[1.0, 0.0], "customer", 100)
            .unwrap();

        match result {
            RetrievalDecision::Found(candidate) => {
                assert_eq!(candidate.token_id, "record_b");
                assert_eq!(candidate.evidence_records, 0);
            }
            _ => panic!("Expected one associated record"),
        }
    }

    #[test]
    fn multiple_relations_remain_ambiguous() {
        let mut memory = memory();
        memory.reinforce("customer", "record_a", 1.0).unwrap();
        memory.reinforce("customer", "record_b", 1.0).unwrap();

        let result = memory
            .process_retrieval(&[1.0, 0.0], "customer", 100)
            .unwrap();

        assert!(matches!(
            result,
            RetrievalDecision::Ambiguous(records) if records.len() == 2
        ));
    }

    #[test]
    fn missing_relation_is_not_guessed() {
        assert_eq!(
            memory()
                .process_retrieval(&[1.0, 0.0], "customer", 100,)
                .unwrap(),
            RetrievalDecision::NotFound
        );
    }

    #[test]
    fn processing_does_not_change_memory() {
        let mut memory = memory();
        memory.reinforce("customer", "record_a", 1.0).unwrap();

        let before = serde_json::to_value(&memory).unwrap();

        let first = memory
            .process_retrieval(&[1.0, 0.0], "customer", 100)
            .unwrap();
        let second = memory
            .process_retrieval(&[1.0, 0.0], "customer", 100)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn invalid_anchor_or_query_is_rejected() {
        let memory = memory();

        assert!(memory
            .process_retrieval(&[1.0, 0.0], "missing", 100,)
            .is_err());

        assert!(memory
            .process_retrieval(&[0.0, 0.0], "customer", 100,)
            .is_err());
    }
}
