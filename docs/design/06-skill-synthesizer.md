---
doc: F6
title: Skill Synthesizer (IDL → Semantic Skills)
owner: AgentGeyser Core
status: draft
depends-on: [F4, F5]
updated: 2026-04-23
---

# AgentGeyser Skill Synthesizer (IDL → Semantic Skills)

## Goals

- 定义 `SkillSynthesizer` 如何将 `Idl` 转换为高层可调用 `Skill` / `SkillVersion`。
- 提供稳定的 IDL→Skill 规则，覆盖至少 `swap` / `transfer` / `stake` / `mint` 四类 archetype。
- 发布可复用的参数 `JSON Schema` 模板，供 `ag_listSkills`、`ag_invokeSkill`、MCP 与 SDK 共享。
- 给出 LLM semantic tagging prompt 模板，确保非 Anchor/弱语义 IDL 也可被一致标注。

## Non-Goals

- 不设计自然语言到交易计划的推理流程（见 [F7](./07-nl-planner.md)）。
- 不定义外部 JSON-RPC 端点完整契约（见 F10）。
- 不规定底层 Postgres DDL 与 Redis key-space 细节（见 F11）。

## Context

本文 fulfills `C.F6.1`, `C.F6.2`, `C.F6.3`, `C.F6.4`。  
输入来自 [F5 IDL Registry](./05-idl-registry.md) 的 `IdlSnapshot`；模块边界遵循 [F4 Modules](./04-modules.md) 中 `SkillSynthesizer`、`IdlRegistry`、`NlPlanner`、`McpServer` canonical 命名。

核心实体使用统一术语：`Program`, `Idl`, `Skill`, `SkillVersion`, `Invocation`, `AuditLog`。

## Design

### 1) Synthesis Pipeline

`SkillSynthesizer` 对每个 active `Idl` 执行确定性多阶段转换：

1. **Instruction Extraction**：读取 instruction 名称、accounts、args、docs、events、errors。
2. **Feature Derivation**：提取语义特征（token movement、authority role、pool/account patterns、amount-like args）。
3. **Archetype Classification**：规则优先，LLM 补充（低置信/冲突时）。
4. **Schema Emission**：输出统一参数 JSON Schema 与 effect metadata。
5. **Version Materialization**：写入 `Skill` / `SkillVersion`，并产出 catalog 索引。

### 2) IDL → Skill Mapping Rules

#### 2.1 Rule Precedence

- **P1 Deterministic Rules**（最高优先）：基于 instruction 名、account 角色、arg 结构的硬规则。
- **P2 Protocol Hints**：来自 `IdlRegistry` 的 provenance、生态协议签名、known discriminator hints。
- **P3 LLM Tagging**：仅在 P1/P2 无法唯一分类或置信度不足时启用。
- **P4 Safe Fallback**：输出 `custom` archetype，限制默认自动执行能力。

#### 2.2 Canonical Skill Archetypes (Required)

至少支持以下 archetype（满足 C.F6.2）：

1. **`transfer`**
   - 识别信号：instruction 名含 `transfer`/`send`；accounts 含 sender/receiver；arg 含 amount。
   - 常见效果：资产从 source 移动到 destination；通常单 hop。
2. **`swap`**
   - 识别信号：instruction 名含 `swap`/`exchange`；accounts 包含 pool/market/vault；输入输出 mint 成对出现。
   - 常见效果：输入 token 转换为输出 token，含滑点与报价相关参数。
3. **`stake`**
   - 识别信号：instruction 名含 `stake`/`delegate`/`unstake`；accounts 包含 validator/stake account。
   - 常见效果：资产锁定、委托或赎回，可能包含 epoch/lockup 约束。
4. **`mint`**
   - 识别信号：instruction 名含 `mint`/`issue`; accounts 含 mint authority 与 mint account。
   - 常见效果：增发资产、更新供应量，常伴随权限校验。

扩展 archetype（可选）：`burn`, `lend`, `borrow`, `claim`, `provide_liquidity`, `withdraw_liquidity`, `governance_vote`。

#### 2.3 Mapping Output Contract

每个 skill 至少包含：

