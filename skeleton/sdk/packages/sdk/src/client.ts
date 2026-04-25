/**
 * JSON-RPC 2.0 client for the AgentGeyser proxy. Isomorphic: uses
 * `globalThis.fetch` where available and falls back to `cross-fetch`.
 * No `fs`, no keypair material — signing lives in `signAndSend.ts`.
 */

import crossFetch from 'cross-fetch';
import {
  NetworkError,
  RpcError,
  SkillNotFound,
  ValidationError,
} from './errors.js';
import type {
  AgentGeyserClientOptions,
  FetchLike,
  InvokeSkillRequest,
  InvokeSkillResponse,
  JsonRpcResponse,
  Plan,
  PlanActionRequest,
  Skill,
  SkillAccount,
  SkillArg,
} from './types.js';

/** Resolve a fetch: caller-supplied → global fetch → cross-fetch. */
function resolveFetch(provided?: FetchLike): FetchLike {
  if (provided) return provided;
  const globalFetch = (globalThis as { fetch?: FetchLike }).fetch;
  if (typeof globalFetch === 'function') return globalFetch;
  return crossFetch as unknown as FetchLike;
}

type WireSkill = {
  skill_id?: unknown;
  program_id?: unknown;
  program_name?: unknown;
  instruction_name?: unknown;
  args?: unknown;
  accounts?: unknown;
  params_schema?: unknown;
  discriminator?: unknown;
};

function normalizeSkillArg(value: unknown): SkillArg {
  if (!value || typeof value !== 'object') {
    return { name: '', ty: value };
  }
  const raw = value as Record<string, unknown>;
  return {
    name: typeof raw.name === 'string' ? raw.name : '',
    ty: raw.ty,
  };
}

function normalizeSkillAccount(value: unknown): SkillAccount {
  const raw = value && typeof value === 'object' ? value as Record<string, unknown> : {};
  return {
    name: typeof raw.name === 'string' ? raw.name : '',
    isMut: typeof raw.isMut === 'boolean' ? raw.isMut : raw.is_mut === true,
    isSigner: typeof raw.isSigner === 'boolean' ? raw.isSigner : raw.is_signer === true,
  };
}

function normalizeSkillFromWire(value: unknown): Skill {
  const raw = value && typeof value === 'object' ? value as WireSkill : {};
  const skill: Skill = {
    skillId: typeof raw.skill_id === 'string' ? raw.skill_id : '',
    programId: typeof raw.program_id === 'string' ? raw.program_id : '',
    instructionName: typeof raw.instruction_name === 'string' ? raw.instruction_name : '',
    args: Array.isArray(raw.args) ? raw.args.map(normalizeSkillArg) : [],
    accounts: Array.isArray(raw.accounts) ? raw.accounts.map(normalizeSkillAccount) : [],
    paramsSchema: raw.params_schema,
  };
  if (typeof raw.program_name === 'string') skill.programName = raw.program_name;
  if (Array.isArray(raw.discriminator)) {
    skill.discriminator = raw.discriminator.filter((v): v is number => typeof v === 'number');
  }
  return skill;
}

export class AgentGeyserClient {
  private readonly proxyUrl: string;
  private readonly fetchImpl: FetchLike;
  private nextId = 1;

  constructor(options: AgentGeyserClientOptions) {
    if (!options || typeof options.proxyUrl !== 'string' || options.proxyUrl.length === 0) {
      throw new ValidationError('AgentGeyserClient: `proxyUrl` is required');
    }
    this.proxyUrl = options.proxyUrl;
    this.fetchImpl = resolveFetch(options.fetch);
  }

  /** Fetch the list of skills the proxy currently exposes (`ag_listSkills`). */
  async listSkills(): Promise<Skill[]> {
    const result = await this.rpc<unknown>('ag_listSkills', []);
    if (!Array.isArray(result)) {
      throw new ValidationError('ag_listSkills: expected array result', result);
    }
    return result.map(normalizeSkillFromWire);
  }

  /**
   * Build (but do NOT sign) a transaction for a given skill invocation
   * (`ag_invokeSkill`). Returns base64 transaction for downstream signing.
   */
  async invokeSkill(request: InvokeSkillRequest): Promise<InvokeSkillResponse> {
    this.validateInvokeRequest(request);
    let result = await this.rpc<InvokeSkillResponse>('ag_invokeSkill', request);
    // Live-proxy wire-contract drift: proxy emits snake_case `transaction_base64`.
    // Normalise to camelCase before validation so the public surface stays camelCase.
    if (result && typeof result === 'object') {
      const raw = result as unknown as Record<string, unknown>;
      if (typeof raw.transactionBase64 !== 'string' && typeof raw.transaction_base64 === 'string') {
        result = { ...raw, transactionBase64: raw.transaction_base64 } as InvokeSkillResponse;
      }
    }
    if (
      !result ||
      typeof result !== 'object' ||
      typeof (result as InvokeSkillResponse).transactionBase64 !== 'string'
    ) {
      throw new ValidationError(
        'ag_invokeSkill: expected { transactionBase64: string }',
        result,
      );
    }
    return result;
  }

