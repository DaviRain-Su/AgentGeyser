/**
 * Error hierarchy for `@agentgeyser/sdk`. All errors extend
 * {@link AgentGeyserError} and carry a short string `code` for ergonomic
 * `catch` narrowing. JSON-RPC errors preserve the numeric `rpcCode`.
 */

/** Base class — every SDK-emitted error is an instance of this. */
export class AgentGeyserError extends Error {
  public readonly code: string;

  constructor(message: string, code: string) {
    super(message);
    this.name = 'AgentGeyserError';
    this.code = code;
    // Preserve prototype chain across transpilation targets.
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/** JSON-RPC 2.0 error returned by the proxy. `rpcCode` preserves the numeric code. */
export class RpcError extends AgentGeyserError {
  public readonly rpcCode: number;
  public readonly data: unknown;

  constructor(message: string, rpcCode: number, data?: unknown) {
    super(message, 'rpc_error');
    this.name = 'RpcError';
    this.rpcCode = rpcCode;
    this.data = data;
  }
}

/** Transport-level failure — network unreachable, non-2xx HTTP, malformed body. */
export class NetworkError extends AgentGeyserError {
  public readonly status?: number;

  constructor(message: string, status?: number) {
    super(message, 'network_error');
    this.name = 'NetworkError';
    this.status = status;
  }
}

/** The proxy reports that the requested `skill_id` does not exist. */
export class SkillNotFound extends AgentGeyserError {
  public readonly skillId: string;

  constructor(skillId: string) {
    super(`Skill not found: ${skillId}`, 'skill_not_found');
    this.name = 'SkillNotFound';
    this.skillId = skillId;
  }
}

/** Client- or proxy-side validation failure — malformed request or schema mismatch. */
export class ValidationError extends AgentGeyserError {
  public readonly details: unknown;

  constructor(message: string, details?: unknown) {
    super(message, 'validation_error');
    this.name = 'ValidationError';
    this.details = details;
  }
}
