//! 本文件以真实 captured NDJSON payload 验收八类通用诊断和关键零误报边界。

use std::io::Cursor;

use actrail_kv_analyzer::run::{analyze_reader, AnalysisOptions};
use actrail_kv_artifacts::{AnalysisResult, FindingCause};
use serde_json::{json, Value};

fn stable(label: &str) -> String {
    format!("{label}:{}", "稳定规则内容。".repeat(32))
}

fn analyze(payloads: Vec<Value>) -> AnalysisResult {
    let mut input = String::new();
    for payload in payloads {
        input.push_str(
            &serde_json::to_string(
                &json!({"captured_at":"2026-01-01T00:00:00Z","payload":payload}),
            )
            .expect("fixture JSON"),
        );
        input.push('\n');
    }
    analyze_reader(
        Cursor::new(input),
        AnalysisOptions {
            top_k: 20,
            ..AnalysisOptions::default()
        },
    )
    .expect("analysis succeeds")
}

fn causes(result: &AnalysisResult) -> Vec<FindingCause> {
    result
        .findings
        .iter()
        .map(|finding| finding.cause.clone())
        .collect()
}

fn chat(messages: Vec<Value>) -> Value {
    json!({"model":"model-a","messages":messages})
}

#[test]
fn detects_early_variable_content_without_value_shape_rules() {
    let tail = stable("shared-after-volatile");
    let payloads = ["青山", "ocean account", "Δοκιμή", "客户七"]
        .into_iter()
        .map(|value| {
            chat(vec![
                json!({"role":"system","content":value}),
                json!({"role":"system","content":tail}),
            ])
        })
        .collect();
    let result = analyze(payloads);
    assert!(causes(&result).contains(&FindingCause::EarlyVariableContent));
}

#[test]
fn detects_inline_dynamic_slot_with_unstructured_values() {
    let suffix = stable("common-inline-suffix");
    let contents: Vec<_> = ["青山", "ocean", "Δοκιμή", "客户七"]
        .into_iter()
        .map(|value| format!("固定开头::{value}::{suffix}"))
        .collect();
    let payloads = contents
        .iter()
        .map(|content| chat(vec![json!({"role":"system","content":content})]))
        .collect();
    let result = analyze(payloads);
    assert!(causes(&result).iter().any(|cause| {
        matches!(
            cause,
            FindingCause::InlineDynamicSlot | FindingCause::SystemPromptDrift
        )
    }));
    let finding = result.findings.first().expect("finding");
    let offset = finding.source.byte_start.expect("byte offset");
    assert!(contents
        .iter()
        .all(|content| offset <= content.len() && content.is_char_boundary(offset)));
}

#[test]
fn detects_dynamic_block_inserted_before_stable_block() {
    let stable_block = stable("fixed-few-shot");
    let mut payloads = vec![chat(vec![json!({"role":"system","content":stable_block})])];
    for value in ["retrieval-A", "another knowledge", "检索片段三"] {
        payloads.push(chat(vec![
            json!({"role":"system","content":value}),
            json!({"role":"system","content":stable_block}),
        ]));
    }
    let result = analyze(payloads);
    assert!(
        causes(&result).contains(&FindingCause::DynamicBlockBeforeStable),
        "causes={:?}, templates={}",
        causes(&result),
        result.templates.len()
    );
}

#[test]
fn detects_unique_tool_reordering() {
    let tool_a = json!({"type":"function","function":{"name":"lookup","description":stable("lookup"),"parameters":{"type":"object"}}});
    let tool_b = json!({"type":"function","function":{"name":"calculate","description":stable("calculate"),"parameters":{"type":"object"}}});
    let payloads = [false, false, true, true]
        .into_iter()
        .map(|reversed| {
            let tools = if reversed {
                vec![tool_b.clone(), tool_a.clone()]
            } else {
                vec![tool_a.clone(), tool_b.clone()]
            };
            json!({"model":"model-a","tools":tools,"messages":[{"role":"system","content":stable("after-tools")} ]})
        })
        .collect();
    let result = analyze(payloads);
    assert!(causes(&result).contains(&FindingCause::ToolOrderDrift));
}

#[test]
fn detects_tool_definition_and_prompt_version_drift() {
    let descriptions = [
        "Search internal docs.",
        "Search internal docs.",
        "Search internal documentation!",
        "Search internal documentation!",
    ];
    let tool_payloads = descriptions
        .into_iter()
        .map(|description| json!({
            "model":"model-a",
            "tools":[{"type":"function","function":{"name":"lookup","description":description,"parameters":{"type":"object","properties":{"query":{"type":"string"}}}}}],
            "messages":[{"role":"system","content":stable("after-definition")}]
        }))
        .collect();
    assert!(causes(&analyze(tool_payloads)).contains(&FindingCause::ToolDefinitionDrift));

    let prompt_payloads = [
        "Follow policy A. ",
        "Follow policy A. ",
        "Follow policy A! ",
        "Follow policy A! ",
    ]
    .into_iter()
    .map(|prefix| {
        chat(vec![json!({
            "role":"system",
            "content":format!("{prefix}{}", stable("prompt-tail"))
        })])
    })
    .collect();
    assert!(causes(&analyze(prompt_payloads)).contains(&FindingCause::SystemPromptDrift));
}

