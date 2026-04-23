---
doc: F13
title: 安全威胁模型（STRIDE / Non-Custodial Invariant / Audit & Compliance Hooks）
owner: AgentGeyser Core
status: draft
depends-on: [F4, F5, F7, F9, F10, F11]
updated: 2026-04-23
---

## Goals

- 为 AgentGeyser 建立可执行的威胁模型，覆盖 prompt injection、恶意 IDL、TX spoofing、key handling 等核心风险面。
- 用 STRIDE 方法系统化映射威胁、攻击路径、影响面、检测与缓解控制。
- 明确并“不可歧义”地声明 non-custodial key invariant（平台永不托管用户私钥）。
- 定义审计与合规 hooks，使 `Invocation` / `AuditLog` 可支撑事后追溯、内部审计与监管响应。

## Non-Goals

- 不在本文件实现密码学协议或具体 HSM/KMS 部署代码。
- 不替代 [F14](./14-deployment-observability.md) 的部署加固细节（网络分段、镜像签名、runtime hardening）。
- 不扩展新的公开 API 名称；仅沿用 `ag_listSkills`、`ag_getIdl`、`ag_invokeSkill`、`ag_planNL`。
- 不宣称外部合规认证已完成（如 SOC 2/ISO 27001）；本文件仅定义可落地控制点与证据路径。

## Context

本安全设计与以下文档一致：

- [F4 Modules](./04-modules.md)：模块边界为威胁建模边界（`IdlRegistry`、`SkillSynthesizer`、`NlPlanner`、`McpServer`、`RpcPassthrough`、`AuthQuota`）。
- [F5 IDL Registry](./05-idl-registry.md)：IDL 来源真实性、版本化与回滚策略是恶意 IDL 防线核心。
- [F7 NL Planner](./07-nl-planner.md)：NL→TX 规划链路是 prompt injection 与交易欺骗高风险面。
- [F9 MCP Server](./09-mcp-server.md)：外部 agent 接入面带来 tool misuse、身份冒用与速率滥用风险。
- [F10 API](./10-api.md)：`ag_*` 方法的鉴权、输入校验、返回签名元数据是主要控制点。
- [F11 Data Model](./11-data-model.md)：`Invocation` 与 `AuditLog` 承担审计可追溯性。

安全设计原则：

1. **Least trust by default**：任何来自用户、上游 RPC、IDL registry、LLM 的输入都视为不可信。
2. **Deterministic over implicit**：关键执行路径以 schema + policy 引擎约束，避免仅靠自然语言解释。
3. **Non-custodial always**：平台可规划、可模拟、可广播，但不可持有或推导用户私钥。
4. **Audit-first**：每次高风险决策必须产生结构化审计事件。

## Design

### 1) Threat Surface & Trust Boundaries

关键边界：

1. **Client boundary**：SDK/MCP client 到 AgentGeyser 入口（身份、配额、输入污染）。
2. **Planning boundary**：`NlPlanner` 与 LLM / 工具调用交互（prompt injection、越权工具调用）。
3. **On-chain schema boundary**：`IdlRegistry` 接收并解析 IDL（恶意/伪造 IDL、格式炸弹）。
4. **Execution boundary**：计划交易到模拟、提交前后（TX spoofing、参数替换、账户重定向）。
5. **Audit boundary**：日志生成、存储、导出（日志篡改、不可否认性不足）。

### 2) STRIDE Threat Model（E.F13.2）

