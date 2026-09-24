//! NDJSON corpus loader enforces byte/count budgets and reports skips instead of hiding loss.

use std::{collections::BTreeMap, io::BufRead};

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    CaptureComparison, CorpusLoadResult, CorpusRecord, CorpusSkip, CorpusSkipReason, ResponseUsage,
};

#[derive(Clone, Debug)]
pub struct CorpusLoadLimits {
    pub max_record_bytes: usize,
    pub max_records: usize,
}

impl Default for CorpusLoadLimits {
    fn default() -> Self {
        Self {
            max_record_bytes: 8 * 1024 * 1024,
            max_records: 1_000_000,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CorpusLoader {
    limits: CorpusLoadLimits,
}

impl CorpusLoader {
    pub fn new(limits: CorpusLoadLimits) -> Self {
        Self { limits }
    }

    pub fn load<R: BufRead>(&self, reader: R) -> CorpusLoadResult {
        let mut result = CorpusLoadResult::default();
        let mut occurrences = BTreeMap::<String, usize>::new();
        for (zero_based, line_result) in reader.split(b'\n').enumerate() {
            let input_line = zero_based + 1;
            let line = match line_result {
                Ok(line) => line,
                Err(error) => {
                    result.skipped.push(CorpusSkip {
                        input_line,
                        reason: CorpusSkipReason::InvalidJson(error.to_string()),
                    });
                    continue;
                }
            };
            if line.iter().all(u8::is_ascii_whitespace) {
                result.skipped.push(CorpusSkip {
                    input_line,
                    reason: CorpusSkipReason::EmptyLine,
                });
                continue;
            }
            if line.len() > self.limits.max_record_bytes {
                result.skipped.push(CorpusSkip {
                    input_line,
                    reason: CorpusSkipReason::RecordTooLarge {
                        bytes: line.len(),
                        limit: self.limits.max_record_bytes,
                    },
                });
                continue;
            }
            if result.corpus.records.len() >= self.limits.max_records {
                result.skipped.push(CorpusSkip {
                    input_line,
                    reason: CorpusSkipReason::RecordBudgetExceeded {
                        limit: self.limits.max_records,
                    },
                });
                continue;
            }
            match parse_record(&line, input_line) {
                Ok(mut record) => {
                    let occurrence = occurrences.entry(record.id.clone()).or_default();
                    record.id = format!("{}:{occurrence}", record.id);
                    *occurrence += 1;
                    result.corpus.records.push(record);
                }
                Err(reason) => result.skipped.push(CorpusSkip { input_line, reason }),
            }
        }
        result
    }
}

fn parse_record(line: &[u8], input_line: usize) -> Result<CorpusRecord, CorpusSkipReason> {
    let record: Value = serde_json::from_slice(line)
        .map_err(|error| CorpusSkipReason::InvalidJson(error.to_string()))?;
    let object = record
        .as_object()
        .ok_or(CorpusSkipReason::RecordNotObject)?;
    let payload = object
        .get("payload")
        .ok_or(CorpusSkipReason::MissingPayload)?;
    if !payload.is_object() {
        return Err(CorpusSkipReason::PayloadNotObject);
    }
    let comparison = object
        .get("comparison")
        .and_then(Value::as_object)
        .ok_or(CorpusSkipReason::MissingComparison)?;
    let endpoint_key = comparison
        .get("endpoint_key")
        .and_then(Value::as_str)
        .ok_or(CorpusSkipReason::MissingEndpointKey)?;
    let endpoint_key = endpoint_key.trim();
    if endpoint_key.is_empty() {
        return Err(CorpusSkipReason::EmptyEndpointKey);
    }
    let agent_key = optional_comparison_key(comparison, "agent_key")?;
    let model_deployment_key = optional_comparison_key(comparison, "model_deployment_key")?;
    let kv_namespace = optional_comparison_key(comparison, "kv_namespace")?;
    let session_key = match object.get("session_key") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.trim().to_owned()),
        Some(Value::String(_)) => return Err(CorpusSkipReason::EmptySessionKey),
        Some(_) => return Err(CorpusSkipReason::InvalidSessionKey),
    };
    let response_usage = payload
        .get("_actrail_response_usage")
        .and_then(Value::as_object)
        .and_then(parse_response_usage);
    // Capture metadata is part of request identity and comparison grouping, not model context.
    let canonical = canonical_json(&record);
    let mut digest = Sha256::new();
    digest.update(canonical.as_bytes());
    let id = hex::encode(digest.finalize());
    Ok(CorpusRecord {
        id,
        captured_at: object
            .get("captured_at")
            .and_then(Value::as_str)
            .map(str::to_owned),
        source: object
            .get("source")
            .and_then(Value::as_str)
            .map(str::to_owned),
        session_key,
        comparison: CaptureComparison {
            endpoint_key: endpoint_key.to_owned(),
            agent_key,
            model_deployment_key,
            kv_namespace,
        },
        payload: payload.clone(),
        response_usage,
        input_line,
    })
}

/// 解析 payload 保留字段 `_actrail_response_usage`。
///
/// 约定（见 `docs/architectures/receiver/input.md`）：
/// - `prompt_tokens` 与 `input_tokens` 是同一字段的两种写法，都表示**总输入**（含命中部分）；
/// - `completion_tokens` 与 `output_tokens` 同理；
/// - `ttft_ms` / `total_ms` 缺失时为 `None`，不按 0 处理；
/// - **缺少总输入数时返回 `None`**：没有分母就算不出命中率，按"没有 usage"处理，
///   让该请求走估算口径，避免产出看起来真实的 0%。
fn parse_response_usage(usage: &serde_json::Map<String, Value>) -> Option<ResponseUsage> {
    let total_input = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .and_then(Value::as_u64)?;
    let completion_tokens = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cached_tokens = usage
        .get("cached_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Some(ResponseUsage {
        prompt_tokens: token_count(total_input),
        cached_tokens: token_count(cached_tokens),
        completion_tokens: token_count(completion_tokens),
        ttft_ms: usage.get("ttft_ms").and_then(Value::as_u64),
        total_ms: usage.get("total_ms").and_then(Value::as_u64),
    })
}

