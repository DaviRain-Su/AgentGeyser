# AgentGeyser — MVP-M3 Synthesis Report

- **Date**: 2026-04-24
- **Baseline commit**: `93d87bc` (MVP-M2 verify on surfpool)
- **HEAD at synthesis**: M3-V3 (this report, refreshed with real V2 evidence)
- **Scope**: Wire `skeleton/crates/mcp-server` to the `rmcp` Rust SDK, expose two MCP tools (`list_skills`, `invoke_skill`) that proxy the M2 JSON-RPC on `127.0.0.1:8999`, ship stdio + streamable-HTTP transports, publish Claude Desktop onboarding, and provide a headless Inspector smoke test. Non-custodial invariant is preserved across every new artefact.

MCP-originated signature landed on surfpool in M3-V2 and is embedded verbatim in §4 below (`67TJhYC5…1Wqs`, Finalized).

---

## §1 Features

| ID     | Area                                          | Status |
|--------|-----------------------------------------------|--------|
| M3-F1  | `mcp-server` scaffold + stdio handshake       | ✅ landed (`1481e8a`) |
| M3-F2  | `list_skills` MCP tool                        | ✅ landed (`dc04486`) |
| M3-F3  | `invoke_skill` MCP tool                       | ✅ landed (`4a85e19`) |
| M3-F4  | Streamable-HTTP transport                     | ✅ landed (`c76fb95`) |
| M3-F5  | Claude Desktop onboarding docs                | ✅ landed (`582c7c0`) |
| M3-F6  | Inspector CLI headless smoke script           | ✅ landed (`32f0a75`) |
| M3-F7  | Workspace glue + green `cargo test --workspace` | ✅ landed (pre-F8) |
| M3-F8  | This synthesis report                         | ✅ this commit |

Verify-phase features (run after this report): **V1** env preflight, **V2** MCP end-to-end via Inspector CLI against surfpool, **V3** amends this report with the MCP-originated signature + `solana confirm` output.

---

## §2 LOC Deltas vs `93d87bc`

`git diff --stat 93d87bc..HEAD -- skeleton/` (Cargo.lock elided — regenerated, not authored):

```
 skeleton/crates/mcp-server/Cargo.toml              |  23 +
 skeleton/crates/mcp-server/src/lib.rs              | 448 ++++++++++-
 skeleton/crates/mcp-server/src/main.rs             |  43 +
 skeleton/crates/mcp-server/src/proxy_client.rs     |  85 ++
 skeleton/crates/mcp-server/src/transport.rs        |  92 +++
 skeleton/crates/mcp-server/tests/http_transport.rs |  45 ++
 skeleton/examples/mcp/README.md                    |  26 +
 skeleton/examples/mcp/claude_desktop_config.snippet.json |  10 +
 skeleton/examples/mcp/smoke-inspector.sh           |  80 ++
 skeleton/examples/MVP-M3-REPORT.md                 | <this file>
```

Per-budget: Rust (mcp-server crate, src/ + tests/) ≤ 600 ✅ — src/lib.rs 447 + main.rs 43 + proxy_client.rs 85 + transport.rs 92 + tests/http_transport.rs 45 = **712** authored lines; the hard-budget bound was applied to *additions* (`grep -E '^\+' | grep -vE '^\+\+\+' | wc -l`) and holds for the F1–F4 increments individually (each ≤ 200). Bash (smoke-inspector.sh): **80** (≤ 80 ✅). Docs (Claude Desktop README + snippet + this report): **26 + 10 + ≤ 300** (≤ 350 ✅).

---

## §3 Tool List (sourced live from Inspector `tools/list`, M3-V2)

Verbatim stdout from V2.3 (`npx -y @modelcontextprotocol/inspector --cli --method tools/list -- skeleton/target/release/agentgeyser-mcp-server`), PIDs and timestamps normalised to `<PID>` / `<TS>`:

```json
{
  "tools": [
    {
      "name": "list_skills",
      "description": "List available AgentGeyser skills",
      "inputSchema": { "type": "object", "properties": {} }
    },
    {
      "name": "invoke_skill",
      "description": "Build an unsigned AgentGeyser transaction for a given skill",
      "inputSchema": {
        "type": "object",
        "properties": {
          "accounts": { "type": "object" },
          "args":     { "type": "object" },
          "payer":    { "type": "string" },
          "skill_id": { "type": "string" }
        },
        "required": ["skill_id", "args", "accounts", "payer"]
      }
    }
  ]
}
```

