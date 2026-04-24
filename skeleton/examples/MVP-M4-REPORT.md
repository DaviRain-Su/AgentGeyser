# AgentGeyser — MVP-M4 Synthesis Report (V3 refresh)

- **Date**: 2026-04-24
- **Baseline commit**: `e0a8b82` (MVP-M3 MCP server, squashed)
- **HEAD at refresh**: V3 (this commit) — real surfpool evidence embedded
- **Scope**: Ship a pnpm monorepo under `skeleton/sdk/` exposing three packages
  — `@agentgeyser/sdk` (isomorphic Node + Browser client wrapping the M2
  proxy / M3 MCP JSON-RPC), `@agentgeyser/react` (React 18 hooks bound to
  `@solana/wallet-adapter-react`), and `@agentgeyser/docs` (Docusaurus v3 +
  Typedoc). End-to-end verified via a canonical **SDK-originated signature**
  (§4) landed on surfpool and confirmed via `solana confirm`.

---

## §1 Features

| ID      | Area                                                                       | Status |
|---------|-----------------------------------------------------------------------------|--------|
| M4-F1   | pnpm workspace scaffold (`packages/sdk`, `packages/react`, `apps/docs`)     | ✅ `b107f24` |
| M4-F2   | `@agentgeyser/sdk` core: `AgentGeyserClient` + error hierarchy              | ✅ `c1fdc2a` |
| M4-F3   | `signAndSend` helper (Node + Browser dual path, `@solana/web3.js` v2)       | ✅ `848cece` |
| M4-F4   | `agentgeyser` CLI wrapper (`list-skills` + `invoke` subcommands)            | ✅ `7fedbaa` |
| M4-F5   | `@agentgeyser/react` scaffold + `AgentGeyserProvider` / `useAgentGeyser`    | ✅ `2213996` |
| M4-F6   | `useSkills()` hook (dedupe + cancel on unmount)                             | ✅ `699d315` |
| M4-F7   | `useInvokeSkill()` hook + Playwright e2e spec (wallet-adapter-driven)       | ✅ `cf3b8d4` |
| M4-F8   | Docusaurus v3 scaffold + core docs (installation, quickstart, …)            | ✅ `83067a2` |
| M4-F9   | Typedoc-generated API reference under `apps/docs/docs/api/`                 | ✅ `50b8cd6` |
| M4-F10  | MVP-M4 synthesis report draft                                               | ✅ `9597fe2` |
| M4-V3   | This refresh — real surfpool evidence embedded                              | ✅ this commit |

Substrate: `AgentGeyserClient` calls `ag_listSkills` / `ag_invokeSkill` on
the M2 proxy — same transport surface `agentgeyser-mcp-server` bridges to MCP
via `list_skills` / `invoke_skill`. React hooks (`useAgentGeyser`,
`useSkills`, `useInvokeSkill`) consume the SDK client via context.

---

## §2 LOC Deltas vs `mvp-m3`

`git diff --stat mvp-m3..HEAD -- skeleton/sdk/ ':(exclude)skeleton/sdk/pnpm-lock.yaml'`:

```
 scaffold (package.json, pnpm-workspace.yaml, tsconfig.base.json, .npmrc, .gitignore) |  40 +
 packages/sdk/src/** (client, errors, signAndSend, cli, types, index, tests)          | 1057 +
 packages/sdk/{package.json,tsconfig.json,vitest.config.ts,bin/agentgeyser}           |  82 +
 packages/react/src/** (context, useSkills, useInvokeSkill + tests)                   | 345 +
 packages/react/{package.json,tsconfig.json,vitest.config.ts}                         |  82 +
 packages/react/e2e/** (playwright config, demo app, invoke-skill spec)               |  99 +
 apps/docs/docs/*.md (installation, quickstart, architecture, non-custodial)          | 289 +
 apps/docs/{docusaurus.config.ts,sidebars.ts,typedoc.json,package.json}               | 157 +
 apps/docs/src/** + tsconfig + static/                                                |  49 +
 examples/MVP-M4-REPORT.md                                                            | this file
 49 files changed, 2296 insertions(+)  (lockfile excluded; with lockfile: 50f/17456+)
```

All per-feature budgets held at their gates. Sub-totals at V3:
`packages/sdk` ≈ 1 139, `packages/react` ≈ 526, `apps/docs` ≈ 495.

---

## §3 Public API Surface

