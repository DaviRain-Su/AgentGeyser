# @agentgeyser/react

React hooks for the AgentGeyser SDK, bound to `@solana/wallet-adapter-react`.
Non-custodial by design: the user's wallet signs; hooks never touch secrets.
Tiny surface: `useAgentGeyser`, `useSkills`, `useInvokeSkill`.

## Install

```bash
npm install @agentgeyser/react@alpha
```

Peer dependencies: `react` `>=18`, `@solana/wallet-adapter-react` `^0.15`,
and `@solana/web3.js` `^2.0.0` (the latter is re-exposed by the SDK).

## Quickstart

```tsx
import {
  AgentGeyserProvider,
  useSkills,
  useInvokeSkill,
} from "@agentgeyser/react";

function Panel({ payer }: { payer: string }) {
  const { data: skills } = useSkills();
  const invoke = useInvokeSkill();
  const firstSkill = skills?.[0];

  return (
    <button
      disabled={!firstSkill || invoke.loading}
      onClick={() =>
        firstSkill &&
        invoke.mutate({
          skill_id: firstSkill.skillId,
          args: {},
          accounts: {},
          payer,
        })
      }
    >
      {invoke.loading ? "Running…" : "Run first skill"}
    </button>
  );
}

export function App() {
  return (
    <AgentGeyserProvider proxyUrl="http://127.0.0.1:8999">
      <Panel payer="<PAYER_PUBKEY>" />
    </AgentGeyserProvider>
  );
}
```

## Links

- Quickstart guide: https://github.com/DaviRain-Su/AgentGeyser/blob/main/skeleton/sdk/apps/docs/docs/quickstart.md
- GitHub repo: https://github.com/DaviRain-Su/AgentGeyser
- Issues: https://github.com/DaviRain-Su/AgentGeyser/issues

## License

MIT. See `LICENSE`.
