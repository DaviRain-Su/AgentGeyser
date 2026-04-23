# AgentGeyser Spike Demo

Three commands to reproduce the end-to-end Spike on your laptop. Run them
from `skeleton/`.

```bash
# 1. Compile and start the proxy (mock Yellowstone mode is the default).
cargo run -p proxy

# 2. In another terminal, install Node deps for the SDK + demo.
pnpm install

# 3. Run the demo against the running proxy.
pnpm demo
```

Expected output is captured verbatim in [`recording.txt`](./recording.txt).

## What the demo does

1. Connects to the proxy at `http://127.0.0.1:8899` (override with
   `AGENTGEYSER_ENDPOINT`).
2. Calls `ag_listSkills` once via the SDK's lazy catalog loader and prints
   each `<program>::<instruction>` pair.
3. Invokes `client.hello_world.greet({ name: 'Spike' })`, which dispatches
   through the dynamic `Proxy` to `ag_invokeSkill` and prints the unsigned
   `transaction_base64`.

The proxy seeds the registry with the fixture in
[`fixtures/hello_world.idl.json`](./fixtures/hello_world.idl.json) so a fresh
boot already has 3 skills available.

## Non-custodial reminder

`ag_invokeSkill` returns an **unsigned** transaction. The Spike intentionally
ships a placeholder string (`SPIKE_UNSIGNED_TX`) instead of a real serialized
TX — the next milestone (MVP) will produce real Solana transactions, but
signing always happens in the caller's wallet, never inside AgentGeyser.
