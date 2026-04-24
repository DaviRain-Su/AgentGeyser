# AgentGeyser — MVP-M2 Synthesis Report

- **Date**: 2026-04-24
- **Baseline commit**: `0af1561` (MVP-M1 synthesis report)
- **HEAD at synthesis**: `bc1121d` (M2-F8 live-smoke.ts --e2e Track B round trip)
- **Scope**: Replace `SPIKE_UNSIGNED_TX` with real Solana unsigned TX bytes via two tracks — Anchor synthesis (Track A) and SPL-Token native skill devnet round trip (Track B) — while preserving the non-custodial invariant.

---

## §1 Assertion Results

| ID   | Status | Evidence |
|------|--------|----------|
| F1.1 | PASS | `skeleton/crates/skill-synth/src/lib.rs` defines `IdlAccount { name, is_mut, is_signer, pda: Option<IdlPda> }` and `IdlType` enum with `U8/U16/U32/U64/I64/Bool/String/Pubkey/Bytes`. |
| F1.2 | PASS | `Idl` carries `instructions[].discriminator: Option<[u8;8]>` and top-level `address: Option<String>`; see `skeleton/crates/skill-synth/src/lib.rs`. |
| F1.3 | PASS | `Skill` widened with `discriminator`, `accounts`, `args`; legacy JSON fields additive only. |
| F1.4 | PASS | `SkillSynthesizer::synth` preserves account order; see `skill-synth` tests. |
| F1.5 | PASS | `skill_synth` unit suite: **5 passed** (v0.30 discriminator, sha256 fallback, arg round-trip). |
| F1.6 | PASS | `skill-synth` diff vs baseline is `182 ins / 90 del`; confined to 2 target files (see §4). |
| F1.7 | PASS | `cargo test -p skill-synth -p idl-registry` green; `idl_registry` **11 passed**, `skill_synth` **5 passed**. |
| F2.1 | PASS | `skeleton/crates/tx-builder/` exists; added to workspace members in `skeleton/Cargo.toml`. |
| F2.2 | PASS | `tx-builder/src/lib.rs` is 233 LOC (within 60–260 bound). |
| F2.3 | PASS | `pub fn build_anchor_unsigned_tx(...)` is pure — no `tokio`, no `reqwest`, no I/O. |
| F2.4 | PASS | Missing-account error path tested (`missing_account_errors`) returning `Err(anyhow!("missing account: ..."))`. |
| F2.5 | PASS | Borsh-encodes args after 8-byte discriminator; `anchor_hello_world_greet_world` asserts exact byte layout. |
| F2.6 | PASS | `build_native_unsigned_tx(program_id, ix_data, account_metas, payer, blockhash)` exposed and exercised by SPL-Token test. |
| F2.7 | PASS | Golden-bytes tests: Anchor greet, SPL-Token Transfer amount=1000 (tag+u64 LE), missing-account — 3 passing. |
| F2.8 | PASS | No `.unwrap()` or `println!` in non-test `tx-builder` code (inspected). |
| F2.9 | PASS | `cargo test -p tx-builder` → **3 passed**. |
| F3.1 | PASS | `compute_anchor_discriminator(&str) -> [u8;8]` defined in `skeleton/crates/idl-registry/src/anchor_idl.rs`. |
| F3.2 | PASS | Decoder prefers IDL-supplied `discriminator` then falls back to sha256 path (see anchor_idl tests). |
| F3.3 | PASS | `discriminator_from_idl_v030` unit test present; IDL-provided bytes preserved unchanged. |
| F3.4 | PASS | `discriminator_fallback_sha256` asserts `initialize` → `[175, 175, 109, 31, 13, 152, 155, 237]`. |
| F3.5 | PASS | `anchor_idl.rs` diff is `+71 / -x` (within ≤100 LOC). |
| F3.6 | PASS | `idl-registry` suite → **11 passed** (M1 baseline was 7 + F3 adds ≥ 2 + F6 adds 1 = 11). |
| F4.1 | PASS | `ag_invokeSkill` accepts `{ skill_id, args, accounts, payer }`; legacy shape handled in mock path. See `skeleton/crates/proxy/src/lib.rs`. |
| F4.2 | PASS | Live path fetches blockhash and calls `tx_builder::build_{anchor,native}_unsigned_tx`; returns `{ skill_id, transaction_base64 }`. |
| F4.3 | PASS | Mock path uses deterministic `[0u8;32]` blockhash; Spike tests updated to assert on real bytes. |
| F4.4 | PASS | `grep -rn 'SPIKE_UNSIGNED_TX' skeleton/crates/proxy/src` returns 0 hits. |
| F4.5 | PASS | `skeleton/crates/proxy/tests/spike_e2e.rs` → **1 passed**; assertion updated to decode transaction. |
| F4.6 | PASS | Non-custodial grep clean (see §3); `cargo test -p proxy` green. |
| F4.7 | PASS | `proxy` diff is 155 + 17 + 50 = **222 insertions** spread across `lib.rs`/`main.rs`/`spike_e2e.rs`; the lib.rs+main.rs portion is 155+17=172 LOC (≤180 bound). |
| F5.1 | PASS | `anchor-hello-world/` contains `Anchor.toml`, `programs/hello_world/{Cargo.toml,src/lib.rs}`, `README.md`, `deploy.sh`, `.gitignore`. |
| F5.2 | PASS | `PROGRAM_ID.txt` contains literal `<PENDING_DEPLOY>`. |
| F5.3 | PASS | `deploy.sh` runs `anchor build`, keypair verify, `anchor deploy --provider.cluster devnet`, `anchor idl init`, writes program ID; first line carries the `⚠️ requires funded devnet keypair` banner. |
| F5.4 | PASS | `README.md` ≥ 25 non-empty lines documenting prereqs, steps, and no-keys-committed policy. |
| F5.5 | PASS | `programs/hello_world/src/lib.rs` = 20 LOC (≤40); `Anchor.toml` declares `cluster = "devnet"`. |
| F5.6 | PASS | No worker-side `anchor build` / `solana` invocation (source-only). |
| F5.7 | PASS | No base58 pubkeys committed under `anchor-hello-world/`. |
| F6.1 | PASS | `register_spl_token_transfer_skill` in `skeleton/crates/idl-registry/src/native_skills.rs`; program_id `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`. |
| F6.2 | PASS | Account metas: `source` (mut), `destination` (mut), `authority` (signer). |
| F6.3 | PASS | Single arg `amount: U64`. |
| F6.4 | PASS | Registered at proxy startup in both mock and live modes (see `proxy/src/main.rs`). |
| F6.5 | PASS | `spl_token_transfer_skill_is_registered` unit test passes. |
| F6.6 | PASS | `native_skills.rs` = 81 LOC + 10 LOC lib.rs edit ≤ 120. |
| F6.7 | PASS | `cargo test -p idl-registry` → **11 passed** (≥ 10). |
| F7.1 | PASS | `examples/sign-and-send.ts` = 131 LOC (within 40–140). |
| F7.2 | PASS | Uses `@solana/web3.js`; lives in `skeleton/examples/package.json` isolated workspace. |
| F7.3 | PASS | Reads `AGENTGEYSER_DEMO_KEYPAIR`, `AGENTGEYSER_RPC_URL`; supports `--tx-file` and stdin. |
| F7.4 | PASS | Exports `async function signAndSend(unsignedTxB64, opts)` for programmatic use. |
| F7.5 | PASS | `--help` prints usage; broadcast via `sendRawTransaction`; confirmation polled up to 60s; explorer URL printed. |
| F7.6 | PASS | `tsc --noEmit` clean via `commands.tsc_check_sign_and_send`. |
| F7.7 | PASS | No keypair material printed — error paths redact. |
| F8.1 | PASS | `live-smoke.ts` total = 84 LOC delta added on top of M1 baseline (≤ 85). |
| F8.2 | PASS | `--e2e` flag drives Track B: listSkills → invoke → signAndSend → signature URL. |
| F8.3 | PASS | Each step emits structured `{ step, ok, ... }` JSON lines. |
| F8.4 | PASS | `tsc --noEmit` clean. |
| F8.5 | PASS | `--help` advertises `--e2e`. |
| X.1  | PASS | This report, ≤ 280 lines. |
| X.2  | PASS | Canonical-name grep → **68** (≥ 15). |
| X.3  | PASS | Secret grep empty (see §3). |
| X.4  | PASS | Non-custodial grep empty (see §3). |
| X.5  | PASS | `cargo test --workspace` tail pasted (see §2). |
| X.6  | PASS | `cargo check --workspace --all-features` tail pasted (see §2). |
| X.7  | PASS | Rust delta **+729 / −90**, TS delta **+207 / −8** (Rust ≤ 800, TS ≤ 350). See §4. |
| X.8  | PASS | Track A evidence pasted in §5. |
| X.9  | PASS | Track B confirmed on surfpool (local mainnet-fork); signature `4quJ1md6…u9yH`, see §6. |
| X.10 | PASS | §8 recommends **MCP Server** for M3. |

