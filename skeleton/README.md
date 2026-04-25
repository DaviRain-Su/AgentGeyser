# AgentGeyser Skeleton

## Architecture

**Live Yellowstone:** M6 adds an opt-in Yellowstone ingestion path that lets the proxy watch mainnet program deployments, fetch Anchor IDLs, synthesize skills, and expose them via `ag_listSkills` when built with `--features live-yellowstone`; see [AGENT.md](AGENT.md) for the required env-var setup.

## Bootstrap

```bash
cargo build --workspace
pnpm install
pnpm -r build
```

This repository skeleton is a structural starting point only.
