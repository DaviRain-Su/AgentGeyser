---
doc: F1
title: Vision & Problem Statement
owner: AgentGeyser Core
status: draft
depends-on: []
updated: 2026-04-23
---

# AgentGeyser Vision & Problem Statement

## Goals

- 明确 AgentGeyser 解决的核心问题与价值主张。
- 定义目标用户与其 jobs-to-be-done，确保后续架构与 API 设计有一致北极星。
- 设定可量化成功指标，作为 Spike/MVP/Beta 阶段验收基线。

## Problem

当前 Solana 生态中，AI Agent 与 dApp 开发者面临一个结构性问题：**链上 Program 与接口变化速度快，但客户端能力演进滞后**。具体表现为：

1. **Program 发现与理解成本高**：新 Program 上线后，开发者需要手工追踪、解析 IDL 或逆向接口语义。
2. **客户端集成碎片化**：每个协议都要单独写 SDK 适配或脚本胶水，重复劳动高且易出错。
3. **自然语言到交易执行断层**：用户意图到可执行 Transaction 之间缺少可审计、可模拟、可控费率的统一规划层。
4. **AI 工具接入标准不统一**：不同 Agent 框架（Claude/Cursor/自建 Agent）需要不同集成路径，导致维护成本上升。

AgentGeyser 的愿景是成为 **AI-native Solana RPC intelligence layer**：持续学习链上能力、统一抽象为技能（Skills），并通过 SDK / NL Planner / MCP 三种入口稳定输出。

## Target Users

### Persona 1: AI Agent Developer（AI 代理开发者）

- **背景**：构建交易助手、研究助手、自动化执行 Agent。
- **JTBD**：希望“告诉 Agent 做什么”，而不是为每个新协议手写 client 代码。
- **痛点**：工具链与协议变化频繁，prompt/tool routing 难以长期维护。
- **期望收益**：通过统一 `ag_*` 能力与 MCP 暴露，快速连接可执行链上动作。

### Persona 2: DeFi Bot Operator（量化/策略机器人运营者）

- **背景**：维护套利、做市、再平衡、清算等策略机器人。
- **JTBD**：尽快识别新协议可调用操作，并在低延迟下安全执行交易。
- **痛点**：新池子/新合约上线时接入窗口短，手工适配会错过机会。
- **期望收益**：自动技能注册 + 交易模拟 + 优先费建议，缩短策略上线时间。

### Persona 3: dApp Application Engineer（dApp 应用工程师）

- **背景**：开发钱包、聚合器、自动化前后端工作流。
- **JTBD**：以稳定 SDK 接口消费链上能力，减少协议侧变更对业务代码影响。
- **痛点**：多协议 SDK 风格不一、版本冲突、文档质量参差。
- **期望收益**：动态 TypeScript SDK + 版本钉选能力，降低集成复杂度与维护成本。

## Success Metrics

以下指标用于衡量 F1 愿景是否被后续里程碑有效承接（均为可量化目标）：

1. **Time-to-Integrate（TTI）**：新 Program 首次被发现到可通过 `ag_listSkills` 可见，P95 ≤ **10 分钟**。
2. **Skill Coverage Rate**：前 100 个活跃 Program 中可自动产出可调用技能的比例 ≥ **70%**。
3. **Planner Reliability**：`ag_planNL` 产出的交易计划在模拟阶段通过率 ≥ **90%**（剔除余额不足等用户侧约束）。
4. **Integration Efficiency**：典型 dApp 集成首个可用调用链路所需工程时间从基线（手工 SDK）下降 **50%+**。
5. **Invocation Success Ratio**：通过 `ag_invokeSkill` 发起调用的链上成功确认率 ≥ **95%**（按可重试错误归一化）。
6. **Operational Cost Envelope**：单次 NL 规划平均推理成本控制在 **$0.01–$0.05** 区间（按缓存命中分层统计）。

## Non-Goals

为避免范围失控，本阶段明确以下非目标：

- **不构建托管钱包或私钥托管服务**（non-custodial only）。
- **不实现完整链上索引器替代品**（仅聚焦 Program/IDL/Skill 相关语义层）。
- **不承诺支持所有非 Anchor Program 的 100% 语义恢复**（保留 fallback 与置信度标注）。
- **不在本 mission 中实现生产级执行引擎**（本 mission 以设计文档与参考骨架为主）。

## Key Decisions & Alternatives

| Decision | Chosen | Alternative | Trade-off |
|---|---|---|---|
| Product focus | AI-native RPC intelligence layer | 仅做传统 RPC 加速层 | 前者复杂度更高，但差异化与网络效应更强 |
| Capability exposure | SDK + NL Planner + MCP 三入口并行 | 只做 SDK | 三入口覆盖面更广，但接口治理成本上升 |
| Learning model | 实时 Geyser + 版本化 Registry | 定时离线抓取 | 实时性更强，但需要处理流式稳定性与回压 |
| Success criteria style | 量化 SLO/SLI 指标驱动 | 仅定性目标 | 可验收性更强，但需要更早建立观测体系 |

## Risks & Open Questions

- **Risk**: 非 Anchor Program 语义推断误差可能影响技能可用性。  
  **Mitigation**: 引入置信度分层、人工回填与回滚机制。
- **Risk**: LLM 成本与延迟在高峰时段不可控。  
  **Mitigation**: 本地模型前置过滤 + 云端按需升级 + 缓存。
- **Open Question**: 初期优先覆盖哪些协议垂类（DEX/借贷/质押）能最快验证 PMF？
- **Open Question**: 对企业客户是否需要私有化部署版本作为 Beta 阶段前置条件？

## References

- [Mission Overview](../../.factory/missions/138e8b5f-4b32-49a6-9988-bdfc12f0b405/mission.md)
- [Validation Contract A.F1](../../.factory/missions/138e8b5f-4b32-49a6-9988-bdfc12f0b405/validation-contract.md)
