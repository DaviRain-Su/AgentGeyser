# @agentgeyser/sdk

Isomorphic TypeScript client for the AgentGeyser proxy over JSON-RPC 2.0.
Browser + Node friendly, non-custodial, with TypeScript types derived from the proxy wire format.
Pairs with `@solana/web3.js` v2 for transaction signing in your wallet.

## Install

```bash
npm install @agentgeyser/sdk@alpha
```

Peer dependency: `@solana/web3.js` `^2.0.0` (bring your own).

## Quickstart

```ts
import {
  AgentGeyserClient,
  signAndSend,
  type Connection,
  type Signer,
} from "@agentgeyser/sdk";

const client = new AgentGeyserClient({
  proxyUrl: "http://127.0.0.1:8999",
});

const skills = await client.listSkills();
const firstSkill = skills[0];

const { transactionBase64 } = await client.invokeSkill({
  skill_id: firstSkill.skillId,
  args: {
    source_ata: "<SOURCE_ATA>",
    destination_ata: "<DESTINATION_ATA>",
    owner: "<OWNER_PUBKEY>",
    amount: 1,
    mint: "<MINT_PUBKEY>",
    decimals: 6,
  },
  accounts: {},
  payer: "<PAYER_PUBKEY>",
});

declare const signer: Signer;
declare const connection: Connection;

const { signature } = await signAndSend({
  unsignedTx: { tx: transactionBase64 },
  signer,
  connection,
});

console.log(signature);
```

`listSkills()` returns camelCase descriptors such as `skillId`, while
`invokeSkill()` sends the proxy wire field `skill_id` and normalizes the proxy
response to `transactionBase64`.

## Links

- Quickstart guide: https://github.com/DaviRain-Su/AgentGeyser/blob/main/skeleton/sdk/apps/docs/docs/quickstart.md
- GitHub repo: https://github.com/DaviRain-Su/AgentGeyser
- Issues: https://github.com/DaviRain-Su/AgentGeyser/issues

## License

MIT. See `LICENSE`.
