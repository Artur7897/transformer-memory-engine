//! Explicit LLM-assisted text Abstraction *proposal* contract.
//! The TME verifies lineage/eligibility, not whether a summary is true.
//! No writes occur until a separate, reviewed commit is implemented.
use crate::{Memory, token_architecture::TokenKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextAbstractionRequest {
    pub source_token_ids: Vec<String>,
    pub proposed_text: String,
    pub proposed_source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextAbstractionPreview {
    pub source_token_ids: Vec<String>,
    pub source_records: Vec<(String, String, String)>,
    pub proposed_text: String,
    pub eligible_for_external_semantic_review: bool,
    pub warnings: Vec<String>,
    /// Fingerprint of exact sources and proposed content for review/commit.
    pub review_digest: String,
}

/// Provenance-preserving abstract, with explicit review reference.
/// The original source segments remain immutable and accessible.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextAbstractionRecord {
    pub abstract_token_id: String,
    pub abstract_anchor_id: String,
    pub source_token_ids: Vec<String>,
    pub review_reference: String,
    pub original_sources: Vec<String>,
    /// Empty only for legacy records created before digest-bound review.
    #[serde(default)]
    pub review_digest: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextAbstractionCommit {
    pub anchor_id: String,
    pub token_id: String,
    pub anchor_vector: Vec<f32>,
    pub text_vector: Vec<f32>,
    /// Caller represents that a separate authorized semantic review occurred.
    /// No truth inference is performed by this method.
    pub review_reference: String,
    /// Must equal the digest shown by abstraction-preview to the reviewer.
    pub reviewed_digest: String,
}

fn abstraction_review_digest(
    source_records: &[(String, String, String)],
    proposed_text: &str,
    proposed_source: &str,
) -> Result<String, String> {
    // Length-delimited canonical serialization avoids ambiguous concatenation.
    let serialized = serde_json::to_vec(&(
        "tme-text-abstraction-review-v1",
        source_records,
        proposed_text,
        proposed_source,
    )).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(&serialized)))
}

impl Memory {
    /// Commit a reviewed abstract atomically, preserving all source text,
    /// source relations and provenance. This does NOT mark its assertions true.
    pub fn text_abstraction_commit(
        &mut self,
        request: &TextAbstractionRequest,
        commit: &TextAbstractionCommit,
    ) -> Result<TextAbstractionRecord, String> {
        use crate::text_ingestion::{TextDocument,TextSegment};
        let preview = self.text_abstraction_preview(request)?;
        if commit.review_reference.trim().is_empty()
            || commit.reviewed_digest != preview.review_digest
            || commit.anchor_id.trim().is_empty()
            || commit.token_id.trim().is_empty()
            || commit.anchor_id == commit.token_id
            || self.states.contains_key(&commit.anchor_id)
            || self.states.contains_key(&commit.token_id)
            || commit.text_vector.len() != commit.anchor_vector.len()
            || commit.text_vector.is_empty()
            || !commit.text_vector.iter().all(|v|v.is_finite())
            || !commit.anchor_vector.iter().all(|v|v.is_finite())
        { return Err("Invalid abstraction commit metadata".into()); }
        if self.abstraction_records.contains_key(&commit.token_id) {
            return Err("Abstract token already registered".into());
        }
        let mut staged=self.clone();
        staged.ingest_text(&TextDocument {
            anchor_id:commit.anchor_id.clone(),
            title:format!("Abstract of {} segments",request.source_token_ids.len()),
            anchor_vector:commit.anchor_vector.clone(),
            source:request.proposed_source.clone(),
            segments:vec![TextSegment {
                id:commit.token_id.clone(),
                text:preview.proposed_text,
                vector:commit.text_vector.clone(),
            }],
        })?;
        let sources:Vec<String> = preview.source_records.iter().map(|(_,_,src)|src.clone()).collect();
        for source_id in &request.source_token_ids {
            // Dependency links are directional and don't rewrite the originals.
            staged.reinforce(source_id, &commit.anchor_id, 0.5)?;
        }
        let record=TextAbstractionRecord {
            abstract_token_id:commit.token_id.clone(),
            abstract_anchor_id:commit.anchor_id.clone(),
            source_token_ids:request.source_token_ids.clone(),
            review_reference:commit.review_reference.clone(),
            original_sources:sources,
            review_digest:preview.review_digest,
        };
        staged.abstraction_records.insert(commit.token_id.clone(),record.clone());
        staged.validate_state_metadata()?;
        *self=staged;
        Ok(record)
    }

