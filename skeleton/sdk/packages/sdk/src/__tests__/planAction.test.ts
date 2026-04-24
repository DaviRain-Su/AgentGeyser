import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AgentGeyserClient } from '../client.js';
import { RpcError } from '../errors.js';
import type { FetchLike } from '../types.js';

const PROXY_URL = 'http://127.0.0.1:8999';
const resp = (body: unknown) => ({
  ok: true, status: 200, statusText: 'OK',
  async text() { return JSON.stringify(body); },
  async json() { return body; },
});

describe('AgentGeyserClient.planAction', () => {
  let fetchMock: ReturnType<typeof vi.fn>;
  beforeEach(() => { fetchMock = vi.fn(); });
  afterEach(() => { vi.restoreAllMocks(); });

  const mkClient = () => new AgentGeyserClient({
    proxyUrl: PROXY_URL,
    fetch: fetchMock as unknown as FetchLike,
  });
  const outBody = () =>
    JSON.parse((fetchMock.mock.calls[0][1] as { body: string }).body) as {
      method: string; params: Record<string, unknown>;
    };

  it('happy path: maps snake_case skill_id to camelCase skillId', async () => {
    fetchMock.mockResolvedValueOnce(resp({
      jsonrpc: '2.0', id: 1,
      result: { skill_id: 'spl-token-transfer', args: { amount: 100 }, rationale: 'test' },
    }));
    const plan = await mkClient().planAction({ prompt: 'send 100 tokens', provider: 'mock' });
    expect(plan).toEqual({ skillId: 'spl-token-transfer', args: { amount: 100 }, rationale: 'test' });
    const { method, params } = outBody();
    expect(method).toBe('ag_planAction');
    expect(params).toEqual({ prompt: 'send 100 tokens', provider: 'mock' });
  });

  it('surfaces JSON-RPC upstream error as RpcError with message', async () => {
    fetchMock.mockResolvedValueOnce(resp({
      jsonrpc: '2.0', id: 1,
      error: { code: -32000, message: 'LLM upstream error' },
    }));
    const err = await mkClient().planAction({ prompt: 'anything' }).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(RpcError);
    expect((err as Error).message).toContain('LLM upstream error');
  });

  it('omits `provider` from outbound params when input.provider is undefined', async () => {
    fetchMock.mockResolvedValueOnce(resp({
      jsonrpc: '2.0', id: 1,
      result: { skill_id: 'noop', args: {}, rationale: 'r' },
    }));
    await mkClient().planAction({ prompt: 'p' });
    const { params } = outBody();
    expect(params).toEqual({ prompt: 'p' });
    expect('provider' in params).toBe(false);
  });
});
