//! 本文件验收 Session ID 经分析主链产生相邻请求前缀结果。

use std::io::Cursor;

use actrail_kv_analyzer::run::{analyze_reader, AnalysisOptions};
use actrail_kv_artifacts::SessionTransitionOutcome;
use serde_json::{json, Value};

#[test]
fn dynamic_result_append_is_normal_then_later_rewrite_is_history_change() {
    let history = json!({"role":"system","content":"stable system"});
    let tool_result = json!({"role":"tool","tool_call_id":"call-1","content":"result-a"});
    let requests = vec![
        request("2026-09-03T00:00:00Z", vec![history.clone()]),
        request(
            "2026-09-03T00:00:01Z",
            vec![history.clone(), tool_result.clone()],
        ),
        request(
            "2026-09-03T00:00:02Z",
            vec![
                history.clone(),
                tool_result,
                json!({"role":"user","content":"next"}),
            ],
        ),
        request(
            "2026-09-03T00:00:03Z",
            vec![
                history,
                json!({"role":"tool","tool_call_id":"call-1","content":"result-b"}),
                json!({"role":"user","content":"next"}),
            ],
        ),
    ];
    let input = requests
        .into_iter()
        .map(|request| serde_json::to_string(&request).expect("serialize fixture"))
        .collect::<Vec<_>>()
        .join("\n");
    let result =
        analyze_reader(Cursor::new(input), AnalysisOptions::default()).expect("analysis succeeds");
    let transitions = &result.session_analysis.timelines[0].transitions;
    assert!(matches!(
        transitions[0].outcome,
        SessionTransitionOutcome::NormalAppend { .. }
    ));
    assert!(matches!(
        transitions[1].outcome,
        SessionTransitionOutcome::NormalAppend { .. }
    ));
    let SessionTransitionOutcome::HistoryChanged { metrics, .. } = &transitions[2].outcome else {
        panic!("expected old tool result rewrite to change history")
    };
    assert!(metrics.invalidated_previous_suffix_bytes > 0);
    assert_eq!(result.session_analysis.history_sites.len(), 1);
}

fn request(captured_at: &str, messages: Vec<Value>) -> Value {
    json!({
        "captured_at": captured_at,
        "session_id": "session-a",
        "comparison": {"endpoint_key": "primary"},
        "payload": {"model": "model-a", "messages": messages}
    })
}
