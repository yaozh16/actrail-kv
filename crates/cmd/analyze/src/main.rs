//! 本文件提供 actrail-kv-analyze 的离线 CLI 入口：JSON 配置文件 + 显式参数覆盖。

mod selection;

use std::path::PathBuf;

use actrail_kv_analyzer::config::AnalyzeConfig;
use actrail_kv_analyzer::run::{analyze_captures_with_selection, analyze_file_with_selection};
use actrail_kv_settings::Settings;
use actrail_kv_storage::{open_store, StorageConfig, StoreAccess};
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

use selection::{SelectionArgs, SelectionSettings};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Analyze captured LLM requests for structural KV cache disruption"
)]
struct Cli {
    #[arg(long, conflicts_with = "storage_config")]
    input: Option<PathBuf>,
    #[arg(long)]
    storage_config: Option<PathBuf>,
    #[command(flatten)]
    selection: SelectionArgs,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, value_parser = parse_positive_usize)]
    top_k: Option<usize>,
    #[arg(long, value_parser = parse_positive_u64)]
    comparison_window_seconds: Option<u64>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AnalyzerSettings {
    input: Option<PathBuf>,
    output: PathBuf,
    options: AnalyzeConfig,
    selection: SelectionSettings,
}

impl Default for AnalyzerSettings {
    fn default() -> Self {
        Self {
            input: None,
            output: "analysis.json".into(),
            options: AnalyzeConfig::default(),
            selection: SelectionSettings::default(),
        }
    }
}

enum CaptureInput {
    File(PathBuf),
    Store {
        config: StorageConfig,
        override_path: Option<PathBuf>,
    },
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
    let mut settings = Settings::load(cli.config.as_deref(), true)?;
    let file = settings.section("analyzer", AnalyzerSettings::default())?;
    let mut config = settings.legacy::<AnalyzeConfig>()?.unwrap_or(file.options);
    if let Some(top_k) = cli.top_k {
        config.top_k = top_k;
    }
    if let Some(window) = cli.comparison_window_seconds {
        config.comparison_window_seconds = window;
    }
    let options = config.into_options()?;
    let (selection, scan_filter) = cli.selection.into_filters(file.selection)?;
    let output = cli.output.unwrap_or_else(|| settings.resolve(file.output));
    settings.check_output(&output)?;
    let input = if let Some(path) = cli.storage_config {
        let config = StorageConfig::from_file(&path)
            .with_context(|| format!("failed to load storage config {}", path.display()))?;
        CaptureInput::Store {
            config,
            override_path: Some(path),
        }
    } else {
        let input = cli
            .input
            .or_else(|| file.input.map(|path| settings.resolve(path)));
        let input = match input {
            Some(input) => Some(input),
            None => settings.configured_path("receiver", "output")?,
        };
        match input {
            Some(input) => CaptureInput::File(input),
            None => {
                let storage = settings.section("storage", StorageConfig::settings_defaults())?;
                let config = StorageConfig::from_settings_section(storage, settings.directory())?;
                CaptureInput::Store {
                    config,
                    override_path: None,
                }
            }
        }
    };
    let settings_path = settings.finish()?;
    match input {
        CaptureInput::File(input) => {
            analyze_file_with_selection(&input, &output, options, &selection)?;
        }
        CaptureInput::Store {
            config,
            override_path,
        } => {
            config.validate_output(override_path.as_deref().unwrap_or(&settings_path), &output)?;
            let store = open_store(config, StoreAccess::ReadOnly).await?;
            analyze_captures_with_selection(
                store.scan(scan_filter).await?,
                &output,
                options,
                &selection,
            )
            .await?;
        }
    }
    Ok(())
}
