# anchor-hello-world

Minimal Anchor program used by AgentGeyser MVP-M2 Track A to demonstrate
end-to-end Anchor instruction synthesis (IDL → `Skill` → real unsigned
Solana transaction bytes).

This directory contains **source only** — no keypairs, no deployed
artifacts, no compiled `.so` files. Deployment is performed **manually**
by an operator with a funded devnet keypair (see `deploy.sh`).

## Program

The program exposes a single instruction:

```rust
pub fn greet(ctx: Context<Greet>, name: String) -> Result<()>
```

which emits `msg!("hello, {}", name)` and returns `Ok(())`. The `Greet`
accounts struct takes a single mutable `Signer` named `user`.

## Prerequisites

- `anchor --version` ≥ 0.30
- `solana --version` (any recent stable; CLI must be configured for devnet)
- A funded devnet keypair at `~/.config/solana/id.json`
  (run `solana airdrop 2` if needed; devnet faucet allows up to 5 SOL/day)

The mission environment that scaffolds this directory does **not** have a
funded keypair, which is why `PROGRAM_ID.txt` initially contains the
literal string `<PENDING_DEPLOY>`.

## Deploy (one-shot)

```bash
cd skeleton/examples/anchor-hello-world
./deploy.sh
```

`deploy.sh` runs, in order:

1. `anchor build` — compiles the BPF program.
2. `solana-keygen pubkey target/deploy/hello_world-keypair.json` — reads
   the freshly-generated program keypair.
3. `anchor deploy --provider.cluster devnet` — deploys the program.
4. `anchor idl init <PROGRAM_ID> --filepath target/idl/hello_world.json
   --provider.cluster devnet` — publishes the IDL on-chain so
   AgentGeyser's `idl-registry` can auto-discover it.
5. Writes the program ID into `PROGRAM_ID.txt`, replacing
   `<PENDING_DEPLOY>`.

The script is guarded with `set -euo pipefail` and a loud
`⚠️ requires a funded devnet keypair; DO NOT run in CI` banner.

## Replacing `<PENDING_DEPLOY>`

After a successful `deploy.sh`, `PROGRAM_ID.txt` is rewritten in place
with the deployed program ID (a base58 pubkey). If you prefer to deploy
by hand, replace the literal string `<PENDING_DEPLOY>` with the pubkey
reported by `solana-keygen pubkey target/deploy/hello_world-keypair.json`.

Do **not** commit the program keypair file; it is covered by
`.gitignore` alongside `target/`, `test-ledger/`, `**/*.so`, and
`**/*.keypair.json`.

## No keys committed

No private key material ever enters this directory. The placeholder
program ID `11111111111111111111111111111112` in `Anchor.toml` and
`declare_id!` is the Solana system program sentinel — it is a public,
well-known address, never a user key, and is replaced at deploy time
by the actual program ID emitted by `anchor build`.
