---
doc: F8
title: Dynamic TypeScript SDK 设计（Proxy Dispatch / Ambient Typings / Version Pinning / Offline Fallback）
owner: AgentGeyser Core
status: draft
depends-on: [F3, F4, F5, F6, F7]
updated: 2026-04-23
---

## Goals

- 定义 `packages/sdk` 的运行时动态调用模型，让客户端无需为每个 Program 手写 SDK。
- 说明基于 JavaScript `Proxy` 的方法分发策略，并给出可执行 TypeScript 样例。
- 定义 ambient `.d.ts` 流式下发机制，让动态方法在 IDE 中获得类型提示与签名校验。
- 定义版本钉选（version pinning）机制，确保线上/回测/审计场景可复现实验结果。
- 定义离线 fallback 策略，在控制面不可达或网络受限时仍可执行已缓存技能。

## Non-Goals

- 不在本设计中实现完整 SDK 代码，只定义接口、协议和行为。
- 不替代底层 Solana 官方客户端（`@solana/web3.js`）；本 SDK 专注于 AgentGeyser 智能层。
- 不引入私钥托管能力；签名仍由调用侧钱包/签名器完成。
- 不在本设计中扩展新的 JSON-RPC 方法名，沿用 `ag_listSkills` / `ag_invokeSkill` / `ag_getIdl` / `ag_planNL`。

## Context

本设计承接以下文档：

- [F3 Architecture](./03-architecture.md)：定义三层架构和数据流。
- [F4 Modules](./04-modules.md)：定义 `RpcPassthrough`、`SkillSynthesizer`、`AuthQuota` 的边界。
- [F5 IDL Registry](./05-idl-registry.md)：提供 `Idl` / `SkillVersion` 的版本化来源。
- [F6 Skill Synthesizer](./06-skill-synthesizer.md)：定义 `Skill` 参数 schema 与语义标签。
- [F7 NL Planner](./07-nl-planner.md)：定义 `Invocation` 和审计上下文，SDK 需透传相关标识。

SDK 对外目标用户包括：

1. AI Agent Runtime（需要在运行期发现并调用新技能）。
2. 策略交易服务（需要严格版本钉选以保证回放一致性）。
3. dApp 前端（需要在 IDE 中获得动态能力的类型提示）。

## Design

### 1) SDK 形态与入口

SDK 暴露一个轻量客户端构造器：

```ts
export interface AgentGeyserClientOptions {
  endpoint: string;                 // AgentGeyser JSON-RPC endpoint
  apiKey?: string;                  // AuthQuota 凭据
  cluster?: "mainnet-beta" | "devnet" | "testnet";
  sdkVersion?: string;              // 当前 SDK 包版本
  skillSet?: SkillSetSelector;      // 版本钉选选择器
  offline?: OfflineModeOptions;     // 离线策略
}
```

构造后返回两类表面：

- `client.program("<programId>")`：按 Program 维度动态分发方法。
- `client.skills`：全局技能空间（按 canonical skill 名调用）。

### 2) Proxy-based Dynamic Method Dispatch（C.F8.2）

#### 2.1 分发核心思路

`program()` 返回一个 `Proxy` 对象。任意属性访问（如 `sdk.swapExactIn`）会触发：

1. 读取本地内存索引（`Map<methodName, SkillDescriptor>`）。
2. 未命中则按 `skillSet` 拉取远程 manifest（`ag_listSkills`）。
3. 命中后返回一个 async invoker，最终调用 `ag_invokeSkill`。
4. 将 invocation 元数据（`traceId`、`skillVersion`、`programId`）写入请求头/上下文。

#### 2.2 TypeScript 代码样例

