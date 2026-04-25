import { describe, expect, it, vi } from 'vitest';
import { AgentGeyserClient } from '../client.js';
import type { FetchLike } from '../types.js';

const PROXY_URL = 'http://127.0.0.1:8999';

const proxySkill = {
  skill_id: 'spl-token::transfer',
  program_id: 'spl-token',
  program_name: 'SPL Token',
  instruction_name: 'transfer',
  args: [{ name: 'amount', ty: 'u64' }],
  accounts: [{ name: 'source', is_mut: true, is_signer: false }],
  params_schema: { type: 'object', required: ['amount'] },
  discriminator: [12, 0, 0, 0, 0, 0, 0, 0],
};

function response(body: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: 'OK',
    async text() { return JSON.stringify(body); },
    async json() { return body; },
  };
}

describe('AgentGeyserClient.listSkills', () => {
  it('Skill type round-trip: listSkills returns camelCase Skill fields from proxy snake_case JSON', async () => {
    const fetchMock = vi.fn(async () => response({
      jsonrpc: '2.0',
      id: 1,
      result: [proxySkill],
    }));
    const client = new AgentGeyserClient({
      proxyUrl: PROXY_URL,
      fetch: fetchMock as unknown as FetchLike,
    });

    await expect(client.listSkills()).resolves.toEqual([
      {
        skillId: 'spl-token::transfer',
        programId: 'spl-token',
        programName: 'SPL Token',
        instructionName: 'transfer',
        args: [{ name: 'amount', ty: 'u64' }],
        accounts: [{ name: 'source', isMut: true, isSigner: false }],
        paramsSchema: { type: 'object', required: ['amount'] },
        discriminator: [12, 0, 0, 0, 0, 0, 0, 0],
      },
    ]);
  });
});
