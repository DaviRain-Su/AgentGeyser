//! `agentgeyser-mcp-server` — stdio MCP server entrypoint.
//!
//! F1 scope: set up a tokio runtime, connect [`AgentGeyserMcpServer`] to the
//! official rmcp stdio transport, and run the event loop until the client
//! disconnects. F4 will add `--transport http` as a secondary entrypoint.

use anyhow::Result;
use clap::Parser;
use mcp_server::{
    AgentGeyserMcpServer,
    transport::{Args, TransportKind, run_http},
};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Route tracing to stderr so it does not corrupt the stdio MCP framing.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let args = Args::parse();
    match args.transport {
        TransportKind::Stdio => {
            let server = AgentGeyserMcpServer::try_from_env()?;
            tracing::info!(proxy_url = %server.proxy_url, "starting agentgeyser-mcp-server on stdio");
            let service = server.serve(stdio()).await.inspect_err(|e| {
                tracing::error!(error = ?e, "mcp serve error");
            })?;
            service.waiting().await?;
        }
        TransportKind::Http => {
            tracing::info!(bind = %args.bind, "starting agentgeyser-mcp-server on streamable-http");
            run_http(&args.bind).await?;
        }
    }
    Ok(())
}
