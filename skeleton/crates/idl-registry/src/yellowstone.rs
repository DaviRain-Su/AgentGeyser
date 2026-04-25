//! Live Yellowstone gRPC client wrapper (feature `live-yellowstone`).
//!
//! This module connects to a Triton Dragon's Mouth-compatible gRPC endpoint
//! and emits program-deployment events from BPF Loader Upgradeable account
//! writes.

#![cfg(feature = "live-yellowstone")]

use std::{collections::HashMap, fmt, time::Duration};

use anyhow::{anyhow, Context};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::prelude::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
    SubscribeRequestFilterAccounts, SubscribeUpdateAccountInfo,
};

const BPF_LOADER_UPGRADEABLE: &str = "BPFLoaderUpgradeab1e11111111111111111111111";
const INITIAL_BACKOFF_MS: u64 = 1_000;
const MAX_BACKOFF_MS: u64 = 60_000;
const IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

use crate::YellowstoneEvent;

/// Connection parameters for a Triton / Helius / self-hosted Yellowstone
/// gRPC endpoint. `token` is optional for endpoints that don't require auth.
#[derive(Clone)]
pub struct YellowstoneConfig {
    pub endpoint: String,
    pub token: Option<String>,
}

impl fmt::Debug for YellowstoneConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YellowstoneConfig")
            .field("endpoint", &redacted_endpoint(&self.endpoint))
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl YellowstoneConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        if let Ok(endpoint) = std::env::var("AGENTGEYSER_YELLOWSTONE_ENDPOINT") {
            return Ok(Self {
                endpoint,
                token: std::env::var("AGENTGEYSER_YELLOWSTONE_TOKEN").ok(),
            });
        }

        let legacy = std::env::var("GRPC_URL")
            .context("AGENTGEYSER_YELLOWSTONE_ENDPOINT or GRPC_URL must be set")?;
        parse_legacy_grpc_url(&legacy)
    }
}

pub fn next_backoff(prev_ms: u64) -> u64 {
    if prev_ms == 0 {
        INITIAL_BACKOFF_MS
    } else {
        prev_ms.saturating_mul(2).min(MAX_BACKOFF_MS)
    }
}

/// Open a Yellowstone gRPC subscription and return a stream of
/// `YellowstoneEvent`s.
pub async fn connect_stream(
    cfg: YellowstoneConfig,
) -> anyhow::Result<impl Stream<Item = YellowstoneEvent> + Send + Unpin + 'static> {
    let (tx, rx) = mpsc::channel(256);
    tokio::spawn(async move {
        let mut attempt = 0u64;
        let mut backoff_ms = INITIAL_BACKOFF_MS;
        loop {
            attempt = attempt.saturating_add(1);
            match run_one_subscription(&cfg, &tx, &mut backoff_ms, attempt).await {
                Ok(()) => {
                    break;
                }
                Err(err) => {
                    tracing::warn!(
                        event = "yellowstone_reconnecting",
                        attempt,
                        backoff_ms,
                        error = %redact_error(&cfg, &err),
                    );
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = next_backoff(backoff_ms);
                }
            }
        }
    });
    Ok(ReceiverStream::new(rx))
}

async fn run_one_subscription(
    cfg: &YellowstoneConfig,
    tx: &mpsc::Sender<YellowstoneEvent>,
    backoff_ms: &mut u64,
    attempt: u64,
) -> anyhow::Result<()> {
    let mut client = build_client(cfg).await?;
    let mut updates = client
        .subscribe_once(program_deploy_subscribe_request())
        .await?;
    tracing::info!(event = "yellowstone_connected", attempt, slot = 0u64);

    let mut last_event_at = Instant::now();
    loop {
        match next_subscription_item(&mut updates, &mut last_event_at).await {
            SubscriptionPoll::Item(update) => match update?.update_oneof {
                Some(UpdateOneof::Account(account_update)) => {
                    *backoff_ms = INITIAL_BACKOFF_MS;
                    if let Some(event) = account_update
                        .account
                        .as_ref()
                        .and_then(parse_account_event)
                    {
                        if tx.send(event).await.is_err() {
                            return Ok(());
                        }
                    }
                }
                Some(UpdateOneof::Slot(_))
                | Some(UpdateOneof::Ping(_))
                | Some(UpdateOneof::Pong(_)) => {
                    *backoff_ms = INITIAL_BACKOFF_MS;
                }
                _ => {}
            },
            SubscriptionPoll::Idle { idle_secs } => {
                let _ = idle_secs;
                continue;
            }
            SubscriptionPoll::Ended => {
                return Err(anyhow!("yellowstone subscription ended"));
            }
        }
    }
}

