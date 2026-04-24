use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod anthropic;
pub mod mock;
pub mod openai;

pub use anthropic::{AnthropicFlavor, AnthropicMessagesProvider};
pub use mock::MockProvider;
pub use openai::OpenAiProvider;

/// Provider-agnostic interface for turning natural language into skill plans.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Produce a structured plan for the supplied natural-language prompt.
    async fn plan(&self, prompt: &str) -> Result<Plan, PlanError>;
}

/// Structured output consumed by later AgentGeyser planning and RPC layers.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Plan {
    pub skill_id: String,
    pub args: serde_json::Value,
    pub rationale: String,
}

/// Error taxonomy shared by planner providers.
#[derive(Debug, Error)]
pub enum PlanError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("deserialize error: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("rate limited: {0}")]
    RateLimited(String),
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
}

/// Which provider kind `provider_from_env` selected. Exposed on
/// [`ProviderHandle`] so callers (e.g. live tests) can introspect without
/// downcasting through the `LlmProvider` trait object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    Anthropic,
    KimiCoding,
    OpenAi,
    /// Deterministic in-process fixture provider — used as the auto fallback
    /// when no API-key env var is set so unit tests / offline dev still work.
    Mock,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::KimiCoding => "kimi-coding",
            ProviderKind::OpenAi => "openai",
            ProviderKind::Mock => "mock",
        }
    }
}

/// Pure selector used by [`provider_from_env`] — kept separate so the env
/// precedence can be unit-tested without touching process env vars.
pub fn select_provider_kind(
    has_anthropic: bool,
    has_kimi: bool,
    has_openai: bool,
) -> Option<ProviderKind> {
    if has_anthropic {
        Some(ProviderKind::Anthropic)
    } else if has_kimi {
        Some(ProviderKind::KimiCoding)
    } else if has_openai {
        Some(ProviderKind::OpenAi)
    } else {
        None
    }
}

/// Wrapper around a boxed [`LlmProvider`] that retains the selected provider
/// kind + default model for diagnostics.
pub struct ProviderHandle {
    kind: ProviderKind,
    default_model: String,
    inner: Box<dyn LlmProvider>,
}

impl ProviderHandle {
    pub fn kind(&self) -> ProviderKind {
        self.kind
    }

    pub fn name(&self) -> &'static str {
        self.kind.as_str()
    }

    pub fn default_model(&self) -> &str {
        &self.default_model
    }
}

#[async_trait]
impl LlmProvider for ProviderHandle {
    async fn plan(&self, prompt: &str) -> Result<Plan, PlanError> {
        self.inner.plan(prompt).await
    }
}

/// Auto-select an LLM provider from the environment. Precedence:
/// 1. `ANTHROPIC_API_KEY` — vanilla Anthropic Messages.
/// 2. `KIMI_API_KEY` — Kimi-for-coding via Anthropic Messages.
/// 3. `OPENAI_API_KEY` — OpenAI Chat Completions.
///
/// The key values themselves are never echoed or logged.
pub fn provider_from_env() -> Result<ProviderHandle, PlanError> {
    let anthropic = std::env::var("ANTHROPIC_API_KEY").ok();
    let kimi = std::env::var("KIMI_API_KEY").ok();
    let openai = std::env::var("OPENAI_API_KEY").ok();

    match select_provider_kind(anthropic.is_some(), kimi.is_some(), openai.is_some()) {
        Some(ProviderKind::Anthropic) => {
            let key = anthropic.expect("env var checked above");
            let provider = AnthropicMessagesProvider::anthropic_default(key);
            let default_model = provider.default_model().to_owned();
            Ok(ProviderHandle {
                kind: ProviderKind::Anthropic,
                default_model,
                inner: Box::new(provider),
            })
        }
        Some(ProviderKind::KimiCoding) => {
            let key = kimi.expect("env var checked above");
            let provider = AnthropicMessagesProvider::kimi_coding_default(key);
            let default_model = provider.default_model().to_owned();
            Ok(ProviderHandle {
                kind: ProviderKind::KimiCoding,
                default_model,
                inner: Box::new(provider),
            })
        }
        Some(ProviderKind::OpenAi) => {
            let key = openai.expect("env var checked above");
            let provider = OpenAiProvider::new(key);
            let default_model = provider.default_model().to_owned();
            Ok(ProviderHandle {
                kind: ProviderKind::OpenAi,
                default_model,
                inner: Box::new(provider),
            })
        }
        None | Some(ProviderKind::Mock) => {
            // L26 — no API key found; fall back to the deterministic MockProvider
            // so offline dev / unit tests can still call `provider_from_env()`.
            // `select_provider_kind` never returns `Mock`, but the arm keeps
            // the match exhaustive without a wildcard.
            let provider = MockProvider::new_deterministic();
            Ok(ProviderHandle {
                kind: ProviderKind::Mock,
                default_model: "mock-deterministic".to_owned(),
                inner: Box::new(provider),
            })
        }
    }
}

#[cfg(test)]
mod selector_tests {
    use super::*;

    #[test]
    fn anthropic_wins_over_all() {
        assert_eq!(
            select_provider_kind(true, true, true),
            Some(ProviderKind::Anthropic)
        );
        assert_eq!(
            select_provider_kind(true, false, false),
            Some(ProviderKind::Anthropic)
        );
    }

    #[test]
    fn kimi_wins_over_openai() {
        assert_eq!(
            select_provider_kind(false, true, true),
            Some(ProviderKind::KimiCoding)
        );
    }

    #[test]
    fn openai_fallback() {
        assert_eq!(
            select_provider_kind(false, false, true),
            Some(ProviderKind::OpenAi)
        );
    }

    #[test]
    fn none_when_no_key() {
        assert_eq!(select_provider_kind(false, false, false), None);
    }

    #[tokio::test]
    async fn provider_from_env_falls_back_to_mock_when_no_keys() {
        // Only assert when the ambient test env has no provider keys; otherwise
        // this test is a non-op so shared-env developer boxes still pass.
        if std::env::var("ANTHROPIC_API_KEY").is_ok()
            || std::env::var("KIMI_API_KEY").is_ok()
            || std::env::var("OPENAI_API_KEY").is_ok()
        {
            return;
        }
        let handle = provider_from_env().expect("mock fallback must succeed");
        assert_eq!(handle.kind(), ProviderKind::Mock);
        assert_eq!(handle.name(), "mock");
        let plan = handle.plan("transfer 0.01 USDC to alice").await.unwrap();
        assert_eq!(plan.skill_id, "spl-token::transfer");
    }
}