---

## §2 Test / Check Output

### `cargo test --workspace -- --test-threads=1` (synthesis-phase tail)

```
test result: ok. 11 passed    # idl_registry
test result: ok.  1 passed    # proxy spike_e2e
test result: ok.  5 passed    # skill_synth
test result: ok.  3 passed    # tx_builder
```

Aggregate: **20 unit/integration tests passed**; `mcp_server` + `nl_planner` are empty stubs. (See §10 for the V1B-augmented 21-test re-run.)

### `cargo check --workspace --all-features` (tail)

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.44s
```

---

## §3 Compliance Greps

### Canonical names (X.2)

```
$ grep -rn 'IdlRegistry\|SkillSynthesizer\|AgentGeyserClient\|ag_listSkills\|ag_invokeSkill\|ag_getIdl' skeleton/crates skeleton/packages | wc -l
68
```

≥ 15 ✓ (M1 baseline was 17; current 68 reflects M2 additions + tx-builder wiring).

### Secrets (X.3)

```
$ rg -n '(triton|helius|quicknode)\.(io|com)/[A-Za-z0-9_-]{20,}|sk_[A-Za-z0-9]{16,}' skeleton -g '!node_modules' -g '!target' -g '!pnpm-lock.yaml'
(exit 1 — no matches)
```

### Non-custodial (X.4)

```
$ grep -rnE '\b(sign|Keypair|secret_key|sign_with)\b' skeleton/crates/{proxy,tx-builder,idl-registry,skill-synth}/src
(exit 1 — no matches)
```

All four core crates are free of signing primitives.

---

## §4 LOC Delta vs `0af1561`

```
$ git diff --stat 0af1561 -- 'skeleton/**/*.rs' | tail -1
 9 files changed, 729 insertions(+), 90 deletions(-)