```ts
type Json = null | boolean | number | string | Json[] | { [k: string]: Json };

interface SkillDescriptor {
  name: string;
  programId: string;
  version: string; // SkillVersion semantic version
  inputSchema: Json;
}

interface RpcTransport {
  call<T = unknown>(method: string, params: Json): Promise<T>;
}

class ProgramClient {
  constructor(
    private readonly transport: RpcTransport,
    private readonly programId: string,
    private readonly registry: Map<string, SkillDescriptor>,
    private readonly pin: SkillSetSelector
  ) {}

  asProxy<T extends object = Record<string, unknown>>(): T {
    return new Proxy({} as T, {
      get: (_target, prop) => {
        if (typeof prop !== "string") return undefined;
        return async (input: Json, ctx?: { traceId?: string }) => {
          const skill = await this.resolveSkill(prop);
          return this.transport.call("ag_invokeSkill", {
            programId: this.programId,
            skillName: skill.name,
            skillVersion: skill.version,
            input,
            traceId: ctx?.traceId ?? null
          });
        };
      }
    });
  }

  private async resolveSkill(name: string): Promise<SkillDescriptor> {
    const key = `${this.programId}:${name}:${this.pin.cacheKey}`;
    const cached = this.registry.get(key);
    if (cached) return cached;
    const manifest = await this.transport.call<{ skills: SkillDescriptor[] }>(
      "ag_listSkills",
      {
        programId: this.programId,
        selector: this.pin
      }
    );
    for (const s of manifest.skills) {
      this.registry.set(`${s.programId}:${s.name}:${this.pin.cacheKey}`, s);
    }
    const resolved = this.registry.get(key);
    if (!resolved) throw new Error(`Skill not found: ${name}`);
    return resolved;
  }
}
```

该模式保证：

- 方法名可随链上 Program 演进动态出现/消失。
- 调用请求携带 `SkillVersion`，避免“最新版本漂移”引发的不确定行为。
- 与 `RpcPassthrough` 解耦：SDK 不关心底层交易细节，只关心能力发现与调用协议。

### 3) Ambient `.d.ts` Streaming（C.F8.3）

动态 Proxy 天生缺少静态类型。为此，SDK 采用 **ambient declaration streaming**：

#### 3.1 交付模型

- 控制面按 `programId + skillSet selector` 生成 `d.ts` 片段。
- SDK 在首次 `program(programId)` 时请求 `ag_getIdl` + `ag_listSkills`，拼装为 `.d.ts` 文本。
- 文本写入本地缓存目录（例如 `~/.agentgeyser/typings/<cacheKey>.d.ts`）。
- 在 Node/TS 环境中，通过 `typeRoots` 或 `/// <reference path>` 注入，形成 IDE 可见的 ambient types。

#### 3.2 流式更新

- 通过 SSE/WebSocket（文档层定义，具体传输在实现阶段确定）推送 `SkillVersion` 变更事件。
- 客户端对比 `etag` / `manifestRevision`，仅增量拉取变化的 declaration block。
- 若 IDE 会话不支持热更新，则在下次 `tsserver` reload 时生效。

#### 3.3 版本关联

每个生成片段必须包含版本戳，示例：

```ts
declare namespace AG.Program["So11111111111111111111111111111111111111112"] {
  /** skill: swapExactIn, version: 2.3.1 */
  export function swapExactIn(input: {
    inMint: string;
    outMint: string;
    amountIn: string;
    slippageBps?: number;
  }): Promise<{ signature: string; slot: number }>;
}
```

这使类型签名与 `SkillVersion` 一一对应，避免“运行时代码是 v2、类型提示还停在 v1”。

### 4) Version Pinning Strategy（C.F8.3）

SDK 必须允许调用方显式声明“我希望看到哪一个技能集合”。定义如下：

```ts
export type SkillSetSelector =
  | { mode: "latest"; cacheKey: "latest" }
  | { mode: "timestamp"; at: string; cacheKey: string }      // ISO8601
  | { mode: "manifest"; manifestId: string; cacheKey: string }
  | { mode: "range"; semver: string; cacheKey: string };     // e.g. ^2.1.0
```

#### Pinning 规则

1. **default = latest**：开发态便捷优先。
2. **生产建议 manifest pin**：发布时固化 `manifestId`，保证重放一致。
3. **审计/回测使用 timestamp pin**：按历史时点恢复当时可见技能。
4. **range pin 仅用于灰度**：允许运营限制在某个 semver 区间内自动升级。

#### 冲突处理

- 若 selector 指向的技能不存在，SDK 返回确定性错误 `ERR_SKILLSET_UNRESOLVABLE`。
- 若 selector 合法但本地 declaration 过期，先调用再后台刷新 typings（调用正确性优先于 IDE 舒适度）。

### 5) Offline Fallback（C.F8.4）

#### 5.1 目标

当 AgentGeyser 控制面暂时不可达时，SDK 仍能在受控范围内运行“最后一次已验证”的技能定义，避免关键路径完全中断。

#### 5.2 离线缓存层次

