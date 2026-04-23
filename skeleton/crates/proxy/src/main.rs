//! AgentGeyser proxy binary. See `lib.rs` for the axum router.

use std::sync::Arc;

use idl_registry::{IdlRegistry, MockYellowstoneStream, YellowstoneEvent};
use proxy::{router, sample_hello_idl, AppState, DEMO_PROGRAM_ID};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let registry = Arc::new(IdlRegistry::new());
    registry.insert_mock_idl(DEMO_PROGRAM_ID, sample_hello_idl());

    let stream = MockYellowstoneStream::new(vec![YellowstoneEvent::ProgramDeployed {
        program_id: DEMO_PROGRAM_ID.to_string(),
    }]);
    registry.attach_stream(stream);

    // Give the spawned task a moment to drain events before accepting requests.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let app = router(AppState {
        registry: Arc::clone(&registry),
    });

    let bind = std::env::var("AGENTGEYSER_BIND").unwrap_or_else(|_| "127.0.0.1:8899".to_string());
    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(?e, %bind, "primary bind failed; falling back to 127.0.0.1:8898");
            tokio::net::TcpListener::bind("127.0.0.1:8898").await?
        }
    };
    tracing::info!(addr = %listener.local_addr()?, "proxy listening");
    axum::serve(listener, app).await?;
    Ok(())
}
