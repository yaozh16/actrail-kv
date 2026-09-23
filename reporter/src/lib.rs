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

    use actrail_kv_artifacts::{SessionEventType, SessionReport, SessionSwitchEvent};

    use super::*;
    use crate::result_loader::tests::fixture;

    #[test]
    fn report_escapes_untrusted_evidence() {
        let directory = tempfile::tempdir().expect("tempdir");
        let input = directory.path().join("analysis.json");
        let output = directory.path().join("report.html");
        let mut result = fixture();
        result.templates[0].comparison_group.endpoint_key = "<iframe>endpoint</iframe>".into();
        result.defects[0].comparison_group.endpoint_key = "<iframe>endpoint</iframe>".into();
        result.defects[0].id = "<svg onload=alert(1)>".into();
        result.top_k[0] = result.defects[0].id.clone();
        result.defects[0].mismatch.variants[0]
            .representative
            .request_id = "<u>request-a</u>".into();
        result.defects[0].mismatch.variants[0].member_request_ids[0] = "<u>request-a</u>".into();
        result.templates[0].member_request_ids[0] = "<u>request-a</u>".into();
        result.templates[0].medoid_request_id = "<u>request-a</u>".into();
        result.session_reports = vec![SessionReport {
            session_key: "<u>session-a</u>".into(),
            endpoint_key: "<u>endpoint</u>".into(),
            model: "<u>model</u>".into(),
            request_count: 2,
            append_count: 0,
            fork_count: 0,
            reorder_count: 0,
            reset_count: 1,
            total_recomputed_bytes: 10,
            total_stable_after_switch_bytes: 5,
            total_prefix_cut_bytes: 3,
            avg_prefix_cut_bytes: 3,
            events: vec![SessionSwitchEvent {
                event_type: SessionEventType::Reset,
                prev_request_id: "<u>prev-a</u>".into(),
                next_request_id: "<u>next-a</u>".into(),
                lcp_units: 1,
                lcp_bytes: 2,
                previous_pair_lcp_bytes: 0,
                prefix_cut_bytes: 3,
                next_total_bytes: 5,
                recomputed_bytes: 3,
                stable_after_switch_bytes: 1,
                next_block_runs: "<script>stable()</script>".into(),
            }],
        }];
        result.defects[0].mismatch.variants[0]
            .representative
            .sources[0]
            .json_path = "<a href=evil>path</a>".into();
        fs::write(&input, serde_json::to_vec(&result).expect("serialize")).expect("fixture");

        write_report(&input, &output).expect("render report");
        let html = fs::read_to_string(output).expect("report");
        for unsafe_fragment in [
            "<script>alert(1)</script>",
            "<script>stable()</script>",
            "<img src=x onerror=alert(1)>",
            "<b>统一生成方式</b>",
            "<iframe>endpoint</iframe>",
            "<svg onload=alert(1)>",
            "<u>request-a</u>",
            "<a href=evil>path</a>",
            "<u>session-a</u>",
            "<u>prev-a</u>",
            "<u>next-a</u>",
        ] {
            assert!(!html.contains(unsafe_fragment));
        }
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("<strong>P1</strong>"));
        assert!(html.contains("<strong>X</strong>"));
        assert!(html.contains("<strong>P2</strong>"));
    }

    #[test]
    fn refuses_to_overwrite_its_input() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("analysis.json");
        fs::write(&path, b"not replaced").expect("fixture");
        assert!(write_report(&path, &path).is_err());
        assert_eq!(fs::read(&path).expect("preserved"), b"not replaced");
    }

    #[test]
    fn report_renders_kv_cache_metrics_with_reported_and_estimated_basis() {
        let directory = tempfile::tempdir().expect("tempdir");
        let input = directory.path().join("analysis.json");
        let output = directory.path().join("report.html");
        let mut result = fixture();
        result.cache_metrics = vec![
            actrail_kv_artifacts::KvCacheMetric {
                request_id: "req-1".into(),
                session_key: Some("session-a".into()),
                sequence: 1,
                captured_at: None,
                source: None,
                prompt_tokens: Some(1000),
                cached_tokens: Some(200),
                miss_tokens: Some(800),
                output_tokens: Some(12),
                hit_rate: 0.2,
                hit_rate_delta: None,
                basis: actrail_kv_artifacts::CacheMetricBasis::Reported,
                estimated_lcp_bytes: None,
                payload_bytes: 4000,
                ttft_ms: Some(320),
                total_ms: Some(1850),
            },
            actrail_kv_artifacts::KvCacheMetric {
                request_id: "req-2".into(),
                session_key: Some("session-a".into()),
                sequence: 2,
                captured_at: None,
                source: None,
                prompt_tokens: None,
                cached_tokens: None,
                miss_tokens: None,
                output_tokens: None,
                hit_rate: 0.8,
                hit_rate_delta: Some(0.6),
                basis: actrail_kv_artifacts::CacheMetricBasis::Estimated,
                estimated_lcp_bytes: Some(3200),
                payload_bytes: 4000,
                ttft_ms: None,
                total_ms: None,
            },
        ];
        fs::write(&input, serde_json::to_vec(&result).expect("serialize")).expect("fixture");
        write_report(&input, &output).expect("render report");
        let html = fs::read_to_string(output).expect("report");
        assert!(html.contains("KV 缓存命中率"));
        assert!(html.contains("20.0%"));
        assert!(html.contains("+60.0pp"));
        assert!(html.contains("估算"));
        assert!(html.contains("LCP 3200B / payload 4000B"));
        // 时延列：上报的按毫秒展示，未上报的显示 —（不是 0）。
        assert!(html.contains("<th>TTFT</th>"));
        assert!(html.contains("<th>总耗时</th>"));
        assert!(html.contains("320 ms"));
        assert!(html.contains("1850 ms"));
    }
}
