//! 本 crate 提供最小 HTTP 接收器，将完整请求 payload 安全追加为 NDJSON。

pub mod config;
mod http_receiver;
mod ndjson_appender;

pub use config::ReceiverConfig;
pub use http_receiver::{router, router_with_limit};
pub use ndjson_appender::{AppendError, NdjsonAppender};
