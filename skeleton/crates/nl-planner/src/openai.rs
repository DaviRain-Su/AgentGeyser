use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::{LlmProvider, Plan, PlanError};

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";
const TOKEN_BUDGET: u64 = 500;

/// OpenAI Chat Completions provider. Kept minimal (OpenAI proper) after F17
/// moved Kimi-for-coding onto its own Anthropic Messages provider.
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Construct from `OPENAI_API_KEY` with OpenAI's default endpoint.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: DEFAULT_MODEL.to_owned(),
            base_url: OPENAI_BASE_URL.to_owned(),
            client: reqwest::Client::new(),
        }
    }

    /// Read `OPENAI_API_KEY` from the environment and construct a default
    /// OpenAI provider. Only used as the last-resort fallback inside
    /// [`crate::provider_from_env`].
    pub fn from_env() -> Result<Self, PlanError> {
        let api_key = std::env::var(OPENAI_API_KEY_ENV)
            .map_err(|_| PlanError::Upstream(format!("{OPENAI_API_KEY_ENV} env var is not set")))?;
        Ok(Self::new(api_key))
    }

    pub fn provider_name(&self) -> &'static str {
        "openai"
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn default_model(&self) -> &str {
        &self.model
    }

    async fn plan_at(&self, prompt: &str, url: &str) -> Result<Plan, PlanError> {
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "system",
                        "content": "Return only JSON with skill_id, args, and rationale fields."
                    },
                    { "role": "user", "content": prompt }
                ],
                "response_format": { "type": "json_object" }
            }))
            .send()
            .await
            .map_err(PlanError::Http)?;

        let status = response.status();
        let body = response.text().await.map_err(PlanError::Http)?;
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(PlanError::RateLimited(body));
        }
        if !status.is_success() {
            return Err(PlanError::Upstream(format!("status {status}: {body}")));
        }

        let completion: ChatCompletionResponse =
            serde_json::from_str(&body).map_err(PlanError::Deserialize)?;
        if completion.usage.total_tokens >= TOKEN_BUDGET {
            return Err(PlanError::BudgetExceeded(format!(
                "total_tokens {} exceeds budget {TOKEN_BUDGET}",
                completion.usage.total_tokens
            )));
        }

        let content = completion
            .choices
            .first()
            .map(|choice| choice.message.content.as_str())
            .ok_or_else(|| PlanError::Upstream("missing chat completion choice".to_owned()))?;

        serde_json::from_str(content).map_err(PlanError::Deserialize)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn plan(&self, prompt: &str) -> Result<Plan, PlanError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        self.plan_at(prompt, &url).await
    }
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    usage: ChatUsage,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

#[derive(Deserialize)]
struct ChatUsage {
    total_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;
    use serde_json::json;

    fn provider() -> OpenAiProvider {
        OpenAiProvider {
            api_key: "test-key".to_owned(),
            model: "gpt-test".to_owned(),
            base_url: "https://api.openai.com/v1".to_owned(),
            client: reqwest::Client::new(),
        }
    }

    fn completion_body(total_tokens: u64) -> String {
        let plan = json!({
            "skill_id": "spl-token::transfer",
            "args": { "amount": "1" },
            "rationale": "parsed json plan"
        });

        json!({
            "choices": [{ "message": { "content": plan.to_string() } }],
            "usage": { "total_tokens": total_tokens }
        })
        .to_string()
    }

    #[tokio::test]
    async fn happy_path_parse() {
        let mut srv = mockito::Server::new_async().await;
        let mock = srv
            .mock("POST", "/v1/chat/completions")
            .match_header("Authorization", "Bearer test-key")
            .match_body(Matcher::PartialJson(json!({
                "model": "gpt-test",
                "response_format": { "type": "json_object" }
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(completion_body(42))
            .create_async()
            .await;

        let url = format!("{}/v1/chat/completions", srv.url());
        let plan = provider()
            .plan_at("transfer one token", &url)
            .await
            .unwrap();

        assert_eq!(plan.skill_id, "spl-token::transfer");
        assert_eq!(plan.args["amount"], "1");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn rate_limit_maps_to_rate_limited() {
        let mut srv = mockito::Server::new_async().await;
        srv.mock("POST", "/v1/chat/completions")
            .with_status(429)
            .with_body(r#"{"error":{"message":"slow down"}}"#)
            .create_async()
            .await;

        let url = format!("{}/v1/chat/completions", srv.url());
        let err = provider()
            .plan_at("transfer one token", &url)
            .await
            .unwrap_err();

        assert!(matches!(err, PlanError::RateLimited(_)));
    }

    #[tokio::test]
    async fn budget_guardrail() {
        let mut srv = mockito::Server::new_async().await;
        srv.mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(completion_body(600))
            .create_async()
            .await;

        let url = format!("{}/v1/chat/completions", srv.url());
        let err = provider()
            .plan_at("transfer one token", &url)
            .await
            .unwrap_err();

        assert!(matches!(err, PlanError::BudgetExceeded(_)));
    }
}