/// token 计数按 u32 收敛；超过上限的记录按上限处理，不做回绕。
fn token_count(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn optional_comparison_key(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<String>, CorpusSkipReason> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_str()
        .ok_or_else(|| CorpusSkipReason::InvalidComparisonKey {
            field: key.to_owned(),
        })?
        .trim();
    if value.is_empty() {
        return Err(CorpusSkipReason::EmptyComparisonKey {
            field: key.to_owned(),
        });
    }
    Ok(Some(value.to_owned()))
}

pub(crate) fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort_unstable();
            let entries = keys.into_iter().map(|key| {
                format!(
                    "{}:{}",
                    serde_json::to_string(key).expect("JSON object key is serializable"),
                    canonical_json(&object[key])
                )
            });
            format!("{{{}}}", entries.collect::<Vec<_>>().join(","))
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => serde_json::to_string(value).expect("parsed JSON value is serializable"),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn loads_valid_records_and_explicitly_skips_every_invalid_line() {
        let input = concat!(
            "{\"captured_at\":\"now\",\"comparison\":{\"endpoint_key\":\"chat\"},\"payload\":{\"model\":\"m\"}}\n",
            "\n",
            "not-json\n",
            "[]\n",
            "{\"source\":\"x\"}\n",
            "{\"payload\":42}\n"
        );
        let result = CorpusLoader::default().load(Cursor::new(input));
        assert_eq!(result.corpus.records.len(), 1);
        assert_eq!(result.skipped.len(), 5);
        assert_eq!(result.corpus.records[0].captured_at.as_deref(), Some("now"));
    }

    #[test]
    fn parses_response_usage_with_aliases_and_latency() {
        let input = concat!(
            "{\"captured_at\":\"now\",\"comparison\":{\"endpoint_key\":\"chat\"},",
            "\"payload\":{\"model\":\"m\",\"_actrail_response_usage\":{",
            "\"input_tokens\":1200,\"cached_tokens\":900,\"output_tokens\":50,",
            "\"ttft_ms\":320,\"total_ms\":1850}}}\n"
        );
        let result = CorpusLoader::default().load(Cursor::new(input));
        let usage = result.corpus.records[0]
            .response_usage
            .as_ref()
            .expect("usage parsed from aliases");
        assert_eq!(usage.prompt_tokens, 1200);
        assert_eq!(usage.cached_tokens, 900);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.ttft_ms, Some(320));
        assert_eq!(usage.total_ms, Some(1850));
    }

    #[test]
    fn usage_without_total_input_is_treated_as_absent() {
        let input = concat!(
            "{\"captured_at\":\"now\",\"comparison\":{\"endpoint_key\":\"chat\"},",
            "\"payload\":{\"model\":\"m\",\"_actrail_response_usage\":{",
            "\"cached_tokens\":900,\"ttft_ms\":320}}}\n"
        );
        let result = CorpusLoader::default().load(Cursor::new(input));
        assert!(
            result.corpus.records[0].response_usage.is_none(),
            "缺少总输入数时按没有 usage 处理，避免产出看起来真实的 0%"
        );
    }

    #[test]
    fn omitted_latency_stays_absent_instead_of_zero() {
        let input = concat!(
            "{\"captured_at\":\"now\",\"comparison\":{\"endpoint_key\":\"chat\"},",
            "\"payload\":{\"model\":\"m\",\"_actrail_response_usage\":{\"prompt_tokens\":100}}}\n"
        );
        let result = CorpusLoader::default().load(Cursor::new(input));
        let usage = result.corpus.records[0]
            .response_usage
            .as_ref()
            .expect("usage");
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.ttft_ms, None);
        assert_eq!(usage.total_ms, None);
    }

    #[test]
    fn stable_id_ignores_outer_object_key_order() {
        let a = "{\"comparison\":{\"endpoint_key\":\"chat\"},\"payload\":{\"model\":\"m\",\"messages\":[]}}";
        let b = "{\"payload\":{\"messages\":[],\"model\":\"m\"},\"comparison\":{\"endpoint_key\":\"chat\"}}";
        let a = CorpusLoader::default().load(Cursor::new(a));
        let b = CorpusLoader::default().load(Cursor::new(b));
        assert_eq!(a.corpus.records[0].id, b.corpus.records[0].id);
    }

    #[test]
    fn duplicate_records_keep_frequency_with_permutation_stable_ids() {
        let a = "{\"source\":\"a\",\"comparison\":{\"endpoint_key\":\"chat\"},\"payload\":{\"model\":\"m\"}}";
        let b = "{\"source\":\"b\",\"comparison\":{\"endpoint_key\":\"chat\"},\"payload\":{\"model\":\"m\"}}";
        let first = CorpusLoader::default().load(Cursor::new(format!("{a}\n{b}\n{a}")));
        let second = CorpusLoader::default().load(Cursor::new(format!("{a}\n{a}\n{b}")));
        let mut first_ids: Vec<_> = first
            .corpus
            .records
            .iter()
            .map(|record| record.id.clone())
            .collect();
        let mut second_ids: Vec<_> = second
            .corpus
            .records
            .iter()
            .map(|record| record.id.clone())
            .collect();
        first_ids.sort();
        second_ids.sort();
        assert_eq!(first_ids, second_ids);
        assert_eq!(first_ids.len(), 3);
        assert_ne!(first_ids[0], first_ids[1]);
    }

    #[test]
    fn rejects_empty_or_non_string_comparison_keys() {
        let input = concat!(
            "{\"comparison\":{\"endpoint_key\":\"   \"},\"payload\":{}}\n",
            "{\"comparison\":{\"endpoint_key\":\"chat\",\"agent_key\":42},\"payload\":{}}\n",
            "{\"comparison\":{\"endpoint_key\":\"chat\",\"kv_namespace\":\" \"},\"payload\":{}}"
        );
        let result = CorpusLoader::default().load(Cursor::new(input));
        assert!(result.corpus.records.is_empty());
        assert!(matches!(
            result.skipped[0].reason,
            CorpusSkipReason::EmptyEndpointKey
        ));
        assert!(matches!(
            result.skipped[1].reason,
            CorpusSkipReason::InvalidComparisonKey { .. }
        ));
        assert!(matches!(
            result.skipped[2].reason,
            CorpusSkipReason::EmptyComparisonKey { .. }
        ));
    }

    #[test]
    fn enforces_record_size_and_count_boundaries() {
        let at_limit = "{\"comparison\":{\"endpoint_key\":\"x\"},\"payload\":{}}";
        let limits = CorpusLoadLimits {
            max_record_bytes: at_limit.len(),
            max_records: 1,
        };
        let input = format!("{at_limit}\n{at_limit} ");
        let result = CorpusLoader::new(limits).load(Cursor::new(input));
        assert_eq!(result.corpus.records.len(), 1);
        assert!(matches!(
            result.skipped[0].reason,
            CorpusSkipReason::RecordTooLarge { .. }
        ));
    }
}
