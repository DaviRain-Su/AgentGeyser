/**
 * `agentgeyser` CLI wrapper. `bin/agentgeyser` imports the compiled
 * `dist/cli.js` and calls {@link run}. Non-custodial: the `--keypair` flag
 * is passed straight through to {@link signAndSend}; this module never
 * parses or constructs key material itself.
 */

import { Command, type Command as CommanderCommand } from 'commander';
import { AgentGeyserClient } from './client.js';
import { signAndSend } from './signAndSend.js';

export const DEFAULT_PROXY_URL = 'http://127.0.0.1:8999';
export const DEFAULT_RPC_URL = 'http://127.0.0.1:8899';

export interface ListSkillsOptions { proxyUrl: string; }
export interface InvokeOptions {
  proxyUrl: string;
  skill: string;
  args: string;
  accounts: string;
  payer: string;
  sign?: boolean;
  keypair?: string;
  rpcUrl: string;
}

/** Injectable action handlers — unit tests swap these for spies. */
export interface CliHandlers {
  listSkills: (opts: ListSkillsOptions) => Promise<void> | void;
  invoke: (opts: InvokeOptions) => Promise<void> | void;
}

function parseJsonObject(raw: string, flag: string): Record<string, unknown> {
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('expected JSON object');
    return parsed as Record<string, unknown>;
  } catch (err) {
    throw new Error(`${flag}: invalid JSON (${err instanceof Error ? err.message : String(err)})`);
  }
}

export const defaultHandlers: CliHandlers = {
  async listSkills(opts) {
    const client = new AgentGeyserClient({ proxyUrl: opts.proxyUrl });
    process.stdout.write(`${JSON.stringify(await client.listSkills(), null, 2)}\n`);
  },
  async invoke(opts) {
    const client = new AgentGeyserClient({ proxyUrl: opts.proxyUrl });
    const { transactionBase64 } = await client.invokeSkill({
      skill_id: opts.skill,
      args: parseJsonObject(opts.args, '--args'),
      accounts: parseJsonObject(opts.accounts, '--accounts'),
      payer: opts.payer,
    });
    if (!opts.sign) {
      process.stdout.write(`${JSON.stringify({ transactionBase64 }, null, 2)}\n`);
      return;
    }
    if (!opts.keypair) throw new Error('--sign requires --keypair <path>');
    const result = await signAndSend({ client, transactionBase64, keypairPath: opts.keypair, rpcUrl: opts.rpcUrl });
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  },
};

/** Build the commander program with (optionally) stubbed handlers. */
export function buildProgram(handlers: CliHandlers = defaultHandlers): CommanderCommand {
  const program = new Command();
  program.name('agentgeyser').description('AgentGeyser SDK command-line interface').exitOverride();

  program
    .command('list-skills')
    .description('List skills exposed by the proxy (ag_listSkills)')
    .option('--proxy-url <url>', 'AgentGeyser proxy URL', DEFAULT_PROXY_URL)
    .action(async (raw: { proxyUrl: string }) => handlers.listSkills({ proxyUrl: raw.proxyUrl }));

  program
    .command('invoke')
    .description('Invoke a skill (ag_invokeSkill); optionally sign+submit')
    .requiredOption('--skill <id>', 'Skill id, e.g. spl-token::transfer')
    .requiredOption('--args <json>', 'Skill args as a JSON object')
    .requiredOption('--accounts <json>', 'Accounts as a JSON object')
    .requiredOption('--payer <pubkey>', 'Fee payer pubkey (base58)')
    .option('--proxy-url <url>', 'AgentGeyser proxy URL', DEFAULT_PROXY_URL)
    .option('--sign', 'Sign with --keypair and submit via --rpc-url', false)
    .option('--keypair <path>', 'Path to a JSON keypair file (required with --sign)')
    .option('--rpc-url <url>', 'Solana RPC endpoint for signed submission', DEFAULT_RPC_URL)
    .action(async (raw: InvokeOptions) => handlers.invoke(raw));

  return program;
}

/** Entry point invoked by `bin/agentgeyser`. */
export async function run(
  argv: string[] = process.argv,
  handlers: CliHandlers = defaultHandlers,
): Promise<void> {
  await buildProgram(handlers).parseAsync(argv);
}
