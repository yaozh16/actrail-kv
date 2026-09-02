//! 本文件提供 actrail-kv-analyze 的最小离线 CLI 入口和参数校验。

use std::path::PathBuf;

use actrail_kv_analyzer::run::{analyze_file, AnalysisOptions};
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Analyze captured LLM requests for structural KV cache disruption"
)]
struct Cli {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 20, value_parser = parse_positive_usize)]
    top_k: usize,
    #[arg(long, default_value_t = 3_600, value_parser = parse_positive_u64)]
    comparison_window_seconds: u64,
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let options = AnalysisOptions {
        top_k: cli.top_k,
        comparison_window_seconds: cli.comparison_window_seconds,
        ..AnalysisOptions::default()
    };
    analyze_file(&cli.input, &cli.output, options)?;
    Ok(())
}
