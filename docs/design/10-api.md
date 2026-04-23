---
doc: F10
title: 对外 API 规范（JSON-RPC ag_* / REST 管理面 / MCP Endpoint）
owner: AgentGeyser Core
status: draft
depends-on: [F4, F8, F9]
updated: 2026-04-23
---

## Goals

- 定义 AgentGeyser 对外 API 契约，覆盖 JSON-RPC、REST 管理面、MCP endpoint 三个入口。
- 规范 canonical JSON-RPC 方法：`ag_listSkills`、`ag_invokeSkill`、`ag_planNL`、`ag_getIdl`。
- 为每个 JSON-RPC 方法提供 request/response JSON Schema（machine-readable）。
- 给出 OpenAPI 片段，覆盖运维/管理所需 REST surface。
- 明确 MCP endpoint 路径、握手过程与认证前置条件，保证与 F9 一致。

## Non-Goals

- 不实现 API 服务器代码，仅定义接口与行为。
- 不定义完整数据库结构（见 F11 Data Model）。
- 不扩展 canonical `ag_*` 名称之外的新核心方法。
- 不在本文件细化部署拓扑与可观测性指标（见 F14）。

## Context

本设计对齐以下上游文档：

- [F4 Modules](./04-modules.md)：`RpcPassthrough` / `AuthQuota` / `McpServer` 责任边界。
- [F8 SDK](./08-sdk.md)：SDK 依赖本文件定义的 `ag_*` 合约与 schema。
- [F9 MCP Server](./09-mcp-server.md)：MCP 工具映射到本文件中的 canonical JSON-RPC 方法。

调用方分层：

1. **应用/Agent 客户端**：主要通过 JSON-RPC 调用 `ag_*`。
2. **平台运维与租户管理端**：通过 REST 管理 API 查询健康、schema、配额。
3. **MCP Host（Claude/Cursor/自建代理）**：通过 MCP endpoint 进行握手与工具调用。

## Design

### 1) API Surface Overview

| Surface | Base Path | Protocol | Primary Consumers | Purpose |
|---|---|---|---|---|
| JSON-RPC | `/rpc` | HTTP POST (JSON-RPC 2.0) | SDK, bots, services | 业务调用主入口（`ag_*`） |
| REST Mgmt | `/v1` | HTTP REST | ops/admin/control-plane | 健康、schema、自省、配额管理 |
| MCP | `/mcp` | Streamable HTTP 或 stdio bridge | MCP hosts | tool/resource/prompt 协议入口 |

统一约束：

- 所有入口都要求 `AuthQuota` 校验（开发模式可配置匿名只读）。
- 每个请求都返回 `traceId`（header 或 payload）用于跨面追踪。
- 错误编码优先稳定机器可解析字段，文本消息仅辅助。

### 2) JSON-RPC 2.0 Contract（D.F10.2）

#### 2.1 Endpoint

- URL: `POST /rpc`
- Content-Type: `application/json`
- Envelope: JSON-RPC 2.0

请求骨架：

```json
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "method": "ag_listSkills",
  "params": {}
}
```

成功响应骨架：

```json
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "result": {},
  "meta": {
    "traceId": "trace_01J..."
  }
}
```

失败响应骨架：

```json
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "error": {
    "code": -32029,
    "message": "RATE_LIMITED",
    "data": {
      "traceId": "trace_01J...",
      "retryAfterMs": 1000
    }
  }
}
```

#### 2.2 Shared JSON Schema Fragments

```json
{
  "$id": "https://api.agentgeyser.dev/schemas/common.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "definitions": {
    "SkillSelector": {
      "oneOf": [
        { "type": "object", "properties": { "mode": { "const": "latest" } }, "required": ["mode"], "additionalProperties": false },
        { "type": "object", "properties": { "mode": { "const": "timestamp" }, "at": { "type": "string", "format": "date-time" } }, "required": ["mode", "at"], "additionalProperties": false },
        { "type": "object", "properties": { "mode": { "const": "manifest" }, "manifestId": { "type": "string", "minLength": 1 } }, "required": ["mode", "manifestId"], "additionalProperties": false },
        { "type": "object", "properties": { "mode": { "const": "range" }, "semver": { "type": "string", "minLength": 1 } }, "required": ["mode", "semver"], "additionalProperties": false }
      ]
    },
    "RiskFlag": {
      "type": "string",
      "enum": ["none", "price-impact", "slippage-risk", "mev-risk", "unknown-program"]
    }
  }
}
```