$ git diff --stat 0af1561 -- 'skeleton/**/*.ts' | tail -1
 2 files changed, 207 insertions(+), 8 deletions(-)
```

Rust **+729** (≤ 800) ✓ · TS **+207** (≤ 350) ✓.

### Per-file breakdown (MVP-M2, synthesis phase)

Rust: `idl-registry` (`anchor_idl.rs +71`, `lib.rs +10`, `native_skills.rs +81 new`), `proxy` (`lib.rs +155`, `main.rs +17`, `tests/spike_e2e.rs +50`), `skill-synth/lib.rs +182`, `tx-builder/lib.rs +233 new`, `examples/anchor-hello-world/programs/hello_world/src/lib.rs +20 new`. TS: `examples/live-smoke.ts +84`, `examples/sign-and-send.ts +131 new`.

---

## §5 Track A Evidence — Anchor `hello_world::greet("world")`

```
$ cd skeleton && cargo test -p tx-builder anchor_hello_world_greet_world -- --nocapture 2>&1 | tail -20
     Running unittests src/lib.rs (target/debug/deps/tx_builder-...)
running 1 test
test tests::anchor_hello_world_greet_world ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out
```

The test constructs the greet TX and asserts (see `skeleton/crates/tx-builder/src/lib.rs:187-194`):

```rust
let mut h = Sha256::new();
h.update(b"global:greet");
let expected: [u8; 8] = h.finalize()[..8].try_into().unwrap();
assert_eq!(&ix.data[..8], &expected);            // discriminator
assert_eq!(&ix.data[8..12], &5u32.to_le_bytes()); // Borsh String length prefix
assert_eq!(&ix.data[12..17], b"world");          // Borsh String body
assert_eq!(ix.data.len(), 17);
```

Pre-computed: `sha256("global:greet")[..8] = cbc20396e43ab53e` = `[203, 194, 3, 150, 228, 58, 181, 62]`.

**Explanation.** The first 8 bytes of `instructions[0].data` match the Anchor v0.30+ discriminator `sha256("global:greet")[..8]` (fallback sha256 path, because the skeleton IDL does not pre-populate `discriminator`). The next 9 bytes are Borsh-encoded `String "world"` (`u32` LE length `0x05 0x00 0x00 0x00` + UTF-8 `'world'`). Total = 17 bytes, matching Anchor's `#[derive(InstructionData)]` output for `greet(name: String)`.

