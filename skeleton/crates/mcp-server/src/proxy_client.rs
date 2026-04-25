//! Thin JSON-RPC client that forwards MCP tool calls to the AgentGeyser
//! proxy. Pure translator — owns no key material, performs no user-side
//! signing. All transaction bytes returned by the proxy flow through
//! verbatim.

use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

pub const PROXY_HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const BODY_TAIL_LIMIT: usize = 200;

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

pub fn http_client() -> Result<reqwest::Client, ProxyError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| ProxyError::Http(format!("failed to build proxy HTTP client: {e}")))
}

fn body_tail(body: &str) -> String {
    body.chars()
        .rev()
        .take(BODY_TAIL_LIMIT)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

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
        .map_err(|e| {
            let msg = if e.is_timeout() {
                format!("request timed out: {e}")
            } else {
                e.to_string()
            };
            ProxyError::Http(msg)
        })?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ProxyError::Http(format!("failed to read proxy response body: {e}")))?;
    if !status.is_success() {
        return Err(ProxyError::Http(format!(
            "HTTP {status}; body tail: {}",
            body_tail(&text)
        )));
    }
    let envelope: JsonRpcResponse = serde_json::from_str(&text)
        .map_err(|e| ProxyError::Malformed(format!("{e}; body tail: {}", body_tail(&text))))?;
    if let Some(err) = envelope.error {
        return Err(ProxyError::JsonRpc(err));
    }
    envelope
        .result
        .ok_or_else(|| ProxyError::Malformed("missing result field".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{io::AsyncReadExt, net::TcpListener};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn proxy_client_timeout_surfaces_transport_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0_u8; 256];
            let _ = socket.read(&mut buf).await;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(50))
            .build()
            .unwrap();
        let err = call(
            &client,
            &format!("http://{addr}"),
            "ag_listSkills",
            json!({}),
        )
        .await
        .unwrap_err();

        match err {
            ProxyError::Http(msg) => assert!(
                msg.contains("timed out") || msg.contains("operation timed out"),
                "unexpected timeout error: {msg}"
            ),
            other => panic!("expected timeout transport error, got {other:?}"),
        }
        server.abort();
    }

    #[tokio::test]
    async fn proxy_client_returns_meaningful_error_on_500_html() {
        let mock = MockServer::start().await;
        let body = "<html>panic</html>0123456789 body tail";
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string(body))
            .mount(&mock)
            .await;

        let client = reqwest::Client::new();
        let err = call(&client, &mock.uri(), "ag_listSkills", json!({}))
            .await
            .unwrap_err();

        match err {
            ProxyError::Http(msg) => {
                assert!(msg.contains("HTTP 500"), "unexpected HTTP error: {msg}");
                assert!(
                    msg.contains(&body[..20]),
                    "HTTP error should include response body tail: {msg}"
                );
            }
            other => panic!("expected HTTP error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn proxy_client_error_includes_body_tail() {
        let mock = MockServer::start().await;
        let body = format!("{}TAIL-MARKER", "x".repeat(240));
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string(body))
            .mount(&mock)
            .await;

        let client = reqwest::Client::new();
        let err = call(&client, &mock.uri(), "ag_listSkills", json!({}))
            .await
            .unwrap_err();

        match err {
            ProxyError::Http(msg) => {
                let tail = msg.split_once("body tail: ").unwrap().1;
                assert!(tail.contains("TAIL-MARKER"), "missing body tail: {msg}");
                assert!(tail.chars().count() <= BODY_TAIL_LIMIT);
            }
            other => panic!("expected HTTP error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn proxy_client_malformed_json_distinct_from_http() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&mock)
            .await;

        let client = reqwest::Client::new();
        let err = call(&client, &mock.uri(), "ag_listSkills", json!({}))
            .await
            .unwrap_err();

        match err {
            ProxyError::Malformed(msg) => {
                assert!(
                    msg.contains("not json"),
                    "malformed error should include response body tail: {msg}"
                );
            }
            other => panic!("expected malformed error, got {other:?}"),
        }
    }
}
