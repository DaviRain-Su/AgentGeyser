---
id: quickstart
title: Quickstart
slug: /quickstart
---

# Quickstart

This page walks two end-to-end flows: signing from Node with a local
keypair, and signing from the browser via a wallet adapter. Both talk to
the same AgentGeyser proxy and never reveal private-key material to the
SDK surface outside of the Node-only `signAndSend` helper.

:::tip Default ports
- AgentGeyser proxy: `http://127.0.0.1:8999`
- Local Solana RPC (surfpool): `http://127.0.0.1:8899`

Set `AGENTGEYSER_PROXY_PORT` to run the proxy on a different port.
:::

## Node

```ts
import {
  AgentGeyserClient,
  signAndSend,
  type Connection,
  type Signer,
} from '@agentgeyser/sdk';

const client = new AgentGeyserClient({
  proxyUrl: 'http://127.0.0.1:8999',
});

const skills = await client.listSkills();
console.log(skills.map((s) => s.skillId));

const { transactionBase64 } = await client.invokeSkill({
  skill_id: 'spl-token::transfer',
  args: {
    source_ata: '<SOURCE_ATA>',
    destination_ata: '<DESTINATION_ATA>',
    owner: '<OWNER_PUBKEY>',
    amount: 1,
    mint: '<MINT_PUBKEY>',
    decimals: 6,
  },
  accounts: {},
  payer: '<PAYER_PUBKEY>',
});

declare const signer: Signer;
declare const connection: Connection;

const { signature } = await signAndSend({ unsignedTx: { tx: transactionBase64 }, signer, connection });

console.log('SIG=' + signature);
```

`signAndSend` accepts an unsigned transaction payload plus caller-provided
`signer` and `connection` adapters. The SDK never reads keypair files or takes
custody of private-key material.

## Browser

In the browser, signing is delegated to `@solana/wallet-adapter-react`.
Wrap your app with `AgentGeyserProvider` alongside the standard
`WalletProvider`:

```tsx
import { AgentGeyserProvider, useInvokeSkill } from '@agentgeyser/react';

function InvokeButton() {
  const { mutate, data, loading, error } = useInvokeSkill();

  return (
    <button
      disabled={loading}
      onClick={() =>
        mutate({
          skill_id: 'spl-token::transfer',
          args: {
            source_ata: '<SOURCE_ATA>',
            destination_ata: '<DESTINATION_ATA>',
            owner: '<OWNER_PUBKEY>',
            amount: 1,
            mint: '<MINT_PUBKEY>',
            decimals: 6,
          },
          accounts: {},
          payer: '<PAYER_PUBKEY>',
        })
      }
    >
      {loading ? 'Signing…' : 'Invoke'}
      {data?.signature && <pre>SIG={data.signature}</pre>}
      {error && <pre style={{ color: 'red' }}>{String(error)}</pre>}
    </button>
  );
}
```

Under the hood `useInvokeSkill` asks the connected wallet to
`signTransaction`, then forwards the signed bytes to the proxy's RPC for
submission. Your app code never touches a secret key.

See [Non-custodial](./non-custodial.md) for the full signing-boundary
story, and [Architecture](./architecture.md) for a diagram of the data
flow.
