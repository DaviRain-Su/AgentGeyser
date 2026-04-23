import { describe, expect, it, vi } from 'vitest';
import {
  AgentGeyserClient,
  UnknownProgramError,
  UnknownSkillError,
  type Skill,
} from '../src/index.js';

function jsonResponse(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  });
}

const sampleSkills: Skill[] = [
  {
    skill_id: 'HELLO::greet',
    program_id: 'HELLO',
    program_name: 'hello_world',
    instruction_name: 'greet',
    params_schema: { type: 'object' },
  },
  {
    skill_id: 'HELLO::set_counter',
    program_id: 'HELLO',
    program_name: 'hello_world',
    instruction_name: 'set_counter',
    params_schema: { type: 'object' },
  },
];

describe('AgentGeyserClient', () => {
  it('fetches the catalog once and reuses it across dispatches', async () => {
    const fetchImpl = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      const body = JSON.parse(init!.body as string);
      if (body.method === 'ag_listSkills') {
        return jsonResponse({ jsonrpc: '2.0', id: body.id, result: sampleSkills });
      }
      if (body.method === 'ag_invokeSkill') {
        return jsonResponse({
          jsonrpc: '2.0',
          id: body.id,
          result: {
            skill_id: body.params.skill_id,
            transaction_base64: 'SPIKE_UNSIGNED_TX',
          },
        });
      }
      throw new Error('unexpected method ' + body.method);
    });

    const client = AgentGeyserClient.create({ endpoint: 'http://mock', fetchImpl: fetchImpl as typeof fetch });

    const first = await client.hello_world.greet({ name: 'Alice' });
    expect(first.transaction_base64).toBe('SPIKE_UNSIGNED_TX');
    expect(first.skill_id).toBe('HELLO::greet');

    const second = await client.hello_world.set_counter({ value: 5 });
    expect(second.skill_id).toBe('HELLO::set_counter');

    // One catalog fetch + two invoke calls = 3 fetch calls total.
    expect(fetchImpl).toHaveBeenCalledTimes(3);
    const catalogCalls = fetchImpl.mock.calls.filter(
      ([, init]) => (JSON.parse((init as RequestInit).body as string).method === 'ag_listSkills'),
    );
    expect(catalogCalls).toHaveLength(1);
  });

  it('throws UnknownProgramError / UnknownSkillError for missing entries', async () => {
    const fetchImpl = vi.fn(async (_url, init?: RequestInit) => {
      const body = JSON.parse(init!.body as string);
      return jsonResponse({ jsonrpc: '2.0', id: body.id, result: sampleSkills });
    });
    const client = AgentGeyserClient.create({ endpoint: 'http://mock', fetchImpl: fetchImpl as typeof fetch });

    await expect(client.unknown_program.something({})).rejects.toBeInstanceOf(UnknownProgramError);
    await expect(client.hello_world.does_not_exist({})).rejects.toBeInstanceOf(UnknownSkillError);
  });

  it('surfaces JSON-RPC errors from the proxy', async () => {
    const fetchImpl = vi.fn(async (_url, init?: RequestInit) => {
      const body = JSON.parse(init!.body as string);
      if (body.method === 'ag_listSkills') {
        return jsonResponse({ jsonrpc: '2.0', id: body.id, result: sampleSkills });
      }
      return jsonResponse({
        jsonrpc: '2.0',
        id: body.id,
        error: { code: -32004, message: 'skill not found' },
      });
    });
    const client = AgentGeyserClient.create({ endpoint: 'http://mock', fetchImpl: fetchImpl as typeof fetch });

    await expect(client.hello_world.greet({})).rejects.toThrow(/-32004/);
  });
});
