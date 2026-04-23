/**
 * examples/live-smoke.ts — manual smoke test for AgentGeyser live mode.
 *
 * Polls `ag_listSkills` every 10s via the proxy at AGENTGEYSER_ENDPOINT
 * (default http://127.0.0.1:8899) and prints the DIFF vs the previous
 * snapshot — i.e. skill ids that just appeared in the catalog.
 *
 * No npm deps beyond the workspace SDK; Node 20+ built-in fetch suffices.
 */

import { AgentGeyserClient, type Skill } from '../packages/sdk/src/index.js';

const DEFAULT_ENDPOINT = 'http://127.0.0.1:8899';
const POLL_INTERVAL_MS = 10_000;

function usage(): void {
  console.log(
    [
      'Usage: pnpm tsx examples/live-smoke.ts [--help]',
      '',
      `  Polls ag_listSkills every ${POLL_INTERVAL_MS / 1000}s and prints newly`,
      '  discovered skill ids (diff vs the previous poll).',
      '',
      'Environment:',
      `  AGENTGEYSER_ENDPOINT   Proxy JSON-RPC endpoint (default ${DEFAULT_ENDPOINT}).`,
    ].join('\n'),
  );
}

const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

async function pollOnce(
  client: ReturnType<typeof AgentGeyserClient.create>,
  seen: Set<string>,
): Promise<void> {
  const skills: Skill[] = await client.refreshSkills();
  const fresh: string[] = [];
  for (const skill of skills) {
    if (!seen.has(skill.skill_id)) {
      seen.add(skill.skill_id);
      fresh.push(skill.skill_id);
    }
  }
  const ts = new Date().toISOString();
  if (fresh.length === 0) {
    console.log(`[${ts}] no diff (catalog size=${skills.length})`);
  } else {
    console.log(`[${ts}] +${fresh.length} new skill(s): ${fresh.join(', ')}`);
  }
}

async function main(): Promise<void> {
  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    usage();
    return;
  }
  const endpoint = process.env.AGENTGEYSER_ENDPOINT ?? DEFAULT_ENDPOINT;
  console.log(`[live-smoke] polling ${endpoint} every ${POLL_INTERVAL_MS}ms`);
  const client = AgentGeyserClient.create({ endpoint });
  const seen = new Set<string>();
  for (;;) {
    try {
      await pollOnce(client, seen);
    } catch (err) {
      console.error('[live-smoke] poll failed:', (err as Error).message);
    }
    await sleep(POLL_INTERVAL_MS);
  }
}

main().catch((err: unknown) => {
  console.error('[live-smoke] fatal:', err);
  process.exit(1);
});
