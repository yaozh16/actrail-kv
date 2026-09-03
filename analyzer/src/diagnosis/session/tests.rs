//! 本文件验证 Session 开放前缀、时间线边界与聚合行为。

use actrail_kv_artifacts::{SessionHistorySiteKind, SessionTransitionOutcome};

use crate::model::projection::{
    CacheSequence, CacheUnit, CacheUnitKind, ComparisonDomain, ContextCollectionKind,
    HierarchyLocation,
};

use super::{analyze_session, prefix, SessionObservation};

#[test]
fn terminal_marker_does_not_turn_message_append_into_history_change() {
    let previous = sequence("a", 0, &["history", "messages:end"]);
    let current = sequence("b", 0, &["history", "dynamic result", "messages:end"]);
    let outcome = prefix::compare(&previous, &current);
    let SessionTransitionOutcome::NormalAppend { metrics } = outcome else {
        panic!("expected normal append")
    };
    assert_eq!(metrics.preserved_prefix_bytes, "history".len());
    assert_eq!(metrics.invalidated_previous_suffix_bytes, 0);
}

#[test]
fn marker_like_non_structural_unit_remains_observable() {
    let mut previous = sequence("a", 0, &["custom:end"]);
    previous.units[0].kind = CacheUnitKind::VisibleText;
    let mut current = sequence("b", 0, &["custom:end"]);
    current.units[0].kind = CacheUnitKind::VisibleText;
    current.units[0].content = "changed".to_owned();
    assert!(matches!(
        prefix::compare(&previous, &current),
        SessionTransitionOutcome::HistoryChanged { .. }
    ));
}

#[test]
fn changed_unicode_content_uses_a_valid_utf8_frontier() {
    let previous = sequence("a", 0, &["稳定甲"]);
    let current = sequence("b", 0, &["稳定乙"]);
    let SessionTransitionOutcome::HistoryChanged {
        metrics,
        divergence,
    } = prefix::compare(&previous, &current)
    else {
        panic!("expected history change")
    };
    assert_eq!(metrics.preserved_prefix_bytes, "稳定".len());
    assert_eq!(
        divergence.previous_source.unwrap().byte_start,
        Some("稳定".len())
    );
}

#[test]
fn current_request_that_ends_inside_an_old_unit_is_prefix_truncated() {
    let previous = sequence("a", 0, &["stable and old suffix"]);
    let current = sequence("b", 0, &["stable"]);
    let SessionTransitionOutcome::PrefixTruncated { metrics, .. } =
        prefix::compare(&previous, &current)
    else {
        panic!("expected prefix truncation")
    };
    assert_eq!(metrics.preserved_prefix_bytes, "stable".len());
    assert_eq!(
        metrics.invalidated_previous_suffix_bytes,
        " and old suffix".len()
    );
}

#[test]
fn empty_unit_with_a_different_shape_is_not_prefix_truncation() {
    let previous = sequence("a", 0, &["old"]);
    let mut current = sequence("b", 0, &[""]);
    current.units[0].kind = CacheUnitKind::Role;
    current.units[0].alignment_key = "message:role".to_owned();
    assert!(matches!(
        prefix::compare(&previous, &current),
        SessionTransitionOutcome::HistoryChanged { .. }
    ));
}

#[test]
fn timeline_crosses_comparison_windows_and_aggregates_repeated_history_changes() {
    let mut sequences = Vec::new();
    let observations = vec![
        observation(
            "a",
            "2026-09-03T00:59:00Z",
            1,
            sequence("a", 0, &["state-a", "tail"]),
            &mut sequences,
        ),
        observation(
            "b",
            "2026-09-03T01:01:00Z",
            2,
            sequence("b", 3600, &["state-b", "tail"]),
            &mut sequences,
        ),
        observation(
            "c",
            "2026-09-03T02:01:00Z",
            3,
            sequence("c", 7200, &["state-c", "tail"]),
            &mut sequences,
        ),
    ];
    let analysis = analyze_session(observations, &sequences);
    assert_eq!(analysis.timelines.len(), 1);
    assert_eq!(analysis.timelines[0].transitions.len(), 2);
    assert!(analysis.timelines[0]
        .transitions
        .iter()
        .all(|transition| matches!(
            transition.outcome,
            SessionTransitionOutcome::HistoryChanged { .. }
        )));
    assert_eq!(analysis.history_sites.len(), 1);
    assert_eq!(analysis.history_sites[0].occurrence_count, 2);
    assert!(analysis.history_sites[0].insight.is_some());
}

