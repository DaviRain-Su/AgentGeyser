//! Thin JSON-RPC client that forwards MCP tool calls to the AgentGeyser
//! proxy. Pure translator — owns no key material, performs no user-side
//! signing. All transaction bytes returned by the proxy flow through
//! verbatim.

use serde::Deserialize;
use serde_json::{Value, json};

/// Minimal JSON-RPC error shape returned by the AgentGeyser proxy.
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

/// JSON-RPC response envelope. Exactly one of `result`/`error` is populated.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

/// Errors the proxy client surfaces to tool handlers. Handlers translate
/// these into `CallToolResult { is_error: Some(true), .. }` instead of
/// panicking, per F2.5.
#[derive(Debug)]
pub enum ProxyError {
    /// Network / transport failure (connect refused, timeout, TLS, etc.).
    Http(String),
    /// Proxy returned a JSON-RPC `{error: ...}` envelope.
    JsonRpc(JsonRpcError),
    /// Proxy returned HTTP 2xx but the body was not decodable JSON-RPC.
    Malformed(String),
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyError::Http(m) => write!(f, "HTTP transport error: {m}"),
            ProxyError::JsonRpc(e) => {
                write!(f, "JSON-RPC error {}: {}", e.code, e.message)
            }
            ProxyError::Malformed(m) => write!(f, "malformed proxy response: {m}"),
        }
    }
}

impl std::error::Error for ProxyError {}

/// POST a JSON-RPC `{method, params}` call to `base_url` and return the
/// decoded `result` field. Surfaces errors via [`ProxyError`] rather than
/// panicking.
pub async fn call(
    client: &reqwest::Client,
    base_url: &str,
    method: &str,
    params: Value,
) -> Result<Value, ProxyError> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let resp = client
        .post(base_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| ProxyError::Http(e.to_string()))?;
    let envelope: JsonRpcResponse = resp
        .json()
        .await
        .map_err(|e| ProxyError::Malformed(e.to_string()))?;
    if let Some(err) = envelope.error {
        return Err(ProxyError::JsonRpc(err));
    }
    envelope
        .result
        .ok_or_else(|| ProxyError::Malformed("missing result field".to_string()))
}
