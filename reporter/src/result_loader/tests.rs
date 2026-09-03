//! 本文件构造完整 analysis fixture 并覆盖 Reporter 对新增 artifact 契约的拒绝路径。

use std::{fs, path::Path};

use actrail_kv_artifacts::*;

use super::load_result;

#[test]
fn accepts_valid_v011_result() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("analysis.json");
    write(&path, &fixture());
    assert!(load_result(&path).is_ok());
}

#[test]
fn rejects_wrong_schema_and_broken_top_k() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("analysis.json");
    let mut result = fixture();
    result.run.schema_version = "legacy".into();
    write(&path, &result);
    assert!(load_result(&path).is_err());

    let mut result = fixture();
    result.top_k = vec!["unknown".into()];
    write(&path, &result);
    assert!(load_result(&path).is_err());
}

#[test]
fn rejects_inconsistent_conditional_site() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("analysis.json");
    let mut result = fixture();
    result.conditional_local_sites[0].episode_index = 1;
    write(&path, &result);
    assert!(load_result(&path).is_err());

    let mut result = fixture();
    result.conditional_local_sites[0].affected_count += 1;
    write(&path, &result);
    assert!(load_result(&path).is_err());
}

#[test]
fn rejects_conditional_episode_index_gaps() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("analysis.json");

    let mut starts_at_three = fixture();
    starts_at_three.conditional_local_sites[0].episode_index = 3;
    write(&path, &starts_at_three);
    assert!(load_result(&path).is_err());

    let mut skips_three = fixture();
    let mut fourth = skips_three.conditional_local_sites[0].clone();
    fourth.id = "conditional-four".into();
    fourth.episode_index = 4;
    skips_three.conditional_local_sites.push(fourth);
    write(&path, &skips_three);
    assert!(load_result(&path).is_err());
}

#[test]
fn accepts_contiguous_conditional_episode_indices() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("analysis.json");
    let mut result = fixture();
    let mut third = result.conditional_local_sites[0].clone();
    third.id = "conditional-three".into();
    third.episode_index = 3;
    result.conditional_local_sites.push(third);
    write(&path, &result);
    assert!(load_result(&path).is_ok());
}

#[test]
fn rejects_broken_session_metrics_and_references() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("analysis.json");
    let mut result = fixture();
    let SessionTransitionOutcome::HistoryChanged { metrics, .. } =
        &mut result.session_analysis.timelines[0].transitions[0].outcome
    else {
        panic!("fixture outcome");
    };
    metrics.invalidated_previous_suffix_bytes += 1;
    write(&path, &result);
    assert!(load_result(&path).is_err());

    let mut result = fixture();
    result.session_analysis.history_sites[0].transition_ids[0] = "unknown".into();
    write(&path, &result);
    assert!(load_result(&path).is_err());
}

#[test]
fn rejects_history_site_kind_or_boundary_mismatch() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("analysis.json");

    let mut wrong_kind = fixture();
    wrong_kind.session_analysis.history_sites[0].kind = SessionHistorySiteKind::PrefixTruncated;
    write(&path, &wrong_kind);
    assert!(load_result(&path).is_err());

    let mut wrong_boundary = fixture();
    wrong_boundary.session_analysis.history_sites[0]
        .boundary
        .model = "other-model".into();
    write(&path, &wrong_boundary);
    assert!(load_result(&path).is_err());

    let mut missing_boundary = fixture();
    missing_boundary.session_analysis.timelines[0].transitions[0].boundary = None;
    write(&path, &missing_boundary);
    assert!(load_result(&path).is_err());
}

#[test]
fn accepts_unanalyzable_boundary_for_invalid_capture_time() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("analysis.json");
    let mut result = fixture();
    let timeline = &mut result.session_analysis.timelines[0];
    timeline.requests[0].captured_at = Some("not-a-time".into());
    timeline.transitions[0].previous_captured_at = Some("not-a-time".into());
    timeline.transitions[0].outcome = SessionTransitionOutcome::UnanalyzableBoundary {
        reason: "captured_at is missing or not RFC 3339".into(),
    };
    timeline.transitions[0].boundary = None;
    result.session_analysis.history_sites.clear();
    write(&path, &result);
    assert!(load_result(&path).is_ok());
}

