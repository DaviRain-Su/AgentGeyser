# @agentgeyser/sdk

Isomorphic TypeScript client for the AgentGeyser proxy over JSON-RPC 2.0.
Browser + Node friendly, non-custodial, and type-safe end-to-end with Zod.
Pairs with `@solana/web3.js` v2 for transaction signing in your wallet.

## Install

```bash
npm install @agentgeyser/sdk@alpha
```

Peer dependency: `@solana/web3.js` `^2.0.0` (bring your own).

## Quickstart

```ts
import { AgentGeyserClient } from "@agentgeyser/sdk";

const client = new AgentGeyserClient({
  endpoint: "http://localhost:9000/rpc",
});

const skills = await client.listSkills();
const built = await client.invokeSkill({
  skill: skills[0].name,
  args: { amount: 1 },
});
// Sign `built.tx` with your wallet, then submit via web3.js.
```

## Links

- Quickstart guide: https://github.com/DaviRain-Su/AgentGeyser/blob/main/skeleton/sdk/apps/docs/docs/quickstart.md
- GitHub repo: https://github.com/DaviRain-Su/AgentGeyser
- Issues: https://github.com/DaviRain-Su/AgentGeyser/issues

## License

MIT. See `LICENSE`.
