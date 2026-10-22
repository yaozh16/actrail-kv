//! 本文件验证 HTML/Markdown 双报告选择与路径派生。

use std::fs;

use crate::{write_report, write_reports, ReportFormat};

fn fixture(directory: &std::path::Path) -> std::path::PathBuf {
    let input = directory.join("analysis.json");
    fs::write(
        &input,
        serde_json::to_vec(&crate::result_loader::tests::fixture()).expect("serialize"),
    )
    .expect("write fixture");
    input
}

#[test]
fn default_mode_generates_html_and_markdown_from_html_or_md_base() {
    for output_name in ["report.html", "report.md", "report"] {
        let directory = tempfile::tempdir().expect("tempdir");
        let input = fixture(directory.path());
        let output = directory.path().join(output_name);
        write_reports(&input, &output, None).expect("write reports");

        let html = fs::read_to_string(directory.path().join("report.html")).expect("html");
        let markdown = fs::read_to_string(directory.path().join("report.md")).expect("markdown");
        assert!(html.starts_with("<!doctype html>"));
        assert!(markdown.starts_with("# LLM KV 缓存结构诊断"));

        let legacy_html = directory.path().join("legacy.html");
        write_report(&input, &legacy_html).expect("legacy html");
        assert_eq!(
            fs::read(directory.path().join("report.html")).expect("html bytes"),
            fs::read(legacy_html).expect("legacy html bytes")
        );
    }
}

#[test]
fn default_mode_treats_other_extensions_as_a_base_path() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = fixture(directory.path());
    write_reports(&input, &directory.path().join("report.v1"), None).expect("reports");

    assert!(directory.path().join("report.v1.html").exists());
    assert!(directory.path().join("report.v1.md").exists());
}

#[test]
fn explicit_format_writes_exact_output_path_only() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = fixture(directory.path());
    let html = directory.path().join("custom.out");
    write_reports(&input, &html, Some(ReportFormat::Html)).expect("html");
    assert!(fs::read_to_string(&html)
        .expect("html")
        .starts_with("<!doctype html>"));
    assert!(!directory.path().join("custom.md").exists());

    let markdown = directory.path().join("custom.md.out");
    write_reports(&input, &markdown, Some(ReportFormat::Markdown)).expect("markdown");
    assert!(fs::read_to_string(&markdown)
        .expect("markdown")
        .starts_with("# LLM KV 缓存结构诊断"));
    assert!(!directory.path().join("custom.md.out.html").exists());
}
