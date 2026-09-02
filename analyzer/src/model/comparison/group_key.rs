//! Comparison group keys combine fixed time windows with explicit capture and adapter metadata.

use crate::model::corpus::CorpusRecord;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComparisonGroupKey {
    pub time_window_key: i64,
    pub endpoint_key: String,
    pub model: String,
    pub model_deployment_key: Option<String>,
    pub context_schema_key: String,
    pub agent_key: Option<String>,
    pub kv_namespace: Option<String>,
    pub dialect: String,
    pub adapter_revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComparisonGroupError {
    MissingCapturedAt,
    InvalidCapturedAt(String),
    InvalidWindowSeconds,
    MissingModel,
}

impl ComparisonGroupKey {
    pub fn from_record(
        record: &CorpusRecord,
        window_seconds: u64,
        dialect: &str,
        adapter_revision: &str,
        context_schema_key: &str,
    ) -> Result<Self, ComparisonGroupError> {
        if window_seconds == 0 {
            return Err(ComparisonGroupError::InvalidWindowSeconds);
        }
        let captured_at = record
            .captured_at
            .as_deref()
            .ok_or(ComparisonGroupError::MissingCapturedAt)?;
        let timestamp = time::OffsetDateTime::parse(
            captured_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|error| ComparisonGroupError::InvalidCapturedAt(error.to_string()))?
        .unix_timestamp();
        let window = i64::try_from(window_seconds)
            .map_err(|_| ComparisonGroupError::InvalidWindowSeconds)?;
        let model = record
            .payload
            .get("model")
            .and_then(serde_json::Value::as_str)
            .filter(|model| !model.is_empty())
            .ok_or(ComparisonGroupError::MissingModel)?;
        Ok(Self {
            time_window_key: timestamp.div_euclid(window) * window,
            endpoint_key: record.comparison.endpoint_key.clone(),
            model: model.to_owned(),
            model_deployment_key: record.comparison.model_deployment_key.clone(),
            context_schema_key: context_schema_key.to_owned(),
            agent_key: record.comparison.agent_key.clone(),
            kv_namespace: record.comparison.kv_namespace.clone(),
            dialect: dialect.to_owned(),
            adapter_revision: adapter_revision.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::model::corpus::{CaptureComparison, CorpusRecord};

    fn record(captured_at: &str) -> CorpusRecord {
        CorpusRecord {
            id: captured_at.into(),
            captured_at: Some(captured_at.into()),
            source: None,
            comparison: CaptureComparison {
                endpoint_key: "chat".into(),
                agent_key: Some("agent-a".into()),
                model_deployment_key: Some("deployment-a".into()),
                kv_namespace: Some("tenant-a".into()),
            },
            payload: json!({"model": "model-a", "messages": []}),
            input_line: 1,
        }
    }

    fn key(record: &CorpusRecord) -> ComparisonGroupKey {
        ComparisonGroupKey::from_record(record, 3_600, "chat", "1", "chat/v1").unwrap()
    }

    #[test]
    fn fixed_windows_are_utc_epoch_aligned_and_end_is_exclusive() {
        let before = key(&record("2026-09-02T08:59:59.999Z"));
        let boundary = key(&record("2026-09-02T09:00:00Z"));
        assert_eq!(boundary.time_window_key - before.time_window_key, 3_600);
    }

    #[test]
    fn every_explicit_cache_scope_dimension_is_part_of_identity() {
        let baseline_record = record("2026-09-02T08:00:00Z");
        let baseline = key(&baseline_record);
        let mut variants = Vec::new();
        let mut endpoint = baseline_record.clone();
        endpoint.comparison.endpoint_key = "responses".into();
        variants.push(key(&endpoint));
        let mut deployment = baseline_record.clone();
        deployment.comparison.model_deployment_key = None;
        variants.push(key(&deployment));
        let mut agent = baseline_record.clone();
        agent.comparison.agent_key = None;
        variants.push(key(&agent));
        let mut namespace = baseline_record.clone();
        namespace.comparison.kv_namespace = None;
        variants.push(key(&namespace));
        let mut model = baseline_record.clone();
        model.payload["model"] = json!("model-b");
        variants.push(key(&model));
        assert!(variants.iter().all(|variant| variant != &baseline));
    }

    #[test]
    fn invalid_timestamp_and_zero_window_are_rejected() {
        let invalid = record("not-a-time");
        assert!(matches!(
            ComparisonGroupKey::from_record(&invalid, 3_600, "chat", "1", "chat/v1"),
            Err(ComparisonGroupError::InvalidCapturedAt(_))
        ));
        assert_eq!(
            ComparisonGroupKey::from_record(
                &record("2026-09-02T08:00:00Z"),
                0,
                "chat",
                "1",
                "chat/v1"
            ),
            Err(ComparisonGroupError::InvalidWindowSeconds)
        );
    }
}
