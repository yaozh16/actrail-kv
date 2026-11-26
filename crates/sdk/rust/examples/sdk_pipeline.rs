//! 演示持续采集、按需分析、直接内存渲染和显式报告导出。
use actrail_kv_sdk::*;
use serde_json::json;
use std::path::PathBuf;
fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let libraries =
        PathBuf::from(args.next().unwrap_or_else(|| "target/release".into())).canonicalize()?;
    let directory = PathBuf::from(args.next().unwrap_or_else(|| "sdk-data".into()));
    std::fs::create_dir_all(&directory)?;
    let directory = directory.canonicalize()?;
    let config = |name: &str| SdkConfig {
        library_path: libraries.join(format!("libactrail_{name}.so")),
        base_dir: directory.clone(),
        config_toml: String::new(),
    };
    let mut receiver = ReceiverFactory::create(config("receiver"))?;
    for workspace in ["billing", "search", "orders", "accounts"] {
        receiver.submit(&CaptureInput {
            payload: json!({"model":"example", "messages":[
                {"role":"system", "content":format!("Workspace: {workspace}. Follow the repository rules, read the code before making changes, and run the relevant tests before completion.")},
                {"role":"user", "content":"Fix the failing validation test."}
            ]}), source: Some("sdk-example".into()), session_key: Some("example".into()),
            comparison: CaptureComparison { endpoint_key: "example-model".into(), agent_key: None, model_deployment_key: None, kv_namespace: None },
        })?;
    }
    let mut analyzer = AnalyzerFactory::create(config("analyzer"))?;
    let analysis = analyzer.analyze(&Selection::default())?;
    analyzer.close()?;
    let mut reporter = ReporterFactory::create(config("reporter"))?;
    let report = reporter.render(
        &analysis,
        &RenderOptions {
            format: Some(ReportFormat::Html),
            locale: None,
        },
    )?;
    reporter.close()?;
    receiver.close()?;
    // 处理完成后仍可读取或显式导出，关闭实例不释放其输出。
    println!(
        "rendered {} bytes in memory",
        report.reports()?[0].content.len()
    );
    report.export("report.html")?;
    println!("exported {}", directory.join("report.html").display());
    Ok(())
}