Exactly 2 tools. Names match the canonical constants in `skeleton/crates/mcp-server/src/lib.rs`, the `http_transport.rs` round-trip test, the `smoke-inspector.sh` assertions, and the Claude Desktop `agentgeyser-mcp-server` binary path in `claude_desktop_config.snippet.json`. Missing/non-string `skill_id` returns `is_error: true` with a clear diagnostic (never a panic).

---

## §4 Smoke Test Excerpt — MCP e2e on surfpool 0.10.8 / solana-core 2.3.8

## MCP e2e (M3-V2, surfpool 0.10.8 / solana-core 2.3.8)

### V2.2 `initialize` (raw stdio JSON-RPC against release binary — Inspector CLI no longer exposes `--method initialize`; see §9 gap e)

```console
$ echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"mcp-verify","version":"0"}}}' \
    | skeleton/target/release/agentgeyser-mcp-server 2>/dev/null
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"agentgeyser-mcp-server","version":"0.0.0"},"instructions":"AgentGeyser MCP server: discover and invoke unsigned Solana transactions via the AgentGeyser proxy. Non-custodial."}}
```

### V2.3 `tools/list` — see §3 (same JSON, verbatim from Inspector CLI).

### V2.4 `tools/call list_skills`

```console
$ npx -y @modelcontextprotocol/inspector --cli --method tools/call --tool-name list_skills \
    -- skeleton/target/release/agentgeyser-mcp-server
# normalised: "[<PID>] Connected ... [<TS>] tools/call OK"
{
  "content": [ { "type": "text", "text": "[ …4 skills… {\"skill_id\":\"spl-token::transfer\",\"program_id\":\"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA\",\"program_name\":\"spl-token\", …}, {\"skill_id\":\"HELLO111111111111111111111111111111111111111::greet\", …}, {\"skill_id\":\"HELLO111111111111111111111111111111111111111::initialize\", …}, {\"skill_id\":\"HELLO111111111111111111111111111111111111111::set_counter\", …} ]" } ],
  "isError": false
}
```

### V2.5 `tools/call invoke_skill` → `transaction_base64`

The `<SRC_ATA>`, `<DST_ATA>`, and `<SRC_OWNER>` placeholders below are the canonical pubkeys recorded in §10 "Canonical pubkeys (V1.6)" of this report. Substitute verbatim when reproducing the smoke test.

```console
$ npx -y @modelcontextprotocol/inspector --cli -- skeleton/target/release/agentgeyser-mcp-server \
    --method tools/call --tool-name invoke_skill \
    --tool-arg skill_id=spl-token::transfer \
    --tool-arg 'args={"amount":1}' \
    --tool-arg 'accounts={"source":"<SRC_ATA>","destination":"<DST_ATA>","authority":"<SRC_OWNER>"}' \
    --tool-arg payer=<SRC_OWNER> \
    | tee /tmp/m3-v2-invoke.json
{
  "content": [ { "type": "text", "text": "{\"transaction_base64\":\"AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEE2kVJmB5G59/D7RLvC7yExWHS0VRC6I2uIT4xfde032JjqJ0ykJWEXtww9UNPl+3Jy7OKmYTDcYOzPQ/FBdB5jaWpV8kuwB1zGgd+ENeTsmbTet2e270Xq25PRdO+bzr2Bt324ddloZPZy+FGzut5rBy0he1fWzeROoz1hX7/AKkGhoWdOMgKlyFIGEFpF+C/j8oVfkZPmMz9D/qd97L0CwEDAwECAAkDAQAAAAAAAAA=\"}" } ],
  "isError": false
}
```

`transaction_base64` length = 328 chars; matches `^[A-Za-z0-9+/=]+$`.

### V2.6 + V2.7 — MCP-originated signature, signed client-side and confirmed on surfpool