#### 2.3 `ag_listSkills`

**Purpose**: 按 Program 或全局视角返回可调用技能目录。

Request schema:

```json
{
  "$id": "https://api.agentgeyser.dev/schemas/ag_listSkills.request.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "programId": { "type": "string", "minLength": 32 },
    "tags": {
      "type": "array",
      "items": { "type": "string", "minLength": 1 },
      "maxItems": 32
    },
    "selector": { "$ref": "common.json#/definitions/SkillSelector" },
    "page": { "type": "integer", "minimum": 1, "default": 1 },
    "pageSize": { "type": "integer", "minimum": 1, "maximum": 200, "default": 50 }
  },
  "additionalProperties": false
}
```

Response schema:

```json
{
  "$id": "https://api.agentgeyser.dev/schemas/ag_listSkills.response.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["skills", "page", "pageSize", "total"],
  "properties": {
    "skills": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["programId", "skillName", "skillVersion", "inputSchema", "effects"],
        "properties": {
          "programId": { "type": "string" },
          "skillName": { "type": "string" },
          "skillVersion": { "type": "string" },
          "summary": { "type": "string" },
          "tags": { "type": "array", "items": { "type": "string" } },
          "inputSchema": { "type": "object" },
          "effects": {
            "type": "array",
            "items": { "type": "string", "enum": ["read", "state-write", "token-transfer", "approval"] }
          }
        },
        "additionalProperties": false
      }
    },
    "page": { "type": "integer" },
    "pageSize": { "type": "integer" },
    "total": { "type": "integer" },
    "manifestId": { "type": "string" }
  },
  "additionalProperties": false
}
```

#### 2.4 `ag_invokeSkill`

**Purpose**: 执行指定 skill（默认包含预检/模拟信息）。

Request schema:

```json
{
  "$id": "https://api.agentgeyser.dev/schemas/ag_invokeSkill.request.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["programId", "skillName", "input"],
  "properties": {
    "programId": { "type": "string", "minLength": 32 },
    "skillName": { "type": "string", "minLength": 1 },
    "skillVersion": { "type": "string" },
    "input": { "type": "object" },
    "dryRun": { "type": "boolean", "default": false },
    "traceId": { "type": "string" }
  },
  "additionalProperties": false
}
```

Response schema:

```json
{
  "$id": "https://api.agentgeyser.dev/schemas/ag_invokeSkill.response.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["status", "invocationId", "skillVersion", "simulation"],
  "properties": {
    "status": { "type": "string", "enum": ["planned", "submitted", "failed"] },
    "invocationId": { "type": "string" },
    "skillVersion": { "type": "string" },
    "signature": { "type": "string" },
    "simulation": {
      "type": "object",
      "required": ["ok", "computeUnits", "riskFlags"],
      "properties": {
        "ok": { "type": "boolean" },
        "computeUnits": { "type": "integer", "minimum": 0 },
        "logs": { "type": "array", "items": { "type": "string" } },
        "riskFlags": {
          "type": "array",
          "items": { "$ref": "common.json#/definitions/RiskFlag" }
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

#### 2.5 `ag_planNL`

**Purpose**: 将自然语言请求规划为可执行交易步骤（不强制广播）。

Request schema:

```json
{
  "$id": "https://api.agentgeyser.dev/schemas/ag_planNL.request.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["utterance"],
  "properties": {
    "utterance": { "type": "string", "minLength": 1, "maxLength": 4000 },
    "constraints": {
      "type": "object",
      "properties": {
        "allowedPrograms": { "type": "array", "items": { "type": "string" } },
        "maxFeeLamports": { "type": "integer", "minimum": 0 },
        "riskTolerance": { "type": "string", "enum": ["low", "medium", "high"] }
      },
      "additionalProperties": false
    },
    "traceId": { "type": "string" }
  },
  "additionalProperties": false
}
```

Response schema:

```json
{
  "$id": "https://api.agentgeyser.dev/schemas/ag_planNL.response.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["planId", "steps", "summary", "riskFlags"],
  "properties": {
    "planId": { "type": "string" },
    "summary": { "type": "string" },
    "steps": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["step", "programId", "skillName", "input"],
        "properties": {
          "step": { "type": "integer", "minimum": 1 },
          "programId": { "type": "string" },
          "skillName": { "type": "string" },
          "skillVersion": { "type": "string" },
          "input": { "type": "object" },
          "estimatedFeeLamports": { "type": "integer", "minimum": 0 }
        },
        "additionalProperties": false
      }
    },
    "riskFlags": {
      "type": "array",
      "items": { "$ref": "common.json#/definitions/RiskFlag" }
    }
  },
  "additionalProperties": false
}
```

#### 2.6 `ag_getIdl`

**Purpose**: 返回 Program 对应的 IDL/ABI 元数据与版本信息。

Request schema:

```json
{
  "$id": "https://api.agentgeyser.dev/schemas/ag_getIdl.request.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["programId"],
  "properties": {
    "programId": { "type": "string", "minLength": 32 },
    "version": { "type": "string" }
  },
  "additionalProperties": false
}
```

Response schema:

```json
{
  "$id": "https://api.agentgeyser.dev/schemas/ag_getIdl.response.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["programId", "idlVersion", "idlFormat", "idl"],
  "properties": {
    "programId": { "type": "string" },
    "idlVersion": { "type": "string" },
    "idlFormat": { "type": "string", "enum": ["anchor", "raw", "inferred"] },
    "idl": { "type": "object" },
    "source": { "type": "string", "enum": ["onchain", "registry", "inferred"] },
    "updatedAt": { "type": "string", "format": "date-time" }
  },
  "additionalProperties": false
}
```

### 3) REST Management Surface OpenAPI Snippet（D.F10.3）

> 说明：REST 管理面用于控制平面与运维自省，不替代 `ag_*` 业务调用。

```yaml
openapi: 3.1.0
info:
  title: AgentGeyser Management API
  version: "0.1.0"
