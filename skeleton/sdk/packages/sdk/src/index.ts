/**
 * Public entry point for `@agentgeyser/sdk`.
 * Only the symbols re-exported here are considered public API.
 */

declare const process: { env?: { AGENTGEYSER_PROXY_PORT?: string | undefined } } | undefined;

export function defaultProxyUrl(): string {
  const port = typeof process === 'undefined'
    ? '8999'
    : process.env?.AGENTGEYSER_PROXY_PORT ?? '8999';
  return `http://127.0.0.1:${port}`;
}

export { AgentGeyserClient } from './client.js';
export { AgentGeyserError, RpcError, NetworkError, SkillNotFound, ValidationError } from './errors.js';
export { signAndSend, SignAndSendError } from './signAndSend.js';
export type {
  Connection,
  Signer,
  UnsignedTxPayload,
} from './signAndSend.js';
export type {
  AgentGeyserClientOptions,
  FetchLike,
  InvokeSkillRequest,
  InvokeSkillResponse,
  Plan,
  PlanActionRequest,
  PlanProvider,
  Skill,
  SkillAccount,
  SkillArg,
} from './types.js';
