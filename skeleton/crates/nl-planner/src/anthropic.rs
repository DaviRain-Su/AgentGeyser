//! F17 — Anthropic Messages API provider.
//!
//! Supports both vanilla Anthropic (`api.anthropic.com`) and the
//! Kimi-for-coding endpoint (`api.kimi.com/coding`). Structured output is
//! obtained through Anthropic's native `tool_use` mechanism (the Messages API
//! does not support OpenAI-style `response_format`).

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::{LlmProvider, Plan, PlanError};

const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_DEFAULT_MODEL: &str = "claude-3-5-sonnet-latest";
const ANTHROPIC_VERSION: &str = "2023-06-01";

const KIMI_CODING_BASE_URL: &str = "https://api.kimi.com/coding";
const KIMI_CODING_DEFAULT_MODEL: &str = "kimi-for-coding";
const KIMI_CODING_USER_AGENT: &str = "KimiCLI/1.5";

const MAX_TOKENS: u64 = 1024;
const UPSTREAM_BODY_TRUNCATE: usize = 512;
const TOOL_NAME: &str = "submit_plan";

/// Which flavor of the Anthropic Messages endpoint we are talking to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnthropicFlavor {
    /// Vanilla Anthropic cloud endpoint.
    Anthropic,
    /// Kimi-for-coding endpoint (shares the Messages API wire format).
    KimiCoding,
}

impl AnthropicFlavor {
    fn provider_name(self) -> &'static str {
        match self {
            AnthropicFlavor::Anthropic => "anthropic",
            AnthropicFlavor::KimiCoding => "kimi-coding",
        }
    }
}

/// Anthropic Messages API provider.
pub struct AnthropicMessagesProvider {
    api_key: String,
    base_url: String,
    default_model: String,
    user_agent: Option<String>,
    flavor: AnthropicFlavor,
    client: reqwest::Client,
}

impl AnthropicMessagesProvider {
    /// Construct a provider pointed at the vanilla Anthropic cloud endpoint.
    pub fn anthropic_default(api_key: String) -> Self {
        Self::new(
            api_key,
            ANTHROPIC_BASE_URL.to_owned(),
            ANTHROPIC_DEFAULT_MODEL.to_owned(),
            None,
            AnthropicFlavor::Anthropic,
        )
    }

    /// Construct a provider pointed at the Kimi-for-coding endpoint.
    pub fn kimi_coding_default(api_key: String) -> Self {
        Self::new(
            api_key,
            KIMI_CODING_BASE_URL.to_owned(),
            KIMI_CODING_DEFAULT_MODEL.to_owned(),
            Some(KIMI_CODING_USER_AGENT.to_owned()),
            AnthropicFlavor::KimiCoding,
        )
    }

    /// Fully parameterised constructor — primarily used by tests and custom
    /// deployments.
    pub fn new(
        api_key: String,
        base_url: String,
        default_model: String,
        user_agent: Option<String>,
        flavor: AnthropicFlavor,
    ) -> Self {
        Self {
            api_key,
            base_url,
            default_model,
            user_agent,
            flavor,
            client: reqwest::Client::new(),
        }
    }

    pub fn provider_name(&self) -> &'static str {
        self.flavor.provider_name()
    }

    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn flavor(&self) -> AnthropicFlavor {
        self.flavor
    }

    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url.trim_end_matches('/'))
    }

    fn request_body(&self, prompt: &str) -> serde_json::Value {
        json!({
            "model": self.default_model,
            "max_tokens": MAX_TOKENS,
            "system": "You are a Solana action planner. Use the submit_plan tool to return your structured plan.",
            "tools": [{
                "name": TOOL_NAME,
                "description": "Submit the structured action plan derived from the user prompt.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "skill_id": {
                            "type": "string",
                            "description": "Canonical skill identifier, e.g. spl-token::transfer."
                        },
                        "args": {
                            "type": "object",
                            "additionalProperties": true,
                            "description": "Structured arguments for the selected skill."
                        },
                        "rationale": {
                            "type": "string",
                            "description": "Short natural-language justification for this plan."
                        }
                    },
                    "required": ["skill_id", "args", "rationale"]
                }
            }],
            "tool_choice": {"type": "tool", "name": TOOL_NAME},
            "messages": [{"role": "user", "content": prompt}]
        })
    }

    async fn plan_at(&self, prompt: &str, url: &str) -> Result<Plan, PlanError> {
        let mut req = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json");
        if let Some(ua) = &self.user_agent {
            req = req.header("User-Agent", ua);
        }

        let response = req
            .json(&self.request_body(prompt))
            .send()
            .await
            .map_err(PlanError::Http)?;
        let status = response.status();
        let body = response.text().await.map_err(PlanError::Http)?;
        if !status.is_success() {
            let truncated: String = body.chars().take(UPSTREAM_BODY_TRUNCATE).collect();
            return Err(PlanError::Upstream(format!("status {status}: {truncated}")));
        }

        let parsed: MessagesResponse =
            serde_json::from_str(&body).map_err(PlanError::Deserialize)?;
        let tool_input = parsed
            .content
            .into_iter()
            .find_map(|block| match block {
                ContentBlock::ToolUse { name, input } if name == TOOL_NAME => Some(input),
                _ => None,
            })
            .ok_or_else(|| {
                PlanError::Upstream(format!(
                    "no {TOOL_NAME} tool_use block in Anthropic response"
                ))
            })?;

        serde_json::from_value(tool_input).map_err(PlanError::Deserialize)
    }
}

