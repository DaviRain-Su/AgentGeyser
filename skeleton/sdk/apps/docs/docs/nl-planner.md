---
id: nl-planner
title: Natural-Language Planner
slug: /nl-planner
sidebar_position: 5
---

# Natural-Language Planner

## Overview

The natural-language planner (`nl-planner`) turns free-form prompts into
structured, deterministic Solana action plans. A `Plan` is the shape
`{ skillId, args, rationale }` — the same skill surface exposed over
JSON-RPC — so any planned action can be executed by an existing skill
without extra wiring. Use the planner when you want the agent to pick
the right skill and arguments from a user-level sentence like
*"transfer 0.01 USDC to alice"*; reach for the direct skill API when you
already know which skill to call.

## OPENAI_API_KEY setup

The planner runs **proxy-side**; no LLM keys ever live inside the SDK or
a browser. Export provider keys in the proxy's environment:

```bash
export OPENAI_API_KEY=sk-...
export KIMI_API_KEY=...
export ANTHROPIC_API_KEY=...
```

The `sk-...` above is a documentation placeholder, not a real key.

With the default `auto` provider, the proxy selects the first available
backend in priority order: **ANTHROPIC → KIMI → OPENAI → Mock**. If no
keys are set, the deterministic `mock` provider is used so tests keep
passing offline.

## SDK usage

```ts
import { AgentGeyserClient } from '@agentgeyser/sdk';

const client = new AgentGeyserClient({ url: 'http://localhost:8787' });

const { skillId, args, rationale } = await client.planAction({
  prompt: 'transfer 0.01 USDC to alice',
});

console.log(skillId, args, rationale);
```

The client speaks the `ag_planAction` JSON-RPC method and returns the
camelCase `Plan` shape. No secrets are read or sent by the SDK.

## Providers (OpenAI vs Mock)

Pass `provider` explicitly on the proxy, or let `auto` decide. All five
provider strings are accepted:

- **`openai`** — OpenAI Chat Completions, requires `OPENAI_API_KEY`.
- **`mock`** — deterministic stub, zero network, used for tests.
- **`kimi-coding`** — Moonshot Kimi coding endpoint, requires
  `KIMI_API_KEY`.
- **`anthropic`** — Anthropic Messages API (`tool_use` mode), requires
  `ANTHROPIC_API_KEY`.
- **`auto`** — default; picks the first live provider in the priority
  order above, falling back to `mock`.

AgentGeyser is non-custodial: only the proxy holds provider credentials,
and user wallets remain fully client-side.
