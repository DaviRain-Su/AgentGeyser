# AgentGeyser MVP-M6 Report

> **Mission**: Yellowstone Real Wiring + Anchor IDL Auto-Fetch + Skill Auto-Registration.
> **Baseline**: `mvp-m5c` @ `671cacc`
> **Endpoint**: `mvp-m6` (pending orchestrator squash + tag) @ `c784631` pre-report
> **Status**: ✅ Complete, with this report as the final validation artifact
> **Date**: 2026-04-26

## §1 Mission summary

MVP-M6 turns the previously stubbed Yellowstone path into a real, opt-in
Triton Dragon's Mouth subscription: gRPC account updates now flow into Anchor
IDL PDA lookup and bounded-concurrent skill auto-registration. The mission also
fixes Anchor IDL discovery by querying the IDL PDA instead of the Program
account, proves the parser with a tonic mock harness, and captures live
mainnet read-only plus surfpool cross-layer evidence.

- **6 implementation features** across 2 milestones, plus V1–V4 verification.
- **19 / 20 validation assertions passed pre-commit.** The remaining pending
  assertion is this report itself; it flips to passed when committed.
- **No M6 wire-level breaking changes.** Existing JSON-RPC, MCP, and SDK wire
  shapes remain compatible.
- **Mainnet remains read-only.** A17 grep audit found zero send/broadcast call
  sites in the live Yellowstone/proxy path.

## §2 Milestones

### M6.1 — Real wiring

| # | Feature | Commit |
|---|---------|--------|
| F1 | Yellowstone `connect_stream` real gRPC subscription, reconnect loop, split/legacy env parsing. | `5b20e82` |
| F2 | Anchor IDL PDA fix: primary v0.30 PDA plus legacy `create_with_seed` fallback. | `b39a3a9` |
| F3 | ProgramDeployed → IDL fetch → skill synth auto-registration with semaphore cap. | `a51de5c` |

### M6.2 — Polish + verify

| # | Feature | Commit |
|---|---------|--------|
| F4 | Reconnect observability and idle watchdog; backoff evidence. | `10d3902` |
| F5 | CI-safe tonic mock-yellowstone harness and parser/registry integration tests. | `a4d024a` |
| F6 | M6 docs, env-var callouts, changelog, and version bump to `0.3.0-alpha.0`. | `da4eaa4` |
| V1 | Cargo gate evidence: fmt, default tests, live-feature tests, clippy. | evidence-only |
| V2 | Live Triton mainnet read-only evidence. | `c784631` |
| V3 | Surfpool cross-layer deploy → auto-skill evidence. | evidence-only |
| V4 | This report + A17 final grep audit. | this commit |

## §3 Evidence

### F2 — `/tmp/m6-evidence/f2-pda-derivation.json`

```json
{
  "legacy_pda": "2ZedRb45p16BRVYfFpo79PtDQ298qVrGXBjD8SQ2a2Sx",
  "match_form": "primary_v0_30_with_legacy_fallback",
  "primary_pda": "6zaeLA816Wb3oEpU5iH1etdeiT6yPUBqgX44SMj742c8",
  "program_id": "4eoZHrR7VEJ1YonxjERYp7Cw95eSYuxpfDL1NtShe42d"
}
```

### F3 — `/tmp/m6-evidence/f3-auto-register.json`

```json
{"latency_ms":2,"program_id":"AUTO111111111111111111111111111111111111111","skills_registered":3}
```

### F4 — `/tmp/m6-evidence/f4-backoff.json`

```json
{"attempts":7,"delays_ms":[1000,2000,4000,8000,16000,32000,60000],"cap_ms":60000}
```

### F5 — `/tmp/m6-evidence/f5-mock-tonic.json`

```json
{"mock_addr":"127.0.0.1:59580","parsed_event_count":1,"update_count":1}
```

### F6 — `/tmp/m6-evidence/f6-version-bump.json`

```json
{
  "cargo_version": "0.3.0-alpha.0",
  "package_versions": {
    "skeleton/sdk/apps/docs/package.json": "0.3.0-alpha.0",
    "skeleton/sdk/package.json": "0.3.0-alpha.0",
    "skeleton/sdk/packages/react/package.json": "0.3.0-alpha.0",
    "skeleton/sdk/packages/sdk/package.json": "0.3.0-alpha.0"
  }
}
```

### V1 — `/tmp/m6-evidence/v1-cargo-gates.json`

```json
{
  "cargo_clippy": {
    "command": "cd skeleton && cargo clippy --workspace --all-targets --all-features -- -D warnings",
    "duration_s": 1.061,
    "exit_code": 0,
    "raw_output": "/tmp/m6-evidence/v1-cargo_clippy.log",
    "tail": "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.98s\n"
  },
  "cargo_fmt": {
    "command": "cd skeleton && cargo fmt --all -- --check",
    "duration_s": 0.262,
    "exit_code": 0,
    "raw_output": "/tmp/m6-evidence/v1-cargo_fmt.log",
    "tail": ""
  },
  "cargo_test_live_feature": {
    "command": "cd skeleton && cargo test --workspace --features live-yellowstone -- --skip live_",
    "duration_s": 13.812,
    "exit_code": 0,
    "raw_output": "/tmp/m6-evidence/v1-cargo_test_live_feature.log",
    "tail": "led; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n   Doc-tests tx_builder\n\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n"
  },
  "cargo_test_workspace": {
    "command": "cd skeleton && cargo test --workspace",
    "duration_s": 3.429,
    "exit_code": 0,
    "raw_output": "/tmp/m6-evidence/v1-cargo_test_workspace.log",
    "tail": "led; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n   Doc-tests tx_builder\n\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n"
  }
}
```