1. **L1 Memory**：当前进程 manifest + descriptor。
2. **L2 Disk**：签名后的 manifest 快照（含 hash、生成时间、selector）。
3. **L3 Optional Bundle**：发布物内嵌静态快照（用于 air-gapped 环境）。

```ts
export interface OfflineModeOptions {
  enabled: boolean;
  maxStalenessMs: number; // 超过即拒绝调用
  allowReadOnlyOnly?: boolean; // 离线时仅允许无资金移动/只读技能
  snapshotPath?: string;
}
```

#### 5.3 离线判定与降级流程

1. 远程调用失败（连接错误/超时/5xx）进入 fallback 判定。
2. 若存在未过期快照，加载快照并继续本地分发。
3. 若 `allowReadOnlyOnly = true`，根据 `Skill` 副作用标签过滤高风险方法。
4. 若快照过期或签名校验失败，返回 `ERR_OFFLINE_SNAPSHOT_INVALID`，拒绝执行。

#### 5.4 安全约束

- 离线快照必须含 `manifestDigest` 与签名（由服务端发行密钥签发；实现细节见后续安全文档）。
- SDK 不在离线模式“猜测”新技能；只执行已知、已签名、未过期技能。
- 涉及资产转移的高风险技能在离线模式默认拒绝，需业务侧显式放开。

### 6) JSON-RPC Contract Touchpoints

SDK 与服务端交互仅依赖 canonical `ag_*` 方法：

- `ag_listSkills`：按 Program + selector 获取技能清单。
- `ag_invokeSkill`：执行技能，返回计划/交易结果。
- `ag_getIdl`：获取 IDL / schema 元数据，用于 typings 生成。
- `ag_planNL`：可选，供 SDK 暴露 `client.plan(prompt)` 的高级入口。

> 方法名与参数完整 schema 以 [F10 API](./10-api.md) 为准，本文件只约束 SDK 使用方式。

## Key Decisions & Alternatives

| Decision | Chosen | Alternatives | Trade-offs |
|---|---|---|---|
| 动态方法分发机制 | JS `Proxy` | 代码生成静态客户端 | Proxy 灵活但类型天然弱，需要额外 typings 补偿 |
| 类型提供方式 | Ambient `.d.ts` 流式下发 | 纯 JSDoc / 本地一次性 codegen | 流式可跟随链上变化，但实现复杂度更高 |
| 版本控制策略 | Selector + 显式 pin | 永远 latest | pin 提高可重复性，但要求调用方管理版本策略 |
| 离线策略 | 签名快照 + 过期时间 | 完全禁用离线 | 可用性更高，但需额外签名与风控逻辑 |
| 调用正确性 vs IDE 新鲜度 | 调用优先、类型后台刷新 | 强制类型先更新再调用 | 前者保证业务连续性，后者体验一致但可能阻塞交易路径 |

## Risks & Open Questions

- **Typings 注入兼容性**：不同工具链（ts-node / vite / bun / deno）对 runtime typeRoots 注入支持不同。  
  - Owner: SDK Team
- **SSE/WebSocket 选择**：声明流传输协议未最终定案，需结合网关与企业代理兼容性决定。  
  - Owner: Platform Team
- **离线签名密钥轮换**：manifest 签名链路在密钥轮换时如何平滑过渡需在 F13 进一步细化。  
  - Owner: Security Team
- **大规模 Program 数量下的类型体积**：多 Program 并行时 `.d.ts` 可能非常大，需增量裁剪策略。  
  - Owner: SDK + Registry

## References

- [ECMAScript Proxy](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Proxy)
- [TypeScript Declaration Files Handbook](https://www.typescriptlang.org/docs/handbook/declaration-files/introduction.html)
- [Semantic Versioning 2.0.0](https://semver.org/)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)

<!--
assertion-evidence:
  C.F8.1: file exists with required frontmatter and standard sections (Goals/Non-Goals/Context/Design/Key Decisions & Alternatives/Risks & Open Questions/References)
  C.F8.2: section "2) Proxy-based Dynamic Method Dispatch" plus TypeScript code sample (ProgramClient + Proxy get trap)
  C.F8.3: section "3) Ambient .d.ts Streaming" and "4) Version Pinning Strategy" including selector type and pinning rules
  C.F8.4: section "5) Offline Fallback" including cache tiers, options interface, degradation flow, and safety constraints
-->
