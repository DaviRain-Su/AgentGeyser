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
- Placeholder for upcoming changes.

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
[0.1.0-alpha.0]: https://github.com/DaviRain-Su/AgentGeyser/releases/tag/v0.1.0-alpha.0