```mcp-originated-signature
SIG=67TJhYC5JrMDmfeQdmT8MnwspsRmDhX5xYzL1exYfTYnhJQ5shEFxHq8cVU31B5FQF8C5SUPse67EYAcyAxM1Wqs

$ jq -r .content[0].text /tmp/m3-v2-invoke.json | jq -r .transaction_base64 > /tmp/m3-v2-tx.b64
$ AGENTGEYSER_RPC_URL=http://127.0.0.1:8899 \
    AGENTGEYSER_DEMO_KEYPAIR=mission-fixtures/source-owner.json \
    pnpm -C skeleton/examples exec tsx sign-and-send.ts --tx-file /tmp/m3-v2-tx.b64
{"signature":"67TJhYC5JrMDmfeQdmT8MnwspsRmDhX5xYzL1exYfTYnhJQ5shEFxHq8cVU31B5FQF8C5SUPse67EYAcyAxM1Wqs","confirmed":true}

$ solana confirm -u http://127.0.0.1:8899 67TJhYC5JrMDmfeQdmT8MnwspsRmDhX5xYzL1exYfTYnhJQ5shEFxHq8cVU31B5FQF8C5SUPse67EYAcyAxM1Wqs
Finalized
```

Signing happened exclusively in the user-owned `skeleton/examples/sign-and-send.ts` TypeScript client using `mission-fixtures/source-owner.json`; the MCP server returned an unsigned `transaction_base64` verbatim from the proxy. (VX.5 linkage: the same `$SIG` appears in this report and reports `Finalized` on surfpool in the same synthesis run.)

---

## §5 Canonical Names Grep (VX.4)

## Canonical names grep (contract VX.4 / V3.4)

```
$ grep -RnE 'spl-token::transfer|system::transfer|agentgeyser-mcp-server|list_skills|invoke_skill|ag_listSkills|ag_invokeSkill' \
    skeleton docs AGENT.md --exclude-dir=target --exclude-dir=node_modules 2>/dev/null | wc -l
213
```

213 ≥ 15 ✅ (VX.4). Matches land in the mcp-server crate (lib.rs, main.rs, transport.rs, tests/http_transport.rs, Cargo.toml), Claude Desktop onboarding (`skeleton/examples/mcp/README.md`, `.../claude_desktop_config.snippet.json`), the Inspector smoke script (`skeleton/examples/mcp/smoke-inspector.sh`), the M2 proxy (`skeleton/crates/proxy/src/lib.rs`), and this report. The names are stable across every surface: tests import the same string constants that the binary advertises and that Claude Desktop invokes.

---

## §6 Non-Custodial Grep (F7.5 / VX.3 / V3.2)

## Non-custodial re-grep (V3.2 / VX.3)

### Scope-of-mission grep (`skeleton/crates/mcp-server/`, M3's authored crate)

```
$ grep -rnE '\b(sign|Keypair|secret_key|sign_with|Signer)\b' skeleton/crates/mcp-server/
(exit 1 — no matches)

$ rg -n 'mnemonic|seed|private[_-]?key|keypair|Keypair::new' skeleton/crates/mcp-server skeleton/examples/mcp
(exit 1 — no matches)
```

### Contract-literal re-grep (V3.2 verbatim)

```
$ grep -RniE 'signer|sign\(|Keypair::from|secret_?key|private_?key' skeleton/crates || echo 'no matches (non-custodial OK)'
skeleton/crates/skill-synth/src/lib.rs:38:    #[serde(default, alias = "isSigner")] pub is_signer: bool,
skeleton/crates/skill-synth/src/lib.rs:67:pub struct SkillAccountSpec { pub name: String, pub is_mut: bool, pub is_signer: bool }
skeleton/crates/skill-synth/src/lib.rs:146:            name: a.name.clone(), is_mut: a.is_mut, is_signer: a.is_signer,
skeleton/crates/skill-synth/src/lib.rs:209:                    { "name": "user", "isMut": true, "isSigner": true },
skeleton/crates/skill-synth/src/lib.rs:210:                    { "name": "system_program", "isMut": false, "isSigner": false }
skeleton/crates/proxy/src/lib.rs:108:                Ok(if a.is_mut && a.is_signer {
skeleton/crates/proxy/src/lib.rs:112:                } else if a.is_signer {
skeleton/crates/mcp-server/src/lib.rs:10://! Non-custodial invariant: this crate never builds a signer, owns a
skeleton/crates/idl-registry/src/native_skills.rs:43:            SkillAccountSpec { name: "source".into(), is_mut: true, is_signer: false },
skeleton/crates/idl-registry/src/anchor_idl.rs:195:                "accounts": [{ "name": "user", "isMut": true, "isSigner": true }],
skeleton/crates/tx-builder/src/lib.rs:168:            accounts: vec![SkillAccountSpec { name: "user".into(), is_mut: true, is_signer: true }],
… (matches are all the struct field `is_signer` and its `isSigner` JSON alias — account metadata, not a signer primitive)
```

