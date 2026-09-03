//! 用 examples/cases 下真实语料做回归：默认阈值应检出对应缺陷，session 诊断非空。

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use actrail_kv_analyzer::run::{analyze_reader, AnalysisOptions};
use actrail_kv_artifacts::{AnalysisResult, DefectFactKind, SessionTransitionOutcome};

fn fixture(name: &str) -> AnalysisResult {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/cases")
        .join(format!("{name}.ndjson"));
    analyze_reader(
        BufReader::new(File::open(path).expect("fixture exists")),
        AnalysisOptions::default(),
    )
    .expect("analysis succeeds")
}

fn has_fact(result: &AnalysisResult, kind: DefectFactKind) -> bool {
    result
        .defects
        .iter()
        .flat_map(|defect| &defect.mismatch.facts)
        .any(|fact| fact.kind == kind)
}

#[test]
fn workspace_fixture_detects_content_variation() {
    let result = fixture("workspace");
    assert!(!result.defects.is_empty(), "result={result:#?}");
    assert!(has_fact(&result, DefectFactKind::ContentVariation));
}

#[test]
fn policy_fixture_detects_fixed_variants() {
    let result = fixture("policy");
    assert!(!result.defects.is_empty(), "result={result:#?}");
    assert!(has_fact(&result, DefectFactKind::FixedVariants));
    assert!(!has_fact(&result, DefectFactKind::ContentVariation));
}

#[test]
fn json_fixture_reports_content_defect() {
    let result = fixture("json");
    assert!(!result.defects.is_empty(), "result={result:#?}");
    assert!(has_fact(&result, DefectFactKind::StructuredDataEquivalent));
}

#[test]
fn insert_fixture_reports_a_defect() {
    let result = fixture("insert");
    assert!(!result.defects.is_empty(), "result={result:#?}");
    assert!(has_fact(&result, DefectFactKind::InsertionDeletion));
}

#[test]
fn natural_fixture_analyzes_without_regression() {
    let result = fixture("natural");
    assert_eq!(result.run.analyzed_records, 4);
    assert!(result.defects.is_empty(), "result={result:#?}");
    assert!(result.conditional_local_sites.is_empty());
}

#[test]
fn session_fixture_reports_current_timeline_evidence() {
    let result = fixture("session");
    assert_eq!(result.session_analysis.session_record_count, 3);
    assert_eq!(result.session_analysis.timelines.len(), 1);
    let timeline = &result.session_analysis.timelines[0];
    assert_eq!(timeline.session_id, "fixture-session");
    assert_eq!(timeline.requests.len(), 3);
    assert_eq!(timeline.transitions.len(), 2);
    for (index, transition) in timeline.transitions.iter().enumerate() {
        assert!(!transition.id.is_empty());
        assert_eq!(
            transition.previous_request_id,
            timeline.requests[index].request_id
        );
        assert_eq!(
            transition.current_request_id,
            timeline.requests[index + 1].request_id
        );
    }
    let SessionTransitionOutcome::NormalAppend { metrics } = &timeline.transitions[0].outcome
    else {
        panic!("first transition must be a normal append");
    };
    assert_eq!(
        metrics.preserved_prefix_bytes,
        metrics.previous_observable_bytes
    );
    let SessionTransitionOutcome::HistoryChanged { metrics, .. } = &timeline.transitions[1].outcome
    else {
        panic!("second transition must be a history change");
    };
    assert!(metrics.preserved_prefix_bytes < metrics.previous_observable_bytes);
    assert!(metrics.invalidated_previous_suffix_bytes > 0);
}
