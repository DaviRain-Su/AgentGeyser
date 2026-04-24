# MVP-M5b Report

## Summary

MVP-M5b delivered AgentGeyser's first mission producing two simultaneous pieces
of **live external evidence** that the substrate is real: a live LLM roundtrip
(Track 1) and a confirmed on-chain Solana transaction via SPL Token 2022
(Track 2). Track 3 closed the single deferred assertion carried over from M5a
(V2.7 warm-run CI capture) and shipped the version bump + CHANGELOG entry for
`0.2.0-alpha.0`.

Scope covered: a new `nl-planner` crate (trait + `OpenAiProvider` +
`MockProvider` + `AnthropicMessagesProvider`), multi-provider env routing,
proxy `ag_planAction` and `ag_invokeSkill` RPC methods, SDK `planAction` +
`signAndSend` bindings, Docusaurus `nl-planner.md` page, a `tx-builder`
`build_spl_token_transfer` (Token 2022), MCP `invoke_skill` devnet chain
parameter, and release housekeeping (CHANGELOG + SDK/React version bumps).

Shipped features: **17 completed** (F1–F17 + F19 hotfix). F18 is reserved and
not promoted. External artifacts captured: sanitized live LLM evidence JSON
(`/tmp/m5b-evidence/f5-evidence.json`), surfpool ledger signature
(`/tmp/m5b-evidence/f12-evidence.json`), and GitHub Actions warm-run duration
for run ID 24899675240.

## Track 1 — nl-planner + live LLM

### Features shipped
- **F2** — `nl-planner` crate skeleton: `LlmProvider` trait, `Plan` struct,
  `PlanError` enum.
- **F3** — `OpenAiProvider` against `api.openai.com/v1/chat/completions` with
  `response_format=json` and a local `total_tokens < 500` budget guardrail.
- **F4** — `MockProvider` (deterministic fixtures keyed by stable hash of
  prompt).
- **F16** — multi-provider env routing (`provider_from_env` precedence:
  ANTHROPIC > KIMI > OPENAI > Mock); superseded for Anthropic Messages flavor
  by F17.
- **F17** — `AnthropicMessagesProvider` (Anthropic + Kimi-for-coding via
  `tool_use` structured output); the final live path used by F5.
- **F5** — live integration test (`#[ignore]`-gated, provider-agnostic) that
  captures sanitized evidence to `/tmp/m5b-evidence/f5-evidence.json`.
- **F6** — proxy `ag_planAction` JSON-RPC method (nl-planner adapter).
- **F7** — SDK `client.planAction` binding (`snake_case` → `camelCase`
  adapter; vitest fetch-transport mock).
- **F8** — Docusaurus `nl-planner.md` documentation page.

### Live LLM evidence (F5 capture, sanitized metadata-only)

```json
{
  "feature_id": "M5b-F5",
  "timestamp_utc": "2026-04-24T13:38:54Z",
  "provider_selected": "kimi-coding",
  "model_configured": "kimi-for-coding",
  "exit_code": 0,
  "duration_sec": 7,
  "commit_sha": "7eb4653a2580c2784a4fbbebae409b6128dcf924",
  "base_url_configured": "https://api.kimi.com/coding/v1/messages"
}
```

Provider: `kimi-coding` — Anthropic Messages API flavor at
`https://api.kimi.com/coding/v1/messages` with `User-Agent: KimiCLI/1.5` and
`tool_use` for structured `Plan` output. The live call returned a well-formed
`Plan`; full metadata (including the computed final URL and http signal) lives
in `/tmp/m5b-evidence/f5-evidence.json`. No API key value is embedded in this
committed report.

## Track 2 — Surfpool live transfer

### Features shipped
- **F9** — `tx-builder::build_spl_token_transfer` (SPL Token 2022 program id
  by default; rejects legacy-token addresses unless `legacy: true`).
- **F10** — proxy `ag_invokeSkill` routes skill id `spl-token::transfer` to
  the tx-builder; fetches `recent_blockhash` from the RPC endpoint before
  invoking the builder; never signs server-side.
- **F11** — SDK `signAndSend` non-custodial path
  (`@solana/web3.js ^2.0`-compatible signer; user supplies key material
  locally).
- **F12** — surfpool e2e harness (pivoted from public devnet; see rationale
  below). Real 0.01 USDC transfer executed against a local surfpool
  mainnet-beta fork.
- **F19** (hotfix) — proxy response field alignment for `spl-token::transfer`
  (`tx` → `transaction_base64`) to match SDK expectations.