    /// Prepare an Abstraction task from an LLM's proposed summary. A valid
    /// preview explicitly does NOT authorize semantic merge or deletion.
    pub fn text_abstraction_preview(
        &self, request: &TextAbstractionRequest,
    ) -> Result<TextAbstractionPreview, String> {
        if request.source_token_ids.len() < 2 || request.source_token_ids.len() > 128
            || request.proposed_text.trim().is_empty() || request.proposed_source.trim().is_empty() {
            return Err("Invalid abstraction request".into());
        }
        let mut unique = BTreeSet::new();
        let mut sources = Vec::new();
        for id in &request.source_token_ids {
            if !unique.insert(id) { return Err("Duplicate source token ID".into()); }
            if self.token_kind(id) != Some(TokenKind::Memory) {
                return Err(format!("Abstraction source is not a Memory token: {id}"));
            }
            let entry = self.text_memories.get(id)
                .ok_or_else(|| format!("Abstraction source has no text: {id}"))?;
            if self.is_anchor(id) == Some(true) {
                return Err("Cannot summarize a structural anchor as a text segment".into());
            }
            sources.push((id.clone(), entry.text.clone(), entry.source.clone()));
        }
        let proposed_text = request.proposed_text.trim().to_string();
        let review_digest = abstraction_review_digest(
            &sources, &proposed_text, &request.proposed_source,
        )?;
        Ok(TextAbstractionPreview {
            source_token_ids: request.source_token_ids.clone(),
            source_records: sources,
            proposed_text,
            eligible_for_external_semantic_review: true,
            review_digest,
            warnings: vec![
                "Semantic preservation has not been verified".into(),
                "Original source texts and provenance must be retained".into(),
                "No persistence or source deletion performed".into(),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{text_ingestion::{TextDocument, TextSegment}, StateToken};
    #[test]
    fn validates_source_types_without_mutation() {
        let mut m = Memory::new();
        m.ingest_text(&TextDocument {
            anchor_id: "doc".into(), title:"Story".into(), source:"local".into(), anchor_vector:vec![1.0,0.0],
            segments: vec![
                TextSegment{id:"s1".into(), text:"Eins".into(), vector:vec![1.0,0.0]},
                TextSegment{id:"s2".into(), text:"Zwei".into(), vector:vec![1.0,0.0]},
            ],
        }).unwrap();
        m.insert(StateToken{id:"skill".into(), vector:vec![1.0,0.0]}).unwrap();
        let before = serde_json::to_value(&m).unwrap();
        let proposal = TextAbstractionRequest {
            source_token_ids:vec!["s1".into(), "s2".into()],
            proposed_text:"Kurzfassung".into(), proposed_source:"agent/review".into(),
        };
        assert!(m.text_abstraction_preview(&proposal).unwrap().eligible_for_external_semantic_review);
        let mut invalid = proposal.clone();
        invalid.source_token_ids[0] = "skill".into();
        assert!(m.text_abstraction_preview(&invalid).is_err());
        assert_eq!(before, serde_json::to_value(&m).unwrap());
    }
    #[test]
    fn commit_preserves_sources_and_record_survives_reload() {
        let mut m=Memory::new();
        m.ingest_text(&TextDocument {
            anchor_id:"doc".into(),title:"Story".into(),source:"original".into(),
            anchor_vector:vec![1.,0.],segments:vec![
                TextSegment{id:"s1".into(),text:"Alpha".into(),vector:vec![1.,0.]},
                TextSegment{id:"s2".into(),text:"Beta".into(),vector:vec![0.,1.]},
            ],
        }).unwrap();
        let req=TextAbstractionRequest {
            source_token_ids:vec!["s1".into(),"s2".into()],
            proposed_text:"Alpha und Beta".into(),proposed_source:"reviewed/academy".into(),
        };
        let metadata=TextAbstractionCommit {
            anchor_id:"abstract-doc".into(),token_id:"abstract-one".into(),
            anchor_vector:vec![1.,0.],text_vector:vec![1.,1.],
            review_reference:"review:academy:001".into(),
            reviewed_digest:m.text_abstraction_preview(&req).unwrap().review_digest,
        };
        let r=m.text_abstraction_commit(&req,&metadata).unwrap();
        assert_eq!(r.source_token_ids.len(),2);
        assert_eq!(m.text_memories["s1"].text,"Alpha");
        assert_eq!(m.text_memories["s2"].text,"Beta");
        assert_eq!(m.text_memories["abstract-one"].text,"Alpha und Beta");
        assert!(m.text_abstraction_commit(&req,&metadata).is_err());
        let restored:Memory=serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(restored.abstraction_records["abstract-one"].review_reference,"review:academy:001");
    }

    #[test]
    fn reviewed_digest_rejects_changes_to_proposal_and_sources_atomically() {
        let mut m = Memory::new();
        m.ingest_text(&TextDocument {
            anchor_id: "doc".into(), title:"Story".into(), source:"original".into(),
            anchor_vector:vec![1.,0.], segments:vec![
                TextSegment{id:"s1".into(),text:"Alpha".into(),vector:vec![1.,0.]},
                TextSegment{id:"s2".into(),text:"Beta".into(),vector:vec![0.,1.]},
            ],
        }).unwrap();
        let request=TextAbstractionRequest {
            source_token_ids:vec!["s1".into(),"s2".into()],
            proposed_text:"Alpha und Beta".into(), proposed_source:"academy".into(),
        };
        let reviewed = m.text_abstraction_preview(&request).unwrap();
        let commit=TextAbstractionCommit {
            anchor_id:"abstract-doc".into(), token_id:"abstract-one".into(),
            anchor_vector:vec![1.,0.],text_vector:vec![1.,1.],
            review_reference:"review:001".into(),reviewed_digest:reviewed.review_digest.clone(),
        };
        let original=serde_json::to_value(&m).unwrap();
        let mut changed=request.clone();
        changed.proposed_text="Alpha, Beta und Gamma".into();
        assert!(m.text_abstraction_commit(&changed,&commit).is_err());
        assert_eq!(original,serde_json::to_value(&m).unwrap());
        changed=request.clone();
        changed.source_token_ids.reverse();
        assert!(m.text_abstraction_commit(&changed,&commit).is_err());
        assert_eq!(original,serde_json::to_value(&m).unwrap());
        let record=m.text_abstraction_commit(&request,&commit).unwrap();
        assert_eq!(record.review_digest, reviewed.review_digest);
    }

}