#[async_trait]
impl LlmProvider for AnthropicMessagesProvider {
    async fn plan(&self, prompt: &str) -> Result<Plan, PlanError> {
        let url = self.messages_url();
        self.plan_at(prompt, &url).await
    }
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    ToolUse {
        name: String,
        input: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;
    use serde_json::json;

    fn tool_use_body() -> String {
        json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-5-sonnet-latest",
            "stop_reason": "tool_use",
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_1",
                    "name": TOOL_NAME,
                    "input": {
                        "skill_id": "spl-token::transfer",
                        "args": { "amount": "1" },
                        "rationale": "unit test"
                    }
                }
            ],
            "usage": { "input_tokens": 10, "output_tokens": 20 }
        })
        .to_string()
    }

    #[tokio::test]
    async fn anthropic_happy_path_parses_tool_use() {
        let mut srv = mockito::Server::new_async().await;
        let mock = srv
            .mock("POST", "/v1/messages")
            .match_header("x-api-key", "test-anthropic-key")
            .match_header("anthropic-version", ANTHROPIC_VERSION)
            .match_body(Matcher::PartialJson(json!({
                "tool_choice": {"type": "tool", "name": TOOL_NAME}
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(tool_use_body())
            .create_async()
            .await;

        let provider = AnthropicMessagesProvider::new(
            "test-anthropic-key".to_owned(),
            srv.url(),
            "claude-3-5-sonnet-latest".to_owned(),
            None,
            AnthropicFlavor::Anthropic,
        );
        let plan = provider
            .plan("transfer one token")
            .await
            .expect("plan succeeds");
        assert_eq!(plan.skill_id, "spl-token::transfer");
        assert_eq!(plan.args["amount"], "1");
        assert_eq!(plan.rationale, "unit test");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn kimi_coding_sends_user_agent_and_api_key() {
        let mut srv = mockito::Server::new_async().await;
        let mock = srv
            .mock("POST", "/v1/messages")
            .match_header("x-api-key", "test-kimi-key")
            .match_header("user-agent", KIMI_CODING_USER_AGENT)
            .match_header("anthropic-version", ANTHROPIC_VERSION)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(tool_use_body())
            .create_async()
            .await;

        let provider = AnthropicMessagesProvider::new(
            "test-kimi-key".to_owned(),
            srv.url(),
            KIMI_CODING_DEFAULT_MODEL.to_owned(),
            Some(KIMI_CODING_USER_AGENT.to_owned()),
            AnthropicFlavor::KimiCoding,
        );
        assert_eq!(provider.provider_name(), "kimi-coding");
        let plan = provider.plan("transfer one token").await.unwrap();
        assert_eq!(plan.skill_id, "spl-token::transfer");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn missing_tool_use_maps_to_upstream() {
        let mut srv = mockito::Server::new_async().await;
        srv.mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "content": [{"type": "text", "text": "hi"}]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let provider = AnthropicMessagesProvider::new(
            "k".to_owned(),
            srv.url(),
            "m".to_owned(),
            None,
            AnthropicFlavor::Anthropic,
        );
        let err = provider.plan("x").await.unwrap_err();
        match err {
            PlanError::Upstream(msg) => assert!(msg.contains("tool_use")),
            other => panic!("expected Upstream, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn non_2xx_maps_to_upstream_truncated() {
        let mut srv = mockito::Server::new_async().await;
        // body longer than UPSTREAM_BODY_TRUNCATE chars.
        let big_body = "x".repeat(2000);
        srv.mock("POST", "/v1/messages")
            .with_status(403)
            .with_body(big_body)
            .create_async()
            .await;

        let provider = AnthropicMessagesProvider::new(
            "k".to_owned(),
            srv.url(),
            "m".to_owned(),
            None,
            AnthropicFlavor::Anthropic,
        );
        let err = provider.plan("x").await.unwrap_err();
        match err {
            PlanError::Upstream(msg) => {
                assert!(msg.contains("403"));
                // "status 403: " prefix (~12 chars) plus 512 truncated body chars.
                assert!(msg.len() < 600, "body was not truncated: len={}", msg.len());
            }
            other => panic!("expected Upstream, got {other:?}"),
        }
    }

    #[test]
    fn constructors_set_expected_defaults() {
        let a = AnthropicMessagesProvider::anthropic_default("k".to_owned());
        assert_eq!(a.provider_name(), "anthropic");
        assert_eq!(a.base_url(), ANTHROPIC_BASE_URL);
        assert_eq!(a.default_model(), ANTHROPIC_DEFAULT_MODEL);
        assert!(a.user_agent.is_none());

        let k = AnthropicMessagesProvider::kimi_coding_default("k".to_owned());
        assert_eq!(k.provider_name(), "kimi-coding");
        assert_eq!(k.base_url(), KIMI_CODING_BASE_URL);
        assert_eq!(k.default_model(), KIMI_CODING_DEFAULT_MODEL);
        assert_eq!(k.user_agent.as_deref(), Some(KIMI_CODING_USER_AGENT));
    }
}
