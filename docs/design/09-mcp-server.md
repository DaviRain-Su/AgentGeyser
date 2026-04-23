---
doc: F9
title: MCP Server 设计（Tools / Resources / Prompts, Auth, Rate Limits）
owner: AgentGeyser Core
status: draft
depends-on: [F3, F4, F6, F8]
updated: 2026-04-23
---

## Goals

- 定义 `McpServer` 对 MCP 客户端暴露的能力面：`tools`、`resources`、`prompts`。
- 规定 MCP 能力与 AgentGeyser canonical `ag_*` 方法的映射关系，避免接口歧义。
- 给出面向 MCP 客户端的认证、授权和速率限制模型，确保多租户下可控、安全、可计费。
- 定义错误模型与观测字段，支持 agent runtime 的自动重试与故障隔离。

## Non-Goals

- 不在本文实现完整 MCP SDK 代码或生产部署参数（部署细节见后续运维文档）。
- 不替代 JSON-RPC 主接口；MCP 是 Agent/IDE 友好的补充协议层。
- 不扩展新的核心 JSON-RPC 方法名；仅复用 `ag_listSkills`、`ag_invokeSkill`、`ag_planNL`、`ag_getIdl`。
- 不定义数据库 DDL；仅说明 `AuthQuota` 与审计实体交互契约。

## Context

本设计承接：

- [F3 Architecture](./03-architecture.md)：确认 `McpServer` 位于接入面，并依赖 `AuthQuota`、`SkillSynthesizer`、`NlPlanner`、`RpcPassthrough`。
- [F4 Modules](./04-modules.md)：定义 `McpServer` 责任边界与 trait/interface 契约。
- [F6 Skill Synthesizer](./06-skill-synthesizer.md)：提供 skill schema 与语义标签，驱动 tool 自动发现。
- [F8 SDK](./08-sdk.md)：确保 API 方法命名跨 SDK/MCP 一致（X.4）。

MCP 面向典型客户端：

1. Claude Desktop / Cursor 等开发工具中的 MCP Host。
2. 服务器侧 agent runtime（多并发工具调用、自动规划）。
3. 需要受限执行面的企业集成代理（只开放白名单资源与 prompt）。

## Design

### 1) MCP Transport 与会话生命周期

`McpServer` 支持 MCP 标准 transport（stdio 与 streamable HTTP，两者由部署配置选择）。无论 transport 如何，能力语义保持一致：

1. `initialize`：协商协议版本、server capabilities、身份上下文来源。
2. `tools/list`、`resources/list`、`prompts/list`：能力发现。
3. `tools/call`、`resources/read`、`prompts/get`：能力执行。
4. `notifications/cancelled` 或连接关闭：结束会话并释放配额上下文。

会话上下文最小字段：

```json
{
  "sessionId": "mcp_sess_01J...",
  "tenantId": "tenant_acme",
  "authSubject": "api_key:ak_live_***",
  "scopes": ["skills:read", "skills:invoke", "nl:plan", "idl:read"],
  "ratePlan": "pro",
  "traceId": "trace_..."
}
```

### 2) Tools 枚举与方法映射（C.F9.2）

MCP `tools/list` 输出来自 skill catalog + 固定平台工具。最小工具集：

| MCP Tool Name | 输入 | 后端映射 | 说明 |
|---|---|---|---|
| `ag_list_skills` | `programId?`, `tags?`, `selector?` | `ag_listSkills` | 列出技能与 schema 元信息 |
| `ag_invoke_skill` | `programId`, `skillName`, `input`, `skillVersion?` | `ag_invokeSkill` | 执行指定技能 |
| `ag_plan_nl` | `utterance`, `constraints?` | `ag_planNL` | 自然语言规划，不直接广播交易 |
| `ag_get_idl` | `programId`, `version?` | `ag_getIdl` | 获取 IDL 与版本信息 |

此外可将 `SkillSynthesizer` 产出的业务技能投影为“别名工具”（例如 `swap_exact_in`），但必须在 metadata 中声明 canonical 回源：

```json
{
  "name": "swap_exact_in",
  "description": "Alias for ag_invoke_skill(program=..., skill=swapExactIn)",
  "annotations": {
    "canonicalMethod": "ag_invokeSkill",
    "programId": "So11111111111111111111111111111111111111112",
    "skillName": "swapExactIn",
    "skillVersion": "2.3.1"
  }
}
```