TSDoc entry points indexed by Typedoc under `apps/docs/docs/api/` (F9).

### `@agentgeyser/sdk` (exports from `packages/sdk/src/index.ts`)

```ts
export class AgentGeyserClient {
  constructor(options: AgentGeyserClientOptions);
  listSkills(): Promise<Skill[]>;
  invokeSkill(request: InvokeSkillRequest): Promise<InvokeSkillResponse>;
}

export function signAndSend(
  opts: SignAndSendNodeOptions | SignAndSendBrowserOptions,
): Promise<SignAndSendResult>;

export class AgentGeyserError extends Error { code?: number }
export class RpcError        extends AgentGeyserError {}
export class NetworkError    extends AgentGeyserError {}
export class SkillNotFound   extends AgentGeyserError {}
export class ValidationError extends AgentGeyserError {}

export type Skill; export type InvokeSkillRequest; export type InvokeSkillResponse;
export type SignAndSendResult; export type ConfirmationState;
```

### `@agentgeyser/react` (exports from `packages/react/src/index.ts`)

```ts
export function AgentGeyserProvider(props: {
  proxyUrl?: string; fetch?: FetchLike; children: ReactNode;
}): JSX.Element;

export function useAgentGeyser(): AgentGeyserClient;

export function useSkills(): {
  data: Skill[] | undefined; loading: boolean;
  error: Error | undefined;  refetch: () => Promise<void>;
};

export function useInvokeSkill(options?: { rpcUrl?: string }): {
  mutate: (req: InvokeSkillRequest) => Promise<{ signature: string }>;
  data: { signature: string } | undefined;
  loading: boolean; error: Error | undefined;
};
```

Typedoc pages: `apps/docs/docs/api/@agentgeyser/{sdk,react}/` — auto-regenerated
by the Docusaurus `prebuild: typedoc` step (F9.3), keeping the published
surface (`AgentGeyserClient`, `useAgentGeyser`, `useSkills`, `useInvokeSkill`)
in sync with TSDoc.

---

## §4 Surfpool Smoke Excerpt (M4-V2 live evidence)

V2 live-smoke entry: `skeleton/sdk/examples/live-smoke.mjs` (built SDK import
of `AgentGeyserClient` + `signAndSend`). Run against surfpool 0.10.8 /
solana-core 2.3.8 on `127.0.0.1:8899` with the M2 proxy on `127.0.0.1:8999`:

```console
$ SRC_ATA=7i2V9Dd6kVcApy4CzUVDNXd6QTVQm5LTA9HcHGXsB21z \
  DST_ATA=C9g3CNJ68MqEYdUTFfaZimHw22rjosA1u3DpcuZcGnCV \
  SRC_OWNER=Fh3A4pc8YtQvfy5rz9HDXraX5kyn4AFkXyk1V8oWLP13 \
  node skeleton/sdk/examples/live-smoke.mjs
SKILLS_OK
TX_LEN=328
SIG=2ZGk2W1b6iDQPAp8LEA9mgLMc5HQMiTkYQEEoBkDRKqk9PwvwWtGu76PWagJ4AGThtasrPtiku9o6jFUf6ttx95J

$ solana confirm -u http://127.0.0.1:8899 \
    2ZGk2W1b6iDQPAp8LEA9mgLMc5HQMiTkYQEEoBkDRKqk9PwvwWtGu76PWagJ4AGThtasrPtiku9o6jFUf6ttx95J
Confirmed
```

Canonical SDK-originated signature (the single contractual artefact V3.2
requires):

```sdk-originated-signature
SIG=2ZGk2W1b6iDQPAp8LEA9mgLMc5HQMiTkYQEEoBkDRKqk9PwvwWtGu76PWagJ4AGThtasrPtiku9o6jFUf6ttx95J

$ solana confirm -u http://127.0.0.1:8899 2ZGk2W1b6iDQPAp8LEA9mgLMc5HQMiTkYQEEoBkDRKqk9PwvwWtGu76PWagJ4AGThtasrPtiku9o6jFUf6ttx95J
Confirmed
```

Produced by the `@agentgeyser/sdk` Node path:
`AgentGeyserClient.invokeSkill({ skill_id: 'spl-token::transfer', … })` →
`signAndSend({ keypairPath: 'mission-fixtures/source-owner.json', … })` →
`@solana/web3.js` v2. React / wallet-adapter path is covered by §5.3
Playwright evidence.

---