### V2 — `/tmp/m6-evidence/v2-triton-live.json`

```json
{"endpoint_host":"davirai-mainnet-e3c9.mainnet.rpcpool.com","event_count_within_30s":144,"first_event_timestamp":"2026-04-25T16:06:29.517926Z","subscription_uid":"d95f3e2f-8c29-4d28-9393-02542a4c43cc"}
```

### V3 — `/tmp/m6-evidence/v3-cross-layer.json`

```json
{
  "program_id": "4eoZHrR7VEJ1YonxjERYp7Cw95eSYuxpfDL1NtShe42d",
  "deploy_signature": "5MHkDP8CP7JTZ2RNjKgeGqKRgzhP8bdCf6yKHk66A14StFvRinbEp2kzNZxeqsF8L3WgWMx8Lh4WLQp1Rbd2FrDj",
  "ag_listSkills_contains_program_id": true,
  "latency_ms": 1093,
  "harness_mode": "direct-mock",
  "harness_note": "Bridge path was attempted, but Anchor CLI idl init returns HTTP 405 against surfpool; direct-mock uses the accepted V3 fallback by attaching MockYellowstoneStream to the proxy router after a real surfpool deploy."
}
```

## §4 Test counts

- **Cargo default gate:** `cargo test --workspace` passed with **106 tests
  passing** and 1 ignored live nl-planner test.
- **Live-feature gate:** `cargo test --workspace --features live-yellowstone
  -- --skip live_` passed with **114 tests passing** and 1 live test filtered.
- **Mock harness:** 3 tonic mock-yellowstone integration tests passed:
  parser emits `ProgramDeployed`, non-program writes are dropped, and registry
  auto-registration succeeds from mock geyser.
- **Live-gated evidence:** 1 env-gated Triton test produced V2 evidence:
  **144 events within 30s**, host-only endpoint recorded, token omitted.
- **Static gates:** `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  both exited 0 in V1 evidence and in final validation.

## §5 Decisions log

| Q | Locked answer |
|---|---------------|
| Q1 | Anchor IDL PDA derivation tries BOTH primary `anchor:idl` PDA and legacy seeded fallback. |
| Q2 | V3 uses a real surfpool deploy of existing `anchor-hello-world`; accepted fallback was direct mock attach. |
| Q3 | `live-yellowstone` remains opt-in; default CI is deterministic. |
| Q4 | Live chain is mainnet, read-only subscription only. |
| Q5 | Mission has 2 milestones: M6.1 wiring, M6.2 polish + verify. |
| Q6 | Live Triton/surfpool features run with concurrency cap 1. |
| Q7 | Preferred env vars are `AGENTGEYSER_YELLOWSTONE_ENDPOINT` and `AGENTGEYSER_YELLOWSTONE_TOKEN`; legacy `GRPC_URL` tolerated. |
| Q8 | IDL fetch concurrency defaults to 8 and is overridden by `AGENTGEYSER_IDL_FETCH_CONCURRENCY`. |
| Q9 | Workspace version is `0.3.0-alpha.0`. |
| Q10 | Proxy `main.rs` was not modified for F1; existing wiring stayed in place. |

## §6 Lessons learned

- **L34 — Triton free-tier single-stream discipline held.** V2 ran as a
  single read-only subscription and observed no token leakage in evidence.
- **L35 — Generated `geyser_server` stubs were already available.** F5 reused
  existing Yellowstone proto outputs; no `tonic-build` step was needed.
- **L36 — Surfpool IDL init is not a perfect Anchor RPC mirror.** V3 saw HTTP
  405 during bridge setup and used the approved direct-mock fallback after a
  real surfpool deploy.
- **L37 — Grep audits should avoid ambiguous short flags.** Installed ripgrep
  treats `-E` as encoding, so the final A17 audit used `rg -n -e ...` to
  verify zero forbidden call sites.

## §7 Final state

- **Commits ahead of `mvp-m5c`:** 9 at pre-report HEAD `c784631`; 10 including
  this V4 report commit.
- **Contract assertions:** A1–A16 and A18–A20 are passed by feature tests and
  evidence; **A17** is passed by the final grep audit. The report evidence
  records `assertions_passed: 19` and `assertions_pending: 1` until this file
  is committed, after which the tally is **20 / 20**.
- **A17 audit:** zero non-comment matches for
  `send_transaction|sendTransaction|signAndSend|broadcast` under
  `skeleton/crates/idl-registry/src/` and `skeleton/crates/proxy/src/main.rs`.
- **Breaking changes:** **none** in M6 at the wire level.
- **Release readiness:** orchestrator may squash/tag `mvp-m6` after this report
  commit; workers did not push remote refs during V4.
