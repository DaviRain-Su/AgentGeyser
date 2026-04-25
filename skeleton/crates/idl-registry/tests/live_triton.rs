#![cfg(feature = "live-yellowstone")]

use std::{env, time::Duration};

use futures::StreamExt;
use idl_registry::yellowstone::{connect_stream, YellowstoneConfig};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::time::{timeout, Instant};
use uuid::Uuid;

fn now_rfc3339() -> anyhow::Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn required_env(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(value) if !value.is_empty() => Some(value),
        _ => {
            eprintln!("skipping: AGENTGEYSER_YELLOWSTONE_ENDPOINT unset");
            None
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_triton_subscribe_yields_one_update() -> anyhow::Result<()> {
    let endpoint = match required_env("AGENTGEYSER_YELLOWSTONE_ENDPOINT") {
        Some(endpoint) => endpoint,
        None => return Ok(()),
    };
    let token = match required_env("AGENTGEYSER_YELLOWSTONE_TOKEN") {
        Some(token) => token,
        None => return Ok(()),
    };

    let endpoint_host = url::Url::parse(&endpoint)?
        .host_str()
        .unwrap_or_default()
        .to_string();
    let mut stream = connect_stream(YellowstoneConfig {
        endpoint,
        token: Some(token),
    })
    .await?;

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut first_event_timestamp = None;
    let mut event_count_within_30s = 0usize;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match timeout(remaining, stream.next()).await {
            Ok(Some(_event)) => {
                event_count_within_30s += 1;
                if first_event_timestamp.is_none() {
                    first_event_timestamp = Some(now_rfc3339()?);
                }
            }
            Ok(None) | Err(_) => break,
        }
    }

    std::fs::create_dir_all("/tmp/m6-evidence")?;
    std::fs::write(
        "/tmp/m6-evidence/v2-triton-live.json",
        format!(
            "{}\n",
            serde_json::json!({
                "subscription_uid": Uuid::new_v4().to_string(),
                "endpoint_host": endpoint_host,
                "first_event_timestamp": first_event_timestamp.unwrap_or_else(String::new),
                "event_count_within_30s": event_count_within_30s
            })
        ),
    )?;

    assert!(
        event_count_within_30s >= 1,
        "expected at least one YellowstoneEvent within 30s"
    );
    Ok(())
}
