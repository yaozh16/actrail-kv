//! 本二进制只解析接收器参数、绑定监听地址并处理优雅关闭。

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use actrail_kv_receiver::{router, NdjsonAppender};
use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "actrail-kv-receiver")]
#[command(about = "Receive complete model request payloads as NDJSON")]
struct Arguments {
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: SocketAddr,
    #[arg(long)]
    output: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = Arguments::parse();
    let appender = NdjsonAppender::open(&arguments.output).with_context(|| {
        format!(
            "failed to open receiver output {}",
            arguments.output.display()
        )
    })?;
    let listener = tokio::net::TcpListener::bind(arguments.listen)
        .await
        .with_context(|| format!("failed to bind receiver to {}", arguments.listen))?;
    eprintln!("listening on {}", listener.local_addr()?);

    axum::serve(listener, router(Arc::new(appender)))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("receiver server failed")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_loopback_listener() {
        let arguments =
            Arguments::try_parse_from(["actrail-kv-receiver", "--output", "requests.ndjson"])
                .expect("parse defaults");

        assert_eq!(arguments.listen, "127.0.0.1:8080".parse().expect("address"));
    }
}
