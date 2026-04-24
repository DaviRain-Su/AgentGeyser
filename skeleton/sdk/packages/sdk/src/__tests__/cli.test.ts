import { describe, expect, it, vi } from 'vitest';
import { buildProgram, run, DEFAULT_PROXY_URL, DEFAULT_RPC_URL, type CliHandlers } from '../cli.js';

function spyHandlers(): CliHandlers & { listSkills: ReturnType<typeof vi.fn>; invoke: ReturnType<typeof vi.fn> } {
  return { listSkills: vi.fn(async () => {}), invoke: vi.fn(async () => {}) };
}

const FAKE_PAYER = 'Fh3A4pc8YtQvfy5rz9HDXraX5kyn4AFkXyk1V8oWLP13';

describe('agentgeyser CLI', () => {
  it('registers both list-skills and invoke subcommands', () => {
    const names = buildProgram(spyHandlers()).commands.map((c) => c.name());
    expect(names).toEqual(expect.arrayContaining(['list-skills', 'invoke']));
  });

  it('parses `invoke --skill spl-token::transfer ...` options into the action handler', async () => {
    const handlers = spyHandlers();
    await run(
      [
        'node', 'agentgeyser', 'invoke',
        '--skill', 'spl-token::transfer',
        '--args', '{"amount":1}',
        '--accounts', '{}',
        '--payer', FAKE_PAYER,
      ],
      handlers,
    );
    expect(handlers.invoke).toHaveBeenCalledTimes(1);
    expect(handlers.listSkills).not.toHaveBeenCalled();
    expect(handlers.invoke.mock.calls[0]?.[0]).toMatchObject({
      skill: 'spl-token::transfer',
      args: '{"amount":1}',
      accounts: '{}',
      payer: FAKE_PAYER,
      proxyUrl: DEFAULT_PROXY_URL,
      rpcUrl: DEFAULT_RPC_URL,
      sign: false,
    });
  });

  it('forwards --sign and --keypair through to the invoke handler', async () => {
    const handlers = spyHandlers();
    await run(
      [
        'node', 'agentgeyser', 'invoke',
        '--skill', 'spl-token::transfer',
        '--args', '{"amount":1}',
        '--accounts', '{}',
        '--payer', FAKE_PAYER,
        '--sign',
        '--keypair', '/tmp/fake-keypair.json',
        '--rpc-url', 'http://127.0.0.1:9988',
      ],
      handlers,
    );
    expect(handlers.invoke.mock.calls[0]?.[0]).toMatchObject({
      sign: true,
      keypair: '/tmp/fake-keypair.json',
      rpcUrl: 'http://127.0.0.1:9988',
    });
  });

  it('delegates list-skills to the listSkills handler with the default proxy URL', async () => {
    const handlers = spyHandlers();
    await run(['node', 'agentgeyser', 'list-skills'], handlers);
    expect(handlers.listSkills).toHaveBeenCalledWith({ proxyUrl: DEFAULT_PROXY_URL });
    expect(handlers.invoke).not.toHaveBeenCalled();
  });
});
