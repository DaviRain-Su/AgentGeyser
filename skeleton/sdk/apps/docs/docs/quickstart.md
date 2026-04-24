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

## Node

```ts
import {
  AgentGeyserClient,
  signAndSend,
} from '@agentgeyser/sdk';

const client = new AgentGeyserClient({
  proxyUrl: 'http://127.0.0.1:8999',
});

const skills = await client.listSkills();
console.log(skills.map((s) => s.id));

const { transactionBase64 } = await client.invokeSkill({
  skill_id: 'spl-token::transfer',
  args: { amount: 1 },
  accounts: {
    source: '<SOURCE_ATA>',
    destination: '<DEST_ATA>',
    authority: '<AUTHORITY>',
  },
  payer: '<PAYER_PUBKEY>',
});

const { signature } = await signAndSend({
  client,
  transactionBase64,
  rpcUrl: 'http://127.0.0.1:8899',
  keypairPath: './payer.json',
});

console.log('SIG=' + signature);
```

`signAndSend` is the **only** surface in the SDK that reads a keypair from
disk, and it only does so when you hand it a `keypairPath`. The import of
`node:fs` is dynamic and never evaluated in a browser bundle.

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
          args: { amount: 1 },
          accounts: {
            source: '<SOURCE_ATA>',
            destination: '<DEST_ATA>',
            authority: '<AUTHORITY>',
          },
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
