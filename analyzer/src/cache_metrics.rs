//! 计算每请求 KV 命中率：优先使用上游 usage，缺失时按与上一条请求的上下文公共前缀估算。

use std::collections::HashMap;

use actrail_kv_artifacts::{CacheMetricBasis, KvCacheMetric};

use crate::model::corpus::CorpusRecord;

pub(crate) fn compute_cache_metrics(records: &[CorpusRecord]) -> Vec<KvCacheMetric> {
    let mut group_index: HashMap<Option<String>, usize> = HashMap::new();
    let mut groups: Vec<(Option<String>, Vec<usize>)> = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let key = record.session_key.clone();
        let group = *group_index.entry(key.clone()).or_insert_with(|| {
            groups.push((key, Vec::new()));
            groups.len() - 1
        });
        groups[group].1.push(index);
    }

    let mut metrics = Vec::new();
    for (session_key, mut indices) in groups {
        indices.sort_by(|left, right| {
            let left_record = &records[*left];
            let right_record = &records[*right];
            left_record
                .captured_at
                .as_deref()
                .unwrap_or("")
                .cmp(right_record.captured_at.as_deref().unwrap_or(""))
                .then_with(|| left_record.input_line.cmp(&right_record.input_line))
        });
        let mut previous_payload: Option<Vec<u8>> = None;
        let mut previous_rate: Option<f64> = None;
        for (sequence, index) in indices.into_iter().enumerate() {
            let record = &records[index];
            let mut estimation_payload = record.payload.clone();
            if let serde_json::Value::Object(object) = &mut estimation_payload {
                object.remove("_actrail_response_usage");
            }
            let payload_bytes = serde_json::to_vec(&estimation_payload)
                .map(|bytes| bytes.len())
                .unwrap_or(0);
            let current_payload = serde_json::to_vec(&estimation_payload).unwrap_or_default();
            let estimated_lcp = previous_payload
                .as_deref()
                .map(|previous| common_prefix_len(previous, &current_payload));

            let (prompt_tokens, cached_tokens, miss_tokens, output_tokens, hit_rate, basis) =
                match &record.response_usage {
                    Some(usage) => {
                        let prompt = usage.prompt_tokens;
                        let cached = usage.cached_tokens.min(prompt);
                        let rate = if prompt > 0 {
                            f64::from(cached) / f64::from(prompt)
                        } else {
                            0.0
                        };
                        (
                            Some(prompt),
                            Some(cached),
                            Some(prompt.saturating_sub(cached)),
                            Some(usage.completion_tokens),
                            rate,
                            CacheMetricBasis::Reported,
                        )
                    }
                    None => {
                        let lcp = estimated_lcp.unwrap_or(0);
                        let rate = if payload_bytes > 0 {
                            lcp as f64 / payload_bytes as f64
                        } else {
                            0.0
                        };
                        (None, None, None, None, rate, CacheMetricBasis::Estimated)
                    }
                };
            let hit_rate_delta = previous_rate.map(|previous| hit_rate - previous);
            let estimated_basis = matches!(basis, CacheMetricBasis::Estimated);
            metrics.push(KvCacheMetric {
                request_id: record.id.clone(),
                session_key: session_key.clone(),
                sequence: sequence + 1,
                captured_at: record.captured_at.clone(),
                source: record.source.clone(),
                prompt_tokens,
                cached_tokens,
                miss_tokens,
                output_tokens,
                hit_rate,
                hit_rate_delta,
                basis,
                estimated_lcp_bytes: if estimated_basis { estimated_lcp } else { None },
                payload_bytes,
            });
            previous_payload = Some(current_payload);
            previous_rate = Some(hit_rate);
        }
    }
    metrics
}

fn common_prefix_len(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(left, right)| left == right)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::corpus::{CaptureComparison, ResponseUsage};
    use serde_json::json;

    fn record(
        id: &str,
        session: Option<&str>,
        messages: serde_json::Value,
        usage: Option<(u32, u32, u32)>,
    ) -> CorpusRecord {
        CorpusRecord {
            id: id.to_string(),
            captured_at: Some(format!("2026-01-01T00:00:0{id}Z")),
            source: None,
            session_key: session.map(str::to_owned),
            comparison: CaptureComparison {
                endpoint_key: "primary".to_string(),
                agent_key: None,
                model_deployment_key: None,
                kv_namespace: None,
            },
            payload: json!({ "model": "m", "messages": messages }),
            response_usage: usage.map(|(prompt, cached, completion)| ResponseUsage {
                prompt_tokens: prompt,
                cached_tokens: cached,
                completion_tokens: completion,
            }),
            input_line: id.parse().unwrap_or(0),
        }
    }

    #[test]
    fn reported_usage_produces_hit_rate_and_delta() {
        let records = vec![
            record(
                "1",
                Some("s"),
                json!([{"role":"user","content":"a"}]),
                Some((1000, 200, 10)),
            ),
            record(
                "2",
                Some("s"),
                json!([{"role":"user","content":"a"},{"role":"user","content":"b"}]),
                Some((1200, 900, 20)),
            ),
        ];
        let metrics = compute_cache_metrics(&records);
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].basis, CacheMetricBasis::Reported);
        assert!((metrics[0].hit_rate - 0.2).abs() < 1e-9);
        assert!(metrics[0].hit_rate_delta.is_none());
        assert_eq!(metrics[1].cached_tokens, Some(900));
        assert_eq!(metrics[1].miss_tokens, Some(300));
        let delta = metrics[1].hit_rate_delta.expect("delta");
        assert!((delta - 0.55).abs() < 1e-9, "delta={delta}");
    }

    #[test]
    fn missing_usage_falls_back_to_context_prefix_estimate() {
        let records = vec![
            record(
                "1",
                Some("s"),
                json!([{"role":"user","content":"same-prefix"}]),
                None,
            ),
            record(
                "2",
                Some("s"),
                json!([{"role":"user","content":"same-prefix"},{"role":"user","content":"tail"}]),
                None,
            ),
        ];
        let metrics = compute_cache_metrics(&records);
        assert_eq!(metrics[0].basis, CacheMetricBasis::Estimated);
        assert!(metrics[0].prompt_tokens.is_none());
        assert!(metrics[0].estimated_lcp_bytes.is_none());
        assert!(metrics[1].estimated_lcp_bytes.unwrap_or(0) > 0);
        assert!(metrics[1].hit_rate > 0.0);
        assert!(metrics[1].hit_rate_delta.is_some());
    }
}