#[test]
fn rejects_overlapping_variant_members_and_inconsistent_score() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("analysis.json");
    let mut result = fixture();
    result.defects[0].mismatch.variants[1]
        .member_request_ids
        .push("a".into());
    write(&path, &result);
    assert!(load_result(&path).is_err());

    let mut result = fixture();
    result.defects[0].score.score += 1.0;
    write(&path, &result);
    assert!(load_result(&path).is_err());
}

fn write(path: &Path, result: &AnalysisResult) {
    fs::write(path, serde_json::to_vec(result).expect("serialize")).expect("write fixture");
}

pub(crate) fn fixture() -> AnalysisResult {
    let group = group();
    let source = source();
    let mismatch = mismatch(&source);
    AnalysisResult {
        run: AnalysisRunSummary {
            schema_version: ANALYSIS_SCHEMA_VERSION.into(),
            input_records: 4,
            analyzed_records: 4,
            skipped_records: vec![],
            options: options(),
            limitations: vec![],
        },
        templates: vec![RequestTemplate {
            id: "template".into(),
            comparison_group: group.clone(),
            member_request_ids: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            medoid_request_id: "a".into(),
            cohesion: 0.9,
            stable_spans: vec![],
            slots: vec![],
        }],
        defects: vec![ContextDefect {
            id: "defect".into(),
            comparison_group: group.clone(),
            template_id: "template".into(),
            mismatch: mismatch.clone(),
            recovered_stable: recovered(&source),
            actual_prefix_bytes: 10,
            potential_prefix_bytes: 110,
            blocked_stable_bytes: 100,
            comparable_count: 4,
            affected_count: 2,
            confidence: 0.9,
            score: ScoreBreakdown {
                blocked_stable_bytes: 100,
                affected_count: 2,
                confidence: 0.9,
                score: 180.0,
            },
            insights: insights(),
        }],
        conditional_local_sites: vec![ConditionalLocalSite {
            id: "conditional".into(),
            comparison_group: group,
            template_id: "template".into(),
            episode_index: 2,
            mismatch,
            recovered_stable: recovered(&source),
            local_prefix_bytes: 40,
            blocked_stable_bytes: 100,
            comparable_count: 4,
            affected_count: 2,
            confidence: 0.9,
            insights: insights(),
        }],
        session_analysis: session_analysis(&source),
        top_k: vec!["defect".into()],
    }
}

fn group() -> ComparisonGroup {
    ComparisonGroup {
        time_window_key: "window".into(),
        endpoint_key: "endpoint".into(),
        model: "model".into(),
        context_schema_key: "schema".into(),
        agent_key: None,
        model_deployment_key: None,
        kv_namespace: None,
    }
}

fn source() -> SourceLocation {
    SourceLocation {
        json_path: "$.messages[0].content".into(),
        logical_scope: vec!["messages".into()],
        unit_index: Some(0),
        byte_start: Some(0),
        byte_end: Some(8),
        role: Some("system".into()),
    }
}

fn mismatch(source: &SourceLocation) -> MismatchRegion {
    MismatchRegion {
        pattern: MismatchPattern::ValueMismatch,
        variants: vec![
            variant("one", &["a", "b"], "a", "<script>alert(1)</script>", source),
            variant("two", &["c", "d"], "c", "south", source),
        ],
        facts: vec![DefectFact {
            kind: DefectFactKind::ContentVariation,
            detail: Some("<img src=x onerror=alert(1)>".into()),
        }],
    }
}

fn recovered(source: &SourceLocation) -> RecoveredStable {
    RecoveredStable {
        sources: vec![source.clone()],
        utf8_bytes: 100,
        support_count: 4,
        excerpt: "<script>stable()</script>".into(),
    }
}

fn insights() -> Vec<OptimizationInsight> {
    vec![OptimizationInsight {
        summary: "<b>统一生成方式</b>".into(),
        detail: Some("move & review".into()),
    }]
}

