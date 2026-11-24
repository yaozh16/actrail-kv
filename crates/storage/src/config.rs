//! 本文件加载存储后端及其保留期设置，并保护采集位置免受输出覆盖。

use std::{
    ffi::OsString,
    fs,
    num::{NonZeroU32, NonZeroU64},
    path::{Path, PathBuf},
};

use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use crate::StorageError;

#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "StorageFile")]
pub enum StorageConfig {
    Sqlite(SqliteConfig),
    Ndjson(NdjsonConfig),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageFile {
    storage: StorageSelection,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageSelection {
    backend: Backend,
    backends: BackendConfigs,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Backend {
    Sqlite,
    Ndjson,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackendConfigs {
    sqlite: Option<SqliteConfig>,
    ndjson: Option<NdjsonConfig>,
}

impl TryFrom<StorageFile> for StorageConfig {
    type Error = StorageError;

    fn try_from(file: StorageFile) -> Result<Self, Self::Error> {
        let StorageSelection { backend, backends } = file.storage;
        let (config, name) = match backend {
            Backend::Sqlite => (backends.sqlite.map(Self::Sqlite), "sqlite"),
            Backend::Ndjson => (backends.ndjson.map(Self::Ndjson), "ndjson"),
        };
        config.ok_or_else(|| {
            StorageError::InvalidConfig(format!(
                "missing storage.backends.{name} for selected backend"
            ))
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SqliteConfig {
    pub path: PathBuf,
    pub retention: Option<RetentionConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NdjsonConfig {
    pub directory: PathBuf,
    #[serde(default = "default_segment_seconds")]
    pub segment_seconds: NonZeroU32,
    #[serde(default = "default_segment_bytes")]
    pub max_segment_bytes: NonZeroU64,
    pub retention: Option<RetentionConfig>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionConfig {
    pub max_age_seconds: NonZeroU32,
    #[serde(default = "default_cleanup_seconds")]
    pub cleanup_interval_seconds: NonZeroU32,
}

impl RetentionConfig {
    pub(crate) fn cutoff(self) -> OffsetDateTime {
        OffsetDateTime::now_utc() - Duration::seconds(self.max_age_seconds.get().into())
    }

    pub(crate) fn check(self, captured_at: OffsetDateTime) -> Result<(), StorageError> {
        if captured_at < self.cutoff() {
            return Err(StorageError::ExpiredCapture);
        }
        Ok(())
    }
}

impl StorageConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let raw = fs::read(path)?;
        let mut config: Self = serde_json::from_slice(&raw)?;
        let relative_to = path.parent().unwrap_or_else(|| Path::new("."));
        match &mut config {
            Self::Sqlite(config) => resolve_path(&mut config.path, relative_to)?,
            Self::Ndjson(config) => resolve_path(&mut config.directory, relative_to)?,
        }
        Ok(config)
    }

    pub fn validate_output(&self, config_file: &Path, output: &Path) -> Result<(), StorageError> {
        let output_path = match output.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let parent = output.parent().filter(|path| !path.as_os_str().is_empty());
                let name = output.file_name().ok_or_else(|| {
                    StorageError::InvalidConfig("output needs a file name".into())
                })?;
                parent
                    .unwrap_or_else(|| Path::new("."))
                    .canonicalize()?
                    .join(name)
            }
            Err(error) => return Err(error.into()),
        };
        if same_file_if_exists(config_file, &output_path)? {
            return Err(StorageError::InvalidConfig(
                "output overlaps with the storage configuration".into(),
            ));
        }
        let overlaps = match self {
            Self::Ndjson(config) => output_path.starts_with(config.directory.canonicalize()?),
            Self::Sqlite(config) => {
                let db_path = config.path.canonicalize()?;
                let mut overlaps = false;
                for suffix in ["", "-wal", "-shm", "-journal"] {
                    let mut path: OsString = db_path.as_os_str().to_owned();
                    path.push(suffix);
                    let path = Path::new(&path);
                    overlaps |= output_path == path || same_file_if_exists(path, &output_path)?;
                }
                overlaps
            }
        };
        if overlaps {
            return Err(StorageError::InvalidConfig(
                "output overlaps with the capture store".into(),
            ));
        }
        Ok(())
    }
}

fn resolve_path(path: &mut PathBuf, relative_to: &Path) -> Result<(), StorageError> {
    if path.as_os_str().is_empty() {
        return Err(StorageError::InvalidConfig(
            "storage path must not be empty".into(),
        ));
    }
    if path.is_relative() {
        *path = relative_to.join(&*path);
    }
    Ok(())
}

fn same_file_if_exists(left: &Path, right: &Path) -> Result<bool, StorageError> {
    match same_path_identity(left, right) {
        Ok(same) => Ok(same),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[cfg(unix)]
fn same_path_identity(left: &Path, right: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let left = fs::metadata(left)?;
    let right = fs::metadata(right)?;
    Ok((left.dev(), left.ino()) == (right.dev(), right.ino()))
}

#[cfg(not(unix))]
fn same_path_identity(left: &Path, right: &Path) -> std::io::Result<bool> {
    same_file::is_same_file(left, right)
}

fn default_segment_seconds() -> NonZeroU32 {
    NonZeroU32::new(3600).unwrap()
}

fn default_segment_bytes() -> NonZeroU64 {
    NonZeroU64::new(64 * 1024 * 1024).unwrap()
}

fn default_cleanup_seconds() -> NonZeroU32 {
    NonZeroU32::new(60).unwrap()
}
