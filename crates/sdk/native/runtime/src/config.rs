//! 从内存中的完整 TOML 配置读取所属模块，并固定相对路径基准。
use crate::{Error, Result};
use config_contract::{
    toml_edit::{DocumentMut, Table},
    Config as ParseConfig,
};
use std::path::{Path, PathBuf};

pub struct Config {
    document: DocumentMut,
    pub base_dir: PathBuf,
}
impl Config {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let value = crate::request(bytes)?;
        let base_dir = value
            .get("base_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::config("base_dir is required"))?;
        let base_dir = PathBuf::from(base_dir);
        if !base_dir.is_absolute() {
            return Err(Error::config("base_dir must be absolute"));
        }
        let text = value
            .get("config_toml")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::config("config_toml is required"))?;
        Ok(Self {
            document: text.parse().map_err(Error::config)?,
            base_dir,
        })
    }
    pub fn section<T: ParseConfig>(&self, name: &str) -> Result<T> {
        let empty = Table::new();
        let table = match self.document.get(name) {
            Some(item) => item
                .as_table()
                .ok_or_else(|| Error::config(format!("{name} must be a TOML table")))?,
            None => &empty,
        };
        T::parse_toml_table(table)
            .map(|result| result.0)
            .map_err(Error::config)
    }
    pub fn path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_owned()
        } else {
            self.base_dir.join(path)
        }
    }
}
