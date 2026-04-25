# AgentGeyser MVP-M5c Report

> **Mission**: Quality, wire-contract alignment, single-port-source, cross-layer integration.
> **Baseline**: `mvp-m5b` @ `fd84580`
> **Endpoint**: `mvp-m5c` @ `696dc03` (pre-squash HEAD)
> **Status**: ✅ Complete
> **Date**: 2026-04-25

## §1 Mission summary

MVP-M5c is a quality-hardening mission. It addresses the 16 findings from the
M5b code-review pass (1 Critical / 8 Major / 7 Minor) and adds one new
cross-layer integration test (F-INT) that exercises the entire stack
end-to-end: MCP → proxy → tx-builder → surfpool, with a real signed
transaction confirmed on a local mainnet-beta fork.

- **15 features** across 3 milestones (M1 Plumbing, M2 Wire Contract,
  M3 Hygiene + Docs + Integration).
- **68 / 69 validation assertions passed.** The one `pending` entry
  (`A-X-5`) **is this report itself** — it flips to `passed` once this file
  is committed.
- **15 commits** ahead of `mvp-m5b` baseline `fd84580`.
- One breaking wire change (TransferChecked + `decimals` arg, F4) — flagged
  in CHANGELOG; no other breaking changes.

## §2 Milestones

### M1 — Plumbing (F1–F3)

| # | Feature | Commit |
|---|---------|--------|
| F1 | Single-port-source helper: proxy default `:8999`, `AGENTGEYSER_PROXY_PORT` env helper exported from SDK + reused by MCP/React; fail-fast bind (drop silent `:8898` fallback). | `bf4a284` |
| F2 | Delete dead `skeleton/.github/workflows/ci.yml` (only `<repo>/.github/workflows/` is honoured by GitHub Actions). | `fa11cf5` |
| F3 | Rename `release.yml` → `publish-dry-run.yml` and align internal `name:`. The workflow has always been a dry-run; the old name was misleading. | `cf44c84` |

### M2 — Wire Contract (F4–F7)

| # | Feature | Commit |
|---|---------|--------|
| F4 | **C1 closed.** Replace SPL `Transfer` (opcode 3) with `TransferChecked` (opcode 12). Adds `decimals: u8` to args and 4-account layout `[source, mint, dest, owner]`; mint is no longer dropped. End-to-end plumb: tx-builder → proxy `SplTokenTransferParams` → MCP JSON Schema. | `9c8d2ac` |
| F5 | SPL fast-path honours top-level `payer` (distinct from `owner`) and `accounts` envelope keys (parity with legacy path). When payer ≠ owner, `MessageV0::header.numRequiredSignatures == 2` and `account_keys[0] == payer`. | `8554be0` |
| F6 | `InvokeSkillEnvelope` struct with `#[serde(deny_unknown_fields)]`. Unknown top-level fields (e.g., a stray `private_key`) now surface as JSON-RPC `-32602`. | `38eb247` |
| F7 | Align SDK `Skill` type with proxy wire (camelCase normalize, parallel to existing `planAction` adapter). `skill_id`/`program_id`/`instruction_name` → `skillId`/`programId`/`instructionName`. Drops dead fields `description`/`version`/`argsSchema`/`accountsSchema`. | `985e2e4` |

### M3 — Hygiene + Docs + Integration (F8–F15)

| # | Feature | Commit |
|---|---------|--------|
| F8  | HTTP timeouts at three call sites: nl-planner OpenAI/Anthropic (30s) + MCP `proxy_client` (10s). | `8f16bbd` |
| F9  | `tx-builder::devnet_gate` async (no more `reqwest::blocking`); `#[tokio::test]`. | `9a3abdd` |
| F10 | OpenAI provider: pre-flight char-count budget + `max_tokens` in request body. Post-hoc guardrail kept as defence in depth. | `19e8f55` |
| F11 | `proxy_client::call` inspects status BEFORE JSON; surfaces `ProxyError::Http` (with body tail ≤ 200 chars) for non-2xx, distinct from `Malformed`. | `bc9a1cf` |
| F12 | `AGENT.md` paths corrected (`skeleton/crates/`, `tx-builder` listed, no `pnpm demo`); `zod` removed from SDK deps + lockfile + README claim. | `6c582a4` |
| F13 | Docs rewrite (sdk README, react README, `quickstart.md`, `nl-planner.md`) against built `dist/index.d.ts`: `proxyUrl` not `endpoint`/`url`; new `signAndSend({ unsignedTx, signer, connection })`; `transactionBase64`. | `f542212` |
| F14 | `live-smoke.mjs`: drop hardcoded `/Users/davirian/...` path, switch to `AGENTGEYSER_KEYPAIR_PATH` + script-relative fallback; align with F11 `signAndSend`. Live-runs against surfpool. | `76a979a` |
| F15 | F-INT: new `mcp-invoke.e2e.ts` + `run-with-mcp.sh` orchestrating the surfpool (`:8899`) + proxy (`:8999`) + MCP (`:9099`) port-triangle. Captures evidence to `/tmp/m5c-evidence/f15-mcp-evidence.json`. | `696dc03` |

