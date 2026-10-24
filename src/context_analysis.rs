use crate::Memory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Descriptive counts for one context value.
/// Records are not assumed to be statistically independent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextGroup {
    pub value: String,
    pub observed: u64,
    pub missed: u64,
    pub total: u64,
    pub empirical_rate: f64,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextAnalysis {
    pub from: String,
    pub to: String,
    pub context_key: String,
    pub cutoff_unix_ms: i64,
    pub groups: Vec<ContextGroup>,
    pub eligible_records_without_key: usize,
}

impl Memory {
    /// Compare recorded transition outcomes by a context field.
    /// No inference, weight update or hypothesis confirmation.
    pub fn analyze_context_as_of(
        &self,
        from: &str,
        to: &str,
        context_key: &str,
        cutoff_unix_ms: i64,
    ) -> Result<ContextAnalysis, String> {
        if context_key.trim().is_empty() {
            return Err("Context key must not be empty".into());
        }

        if !self.states.contains_key(from) || !self.states.contains_key(to) {
            return Err("Unknown transition state".into());
        }

        let mut groups: BTreeMap<String, ContextGroup> = BTreeMap::new();
        let mut missing = 0usize;

        for record in self.contextual_evidence.values() {
            if record.from != from
                || record.to != to
                || record.available_at_unix_ms > cutoff_unix_ms
            {
                continue;
            }

            let Some(value) = record.context.get(context_key) else {
                missing += 1;
                continue;
            };

            let group = groups.entry(value.clone()).or_insert_with(|| ContextGroup {
                value: value.clone(),
                observed: 0,
                missed: 0,
                total: 0,
                empirical_rate: 0.0,
                evidence_ids: Vec::new(),
            });

            if record.occurred {
                group.observed = group
                    .observed
                    .checked_add(1)
                    .ok_or("Observed counter overflow")?;
            } else {
                group.missed = group
                    .missed
                    .checked_add(1)
                    .ok_or("Missed counter overflow")?;
            }

            group.total = group.total.checked_add(1).ok_or("Total counter overflow")?;
            group.evidence_ids.push(record.id.clone());
        }

        let mut result: Vec<ContextGroup> = groups.into_values().collect();

        for group in &mut result {
            group.empirical_rate = group.observed as f64 / group.total as f64;
            group.evidence_ids.sort();
        }

        Ok(ContextAnalysis {
            from: from.into(),
            to: to.into(),
            context_key: context_key.into(),
            cutoff_unix_ms,
            groups: result,
            eligible_records_without_key: missing,
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
        let mut m = Memory::new();

        for id in ["input", "output"] {
            m.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })
            .unwrap();
        }

        for (id, mode, occurred, available) in [
            ("e1", "automated", true, 100),
            ("e2", "automated", true, 200),
            ("e3", "manual", false, 300),
            ("e4", "manual", true, 500),
        ] {
            let context = BTreeMap::from([("mode".into(), mode.into())]);

            m.record_contextual_evidence(ContextualEvidenceRecord {
                schema_version: CONTEXTUAL_EVIDENCE_SCHEMA_VERSION,
                id: id.into(),
                from: "input".into(),
                to: "output".into(),
                occurred,
                observed_at_unix_ms: 0,
                available_at_unix_ms: available,
                source: EvidenceSource::ExternalObservation,
                reference: format!("synthetic/{id}"),
                context,
            })
            .unwrap();
        }

        m
    }

    #[test]
    fn groups_evidence_by_context() {
        let m = fixture();
        let report = m
            .analyze_context_as_of("input", "output", "mode", 1000)
            .unwrap();

        assert_eq!(report.groups.len(), 2);
        assert_eq!(report.groups[0].value, "automated");
        assert_eq!(report.groups[0].observed, 2);
        assert_eq!(report.groups[0].empirical_rate, 1.0);
        assert_eq!(report.groups[1].value, "manual");
        assert_eq!(report.groups[1].observed, 1);
        assert_eq!(report.groups[1].missed, 1);
        assert_eq!(report.groups[1].empirical_rate, 0.5);
    }

    #[test]
    fn future_evidence_is_excluded() {
        let m = fixture();
        let report = m
            .analyze_context_as_of("input", "output", "mode", 300)
            .unwrap();

        assert_eq!(report.groups[1].total, 1);
        assert_eq!(report.groups[1].empirical_rate, 0.0);
        assert!(!report.groups[1].evidence_ids.contains(&"e4".to_string()));
    }

    #[test]
    fn analysis_is_read_only_and_deterministic() {
        let m = fixture();
        let before = serde_json::to_value(&m).unwrap();

        let a = m
            .analyze_context_as_of("input", "output", "mode", 1000)
            .unwrap();
        let b = m
            .analyze_context_as_of("input", "output", "mode", 1000)
            .unwrap();

        assert_eq!(a, b);
        assert_eq!(serde_json::to_value(&m).unwrap(), before);
    }

    #[test]
    fn missing_context_and_unknown_states_are_handled() {
        let m = fixture();

        let report = m
            .analyze_context_as_of("input", "output", "unknown_key", 1000)
            .unwrap();

        assert!(report.groups.is_empty());
        assert_eq!(report.eligible_records_without_key, 4);

        assert!(m
            .analyze_context_as_of("missing", "output", "mode", 1000)
            .is_err());
    }
}
