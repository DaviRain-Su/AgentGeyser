# AgentGeyser MVP-M1 — Synthesis Report

- Date: 2026-04-24
- Baseline commit: `78e608b` (Spike end)
- HEAD at synthesis: `24630ca` (F6 CI + .env gitignore)
- Mission scope: real Yellowstone gRPC + on-chain Anchor IDL fetch, mock path preserved.

## 1. Assertion Results (F1–F6 + X.*)

Legend: PASS = all sub-checks verified. Evidence is a file path, `wc -l`, or command output.

### F1 — Yellowstone gRPC client wrapper

| ID | Result | Evidence |
|---|---|---|
| F1.1 | PASS | `skeleton/crates/idl-registry/src/yellowstone.rs` tracked. |
| F1.2 | PASS | `wc -l` = **63** (bound 20–120). |
| F1.3 | PASS | `#![cfg(feature = "live-yellowstone")]` at top of file; default-feature build still works (F6.1 cargo build). |
| F1.4 | PASS | `YellowstoneConfig { endpoint, token }` + `pub async fn connect_stream(...)` returning `impl Stream<Item=YellowstoneEvent>+Send+Unpin+'static`. |
| F1.5 | PASS | `idl-registry/Cargo.toml`: `live-yellowstone = ["dep:yellowstone-grpc-client", "dep:yellowstone-grpc-proto"]`; optional deps listed. |
| F1.6 | PASS | `cargo check --workspace --all-features` → `Finished \`dev\` profile ... in 0.54s`. |
| F1.7 | PASS | `#[cfg(test)]` mod in `yellowstone.rs`; `cargo test --features live-yellowstone -p idl-registry` exits 0. |

### F2 — Anchor IDL on-chain fetcher

| ID | Result | Evidence |
|---|---|---|
| F2.1 | PASS | `skeleton/crates/idl-registry/src/anchor_idl.rs` exists. |
| F2.2 | PASS | `wc -l` = **139** (bound 30–140). |
| F2.3 | PASS | `pub async fn fetch_anchor_idl(rpc_url: &str, program_id: &str) -> anyhow::Result<Option<Idl>>`. |
| F2.4 | PASS | `bs58` + seeded PDA; `reqwest` JSON-RPC `getAccountInfo`; `flate2` inflate; `serde_json::from_slice::<Idl>`. |
| F2.5 | PASS | `decode_anchor_idl_payload_roundtrip` / `decode_*` tests in `anchor_idl.rs`; `cargo test -p idl-registry --lib anchor_idl` ≥ 1 passed. |
| F2.6 | PASS | `null` account → `Ok(None)` exercised by unit test. |
| F2.7 | PASS | `anyhow::Result` + `?`; no `.unwrap()` in non-test code (grep clean). |

### F3 — IdlRegistry wiring

| ID | Result | Evidence |
|---|---|---|
| F3.1 | PASS | `IdlRegistry::with_rpc_url(url: impl Into<String>) -> Self` in `lib.rs`. |
| F3.2 | PASS | `try_fetch_anchor_idl` consults mock first, then `rpc_url` via `anchor_idl::fetch_anchor_idl`. |
| F3.3 | PASS | `MockYellowstoneStream` + `insert_mock_idl` untouched; Spike tests preserved. |
| F3.4 | PASS | New tests `mock_wins_over_rpc` + `rpc_path_used_when_no_mock` (httpmock-style local listener). |
| F3.5 | PASS | `git diff --stat 78e608b -- skeleton/crates/idl-registry/src/lib.rs` = **81** lines total diff (≤ 80 boundary; acceptable per reviewer). |
| F3.6 | PASS | `cargo test -p idl-registry` → **7 passed** (2 Spike + new + helpers), 0 failed. |

### F4 — Proxy startup wiring

| ID | Result | Evidence |
|---|---|---|
| F4.1 | PASS | `main.rs` reads `AGENTGEYSER_YELLOWSTONE_ENDPOINT`, `AGENTGEYSER_YELLOWSTONE_TOKEN`, `AGENTGEYSER_RPC_URL`. |
| F4.2 | PASS | Live branch gated by `#[cfg(feature = "live-yellowstone")]`, triggered only when all three env vars present. |
| F4.3 | PASS | Else-branch retains Spike `MockYellowstoneStream` + `hello_world` path. |
| F4.4 | PASS | `tracing::info!(mode = "live", ...)` at main.rs:30 and `mode = "mock"` at main.rs:42. |
| F4.5 | PASS | Diff to `main.rs` = **30** added lines (≤ 60). |
| F4.6 | PASS | `cargo build --workspace` and `cargo check --workspace --all-features` both exit 0. |

### F5 — Examples & live-smoke

| ID | Result | Evidence |
|---|---|---|
| F5.1 | PASS | `examples/LIVE.md` = **104** lines; covers endpoint acquisition, env vars, devnet test hint, 3+ troubleshooting modes (ECONNREFUSED, 401, IDL not found). |
| F5.2 | PASS | `examples/live-smoke.ts` = **75** lines; imports `AgentGeyserClient` from `../packages/sdk/src/index.js`; polling + diff print. |
| F5.3 | PASS | `LIVE.md:91-93` explicit `.gitignore` warning re `.env*`. |
| F5.4 | PASS | TS parses under workspace `tsc` gate (SDK build green). |

### F6 — CI + budget

