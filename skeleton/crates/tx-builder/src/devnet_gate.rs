//! Devnet balance probe + airdrop gate (M5b-F1).
use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};
use thiserror::Error;

pub const DEVNET_RPC_URL: &str = "https://api.devnet.solana.com";
pub const AIRDROP_LAMPORTS: u64 = 1_000_000_000;
pub const AIRDROP_ENV_FLAG: &str = "AGENTGEYSER_ALLOW_AIRDROP";

#[derive(Debug, Error)]
pub enum DevnetGateError {
    #[error("rpc error: {0}")]
    Rpc(String),
    #[error("insufficient funds: have {have} lamports, need {need}")]
    InsufficientFunds { have: u64, need: u64 },
    #[error("airdrop requires {0}=1 in env")]
    AirdropNotAllowed(&'static str),
}

pub async fn ensure_devnet_funded(pubkey: &str, min_lamports: u64) -> Result<(), DevnetGateError> {
    ensure_devnet_funded_at(DEVNET_RPC_URL, pubkey, min_lamports).await
}

pub async fn ensure_devnet_funded_at(
    url: &str,
    pubkey: &str,
    min_lamports: u64,
) -> Result<(), DevnetGateError> {
    let body = json!({"jsonrpc":"2.0","id":1,"method":"getBalance","params":[pubkey]});
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| DevnetGateError::Rpc(e.to_string()))?;
    let resp: Value = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| DevnetGateError::Rpc(e.to_string()))?
        .json()
        .await
        .map_err(|e| DevnetGateError::Rpc(e.to_string()))?;
    let have = resp
        .pointer("/result/value")
        .and_then(Value::as_u64)
        .ok_or_else(|| DevnetGateError::Rpc(format!("bad response: {resp}")))?;
    if have < min_lamports {
        return Err(DevnetGateError::InsufficientFunds {
            have,
            need: min_lamports,
        });
    }
    Ok(())
}

pub async fn airdrop_if_needed(pubkey: &str) -> Result<(), DevnetGateError> {
    airdrop_if_needed_at(DEVNET_RPC_URL, pubkey).await
}

pub async fn airdrop_if_needed_at(url: &str, pubkey: &str) -> Result<(), DevnetGateError> {
    if std::env::var(AIRDROP_ENV_FLAG).ok().as_deref() != Some("1") {
        return Err(DevnetGateError::AirdropNotAllowed(AIRDROP_ENV_FLAG));
    }
    let body = json!({"jsonrpc":"2.0","id":1,"method":"requestAirdrop","params":[pubkey, AIRDROP_LAMPORTS]});
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| DevnetGateError::Rpc(e.to_string()))?
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| DevnetGateError::Rpc(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn balance_response(lamports: u64) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","result":{{"context":{{"slot":1}},"value":{lamports}}},"id":1}}"#
        )
    }
    #[tokio::test]
    async fn ok_when_balance_meets_threshold() {
        let mut srv = mockito::Server::new_async().await;
        let m = srv
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(balance_response(2_000_000_000))
            .create_async()
            .await;
        ensure_devnet_funded_at(&srv.url(), "placeholder", 1_000_000_000)
            .await
            .expect("ok");
        m.assert_async().await;
    }
    #[tokio::test]
    async fn insufficient_funds_when_balance_below_threshold() {
        let mut srv = mockito::Server::new_async().await;
        srv.mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(balance_response(10))
            .create_async()
            .await;
        let err = ensure_devnet_funded_at(&srv.url(), "placeholder", 1_000_000_000)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DevnetGateError::InsufficientFunds {
                have: 10,
                need: 1_000_000_000
            }
        ));
    }
    #[tokio::test]
    async fn airdrop_requires_env_flag() {
        std::env::remove_var(AIRDROP_ENV_FLAG);
        let err = airdrop_if_needed_at("http://127.0.0.1:1", "placeholder")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DevnetGateError::AirdropNotAllowed(AIRDROP_ENV_FLAG)
        ));
    }
}