#### Tool 调用处理流程

1. `tools/call` 请求到达，校验 JSON schema 与 required scopes。
2. 若为平台工具，直接映射到对应 `ag_*` 方法。
3. 若为别名工具，先解引用到 canonical `ag_invokeSkill` 参数。
4. 执行后统一封装 MCP `content[]`，包含结构化 JSON + 可读摘要。

### 3) Resources 暴露模型（C.F9.2）

`resources/list` 提供只读信息资源，便于 agent 在调用前做上下文检索。推荐 URI 约定：

- `ag://skills/catalog`：全局技能目录快照（可分页）。
- `ag://skills/{programId}`：特定 Program 的技能与版本。
- `ag://idl/{programId}?version=x`：IDL 文本或规范化 JSON。
- `ag://policies/quota`：当前租户配额、限流窗口、剩余额度。
- `ag://audit/recent?limit=50`：最近调用审计摘要（脱敏）。

`resources/read` 对应内部查询：

- skills 类资源 -> `ag_listSkills`
- idl 类资源 -> `ag_getIdl`
- quota/audit 类资源 -> `AuthQuota` 与审计存储读模型（只读）

资源返回示例：

```json
{
  "uri": "ag://skills/So11111111111111111111111111111111111111112",
  "mimeType": "application/json",
  "text": "{\"programId\":\"So...\",\"skills\":[{\"name\":\"swapExactIn\",\"version\":\"2.3.1\"}]}"
}
```

### 4) Prompts 库设计（C.F9.2）

`prompts/list` 暴露可参数化模板，帮助 MCP Host 以安全、稳定方式调用 planner/invoker。基础 prompts：

1. `plan_trade_intent`
   - 参数：`goal`, `riskProfile`, `maxFeeLamports`, `allowedPrograms[]`
   - 输出：结构化 planning prompt（供 `ag_plan_nl`）
2. `invoke_skill_safely`
   - 参数：`programId`, `skillName`, `inputJson`, `requireSimulation`
   - 输出：强制含“先模拟后执行”的操作模板
3. `explain_skill_schema`
   - 参数：`programId`, `skillName`, `version?`
   - 输出：对 schema 字段的人类可读解释模板

`prompts/get` 返回包含变量声明与安全约束的对象：

```json
{
  "name": "invoke_skill_safely",
  "arguments": {
    "programId": "So11111111111111111111111111111111111111112",
    "skillName": "swapExactIn",
    "inputJson": "{\"inMint\":\"...\",\"outMint\":\"...\",\"amountIn\":\"1000000\"}",
    "requireSimulation": true
  },
  "messages": [
    {
      "role": "system",
      "content": "Never bypass simulation for state-changing actions unless explicitly overridden by policy."
    },
    {
      "role": "user",
      "content": "Invoke skill with validated schema and return traceId + risk flags."
    }
  ]
}
```

### 5) AuthN/AuthZ 模型（C.F9.3）

MCP 客户端认证统一由 `AuthQuota` 完成，`McpServer` 只负责提取凭据与转发判定。

#### 5.1 凭据来源

- HTTP transport：`Authorization: Bearer <token>` 或 `x-api-key`。
- stdio transport：启动参数注入短期 token（不落盘）或环境变量引用（由 host 管理）。
- 可选 mTLS：企业网关前置校验后透传 `x-verified-subject`。

#### 5.2 授权粒度

建议 scope 最小集合：

- `skills:read`（tools/resources 列表与 read）
- `skills:invoke`（`ag_invoke_skill`/别名工具）
- `nl:plan`（`ag_plan_nl`）
- `idl:read`（IDL 资源）
- `audit:read`（审计摘要资源，默认关闭）

授权策略：

1. 未认证 -> `401`（MCP error code `AUTH_REQUIRED`）。
2. 已认证但 scope 不足 -> `403`（`SCOPE_DENIED`）。
3. 账户禁用/密钥撤销 -> `403`（`SUBJECT_REVOKED`）。

### 6) Rate Limiting 与配额计量（C.F9.3）

MCP 层执行“双桶模型”：**QPS 限流桶 + 成本配额桶**。

#### 6.1 限流维度

- 主键：`tenantId + authSubject + methodClass`
- methodClass：
  - `read`（tools/list, resources/read, ag_get_idl）
  - `invoke`（ag_invoke_skill）
  - `plan`（ag_plan_nl，成本最高）