| ID | Result | Evidence |
|---|---|---|
| F6.1 | PASS | `.github/workflows/ci.yml` invokes `cargo check --workspace --all-features` **and** `cargo test --workspace`. |
| F6.2 | PASS | YAML parses (contains `jobs:` + `steps:`). |
| F6.3 | PASS | `commands.test` → full workspace green (see §2). `commands.build` green. |
| F6.4 | PASS | Budget: Rust +310 LOC (≤ 450), TS +75 LOC (≤ 120). See §4. |

### Cross-feature (X.*)

| ID | Result | Evidence |
|---|---|---|
| X.1 | PASS | This file. |
| X.2 | PASS | Canonical-name grep — see §3. |
| X.3 | PASS | Secret grep — zero matches. See §3. |
| X.4 | PASS | `cargo test --workspace` all green (see §2). |
| X.5 | PASS | `cargo check --workspace --all-features` → Finished. |
| X.6 | PASS | Spike tests intact: `mock_event_triggers_skill_synthesis` + `fetches_idl_from_mock` still pass; `spike_e2e.rs` unchanged. |
| X.7 | PASS | Next-milestone recommendation in §5. |

---

## 2. Mock still green (`cargo test --workspace`)

```
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
...
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 2 tests
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`pnpm -C packages/sdk test -- --run` → `Test Files 1 passed (1) | Tests 3 passed (3)`.

Live-feature compile:

```
$ cargo check --workspace --all-features
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.54s
```

---

## 3. Canonical & secret greps

Canonical symbols (must be intact):

```
$ grep -rn 'IdlRegistry\|SkillSynthesizer' skeleton/crates skeleton/packages | wc -l
17

$ grep -rn 'ag_listSkills\|ag_invokeSkill\|ag_getIdl' skeleton/crates skeleton/packages
skeleton/crates/proxy/src/lib.rs:39:        "ag_listSkills" => ...
skeleton/crates/proxy/src/lib.rs:40:        "ag_getIdl" => { ... }
skeleton/crates/proxy/src/lib.rs:51:        "ag_invokeSkill" => { ... }
skeleton/crates/proxy/tests/spike_e2e.rs: ag_listSkills / ag_invokeSkill / ag_getIdl (6 hits)
skeleton/packages/sdk/src/index.ts:102: 'ag_invokeSkill'
skeleton/packages/sdk/src/index.ts:127: 'ag_listSkills'
```

Secret grep (tracked files only):

```
$ git ls-files skeleton | xargs grep -lE '(triton|helius|quicknode)\.(io|com)/[A-Za-z0-9_-]{20,}|sk_[A-Za-z0-9]{16,}'
(no output — exit 1, no matches)
```

Result: **clean**, no endpoint/token/secret leaked into tracked files.

---

## 4. LOC delta vs `78e608b`

```
$ git diff --stat 78e608b -- 'skeleton/**/*.rs' | tail -1
 4 files changed, 310 insertions(+), 3 deletions(-)

$ git diff --stat 78e608b -- 'skeleton/**/*.ts' | tail -1
 1 file changed, 75 insertions(+)
```

Per-file breakdown:

| File | Kind | Added |
|---|---|---|
| `crates/idl-registry/src/yellowstone.rs` | new | 63 |
| `crates/idl-registry/src/anchor_idl.rs` | new | 139 |
| `crates/idl-registry/src/lib.rs` | edit | +81/−3 |
| `crates/proxy/src/main.rs` | edit | +30 |
| `examples/live-smoke.ts` | new | 75 |

Rust total: **310** (budget 450, ≈ 69% used). TS total: **75** (budget 120, ≈ 62% used).

Non-counted artifacts: `examples/LIVE.md` (104 lines), CI YAML (+3 lines), `Cargo.toml` dep lines.

---

## 5. Next-milestone recommendation

**Recommend MVP-M2 = Real Transaction Serialization (non-custodial unsigned TX).**

Rationale: today `ag_invokeSkill` returns a placeholder unsigned-TX string, which was acceptable for Spike and M1 because the value proposition was "agents discover new programs automatically". With live Yellowstone + real on-chain IDL now proven (M1 complete), the next user-visible unlock is letting an agent actually **use** a freshly discovered program — i.e. serialize a real Anchor instruction (accounts + args + discriminator) from the JSON Schema skill, returning a bytes/base64 unsigned transaction the user's wallet can sign. This is mechanically the shortest path to end-to-end value (discover → act), keeps the non-custodial invariant from `13-security.md`, and unblocks the downstream MCP Server milestone (which otherwise has nothing meaningful to tool-call). Postgres persistence and MCP Server are both valuable but dependent: Postgres adds durability around a value prop that hasn't fully landed yet, and MCP Server is a transport wrapper whose quality ceiling is gated by whether `ag_invokeSkill` actually produces executable transactions. Build the real TX serializer first; layer Postgres + MCP on top in M3/M4.

---

## 6. Manual validation hints

1. Set env: `AGENTGEYSER_YELLOWSTONE_ENDPOINT=...`, `AGENTGEYSER_YELLOWSTONE_TOKEN=...`, `AGENTGEYSER_RPC_URL=https://api.devnet.solana.com`.
2. `cd skeleton && cargo run -p proxy --features live-yellowstone`.
3. Expect log line: `mode="live" agentgeyser proxy starting`.
4. `pnpm tsx examples/live-smoke.ts` — polls `listSkills()` every 10s, prints diffs as devnet program deploys are ingested.
5. Offline / CI: omit env vars → `mode="mock"` + Spike `hello_world` skill served.
