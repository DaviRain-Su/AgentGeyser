---
id: non-custodial
title: Non-custodial by design
slug: /non-custodial
---

# Non-custodial by design

AgentGeyser is **non-custodial**. No package in this repository ever
constructs, imports, or stores a Solana secret key on behalf of the user.
Signing is always delegated to a surface the user already trusts.

## The signing boundary

- **Browser apps:** signing happens inside the wallet adapter. The
  `useInvokeSkill` hook from `@agentgeyser/react` calls
  `wallet.signTransaction(...)` on the `useWallet()` value returned by
  `@solana/wallet-adapter-react`. The hook never sees the secret key; it
  only receives the already-signed transaction bytes.
- **Node / CLI:** signing happens in `signAndSend` from
  `@agentgeyser/sdk`, and only when the caller explicitly passes a
  `keypairPath`. The import of `node:fs/promises` is dynamic, guarded by
  an `isNodeEnvironment()` check, and is therefore tree-shaken out of
  browser bundles.

Everywhere else, transaction bytes are treated as opaque base64. That is
why `useInvokeSkill` + `@solana/wallet-adapter-react` are the single
signing boundary for any React app using this SDK.

## Enforced invariant

A CI-grade grep runs across `skeleton/sdk/packages/` on every feature:

```bash
grep -rnE '\b(Keypair\.fromSecretKey|privateKey|seedPhrase|mnemonic)\b' \
  skeleton/sdk/packages/ || echo 'no matches (non-custodial OK)'
```

If that grep ever produces a hit, the offending change is rejected. The
word-boundary regex is precise on purpose: the function literally named
`signAndSend`, the `Signer` / `TransactionSigner` types from
`@solana/web3.js` v2, and prose mentions of "sign" in docs are all
legitimate and expected.

## What this rules out

- No environment variables named `KEYPAIR`, `PRIVATE_KEY`, or
  `MNEMONIC` anywhere in the SDK or React sources.
- No `Keypair.fromSecretKey(...)` calls. The only key-material code path
  is `createKeyPairSignerFromBytes` inside `signAndSend` on the Node path,
  operating on bytes the caller just handed us.
- No custom wallet implementations in `@agentgeyser/react`. Consumers
  always pick a real wallet adapter (Phantom, Backpack, Ledger, …) and we
  use whatever signer it exposes.

## What this still allows

- Server-side batch jobs that have legitimate custody of a keypair can
  still use `signAndSend({ keypairPath })`. The keypair file path is
  always caller-supplied, never inferred from an env variable or a config
  file inside the SDK.
- Hardware wallets work out of the box: the wallet adapter returns
  pre-signed bytes and the hook forwards them to the proxy's RPC.

If you need to write a custom signer, wire it into the wallet adapter
layer — not into `@agentgeyser/react`.
