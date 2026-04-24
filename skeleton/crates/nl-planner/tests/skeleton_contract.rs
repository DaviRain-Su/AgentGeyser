use nl_planner::{LlmProvider, Plan, PlanError};

#[test]
fn plan_serializes_expected_contract_fields() {
    let plan = Plan {
        skill_id: "list_skills".to_owned(),
        args: serde_json::json!({ "limit": 3 }),
        rationale: "List available skills for the caller.".to_owned(),
    };

    let encoded = serde_json::to_value(&plan).expect("Plan serializes to JSON");

    assert_eq!(encoded["skill_id"], "list_skills");
    assert_eq!(encoded["args"]["limit"], 3);
    assert_eq!(
        encoded["rationale"],
        "List available skills for the caller."
    );
}

#[test]
fn plan_error_variants_are_public_and_displayable() {
    let errors = [
        PlanError::Upstream("provider failed".to_owned()).to_string(),
        PlanError::RateLimited("retry later".to_owned()).to_string(),
        PlanError::BudgetExceeded("token budget exceeded".to_owned()).to_string(),
    ];

    assert!(errors.iter().all(|message| !message.is_empty()));
}

#[test]
fn static_provider_implements_llm_provider_trait() {
    fn assert_provider<T: LlmProvider + Send + Sync>() {}

    assert_provider::<StaticProvider>();
}

struct StaticProvider;

#[async_trait::async_trait]
impl LlmProvider for StaticProvider {
    async fn plan(&self, prompt: &str) -> Result<Plan, PlanError> {
        Ok(Plan {
            skill_id: prompt.to_owned(),
            args: serde_json::json!({}),
            rationale: "static test provider".to_owned(),
        })
    }
}
