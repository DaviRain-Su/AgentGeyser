# AgentGeyser Skeleton — Agent Guide

## M6 Live Yellowstone

M6 enables real Yellowstone ingestion behind the opt-in `live-yellowstone` Cargo feature, which remains default-off; CI includes an explicit live-feature matrix entry while default builds stay deterministic. To use the live path, set `AGENTGEYSER_YELLOWSTONE_ENDPOINT` and `AGENTGEYSER_YELLOWSTONE_TOKEN`; optionally set `AGENTGEYSER_IDL_FETCH_CONCURRENCY` to override the default bounded Anchor IDL fetch concurrency of `8`. When the proxy starts with `--features live-yellowstone` and both Yellowstone env vars are set, deployed Anchor programs can be fetched, synthesized, and auto-registered so their skills surface through `ag_listSkills`.