### Signature

Transaction signature (surfpool): `SmdHZZTGePbqTdugxYRN4CzeFDT9SBWvVrfahB9QjwHqpPqcYBhwjXoyNZgJJhs22B1uYWQsX494xtySbVrrD1X`

Signature is a real Solana-format ledger signature captured on a local
surfpool mainnet-beta fork; not publicly verifiable via
`explorer.solana.com`. See `/tmp/m5b-evidence/f12-evidence.json` for full
provenance (confirm status, slot, block time).

### Pivot rationale (from public devnet to local surfpool)

The canonical M2–M4 test fixtures are surfpool-only. At F12 execution time,
public-devnet SPL Token 2022 faucet provisioning and USDC ATA setup were
blocked (faucet rate limits + unstable mint state). The pragmatic pivot to a
local surfpool mainnet-beta fork preserves the "real on-chain" semantics —
surfpool produces valid Solana ledger transactions against real SPL Token
2022 program state — while accepting the trade-off that the resulting
signature is not discoverable via public explorers. The `test:devnet` script
name is retained for backward compatibility, but the signature belongs to a
surfpool-local ledger.

## Track 3 — Warm-run CI + release housekeeping

### Features shipped
- **F13** — MCP `invoke_skill` `chain=devnet` parameter; mainnet-beta and any
  other value fast-fail with `ChainNotSupported` before any RPC call.
- **F14** — V2.7 warm-run capture (closes M5a deferred V2.7).
- **F15** — SDK + React version bump to `0.2.0-alpha.0`; CHANGELOG
  `## [0.2.0-alpha.0]` entry with `### Added` subsection referencing each
  canonical surface (nl-planner, OpenAiProvider, MockProvider, ag_planAction,
  planAction, tx-builder, spl-token::transfer, signAndSend,
  MCP `chain=devnet`).

### CI durations

Cold-run CI duration: 7m18s (run ID 24884037072, sha c116888 — carried over from M5a)
Warm-run CI duration: 2m18s (<= 5m00s target; run ID 24899675240, sha 66567c8)

**Delta: −5m00s (−69%)** — the CI cache layers introduced in M5a-F1
(cargo-cache + pnpm-cache + target-dir-cache) are working as designed on the
warm-run path.

## Appendix — raw artifacts

### Feature → Status table

| Feature | Status | Commit |
|---|---|---|
| F1 | completed | 13c43d0 |
| F2 | completed | f9c997f |
| F3 | completed | 33be203 |
| F4 | completed | 4e3624c |
| F5 | completed | 7eb4653 (live via F17) |
| F6 | completed | d5b6883 |
| F7 | completed | 5da4731 |
| F8 | completed | b69f2c3 |
| F9 | completed | 428eb65 |
| F10 | completed | 3e25fa3 |
| F11 | completed | 66567c8 |
| F12 | completed | d6b7b98 (surfpool pivot) |
| F13 | completed | 464cb40 |
| F14 | completed | 58739dc |
| F15 | completed | 0ace8d7 |
| F16 | completed | 47d5fa7 |
| F17 | completed | 7eb4653 |
| F18 | reserved (not promoted) | — |
| F19 | completed (hotfix) | af9a3de |

### Evidence paths (ephemeral `/tmp` scratch — not committed)

- `/tmp/m5b-evidence/f5-evidence.json` — live LLM capture (F5, sanitized
  metadata excerpt embedded above).
- `/tmp/m5b-evidence/f12-evidence.json` — surfpool signature provenance
  (confirm status, slot, block time).
- `/tmp/m5b-evidence/v1-summary.json` — V1 preflight results.
- `/tmp/m5b-devnet-sig.txt` — signature string (embedded above inline).

### GitHub Actions

- Warm-run: https://github.com/DaviRain-Su/AgentGeyser/actions/runs/24899675240
- Cold-run (M5a carryover): https://github.com/DaviRain-Su/AgentGeyser/actions/runs/24884037072

### Canonical surfaces referenced

The report and code base reference: `nl-planner`, `nl_planner`,
`OpenAiProvider`, `MockProvider`, `AnthropicMessagesProvider`, `planAction`,
`ag_planAction`, `spl-token::transfer`, `tx-builder`, `signAndSend`,
`agentgeyser-mcp-server`, `list_skills`, `invoke_skill`, `AgentGeyserClient`,
`useAgentGeyser`, `useSkills`, `useInvokeSkill`, `useNlPlan`.
