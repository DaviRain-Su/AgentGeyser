# AetherNode Vision (archived PRD)

> **Status: vision document, NOT a committed roadmap.**
> This file archives an externally-authored PRD describing a broader multi-chain
> vision called **AetherNode**. It is kept here for reference only. It is **not**
> the authoritative mission tracker for AgentGeyser. The real, committed,
> incrementally-executed plan lives in `AGENTS.md` and the mission tracker that
> it references. If this file and `AGENTS.md` disagree, `AGENTS.md` wins.

AetherNode's thesis, in one sentence: **blockchain nodes should natively embed
agent capabilities** — dynamic skill generation from on-chain IDL/ABI, pluggable
LLM config, reflection loops, multi-provider fallback — rather than relying on a
sidecar proxy. AgentGeyser is the Solana-first, proxy-first step toward that
vision; it corresponds to AetherNode PRD §8 "Solana Geyser 版 (Proxy → 节点嵌入)".

## Original PRD

### 产品需求文档 (PRD)

**产品名称**：**AetherNode**（以太节点 / 智能节点）
**版本**：v0.1（MVP）
**文档版本**：1.0
**日期**：2026年4月24日
**作者**：Grok（根据用户的第一性原理想法整理）

---

#### 1. 产品概述

**一句话描述**：
AetherNode 是一个**原生嵌入区块链节点的 Agent Module**，让全节点本身变成"会学习、动态自适应"的智能接入层。

**核心理念**（第一性原理）：
传统区块链节点（RPC / Fullnode）的接口是静态、硬编码的。我们要让节点本身内置轻量 Agent 能力，通过配置外部 LLM，实时监听新合约/Module，自动生成高层技能（Skills），向上游 AI Agent 提供动态、智能的交互接口。

**愿景**：
成为区块链基础设施层的"Agent-native 操作系统"，让任何 AI Agent 连接节点后，无需手动写 SDK、适配新合约，即可自然语言或动态方法调用链上功能。

---

#### 2. 目标用户

1. **AI Agent 开发者**（核心用户）——构建自主交易、DeFi Agent、监控 Agent 等
2. **区块链节点运营商 / RPC 服务商**（Helius、QuickNode 类）
3. **dApp 开发者** ——需要快速适配新协议
4. **链上协议方** ——希望降低用户接入门槛

---

#### 3. 核心价值主张

- **动态适配**：新合约/新 Package 上线后，节点自动解析并生成高层技能
- **配置化 LLM**：不内置重模型，只通过 toml 配置调用外部 LLM（Grok、Claude、OpenAI、Ollama 等）
- **节点原生**：下沉到节点内核（非外部 Proxy），性能高、安全、可选加载
- **持续学习**：基于历史调用日志实现反思与技能优化
- **跨链兼容**：优先支持 Sui + Reth（覆盖 Move + 大部分 EVM 链）

---

#### 4. 核心功能需求（MVP）

##### Phase 1：基础 Agent Module
- 配置化外部 LLM（多 Provider + Fallback）
- 实时监听新合约/Module 部署
- 自动解析（Sui：Move Module；EVM：ABI）
- 动态生成高层 Skills（自然语言描述 + 执行逻辑）
- 暴露增强接口（gRPC / JSON-RPC / MCP）

##### Phase 2：智能与学习
- 调用日志本地缓存
- 定期反思循环（Reflection Loop）
- 技能版本管理与优化
- 请求脱敏与安全 sandbox

##### Phase 3：扩展能力
- 多链支持（Sui + Reth + Monad + Solana Proxy）
- 技能共享机制（节点间可选同步）
- 可视化管理面板（可选）

---

#### 5. 非功能需求

- **性能**：模块默认关闭，对节点 TPS / 延迟影响 < 2%
- **安全性**：可选加载、权限隔离、数据脱敏、sandbox 执行
- **可扩展性**：纯 Rust 实现（pi-rs / codex-rs 作为 Agent 内核）
- **部署**：支持 Docker、一键启用（`--enable-agent-module`）
- **兼容性**：不破坏现有 RPC / gRPC 接口

---

#### 6. 技术实现框架（推荐）

- **Agent 内核**：pi-rs（优先）或 codex-rs
- **Sui 实现**：fork MystenLabs/sui → 新增 `sui-agent-module`
- **Reth 实现**：fork paradigmxyz/reth → 使用 ExEx + `extend_rpc_modules`
- **Monad 实现**：fork category-labs/monad + monad-bft → 使用 Execution Events SDK
- **配置格式**：TOML
- **通信协议**：MCP + 增强 gRPC/JSON-RPC

---

#### 7. 成功指标（OKR）

- **MVP 成功**：能在 Testnet 上部署带 Agent Module 的节点，新合约上线后 10 秒内自动生成可用 Skill
- **用户指标**：首月 50 个开发者节点部署
- **技术指标**：模块内存占用 < 150MB，额外延迟 < 800ms（LLM 调用）

---

#### 8. 开发优先级（Roadmap）

**MVP（4-6 周）**：
1. Reth 版本（ExEx 集成）——覆盖大部分 EVM 链
2. Sui 版本（全节点嵌入）
3. 基础配置 + 动态 Skill 生成 + MCP

