//! 本文件公开 analysis.json 加载与安全静态 HTML 报告生成入口。

mod html_report_renderer;
mod result_loader;
mod session_report_renderer;

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

    let parent = output_parent(output);
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
        let parent = output_parent(output).canonicalize()?;
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

fn output_parent(output: &Path) -> &Path {
    output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
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
        result.conditional_local_sites[0]
            .comparison_group
            .endpoint_key = "<iframe>endpoint</iframe>".into();
        result.defects[0].id = "<svg onload=alert(1)>".into();
        result.top_k[0] = result.defects[0].id.clone();
        result.defects[0].mismatch.variants[0]
            .representative
            .request_id = "<u>request-a</u>".into();
        result.defects[0].mismatch.variants[0].member_request_ids[0] = "<u>request-a</u>".into();
        result.conditional_local_sites[0].mismatch.variants[0]
            .representative
            .request_id = "<u>request-a</u>".into();
        result.conditional_local_sites[0].mismatch.variants[0].member_request_ids[0] =
            "<u>request-a</u>".into();
        result.templates[0].member_request_ids[0] = "<u>request-a</u>".into();
        result.templates[0].medoid_request_id = "<u>request-a</u>".into();
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
            "<session&one>",
            "<old-history>",
            "<new-history>",
            "<messages/system>",
            "<source>",
            "<session-endpoint>",
        ] {
            assert!(!html.contains(unsafe_fragment));
        }
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("<strong>P1</strong>"));
        assert!(html.contains("<strong>X</strong>"));
        assert!(html.contains("<strong>P2</strong>"));
        assert!(html.contains("条件性局部位点"));
        assert!(html.contains("Session 时间线"));
        assert!(html.contains("source=&lt;source&gt;"));
        assert!(html.contains("endpoint=&lt;session-endpoint&gt;"));
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
    fn report_distinguishes_missing_session_data_from_zero_history_changes() {
        let mut result = fixture();
        result.session_analysis = actrail_kv_artifacts::SessionAnalysis::default();
        let html = render_html(&result).expect("render report");
        assert!(html.contains("未提供 Session ID 或没有可分析的时间线"));
    }
}
