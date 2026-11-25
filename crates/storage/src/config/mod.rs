//! 存储配置契约、类别选择和运行时参数解析。
mod backends;
mod defaults;
mod resolved;

use crate::StorageError;
pub use backends::{NdjsonConfig, NdjsonRetentionConfig, SqliteConfig, SqliteRetentionConfig};
use config_contract::{Config, RootConfig};
pub(crate) use resolved::{ResolvedStorageConfig, RetentionConfig};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Config)]
#[serde(default)]
pub struct StorageConfig {
    /// 存储后端类别，可选 sqlite、ndjson
    pub backend: StorageBackendKind,
    /// 共享保留期
    pub retention: Option<StorageRetentionConfig>,
    /// 各存储后端的参数
    pub backends: StorageBackendConfigs,
}
impl RootConfig for StorageConfig {}
impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackendKind::default(),
            retention: Some(StorageRetentionConfig::default()),
            backends: StorageBackendConfigs::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageBackendKind {
    #[default]
    Sqlite,
    Ndjson,
}

#[derive(Clone, Debug, Default, Serialize, Config)]
#[serde(default)]
pub struct StorageRetentionConfig {
    /// 共享保留秒数，0 关闭过期清理
    pub max_age_seconds: u32,
}

#[derive(Clone, Debug, Serialize, Config)]
#[serde(default)]
pub struct StorageBackendConfigs {
    /// SQLite 存储设置
    pub sqlite: Option<SqliteConfig>,
    /// NDJSON 存储设置
    pub ndjson: Option<NdjsonConfig>,
}
impl Default for StorageBackendConfigs {
    fn default() -> Self {
        Self {
            sqlite: Some(SqliteConfig::default()),
            ndjson: Some(NdjsonConfig::default()),
        }
    }
}

impl StorageConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let config = Self::load(path, &["storage"])
            .map_err(|error| StorageError::InvalidConfig(error.to_string()))?;
        config.resolve(path.parent().unwrap_or_else(|| Path::new(".")))
    }

    pub fn resolve(mut self, directory: &Path) -> Result<Self, StorageError> {
        match self.backend {
            StorageBackendKind::Sqlite => {
                if let Some(config) = &mut self.backends.sqlite {
                    resolved::resolve_path(&mut config.path, directory)?;
                }
            }
            StorageBackendKind::Ndjson => {
                if let Some(config) = &mut self.backends.ndjson {
                    resolved::resolve_path(&mut config.directory, directory)?;
                }
            }
        }
        Ok(self)
    }

    pub fn validate_output(&self, config_file: &Path, output: &Path) -> Result<(), StorageError> {
        self.clone()
            .into_backend()?
            .validate_output(config_file, output)
    }

    pub(crate) fn into_backend(self) -> Result<ResolvedStorageConfig, StorageError> {
        let shared_age = self.retention.unwrap_or_default().max_age_seconds;
        let resolved = match self.backend {
            StorageBackendKind::Sqlite => ResolvedStorageConfig::Sqlite(
                self.backends
                    .sqlite
                    .ok_or_else(|| {
                        StorageError::InvalidConfig("missing storage.backends.sqlite".into())
                    })?
                    .resolve(shared_age),
            ),
            StorageBackendKind::Ndjson => ResolvedStorageConfig::Ndjson(
                self.backends
                    .ndjson
                    .ok_or_else(|| {
                        StorageError::InvalidConfig("missing storage.backends.ndjson".into())
                    })?
                    .resolve(shared_age),
            ),
        };
        Ok(resolved)
    }
}
