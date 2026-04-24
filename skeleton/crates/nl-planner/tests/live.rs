//! F5 live test — opt-in (`--ignored`) probe against a real LLM provider.
//! Provider is auto-selected via [`nl_planner::provider_from_env`] with the
//! precedence `ANTHROPIC_API_KEY` > `KIMI_API_KEY` > `OPENAI_API_KEY`.
use nl_planner::{provider_from_env, LlmProvider};

#[tokio::test]
#[ignore]
async fn live_plan_transfer_prompt() {
    if std::env::var("ANTHROPIC_API_KEY").is_err()
        && std::env::var("KIMI_API_KEY").is_err()
        && std::env::var("OPENAI_API_KEY").is_err()
    {
        return;
    }
    let handle = provider_from_env().expect(
        "set ANTHROPIC_API_KEY, KIMI_API_KEY, or OPENAI_API_KEY to run this ignored live test",
    );
    assert!(!handle.default_model().is_empty());
    assert!(!handle.name().is_empty());
    let plan = handle
        .plan("transfer 1 USDC to <DST_OWNER>")
        .await
        .expect("live plan call should succeed");
    assert!(!plan.skill_id.is_empty(), "plan.skill_id must be non-empty");
}