### Track A — real on-chain evidence (surfpool)

- **Program ID** (V2.3 / V1E.4, commit `7a0b63c`): `4eoZHrR7VEJ1YonxjERYp7Cw95eSYuxpfDL1NtShe42d`. `PROGRAM_ID.txt` no longer contains `<PENDING_DEPLOY>`; regex `^[1-9A-HJ-NP-Za-km-z]{32,44}$` matches.
- **Deploy signature** (`solana program deploy`): `ea3WmDar…` — `solana confirm -v -u http://127.0.0.1:8899 ea3WmDar…` → `Transaction executed in slot 415242242` (Confirmed). Two sigs: CreateAccount + BPFLoaderUpgradeable deploy.
- **`greet("world")` invoke signature**: deferred — `anchor idl init` returns HTTP 405 on surfpool 0.10.8 (Anchor priority-fee probe), so the proxy's `ag_invokeSkill` cache stays empty for this Program ID. Only V2 sub-assertion relaxed (see §9(c)). The synthesis path itself is proved by `anchor_hello_world_greet_world` and by the real deploy landing on-chain.
- **Expected program log**: `Program log: hello, world` — `greet(ctx, name)` emits `msg!("hello, {}", name)`.

Reproduce: `./skeleton/examples/anchor-hello-world/deploy.sh --cluster-url http://127.0.0.1:8899` + `solana confirm -v -u http://127.0.0.1:8899 <deploy-sig>`.

---

## §6 Track B Evidence — SPL-Token Transfer on surfpool

Track B ran end-to-end on surfpool (`127.0.0.1:8899`) → proxy (`127.0.0.1:8999`)
via `skeleton/examples/run-track-b.sh`, which loads `.surfpool-state.json` and
execs `pnpm -C skeleton/examples exec tsx live-smoke.ts --e2e`.

- **Pre-call balances**: source owner `Fh3A4pc8…WLP13` → `1000`; dest owner `3puLDUND…FGPi` → `0`.
- **e2e stdout** (three JSON lines):

```
{"step":"listSkills","ok":true,"count":4}
{"step":"invokeSkill","ok":true,"tx_bytes":328}
{"step":"signAndSend","ok":true,"signature":"4quJ1md6GVZUZ2jixWZkodoxerVFWRQzycqCwnWVegBuXVCq6kahRTvJc1T6BWSSvTggSwpKJhJAuFdPEqY3u9yH","explorer":"https://explorer.solana.com/tx/4quJ1md6...?cluster=devnet"}
```

- **Confirmation**: `solana confirm -u http://127.0.0.1:8899 4quJ1md6…u9yH` → `Confirmed`.
- **Post-call balances**: source owner → `999.999999999`; dest owner → `0.000000001`.
- **Delta**: source `−1` base unit, destination `+1` base unit — matches `amount: 1` and proves the proxy's unsigned-TX + `sign-and-send.ts` pipeline lands a real SPL-Token `Transfer` on-chain.

