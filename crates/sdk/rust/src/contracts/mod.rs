//! 定义 SDK 配置与错误，业务配置文本保持整体透传。
use actrail_kv_sdk_contracts::ErrorDetail;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct SdkConfig {
    pub library_path: PathBuf,
    pub base_dir: PathBuf,
    pub config_toml: String,
}
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("dynamic library load failed: {0}")]
    Load(String),
    #[error("ABI mismatch: {0}")]
    Abi(String),
    #[error("invalid SDK response: {0}")]
    Protocol(String),
    #[error("operation failed ({status}, {code}): {message}", code=.detail.code, message=.detail.message)]
    Operation { status: i32, detail: ErrorDetail },
    #[error("JSON conversion failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O failed: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, SdkError>;
