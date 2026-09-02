//! 本二进制加载 receiver 配置、解析参数覆盖、绑定监听并处理优雅关闭。

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use actrail_kv_receiver::{router_with_limit, NdjsonAppender, ReceiverConfig};
use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "actrail-kv-receiver")]
#[command(about = "Receive complete model request payloads as NDJSON")]
struct Arguments {
    #[arg(long)]
    listen: Option<SocketAddr>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, value_parser = parse_positive_usize)]
    max_payload_bytes: Option<usize>,
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

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = Arguments::parse();
    let mut config = match &arguments.config {
        Some(path) => {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read config {}", path.display()))?;
            ReceiverConfig::from_json_str(&raw)
                .with_context(|| format!("invalid config {}", path.display()))?
        }
        None => ReceiverConfig::default(),
    };
    if let Some(listen) = arguments.listen {
        config.listen = listen.to_string();
    }
    if let Some(max_payload_bytes) = arguments.max_payload_bytes {
        config.max_payload_bytes = max_payload_bytes;
    }
    config.validate()?;
    let listen: SocketAddr = config
        .listen
        .parse()
        .with_context(|| format!("invalid listen address {}", config.listen))?;
    let appender = NdjsonAppender::open(&arguments.output).with_context(|| {
        format!(
            "failed to open receiver output {}",
            arguments.output.display()
        )
    })?;
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("failed to bind receiver to {}", listen))?;
    eprintln!("listening on {}", listener.local_addr()?);

    axum::serve(
        listener,
        router_with_limit(Arc::new(appender), config.max_payload_bytes),
    )
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
    fn config_defaults_to_loopback_listener() {
        let config = ReceiverConfig::default();
        let address: SocketAddr = config.listen.parse().expect("valid default address");
        assert_eq!(address, "127.0.0.1:8080".parse().expect("address"));
    }
}