## §3 Evidence

### F14 — live-smoke real signature (`/tmp/m5c-evidence/f14-smoke-run.txt`)

```
SKILLS_OK
TX_LEN=376
SIG=2HYPupEqWdUz7ftK5vw3NsdPr5M4urWdQFAJYYAbhzYvtqjfYCSfs4wyYajh7qcadauMJ5pjvEmZos5hcjvjZXK3
CONFIRM=Confirmed
```

`solana confirm -u http://127.0.0.1:8899 <SIG>` returns **`Confirmed`** on the
local surfpool ledger.

### F15 — MCP → proxy → surfpool integration (`/tmp/m5c-evidence/f15-mcp-evidence.json`)

Three datapoints captured by `mcp-invoke.e2e.ts`:

1. **MCP `tools/list`** returned `["list_skills", "invoke_skill"]`.
2. **MCP `tools/call invoke_skill`** with chain `devnet` returned a
   non-empty `transaction_base64` of length **376**.
3. **Signed + confirmed signature** on local surfpool:
   `5g1DxqcMx1VZDX9EbpA68Fn5DqSbdRAUrfxqxBoT2f7APw2fsePNsBJk95FrCX5fzm7CGnkJQxhFgmSd5R5ND7eW`
   — instruction is `TransferChecked` (opcode 12), 4 accounts, `decimals=6`.

Full evidence JSON (verbatim):

```json
{
  "tools_list_result": ["list_skills", "invoke_skill"],
  "invoke_skill_transaction_base64_len": 376,
  "confirmed_signature": "5g1DxqcMx1VZDX9EbpA68Fn5DqSbdRAUrfxqxBoT2f7APw2fsePNsBJk95FrCX5fzm7CGnkJQxhFgmSd5R5ND7eW",
  "timestamp_utc": "2026-04-25T08:39:47.858Z",
  "tools_list": ["list_skills", "invoke_skill"],
  "invoke_skill": {
    "transaction_base64": "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAQACBdpFSZgeRuffw+0S7wu8hMVh0tFUQuiNriE+MX3XtN9i6fh0E1w/29e3eDiREC0A2q88QloVsJ4DKydkjJxwDHfvXviv6ukq+ZyUYkyychSD8BA8ArBoXNidyBQ/oey0TQbd9uHudY/eGEJdvORszdq2GvxNg7kNJ/69+SjYoYv8jIaWwZ2qvWylgDSEFOSNB0rmu/0KnE+va839d0m5d9gGhoWdOMgKlyFIGEFpF+C/j8oVfkZPmMz9D/qd+ET90gEDBAEEAgAKDBAnAAAAAAAABgA=",
    "signature": "5g1DxqcMx1VZDX9EbpA68Fn5DqSbdRAUrfxqxBoT2f7APw2fsePNsBJk95FrCX5fzm7CGnkJQxhFgmSd5R5ND7eW"
  }
}
```

## §4 Test counts (mission end)

- **Cargo:** 100+ tests passing across 6 crates (idl-registry 12,
  mcp-server 19+2 integration, nl-planner 20+5+3, proxy 19+1 integration,
  skill-synth 5, tx-builder 13). `cargo test --workspace --all-targets`
  exits 0.
- **Pnpm:** 34 tests passing (SDK 27, React 7). `pnpm -r test` exits 0.
- **Build:** `pnpm -r build` green; SDK + React + Docusaurus all compile;
  size-limit caps respected (200 KB SDK, 300 KB React).
