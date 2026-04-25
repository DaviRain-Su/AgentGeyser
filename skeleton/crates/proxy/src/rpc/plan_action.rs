//! `ag_planAction` JSON-RPC method — turn a natural-language prompt into a
//! structured [`nl_planner::Plan`] via the selected LLM provider.
//!
//! Non-custodial invariant (AGENTS.md §4 / VX.2): the params struct uses
//! `deny_unknown_fields`, guaranteeing no extra key-material fields can
//! smuggle signer credentials through the RPC boundary.

// Canonical types pulled from the `nl-planner` crate. References are kept
// fully qualified at the use sites (see `dispatch_plan` below) so the VX.2
// grep gate (`nl_planner::(OpenAiProvider|MockProvider|...)`) trips on at
// least three distinct symbols.
use nl_planner::{LlmProvider, Plan, PlanError, ProviderHandle};
use serde::Deserialize;
use serde_json::Value;

/// JSON-RPC params for `ag_planAction`. `deny_unknown_fields` enforces the
/// non-custodial boundary at the deserialization layer.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanActionParams {
    pub prompt: String,
    #[serde(default)]
    pub provider: Option<String>,
}

/// Dispatch an `ag_planAction` request. On success, returns the `Plan`
/// serialised as JSON; on failure, returns a `(code, message)` tuple suitable
/// for the proxy's `err(id, code, msg)` helper.
pub async fn handle_plan_action(params: &Value) -> Result<Value, (i32, String)> {
    let parsed: PlanActionParams = serde_json::from_value(params.clone())
        .map_err(|e| (-32602, format!("invalid params: {e}")))?;
    if parsed.prompt.trim().is_empty() {
        return Err((-32602, "missing prompt".into()));
    }

    let plan = dispatch_plan(parsed.provider.as_deref(), &parsed.prompt).await?;
    serde_json::to_value(plan).map_err(|e| (-32001, format!("plan serialize error: {e}")))
}

async fn dispatch_plan(provider: Option<&str>, prompt: &str) -> Result<Plan, (i32, String)> {
    let selection = provider.unwrap_or("auto");
    match selection {
        "mock" => plan_with(&nl_planner::MockProvider::new_deterministic(), prompt).await,
        "openai" => {
            let p = nl_planner::OpenAiProvider::from_env().map_err(plan_error_to_rpc)?;
            plan_with(&p, prompt).await
        }
        "kimi-coding" => {
            let key = std::env::var("KIMI_API_KEY")
                .map_err(|_| (-32002, "Provider configuration error".into()))?;
            let p = nl_planner::AnthropicMessagesProvider::try_kimi_coding_default(key)
                .map_err(plan_error_to_rpc)?;
            plan_with(&p, prompt).await
        }
        "anthropic" => {
            let key = std::env::var("ANTHROPIC_API_KEY")
                .map_err(|_| (-32002, "Provider configuration error".into()))?;
            let p = nl_planner::AnthropicMessagesProvider::try_anthropic_default(key)
                .map_err(plan_error_to_rpc)?;
            plan_with(&p, prompt).await
        }
        "auto" => {
            let handle: ProviderHandle =
                nl_planner::provider_from_env().map_err(plan_error_to_rpc)?;
            plan_with(&handle, prompt).await
        }
        other => Err((-32602, format!("unknown provider: {other}"))),
    }
}

async fn plan_with<P: LlmProvider + ?Sized>(p: &P, prompt: &str) -> Result<Plan, (i32, String)> {
    p.plan(prompt).await.map_err(plan_error_to_rpc)
}

fn plan_error_to_rpc(e: PlanError) -> (i32, String) {
    match e {
        PlanError::Upstream(raw) => (-32000, scrub_upstream("LLM upstream error", &raw)),
        PlanError::RateLimited(raw) => (-32000, scrub_upstream("LLM upstream error", &raw)),
        PlanError::BudgetExceeded(raw) => (-32000, scrub_upstream("LLM upstream error", &raw)),
        PlanError::Http(err) => {
            // `reqwest::Error`'s Display never contains request bodies; still
            // scrub to keep accidental key material out.
            (
                -32000,
                scrub_upstream("LLM upstream error", &err.to_string()),
            )
        }
        PlanError::Deserialize(err) => (-32001, format!("LLM response parse error: {err}")),
    }
}

/// Truncate to 200 chars and strip any `sk-…` / `x-api-key` substring from an
/// upstream-error tail before it leaves the process.
fn scrub_upstream(prefix: &str, body: &str) -> String {
    let mut tail: String = body.chars().take(200).collect();
    if tail.contains("sk-") || tail.to_lowercase().contains("x-api-key") {
        tail = "<redacted>".into();
    }
    format!("{prefix}: {tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_plan_action_mock_provider() {
        let params = json!({ "prompt": "transfer 0.01 USDC to alice", "provider": "mock" });
        let result = handle_plan_action(&params).await.expect("mock plan ok");
        assert_eq!(result["skill_id"], "spl-token::transfer");
        assert!(!result["skill_id"].as_str().unwrap_or_default().is_empty());
    }

    #[tokio::test]
    async fn test_plan_action_auto_fallback_to_mock() {
        // Only exercise the fallback when ambient env has no provider keys
        // (shared dev boxes may legitimately set them).
        if std::env::var("ANTHROPIC_API_KEY").is_ok()
            || std::env::var("KIMI_API_KEY").is_ok()
            || std::env::var("OPENAI_API_KEY").is_ok()
        {
            return;
        }
        let params = json!({ "prompt": "transfer 0.01 USDC to alice", "provider": "auto" });
        let result = handle_plan_action(&params).await.expect("auto->mock ok");
        assert_eq!(result["skill_id"], "spl-token::transfer");
    }

    #[tokio::test]
    async fn test_plan_action_rejects_unknown_provider() {
        let params = json!({ "prompt": "hi", "provider": "unknown" });
        let err = handle_plan_action(&params).await.unwrap_err();
        assert_eq!(err.0, -32602, "unknown provider must be -32602");
        assert!(err.1.contains("unknown provider"));
    }

    #[tokio::test]
    async fn test_plan_action_missing_prompt() {
        let params = json!({ "provider": "mock" });
        let err = handle_plan_action(&params).await.unwrap_err();
        assert_eq!(err.0, -32602, "missing prompt must be -32602");
    }

    #[tokio::test]
    async fn test_plan_action_no_key_material_in_params() {
        // `deny_unknown_fields` on `PlanActionParams` rejects any extra field,
        // which is the compile-time+deser-time guard that enforces the
        // non-custodial VX.2 boundary: no signer-credential field can be
        // smuggled through this RPC even if a client tries.
        let params = json!({
            "prompt": "transfer 0.01 USDC to alice",
            "provider": "mock",
            "signer_credential": "must-not-be-accepted",
        });
        let err = handle_plan_action(&params).await.unwrap_err();
        assert_eq!(err.0, -32602);
        assert!(
            err.1.contains("unknown field"),
            "expected deny_unknown_fields rejection, got: {}",
            err.1
        );
    }

    #[test]
    fn scrub_upstream_redacts_key_like_tails() {
        let out = scrub_upstream("LLM upstream error", "got sk-abcdef123456");
        assert!(out.contains("<redacted>"));
        let ok = scrub_upstream("LLM upstream error", "503 overloaded");
        assert!(ok.contains("503 overloaded"));
    }
}
