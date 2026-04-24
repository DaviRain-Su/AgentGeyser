//! AgentGeyser MCP server library.
//!
//! Hosts the [`AgentGeyserMcpServer`] handler that translates MCP requests
//! into AgentGeyser proxy JSON-RPC calls. F1 wired `initialize`; F2 added the
//! `list_skills` tool; F3 adds the `invoke_skill` tool that forwards to the
//! proxy's `ag_invokeSkill` JSON-RPC endpoint and returns the unsigned
//! unsigned `transaction_base64` for the user-side client. Future features
//! (F4) will add the streamable-HTTP transport.
//!
//! Non-custodial invariant: this crate never builds a signer, owns a
//! private key, or signs a transaction. It is a pure JSON-RPC ↔ MCP
//! translator.

pub mod proxy_client;
pub mod transport;

use std::sync::Arc;

use rmcp::{
    ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Content, Implementation, JsonObject,
        ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities,
        ServerInfo, Tool,
    },
    service::{RequestContext, RoleServer},
};
use serde_json::json;

use crate::proxy_client::ProxyError;

/// Default agentGeyser proxy URL (matches MVP-M2 user-managed proxy).
pub const DEFAULT_PROXY_URL: &str = "http://127.0.0.1:8999";

/// Canonical MCP tool name for listing AgentGeyser skills.
pub const TOOL_LIST_SKILLS: &str = "list_skills";

/// Canonical MCP tool description for `list_skills` (mirrored in docs).
pub const TOOL_LIST_SKILLS_DESCRIPTION: &str = "List available AgentGeyser skills";

/// Canonical MCP tool name for building an unsigned AgentGeyser transaction.
pub const TOOL_INVOKE_SKILL: &str = "invoke_skill";

/// Canonical MCP tool description for `invoke_skill` (mirrored in docs).
pub const TOOL_INVOKE_SKILL_DESCRIPTION: &str =
    "Build an unsigned AgentGeyser transaction for a given skill";

/// MCP server handler for AgentGeyser. Pure translator: MCP ↔ proxy JSON-RPC.
/// Holds NO key material; the unsigned transaction bytes returned by the
/// proxy are forwarded verbatim to the MCP client.
#[derive(Debug, Clone)]
pub struct AgentGeyserMcpServer {
    /// HTTP base URL of the agentGeyser proxy (e.g. `http://127.0.0.1:8999`).
    pub proxy_url: String,
    /// Reusable `reqwest` client for JSON-RPC forwarding.
    http: reqwest::Client,
}