#[test]
fn detects_non_append_history_and_visible_json_formatting() {
    let old = stable("old-history");
    let baseline = chat(vec![
        json!({"role":"system","content":"fixed"}),
        json!({"role":"user","content":old}),
    ]);
    let mut histories = vec![baseline];
    for value in ["new-a", "新增消息乙", "another-new"] {
        histories.push(chat(vec![
            json!({"role":"system","content":"fixed"}),
            json!({"role":"user","content":value}),
            json!({"role":"user","content":old}),
        ]));
    }
    let history_result = analyze(histories);
    assert!(
        causes(&history_result).contains(&FindingCause::NonAppendOnlyHistory),
        "causes={:?}, templates={}",
        causes(&history_result),
        history_result.templates.len()
    );

    let payloads = [
        "{\"alpha\":1,\"beta\":2}",
        "{ \"beta\": 2, \"alpha\": 1 }",
        "{\n  \"alpha\": 1, \"beta\": 2\n}",
        "{\"beta\":2,\"alpha\":1}",
    ]
    .into_iter()
    .map(|visible| {
        chat(vec![
            json!({"role":"system","content":visible}),
            json!({"role":"system","content":stable("after-json")}),
        ])
    })
    .collect();
    assert!(causes(&analyze(payloads)).contains(&FindingCause::ModelVisibleFormatDrift));
}

#[test]
fn suppresses_dynamic_suffix_non_context_distinct_template_and_cross_domain() {
    let suffix_only = ["question a", "问题乙", "third request", "delta"]
        .into_iter()
        .map(|question| {
            chat(vec![
                json!({"role":"system","content":stable("same-prefix")}),
                json!({"role":"user","content":question}),
            ])
        })
        .collect();
    assert!(analyze(suffix_only).findings.is_empty());

    let non_context = [0.1, 0.2, 0.3, 0.4]
        .into_iter()
        .map(|temperature| json!({"model":"model-a","temperature":temperature,"messages":[{"role":"system","content":stable("identical")}]}))
        .collect();
    assert!(analyze(non_context).findings.is_empty());

    let distinct = [
        "translate text",
        "review code",
        "answer legal questions",
        "compose music",
    ]
    .into_iter()
    .map(|role| chat(vec![json!({"role":"system","content":role})]))
    .collect();
    assert!(analyze(distinct).findings.is_empty());

    let cross_model = ["a", "b", "c", "d"]
        .into_iter()
        .map(|model| json!({"model":model,"messages":[{"role":"system","content":format!("variable {model} {}",stable("tail"))}]}))
        .collect();
    assert!(analyze(cross_model).findings.is_empty());
}

#[test]
fn findings_and_ranking_are_input_permutation_invariant() {
    let tail = stable("permutation-tail");
    let mut payloads: Vec<_> = ["north", "south", "east", "west"]
        .into_iter()
        .map(|value| {
            chat(vec![
                json!({"role":"system","content":value}),
                json!({"role":"system","content":tail}),
            ])
        })
        .collect();
    let first = analyze(payloads.clone());
    payloads.reverse();
    let second = analyze(payloads);
    assert_eq!(first.templates, second.templates);
    assert_eq!(first.findings, second.findings);
    assert_eq!(first.top_k, second.top_k);
}

#[test]
fn mixed_corpus_ranks_aggregated_roots_without_pair_amplification() {
    fn group(label: &str, member_count: usize, stable_bytes: usize) -> Vec<Value> {
        (0..member_count)
            .map(|index| {
                chat(vec![
                    json!({"role":"system","content":format!("{label}-variant-{index}")}),
                    json!({"role":"system","content":label.repeat(stable_bytes / label.len())}),
                ])
            })
            .collect()
    }

    let mut payloads = group("ALPHA", 6, 600);
    payloads.extend(group("BRAVO", 5, 360));
    payloads.extend(group("CHARLIE", 4, 180));
    let result = analyze(payloads);

    assert_eq!(result.findings.len(), 3, "findings={:?}", result.findings);
    assert_eq!(result.top_k.len(), 3);
    assert!(result.top_k[0].score.score > result.top_k[1].score.score);
    assert!(result.top_k[1].score.score > result.top_k[2].score.score);
    assert_eq!(result.findings[0].affected_count, 5);
    assert_eq!(result.findings[1].affected_count, 4);
    assert_eq!(result.findings[2].affected_count, 3);
}
