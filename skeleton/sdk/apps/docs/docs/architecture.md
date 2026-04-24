---
id: architecture
title: Architecture
slug: /architecture
---

# Architecture

AgentGeyser is a thin JSON-RPC proxy that sits in front of a Solana RPC
endpoint and exposes *skills* — deterministic, named transaction builders —
as first-class `ag_invokeSkill` / `ag_listSkills` methods. The SDK wraps
those methods with types; the React package wraps the SDK with hooks.

## Data flow (happy path)

```text
 ┌──────────────┐     signTransaction      ┌────────────────────┐
 │   Wallet     │ ◀─────────────────────── │    React hook      │
 │  (adapter)   │                          │  useInvokeSkill    │
 └──────┬───────┘                          └────────┬───────────┘
        │ signed tx bytes                           │ build tx request
        ▼                                           ▼
 ┌──────────────┐  ag_invokeSkill (JSON-RPC)  ┌────────────────────┐
 │  AgentGeyser │ ◀─────────────────────────  │ @agentgeyser/sdk    │
 │    proxy     │                             │ AgentGeyserClient   │
 └──────┬───────┘                             └────────┬───────────┘
        │ sendTransaction                              │ signAndSend (Node)
        ▼                                              ▼
 ┌──────────────────────────────────────────────────────────────────┐
 │                 Solana RPC  (surfpool / devnet / mainnet)         │
 └──────────────────────────────────────────────────────────────────┘
```

Read the diagram top-to-bottom: the wallet signs, the React hook asks the
SDK to build the transaction via the proxy, the SDK (or the wallet)
submits the signed bytes, and the RPC confirms.

## Component responsibilities

| Component | Responsibility |
|---|---|
| `@solana/wallet-adapter-react` | Owns private keys; exposes `signTransaction`. |
| `@agentgeyser/react` | Bridges React lifecycle to SDK calls; never holds secrets. |
| `@agentgeyser/sdk` | Types, transport, `signAndSend` helper, CLI. |
| AgentGeyser proxy | Registers skills, builds unsigned `Transaction`s, relays RPC. |
| Solana RPC | Consensus + state. |

## Skill anatomy

A skill is registered on the proxy (Rust side; see the
`agentgeyser-crates` documentation) with a stable id such as
`spl-token::transfer`. When you call `ag_invokeSkill`, the proxy returns
a base64-encoded unsigned `Transaction` plus metadata — the client is
free to sign it with any signer, including a hardware wallet.

## Why a proxy?

Without the proxy, every client would need to reimplement transaction
builders for each on-chain program. The proxy keeps those builders in one
audited place, behind a JSON-RPC surface that is trivial to type-check
from TypeScript. The SDK is therefore small (≤ 60 KB gzipped) and the
attack surface for key material stays narrow — the proxy never sees a
secret key.

Continue to [Non-custodial](./non-custodial.md) for the signing-boundary
invariant and why it matters.
