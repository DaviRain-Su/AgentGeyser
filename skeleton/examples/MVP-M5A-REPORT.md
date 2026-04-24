# AgentGeyser — MVP-M5a Synthesis Report (V3 refresh)

- **Date**: 2026-04-24
- **Baseline tag**: `mvp-m4` → commit `52025bc` (M4 squashed)
- **HEAD at refresh**: V3 (this commit) — real GitHub Actions evidence embedded
- **Scope**: Make the repo **release-ready from a CI + packaging standpoint**
  without pushing to npm or a real devnet. External artefact: a green
  `ci.yml` run and a green tag-triggered `release.yml` dry-run on
  GitHub Actions. No real publish, no OpenAI key, no funded devnet
  keypair — those are deferred to M5b / M5c.

---

## §1 Features

| ID       | Area                                                                     | Status |
|----------|--------------------------------------------------------------------------|--------|
| M5a-F1   | `ci.yml` caching layers (cargo registry, cargo target, pnpm store, Playwright) | ✅ |
| M5a-F2   | `pnpm -r test` + `pnpm -r build` + SDK size-limit job                    | ✅ |
| M5a-F3   | Playwright e2e in CI (headless-shell, MOCKED env → `1 skipped`)          | ✅ |
| M5a-F4   | Docusaurus build + typedoc determinism check (`diff -r` clean)           | ✅ |
| M5a-F5   | `cargo clippy --workspace --all-features -- -D warnings`                 | ✅ |
| M5a-F6   | `release.yml` skeleton (tag-triggered, `--dry-run`, artifacts, concurrency) | ✅ |
| M5a-F7   | OIDC `id-token: write` + `contents: read` permissions (M5c prep)         | ✅ |
| M5a-F8   | Tarball size validator (SDK < 200 KB, React < 300 KB unpacked)           | ✅ |
| M5a-F9   | Package metadata bump → `0.1.0-alpha.0` + `files` whitelist              | ✅ |
| M5a-F10  | Root `LICENSE` (MIT) + per-package `LICENSE` copies                      | ✅ |
| M5a-F11  | Per-package READMEs (≤ 80 LOC each, install + peer-dep note)             | ✅ |
| M5a-F12  | Root `CHANGELOG.md` (Keep-a-Changelog, `[0.1.0-alpha.0]` anchor)         | ✅ |
| M5a-F13  | Pin `pnpm/action-setup@v4` to `9.15.0` in both workflows                 | ✅ |
| M5a-F14  | Serialize `pnpm -r build` with `--workspace-concurrency=1`               | ✅ |
| M5a-F15  | Explicit per-package build sequence `sdk → react → docs`                 | ✅ |
| M5a-V1   | Local preflight (YAML parse, `pnpm pack --dry-run`, `cargo test` green)  | ✅ |
| M5a-V2   | Live GitHub Actions verify (real URLs + timings captured)                | ✅ |
| M5a-V3   | This refresh — real URLs + timings + tarball excerpts embedded           | ✅ this commit |

---

## §2 Real GitHub Actions Evidence (V2 live verify)

Both workflows triggered by the annotated RC tag `v0.1.0-alpha.0-rc1`
pointing at commit `c116888` (F15 build-order fix). Proxy env
(`HTTPS_PROXY=http://127.0.0.1:3080`) was exported so `gh` CLI could
reach `api.github.com` through the local TLS-intercepting proxy
(AGENTS.md §8.1).

### §2.1 ci.yml — green

Canonical cold run (main-branch trigger, F15 commit):

- **URL:** https://github.com/DaviRain-Su/AgentGeyser/actions/runs/24884037072
- **Run ID:** 24884037072
- **Conclusion:** success
- **Duration:** `7m18s` (`updatedAt - createdAt` = 438s)

Parallel tag-branch run (same commit, second trigger):

- **URL:** https://github.com/DaviRain-Su/AgentGeyser/actions/runs/24884042606
- **Run ID:** 24884042606
- **Conclusion:** success
- **Duration:** `6m23s`

All verify job steps green: cargo check / clippy / test, pnpm install /
build / test, SDK size-limit (5.26 kB / 60 kB budget), Playwright
install + `@agentgeyser/react` e2e (readiness-guard `1 skipped` per
F3.2), docusaurus build (`onBrokenLinks=throw`), typedoc determinism
check (`diff -r` exit 0).

### §2.2 release.yml — green (dry-run mode)

- **URL:** https://github.com/DaviRain-Su/AgentGeyser/actions/runs/24884042590
- **Run ID:** 24884042590
- **Event:** push / tag `v0.1.0-alpha.0-rc1`
- **Conclusion:** success
- **Duration:** `1m9s`

All 14 job steps green: setup → install → build → SDK publish
`--dry-run` → React publish `--dry-run` → SDK size validate → React
size validate → pack SDK → pack React → upload SDK tarball artifact →
upload React tarball artifact. No NPM_TOKEN referenced (F7.2); OIDC
permission block declared (F7.1).

### §2.3 Cold-run duration

```
Cold-run CI duration: 7m18s
```