servers:
  - url: https://api.agentgeyser.dev
paths:
  /v1/health:
    get:
      summary: Liveness and dependency health
      operationId: getHealth
      responses:
        "200":
          description: Healthy
          content:
            application/json:
              schema:
                type: object
                required: [status, components]
                properties:
                  status:
                    type: string
                    enum: [ok, degraded]
                  components:
                    type: object
                    additionalProperties:
                      type: string
                      enum: [ok, degraded, down]
  /v1/schemas/{method}:
    get:
      summary: Get JSON Schema by canonical ag_* method
      operationId: getMethodSchema
      parameters:
        - in: path
          name: method
          required: true
          schema:
            type: string
            enum: [ag_listSkills, ag_invokeSkill, ag_planNL, ag_getIdl]
      responses:
        "200":
          description: Schema found
          content:
            application/schema+json:
              schema:
                type: object
        "404":
          description: Unknown method
  /v1/tenants/{tenantId}/quotas:
    get:
      summary: Read tenant quota and current usage buckets
      operationId: getTenantQuota
      parameters:
        - in: path
          name: tenantId
          required: true
          schema:
            type: string
      responses:
        "200":
          description: Current quota state
          content:
            application/json:
              schema:
                type: object
                required: [tenantId, plan, buckets]
                properties:
                  tenantId:
                    type: string
                  plan:
                    type: string
                  buckets:
                    type: array
                    items:
                      type: object
                      required: [class, limitPerSecond, remaining, resetAt]
                      properties:
                        class:
                          type: string
                          enum: [read, invoke, plan]
                        limitPerSecond:
                          type: integer
                        remaining:
                          type: integer
                        resetAt:
                          type: string
                          format: date-time
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
security:
  - bearerAuth: []
