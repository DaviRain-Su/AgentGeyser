/**
 * examples/live-smoke.ts — manual smoke test for AgentGeyser live mode.
 *
 * Default (no flags): polls `ag_listSkills` every 10s via the proxy at
 * AGENTGEYSER_ENDPOINT (default http://127.0.0.1:8899) and prints the DIFF
 * vs the previous snapshot (M1 behavior).
 *
 * With `--e2e`: performs the Track B round trip (listSkills → invokeSkill
 * for `spl-token::transfer` → sign-and-send on devnet) and exits.
 *
 * No npm deps beyond the workspace SDK and sign-and-send.ts; Node 20+
 * built-in fetch suffices.
 */

import { AgentGeyserClient, type Skill } from '../packages/sdk/src/index.js';

const DEFAULT_ENDPOINT = 'http://127.0.0.1:8899';
const POLL_INTERVAL_MS = 10_000;

function usage(): void {
  console.log(
    [
      'Usage: pnpm tsx examples/live-smoke.ts [--help] [--e2e]',
      '',
      `  Default: polls ag_listSkills every ${POLL_INTERVAL_MS / 1000}s and prints newly`,
      '           discovered skill ids (diff vs the previous poll).',
      '  --e2e    Run Track B round trip once: listSkills → invokeSkill',
      '           (spl-token::transfer) → sign-and-send on devnet, then exit.',
      '',
      'Environment:',
      `  AGENTGEYSER_ENDPOINT       Proxy JSON-RPC endpoint (default ${DEFAULT_ENDPOINT}).`,
      '  AGENTGEYSER_DEMO_SOURCE    SPL token source account (pubkey)   [--e2e only]',
      '  AGENTGEYSER_DEMO_DEST      SPL token destination account       [--e2e only]',
      '  AGENTGEYSER_DEMO_AUTHORITY SPL token authority (payer/signer)  [--e2e only]',
      '  AGENTGEYSER_DEMO_KEYPAIR   JSON file path OR base58 secret     [--e2e only]',
      '  AGENTGEYSER_RPC_URL        Solana RPC (default devnet)         [--e2e only]',
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

function required(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`missing env var: ${name}`);
  return v;
}

async function rpc<T>(endpoint: string, method: string, params: unknown): Promise<T> {
  const resp = await fetch(endpoint, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
  });
  if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  const body = (await resp.json()) as { result?: T; error?: { code: number; message: string } };
  if (body.error) throw new Error(`RPC ${body.error.code}: ${body.error.message}`);
  if (body.result === undefined) throw new Error('RPC: missing result');
  return body.result;
}

async function runE2e(endpoint: string): Promise<void> {
  const client = AgentGeyserClient.create({ endpoint });
  const skills: Skill[] = await client.listSkills();
  console.log(JSON.stringify({ step: 'listSkills', ok: true, count: skills.length }));
  const spl = skills.find((s) => s.skill_id === 'spl-token::transfer');
  if (!spl) throw new Error('spl-token::transfer skill not registered');
  const invoke = await rpc<{ skill_id: string; transaction_base64: string }>(
    endpoint,
    'ag_invokeSkill',
    {
      skill_id: spl.skill_id,
      args: { amount: 1 },
      accounts: {
        source: required('AGENTGEYSER_DEMO_SOURCE'),
        destination: required('AGENTGEYSER_DEMO_DEST'),
        authority: required('AGENTGEYSER_DEMO_AUTHORITY'),
      },
      payer: required('AGENTGEYSER_DEMO_AUTHORITY'),
    },
  );
  const tx = invoke.transaction_base64;
  console.log(JSON.stringify({ step: 'invokeSkill', ok: true, tx_bytes: tx.length }));
  const { signAndSend } = await import('./sign-and-send.js');
  const res = await signAndSend(tx);
  console.log(
    JSON.stringify({
      step: 'signAndSend',
      ok: true,
      signature: res.signature,
      explorer: `https://explorer.solana.com/tx/${res.signature}?cluster=devnet`,
    }),
  );
}

async function main(): Promise<void> {
  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    usage();
    return;
  }
  const endpoint = process.env.AGENTGEYSER_ENDPOINT ?? DEFAULT_ENDPOINT;
  if (process.argv.includes('--e2e')) {
    await runE2e(endpoint);
    return;
  }
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