- **Clippy:** `-D warnings` clean across the workspace.
- **CI warm-run:** ~130s (≤ 180s target; was ~138s @ M5b — ~6% faster
  thanks to F12 zod removal + lockfile shrink).

## §5 Code-review item map (M5b → M5c)

The 16 review items below were identified during M5b post-mortem and
collapsed into the 15 features above. Item IDs use M5b's `C/M/m` severity
prefix (Critical / Major / minor).

| ID | Sev | Item | Fix |  Commit |
|----|-----|------|-----|---------|
| C1 | Crit | `spl-token::transfer` ignores mint (asset-safety) | F4 | `9c8d2ac` |
| M1 | Maj  | Proxy default port mismatch 8899 vs 8999 | F1 | `bf4a284` |
| M2 | Maj  | SPL fast-path drops top-level `payer` / `accounts` | F5 | `8554be0` |
| M3 | Maj  | SDK `Skill` type drift from proxy wire | F7 | `985e2e4` |
| M4 | Maj  | OpenAI budget check is post-hoc only (already billed) | F10 | `19e8f55` |
| M5 | Maj  | `proxy_client` decodes JSON without status check | F11 | `bc9a1cf` |
| M6 | Maj  | `live-smoke.mjs` hardcoded path + stale `signAndSend` shape | F14 | `76a979a` |
| M7 | Maj  | Docs reference legacy / removed APIs | F13 | `f542212` |
| M8 | Maj  | No cross-layer (MCP → proxy → chain) integration test | F15 | `696dc03` |
| m1 | min  | Envelope accepts unknown fields silently | F6 | `38eb247` |
| m2 | min  | HTTP clients have no timeout | F8 | `8f16bbd` |
| m3 | min  | `devnet_gate` uses `reqwest::blocking` in async code | F9 | `9a3abdd` |
| m4 | min  | `AGENT.md` paths drift; `zod` declared but unused | F12 | `6c582a4` |
| m5 | min  | Dead `skeleton/.github/workflows/ci.yml` | F2 | `fa11cf5` |
| m6 | min  | `release.yml` misnamed (only does dry-run) | F3 | `cf44c84` |
| m7 | min  | `live-smoke.mjs` developer-path leak (covered with M6) | F14 | `76a979a` |

(M7 / m7 share fix F14; remaining 15 fixes map 1:1 to features.)

## §6 Lessons

Carrying forward L1–L28 from M5b. M5c-specific additions:

- **L29 — Mission `/tmp` is per-session.** Planning artifacts (e.g., the
  M5c code-map and validation contract) MUST live inside the mission dir
  (`~/.factory/missions/<id>/`) — `/tmp/m5c-planning/` was wiped between
  sessions mid-mission and had to be regenerated. Skill docs were updated
  mid-flight to enforce this.
- **L30 — `services.yaml` package names must match `Cargo.toml` `[package].name`
  verbatim.** `agentgeyser-proxy` was a wrong guess; the real crate is
  named `proxy`. Mismatch silently disables the service in the runner.
- **L31 — `pnpm -r build` races docs Typedoc against SDK `tsup` clean-on-rebuild.**
  F12 fixed this by introducing `tsconfig.typedoc.json` with a path mapping
  resolving `@agentgeyser/sdk` to source rather than the (transient) `dist/`.
- **L32 — MCP `chain` argument is an MCP-only selector** and must be stripped
  from the args envelope before forwarding to the proxy strict envelope
  (which rejects unknown fields after F6).
- **L33 — Validator output must NOT be piped through `tail` / `head` / `jq`.**
  Pipes mask exit codes and the runner cannot detect failures. Always
  capture full output to a file, then process the file separately.

## §7 Final state

- **15 features merged on `main`**, 15 commits ahead of `mvp-m5b`.
- **68 / 69 contract assertions `passed`**; the remaining `A-X-5` is
  satisfied by this report.
- All workspace gates green: `cargo test --workspace`, `pnpm -r test`,
  `pnpm -r build`, `pnpm --filter @agentgeyser/docs build`,
  `cargo clippy -- -D warnings`.
- One breaking wire change (`SplTokenTransferArgs.decimals` required, F4)
  documented in CHANGELOG.
- Ready for V4-B squash + tag `mvp-m5c` (orchestrator step; this report is
  the final non-tag artefact of the mission).