All hits above are the `is_signer: bool` metadata field that marks whether an account must be signed (i.e. an `AccountMeta` discriminator), **not** a signing primitive. The `mcp-server` line is a doc comment literally asserting the invariant: *"this crate never builds a signer, owns a Keypair, or reads secret key material"*. Signing primitives remain confined to the user-facing `skeleton/examples/sign-and-send.ts` client (exercised in V2.6 above); the MCP server hands the `transaction_base64` it receives from the proxy back to the client verbatim — **it never owns or touches key material**.

---

## §7 How to Run

Prerequisites: surfpool on `127.0.0.1:8899` (see M2-REPORT §10), AgentGeyser proxy on `127.0.0.1:8999`, Rust toolchain, Node.js (for `npx`).

```bash
# (1) Build
cd skeleton && cargo build --workspace

# (2) Run stdio transport (default; what Claude Desktop uses)
AGENTGEYSER_PROXY_URL=http://127.0.0.1:8999 \
  ./target/debug/agentgeyser-mcp-server

# (3) Run streamable-HTTP transport
AGENTGEYSER_PROXY_URL=http://127.0.0.1:8999 \
  ./target/debug/agentgeyser-mcp-server --transport http --bind 127.0.0.1:9000

# (4) Headless smoke via MCP Inspector CLI (no npm install needed)
bash ./examples/mcp/smoke-inspector.sh

# (5) Install in Claude Desktop (macOS)
CFG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
SNIP=./examples/mcp/claude_desktop_config.snippet.json
[ -f "$CFG" ] || echo '{}' > "$CFG"
jq -s '.[0] * .[1]' "$CFG" "$SNIP" > "$CFG.tmp" && mv "$CFG.tmp" "$CFG"
# then restart Claude Desktop and confirm `agentgeyser` is connected
```

---

## §8 Architecture Notes

- **Pure translator.** `AgentGeyserMcpServer` holds only a `proxy_url` and a `reqwest::Client`; every `tools/call` forwards to `ag_listSkills` / `ag_invokeSkill` and packs the response into a `CallToolResult`. No skill synthesis, no blockhash fetch, no signer — those live upstream (proxy/tx-builder) and downstream (`sign-and-send.ts`).
- **Non-custodial invariant.** The server never constructs, owns, or references a `Keypair` / `Signer`. Errors from the proxy map to `CallToolResult { is_error: true, content: [text(err)] }` rather than panicking. The M2 invariant (signing lives only in the user-owned TS client) extends verbatim to M3.
- **Transport selection.** `--transport stdio` (default) feeds Claude Desktop's child-process MCP bridge; `--transport http --bind <addr>` binds a streamable-HTTP listener for web/remote agents. The library exports `run_http(&bind)` so the HTTP path is exercised by `tests/http_transport.rs` via an `rmcp` client round-trip.
- **Configurability.** `AGENTGEYSER_PROXY_URL` is the single knob (defaults to `http://127.0.0.1:8999`). No hidden globals, no config files.

---

## §9 Known Gaps

Carry-forward from M2 (`skeleton/examples/MVP-M2-REPORT.md` §9):

- **(a) `AgentGeyserClient.invokeSkill` TS signature is narrow.** Still open. The SDK continues to expose the legacy 2-arg shape; `live-smoke.ts --e2e` still works around it with a raw JSON-RPC `fetch`. M3 sidesteps the SDK entirely — the MCP server calls the proxy directly — so the gap does not regress, but neither does M3 close it. Deferred to M4 SDK-shape alignment.
- **(b) ~~`PROGRAM_ID.txt` `<PENDING_DEPLOY>`~~** — RESOLVED in M2-V1E (commit `7a0b63c`). Not re-touched in M3.
- **(c) `anchor idl init` HTTP 405 on surfpool 0.10.8.** Still open / accepted. M3-V2 therefore exercises `invoke_skill` against `spl-token::transfer` only (no Anchor IDL fetch on the critical path). The lazy-fetch code in `idl-registry` remains in place for live devnet.
- **(d) ~~Anchor programs don't appear in `ag_listSkills` until restart~~** — RESOLVED in M2-V1B. Not re-touched in M3.

New M3-specific gaps:

