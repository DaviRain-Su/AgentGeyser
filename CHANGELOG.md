# Changelog

All notable changes to AgentGeyser will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

AgentGeyser is the non-custodial Solana agent runtime. The `0.1.0-alpha.0`
release anchors the M1→M4 substrate: the signer-optional proxy
(`agentgeyser-proxy`), the TypeScript SDK (`@agentgeyser/sdk`), the React
hooks package (`@agentgeyser/react`), the MCP server
(`agentgeyser-mcp-server`), and the docusaurus documentation site under
`skeleton/sdk/apps/docs`. Together they make a single `spl-token::transfer`
skill invocable end-to-end — from `invoke_skill` / `AgentGeyserClient` down
to a confirmed Solana transaction — without the runtime ever touching a
user secret.

## [Unreleased]

### Added
- `AGENTGEYSER_PROXY_PORT` as the shared proxy-port default for the Rust proxy,
  MCP server, TypeScript SDK CLI, and React provider.
- F15 cross-layer integration test (MCP→proxy→tx-builder→surfpool) with real
  signed-and-confirmed transaction evidence.

### Changed
- **Breaking:** `spl-token::transfer` now requires `args.decimals` and emits
  SPL Token `TransferChecked` instructions that bind the mint account.
- `spl-token::transfer` now honors the top-level `payer` envelope field in
  the proxy SPL fast path, while accepting the legacy `accounts` envelope.
- `@agentgeyser/sdk` `Skill` descriptors returned by `listSkills()` now use
  camelCase proxy-aligned fields (`skillId`, `programId`, `instructionName`,
  `paramsSchema`) instead of the old placeholder schema/name fields.
- AgentGeyser proxy now defaults to `127.0.0.1:8999` (Solana RPC remains
  `127.0.0.1:8899`) and fails fast on bind errors instead of falling back to
  `127.0.0.1:8898`.
- Renamed the tag-triggered npm publishing workflow to `publish-dry-run.yml`
  and labeled it as a dry-run until real npm publishing ships in M5d.
- Updated SDK, React, quickstart, installation, and planner docs to use the
  current proxy URL, `skill_id`/`skillId`, `transactionBase64`, and
  `signAndSend({ unsignedTx, signer, connection })` APIs.
- Updated the root `AGENT.md` guide to reflect the actual `skeleton/`
  workspace layout and current pnpm commands.

### Fixed
- Added bounded HTTP client timeouts for OpenAI, Anthropic, and MCP proxy
  forwarding calls so hung upstreams cannot wedge requests indefinitely.
- OpenAI planning now rejects oversized prompts before making upstream calls and
  caps completion length with `max_tokens`.
- MCP proxy forwarding now reports non-2xx proxy responses with HTTP status and
  a bounded response-body tail instead of surfacing useless JSON parse errors.
- Converted devnet funding/airdrop probes to async reqwest clients with a
  bounded timeout so async consumers do not block executor threads.
- JSON-RPC `ag_invokeSkill` now rejects unknown top-level envelope fields
  instead of silently dropping them.

### Removed
- Deleted the dead `skeleton/.github/workflows/ci.yml` workflow; GitHub Actions
  only honors the root `.github/workflows/` directory.
- Removed the unused `zod` dependency from `@agentgeyser/sdk`.

## 0.3.0-alpha.0 — MVP-M6

### Added
- Wired the opt-in `live-yellowstone` ingestion path to real Yellowstone gRPC
  subscriptions, bounded Anchor IDL fetches, and auto-skill registration.
- Documented the split Yellowstone env vars and IDL fetch concurrency override.

### Fixed
- Anchor IDL fetches now query the IDL PDA with primary and legacy fallback
  derivations, allowing mainnet Anchor deployments to surface in `ag_listSkills`.

## [0.2.0-alpha.0] - 2026-04-24

### Added
- `nl-planner` crate: natural-language → structured Plan with multiple LLM providers
- `OpenAiProvider` (OpenAI Chat Completions API)
- `MockProvider` (deterministic testing path)
- `AnthropicMessagesProvider` (Anthropic Messages API + tool_use; supports Kimi-for-coding via `api.kimi.com/coding` + `User-Agent: KimiCLI/1.5`)
- `ag_planAction` JSON-RPC method on the proxy (routes prompts through provider_from_env)
- `client.planAction` SDK binding (snake_case → camelCase adapter)
- `tx-builder` `build_spl_token_transfer` (SPL-Token-2022 unsigned transactions)
- Proxy `ag_invokeSkill` routing for `spl-token::transfer` (devnet RPC blockhash fetch)
- `signAndSend` SDK helper (non-custodial client-side signing path)
- MCP server `chain=devnet` parameter (fast-fails mainnet)
- Docusaurus `nl-planner` documentation page

### Fixed
- Proxy `spl-token::transfer` response field renamed `tx` → `transaction_base64` to align with SDK adapter contract

## [0.1.0-alpha.0] - 2026-04-24

Initial public alpha of the AgentGeyser substrate.

### Added
- `agentgeyser-proxy` JSON-RPC server exposing the agent-side surface
  (`list_skills`, `invoke_skill`) on top of Solana RPC, with the signer
  remaining client-side (non-custodial by construction).
- `@agentgeyser/sdk` TypeScript client (`AgentGeyserClient`) with
  `listSkills`, `invokeSkill`, and `signAndSend` helpers, targeting
  `@solana/web3.js` `^2.0.0`.
- `@agentgeyser/react` React bindings (`useAgentGeyser`, `useSkills`,
  `useInvokeSkill`) plus Playwright e2e harness for the
  `spl-token::transfer` flow.
- `agentgeyser-mcp-server` Model Context Protocol server that bridges
  MCP tool calls (`list_skills`, `invoke_skill`) onto the same proxy
  surface, enabling Claude Desktop and other MCP hosts to drive the
  substrate.
- Docusaurus documentation site (`@agentgeyser/docs`) covering
  quickstart, SDK reference (typedoc-generated), React recipes, and the
  MCP integration guide.
- Synthesis reports for MVP-M1 through MVP-M4 under
  `skeleton/examples/`, each embedding real, user-confirmed on-chain
  signatures captured during live verification.

### Changed
- Bumped `@agentgeyser/sdk` and `@agentgeyser/react` from `0.0.0` to
  `0.1.0-alpha.0` and added publish-readiness metadata (keywords,
  repository, bugs, homepage, license, files allowlist) in preparation
  for the MVP-M5c npm publish flip.

### Fixed
- No user-facing fixes in this initial release.

[Unreleased]: https://github.com/DaviRain-Su/AgentGeyser/compare/v0.1.0-alpha.0...HEAD
[0.2.0-alpha.0]: https://github.com/DaviRain-Su/AgentGeyser/releases/tag/v0.2.0-alpha.0
[0.1.0-alpha.0]: https://github.com/DaviRain-Su/AgentGeyser/releases/tag/v0.1.0-alpha.0
