//! 本文件用通用合成语料验证八类诊断、负例抑制和 UTF-8 证据边界。

use super::*;

fn unit(role: UnitRole, content: impl Into<String>, index: usize) -> DiagnosticUnit {
    let content = content.into();
    DiagnosticUnit {
        role,
        source: SourceLocation {
            json_path: format!("$.visible[{index}]"),
            utf8_range: Some((0, content.len())),
        },
        content,
        stable_identity: None,
        ordinal: Some(index),
    }
}

fn tool(identity: &str, content: &str, index: usize) -> DiagnosticUnit {
    let mut result = unit(UnitRole::ToolDefinition, content, index);
    result.stable_identity = Some(identity.to_owned());
    result.ordinal = None;
    result
}

fn template(requests: Vec<Vec<DiagnosticUnit>>) -> DiagnosticTemplate {
    DiagnosticTemplate {
        template_id: "t".into(),
        members: requests
            .into_iter()
            .enumerate()
            .map(|(index, units)| DiagnosticRequest {
                request_id: format!("r{index}"),
                units,
            })
            .collect(),
        cohesion: 0.9,
        projection_reliability: 1.0,
    }
}

fn opts() -> DiagnosisOptions {
    DiagnosisOptions {
        min_stable_support: 3,
        stable_support_rate: 0.8,
        min_blocked_bytes: 20,
        min_anchor_bytes: 10,
        local_text_similarity: 0.7,
    }
}

fn stable() -> String {
    "a long stable instruction after the changing region".into()
}

fn assert_cause(cause: Cause, requests: Vec<Vec<DiagnosticUnit>>) {
    let findings = diagnose_template(&template(requests), &opts());
    assert!(!findings.is_empty(), "expected {cause:?}");
    assert!(findings.iter().all(|finding| finding.cause == cause));
}

#[test]
fn detects_early_volatile_content() {
    assert_cause(
        Cause::EarlyVolatileContent,
        (0..3)
            .map(|i| {
                vec![
                    unit(UnitRole::Other, format!("change-{i}"), 0),
                    unit(UnitRole::Other, stable(), 1),
                ]
            })
            .collect(),
    );
}

#[test]
fn detects_inline_slot_and_keeps_unicode_boundary() {
    let findings = diagnose_template(
        &template(
            (0..3)
                .map(|i| {
                    vec![unit(
                        UnitRole::Other,
                        format!("固定🙂变化{i}{}", stable()),
                        0,
                    )]
                })
                .collect(),
        ),
        &opts(),
    );
    assert!(findings.iter().all(|f| f.cause == Cause::InlineDynamicSlot));
    for finding in findings {
        let offset = finding.source.utf8_range.unwrap().0;
        let source = &format!("固定🙂变化1{}", stable());
        assert!(source.is_char_boundary(offset));
    }
}

#[test]
fn detects_dynamic_block_before_static() {
    let base = vec![
        unit(UnitRole::Other, stable(), 0),
        unit(UnitRole::Other, "tail", 1),
    ];
    let shifted = |value: &str| {
        vec![
            unit(UnitRole::Other, value, 9),
            unit(UnitRole::Other, stable(), 0),
            unit(UnitRole::Other, "tail", 1),
        ]
    };
    assert_cause(
        Cause::DynamicBlockBeforeStatic,
        vec![base, shifted("x"), shifted("y")],
    );
}

#[test]
fn detects_unique_tool_reorder() {
    let ordered = vec![
        tool("a", "definition a is stable and long", 0),
        tool("b", "definition b is stable and long", 1),
        unit(UnitRole::Other, stable(), 2),
    ];
    let reversed = vec![
        tool("b", "definition b is stable and long", 1),
        tool("a", "definition a is stable and long", 0),
        unit(UnitRole::Other, stable(), 2),
    ];
    assert_cause(
        Cause::ToolOrderDrift,
        vec![ordered, reversed.clone(), reversed],
    );
}

#[test]
fn suppresses_ambiguous_duplicate_tool_identity() {
    let left = vec![
        tool("same", "one", 0),
        tool("same", "two", 1),
        unit(UnitRole::Other, stable(), 2),
    ];
    let right = vec![
        tool("same", "two", 1),
        tool("same", "one", 0),
        unit(UnitRole::Other, stable(), 2),
    ];
    let findings = diagnose_template(&template(vec![left, right.clone(), right]), &opts());
    assert!(findings.iter().all(|f| f.cause != Cause::ToolOrderDrift));
}