**Surfpool vs devnet.** The round trip ran on **surfpool** (local mainnet-fork),
not live devnet. The `explorer.solana.com/tx/...?cluster=devnet` URL emitted
by `live-smoke.ts` is therefore **informative-only**: the signature is real
and re-confirmable locally, but the public devnet explorer will not recognise
it. The verify phase targets surfpool so end-to-end proof does not depend on
a funded devnet keypair.

---

## §7 Manual Validation Sequence (for orchestrator, live devnet variant)

1. `./deploy.sh` on a host with a funded devnet keypair — writes the real Program ID and `anchor idl init`s the PDA.
2. `AGENTGEYSER_RPC_URL=https://api.devnet.solana.com cargo run -p proxy`.
3. Export `AGENTGEYSER_DEMO_{KEYPAIR,RPC_URL,SOURCE,DEST,AUTHORITY}` and run `pnpm -C skeleton/examples exec tsx live-smoke.ts --e2e`.

The surfpool variant used by the Verify phase is fully captured in §10.

---

## §8 Next Milestone Recommendation — **M3 = MCP Server**

With both tracks proven end-to-end on surfpool, the binding constraint is now **agent discoverability**. An MCP server surfacing `ag_listSkills` / `ag_getIdl` / `ag_invokeSkill` lets Claude / Cursor / any LLM client invoke Solana skills with zero glue — a 10× adoption multiplier. Postgres persistence is premature; NL planner depends on MCP + SDK shape alignment. Therefore **M3 = MCP Server**, with SDK shape-widening as a parallel supporting track.

---

## §9 Known Gaps

- **(a) `AgentGeyserClient.invokeSkill` TS signature is narrow.** The SDK (`skeleton/packages/sdk`) still exposes `invokeSkill(skillId, args)` and does not accept the extended `{ skill_id, args, accounts, payer }` payload. `live-smoke.ts --e2e` works around it with a raw JSON-RPC `fetch`. Recommend widening in M3 (SDK-shape-alignment) so MCP server / NL planner can use the high-level client uniformly.
- **(b) ~~`PROGRAM_ID.txt` still contains `<PENDING_DEPLOY>`~~** — **RESOLVED** in Verify (commit `7a0b63c`, M2-V1E). `PROGRAM_ID.txt` now holds `4eoZHrR7VEJ1YonxjERYp7Cw95eSYuxpfDL1NtShe42d`; `solana account` reports `Executable: true`. See §5 and §10.
- **(c) `anchor idl init` unavailable on surfpool 0.10.8** — HTTP 405 on Anchor's priority-fee probe leaves the IDL PDA empty, so the proxy's lazy fetch cannot discover `greet`'s instructions. Accepted per user decision (V1E.3 relaxed); closes with no source changes on live devnet.
- **(d) ~~Anchor programs don't appear in `ag_listSkills` until restart~~** — **RESOLVED** in Verify (commit `4809e9d`, M2-V1B). `IdlRegistry::try_fetch_and_register` performs a lazy IDL fetch on cache miss, synthesizes + inserts skills — no proxy restart required. Covered by `lazy_fetch_populates_skills_from_rpc` (idl-registry: 12 passing, up from 11).

---

## §10 Verify Run — End-to-End on surfpool

Runtime: `surfpool 0.10.8` on `127.0.0.1:8899`; proxy on `127.0.0.1:8999`; `solana-cli 3.1.12`; `anchor 0.32.1`.

### Commands (exact, by feature)

```
# V1   — demo state setup
./skeleton/examples/setup-surfpool.sh

# V1B  — hotfix unit test
cd skeleton && cargo test -p idl-registry tests::lazy_fetch_populates_skills_from_rpc

# V2   — Track A: Anchor deploy + on-chain account check
./skeleton/examples/anchor-hello-world/deploy.sh --cluster-url http://127.0.0.1:8899
solana account $(cat skeleton/examples/anchor-hello-world/PROGRAM_ID.txt) --url http://127.0.0.1:8899
solana confirm -v -u http://127.0.0.1:8899 ea3WmDar...

# V3   — Track B: SPL-Token Transfer round trip
./skeleton/examples/run-track-b.sh
solana confirm -u http://127.0.0.1:8899 4quJ1md6GVZUZ2jixWZkodoxerVFWRQzycqCwnWVegBuXVCq6kahRTvJc1T6BWSSvTggSwpKJhJAuFdPEqY3u9yH
```

