//! 本文件公开 analysis.json 加载与安全静态 HTML 报告生成入口。

mod html_report_renderer;
mod result_loader;

use std::path::Path;

use anyhow::Result;

pub use html_report_renderer::render_html;
pub use result_loader::load_result;

/// 从 analysis.json 生成一个原子写入的静态 HTML 报告。
pub fn write_report(input: &Path, output: &Path) -> Result<()> {
    ensure_distinct_paths(input, output)?;
    let result = load_result(input)?;
    let html = render_html(&result)?;
    write_atomic(output, html.as_bytes())
}

fn write_atomic(output: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;

    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file_mut().sync_all()?;
    temporary.persist(output)?;
    sync_parent(parent)?;
    Ok(())
}

fn ensure_distinct_paths(input: &Path, output: &Path) -> Result<()> {
    if output.exists() && same_file::is_same_file(input, output).unwrap_or(false) {
        anyhow::bail!("input and output must refer to different files");
    }
    if !output.exists() {
        let input = input.canonicalize()?;
        let parent = output
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .canonicalize()?;
        let destination = match output.file_name() {
            Some(name) => parent.join(name),
            None => parent,
        };
        if input == destination {
            anyhow::bail!("input and output must refer to different files");
        }
    }
    Ok(())
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> Result<()> {
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use actrail_kv_artifacts::{
        AnalysisOptionsSnapshot, AnalysisResult, AnalysisRunSummary, Evidence, Finding,
        FindingCause, RequestTemplate, ScoreBreakdown, SourceLocation, TopKEntry,
    };

    use super::*;

    #[test]
    fn report_escapes_untrusted_evidence() {
        let directory = tempfile::tempdir().expect("tempdir");
        let input = directory.path().join("analysis.json");
        let output = directory.path().join("report.html");
        let score = ScoreBreakdown {
            blocked_stable_bytes: 100,
            affected_count: 2,
            confidence: 0.9,
            score: 180.0,
        };
        let result = AnalysisResult {
            run: AnalysisRunSummary {
                schema_version: "0.1.0".into(),
                input_records: 3,
                analyzed_records: 3,
                skipped_records: vec![],
                options: AnalysisOptionsSnapshot {
                    top_k: 1,
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
                },
                limitations: vec!["proxy only".into()],
            },
            templates: vec![RequestTemplate {
                id: "template".into(),
                dialect: "openai-compatible-chat".into(),
                model: "model".into(),
                adapter_revision: "1".into(),
                member_request_ids: vec!["request-a".into(), "request-b".into()],
                medoid_request_id: "request-a".into(),
                cohesion: 0.9,
                stable_spans: vec![],
                slots: vec![],
            }],
            findings: vec![Finding {
                id: "finding".into(),
                template_id: "template".into(),
                cause: FindingCause::InlineDynamicSlot,
                source: SourceLocation {
                    json_path: "$.messages[0].content".into(),
                    unit_index: Some(0),
                    byte_start: Some(0),
                    byte_end: Some(8),
                    role: Some("system".into()),
                },
                actual_prefix_bytes: 0,
                potential_prefix_bytes: 100,
                blocked_stable_bytes: 100,
                affected_count: 2,
                confidence: 0.9,
                score: score.clone(),
                evidence: Evidence {
                    representative_request_ids: vec!["request-a".into(), "request-b".into()],
                    divergent_excerpts: vec!["<script>alert(1)</script>&\"".into()],
                    blocked_stable_excerpt: "stable".into(),
                },
                counterfactual: "move".into(),
                recommendation: "review".into(),
            }],
            top_k: vec![TopKEntry {
                rank: 1,
                finding_id: "finding".into(),
                score,
            }],
        };
        fs::write(&input, serde_json::to_vec(&result).expect("serialize")).expect("fixture");

        write_report(&input, &output).expect("render report");
        let html = fs::read_to_string(output).expect("report");
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;&amp;\""));
    }

    #[test]
    fn refuses_to_overwrite_its_input() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("analysis.json");
        fs::write(&path, b"not replaced").expect("fixture");
        assert!(write_report(&path, &path).is_err());
        assert_eq!(fs::read(&path).expect("preserved"), b"not replaced");
    }
}