| STRIDE | Concrete Threat | Primary Attack Path | Impact | Detection | Mitigation Controls |
|---|---|---|---|---|---|
| **S** Spoofing | API key / MCP client 身份伪造 | 泄露 token、重放、伪造 client metadata | 越权调用 `ag_invokeSkill` / `ag_planNL` | 异常 IP/UA 指纹、nonce 重放命中、失败签名校验 | mTLS（server-to-server 可选）、短期 token、签名挑战 nonce、`AuthQuota` 绑定 tenant + scope |
| **T** Tampering | 恶意 IDL 注入或 schema 篡改 | 伪造 registry 源、MITM、缓存污染 | 生成错误 Skill，导致错误交易构造 | `schema_hash` 不一致告警、来源签名校验失败、版本突变异常 | 多源比对（on-chain + trusted registry）、IDL 签名与哈希校验、append-only 版本、快速回滚 |
| **R** Repudiation | 调用方否认曾发起高风险请求 | 缺乏强审计锚点、request-id 不可追溯 | 争议无法裁定，合规失败 | 缺失 trace 检查、审计事件链断裂报警 | `Invocation` + `AuditLog` 强制落库、不可变事件序列、签名时间戳与 request digest |
| **I** Information Disclosure | Prompt/日志泄露敏感信息（地址映射、策略） | LLM 回显、调试日志、错误栈外泄 | 隐私泄漏、策略被对手利用 | DLP 规则命中、PII 模式匹配、异常导出 | 敏感字段脱敏、最小日志原则、响应 redaction、按租户隔离审计导出 |
| **D** Denial of Service | Prompt flooding / IDL bomb / 模拟风暴 | 大体积输入、递归 tool 调用、恶意批量 simulate | 可用性下降、成本飙升 | QPS/queue depth 激增、token 消耗异常、解析超时比率 | `AuthQuota` 分层限流、请求体大小上限、超时/并发熔断、缓存优先与降级模式 |
| **E** Elevation of Privilege | Prompt injection 导致越权工具路由 | 用户文本诱导 planner 调用不允许 skill 或参数 | 未授权资产操作建议/提交 | policy deny 命中、route 偏离基线检测 | policy engine allowlist、工具级 scope ACL、高风险操作二次确认（human-in-the-loop 可选） |

#### 2.1 Mandatory Threats Coverage

- **Prompt injection**：归类为 `Elevation of Privilege` + `Information Disclosure` 复合威胁；通过 tool allowlist、system prompt sealing、policy post-check 阻断。
- **Malicious IDL**：归类为 `Tampering`；通过 `schema_hash`、来源签名、版本冻结与回滚控制。
- **TX spoofing**：归类为 `Tampering` + `Spoofing`；通过交易摘要绑定、关键账户白名单校验、提交前再模拟。
- **Key handling**：归类为 `Information Disclosure` + `Elevation`；核心在 non-custodial invariant（见下一节）。

### 3) Non-Custodial Key Invariant（E.F13.3）

> **Invariant（强约束）**：AgentGeyser **永不生成、存储、托管、导出、或代签名**用户私钥/助记词。  
> 平台仅处理：公开地址、公有链状态、IDL/Skill 元数据、未签名交易草案、模拟结果、以及已签名交易的广播回执。

#### 3.1 Allowed vs Forbidden

| Category | Allowed | Forbidden |
|---|---|---|
| Key material | 公钥、地址、签名后的交易字节（可验证） | 私钥、助记词、seed phrase、可逆派生材料 |
| Signing flow | 客户端本地钱包签名（Wallet Adapter、硬件钱包、MPC 客户端） | 服务端代理签名、服务端密钥缓存、明文密钥上传 |
| Planner output | 未签名 transaction message + 风险说明 | 自动附带私钥、绕过客户端确认直接签名 |
| Telemetry | 哈希化 key fingerprint（不可逆） | 可还原密钥片段、完整签名原文长期留存 |

#### 3.2 Enforcements

1. API schema 层拒绝任何“私钥样式字段”输入（base58 长度 + entropy 特征 + known mnemonic wordlist）。
2. 请求日志中对疑似密钥载荷执行硬拦截与敏感告警，不写入持久日志。
3. `ag_invokeSkill` 仅返回待签名 payload 或提交结果，不提供 server-side signer 选项。
4. SDK/MCP 文档和响应元数据显式提示：“sign locally, broadcast optionally via proxy”。

### 4) Audit & Compliance Hooks（E.F13.4）

为满足内部审计与合规取证，定义以下 hooks（事件必须进入 `AuditLog`，并与 `Invocation` 关联）：

1. **Auth Hook**：鉴权成功/失败、scope 判定、quota 决策、token 轮换事件。
2. **Planner Hook**：意图分类结果、tool route 决策、policy deny 原因码、风险标签（MEV/slippage/account-drift）。
3. **IDL Integrity Hook**：IDL 来源、`schema_hash`、签名验证结果、版本切换与回滚记录。
4. **Execution Hook**：simulate 结果摘要、最终提交 payload digest、tx signature、重试与失败原因。
5. **Security Incident Hook**：prompt injection 命中、密钥输入拦截、异常流量封禁、疑似账户冒用。
6. **Compliance Export Hook**：按 `tenant_id + time range + trace_id` 导出审计链，支持不可变快照签名。

#### 4.1 Audit Event Minimum Schema