- `skill_id`: 稳定标识（建议 `program_id:instruction_name:archetype` hash）
- `name`: 语义化名称（如 `jupiter.swapExactIn`）
- `archetype`: `swap|transfer|stake|mint|custom|...`
- `program_id`, `idl_version`, `skill_version`
- `input_schema`（JSON Schema）
- `effects`: side-effect 标签（资产变化、权限需求、可逆性）
- `confidence`: 0~1
- `source`: `rules` | `rules+llm`

### 3) JSON Schema Template for Skill Parameters

下列模板用于 `SkillVersion.input_schema`（满足 C.F6.3）：

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agentgeyser.dev/schema/skill-params/v1",
  "title": "SkillParameters",
  "type": "object",
  "required": ["programId", "skillName", "params"],
  "properties": {
    "programId": {
      "type": "string",
      "description": "Solana program pubkey (base58)",
      "pattern": "^[1-9A-HJ-NP-Za-km-z]{32,44}$"
    },
    "skillName": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "params": {
      "type": "object",
      "description": "Instruction arguments mapped to semantic names",
      "additionalProperties": true
    },
    "accounts": {
      "type": "object",
      "description": "Optional explicit account overrides",
      "additionalProperties": {
        "type": "string",
        "pattern": "^[1-9A-HJ-NP-Za-km-z]{32,44}$"
      }
    },
    "constraints": {
      "type": "object",
      "properties": {
        "slippageBps": { "type": "integer", "minimum": 0, "maximum": 10000 },
        "maxLamports": { "type": "integer", "minimum": 0 },
        "deadlineTs": { "type": "integer", "minimum": 0 }
      },
      "additionalProperties": false
    },
    "dryRun": {
      "type": "boolean",
      "default": true
    },
    "clientMeta": {
      "type": "object",
      "properties": {
        "sdkVersion": { "type": "string" },
        "traceId": { "type": "string" }
      },
      "additionalProperties": true
    }
  },
  "additionalProperties": false
}
```

#### Archetype-specific refinement example (`swap`)

```json
{
  "allOf": [
    { "$ref": "https://agentgeyser.dev/schema/skill-params/v1" },
    {
      "type": "object",
      "required": ["params"],
      "properties": {
        "params": {
          "type": "object",
          "required": ["inputMint", "outputMint", "amountIn"],
          "properties": {
            "inputMint": { "type": "string", "pattern": "^[1-9A-HJ-NP-Za-km-z]{32,44}$" },
            "outputMint": { "type": "string", "pattern": "^[1-9A-HJ-NP-Za-km-z]{32,44}$" },
            "amountIn": { "type": "string", "pattern": "^[0-9]+$" },
            "minAmountOut": { "type": "string", "pattern": "^[0-9]+$" }
          },
          "additionalProperties": false
        }
      }
    }
  ]
}
```

### 4) Effect Labels & Execution Guardrails

每个 `SkillVersion` 附带标准 effects 标签：

- `asset_movement`: `none|single_token|multi_token`
- `state_mutation`: `read_only|mutable`
- `authority_required`: `none|signer|multisig|program_authority`
- `reversibility`: `reversible|partially_reversible|irreversible`
- `risk_level`: `low|medium|high`

Guardrail 规则：

- `risk_level=high` 或 `confidence<threshold` 默认要求 `dryRun=true`。
- `custom` archetype 仅允许显式租户白名单执行。
- schema validation 失败时禁止进入 `ag_invokeSkill` 执行链路。

### 5) LLM Prompt Templates for Semantic Tagging

以下模板用于 P3 阶段（满足 C.F6.4）。模板通过变量插值后送入模型；输出必须是严格 JSON。

#### 5.1 System Prompt Template

```text
You are SkillTagger for Solana IDL analysis.
Task: classify each instruction into a canonical archetype and produce safe parameter semantics.
Allowed archetypes: swap, transfer, stake, mint, burn, lend, borrow, custom.
Hard rules:
1) Prefer deterministic evidence from instruction names/accounts/args.
2) If evidence is insufficient, choose custom and lower confidence.
3) Never invent accounts or arguments not present in IDL.
4) Output valid JSON only, matching the required schema.
```

#### 5.2 User Prompt Template

```text
Input:
- program_id: {{program_id}}
- idl_version: {{idl_version}}
- instruction: {{instruction_name}}
- docs: {{instruction_docs}}
- accounts: {{accounts_json}}
- args: {{args_json}}
- provenance: {{provenance_json}}