#### 6.2 默认策略（示例）

| Plan | read QPS | invoke QPS | plan QPS | monthly cost units |
|---|---:|---:|---:|---:|
| free | 5 | 2 | 0.5 | 100,000 |
| pro | 30 | 10 | 3 | 5,000,000 |
| enterprise | custom | custom | custom | contract |

每次调用按 `units` 扣减（示例）：`read=1`, `invoke=5`, `plan=20`。  
当月余额不足返回 `429`（`QUOTA_EXCEEDED`），并附带 reset 时间。

#### 6.3 返回头与错误体

无论成功失败，MCP response metadata 应包含：

- `x-ag-rate-limit-limit`
- `x-ag-rate-limit-remaining`
- `x-ag-rate-limit-reset`
- `x-ag-quota-remaining-units`
- `x-ag-trace-id`

错误体（MCP content JSON）示例：

```json
{
  "error": {
    "code": "RATE_LIMITED",
    "httpStatus": 429,
    "message": "Rate limit exceeded for methodClass=plan",
    "retryAfterMs": 1200,
    "traceId": "trace_01J..."
  }
}
```

### 7) 可观测性与审计

`McpServer` 每次请求至少记录：

- `sessionId`, `tenantId`, `toolName/resourceUri/promptName`
- `canonicalMethod`（若有）
- `latencyMs`, `status`, `rateLimited`, `quotaUnitsCharged`
- `traceId`, `requestId`

审计落地到 `Invocation` / `AuditLog` 读写模型（实体定义见后续数据模型文档），确保：

1. 工具别名调用可追溯到 canonical `ag_*` 方法。
2. 配额扣费与调用结果可对账。
3. 失败调用（401/403/429/5xx）也有完整追踪。

## Key Decisions & Alternatives

| Decision | Chosen | Alternatives | Trade-offs |
|---|---|---|---|
| MCP 能力暴露 | tools + resources + prompts 全量支持 | 仅 tools | 全量更易与现代 agent 集成，但需要更细粒度鉴权 |
| 工具命名策略 | canonical `ag_*` + 可选别名 | 仅别名业务工具 | canonical 更稳定可审计；别名增强易用性但引入映射复杂度 |
| 认证入口 | 全部汇聚 `AuthQuota` | `McpServer` 内联鉴权 | 汇聚可统一策略与计费；但增加一次模块调用 |
| 限流模型 | QPS 桶 + 成本桶双轨 | 单纯 QPS | 双轨更符合 LLM/链上成本现实，但实现复杂度更高 |
| 错误回传 | MCP payload 内嵌结构化错误 + 速率头 | 仅文本错误 | 结构化更利于 agent 自动恢复，代价是规范维护成本 |

## Risks & Open Questions

- **Prompt 注入风险**：恶意上游上下文可能影响 prompt 模板拼接。  
  - Owner: Security Team（需在 F13 细化 prompt sanitization）。
- **Host 兼容性差异**：不同 MCP Host 对 metadata/header 呈现能力不同。  
  - Owner: MCP Integration Team（需要兼容矩阵测试）。
- **stdio 凭据管理**：本地桌面环境中 token 注入方式存在泄露风险。  
  - Owner: Platform + Security（建议短期 token + 最小 scope）。
- **Alias 工具爆炸**：Program 数量增长时工具列表可能过大。  
  - Owner: SkillSynthesizer Team（需分页/标签过滤策略）。

## References

- [Model Context Protocol Specification](https://modelcontextprotocol.io/specification)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [F3 Architecture](./03-architecture.md)
- [F4 Modules](./04-modules.md)
- [F6 Skill Synthesizer](./06-skill-synthesizer.md)
- [F8 SDK](./08-sdk.md)

<!--
assertion-evidence:
  C.F9.1: file exists at docs/design/09-mcp-server.md with required frontmatter and standard sections.
  C.F9.2: Design sections "2) Tools 枚举与方法映射", "3) Resources 暴露模型", and "4) Prompts 库设计" enumerate MCP tools/resources/prompts with examples.
  C.F9.3: Design sections "5) AuthN/AuthZ 模型" and "6) Rate Limiting 与配额计量" define authentication, authorization scopes, rate limits, quotas, and error semantics for MCP clients.
-->