```json
{
  "log_id": "uuid",
  "invocation_id": "uuid",
  "trace_id": "trace-...",
  "tenant_id": "tenant-...",
  "event_type": "planner.policy_denied",
  "severity": "warn",
  "actor_type": "system",
  "request_digest": "sha256:...",
  "policy_version": "2026-04-23.1",
  "risk_flags": ["prompt_injection_suspected"],
  "created_at": "2026-04-23T12:34:56Z"
}
```

#### 4.2 Compliance Posture Mapping（示例）

- **Access control evidence**：Auth Hook + scope decision log。
- **Change management evidence**：IDL Integrity Hook（版本变更、回滚审批）。
- **Incident response evidence**：Security Incident Hook + trace timeline。
- **Financial-action traceability**：Execution Hook 关联 `invocation_id -> tx_signature`。

### 5) Security Control Plane by Module

| Module | Critical Controls | Failure Mode | Guardrail |
|---|---|---|---|
| `AuthQuota` | API key scope、nonce、防重放、分层限流 | token 滥用 / DoS | 强制短 TTL 凭证、租户级异常封禁 |
| `IdlRegistry` | 来源验证、hash pinning、版本回滚 | 恶意 IDL 污染 | 多源校验 + quarantine 队列 |
| `SkillSynthesizer` | schema 验证、effect annotation policy | 语义误标导致危险 skill | 高风险 archetype 人审阈值 |
| `NlPlanner` | prompt sandbox、tool allowlist、policy post-check | prompt injection 越权 | route deny + explainable rejection |
| `RpcPassthrough` | upstream pinning、响应完整性检查 | 中间人或上游污染 | provider allowlist + failover validation |
| `McpServer` | client auth、tool ACL、session isolation | agent 侧越权调用 | 每工具独立 scope + 配额隔离 |

## Key Decisions & Alternatives

| Decision | Chosen | Alternatives | Trade-offs |
|---|---|---|---|
| 威胁框架 | STRIDE 主模型 + 风险标签扩展 | 仅 OWASP Top 10 列表 | STRIDE 覆盖更系统；需更高建模成本 |
| 私钥策略 | 强 non-custodial invariant（零托管） | 托管式 signer as-a-service | 安全与责任边界清晰；牺牲部分“一键体验” |
| IDL 信任策略 | 多源验证 + hash pinning + quarantine | 单一 registry 直接信任 | 抗污染更强；增加同步与运营复杂度 |
| Planner 安全策略 | pre-route + post-route 双 policy | 仅前置 prompt 过滤 | 双重防线更稳；延迟略增 |
| 审计存储 | `Invocation` + append-only `AuditLog` | 仅文本日志 | 可检索、可追责；存储成本更高 |

## Risks & Open Questions

- **LLM 供应链风险**：第三方模型行为漂移可能绕过旧规则。  
  - Mitigation: 模型版本 pinning + policy regression suite。  
  - Owner: AI Platform
- **跨链/跨程序组合交易复杂性**：多步计划增加 spoofing 检测难度。  
  - Mitigation: 关键账户与 token-mint allowlist，步骤级模拟。  
  - Owner: NlPlanner
- **误报率与用户体验权衡**：过严拦截影响转化。  
  - Mitigation: 分级策略（warn/challenge/block）+ 租户可配置阈值。  
  - Owner: Product + Security
- **审计数据驻留法规差异**：多区域法规对日志保留期限不同。  
  - Mitigation: region-aware retention policy + export controls。  
  - Owner: Compliance

## References

- [F4 Module Decomposition](./04-modules.md)
- [F5 IDL Registry](./05-idl-registry.md)
- [F7 NL Planner](./07-nl-planner.md)
- [F9 MCP Server](./09-mcp-server.md)
- [F10 External API](./10-api.md)
- [F11 Data Model](./11-data-model.md)
- [Microsoft STRIDE Threat Modeling](https://learn.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [OWASP ASVS](https://owasp.org/www-project-application-security-verification-standard/)

<!--
assertion-evidence:
  E.F13.1: file exists at docs/design/13-security.md with required frontmatter (doc/title/owner/status/depends-on/updated).
  E.F13.2: section "2) STRIDE Threat Model" contains STRIDE table explicitly covering prompt injection, malicious IDL, TX spoofing, and key handling.
  E.F13.3: section "3) Non-Custodial Key Invariant" states explicit invariant that AgentGeyser never generates/stores/custodies/signs with user private keys.
  E.F13.4: section "4) Audit & Compliance Hooks" lists concrete audit/compliance hooks and minimum audit event schema.
-->