Return JSON:
{
  "instruction": "string",
  "archetype": "swap|transfer|stake|mint|burn|lend|borrow|custom",
  "confidence": 0.0,
  "semantic_name": "string",
  "param_mapping": [
    { "idl_arg": "string", "semantic_arg": "string", "type": "string", "required": true }
  ],
  "effects": {
    "asset_movement": "none|single_token|multi_token",
    "state_mutation": "read_only|mutable",
    "authority_required": "none|signer|multisig|program_authority",
    "reversibility": "reversible|partially_reversible|irreversible",
    "risk_level": "low|medium|high"
  },
  "rationale": ["string"]
}
```

#### 5.3 Conflict-Resolution Prompt Template

```text
Given:
- rule_engine_result: {{rule_result_json}}
- llm_result: {{llm_result_json}}

Choose final result with policy:
1) If archetype differs and rule confidence >= 0.8, keep rule result.
2) If both confidence < 0.6, set archetype=custom.
3) Preserve strict subset of args existing in IDL only.
Return JSON with fields: final_archetype, final_confidence, merged_param_mapping, notes.
```

### 6) Versioning & Publishing Policy

- 同一 `program_id + instruction_signature` 的语义变化创建新 `SkillVersion`。
- 仅当 `Idl.active_version` 更新或映射规则版本升级时触发重算。
- 发布条件：
  - schema 可验证；
  - confidence 达标（默认 ≥0.72）；
  - 无高危冲突标签未审计。

`SkillSynthesizer` 输出将被：
- `ag_listSkills` 读取技能目录；
- `ag_invokeSkill` 读取 schema 与 effects；
- `NlPlanner` 用于工具路由与风控；
- `McpServer` 暴露为 MCP tools/resources 元数据。

## Key Decisions & Alternatives

| Decision | Chosen | Alternative | Trade-off |
|---|---|---|---|
| Archetype classification | 规则优先 + LLM 补充 | 全量依赖 LLM | 稳定可解释，成本可控；但规则维护成本更高 |
| Schema standard | JSON Schema Draft 2020-12 | 自定义 DSL | 与生态兼容性高；但表达某些链上约束需扩展 |
| Low-confidence handling | 降级为 `custom` + 限制执行 | 仍自动发布 | 安全性更高；覆盖率与自动化程度下降 |
| Version trigger | IDL/规则版本变化触发新 `SkillVersion` | 每次重算覆盖旧版 | 可审计可回滚；需要更多存储与索引 |
| Effect model | 标准五维标签 | 自由文本描述 | 机器可消费；但初期标签体系需迭代校准 |

## Risks & Open Questions

- **Risk**: 同一生态协议跨版本命名波动大，可能导致 archetype 抖动。  
  **Owner**: Runtime Intelligence.
- **Risk**: LLM 在长尾非 Anchor 合约上给出高置信错误语义。  
  **Owner**: Safety Review.
- **Open Question**: MVP 是否支持租户自定义 archetype 覆盖规则（BYO mapping）？
- **Open Question**: `confidence` 阈值是否按 archetype 动态调整（如 `mint` 更严格）？

## References

- [F4 Module Decomposition](./04-modules.md)
- [F5 IDL Registry](./05-idl-registry.md)
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)

<!--
assertion-evidence:
  C.F6.1: frontmatter at lines 1-7 includes doc/title/owner/status/depends-on/updated
  C.F6.2: section "2) IDL → Skill Mapping Rules" and "2.2 Canonical Skill Archetypes (Required)" defines mapping rules and includes swap/transfer/stake/mint
  C.F6.3: section "3) JSON Schema Template for Skill Parameters" publishes full JSON Schema template
  C.F6.4: section "5) LLM Prompt Templates for Semantic Tagging" includes system/user/conflict prompt templates
-->
