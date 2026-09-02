//! 本 crate 提供最小 HTTP 接收器，将完整请求 payload 安全追加为 NDJSON。

mod http_receiver;
mod ndjson_appender;

pub use http_receiver::router;
pub use ndjson_appender::{AppendError, NdjsonAppender};
