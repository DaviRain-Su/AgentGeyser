//! F17 — integration coverage for AnthropicMessagesProvider + provider_from_env.
//! Unit coverage of the HTTP path already lives in `src/anthropic.rs`; this
//! module focuses on crate-public API surface (flavor constructors, the
//! `select_provider_kind` precedence helper, and trait-object usage).

use nl_planner::{
    select_provider_kind, AnthropicFlavor, AnthropicMessagesProvider, LlmProvider, Plan,
    ProviderKind,
};

#[test]
fn selector_respects_precedence() {
    assert_eq!(
        select_provider_kind(true, true, true),
        Some(ProviderKind::Anthropic)
    );
    assert_eq!(
        select_provider_kind(false, true, true),
        Some(ProviderKind::KimiCoding)
    );
    assert_eq!(
        select_provider_kind(false, false, true),
        Some(ProviderKind::OpenAi)
    );
    assert_eq!(select_provider_kind(false, false, false), None);
}

#[test]
fn provider_kind_as_str() {
    assert_eq!(ProviderKind::Anthropic.as_str(), "anthropic");
    assert_eq!(ProviderKind::KimiCoding.as_str(), "kimi-coding");
    assert_eq!(ProviderKind::OpenAi.as_str(), "openai");
}

#[test]
fn anthropic_default_and_kimi_default_expose_expected_metadata() {
    let a = AnthropicMessagesProvider::anthropic_default("k".to_owned());
    assert_eq!(a.provider_name(), "anthropic");
    assert_eq!(a.flavor(), AnthropicFlavor::Anthropic);
    assert!(a.base_url().starts_with("https://api.anthropic.com"));
    assert!(!a.default_model().is_empty());

    let k = AnthropicMessagesProvider::kimi_coding_default("k".to_owned());
    assert_eq!(k.provider_name(), "kimi-coding");
    assert_eq!(k.flavor(), AnthropicFlavor::KimiCoding);
    assert_eq!(k.base_url(), "https://api.kimi.com/coding");
    assert_eq!(k.default_model(), "kimi-for-coding");
}

#[test]
fn provider_is_object_safe() {
    fn take_trait(_: Box<dyn LlmProvider>) {}
    take_trait(Box::new(AnthropicMessagesProvider::anthropic_default(
        "k".to_owned(),
    )));
}

#[tokio::test]
async fn end_to_end_mock_flow_parses_tool_use() {
    let mut srv = mockito::Server::new_async().await;
    let body = serde_json::json!({
        "content": [{
            "type": "tool_use",
            "id": "toolu_1",
            "name": "submit_plan",
            "input": {
                "skill_id": "list_skills",
                "args": { "limit": 5 },
                "rationale": "e2e"
            }
        }]
    })
    .to_string();

    let mock = srv
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let provider = AnthropicMessagesProvider::new(
        "key".to_owned(),
        srv.url(),
        "claude-3-5-sonnet-latest".to_owned(),
        None,
        AnthropicFlavor::Anthropic,
    );
    let plan: Plan = provider.plan("list my skills").await.unwrap();
    assert_eq!(plan.skill_id, "list_skills");
    assert_eq!(plan.args["limit"], 5);
    mock.assert_async().await;
}