impl AgentGeyserMcpServer {
    /// Construct a new server pointed at the given proxy base URL.
    pub fn new(proxy_url: impl Into<String>) -> Self {
        Self {
            proxy_url: proxy_url.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Construct a server using `AGENTGEYSER_PROXY_URL` if set, else the default.
    pub fn from_env() -> Self {
        let url = std::env::var("AGENTGEYSER_PROXY_URL")
            .unwrap_or_else(|_| DEFAULT_PROXY_URL.to_string());
        Self::new(url)
    }

    /// Build the `Tool` descriptor for `list_skills`. Empty input schema —
    /// the tool takes no arguments. Kept as a free-standing helper so tests
    /// can assert the exact shape without spinning up a service.
    pub fn list_skills_tool() -> Tool {
        // F2.2: inputSchema = {"type":"object","properties":{}}
        let schema: JsonObject = json!({
            "type": "object",
            "properties": {},
        })
        .as_object()
        .cloned()
        .expect("object literal is an object");
        Tool::new(
            TOOL_LIST_SKILLS,
            TOOL_LIST_SKILLS_DESCRIPTION,
            Arc::new(schema),
        )
    }

    /// Forward an MCP `tools/call list_skills` invocation to the proxy's
    /// `ag_listSkills` JSON-RPC method and pack the result into a
    /// [`CallToolResult`]. On any transport / JSON-RPC error, returns a
    /// `CallToolResult { is_error: Some(true), .. }` instead of panicking
    /// (F2.5).
    pub async fn handle_list_skills(&self) -> CallToolResult {
        match proxy_client::call(&self.http, &self.proxy_url, "ag_listSkills", json!({})).await
        {
            Ok(skills) => {
                // Packs the `result` array into a text content item
                // containing the JSON-serialized skill list (F2.3).
                let text = serde_json::to_string(&skills)
                    .unwrap_or_else(|e| format!("\"failed to serialize skills: {e}\""));
                CallToolResult::success(vec![Content::text(text)])
            }
            Err(err) => error_result(&err),
        }
    }

    /// Build the `Tool` descriptor for `invoke_skill`. Input schema documents
    /// the four required fields forwarded to proxy `ag_invokeSkill`
    /// (F3.1).
    pub fn invoke_skill_tool() -> Tool {
        let schema: JsonObject = json!({
            "type": "object",
            "required": ["skill_id", "args", "accounts", "payer"],
            "properties": {
                "skill_id": {"type": "string"},
                "args": {"type": "object"},
                "accounts": {"type": "object"},
                "payer": {"type": "string"},
            },
        })
        .as_object()
        .cloned()
        .expect("object literal is an object");
        Tool::new(
            TOOL_INVOKE_SKILL,
            TOOL_INVOKE_SKILL_DESCRIPTION,
            Arc::new(schema),
        )
    }

    /// Forward an MCP `tools/call invoke_skill` invocation to the proxy's
    /// `ag_invokeSkill` JSON-RPC method. The resulting `CallToolResult`
    /// carries a single text-content item whose body is JSON of the form
    /// `{"transaction_base64":"<b64>"}` (F3.2).
    ///
    /// Validates that `skill_id` is present and a string before calling the
    /// proxy; missing / non-string arguments return
    /// `CallToolResult { is_error: Some(true), .. }` with a clear message
    /// (F3.4) — never a panic.
    pub async fn handle_invoke_skill(&self, args: Option<JsonObject>) -> CallToolResult {
        let args = args.unwrap_or_default();
        if !args.get("skill_id").is_some_and(|v| v.is_string()) {
            return CallToolResult::error(vec![Content::text(
                "invoke_skill: missing or non-string required argument `skill_id`"
                    .to_string(),
            )]);
        }
        let params = serde_json::Value::Object(args);
        match proxy_client::call(&self.http, &self.proxy_url, "ag_invokeSkill", params).await {
            Ok(result) => {
                let b64 = result
                    .get("transaction_base64")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let text = json!({ "transaction_base64": b64 }).to_string();
                CallToolResult::success(vec![Content::text(text)])
            }
            Err(err) => error_result(&err),
        }
    }
}

/// Build an MCP-side error result from a [`ProxyError`]. Kept tiny so both
/// `list_skills` and the forthcoming `invoke_skill` handler can reuse it.
fn error_result(err: &ProxyError) -> CallToolResult {
    CallToolResult::error(vec![Content::text(format!("AgentGeyser error: {err}"))])
}

impl ServerHandler for AgentGeyserMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "agentgeyser-mcp-server",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "AgentGeyser MCP server: discover and invoke unsigned Solana \
                 transactions via the AgentGeyser proxy. Non-custodial."
                    .to_string(),
            )
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<
        Output = Result<ListToolsResult, rmcp::ErrorData>,
    > + Send
           + '_ {
        let tools = vec![Self::list_skills_tool(), Self::invoke_skill_tool()];
        async move {
            let mut result = ListToolsResult::default();
            result.tools = tools;
            Ok(result)
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<
        Output = Result<CallToolResult, rmcp::ErrorData>,
    > + Send
           + '_ {
        async move {
            match request.name.as_ref() {
                TOOL_LIST_SKILLS => Ok(self.handle_list_skills().await),
                TOOL_INVOKE_SKILL => Ok(self.handle_invoke_skill(request.arguments).await),
                other => Ok(CallToolResult::error(vec![Content::text(format!(
                    "unknown tool: {other}"
                ))])),
            }
        }
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        match name {
            TOOL_LIST_SKILLS => Some(Self::list_skills_tool()),
            TOOL_INVOKE_SKILL => Some(Self::invoke_skill_tool()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::{ClientHandler, ServiceExt};

    #[derive(Default, Clone)]
    struct EmptyClient;
    impl ClientHandler for EmptyClient {}

    /// F1.4: drive an MCP `initialize` request over an in-memory duplex pair
    /// against [`AgentGeyserMcpServer`] and assert that the resulting
    /// `InitializeResult` advertises the `tools` capability and that exactly
    /// one tool (`list_skills`) is registered after F2.
    #[tokio::test]
    async fn mcp_initialize_handshake_roundtrip() -> anyhow::Result<()> {
        let (server_io, client_io) = tokio::io::duplex(4096);

        let server_handle = tokio::spawn(async move {
            let server = AgentGeyserMcpServer::new(DEFAULT_PROXY_URL)
                .serve(server_io)
                .await?;
            server.waiting().await?;
            anyhow::Ok(())
        });

        let client = EmptyClient::default().serve(client_io).await?;
        let info = client
            .peer_info()
            .cloned()
            .expect("peer_info populated after initialize");

        assert!(
            info.capabilities.tools.is_some(),
            "tools capability missing: {:?}",
            info.capabilities
        );
        let tools = client.list_tools(Default::default()).await?;
        // After F3 we expose exactly two tools: list_skills, invoke_skill.
        assert_eq!(tools.tools.len(), 2, "expected 2 tools, got {tools:?}");
        let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&TOOL_LIST_SKILLS));
        assert!(names.contains(&TOOL_INVOKE_SKILL));
        assert_eq!(info.server_info.name, "agentgeyser-mcp-server");

        client.cancel().await?;
        server_handle.await??;
        Ok(())
    }

    /// F2.2 shape check: tool advertises empty object schema + expected
    /// description.
    #[test]
    fn list_skills_tool_shape() {
        let t = AgentGeyserMcpServer::list_skills_tool();
        assert_eq!(t.name, TOOL_LIST_SKILLS);
        assert_eq!(
            t.description.as_deref(),
            Some(TOOL_LIST_SKILLS_DESCRIPTION)
        );
        let schema = serde_json::Value::Object((*t.input_schema).clone());
        assert_eq!(
            schema,
            json!({"type": "object", "properties": {}}),
            "inputSchema must be empty object schema"
        );
    }

    /// F2.4 happy path: spins up a `wiremock` server that answers
    /// `ag_listSkills` with a canned skill list; asserts the
    /// `CallToolResult` text content contains that skill's name.
    #[tokio::test]
    async fn list_skills_forwards_to_proxy() -> anyhow::Result<()> {
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock = MockServer::start().await;
        let canned = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": [
                {"skill_id": "spl-token::transfer", "program_id": "Tokenkeg..."}
            ]
        });
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_partial_json(json!({"method": "ag_listSkills"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(canned))
            .expect(1)
            .mount(&mock)
            .await;

        let srv = AgentGeyserMcpServer::new(mock.uri());
        let result = srv.handle_list_skills().await;
        assert_eq!(result.is_error, Some(false), "unexpected error: {result:?}");
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.clone())
            .expect("text content present");
        assert!(
            text.contains("spl-token::transfer"),
            "expected canned skill name in content: {text}"
        );
        Ok(())
    }

    /// F2.5 error path: proxy returns a JSON-RPC error envelope; handler
    /// must return `is_error: true` with the error message, not panic.
    #[tokio::test]
    async fn list_skills_maps_error_to_is_error_true() -> anyhow::Result<()> {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock = MockServer::start().await;
        let canned = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {"code": -32000, "message": "registry unavailable"}
        });
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(canned))
            .mount(&mock)
            .await;

        let srv = AgentGeyserMcpServer::new(mock.uri());
        let result = srv.handle_list_skills().await;
        assert_eq!(
            result.is_error,
            Some(true),
            "proxy JSON-RPC error must surface as is_error=true"
        );
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.clone())
            .expect("error text present");
        assert!(
            text.contains("registry unavailable"),
            "error message missing from content: {text}"
        );
        Ok(())
    }

    /// F3.3 happy path: mock proxy returns `{transaction_base64:"AQID"}`;
    /// asserts MCP `CallToolResult` text content parses to JSON with
    /// `transaction_base64 == "AQID"`.
    #[tokio::test]
    async fn invoke_skill_returns_transaction_base64() -> anyhow::Result<()> {
        use wiremock::matchers::{body_partial_json, method};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock = MockServer::start().await;
        let canned = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"transaction_base64": "AQID"}
        });
        Mock::given(method("POST"))
            .and(body_partial_json(json!({"method": "ag_invokeSkill"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(canned))
            .expect(1)
            .mount(&mock)
            .await;

        let srv = AgentGeyserMcpServer::new(mock.uri());
        let args = json!({
            "skill_id": "spl-token::transfer",
            "args": {"amount": 1},
            "accounts": {},
            "payer": "11111111111111111111111111111111",
        })
        .as_object()
        .cloned();
        let result = srv.handle_invoke_skill(args).await;
        assert_eq!(result.is_error, Some(false), "unexpected error: {result:?}");
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.clone())
            .expect("text content present");
        let parsed: serde_json::Value = serde_json::from_str(&text)?;
        assert_eq!(parsed["transaction_base64"], "AQID");
        Ok(())
    }

    /// F3.4 error path: missing `skill_id` must return a structured error,
    /// not a panic — no HTTP call issued.
    #[tokio::test]
    async fn invoke_skill_missing_skill_id_is_error() {
        let srv = AgentGeyserMcpServer::new("http://127.0.0.1:1"); // unreachable; must not be hit
        let args = json!({
            "args": {"amount": 1},
            "accounts": {},
            "payer": "11111111111111111111111111111111",
        })
        .as_object()
        .cloned();
        let result = srv.handle_invoke_skill(args).await;
        assert_eq!(result.is_error, Some(true));
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.clone())
            .expect("error text present");
        assert!(
            text.contains("skill_id"),
            "error message must mention skill_id: {text}"
        );
    }
}