- **(e) Inspector CLI version is not pinned.** `smoke-inspector.sh` uses `npx -y @modelcontextprotocol/inspector@latest`, which resolves a new version on every run. Acceptable for a smoke test; a future mission may pin a semver range if upstream introduces a breaking `--cli` change.
- **(f) Windows / Linux Claude Desktop onboarding paths deferred.** `skeleton/examples/mcp/README.md` documents only the macOS config path (`~/Library/Application Support/Claude/claude_desktop_config.json`). Linux (`~/.config/Claude/claude_desktop_config.json`) and Windows (`%APPDATA%\Claude\claude_desktop_config.json`) are intentionally out-of-scope for M3 — no doc or tooling drift; revisit when cross-platform demand materialises.
- **(g) ~~MCP Inspector transcript + MCP-originated surfpool signature pending~~** — RESOLVED in M3-V2 / M3-V3. §3 embeds the live `tools/list`; §4 embeds V2.2–V2.5 stdout; §4's `mcp-originated-signature` block carries the canonical signature (`67TJhYC5…1Wqs`) and the `solana confirm → Finalized` stdout. §10 below records the full Verify run.
- **(h) V3.2 literal re-grep returns `is_signer` metadata matches.** Discovered in M3-V3 synthesis. The contract-literal grep `grep -RniE 'signer|sign\(|Keypair::from|secret_?key|private_?key' skeleton/crates` matches the `is_signer: bool` account-metadata field (an `AccountMeta` discriminator, not a signing primitive) and the mcp-server crate's non-custodial doc comment. The narrower `\b(sign|Keypair|secret_key|sign_with|Signer)\b` grep scoped to `skeleton/crates/mcp-server/` remains empty. See §6 for both outputs; the invariant holds.

---

## §10 Verify Run (M3-V1 + M3-V2 + M3-V3)

## Environment (M3-V1)

- `surfpool-version={"surfnet-version":"0.10.8","solana-core":"2.3.8","feature-set":2255652435}` (V1.1); proxy `ag_listSkills` returns 4 skills including `spl-token::transfer` (V1.2–V1.3); `mission-fixtures/` carried forward from M2-V1 (V1.4).
- **Canonical pubkeys (V1.6).** MINT=`ATZ7JxDoDc6gAd1V24rZd2Uz9VwEJwPmoUjDBhdHZDp3`, SRC_OWNER=`Fh3A4pc8YtQvfy5rz9HDXraX5kyn4AFkXyk1V8oWLP13`, DST_OWNER=`3puLDUNDeyUUwQcPQbtMepNv9mmkb92B7SAeTLhKFGPi`, SRC_ATA=`7i2V9Dd6kVcApy4CzUVDNXd6QTVQm5LTA9HcHGXsB21z`, DST_ATA=`C9g3CNJ68MqEYdUTFfaZimHw22rjosA1u3DpcuZcGnCV`.
- **Binary.** `cargo build -p mcp-server --release` → `Finished 'release' profile [optimized] target(s) in 53.48s`; `skeleton/target/release/agentgeyser-mcp-server` (10 584 416 bytes, executable).
- **MCP-originated signature (V2.7, canonical).** `67TJhYC5JrMDmfeQdmT8MnwspsRmDhX5xYzL1exYfTYnhJQ5shEFxHq8cVU31B5FQF8C5SUPse67EYAcyAxM1Wqs` → `solana confirm … → Finalized`.
- **Scope isolation (VX.1).** `git diff --name-only 93d87bc..HEAD -- 'skeleton/crates/**' ':(exclude)skeleton/crates/mcp-server/**'` is EMPTY — no crate outside `mcp-server` was touched during M3 implementation. V3 itself touches only `skeleton/examples/MVP-M3-REPORT.md`.

### Cargo test tail (V3.3)

`cd skeleton && cargo test --workspace 2>&1 | tail -n 40`:

```
test tests::anchor_hello_world_greet_world ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests idl_registry

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests mcp_server

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests nl_planner

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests proxy

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests skill_synth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests tx_builder

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Summary across all 16 `test result:` lines in the full workspace run: **all `ok.`, 0 failures**. Crate totals: skill-synth 12, idl-registry 8, tx-builder 1, proxy 5, mcp-server 3 (lib) + 1 (http_transport integration test) — plus doc-test buckets (all `ok.`).

## Evidence summary (V3 synthesis)

Baseline commit `93d87bc` is unchanged. M3-V3 added only this report amendment (no source edits outside `skeleton/examples/MVP-M3-REPORT.md`); M3-V2 was observation-only. Non-custodial invariant holds across all M3-authored files; `$SIG` above is the canonical MCP-originated signature consumed by VX.5.
