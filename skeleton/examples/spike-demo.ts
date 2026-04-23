/**
 * AgentGeyser Spike demo.
 *
 * Usage (from skeleton/):
 *   1. cargo run -p proxy
 *   2. pnpm install
 *   3. pnpm tsx examples/spike-demo.ts
 */

import { AgentGeyserClient } from '../packages/sdk/src/index.js';

const ENDPOINT = process.env.AGENTGEYSER_ENDPOINT ?? 'http://127.0.0.1:8899';

async function main(): Promise<void> {
  console.log(`[agentgeyser] connecting to ${ENDPOINT}`);
  const client = AgentGeyserClient.create({ endpoint: ENDPOINT });

  let skills;
  try {
    skills = await client.listSkills();
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    if (/ECONNREFUSED|fetch failed/i.test(message)) {
      console.error(`[agentgeyser] Proxy not reachable at ${ENDPOINT}`);
      console.error(`[agentgeyser] Start it with: cargo run -p proxy`);
      process.exit(1);
    }
    throw err;
  }

  console.log(`[agentgeyser] Discovered ${skills.length} skills`);
  for (const s of skills) {
    console.log(`  - ${s.program_name ?? s.program_id}::${s.instruction_name}  (${s.skill_id})`);
  }

  if (skills.length === 0) {
    console.warn('[agentgeyser] No skills available; is the proxy in mock mode?');
    return;
  }

  const result = await client.hello_world.greet({ name: 'Spike' });
  console.log(`[agentgeyser] invoked ${result.skill_id}`);
  console.log(`[agentgeyser] unsigned TX: ${result.transaction_base64}`);
}

main().catch((err) => {
  console.error('[agentgeyser] fatal:', err);
  process.exit(1);
});
