//! 本文件提供 actrail-kv-analyze 的离线 CLI 入口：JSON 配置文件 + 显式参数覆盖。

use std::path::PathBuf;

use actrail_kv_analyzer::config::AnalyzeConfig;
use actrail_kv_analyzer::run::{analyze_captures, analyze_file};
use actrail_kv_storage::{open_store, CaptureFilter, StorageConfig, StoreAccess};
use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Analyze captured LLM requests for structural KV cache disruption"
)]
struct Cli {
    #[arg(
        long,
        required_unless_present = "storage_config",
        conflicts_with = "storage_config"
    )]
    input: Option<PathBuf>,
    #[arg(long)]
    storage_config: Option<PathBuf>,
    #[arg(long, requires = "storage_config")]
    session_key: Option<String>,
    #[arg(long, requires = "storage_config")]
    from: Option<String>,
    #[arg(long, requires = "storage_config")]
    until: Option<String>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, value_parser = parse_positive_usize)]
    top_k: Option<usize>,
    #[arg(long, value_parser = parse_positive_u64)]
    comparison_window_seconds: Option<u64>,
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|error| format!("invalid positive integer: {error}"))?;
    if parsed == 0 {
        return Err("value must be greater than zero".to_owned());
    }
    Ok(parsed)
}

fn parse_positive_u64(value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|error| format!("invalid positive integer: {error}"))?;
    if parsed == 0 {
        return Err("value must be greater than zero".to_owned());
    }
    Ok(parsed)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = match &cli.config {
        Some(path) => {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read config {}", path.display()))?;
            AnalyzeConfig::from_json_str(&raw)
                .with_context(|| format!("invalid config {}", path.display()))?
        }
        None => AnalyzeConfig::default(),
    };
    if let Some(top_k) = cli.top_k {
        config.top_k = top_k;
    }
    if let Some(window) = cli.comparison_window_seconds {
        config.comparison_window_seconds = window;
    }
    let options = config.into_options()?;
    if let Some(path) = cli.storage_config {
        let filter =
            CaptureFilter::parse(cli.session_key, cli.from.as_deref(), cli.until.as_deref())?;
        let storage = StorageConfig::from_file(&path)
            .with_context(|| format!("failed to load storage config {}", path.display()))?;
        storage.validate_output(&path, &cli.output)?;
        let store = open_store(storage, StoreAccess::ReadOnly).await?;
        analyze_captures(store.scan(filter).await?, &cli.output, options).await?;
    } else {
        let input = cli.input.context("missing analysis input")?;
        analyze_file(&input, &cli.output, options)?;
    }
    Ok(())
}
