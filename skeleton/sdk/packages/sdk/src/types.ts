/**
 * Public TypeScript types for `@agentgeyser/sdk`. Mirrors the JSON-RPC surface
 * of the AgentGeyser proxy (`ag_listSkills` / `ag_invokeSkill`). No `any`.
 */

/** Argument metadata for a skill returned by `ag_listSkills`. */
export interface SkillArg {
  name: string;
  ty: unknown;
}

/** Account metadata for a skill returned by `ag_listSkills`. */
export interface SkillAccount {
  name: string;
  isMut: boolean;
  isSigner: boolean;
}

/** A skill descriptor returned by `ag_listSkills` (camelCase SDK surface). */
export interface Skill {
  skillId: string;
  programId: string;
  programName?: string;
  instructionName: string;
  args: SkillArg[];
  accounts: SkillAccount[];
  paramsSchema: unknown;
  discriminator?: number[];
}

/** Request body for `ag_invokeSkill`. */
export interface InvokeSkillRequest {
  skill_id: string;
  args: Readonly<Record<string, unknown>>;
  accounts: Readonly<Record<string, unknown>>;
  payer: string;
}

/** Response body for `ag_invokeSkill`. */
export interface InvokeSkillResponse {
  transactionBase64: string;
}

/** LLM provider selector accepted by `ag_planAction`. */
export type PlanProvider = 'openai' | 'mock' | 'kimi-coding' | 'anthropic' | 'auto';

/** Request body for `ag_planAction`. */
export interface PlanActionRequest {
  prompt: string;
  provider?: PlanProvider;
}

/** Structured plan returned by `ag_planAction` (camelCase). */
export interface Plan {
  skillId: string;
  args: Record<string, unknown>;
  rationale: string;
}

/** Minimal fetch contract we rely on (browser, Node 20+, cross-fetch). */
export type FetchLike = (
  input: string | URL,
  init?: {
    method?: string;
    headers?: Record<string, string>;
    body?: string;
    signal?: AbortSignal;
  },
) => Promise<{
  ok: boolean;
  status: number;
  statusText: string;
  text(): Promise<string>;
  json(): Promise<unknown>;
}>;

/** Options accepted by the {@link AgentGeyserClient} constructor. */
export interface AgentGeyserClientOptions {
  proxyUrl: string;
  fetch?: FetchLike;
}

/** @internal JSON-RPC 2.0 error object. */
export interface JsonRpcErrorObject {
  code: number;
  message: string;
  data?: unknown;
}

/** @internal JSON-RPC 2.0 response envelope. */
export interface JsonRpcResponse<TResult> {
  jsonrpc: '2.0';
  id: number;
  result?: TResult;
  error?: JsonRpcErrorObject;
}
