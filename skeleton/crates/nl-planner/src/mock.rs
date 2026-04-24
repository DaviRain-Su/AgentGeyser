use crate::{LlmProvider, Plan, PlanError};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

pub struct MockProvider {
    fixtures: HashMap<u64, Plan>,
}
impl MockProvider {
    pub fn new_deterministic() -> Self {
        let mut fixtures = HashMap::new();
        for (prompt, skill_id, args, rationale) in [
            (
                "transfer 0.01 USDC to alice",
                "spl-token::transfer",
                json!({ "amount": "0.01" }),
                "transfer fixture",
            ),
            (
                "list my skills",
                "list_skills",
                json!({}),
                "skill listing fixture",
            ),
            ("unknown", "unknown", json!({}), "fallback fixture"),
        ] {
            fixtures.insert(
                prompt_hash(prompt),
                Plan {
                    skill_id: skill_id.to_owned(),
                    args,
                    rationale: rationale.to_owned(),
                },
            );
        }
        Self { fixtures }
    }
}
#[async_trait]
impl LlmProvider for MockProvider {
    async fn plan(&self, prompt: &str) -> Result<Plan, PlanError> {
        self.fixtures
            .get(&prompt_hash(prompt))
            .cloned()
            .ok_or_else(|| PlanError::Upstream("no fixture".to_owned()))
    }
}
fn prompt_hash(prompt: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    prompt.hash(&mut hasher);
    hasher.finish()
}
#[cfg(test)]
mod tests {
    use super::MockProvider;
    use crate::{LlmProvider, PlanError};
    #[tokio::test]
    async fn deterministic_transfer() {
        let provider = MockProvider::new_deterministic();
        let first = provider.plan("transfer 0.01 USDC to alice").await.unwrap();
        let second = provider.plan("transfer 0.01 USDC to alice").await.unwrap();
        assert_eq!(first, second);
        assert_eq!(first.skill_id, "spl-token::transfer");
        assert_eq!(first.args["amount"], "0.01");
    }
    #[tokio::test]
    async fn deterministic_list_skills() {
        let provider = MockProvider::new_deterministic();
        let plan = provider.plan("list my skills").await.unwrap();
        assert_eq!(plan.skill_id, "list_skills");
        assert_eq!(plan.args, serde_json::json!({}));
    }
    #[tokio::test]
    async fn unknown_prompt_returns_upstream() {
        let provider = MockProvider::new_deterministic();
        let err = provider.plan("missing fixture").await.unwrap_err();
        assert!(matches!(err, PlanError::Upstream(message) if message == "no fixture"));
    }
}