  /**
   * Ask the proxy's NL planner to translate a natural-language prompt into
   * a structured {@link Plan} (`ag_planAction`). Server emits snake_case
   * `skill_id`; this method renames it to `skillId` for the public surface.
   */
  async planAction(input: PlanActionRequest): Promise<Plan> {
    if (!input || typeof input !== 'object') {
      throw new ValidationError('planAction: input must be an object');
    }
    if (typeof input.prompt !== 'string' || input.prompt.length === 0) {
      throw new ValidationError('planAction: `prompt` is required');
    }
    const params: { prompt: string; provider?: string } = { prompt: input.prompt };
    if (input.provider !== undefined) params.provider = input.provider;

    const raw = await this.rpc<{ skill_id?: unknown; args?: unknown; rationale?: unknown }>(
      'ag_planAction',
      params,
    );
    if (!raw || typeof raw !== 'object') {
      throw new ValidationError('ag_planAction: expected object result', raw);
    }
    const skillId = raw.skill_id;
    const args = raw.args;
    const rationale = raw.rationale;
    if (typeof skillId !== 'string' || skillId.length === 0) {
      throw new ValidationError('ag_planAction: missing `skill_id`', raw);
    }
    if (!args || typeof args !== 'object') {
      throw new ValidationError('ag_planAction: missing `args`', raw);
    }
    if (typeof rationale !== 'string') {
      throw new ValidationError('ag_planAction: missing `rationale`', raw);
    }
    return { skillId, args: args as Record<string, unknown>, rationale };
  }

  private validateInvokeRequest(request: InvokeSkillRequest): void {
    if (!request || typeof request !== 'object') {
      throw new ValidationError('invokeSkill: request must be an object');
    }
    if (typeof request.skill_id !== 'string' || request.skill_id.length === 0) {
      throw new ValidationError('invokeSkill: `skill_id` is required');
    }
    if (typeof request.payer !== 'string' || request.payer.length === 0) {
      throw new ValidationError('invokeSkill: `payer` is required');
    }
    if (!request.args || typeof request.args !== 'object') {
      throw new ValidationError('invokeSkill: `args` must be an object');
    }
    if (!request.accounts || typeof request.accounts !== 'object') {
      throw new ValidationError('invokeSkill: `accounts` must be an object');
    }
  }

  /** Issue a JSON-RPC 2.0 POST and decode the result or error. */
  private async rpc<TResult>(method: string, params: unknown): Promise<TResult> {
    const id = this.nextId++;
    const body = JSON.stringify({ jsonrpc: '2.0', id, method, params });

    let httpResponse;
    try {
      httpResponse = await this.fetchImpl(this.proxyUrl, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      throw new NetworkError(`fetch to ${this.proxyUrl} failed: ${msg}`);
    }

    if (!httpResponse.ok) {
      throw new NetworkError(
        `proxy returned HTTP ${httpResponse.status} ${httpResponse.statusText}`,
        httpResponse.status,
      );
    }

    let payload: unknown;
    try {
      payload = await httpResponse.json();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      throw new NetworkError(`failed to decode JSON body: ${msg}`, httpResponse.status);
    }

    const envelope = payload as JsonRpcResponse<TResult>;
    if (!envelope || typeof envelope !== 'object' || envelope.jsonrpc !== '2.0') {
      throw new ValidationError('malformed JSON-RPC envelope', payload);
    }

    if (envelope.error) {
      throw this.mapRpcError(method, envelope.error.code, envelope.error.message, envelope.error.data, params);
    }

    if (envelope.result === undefined) {
      throw new ValidationError('JSON-RPC response missing `result`', payload);
    }
    return envelope.result;
  }

  /** Narrow proxy error codes / messages to typed subclasses where possible. */
  private mapRpcError(
    method: string,
    code: number,
    message: string,
    data: unknown,
    params: unknown,
  ): Error {
    const msg = `${method}: ${message}`;
    // Heuristic: proxy signals missing skills via message text or code -32601.
    if (
      method === 'ag_invokeSkill' &&
      /skill.*(not\s+found|unknown)/i.test(message)
    ) {
      const skillId =
        params && typeof params === 'object' && 'skill_id' in (params as Record<string, unknown>)
          ? String((params as Record<string, unknown>).skill_id)
          : 'unknown';
      return new SkillNotFound(skillId);
    }
    if (code === -32602 || /invalid\s+params|validation/i.test(message)) {
      return new ValidationError(msg, data);
    }
    return new RpcError(msg, code, data);
  }
}