```

### 4) MCP Endpoint Path & Handshake（D.F10.4）

#### 4.1 Endpoint

- Primary HTTP endpoint: `POST /mcp`
- Optional stream endpoint (server push): `GET /mcp/stream`（若部署选择 streamable transport）
- Local bridge for desktop hosts: stdio adapter -> internally forwards to `/mcp`

#### 4.2 Handshake Sequence

1. Client sends MCP `initialize` to `/mcp` with:
   - protocol version
   - client capabilities
   - auth credential (Bearer/API key)
2. Server validates credential via `AuthQuota` and establishes `sessionId`.
3. Server returns:
   - negotiated protocol version
   - server capabilities (`tools`, `resources`, `prompts`)
   - `traceId` + rate/quota headers
4. Client issues `tools/list` / `resources/list` / `prompts/list`.
5. Subsequent `tools/call` 映射到 canonical `ag_*` 方法（映射详情见 F9）。

握手最小消息示例：

```json
{
  "type": "initialize",
  "protocolVersion": "2025-06-18",
  "clientInfo": { "name": "cursor", "version": "1.0.0" },
  "capabilities": { "tools": true, "resources": true, "prompts": true }
}
```

握手成功响应示例：

```json
{
  "type": "initialize_result",
  "protocolVersion": "2025-06-18",
  "serverInfo": { "name": "agentgeyser-mcp", "version": "0.1.0" },
  "capabilities": {
    "tools": { "listChanged": true },
    "resources": { "subscribe": false },
    "prompts": { "listChanged": false }
  },
  "meta": {
    "sessionId": "mcp_sess_01J...",
    "traceId": "trace_01J..."
  }
}
```

#### 4.3 Security/Quota Notes

- 未认证握手返回 `401 AUTH_REQUIRED`。
- 认证通过但 scope 不足（如缺少 `skills:invoke`）返回 `403 SCOPE_DENIED`。
- 握手成功后所有调用受限于双桶限流（`read`/`invoke`/`plan`），超限返回 `429 RATE_LIMITED`。

### 5) Versioning & Compatibility

- JSON-RPC 方法名保持稳定；字段新增遵循“仅追加可选字段”原则。
- REST 管理面通过 `/v1` 前缀进行 major 隔离。
- MCP 协议版本在握手中协商；若不兼容返回 `MCP_VERSION_UNSUPPORTED`。
- SDK 与 MCP 客户端均应在调用前读取 schema 或 capabilities 缓存，并在 `etag` 变化时刷新。

## Key Decisions & Alternatives

| Decision | Chosen | Alternatives | Trade-offs |
|---|---|---|---|
| 业务入口协议 | JSON-RPC `ag_*` 为主 | 纯 REST | JSON-RPC 更适合工具调用；REST 更直观但 method 扩展一致性较弱 |
| Schema 发布方式 | 每方法独立 JSON Schema + REST 查询入口 | 文档内静态表格 | Schema 可直接机器消费；维护成本更高 |
| REST 范围 | 仅管理/自省，不承载业务执行 | REST 同时承载执行 | 分层清晰、权限更易控；需要两套客户端能力 |
| MCP 路径 | `/mcp` 统一入口 + 可选 `/mcp/stream` | 多端点拆分 tools/resources/prompts | 单入口简化 host 配置；服务端路由复杂度略升 |
| 错误语义 | 稳定错误码 + traceId | 仅自然语言错误 | 便于自动恢复与审计；需要严格版本治理 |

## Risks & Open Questions

- **Schema 演化治理**：多团队并行更新 schema 时需要统一兼容性检查与发布门禁。  
  - Owner: API Council
- **MCP 协议版本更新节奏**：上游 MCP 规范迭代可能影响握手字段兼容。  
  - Owner: MCP Integration Team
- **JSON-RPC 与 REST 权限一致性**：同一租户在不同入口的 scope 映射需防止“权限穿透”。  
  - Owner: Security + Platform
- **超大 input payload**：`ag_planNL` 可能出现超长上下文，需在网关层强制大小限制并返回可恢复错误。  
  - Owner: Gateway Team

## References

- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)
- [OpenAPI Specification 3.1](https://spec.openapis.org/oas/v3.1.0)
- [Model Context Protocol](https://modelcontextprotocol.io/specification)
- [F8 SDK](./08-sdk.md)
- [F9 MCP Server](./09-mcp-server.md)

<!--
assertion-evidence:
  D.F10.1: frontmatter present at file top (doc/title/owner/status/depends-on/updated) and required section structure is included.
  D.F10.2: section "2) JSON-RPC 2.0 Contract" defines ag_listSkills/ag_invokeSkill/ag_planNL/ag_getIdl with explicit request/response JSON Schemas.
  D.F10.3: section "3) REST Management Surface OpenAPI Snippet" provides an OpenAPI 3.1 YAML snippet for REST management endpoints.
  D.F10.4: section "4) MCP Endpoint Path & Handshake" specifies MCP endpoint paths (/mcp, /mcp/stream) and initialize handshake sequence/examples.
-->