Well under the F1.5 / V2.6 budget of 15 min. Warm-run is **deferred to
M5b** per V2.7: only one CI push-sequence at F15 exists and both parallel
runs started from cold caches (prior F13/F14 runs failed at
`Build pnpm workspace` *before* the cache-save post-steps executed, so no
warm cache was ever persisted). The first warm run opportunity is the
next push to `main` in M5b, or `gh run rerun 24884037072`.

---

## §3 Tarball Metrics (V3.5)

Excerpts from the `pnpm pack --dry-run --json | jq` surface defined in
F8 / V3.5, captured via `npm notice` output of the release.yml run
`24884042590`. Threshold assertions (SDK < 200 KB, React < 300 KB) hold.

### §3.1 `@agentgeyser/sdk`

```json
{
  "name": "@agentgeyser/sdk",
  "version": "0.1.0-alpha.0",
  "unpackedSize": 25300,
  "files": 9
}
```

Tarball: `agentgeyser-sdk-0.1.0-alpha.0.tgz` — package size 8.3 kB,
unpacked 25.3 kB (8× headroom under the 200 KB budget). Contents:
`LICENSE`, `README.md`, `bin/agentgeyser`, `dist/chunk-*.js`,
`dist/cli.d.ts`, `dist/cli.js`, `dist/index.d.ts`, `dist/index.js`,
`package.json`.

### §3.2 `@agentgeyser/react`

```json
{
  "name": "@agentgeyser/react",
  "version": "0.1.0-alpha.0",
  "unpackedSize": 119100,
  "files": 5
}
```

Tarball: `agentgeyser-react-0.1.0-alpha.0.tgz` — package size 28.6 kB,
unpacked 119.1 kB (2.5× headroom under the 300 KB budget). Contents:
`LICENSE`, `README.md`, `dist/index.d.ts`, `dist/index.js`,
`package.json`. No `src/`, `e2e/`, or `vitest.config.*` leak (F9.7 /
F9.8).

### §3.3 Artifacts uploaded (V2.5)

From `gh api repos/DaviRain-Su/AgentGeyser/actions/runs/24884042590/artifacts`:

| Name | Size (bytes) | Expired |
|---|---|---|
| `agentgeyser-sdk-tarball`   | 8517  | false |
| `agentgeyser-react-tarball` | 28753 | false |

Both artifacts present, retention 14 days (F6.5).

---

## §4 Dry-Run Publish Excerpt (V2.4)

From `gh run view 24884042590 --log`:

```
SDK   publish dry-run  npm notice 📦  @agentgeyser/sdk@0.1.0-alpha.0
SDK   publish dry-run  npm notice filename: agentgeyser-sdk-0.1.0-alpha.0.tgz
SDK   publish dry-run  npm notice package size: 8.3 kB
SDK   publish dry-run  npm notice unpacked size: 25.3 kB
SDK   publish dry-run  npm notice total files: 9
SDK   publish dry-run  + @agentgeyser/sdk@0.1.0-alpha.0

React publish dry-run  npm notice 📦  @agentgeyser/react@0.1.0-alpha.0
React publish dry-run  npm notice filename: agentgeyser-react-0.1.0-alpha.0.tgz
React publish dry-run  npm notice package size: 28.6 kB
React publish dry-run  npm notice unpacked size: 119.1 kB
React publish dry-run  npm notice total files: 5
React publish dry-run  + @agentgeyser/react@0.1.0-alpha.0
```

Both packages advertise `0.1.0-alpha.0` — matches the version bump in
F9 and the CHANGELOG anchor in F12.

---

## §5 VX Cross-Cutting Invariants

### §5.1 VX.1 — Scope isolation

`git diff --name-only mvp-m4..HEAD` returned a subset of the M5a
allow-list (workflows, package.json / README.md / LICENSE under
`skeleton/sdk/packages/{sdk,react}`, root LICENSE, root CHANGELOG.md,
`skeleton/sdk/apps/docs/docusaurus.config.ts` under F4's exception, this
report, plus four `skeleton/crates/**` files that fall under the F5
clippy exception documented in AGENTS.md §2):

```
.github/workflows/ci.yml
.github/workflows/release.yml
CHANGELOG.md
LICENSE
skeleton/crates/idl-registry/src/anchor_idl.rs   (F5 clippy)
skeleton/crates/idl-registry/src/lib.rs          (F5 clippy)
skeleton/crates/mcp-server/src/lib.rs            (F5 clippy)
skeleton/crates/proxy/src/lib.rs                 (F5 clippy)
skeleton/sdk/apps/docs/docusaurus.config.ts     (F4 onBrokenLinks flip)
skeleton/sdk/packages/react/LICENSE
skeleton/sdk/packages/react/README.md
skeleton/sdk/packages/react/package.json
skeleton/sdk/packages/sdk/LICENSE
skeleton/sdk/packages/sdk/README.md
skeleton/sdk/packages/sdk/package.json
```

Zero scope violations. No `src/`, `e2e/`, or `docs/` page edits.

### §5.2 VX.2 — Non-custodial

