# AgentGeyser — Live Mode Guide

This document explains how to run the AgentGeyser `proxy` binary against a
**real** Solana cluster by subscribing to a hosted Yellowstone gRPC feed and
fetching Anchor IDLs from on-chain PDAs.

CI stays on the mock path; live mode is entirely opt-in via environment
variables and requires the `live-yellowstone` Cargo feature to be enabled at
build time.

## Obtaining a Yellowstone endpoint

Yellowstone is the standard gRPC geyser plugin that mainstream Solana RPC
providers expose. You need a URL and a bearer token from one of them:

- **Triton One** — sign up at <https://triton.one>, create a Yellowstone
  endpoint from the dashboard, and copy the `https://…` URL plus the token.
- **Helius** — sign up at <https://helius.dev>, enable the Yellowstone add-on
  for your project, and copy the URL plus the API key used as the token.

Either provider works; AgentGeyser only speaks the standard
`yellowstone-grpc-proto` subscription schema.

## Environment variables

| Variable                              | Required | Description                                                              |
| ------------------------------------- | -------- | ------------------------------------------------------------------------ |
| `AGENTGEYSER_YELLOWSTONE_ENDPOINT`    | yes      | Yellowstone gRPC URL, e.g. `https://yellowstone.triton.one`.             |
| `AGENTGEYSER_YELLOWSTONE_TOKEN`       | yes      | Bearer / `x-token` credential for the Yellowstone endpoint.              |
| `AGENTGEYSER_RPC_URL`                 | yes      | Solana JSON-RPC endpoint used to fetch Anchor IDL accounts (devnet ok).  |

All three must be set for the proxy to flip from `mode=mock` to `mode=live`.
`AGENTGEYSER_BIND` (optional, default `127.0.0.1:8899`) is unchanged from the
Spike.

## Deploying a test Anchor program on devnet

To see the live path actually fire you need an Anchor program with an IDL
account on devnet. The fastest route is the upstream Anchor hello-world
example: <https://github.com/coral-xyz/anchor/tree/master/examples/tutorial/basic-0>.

Short recipe:

1. `anchor init hello_world && cd hello_world`
2. Edit `Anchor.toml` to set `cluster = "devnet"`.
3. `solana-keygen new -o ~/.config/solana/id.json` (skip if you already have one).
4. `solana airdrop 2 --url https://api.devnet.solana.com`.
5. `anchor build && anchor deploy`.
6. `anchor idl init <PROGRAM_ID> -f target/idl/hello_world.json`.

Once step 6 lands, the IDL account becomes discoverable via the Anchor IDL
PDA and the proxy will pick it up the next time Yellowstone emits a
program-write event for `<PROGRAM_ID>`.

## Running the proxy in live mode

```bash
export AGENTGEYSER_YELLOWSTONE_ENDPOINT="https://<your-triton-or-helius>/"
export AGENTGEYSER_YELLOWSTONE_TOKEN="<token-from-dashboard>"
export AGENTGEYSER_RPC_URL="https://api.devnet.solana.com"

cargo run -p proxy --features live-yellowstone
```

On startup the first tracing line will contain `mode=live`. If any of the
three variables above is missing the proxy falls back to `mode=mock` and
logs that instead — useful for sanity-checking your shell environment.

## Troubleshooting

- **`ECONNREFUSED` / connection refused** — the endpoint URL is wrong or the
  host is unreachable from your network. Re-copy the URL from the provider
  dashboard and confirm with `curl -I <endpoint>`. Corporate VPNs frequently
  block gRPC; try from a plain network first.
- **`401 Unauthorized`** — the token is missing, expired, or belongs to a
  different project. Regenerate it in the provider dashboard and make sure
  you exported it in the same shell where you run `cargo run`. Do not wrap
  the token in quotes that your shell will strip.
- **`IDL not found on chain` / `Ok(None)` from `fetch_anchor_idl`** — the
  target program has not published its IDL via `anchor idl init` yet, or you
  pointed `AGENTGEYSER_RPC_URL` at a cluster (e.g. mainnet) where the
  program is not deployed. Double-check with `solana account <IDL_PDA>
  --url <rpc>`.
- **Rate limiting / `429 Too Many Requests`** — free Helius/Triton tiers
  throttle aggressively. Back off the polling interval or upgrade the plan.
- **`cargo` says `feature 'live-yellowstone' is not enabled`** — you forgot
  `--features live-yellowstone` on the `cargo run` / `cargo build` command.

## Security

**Never commit real endpoints or tokens.** The repo-level `.gitignore`
already excludes `target/`, `node_modules/`, `dist/`, `*.log`, and
`.DS_Store`; extend it with `.env`, `.env.*`, and any local secret files
before you paste a token into your shell history. Treat `AGENTGEYSER_*`
credentials like production API keys:

- Keep them in `~/.config/agentgeyser/env` (outside the working tree) and
  `source` it on demand.
- Rotate tokens whenever a laptop is lost or a teammate leaves.
- If you ever accidentally commit one, revoke it in the provider dashboard
  immediately and force-rotate before rewriting history.

If you find a committed secret during review, stop and file an incident
report; do **not** simply delete the line in a follow-up commit.
