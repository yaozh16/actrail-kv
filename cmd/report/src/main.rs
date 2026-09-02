//! 本文件提供 actrail-kv-report 的最小 CLI 入口和退出错误输出。

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about = "Render an Actrail KV analysis as static HTML")]
struct Cli {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    actrail_kv_reporter::write_report(&cli.input, &cli.output)
}