```
$ grep -rnE '\b(Keypair\.fromSecretKey|privateKey|seedPhrase|mnemonic)\b' \
    skeleton/sdk/packages/ --exclude-dir=node_modules --exclude-dir=dist
(exit 1 — no matches)
```

The `@agentgeyser/react` hooks still delegate signing to
`@solana/wallet-adapter-react`; `@agentgeyser/sdk`'s `signAndSend` Node
path reads a keypair only via caller-supplied path. No secret material
is ever embedded.

### §5.3 VX.3 — Rust invariant

`cd skeleton && cargo test --workspace` exits 0; the four crate edits
listed in VX.1 are the F5 clippy-driven fixes (documented exception in
AGENTS.md §2 and in the F5 commit). No functional Rust changes.

### §5.4 VX.4 — Droid-Shield safety

Pre-commit grep on every file changed in M5a:

```
$ grep -rnE '[1-9A-HJ-NP-Za-km-z]{32,44}' -- \
    $(git diff --name-only mvp-m4..HEAD)
(zero matches across workflows, READMEs, LICENSE, CHANGELOG, docusaurus
config, and this report)
```

SHAs embedded above (`c116888`, `52025bc`, `mvp-m4`) are hex / short
git IDs — not base58. URLs contain only decimal `run_id` integers. This
report uses angle-bracket placeholders where a pubkey would otherwise
appear (see §4 — only package names, versions, and file sizes).

### §5.5 VX.5 — mvp-m4 ancestry

```
$ git merge-base --is-ancestor mvp-m4 HEAD && echo ancestor-ok
ancestor-ok
```

`mvp-m4` (`52025bc`) is reachable from HEAD. Tag discipline preserved.

### §5.6 VX.6 — Canonical-names grep

```
$ grep -rnE 'spl-token::transfer|agentgeyser-mcp-server|list_skills|\
invoke_skill|AgentGeyserClient|useAgentGeyser|useSkills|useInvokeSkill' \
    skeleton docs AGENT.md --exclude-dir=target --exclude-dir=node_modules | wc -l
549
```

549 hits — far above the VX.6 floor of 25. Canonical names remain
consistent across SDK, React hooks, MCP server, docs, and this report.

---

## §6 Known Gaps (carry into M5b / M5c)

- **(n)** Warm-run CI timing — V2.7 deferred to first post-F15 push on `main` in M5b.
- **(o)** Real `npm publish` — still `--dry-run`; flipped via OIDC trusted publishing in M5c.
- **(p)** Algolia DocSearch — still `PLACEHOLDER` appId (carry from M4 gap (i)); M5b or later.
- **(q)** Live devnet Track A — `spl-token::transfer` on `api.devnet.solana.com` is M5b.
- **(r)** Docs GitHub Pages deploy — deferred to M5c. **(s)** Yellowstone gRPC — M6.

---

## §7 How to Reproduce Locally (V1 surface)

```bash
# (1) YAML parse
python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/ci.yml")); \
  yaml.safe_load(open(".github/workflows/release.yml"))'

# (2) pnpm workspace install + build + test
cd skeleton/sdk && pnpm install --frozen-lockfile=false && \
  pnpm --filter @agentgeyser/sdk build && \
  pnpm --filter @agentgeyser/react build && \
  pnpm --filter @agentgeyser/docs build && \
  pnpm -r test

# (3) Tarball dry-run + size validator (F8)
cd skeleton/sdk/packages/sdk   && npm pack --dry-run --json | \
  jq '{name, version, unpackedSize, files: (.files|length)}'
cd skeleton/sdk/packages/react && npm pack --dry-run --json | \
  jq '{name, version, unpackedSize, files: (.files|length)}'

# (4) Rust workspace
cd skeleton && cargo test --workspace
cd skeleton && cargo clippy --workspace --all-features -- -D warnings
```

For the live GitHub Actions surface, the user pushes
`v0.1.0-alpha.0-rc1` after the last M5a feature lands; `ci.yml` and
`release.yml` fire automatically. V2's `gh run list` / `gh run view`
commands in `library/m5a-v2-handoff.md` replay the run queries.

---

## §8 Milestone Gate

M5a is complete when:

1. All 15 F-features + V1 + V2 + V3 are `completed` in features.json.
2. Real `ci.yml` URL (§2.1) is embedded in this report and is green.
3. Real `release.yml` URL (§2.2) is embedded, green, tag-triggered,
   `--dry-run` mode.
4. VX.1 (scope), VX.2 (non-custodial), VX.3 (Rust),
   VX.4 (Droid-Shield), VX.5 (ancestry), VX.6 (canonical names ≥ 25)
   all pass — see §5 above.
5. User squashes M5a commits onto `mvp-m4`, tags `mvp-m5a`, pushes.

Handoff to M5b carries forward: cached CI, tag-triggered release
skeleton in dry-run, publish-ready package metadata. M5b can focus
entirely on nl-planner (provider-agnostic LLM trait + OpenAI impl) and
live-devnet Track A without touching CI scaffolding.

*MVP-M5a complete. Baseline `mvp-m4` reachable from HEAD; tag `mvp-m5a`
staged for the orchestrator / user after V3 lands.*