#[test]
fn detects_tool_definition_drift() {
    let make = |suffix: &str| {
        vec![
            tool(
                "unique",
                &format!("a mostly stable tool definition {suffix}"),
                0,
            ),
            unit(UnitRole::Other, stable(), 1),
        ]
    };
    assert_cause(
        Cause::ToolDefinitionDrift,
        vec![make("x"), make("y"), make("z")],
    );
}

#[test]
fn detects_prompt_micro_drift() {
    let make = |suffix: &str| {
        vec![
            unit(
                UnitRole::SystemPrompt,
                format!("a mostly stable system prompt {suffix}"),
                0,
            ),
            unit(UnitRole::Other, stable(), 1),
        ]
    };
    assert_cause(
        Cause::PromptMicroDrift,
        vec![make("x"), make("y"), make("z")],
    );
}

#[test]
fn detects_non_append_history() {
    let make = |value: &str| {
        vec![
            unit(UnitRole::Conversation, value, 0),
            unit(UnitRole::Conversation, stable(), 1),
        ]
    };
    assert_cause(
        Cause::NonAppendHistory,
        vec![make("old"), make("replaced-a"), make("replaced-b")],
    );
}

#[test]
fn detects_only_model_visible_json_formatting() {
    let make = |json: &str| {
        vec![
            unit(UnitRole::ModelVisibleText, json, 0),
            unit(UnitRole::Other, stable(), 1),
        ]
    };
    assert_cause(
        Cause::ModelVisibleJsonFormattingDrift,
        vec![
            make("{\"a\":1,\"b\":2}"),
            make("{ \"b\": 2, \"a\": 1 }"),
            make("{\n\"a\": 1, \"b\": 2}"),
        ],
    );

    let outer = |json: &str| {
        vec![
            unit(UnitRole::Other, json, 0),
            unit(UnitRole::Other, stable(), 1),
        ]
    };
    let findings = diagnose_template(
        &template(vec![
            outer("{\"a\":1}"),
            outer("{ \"a\": 1 }"),
            outer("{\n\"a\":1}"),
        ]),
        &opts(),
    );
    assert!(findings
        .iter()
        .all(|f| f.cause != Cause::ModelVisibleJsonFormattingDrift));

    let array_order = |json: &str| {
        vec![
            unit(UnitRole::ModelVisibleText, json, 0),
            unit(UnitRole::Other, stable(), 1),
        ]
    };
    let findings = diagnose_template(
        &template(vec![
            array_order("[1,2]"),
            array_order("[2,1]"),
            array_order("[2,1]"),
        ]),
        &opts(),
    );
    assert!(findings
        .iter()
        .all(|f| f.cause != Cause::ModelVisibleJsonFormattingDrift));
}

#[test]
fn suppresses_expected_dynamic_suffix_and_real_template_difference() {
    let suffix = template(vec![
        vec![
            unit(UnitRole::Other, stable(), 0),
            unit(UnitRole::Other, "question a", 1),
        ],
        vec![
            unit(UnitRole::Other, stable(), 0),
            unit(UnitRole::Other, "question b", 1),
        ],
        vec![
            unit(UnitRole::Other, stable(), 0),
            unit(UnitRole::Other, "question c", 1),
        ],
    ]);
    assert!(diagnose_template(&suffix, &opts()).is_empty());

    let distinct = template(vec![
        vec![unit(UnitRole::SystemPrompt, "entirely alpha", 0)],
        vec![unit(UnitRole::SystemPrompt, "completely beta", 0)],
        vec![unit(UnitRole::SystemPrompt, "unrelated gamma", 0)],
    ]);
    assert!(diagnose_template(&distinct, &opts()).is_empty());
}

#[test]
fn enforces_stable_support_count_and_rate() {
    let requests = vec![
        vec![
            unit(UnitRole::Other, "variant-a", 0),
            unit(UnitRole::Other, stable(), 1),
        ],
        vec![
            unit(UnitRole::Other, "variant-b", 0),
            unit(UnitRole::Other, stable(), 1),
        ],
        vec![unit(UnitRole::Other, "unrelated suffix", 0)],
        vec![unit(UnitRole::Other, "another unrelated suffix", 0)],
    ];
    assert!(diagnose_template(&template(requests), &opts()).is_empty());
}

#[test]
fn identical_baseline_members_count_toward_stable_support() {
    let baseline = vec![
        unit(UnitRole::Other, "baseline", 0),
        unit(UnitRole::Other, stable(), 1),
    ];
    let variant = |value: &str| {
        vec![
            unit(UnitRole::Other, value, 0),
            unit(UnitRole::Other, stable(), 1),
        ]
    };
    let findings = diagnose_template(
        &template(vec![
            baseline.clone(),
            baseline.clone(),
            baseline,
            variant("variant-a"),
            variant("variant-b"),
        ]),
        &opts(),
    );
    assert_eq!(findings.len(), 2);
}

