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
import { useSkills, useInvokeSkill } from "@agentgeyser/react";

export function Panel() {
  const { data: skills } = useSkills();
  const invoke = useInvokeSkill();
  return (
    <button
      onClick={() => invoke.mutate({ skill: skills?.[0]?.name, args: {} })}
    >
      Run first skill
    </button>
  );
}
```

## Links

- Quickstart guide: https://github.com/DaviRain-Su/AgentGeyser/blob/main/skeleton/sdk/apps/docs/docs/quickstart.md
- GitHub repo: https://github.com/DaviRain-Su/AgentGeyser
- Issues: https://github.com/DaviRain-Su/AgentGeyser/issues

## License

MIT. See `LICENSE`.
