//! 本文件用性质测试验证业务值无关、动态后缀抑制和稳定字节单调性。

use std::io::Cursor;

use actrail_kv_analyzer::run::{analyze_reader, AnalysisOptions};
use actrail_kv_artifacts::FindingCause;
use proptest::prelude::*;
use serde_json::{json, Value};

fn analyze(payloads: Vec<Value>) -> actrail_kv_artifacts::AnalysisResult {
    let input = payloads
        .into_iter()
        .map(|payload| serde_json::to_string(&json!({"payload":payload})).expect("fixture"))
        .collect::<Vec<_>>()
        .join("\n");
    analyze_reader(Cursor::new(input), AnalysisOptions::default()).expect("analysis")
}

fn request(first: String, tail: String) -> Value {
    json!({
        "model":"property-model",
        "messages":[
            {"role":"system","content":first},
            {"role":"system","content":tail}
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn arbitrary_business_values_still_use_relative_structure(
        values in prop::collection::vec("[a-zA-Z0-9 ]{1,16}", 4)
    ) {
        let tail = "共同且足够长的稳定规则。".repeat(24);
        let payloads = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| request(format!("{index}:{value}"), tail.clone()))
            .collect();
        let result = analyze(payloads);
        prop_assert!(result
            .findings
            .iter()
            .any(|finding| finding.cause == FindingCause::EarlyVariableContent));
    }

    #[test]
    fn arbitrary_last_user_messages_never_become_opportunities(
        values in prop::collection::vec("[a-zA-Z0-9 ]{1,20}", 4)
    ) {
        let stable = "stable prefix instruction ".repeat(12);
        let payloads = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| json!({
                "model":"property-model",
                "messages":[
                    {"role":"system","content":stable},
                    {"role":"user","content":format!("{index}:{value}")}
                ]
            }))
            .collect();
        prop_assert!(analyze(payloads).findings.is_empty());
    }

    #[test]
    fn extending_a_common_stable_tail_never_reduces_blocked_bytes(
        extra in 0usize..160
    ) {
        let values = ["one", "two", "three", "four"];
        let short_tail = "S".repeat(96);
        let long_tail = format!("{short_tail}{}", "L".repeat(extra));
        let short = analyze(values.iter().map(|value| request((*value).into(), short_tail.clone())).collect());
        let long = analyze(values.iter().map(|value| request((*value).into(), long_tail.clone())).collect());
        let short_bytes = short.findings.first().expect("short finding").blocked_stable_bytes;
        let long_bytes = long.findings.first().expect("long finding").blocked_stable_bytes;
        prop_assert!(long_bytes >= short_bytes);
    }
}
