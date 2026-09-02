//! NDJSON corpus loader enforces byte/count budgets and reports skips instead of hiding loss.

use std::{collections::BTreeMap, io::BufRead};

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{CorpusLoadResult, CorpusRecord, CorpusSkip, CorpusSkipReason};

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
    // Capture metadata is part of identity but never enters context projection or comparison.
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
        payload: payload.clone(),
        input_line,
    })
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
            "{\"captured_at\":\"now\",\"payload\":{\"model\":\"m\"}}\n",
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
    fn stable_id_ignores_outer_object_key_order() {
        let a = "{\"payload\":{\"model\":\"m\",\"messages\":[]}}";
        let b = "{\"payload\":{\"messages\":[],\"model\":\"m\"}}";
        let a = CorpusLoader::default().load(Cursor::new(a));
        let b = CorpusLoader::default().load(Cursor::new(b));
        assert_eq!(a.corpus.records[0].id, b.corpus.records[0].id);
    }

    #[test]
    fn duplicate_records_keep_frequency_with_permutation_stable_ids() {
        let a = "{\"source\":\"a\",\"payload\":{\"model\":\"m\"}}";
        let b = "{\"source\":\"b\",\"payload\":{\"model\":\"m\"}}";
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
    fn enforces_record_size_and_count_boundaries() {
        let limits = CorpusLoadLimits {
            max_record_bytes: 14,
            max_records: 1,
        };
        let at_limit = "{\"payload\":{}}";
        assert_eq!(at_limit.len(), 14);
        let input = format!("{at_limit}\n{at_limit} ");
        let result = CorpusLoader::new(limits).load(Cursor::new(input));
        assert_eq!(result.corpus.records.len(), 1);
        assert!(matches!(
            result.skipped[0].reason,
            CorpusSkipReason::RecordTooLarge { .. }
        ));
    }
}
