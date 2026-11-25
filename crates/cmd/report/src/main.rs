//! 本文件提供 HTML/Markdown 报告 CLI 入口和退出错误输出。

use std::path::{self, PathBuf};

use actrail_kv_reporter::{report_paths, write_reports, LocaleFactory, ReportFormat};
use actrail_kv_settings::Settings;
use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Render an Actrail KV analysis as HTML and/or Markdown"
)]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    input: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    /// Output format. Omit to generate both HTML and Markdown.
    #[arg(long, value_enum)]
    format: Option<FormatArg>,
    /// Report locale. Omit to use zh-CN.
    #[arg(long)]
    locale: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum FormatArg {
    Html,
    Md,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReportSettings {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    format: Option<FormatArg>,
    locale: String,
}

impl Default for ReportSettings {
    fn default() -> Self {
        Self {
            input: None,
            output: None,
            format: None,
            locale: "zh-CN".into(),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut settings = Settings::load(cli.config.as_deref(), false)?;
    let file = settings.section("report", ReportSettings::default())?;
    LocaleFactory::init_from_code(&cli.locale.unwrap_or(file.locale))?;
    let format = cli.format.or(file.format).map(|format| match format {
        FormatArg::Html => ReportFormat::Html,
        FormatArg::Md => ReportFormat::Markdown,
    });

    let input = cli
        .input
        .or_else(|| file.input.map(|path| settings.resolve(path)));
    let input = match input {
        Some(path) => path,
        None => settings
            .configured_path("analyzer", "output")?
            .unwrap_or_else(|| settings.resolve("analysis.json")),
    };
    let output = match cli.output {
        Some(path) => path::absolute(path)?,
        None => {
            let default = if format == Some(ReportFormat::Markdown) {
                "report.md"
            } else {
                "report.html"
            };
            settings.resolve(file.output.unwrap_or_else(|| default.into()))
        }
    };
    for (_, path) in report_paths(&output, format) {
        settings.check_output(&path)?;
    }
    settings.finish()?;
    write_reports(&input, &output, format)
}

#[cfg(test)]
#[path = "../../../../tests/unit/cmd/report/main.rs"]
mod tests;