**后续版本**：
- Monad 版本
- Solana Geyser 版（Proxy → 节点嵌入）
- 跨链 Skill 同步
- 管理 Dashboard

---

#### 9. 风险与假设

- **风险**：LLM 调用延迟、hallucination、节点升级兼容性
- **缓解**：缓存 + 多模型 fallback + 确定性执行分离
- **假设**：外部 LLM API 稳定可用；社区接受节点嵌入 Agent 概念

---

#### 10. 附录

- **关键用户故事**：
  - 作为 AI Agent，我连接一个 AetherNode 后，可以直接说"用最新上线的 DEX 把 100 USDC swap 成 ETH"，无需手动查 ABI。
  - 作为节点运营商，我只需在配置文件里加几行 LLM Key，就能让我的节点支持智能动态调用。

## AgentGeyser ↔ AetherNode mapping

| AetherNode PRD dimension   | AgentGeyser current state (as of mvp-m5a)                    | M5b in progress                                             | Future (M5c+)                                                   |
|----------------------------|--------------------------------------------------------------|-------------------------------------------------------------|-----------------------------------------------------------------|
| Scope                      | Solana only (mainnet/devnet via RPC proxy)                   | Solana only, adds NL planning surface                       | Solana first-class; Sui/Reth/Monad as separate tracks           |
| LLM providers              | None in-process (MCP clients bring their own)                | Mock + OpenAI (F16 adds Kimi) behind `LlmProvider` trait    | Add Claude + Gemini; pluggable fallback chain                   |
| Node embedding             | Out-of-node proxy (Axum) in front of RPC                     | Same proxy model; planner is a library crate                | Yellowstone gRPC plugin / node-kernel extension explored        |
| Skill dynamic generation   | IDL-registry + skill-synth from known Anchor IDLs            | Unchanged — planner consumes existing skill catalog          | Auto-register skills on new SPL program deployments             |
| Continuous learning        | None (static catalog refresh)                                | None (deterministic planning only)                          | Reflection loop: post-execution feedback re-tunes planner/LLM   |
| Cross-chain                | Not attempted                                                | Not attempted                                               | Separate repos/orgs for non-Solana chains (out of scope here)   |

## Current progress on the Solana track

- **M1–M2 — Smart RPC proxy.** Done. Axum-based proxy in `crates/proxy`.
- **M3 — MCP server.** Done. `crates/mcp-server` + `packages/mcp-client`.
- **M4 — TypeScript SDK + React hooks + Docs.** Done. Tagged `mvp-m4`.
- **M5a — Release foundation.** Done (CI + npm-publish-readiness). Tagged
  `mvp-m5a`, commit `c9ef672`.
- **M5b — in progress.** Natural-language planner crate (`nl-planner`) with a
  multi-provider `LlmProvider` trait: Mock (deterministic fixtures), OpenAI
  (reqwest + JSON `response_format` + token budget), and Kimi (F16). Includes a
  live devnet Track A probe with an airdrop gate.

## Gaps vs AetherNode vision

- **Dynamic skill generation from new contract deployments.** AgentGeyser today
  registers skills from curated Anchor IDLs; AetherNode envisions auto-indexing
  new program deployments (e.g. via Yellowstone gRPC) and synthesizing
  invocable skills without a human-in-the-loop commit.
- **Reflection loop.** AetherNode expects the agent to observe execution
  outcomes and refine future plans; AgentGeyser's planner is currently
  stateless and single-shot.
- **Node-kernel embedding.** AgentGeyser runs as an out-of-node proxy;
  AetherNode's end-state embeds the agent inside the node (Yellowstone gRPC
  plugin for Solana, analogous hooks for other chains).
- **Multi-provider depth.** OpenAI + Kimi + Mock only; no Claude / Gemini /
  local-model adapters and no declared fallback policy.
- **Cross-chain.** AgentGeyser is strictly Solana; AetherNode targets Sui /
  Reth / Monad as peer tracks.

## Path from AgentGeyser to AetherNode (Solana edition)

_Forward-looking, not committed. Treat as hypothesis, not a schedule._

- **M5c (proposed).** npm publish via trusted publishing; Yellowstone gRPC
  auto-register so newly deployed SPL programs surface as skills without manual
  IDL curation.
- **M6 (proposed).** Claude + Gemini providers behind the existing
  `LlmProvider` trait; first-cut reflection loop that feeds invocation results
  back into the planner; node-operator integration docs.
- **M7+ (proposed).** Sui / Reth / Monad tracks. These should almost certainly
  spin off to a separate org or repo — they are **out of scope** for this
  repository.

## Where the real plan lives

For the concrete, authoritative, incremental plan — milestone gates, ticket
IDs, CI invariants, git policy — see:

- `AGENTS.md` (repo root) — build/test/run, architecture, code style, git
  policy (Option C: `--force-with-lease` allowed).
- The mission tracker referenced from `AGENTS.md`.

This document is vision-level and intentionally speculative. Do not port items
from here into mission-tracker state without an explicit decision recorded in
`AGENTS.md`.