fn session_analysis(source: &SourceLocation) -> SessionAnalysis {
    let metrics = SessionPrefixMetrics {
        previous_observable_bytes: 100,
        current_observable_bytes: 120,
        preserved_prefix_bytes: 40,
        prefix_retention_ratio: 0.4,
        invalidated_previous_suffix_bytes: 60,
    };
    SessionAnalysis {
        session_record_count: 2,
        timelines: vec![SessionTimeline {
            session_id: "<session&one>".into(),
            requests: vec![
                session_request("request-one", "2026-09-03T06:00:00Z", 1),
                session_request("request-two", "2026-09-03T06:01:00Z", 2),
            ],
            transitions: vec![SessionTransition {
                id: "transition-one".into(),
                previous_request_id: "request-one".into(),
                current_request_id: "request-two".into(),
                previous_captured_at: Some("2026-09-03T06:00:00Z".into()),
                current_captured_at: Some("2026-09-03T06:01:00Z".into()),
                boundary: Some(session_boundary()),
                outcome: SessionTransitionOutcome::HistoryChanged {
                    metrics,
                    divergence: SessionDivergence {
                        previous_source: Some(source.clone()),
                        current_source: Some(source.clone()),
                        previous_excerpt: Some("<old-history>".into()),
                        current_excerpt: Some("<new-history>".into()),
                        logical_position: "<messages/system>".into(),
                    },
                },
            }],
        }],
        history_sites: vec![SessionHistorySite {
            id: "history-site-one".into(),
            kind: SessionHistorySiteKind::HistoryChanged,
            boundary: session_boundary(),
            logical_position: "<messages/system>".into(),
            transition_ids: vec!["transition-one".into()],
            occurrence_count: 1,
            affected_session_count: 1,
            invalidated_previous_suffix_bytes_total: 60,
            invalidated_previous_suffix_bytes_min: 60,
            invalidated_previous_suffix_bytes_max: 60,
            insight: None,
        }],
    }
}

fn session_boundary() -> SessionBoundaryContext {
    SessionBoundaryContext {
        endpoint_key: "<session-endpoint>".into(),
        model: "session-model".into(),
        context_schema_key: "openai-compatible-chat/v1".into(),
        agent_key: Some("session-agent".into()),
        model_deployment_key: None,
        kv_namespace: None,
        dialect: "openai-compatible-chat".into(),
        adapter_revision: "v1".into(),
    }
}

fn session_request(
    request_id: &str,
    captured_at: &str,
    input_line: usize,
) -> SessionRequestReference {
    SessionRequestReference {
        request_id: request_id.into(),
        captured_at: Some(captured_at.into()),
        input_line,
        source: Some("<source>".into()),
    }
}

fn variant(
    fingerprint: &str,
    members: &[&str],
    representative: &str,
    excerpt: &str,
    source: &SourceLocation,
) -> MismatchVariant {
    MismatchVariant {
        fingerprint: fingerprint.into(),
        member_request_ids: members.iter().map(|member| (*member).into()).collect(),
        representative: VariantEvidence {
            request_id: representative.into(),
            sources: vec![source.clone()],
            utf8_bytes: excerpt.len(),
            excerpt: Some(excerpt.into()),
        },
    }
}

fn options() -> AnalysisOptionsSnapshot {
    AnalysisOptionsSnapshot {
        top_k: 1,
        comparison_window_seconds: 3_600,
        min_template_members: 3,
        stable_span_support_ratio: 0.8,
        min_stable_support: 3,
        min_blocked_stable_bytes: 64,
        min_exact_anchor_bytes: 24,
        text_similarity_threshold: 0.8,
        template_compatibility_threshold: 0.68,
        max_dynamic_coverage_ratio: 0.35,
        max_candidates_per_request: 128,
        max_projection_units: 512,
        max_text_unit_bytes: 1_048_576,
        max_payload_bytes: 8_388_608,
        max_alignment_cells: 2_000_000,
        max_total_alignment_cells: 64_000_000,
        max_records: 1_000_000,
    }
}
