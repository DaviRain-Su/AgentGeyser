//! F4: CLI args + streamable-HTTP transport entry point. Pure wire-format
//! swap; non-custodial invariant preserved (no signing code).

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};

use crate::AgentGeyserMcpServer;

/// Transport selector. `stdio` is the default (Claude Desktop).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum TransportKind {
    #[default]
    Stdio,
    Http,
}

/// Default bind for streamable-HTTP (loopback only).
pub const DEFAULT_BIND: &str = "127.0.0.1:9000";

/// CLI arguments for `agentgeyser-mcp-server`.
#[derive(Debug, Parser)]
#[command(
    name = "agentgeyser-mcp-server",
    about = "AgentGeyser MCP server (stdio | streamable-http)",
    version
)]
pub struct Args {
    /// Transport to serve on. Defaults to stdio for Claude Desktop.
    #[arg(long, value_enum, default_value_t = TransportKind::Stdio)]
    pub transport: TransportKind,

    /// Bind address for --transport http. Ignored for stdio.
    #[arg(long, default_value = DEFAULT_BIND)]
    pub bind: String,
}

/// Build the axum router hosting the MCP streamable-HTTP service at `/mcp`.
/// Returns a `CancellationToken` so tests can shut the service down cleanly.
pub fn http_router() -> (axum::Router, tokio_util::sync::CancellationToken) {
    let ct = tokio_util::sync::CancellationToken::new();
    let service: StreamableHttpService<AgentGeyserMcpServer, LocalSessionManager> =
        StreamableHttpService::new(
            || {
                AgentGeyserMcpServer::try_from_env()
                    .map_err(|e| std::io::Error::other(e.to_string()))
            },
            Default::default(),
            StreamableHttpServerConfig::default()
                .with_sse_keep_alive(None)
                .with_cancellation_token(ct.child_token()),
        );
    (axum::Router::new().nest_service("/mcp", service), ct)
}

/// Bind the streamable-HTTP transport on `addr` and serve until shutdown.
pub async fn run_http(addr: &str) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;
    tracing::info!(bound = %listener.local_addr()?, "streamable-http bound");
    let (router, _ct) = http_router();
    axum::serve(listener, router).await.context("axum serve")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// F4.4: `--transport` defaults to stdio when omitted.
    #[test]
    fn transport_defaults_to_stdio() {
        let parsed = Args::parse_from(["agentgeyser-mcp-server"]);
        assert_eq!(parsed.transport, TransportKind::Stdio);
        assert_eq!(parsed.bind, DEFAULT_BIND);
    }

    /// Explicit --transport http --bind override parses as expected.
    #[test]
    fn transport_http_with_bind_overrides_default() {
        let parsed = Args::parse_from([
            "agentgeyser-mcp-server",
            "--transport",
            "http",
            "--bind",
            "127.0.0.1:0",
        ]);
        assert_eq!(parsed.transport, TransportKind::Http);
        assert_eq!(parsed.bind, "127.0.0.1:0");
    }
}
