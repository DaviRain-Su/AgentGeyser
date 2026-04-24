//! F4.3 integration test: boot the HTTP transport on an ephemeral port and
//! drive `initialize` + `tools/list` via rmcp's client; assert both tools
//! are advertised. Tool-call forwarding is covered by F2/F3 unit tests.

use mcp_server::transport::http_router;
use rmcp::{
    ServiceExt,
    model::ClientInfo,
    transport::{
        StreamableHttpClientTransport,
        streamable_http_client::StreamableHttpClientTransportConfig,
    },
};

#[tokio::test]
async fn mcp_http_transport_roundtrip() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (router, ct) = http_router();
    let server_handle = tokio::spawn({
        let ct = ct.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { ct.cancelled_owned().await })
                .await;
        }
    });

    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!("http://{addr}/mcp")),
    );
    let client = ClientInfo::default().serve(transport).await?;
    let info = client.peer_info().cloned().expect("peer_info");
    assert_eq!(info.server_info.name, "agentgeyser-mcp-server");

    let tools = client.list_tools(Default::default()).await?;
    let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains(&"list_skills"), "missing: {names:?}");
    assert!(names.contains(&"invoke_skill"), "missing: {names:?}");

    let _ = client.cancel().await;
    ct.cancel();
    server_handle.await?;
    Ok(())
}
