//! Live Yellowstone gRPC client wrapper (feature `live-yellowstone`).
//!
//! The real gRPC handshake arrives in a later milestone. For now this module
//! exists so the type contract compiles under `--features live-yellowstone`;
//! `connect_stream` returns an empty stream so downstream wiring (F4) can be
//! authored against the final signature without a real endpoint.

#![cfg(feature = "live-yellowstone")]

use tokio_stream::Stream;

use crate::YellowstoneEvent;

/// Connection parameters for a Triton / Helius / self-hosted Yellowstone
/// gRPC endpoint. `token` is optional for endpoints that don't require auth.
#[derive(Clone, Debug)]
pub struct YellowstoneConfig {
    pub endpoint: String,
    pub token: Option<String>,
}

/// Open a Yellowstone gRPC subscription and return a stream of
/// `YellowstoneEvent`s. The stub implementation returns an empty stream so
/// the feature compiles end-to-end; the real subscription handshake is a
/// follow-up milestone.
pub async fn connect_stream(
    cfg: YellowstoneConfig,
) -> anyhow::Result<impl Stream<Item = YellowstoneEvent> + Send + Unpin + 'static> {
    let _ = cfg; // TODO(live): wire yellowstone-grpc-client subscription here.
    Ok(Box::pin(tokio_stream::empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn connect_stream_returns_empty_stream() {
        use tokio_stream::StreamExt;
        let cfg = YellowstoneConfig {
            endpoint: "https://example.invalid".into(),
            token: None,
        };
        let mut stream = connect_stream(cfg).await.expect("stub must succeed");
        assert!(stream.next().await.is_none());
    }
}
