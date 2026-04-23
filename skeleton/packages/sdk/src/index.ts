/**
 * AgentGeyser dynamic TypeScript SDK.
 *
 * `AgentGeyserClient` uses a `Proxy` so callers can write
 * `client.<program_name>.<instruction_name>(params)` and have it resolve
 * at runtime against the proxy's `ag_listSkills` catalog.
 *
 * The SDK NEVER signs transactions. `invokeSkill` returns the unsigned
 * `transaction_base64` returned by the proxy; the caller is responsible
 * for signing with their wallet before submission.
 */

export interface Skill {
  skill_id: string;
  program_id: string;
  program_name?: string;
  instruction_name: string;
  params_schema: unknown;
}

export interface InvokeResult {
  skill_id: string;
  transaction_base64: string;
}

export class UnknownProgramError extends Error {
  constructor(name: string) {
    super(`AgentGeyser: unknown program "${name}"`);
    this.name = 'UnknownProgramError';
  }
}

export class UnknownSkillError extends Error {
  constructor(program: string, instruction: string) {
    super(`AgentGeyser: unknown instruction "${instruction}" on program "${program}"`);
    this.name = 'UnknownSkillError';
  }
}

export interface AgentGeyserClientOptions {
  endpoint: string;
  fetchImpl?: typeof fetch;
}

type FetchFn = typeof fetch;

interface Catalog {
  byProgram: Map<string, Map<string, Skill>>;
  raw: Skill[];
}

type InstructionDispatcher = (params?: Record<string, unknown>) => Promise<InvokeResult>;
type ProgramDispatcher = Record<string, InstructionDispatcher>;

export type AgentGeyserClientProxy = AgentGeyserClient & Record<string, ProgramDispatcher>;

export class AgentGeyserClient {
  readonly endpoint: string;
  private readonly fetchImpl: FetchFn;
  private catalog: Catalog | null = null;
  private catalogPromise: Promise<Catalog> | null = null;
  private rpcIdCounter = 0;

  constructor(opts: AgentGeyserClientOptions) {
    this.endpoint = opts.endpoint;
    this.fetchImpl = opts.fetchImpl ?? globalThis.fetch.bind(globalThis);
  }

  /**
   * Wrap an instance in a `Proxy` for dynamic dispatch. Use this at construction:
   *
   *   const client = AgentGeyserClient.create({ endpoint });
   *   await client.hello_world.greet({ name: 'world' });
   */
  static create(opts: AgentGeyserClientOptions): AgentGeyserClientProxy {
    const instance = new AgentGeyserClient(opts);
    return new Proxy(instance, {
      get(target, prop, receiver) {
        if (typeof prop === 'symbol') return Reflect.get(target, prop, receiver);
        if (prop in target) return Reflect.get(target, prop, receiver);
        // Dynamic path: treat prop as a program name.
        return target._programProxy(prop as string);
      },
    }) as AgentGeyserClientProxy;
  }

  /** Fetch and cache the catalog; subsequent calls reuse it. */
  async listSkills(): Promise<Skill[]> {
    const catalog = await this.loadCatalog();
    return catalog.raw;
  }

  /** Force-refresh the catalog. */
  async refreshSkills(): Promise<Skill[]> {
    this.catalog = null;
    this.catalogPromise = null;
    return this.listSkills();
  }

  /** Low-level invoke used by dynamic dispatch and callers who already know the skill id. */
  async invokeSkill(skillId: string, params: Record<string, unknown> = {}): Promise<InvokeResult> {
    const resp = await this.rpc<InvokeResult>('ag_invokeSkill', { skill_id: skillId, params });
    return resp;
  }

  private _programProxy(programName: string): ProgramDispatcher {
    return new Proxy({} as ProgramDispatcher, {
      get: (_target, instructionProp) => {
        if (typeof instructionProp === 'symbol') return undefined;
        const instructionName = instructionProp as string;
        const dispatcher: InstructionDispatcher = async (params = {}) => {
          const catalog = await this.loadCatalog();
          const program = catalog.byProgram.get(programName);
          if (!program) throw new UnknownProgramError(programName);
          const skill = program.get(instructionName);
          if (!skill) throw new UnknownSkillError(programName, instructionName);
          return this.invokeSkill(skill.skill_id, params);
        };
        return dispatcher;
      },
    });
  }

  private async loadCatalog(): Promise<Catalog> {
    if (this.catalog) return this.catalog;
    if (!this.catalogPromise) {
      this.catalogPromise = this.rpc<Skill[]>('ag_listSkills', {}).then((skills) => {
        const byProgram = new Map<string, Map<string, Skill>>();
        for (const skill of skills) {
          const programKey = skill.program_name ?? skill.program_id;
          let bucket = byProgram.get(programKey);
          if (!bucket) {
            bucket = new Map();
            byProgram.set(programKey, bucket);
          }
          bucket.set(skill.instruction_name, skill);
        }
        const catalog: Catalog = { byProgram, raw: skills };
        this.catalog = catalog;
        return catalog;
      });
    }
    return this.catalogPromise;
  }

  private async rpc<T>(method: string, params: unknown): Promise<T> {
    const id = ++this.rpcIdCounter;
    const response = await this.fetchImpl(this.endpoint, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', id, method, params }),
    });
    if (!response.ok) {
      throw new Error(`AgentGeyser RPC HTTP ${response.status}`);
    }
    const body = (await response.json()) as { result?: T; error?: { code: number; message: string } };
    if (body.error) {
      throw new Error(`AgentGeyser RPC error ${body.error.code}: ${body.error.message}`);
    }
    if (body.result === undefined) {
      throw new Error('AgentGeyser RPC: missing result');
    }
    return body.result;
  }
}