#[derive(Debug)]
enum SubscriptionPoll<T> {
    Item(T),
    Idle { idle_secs: u64 },
    Ended,
}

async fn next_subscription_item<S, T>(
    updates: &mut S,
    last_event_at: &mut Instant,
) -> SubscriptionPoll<T>
where
    S: Stream<Item = T> + Unpin,
{
    let idle_deadline = *last_event_at + IDLE_TIMEOUT;
    tokio::select! {
        item = updates.next() => match item {
            Some(item) => {
                *last_event_at = Instant::now();
                SubscriptionPoll::Item(item)
            }
            None => SubscriptionPoll::Ended,
        },
        _ = tokio::time::sleep_until(idle_deadline) => {
            let idle_secs = Instant::now().duration_since(*last_event_at).as_secs();
            tracing::warn!(event = "yellowstone_idle", idle_secs);
            *last_event_at = Instant::now();
            SubscriptionPoll::Idle { idle_secs }
        },
    }
}

async fn build_client(
    cfg: &YellowstoneConfig,
) -> anyhow::Result<
    yellowstone_grpc_client::GeyserGrpcClient<impl yellowstone_grpc_client::Interceptor>,
> {
    let mut builder =
        GeyserGrpcClient::build_from_shared(cfg.endpoint.clone())?.x_token(cfg.token.clone())?;
    if cfg.endpoint.starts_with("https://") {
        builder = builder.tls_config(ClientTlsConfig::new().with_native_roots())?;
    }
    Ok(builder
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .http2_keep_alive_interval(Duration::from_secs(30))
        .connect()
        .await?)
}

fn program_deploy_subscribe_request() -> SubscribeRequest {
    SubscribeRequest {
        accounts: HashMap::from([(
            "ag-program-deploys".to_string(),
            SubscribeRequestFilterAccounts {
                account: vec![],
                owner: vec![BPF_LOADER_UPGRADEABLE.to_string()],
                filters: vec![],
                nonempty_txn_signature: Some(true),
            },
        )]),
        commitment: Some(CommitmentLevel::Confirmed as i32),
        ..Default::default()
    }
}

fn parse_account_event(acc: &SubscribeUpdateAccountInfo) -> Option<YellowstoneEvent> {
    let owner = bs58::decode(BPF_LOADER_UPGRADEABLE).into_vec().ok()?;
    if acc.owner != owner
        || !acc.executable
        || acc.data.len() != 36
        || acc.data.get(..4) != Some(&[2, 0, 0, 0])
    {
        return None;
    }
    Some(YellowstoneEvent::ProgramDeployed {
        program_id: bs58::encode(&acc.pubkey).into_string(),
    })
}

fn parse_legacy_grpc_url(raw: &str) -> anyhow::Result<YellowstoneConfig> {
    let (scheme, rest) = raw
        .split_once("://")
        .ok_or_else(|| anyhow!("GRPC_URL must include a scheme"))?;
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    if authority.is_empty() {
        return Err(anyhow!("GRPC_URL must include a host"));
    }
    let token = path
        .split('/')
        .find(|segment| !segment.is_empty())
        .map(|segment| {
            segment
                .split(['?', '#'])
                .next()
                .unwrap_or(segment)
                .to_string()
        });
    Ok(YellowstoneConfig {
        endpoint: format!("{scheme}://{authority}"),
        token,
    })
}