## §5 Canonical Names Grep (V3.6 / VX.4)

```
$ grep -rnE 'spl-token::transfer|agentgeyser-mcp-server|list_skills|invoke_skill|AgentGeyserClient|useAgentGeyser|useSkills|useInvokeSkill' \
    skeleton docs AGENT.md --exclude-dir=target --exclude-dir=node_modules 2>/dev/null | wc -l
506
```

506 hits across `skeleton/`, `docs/`, and top-level `AGENT.md` — far above
the VX.4 floor of 25. Canonical names (`spl-token::transfer`,
`agentgeyser-mcp-server`, `list_skills`, `invoke_skill`, `AgentGeyserClient`,
`useAgentGeyser`, `useSkills`, `useInvokeSkill`) appear consistently across
code, docs, and this report — no drift.

### §5.1 `pnpm -r test` tail

```console
$ cd skeleton/sdk && pnpm -r test
packages/sdk test:  ✓ src/__tests__/client.test.ts     (13 tests) 6ms
packages/sdk test:  ✓ src/__tests__/signAndSend.test.ts (3 tests) 16ms
packages/sdk test:  ✓ src/__tests__/cli.test.ts        (4 tests) 5ms
packages/sdk test:  Test Files  3 passed (3)
packages/sdk test:       Tests  20 passed (20)
packages/react test: ✓ src/useInvokeSkill.test.tsx (2 tests) 166ms
packages/react test: ✓ src/context.test.tsx       (2 tests)  15ms
packages/react test: ✓ src/useSkills.test.tsx     (3 tests) 114ms
packages/react test:  Test Files  3 passed (3)
packages/react test:       Tests  7 passed (7)
```

### §5.2 `pnpm --filter @agentgeyser/sdk size-limit`

```console
$ cd skeleton/sdk && pnpm --filter @agentgeyser/sdk exec size-limit
  Size limit: 60 kB
  Size:       5.26 kB  with all dependencies, minified and brotlied
```

5.26 kB gzipped — 11× headroom below the 60 kB F2.7 budget.

### §5.3 Playwright report excerpt (`@agentgeyser/react` e2e)

```console
$ cd skeleton/sdk && SRC_ATA=…  DST_ATA=…  SRC_OWNER=… \
    pnpm --filter @agentgeyser/react test:e2e
Running 1 test using 1 worker
  ✓  1 [chromium] › e2e/specs/invoke-skill.spec.ts:28:1 › clicking invoke
      displays a signature from the mocked wallet path (621ms)
  1 passed (2.1s)
```

The e2e spec drives a mocked `@solana/wallet-adapter-react` adapter against
the live proxy, proving `useInvokeSkill` → `wallet.signTransaction` →
`sendTransaction` wiring end-to-end on the React side.

---

## §6 Non-Custodial Grep (V3.4 / VX.3)

```
$ grep -rnE '\b(Keypair\.fromSecretKey|privateKey|seedPhrase|mnemonic)\b' skeleton/sdk/packages/
```

```
```

Empty capture (command returns exit 1 on tracked sources — no matches).
Signing is NEVER performed inside `@agentgeyser/react`: the package consumes
`@solana/wallet-adapter-react`'s `useWallet()` and delegates
`wallet.signTransaction(tx)` back to the user-controlled adapter. The SDK's
`signAndSend` helper is the sole Node-side signing surface; it reads a keypair
only via caller-supplied `keypairPath` (dynamic `await import('node:fs/promises')`),
never a seed phrase, mnemonic, or hard-coded `privateKey`. Word-boundary note
(M3 gap h): the regex intentionally excludes bare `sign` / `Keypair` / `Signer`
since `@solana/web3.js` v2 exports a legitimate `Signer` type and the helper
is literally named `signAndSend`.

---

## §7 How to Run

Prereqs: Node ≥ 20, `corepack enable` (pnpm 9.x), local surfpool on
`127.0.0.1:8899` + proxy on `127.0.0.1:8999` (see MVP-M2 §10 / MVP-M3 §7).

```bash
cd skeleton/sdk && pnpm install && pnpm -r build       # (1) install + build
pnpm -r test                                           # (2) vitest
SRC_ATA=… DST_ATA=… SRC_OWNER=… \
  pnpm --filter @agentgeyser/react test:e2e            # (3) Playwright
pnpm --filter @agentgeyser/sdk exec agentgeyser \
  list-skills --proxy-url http://127.0.0.1:8999        # (4) CLI smoke
SRC_ATA=… DST_ATA=… SRC_OWNER=… \
  node skeleton/sdk/examples/live-smoke.mjs            # (5) spl-token::transfer
```