#[test]
fn equal_timestamps_are_never_presented_as_certain_order() {
    let mut sequences = Vec::new();
    let observations = vec![
        observation(
            "a",
            "2026-09-03T00:00:00Z",
            1,
            sequence("a", 0, &["a"]),
            &mut sequences,
        ),
        observation(
            "b",
            "2026-09-03T00:00:00Z",
            2,
            sequence("b", 0, &["b"]),
            &mut sequences,
        ),
    ];
    let analysis = analyze_session(observations, &sequences);
    assert!(matches!(
        analysis.timelines[0].transitions[0].outcome,
        SessionTransitionOutcome::AmbiguousOrder { .. }
    ));
    assert!(analysis.history_sites.is_empty());
}

#[test]
fn known_model_change_is_an_incomparable_boundary() {
    let mut sequences = Vec::new();
    let previous = sequence("a", 0, &["same"]);
    let mut current = sequence("b", 0, &["same"]);
    current.domain.model = "other-model".to_owned();
    let observations = vec![
        observation("a", "2026-09-03T00:00:00Z", 1, previous, &mut sequences),
        observation("b", "2026-09-03T00:00:01Z", 2, current, &mut sequences),
    ];
    let analysis = analyze_session(observations, &sequences);
    assert!(matches!(
        analysis.timelines[0].transitions[0].outcome,
        SessionTransitionOutcome::IncomparableBoundary { .. }
    ));
    assert!(analysis.history_sites.is_empty());
}

#[test]
fn invalid_timestamp_breaks_the_entire_unknown_chronology_without_cross_gap_linking() {
    let mut sequences = Vec::new();
    let observations = vec![
        observation(
            "a",
            "2026-09-03T00:00:00Z",
            1,
            sequence("a", 0, &["a"]),
            &mut sequences,
        ),
        SessionObservation {
            session_id: "session-a".to_owned(),
            request_id: "invalid".to_owned(),
            captured_at: Some("not-a-time".to_owned()),
            input_line: 2,
            source: None,
            sequence_index: None,
        },
        observation(
            "c",
            "2026-09-03T00:00:02Z",
            3,
            sequence("c", 0, &["c"]),
            &mut sequences,
        ),
    ];
    let analysis = analyze_session(observations, &sequences);
    let timeline = &analysis.timelines[0];
    assert_eq!(timeline.requests[1].captured_at, None);
    assert_eq!(timeline.transitions[0].current_captured_at, None);
    assert_eq!(timeline.transitions[1].previous_captured_at, None);
    assert!(timeline.transitions.iter().all(|transition| matches!(
        transition.outcome,
        SessionTransitionOutcome::UnanalyzableBoundary { .. }
    )));
    assert!(analysis.history_sites.is_empty());
}