fn redacted_endpoint(endpoint: &str) -> String {
    parse_legacy_grpc_url(endpoint)
        .map(|cfg| cfg.endpoint)
        .unwrap_or_else(|_| "<invalid>".to_string())
}

fn redact_error(cfg: &YellowstoneConfig, err: &anyhow::Error) -> String {
    let mut text = err.to_string();
    if let Some(token) = cfg.token.as_deref() {
        text = text.replace(token, "<redacted>");
    }
    if cfg.endpoint.contains('/') {
        text = text.replace(&cfg.endpoint, &redacted_endpoint(&cfg.endpoint));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use yellowstone_grpc_proto::prelude::SubscribeUpdate;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn yellowstone_config_constructs() {
        let cfg = YellowstoneConfig {
            endpoint: "https://example.invalid".into(),
            token: None,
        };
        assert_eq!(cfg.endpoint, "https://example.invalid");
        assert!(cfg.token.is_none());

        let cfg2 = YellowstoneConfig {
            endpoint: "x".into(),
            token: Some("tok".into()),
        };
        assert_eq!(cfg2.token.as_deref(), Some("tok"));
    }

    #[test]
    fn next_backoff_doubles_until_cap() {
        assert_eq!(next_backoff(0), 1_000);
        assert_eq!(next_backoff(1_000), 2_000);
        assert_eq!(next_backoff(2_000), 4_000);
        assert_eq!(next_backoff(4_000), 8_000);
        assert_eq!(next_backoff(8_000), 16_000);
        assert_eq!(next_backoff(16_000), 32_000);
        assert_eq!(next_backoff(32_000), 60_000);
        assert_eq!(next_backoff(60_000), 60_000);

        let mut prev = 0;
        let mut schedule = Vec::new();
        for _ in 0..8 {
            prev = next_backoff(prev);
            schedule.push(prev);
        }
        assert_eq!(
            schedule,
            vec![1_000, 2_000, 4_000, 8_000, 16_000, 32_000, 60_000, 60_000]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn idle_watchdog_fires_after_five_minutes_without_updates() {
        let mut updates = futures::stream::pending::<anyhow::Result<SubscribeUpdate>>();
        let mut last_event_at = tokio::time::Instant::now();

        let idle =
            tokio::spawn(
                async move { next_subscription_item(&mut updates, &mut last_event_at).await },
            );

        tokio::time::advance(Duration::from_secs(6 * 60)).await;

        match idle.await.expect("idle task should complete") {
            SubscriptionPoll::Idle { idle_secs } => assert!(idle_secs >= 300),
            other => panic!("expected idle watchdog event, got {other:?}"),
        }
    }

    #[test]
    fn yellowstone_config_from_env_accepts_split_form() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var(
                "AGENTGEYSER_YELLOWSTONE_ENDPOINT",
                "https://example.invalid",
            );
            std::env::set_var("AGENTGEYSER_YELLOWSTONE_TOKEN", "split-token");
            std::env::remove_var("GRPC_URL");
        }
        let cfg = YellowstoneConfig::from_env().expect("split env should parse");
        assert_eq!(cfg.endpoint, "https://example.invalid");
        assert_eq!(cfg.token.as_deref(), Some("split-token"));
        unsafe {
            std::env::remove_var("AGENTGEYSER_YELLOWSTONE_ENDPOINT");
            std::env::remove_var("AGENTGEYSER_YELLOWSTONE_TOKEN");
        }
    }

    #[test]
    fn yellowstone_config_from_env_accepts_legacy_grpc_url() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("AGENTGEYSER_YELLOWSTONE_ENDPOINT");
            std::env::remove_var("AGENTGEYSER_YELLOWSTONE_TOKEN");
            std::env::set_var("GRPC_URL", "https://example.invalid/legacy-token");
        }
        let cfg = YellowstoneConfig::from_env().expect("legacy env should parse");
        assert_eq!(cfg.endpoint, "https://example.invalid");
        assert_eq!(cfg.token.as_deref(), Some("legacy-token"));
        unsafe {
            std::env::remove_var("GRPC_URL");
        }
    }
}
