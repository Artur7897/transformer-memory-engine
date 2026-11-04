//! Text ingestion: structured semantic segments supplied by an external LLM.
//! State/event ingestion remains unchanged. This module does not call an LLM.
//! The first version preserves text, order and provenance; semantic equivalence
//! detection is deliberately NOT claimed (exact normalized re-ingest only).
use crate::{token_architecture::TokenKind, Memory, StateToken};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextMemoryEntry {
    pub text: String,
    pub source: String,
    pub anchor_id: String,
    pub position: usize,
}

#[derive(Clone, Debug)]
pub struct TextSegment {
    pub id: String,
    pub text: String,
    pub vector: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct TextDocument {
    pub anchor_id: String,
    pub title: String,
    pub anchor_vector: Vec<f32>,
    pub source: String,
    pub segments: Vec<TextSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextIngestionReport {
    pub inserted: Vec<String>,
    pub reused: Vec<String>,
}

fn normalized(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn valid_vector(vector: &[f32]) -> bool {
    !vector.is_empty() && vector.iter().all(|v| v.is_finite()) && vector.iter().any(|v| *v != 0.0)
}

impl Memory {
    /// Atomic in-memory commit (clone/validate/swap). Caller persists with
    /// state_container::update or creates a fresh test state via create.
    /// Input text segments are proposals from a semantic resolver, not proof
    /// of facts and not automatically validated Skills.
    pub fn ingest_text(&mut self, document: &TextDocument) -> Result<TextIngestionReport, String> {
        if document.anchor_id.trim().is_empty()
            || normalized(&document.title).is_empty()
            || normalized(&document.source).is_empty()
            || !valid_vector(&document.anchor_vector)
            || document.segments.is_empty()
            || document.segments.len() > 128
        {
            return Err("Invalid text document".into());
        }
        let dimension = document.anchor_vector.len();
        let mut ids = BTreeSet::new();
        ids.insert(document.anchor_id.as_str());
        for segment in &document.segments {
            if segment.id.trim().is_empty()
                || !ids.insert(segment.id.as_str())
                || normalized(&segment.text).is_empty()
                || !valid_vector(&segment.vector)
                || segment.vector.len() != dimension
            {
                return Err("Invalid text segment".into());
            }
        }
        let mut staged = self.clone();
        let mut inserted = vec![];
        let mut reused = vec![];
        let all = std::iter::once((
            document.anchor_id.as_str(),
            document.title.as_str(),
            document.anchor_vector.as_slice(),
            0usize,
        ))
        .chain(document.segments.iter().enumerate().map(|(i, item)| {
            (
                item.id.as_str(),
                item.text.as_str(),
                item.vector.as_slice(),
                i + 1,
            )
        }));
        for (id, text, vector, position) in all {
            if staged.states.contains_key(id) {
                let old = staged
                    .text_memories
                    .get(id)
                    .ok_or_else(|| format!("Existing non-text token ID collision: {id}"))?;
                if normalized(&old.text) != normalized(text)
                    || old.source != document.source
                    || old.anchor_id != document.anchor_id
                    || old.position != position
                    || staged.token_kind(id) != Some(TokenKind::Memory)
                    || staged.states[id].vector.as_slice() != vector
                {
                    return Err(format!("Incompatible existing text token: {id}"));
                }
                reused.push(id.to_string());
            } else {
                staged.insert(StateToken {
                    id: id.into(),
                    vector: vector.to_vec(),
                })?;
                staged.set_token_kind(id, TokenKind::Memory)?;
                staged.text_memories.insert(
                    id.into(),
                    TextMemoryEntry {
                        text: normalized(text),
                        source: document.source.clone(),
                        anchor_id: document.anchor_id.clone(),
                        position,
                    },
                );
                inserted.push(id.to_string());
            }
        }
        staged.set_anchor(&document.anchor_id, true)?;
        // Directed hierarchy/order. Do not repeatedly reinforce on re-ingestion.
        for (i, segment) in document.segments.iter().enumerate() {
            if !self.states.contains_key(&segment.id) {
                staged.reinforce(&document.anchor_id, &segment.id, 0.5)?;
            }
            if i > 0 && (!self.states.contains_key(&segment.id)) {
                staged.reinforce(&document.segments[i - 1].id, &segment.id, 0.5)?;
            }
        }
        staged.validate_state_metadata()?;
        *self = staged;
        Ok(TextIngestionReport { inserted, reused })
    }

    /// Text content in the original segment order, without LLM regeneration.
    pub fn recall_text_segments(&self, anchor_id: &str) -> Vec<&TextMemoryEntry> {
        let mut entries: Vec<_> = self
            .text_memories
            .values()
            .filter(|item| item.anchor_id == anchor_id && item.position > 0)
            .collect();
        entries.sort_by_key(|item| item.position);
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> TextDocument {
        TextDocument {
            anchor_id: "story".into(),
            title: "Die Reise".into(),
            anchor_vector: vec![1.0, 0.0],
            source: "test/story.md".into(),
            segments: vec![
                TextSegment {
                    id: "scene-1".into(),
                    text: "Ein Reisender verlässt das Dorf.".into(),
                    vector: vec![0.9, 0.1],
                },
                TextSegment {
                    id: "scene-2".into(),
                    text: "Er begegnet einem Fremden.".into(),
                    vector: vec![0.8, 0.2],
                },
            ],
        }
    }
    #[test]
    fn creates_memory_only_and_preserves_order() {
        let mut m = Memory::new();
        let r = m.ingest_text(&document()).unwrap();
        assert_eq!(r.inserted.len(), 3);
        assert_eq!(m.token_kind("scene-1"), Some(TokenKind::Memory));
        assert_eq!(m.is_anchor("story"), Some(true));
        assert_eq!(
            m.recall_text_segments("story")[1].text,
            "Er begegnet einem Fremden."
        );
    }
    #[test]
    fn repeated_identical_ingestion_reuses_tokens() {
        let mut m = Memory::new();
        m.ingest_text(&document()).unwrap();
        let before = serde_json::to_value(&m).unwrap();
        let r = m.ingest_text(&document()).unwrap();
        assert_eq!(r.reused.len(), 3);
        assert_eq!(before, serde_json::to_value(&m).unwrap());
    }
    #[test]
    fn failed_batch_does_not_mutate_memory() {
        let mut m = Memory::new();
        m.ingest_text(&document()).unwrap();
        let before = serde_json::to_value(&m).unwrap();
        let mut changed = document();
        changed.segments[1].text = "Different content".into();
        assert!(m.ingest_text(&changed).is_err());
        assert_eq!(before, serde_json::to_value(&m).unwrap());
    }
    #[test]
    fn text_survives_tme_container_reload() {
        let dir = tempfile::tempdir().unwrap();
        let id = "12345678-1234-1234-1234-123456789abc";
        let path = dir.path().join(format!("{id}.tme"));
        let mut m = Memory::new();
        m.ingest_text(&document()).unwrap();
        crate::state_container::create(&path, id, "TEST", &m).unwrap();
        let restored = crate::state_container::load(&path, id, "TEST").unwrap();
        assert_eq!(restored.recall_text_segments("story").len(), 2);
        assert_eq!(restored.is_anchor("story"), Some(true));
    }
    #[test]
    fn text_roundtrips_with_backward_compatible_default() {
        let mut m = Memory::new();
        m.ingest_text(&document()).unwrap();
        let restored: Memory = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(restored.recall_text_segments("story").len(), 2);
        let mut old = serde_json::to_value(&m).unwrap();
        old.as_object_mut().unwrap().remove("text_memories");
        let restored_old: Memory = serde_json::from_value(old).unwrap();
        assert!(restored_old.text_memories.is_empty());
    }
}
