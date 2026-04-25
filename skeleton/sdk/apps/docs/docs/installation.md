---
id: installation
title: Installation
slug: /installation
---

# Installation

AgentGeyser ships as two packages: `@agentgeyser/sdk` (isomorphic Node +
Browser client) and `@agentgeyser/react` (React 18 hooks bound to
`@solana/wallet-adapter-react`).

:::tip Default ports
AgentGeyser uses proxy port `8999` and local Solana RPC port `8899` by
default. Override the proxy port with `AGENTGEYSER_PROXY_PORT`; keep
`AGENTGEYSER_RPC_URL` pointed at your Solana RPC.
:::

## Requirements

- Node.js **≥ 20**
- pnpm, npm, or yarn
- A running AgentGeyser proxy (default `http://127.0.0.1:8999`) bridged to
  a Solana RPC — locally that means [`surfpool`][surfpool] on `127.0.0.1:8899`.

[surfpool]: https://github.com/txtx/surfpool

## SDK (Node or Browser)

```bash
pnpm add @agentgeyser/sdk @solana/web3.js
```

`@solana/web3.js` is a **peer dependency** — you control the exact version.
The SDK is tree-shakeable ESM and stays under a 60 KB gzipped bundle budget.

## React hooks

```bash
pnpm add @agentgeyser/react @agentgeyser/sdk \
  react @solana/wallet-adapter-react @solana/web3.js
```

The React package exposes `AgentGeyserProvider`, `useAgentGeyser`,
`useSkills`, and `useInvokeSkill`. It never constructs a Keypair — signing
is delegated to whatever wallet adapter the consumer wires up at the app
root.

## CLI (optional)

Installing `@agentgeyser/sdk` puts an `agentgeyser` binary on your PATH:

```bash
pnpm exec agentgeyser --help
pnpm exec agentgeyser list-skills
```

## Verifying your setup

```bash
curl -sS -X POST -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"ag_listSkills","params":[]}' \
  http://127.0.0.1:8999 | jq '.result | length'
```

A numeric count means the proxy is reachable; `0` is fine for a fresh
install. Continue to the [Quickstart](./quickstart.md).
