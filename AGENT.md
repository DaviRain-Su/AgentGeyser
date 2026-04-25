# AgentGeyser Skeleton — Agent Guide

## Build / Test / Run
- Rust: `cd skeleton && cargo build --workspace`, `cargo test --workspace`, `cargo run -p proxy --bin proxy`
- Node: `cd skeleton/sdk && pnpm install`, `pnpm -r build`, `pnpm --filter @agentgeyser/sdk test`
- Devnet SDK smoke: `pnpm -C skeleton/sdk/packages/sdk run test:devnet`
- Run single Rust test: `cd skeleton && cargo test -p <crate> <filter>` (e.g. `cargo test -p skill-synth`)

## Architecture
- Rust workspace (`skeleton/Cargo.toml`) with crates in `skeleton/crates/`:
  - `proxy` — Axum JSON-RPC proxy binary and library
  - `idl-registry` — Anchor IDL registry and optional Yellowstone ingestion
  - `skill-synth` — deterministic skill synthesis from IDLs
  - `tx-builder` — unsigned Solana transaction builders
  - `nl-planner` — natural-language planning
  - `mcp-server` — MCP server implementation
- Node/TS packages in `skeleton/sdk/packages/` (`@agentgeyser/sdk`, `@agentgeyser/react`) managed by the `skeleton/sdk` pnpm workspace
- Examples and smoke tests in `skeleton/examples/` and `skeleton/sdk/examples/`

## Code Style
- Rust 2021 edition; use `anyhow` for errors, `thiserror` for structured errors, `tracing` for logs
- Prefer `serde_json` for JSON, `async-trait` for async traits, `tokio` runtime
- Import order: std → crates → workspace deps → local crates
- Naming: `snake_case` for fns/vars, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants
- TS: ESM (`"type": "module"`), use `tsx` to run scripts, prefer `const/let`, async/await
