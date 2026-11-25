//! 本命令通过统一存储接口将匹配的采集记录导出为 NDJSON。

use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use actrail_kv_settings::Settings;
use actrail_kv_storage::{open_store, CaptureFilter, StorageConfig, StoreAccess};
use anyhow::{Context, Result};
use clap::Parser;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(version, about = "Export captured model requests as NDJSON")]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    storage_config: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    session_key: Option<String>,
    #[arg(long)]
    from: Option<String>,
    #[arg(long)]
    until: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExportSettings {
    output: PathBuf,
    session_key: Option<String>,
    from: Option<String>,
    until: Option<String>,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            output: "exported.ndjson".into(),
            session_key: None,
            from: None,
            until: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut settings = Settings::load(cli.config.as_deref(), false)?;
    let file = settings.section("export", ExportSettings::default())?;
    let from = cli.from.or(file.from);
    let until = cli.until.or(file.until);
    let filter = CaptureFilter::parse(
        cli.session_key.or(file.session_key),
        from.as_deref(),
        until.as_deref(),
    )?;
    let output = cli.output.unwrap_or_else(|| settings.resolve(file.output));
    settings.check_output(&output)?;
    let config = if let Some(path) = &cli.storage_config {
        StorageConfig::from_file(path)
            .with_context(|| format!("failed to load storage config {}", path.display()))?
    } else {
        let storage = settings.section("storage", StorageConfig::settings_defaults())?;
        StorageConfig::from_settings_section(storage, settings.directory())?
    };
    let settings_path = settings.finish()?;
    config.validate_output(
        cli.storage_config.as_deref().unwrap_or(&settings_path),
        &output,
    )?;
    let store = open_store(config, StoreAccess::ReadOnly).await?;
    let mut rows = store.scan(filter).await?;

    let output_directory = output.parent().filter(|path| !path.as_os_str().is_empty());
    let mut temporary =
        tempfile::NamedTempFile::new_in(output_directory.unwrap_or_else(|| Path::new(".")))?;
    {
        let mut output = BufWriter::new(temporary.as_file_mut());
        while let Some(record) = rows.try_next().await? {
            serde_json::to_writer(&mut output, &record)?;
            output.write_all(b"\n")?;
        }
        output.flush()?;
    }
    temporary.as_file().sync_all()?;
    temporary.persist(&output)?;
    Ok(())
}
