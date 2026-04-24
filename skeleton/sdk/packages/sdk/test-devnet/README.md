# `test-devnet/` — end-to-end SPL-Token transfer harness

> **Note.** The directory name is `test-devnet/` for contract compatibility
> with the M5b feature spec, but the harness actually runs against
> **surfpool** (a local mainnet-beta fork), *not* public devnet.
>
> F12 was pivoted from devnet to surfpool because:
> 1. User's default keypair owner has 0 USDC on devnet.
> 2. Canonical M2–M4 fixture owners have 0 SOL on devnet.
> 3. The canonical Token-2022 mint `ATZ7Jx…` does not exist on devnet.
>
> Surfpool produces **real Solana-format signatures** (validated by
> `solana-cli` the same way ledger-format signatures are), but they are
> not publicly verifiable via `explorer.solana.com`.

## Prerequisites

- [`surfpool`](https://docs.surfpool.run/) CLI (`cargo install surfpool`)
- `solana-cli`, `spl-token-cli`, `jq`, `curl`
- A default keypair at `~/.config/solana/id.json` (used as Token-2022 mint
  authority and fee payer for fixture provisioning)
- Canonical fixture keypairs already present in `mission-fixtures/`:
  - `mint.json`, `source-owner.json`, `dest-owner.json`

## Run

```bash
pnpm --filter @agentgeyser/sdk test:devnet
```

The wrapper `run.sh` spawns surfpool + the proxy, seeds Token-2022 state,
executes `transfer.e2e.ts` (amount: 10000 base units = 0.01 USDC @ 6dp),
calls `solana confirm` on the resulting signature, and tears both
processes down on exit.

## Outputs

| path | description |
| --- | --- |
| `/tmp/m5b-devnet-sig.txt` | Raw base58 transaction signature (ephemeral). |
| `test-devnet/.surfpool-state.json` | Pubkey scratchpad written by `setup-surfpool.sh`. Git-ignored. |

## Environment variables

The e2e reads everything from env vars (VX.4: no base58 literals in TS).
`run.sh` assembles them from `.surfpool-state.json`; override on the
command line if you point the harness at an externally-managed surfpool.

- `AGENTGEYSER_PROXY_URL` — proxy RPC (default `http://127.0.0.1:8999`)
- `AGENTGEYSER_RPC_URL` — surfpool RPC (default `http://127.0.0.1:8899`)
- `AGENTGEYSER_DEVNET_MINT`, `..._SRC_ATA`, `..._DST_ATA`,
  `..._SRC_OWNER`, `..._DST_OWNER`, `..._AMOUNT`, `..._KEYPAIR`