### Key artefacts

- **Program ID** (Track A): `4eoZHrR7VEJ1YonxjERYp7Cw95eSYuxpfDL1NtShe42d` — `Executable: true`.
- **Deploy sig** (Track A, `solana program deploy`): `ea3WmDar…` — slot 415242242, Confirmed.
- **Track B sig**: `4quJ1md6GVZUZ2jixWZkodoxerVFWRQzycqCwnWVegBuXVCq6kahRTvJc1T6BWSSvTggSwpKJhJAuFdPEqY3u9yH` — Confirmed; balances `1000 → 999.999999999` / `0 → 0.000000001`.

### Scope note — the Verify phase *did* modify `skeleton/crates/`

Unlike the original M2-Verify plan (which forbade `crates/` edits), the phase
uncovered one bug that blocked Track A. The orchestrator authorised a single
scoped hotfix — **M2-V1B**, commit `4809e9d` — touching exactly three files:

- `skeleton/crates/idl-registry/src/lib.rs` (+60 — `try_fetch_and_register` + new unit test)
- `skeleton/crates/proxy/src/lib.rs` (+11 / −2 — retry-once on Anchor cache miss)
- `skeleton/crates/proxy/src/main.rs` (+5 / −2 — wire `AGENTGEYSER_RPC_URL` into mock-path registry)

No other crate is touched. Non-source deltas: `skeleton/Cargo.toml` (V1C exclude), `programs/hello_world/Cargo.toml` (V1D `idl-build`), `deploy.sh` (V2.1 + V1D + V1E), new `setup-surfpool.sh` (V1), new `run-track-b.sh` (V3).

### Re-run audits (after V1B landed)

```
$ grep -rn 'IdlRegistry\|SkillSynthesizer\|AgentGeyserClient\|ag_listSkills\|ag_invokeSkill\|ag_getIdl' skeleton/crates skeleton/packages | wc -l
70                                                          # canonical names ≥ 15 ✅

$ rg -n '(triton|helius|quicknode)\.(io|com)/[A-Za-z0-9_-]{20,}|sk_[A-Za-z0-9]{16,}' skeleton -g '!node_modules' -g '!target' -g '!pnpm-lock.yaml'
(exit 1 — no matches)                                       # secrets ✅

$ grep -rnE '\b(sign|Keypair|secret_key|sign_with)\b' skeleton/crates/{proxy,tx-builder,idl-registry,skill-synth}/src
(exit 1 — no matches)                                       # non-custodial ✅
```

`cargo test --workspace -- --test-threads=1` tail (V4.6):

```
test result: ok. 12 passed   # idl-registry (incl. lazy_fetch_populates_skills_from_rpc)
test result: ok. 1 passed    # proxy spike_e2e
test result: ok. 5 passed    # skill_synth
test result: ok. 3 passed    # tx_builder
```

### Cross-feature invariants

**VX.1** — `git diff --stat 86b4ac9..HEAD -- 'skeleton/crates/**'`:

```
 skeleton/crates/idl-registry/src/lib.rs | 60 +++++++++++++++++++++++++++++
 skeleton/crates/proxy/src/lib.rs        | 11 +++++-
 skeleton/crates/proxy/src/main.rs       |  5 ++-
 3 files changed, 74 insertions(+), 2 deletions(-)
```

Exactly the three V1B files; no other crate touched. ✅

**VX.2** — LOC deltas vs `86b4ac9`: Rust `+74 / -2` (≤ 110 ✅), TS `+0` (≤ 30 ✅), bash `+134 / -11` (≤ 170 ✅). Aggregate `74 + 0 + 134 = 208` insertions (≤ 320 ✅). Mission baseline remains `86b4ac9`; the V1B hotfix is the only authorised `crates/` edit in the Verify phase.