#[test]
fn diagnosis_is_member_order_invariant_and_metrics_are_conservative() {
    let requests: Vec<_> = (0..3)
        .map(|index| DiagnosticRequest {
            request_id: format!("stable-id-{index}"),
            units: vec![
                unit(UnitRole::Other, format!("vary-{index}"), 0),
                unit(UnitRole::Other, stable(), 1),
            ],
        })
        .collect();
    let build = |members: Vec<DiagnosticRequest>| DiagnosticTemplate {
        template_id: "t".into(),
        members,
        cohesion: 1.0,
        projection_reliability: 1.0,
    };
    let mut reversed = requests.clone();
    reversed.reverse();
    let mut forward = diagnose_template(&build(requests), &opts());
    let mut backward = diagnose_template(&build(reversed), &opts());
    forward.sort_by_key(|finding| finding.member_id.clone());
    backward.sort_by_key(|finding| finding.member_id.clone());
    assert_eq!(forward, backward);
    assert!(forward.iter().all(|finding| {
        finding.potential_prefix_bytes == finding.actual_prefix_bytes + finding.blocked_stable_bytes
    }));
}

#[test]
fn converts_projected_units_without_exposing_outer_json() {
    use std::ops::Range;

    use crate::model::projection::{
        CacheSequence, CacheUnit, CacheUnitKind, ComparisonDomain,
        SourceLocation as ProjectionSource,
    };

    let sequence = CacheSequence {
        request_id: "request".into(),
        domain: ComparisonDomain {
            dialect: "dialect".into(),
            model: "model".into(),
            adapter_revision: "revision".into(),
        },
        units: vec![CacheUnit {
            kind: CacheUnitKind::VisibleText,
            alignment_key: "message.content".into(),
            content: "你好".into(),
            source: ProjectionSource {
                json_path: "$.messages[0].content".into(),
                utf8_bytes: Some(Range { start: 0, end: 6 }),
                message_index: Some(0),
                role: Some("system".into()),
            },
            tool_identity: None,
        }],
        projection_reliability_millis: 1000,
    };
    let converted = DiagnosticRequest::from(&sequence);
    assert_eq!(converted.units[0].role, UnitRole::SystemPrompt);
    assert_eq!(converted.units[0].source.utf8_range, Some((0, 6)));
    assert_eq!(
        converted.units[0].stable_identity.as_deref(),
        Some("message.content")
    );
}

#[test]
fn cross_role_history_insertion_is_one_root_with_real_stable_anchor() {
    let old = stable();
    let baseline = vec![
        unit(UnitRole::Conversation, "user", 0),
        unit(UnitRole::Conversation, old.clone(), 0),
    ];
    let inserted = |value: &str| {
        vec![
            unit(UnitRole::Conversation, "assistant", 0),
            unit(UnitRole::Conversation, value, 0),
            unit(UnitRole::Conversation, "user", 1),
            unit(UnitRole::Conversation, old.clone(), 1),
        ]
    };
    let observations = diagnose_template(
        &template(vec![
            baseline,
            inserted("new value alpha"),
            inserted("新增内容乙"),
            inserted("third inserted value"),
        ]),
        &opts(),
    );
    assert_eq!(observations.len(), 3);
    assert!(observations
        .iter()
        .all(|item| item.cause == Cause::NonAppendHistory));
    assert!(observations
        .iter()
        .all(|item| item.stable_anchor == old.chars().take(48).collect::<String>()));
    let ranked = crate::ranking::rank_findings(&observations, 10);
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].affected, 3);
}

#[test]
fn equal_length_non_conversation_reorder_is_not_dynamic_insertion() {
    let first = "A".repeat(80);
    let second = "B".repeat(80);
    let ordered = vec![
        unit(UnitRole::SystemPrompt, first.clone(), 0),
        unit(UnitRole::SystemPrompt, second.clone(), 1),
    ];
    let reversed = vec![
        unit(UnitRole::SystemPrompt, second, 1),
        unit(UnitRole::SystemPrompt, first, 0),
    ];
    let findings = diagnose_template(
        &template(vec![ordered.clone(), ordered, reversed.clone(), reversed]),
        &opts(),
    );
    assert!(findings
        .iter()
        .all(|item| item.cause != Cause::DynamicBlockBeforeStatic));
}
