use crate::{
    context_analysis::ContextAnalysis, evidence_quality::EvidenceQualityReport,
    reflect::ReflectReport, Memory,
};
use serde::{Deserialize, Serialize};

/// Read-only coordination of existing evidence analyses.
/// Mixed outcomes are review signals, not proven contradictions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextualReflectPreview {
    pub from: String,
    pub to: String,
    pub context_key: String,
    pub cutoff_unix_ms: i64,
    pub reflection: ReflectReport,
    pub context_analysis: ContextAnalysis,
    pub evidence_quality: EvidenceQualityReport,
    pub mixed_context_values: Vec<String>,
    pub review_required: bool,
    pub memory_changed: bool,
}

impl Memory {
    pub fn reflect_contextual_as_of(
        &self,
        from: &str,
        to: &str,
        context_key: &str,
        cutoff_unix_ms: i64,
    ) -> Result<ContextualReflectPreview, String> {
        let context_analysis = self.analyze_context_as_of(from, to, context_key, cutoff_unix_ms)?;

        let evidence_quality = self.analyze_evidence_quality_as_of(from, to, cutoff_unix_ms)?;

        let reflection = self.reflect_preview()?;

        let mixed_context_values: Vec<String> = context_analysis
            .groups
            .iter()
            .filter(|group| group.observed > 0 && group.missed > 0)
            .map(|group| group.value.clone())
            .collect();

        let review_required = !mixed_context_values.is_empty();

        Ok(ContextualReflectPreview {
            from: from.into(),
            to: to.into(),
            context_key: context_key.into(),
            cutoff_unix_ms,
            reflection,
            context_analysis,
            evidence_quality,
            mixed_context_values,
            review_required,
            memory_changed: false,
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
    use std::collections::BTreeMap;

    fn fixture() -> Memory {
        let mut memory = Memory::new();

        for id in ["event", "outcome"] {
            memory
                .insert(StateToken {
                    id: id.into(),
                    vector: vec![1.0, 0.0],
                })
                .unwrap();
        }

        for (id, mode, occurred, available, reference) in [
            ("a", "normal", true, 10, "source/a"),
            ("b", "normal", false, 20, "source/b"),
            ("c", "special", true, 30, "source/c"),
            ("d", "normal", true, 40, "source/a"),
        ] {
            memory
                .record_contextual_evidence(ContextualEvidenceRecord {
                    schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                    id: id.into(),
                    from: "event".into(),
                    to: "outcome".into(),
                    occurred,
                    observed_at_unix_ms: 0,
                    available_at_unix_ms: available,
                    source: EvidenceSource::ExternalObservation,
                    reference: reference.into(),
                    context: BTreeMap::from([("mode".into(), mode.into())]),
                })
                .unwrap();
        }

        memory
    }

    #[test]
    fn mixed_outcomes_require_review() {
        let report = fixture()
            .reflect_contextual_as_of("event", "outcome", "mode", 40)
            .unwrap();

        assert!(report.review_required);
        assert_eq!(report.mixed_context_values, vec!["normal"]);
        assert_eq!(report.evidence_quality.total_records, 4);
        assert_eq!(report.evidence_quality.repeated_reference_groups, 1);
        assert!(!report.memory_changed);
    }

    #[test]
    fn historical_cutoff_prevents_future_leakage() {
        let memory = fixture();

        let early = memory
            .reflect_contextual_as_of("event", "outcome", "mode", 10)
            .unwrap();
        assert!(!early.review_required);
        assert!(early.mixed_context_values.is_empty());
        assert_eq!(early.evidence_quality.total_records, 1);

        let later = memory
            .reflect_contextual_as_of("event", "outcome", "mode", 20)
            .unwrap();
        assert!(later.review_required);
        assert_eq!(later.evidence_quality.total_records, 2);
    }

    #[test]
    fn analysis_is_deterministic_and_read_only() {
        let memory = fixture();
        let before = serde_json::to_value(&memory).unwrap();

        let first = memory
            .reflect_contextual_as_of("event", "outcome", "mode", 40)
            .unwrap();
        let second = memory
            .reflect_contextual_as_of("event", "outcome", "mode", 40)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(before, serde_json::to_value(&memory).unwrap());
    }

    #[test]
    fn invalid_context_and_states_are_rejected() {
        let memory = fixture();

        assert!(memory
            .reflect_contextual_as_of("event", "outcome", "", 40)
            .is_err());
        assert!(memory
            .reflect_contextual_as_of("missing", "outcome", "mode", 40)
            .is_err());
    }
}
