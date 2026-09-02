//! 本文件定义 requests.ndjson 中每一行的采集请求格式。

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedRequest {
    pub captured_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub payload: Value,
}

#[cfg(test)]
mod tests {
    use super::CapturedRequest;
    use serde_json::json;

    #[test]
    fn round_trips_without_losing_payload_order_or_values() {
        let record = CapturedRequest {
            captured_at: "2026-09-02T08:09:10.123456789Z".to_owned(),
            source: Some("instrumentation/agent-a".to_owned()),
            payload: json!({"model": "example", "messages": [{"role": "user", "content": "你好"}]}),
        };

        let encoded = serde_json::to_string(&record).expect("serialize captured request");
        let decoded: CapturedRequest =
            serde_json::from_str(&encoded).expect("deserialize captured request");

        assert_eq!(decoded, record);
    }

    #[test]
    fn omits_absent_source_but_accepts_it_on_round_trip() {
        let record = CapturedRequest {
            captured_at: "2026-09-02T08:09:10Z".to_owned(),
            source: None,
            payload: json!({}),
        };

        let encoded = serde_json::to_value(&record).expect("serialize captured request");
        assert!(encoded.get("source").is_none());
        assert_eq!(
            serde_json::from_value::<CapturedRequest>(encoded).expect("deserialize record"),
            record
        );
    }
}