#[test]
fn history_sites_never_merge_the_same_position_across_models() {
    let mut sequences = Vec::new();
    let mut a1 = observation(
        "a1",
        "2026-09-03T00:00:00Z",
        1,
        sequence("a1", 0, &["old-a"]),
        &mut sequences,
    );
    let mut a2 = observation(
        "a2",
        "2026-09-03T00:00:01Z",
        2,
        sequence("a2", 0, &["new-a"]),
        &mut sequences,
    );
    a1.session_id = "session-a".to_owned();
    a2.session_id = "session-a".to_owned();
    let mut model_b_old = sequence("b1", 0, &["old-b"]);
    model_b_old.domain.model = "model-b".to_owned();
    let mut model_b_new = sequence("b2", 0, &["new-b"]);
    model_b_new.domain.model = "model-b".to_owned();
    let mut b1 = observation("b1", "2026-09-03T00:00:00Z", 3, model_b_old, &mut sequences);
    let mut b2 = observation("b2", "2026-09-03T00:00:01Z", 4, model_b_new, &mut sequences);
    b1.session_id = "session-b".to_owned();
    b2.session_id = "session-b".to_owned();
    let analysis = analyze_session(vec![a1, a2, b1, b2], &sequences);
    assert_eq!(analysis.history_sites.len(), 2);
    assert!(analysis
        .history_sites
        .iter()
        .all(|site| site.occurrence_count == 1 && site.insight.is_none()));
}

#[test]
fn repeated_truncation_uses_a_truncation_specific_insight() {
    let mut sequences = Vec::new();
    let observations = vec![
        observation(
            "a",
            "2026-09-03T00:00:00Z",
            1,
            sequence("a", 0, &["stable-long-tail"]),
            &mut sequences,
        ),
        observation(
            "b",
            "2026-09-03T00:00:01Z",
            2,
            sequence("b", 0, &["stable-long"]),
            &mut sequences,
        ),
        observation(
            "c",
            "2026-09-03T00:00:02Z",
            3,
            sequence("c", 0, &["stable"]),
            &mut sequences,
        ),
    ];
    let analysis = analyze_session(observations, &sequences);
    assert_eq!(analysis.history_sites.len(), 1);
    let site = &analysis.history_sites[0];
    assert_eq!(site.kind, SessionHistorySiteKind::PrefixTruncated);
    let insight = site.insight.as_ref().expect("repeated site has insight");
    assert!(!insight.summary.contains("追加"));
    assert!(!insight
        .detail
        .as_deref()
        .unwrap_or_default()
        .contains("追加"));
}

fn observation(
    request_id: &str,
    captured_at: &str,
    input_line: usize,
    projection: CacheSequence,
    sequences: &mut Vec<CacheSequence>,
) -> SessionObservation {
    let sequence_index = sequences.len();
    sequences.push(projection);
    SessionObservation {
        session_id: "session-a".to_owned(),
        request_id: request_id.to_owned(),
        captured_at: Some(captured_at.to_owned()),
        input_line,
        source: None,
        sequence_index: Some(sequence_index),
    }
}

fn sequence(request_id: &str, time_window_key: i64, contents: &[&str]) -> CacheSequence {
    CacheSequence {
        request_id: request_id.to_owned(),
        domain: ComparisonDomain {
            time_window_key,
            endpoint_key: "endpoint".to_owned(),
            model: "model".to_owned(),
            model_deployment_key: None,
            context_schema_key: "openai-compatible-chat/v1".to_owned(),
            agent_key: None,
            kv_namespace: None,
            dialect: "openai-compatible-chat".to_owned(),
            adapter_revision: "1".to_owned(),
        },
        units: contents
            .iter()
            .enumerate()
            .map(|(index, content)| {
                let terminal = *content == "messages:end";
                let source_path = if terminal {
                    "$.messages".to_owned()
                } else {
                    format!("$.messages[{index}].content")
                };
                CacheUnit {
                    kind: if terminal {
                        CacheUnitKind::Structural
                    } else {
                        CacheUnitKind::VisibleText
                    },
                    alignment_key: if terminal || content.ends_with(":end") {
                        (*content).to_owned()
                    } else {
                        "message:user:content".to_owned()
                    },
                    content: (*content).to_owned(),
                    source: prefix::source_location(&source_path),
                    tool_identity: None,
                    hierarchy: HierarchyLocation {
                        collection: ContextCollectionKind::Messages,
                        parent_json_path: "$.messages".to_owned(),
                        element_index: (!terminal).then_some(index),
                        content_block_index: None,
                    },
                }
            })
            .collect(),
        projection_reliability_millis: 1000,
    }
}
