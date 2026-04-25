import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AgentGeyserClient } from '../client.js';
import {
  AgentGeyserError,
  NetworkError,
  RpcError,
  SkillNotFound,
  ValidationError,
} from '../errors.js';
import type { FetchLike, Skill } from '../types.js';

/** Minimal mock of the Fetch Response surface we consume. */
function mockResponse(body: unknown, init?: { ok?: boolean; status?: number; statusText?: string }) {
  return {
    ok: init?.ok ?? true,
    status: init?.status ?? 200,
    statusText: init?.statusText ?? 'OK',
    async text() {
      return typeof body === 'string' ? body : JSON.stringify(body);
    },
    async json() {
      return body;
    },
  };
}

const PROXY_URL = 'http://127.0.0.1:8999';

describe('AgentGeyserClient', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('constructor', () => {
    it('rejects missing proxyUrl', () => {
      // @ts-expect-error — intentionally invalid at runtime
      expect(() => new AgentGeyserClient({})).toThrow(ValidationError);
      // @ts-expect-error — intentionally invalid at runtime
      expect(() => new AgentGeyserClient({ proxyUrl: '' })).toThrow(ValidationError);
    });
  });

  describe('listSkills', () => {
    it('returns the parsed skill array on success (happy path)', async () => {
      const skills = [
        {
          skill_id: 'spl-token::transfer',
          program_id: 'spl-token',
          instruction_name: 'transfer',
          args: [{ name: 'amount', ty: 'u64' }],
          accounts: [{ name: 'source', is_mut: true, is_signer: false }],
          params_schema: { type: 'object' },
          discriminator: [12, 0, 0, 0, 0, 0, 0, 0],
        },
      ];
      fetchMock.mockResolvedValueOnce(mockResponse({ jsonrpc: '2.0', id: 1, result: skills }));

      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      const out = await client.listSkills();

      expect(out).toEqual([
        {
          skillId: 'spl-token::transfer',
          programId: 'spl-token',
          instructionName: 'transfer',
          args: [{ name: 'amount', ty: 'u64' }],
          accounts: [{ name: 'source', isMut: true, isSigner: false }],
          paramsSchema: { type: 'object' },
          discriminator: [12, 0, 0, 0, 0, 0, 0, 0],
        },
      ] satisfies Skill[]);
      expect(fetchMock).toHaveBeenCalledOnce();
      const call = fetchMock.mock.calls[0];
      expect(call[0]).toBe(PROXY_URL);
      const init = call[1] as { method: string; body: string; headers: Record<string, string> };
      expect(init.method).toBe('POST');
      expect(init.headers['content-type']).toBe('application/json');
      const parsed = JSON.parse(init.body) as { method: string; params: unknown; jsonrpc: string };
      expect(parsed.jsonrpc).toBe('2.0');
      expect(parsed.method).toBe('ag_listSkills');
      expect(parsed.params).toEqual([]);
    });

    it('throws NetworkError on non-2xx HTTP status', async () => {
      fetchMock.mockResolvedValue(
        mockResponse('', { ok: false, status: 502, statusText: 'Bad Gateway' }),
      );
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      const err = await client.listSkills().catch((e: unknown) => e);
      expect(err).toBeInstanceOf(NetworkError);
      expect((err as NetworkError).code).toBe('network_error');
      expect((err as NetworkError).status).toBe(502);
    });

    it('throws NetworkError when fetch itself rejects', async () => {
      fetchMock.mockRejectedValueOnce(new Error('ECONNREFUSED'));
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      const err = await client.listSkills().catch((e: unknown) => e);
      expect(err).toBeInstanceOf(NetworkError);
      expect((err as AgentGeyserError).code).toBe('network_error');
    });

    it('throws ValidationError when result is not an array', async () => {
      fetchMock.mockResolvedValueOnce(
        mockResponse({ jsonrpc: '2.0', id: 1, result: { not: 'an array' } }),
      );
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      await expect(client.listSkills()).rejects.toBeInstanceOf(ValidationError);
    });

    it('surfaces JSON-RPC error as RpcError with rpcCode preserved', async () => {
      fetchMock.mockResolvedValueOnce(
        mockResponse({
          jsonrpc: '2.0',
          id: 1,
          error: { code: -32000, message: 'internal boom', data: { hint: 'retry' } },
        }),
      );
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      const err = await client.listSkills().catch((e: unknown) => e);
      expect(err).toBeInstanceOf(RpcError);
      expect((err as RpcError).rpcCode).toBe(-32000);
      expect((err as RpcError).code).toBe('rpc_error');
      expect((err as RpcError).data).toEqual({ hint: 'retry' });
    });
  });

  describe('invokeSkill', () => {
    const validRequest = {
      skill_id: 'spl-token::transfer',
      args: { amount: 1 },
      accounts: { source: 'A', destination: 'B', authority: 'C' },
      payer: 'C',
    } as const;

    it('returns transactionBase64 on success (happy path)', async () => {
      fetchMock.mockResolvedValueOnce(
        mockResponse({ jsonrpc: '2.0', id: 1, result: { transactionBase64: 'AAA=' } }),
      );
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      const out = await client.invokeSkill({ ...validRequest });
      expect(out.transactionBase64).toBe('AAA=');
      const parsed = JSON.parse(
        (fetchMock.mock.calls[0][1] as { body: string }).body,
      ) as { method: string; params: unknown };
      expect(parsed.method).toBe('ag_invokeSkill');
      expect(parsed.params).toMatchObject({ skill_id: 'spl-token::transfer' });
    });

    it('throws ValidationError when required fields are missing', async () => {
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      // @ts-expect-error — intentionally invalid at runtime
      await expect(client.invokeSkill({})).rejects.toBeInstanceOf(ValidationError);
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it('maps proxy "skill not found" error to SkillNotFound', async () => {
      fetchMock.mockResolvedValueOnce(
        mockResponse({
          jsonrpc: '2.0',
          id: 1,
          error: { code: -32001, message: 'Skill not found: mystery::skill' },
        }),
      );
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      const err = await client
        .invokeSkill({ ...validRequest, skill_id: 'mystery::skill' })
        .catch((e: unknown) => e);
      expect(err).toBeInstanceOf(SkillNotFound);
      expect((err as SkillNotFound).skillId).toBe('mystery::skill');
      expect((err as SkillNotFound).code).toBe('skill_not_found');
    });

    it('maps -32602 invalid params to ValidationError', async () => {
      fetchMock.mockResolvedValueOnce(
        mockResponse({
          jsonrpc: '2.0',
          id: 1,
          error: { code: -32602, message: 'Invalid params: amount < 0' },
        }),
      );
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      await expect(client.invokeSkill({ ...validRequest })).rejects.toBeInstanceOf(ValidationError);
    });

    it('invokeSkill accepts proxy snake_case transaction_base64', async () => {
      fetchMock.mockResolvedValueOnce(
        mockResponse({
          jsonrpc: '2.0',
          id: 1,
          result: { transaction_base64: 'AAECAwQ=', skill_id: 'spl-token::transfer' },
        }),
      );
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      const result = await client.invokeSkill({ ...validRequest });
      expect(result.transactionBase64).toBe('AAECAwQ=');
    });

    it('throws ValidationError when result shape is wrong', async () => {
      fetchMock.mockResolvedValueOnce(
        mockResponse({ jsonrpc: '2.0', id: 1, result: { wrong: 'field' } }),
      );
      const client = new AgentGeyserClient({
        proxyUrl: PROXY_URL,
        fetch: fetchMock as unknown as FetchLike,
      });
      await expect(client.invokeSkill({ ...validRequest })).rejects.toBeInstanceOf(ValidationError);
    });
  });

  describe('error hierarchy', () => {
    it('every subclass is an AgentGeyserError', () => {
      expect(new RpcError('x', -1)).toBeInstanceOf(AgentGeyserError);
      expect(new NetworkError('x')).toBeInstanceOf(AgentGeyserError);
      expect(new SkillNotFound('x')).toBeInstanceOf(AgentGeyserError);
      expect(new ValidationError('x')).toBeInstanceOf(AgentGeyserError);
    });
  });
});