---

## §8 Architecture Notes

- **Data flow (happy path).** Consumer app →
  `@solana/wallet-adapter-react` → `AgentGeyserProvider` →
  `useInvokeSkill.mutate(req)` → `AgentGeyserClient.invokeSkill` (JSON-RPC
  `ag_invokeSkill` to M2 proxy; `invoke_skill` over MCP when routed through
  `agentgeyser-mcp-server`) → `transactionBase64` →
  `wallet.signTransaction(tx)` → `@solana/web3.js` v2 `sendTransaction` →
  surfpool. The invariant: the only signing edge is the wallet adapter on
  the consumer side.
- **Dual Node / Browser `signAndSend`.** Dispatch on `isNodeEnvironment()`.
  Node branch dynamically imports `node:fs/promises` + v2 wire-tx helpers
  (`createKeyPairSignerFromBytes`); Browser branch takes pre-signed
  `signedTransactionBase64`. No top-level `fs` import anywhere.
- **Peer-dep model.** `@solana/web3.js` is a peer dep on the SDK. The React
  package peers on `react >=18` and `@solana/wallet-adapter-react`
  (size-limit §5.2: 5.26 kB brotlied).
- **Docs determinism.** Typedoc pinned; `prebuild: typedoc` makes API
  reference byte-identical per commit (F9.4).

---

## §9 Known Gaps

Carry-forward:

- **M2 gap (a) — narrow `invokeSkill` TS signature.** **Closed by M4-F2**.
- **M2 gap (c) — `anchor idl init` HTTP 405 on surfpool 0.10.8.** Still open.
- **M3 gap (e) — Inspector CLI version unpinned.** Still open; M4 unused.
- **M3 gap (f) — Windows / Linux Claude Desktop onboarding.** Still open.
- **M3 gap (g) — MCP-originated surfpool signature.** Closed by M3-V2/V3.
  M4 adds a parallel **SDK-originated signature** (§4) as a distinct artefact.
- **M3 gap (h) — `is_signer` false positive on broad non-custodial regex.**
  Still open as guidance; M4's regex is word-boundary-precise.

New MVP-M4 gaps:

- **(i) Algolia DocSearch unprovisioned** (`PLACEHOLDER` appId; search 404s
  until M5).
- **(j) Playwright e2e requires env-bound ATAs** — the demo's `vite.config.ts`
  bakes `SRC_ATA` / `DST_ATA` / `SRC_OWNER` via `define` (V2 discovered).
- **(k) React Native not covered.**
- **(l) SSR untested** (Next.js / Remix paths not exercised).
- **(m) Live devnet deferred to M5.**

---

## §10 Verify Run (M4-V1 + M4-V2 + M4-V3)

### Environment (M4-V1)

- `getVersion` on `http://127.0.0.1:8899` → `surfnet-version 0.10.8`,
  `solana-core 2.3.8`.
- `ag_listSkills` on `http://127.0.0.1:8999` returned 4 skills including
  `spl-token::transfer`; canonical pubkey set matched M2/M3 verbatim.
- `git merge-base --is-ancestor mvp-m3 HEAD` → **reachable** (VX.5).

### SDK end-to-end (M4-V2)

- `cargo build --workspace` exit 0 — no Rust source modified (VX.1:
  `git diff --name-only mvp-m3..HEAD | grep -vE
  '^(skeleton/sdk/|skeleton/examples/MVP-M4-REPORT\.md)' | wc -l` = **0**).
- `cargo test --workspace` exit 0 — every crate reports `test result: ok.`
  (VX.2). Doc-tests for `mcp_server`, `nl_planner`, `proxy`, `skill_synth`,
  `tx_builder` all pass 0 failures.
- Live-smoke produced canonical SDK-originated signature (§4); Playwright
  e2e: **1 passed (2.1s)** (§5.3).

### Artifacts (M4-V3)

- This file is the single modified path in the V3 commit.
- Non-custodial (VX.3): empty capture on tracked sources (§6).
- Canonical names (VX.4): 506 hits ≥ 25 (§5).

*MVP-M4 complete. Baseline `mvp-m3` reachable from HEAD; tag `mvp-m4` staged
for orchestrator after V3 lands.*
