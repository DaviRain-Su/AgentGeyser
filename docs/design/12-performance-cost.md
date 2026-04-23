---
doc: F12
title: 性能、可扩展性与成本模型（QPS / Latency / LLM Cost / Scale Plan）
owner: AgentGeyser Core
status: draft
depends-on: [F5, F7, F9, F10, F11]
updated: 2026-04-23
---

## Goals

- 定义 AgentGeyser 在核心入口上的性能目标：吞吐（QPS）与延迟（p50/p95）。
- 建立可执行的 LLM 成本模型：按请求类型估算 token 与美元成本，并明确缓存命中率目标。
- 给出水平扩展方案：`stateless proxy` + `sharded registry`，覆盖扩容触发条件与容量规划。
- 将上游 Geyser 订阅成本（Helius/Triton/Quicknode Yellowstone）纳入总成本视角，形成可运营的 unit economics。

## Non-Goals

- 不在本文件实现压测脚本或生产自动扩缩容策略代码。
- 不替代 [F14 Deployment & Observability](./14-deployment-observability.md) 的部署细节与监控仪表盘定义。
- 不绑定某一云厂商 SKU 价格；价格以区间与公式建模，便于后续按采购合同替换。
- 不扩展新的 public API 方法名，仅覆盖 `ag_listSkills`、`ag_getIdl`、`ag_invokeSkill`、`ag_planNL`。

## Context

本设计与以下文档一致并互相约束：

- [F10 API](./10-api.md)：定义四个 canonical `ag_*` 方法，是性能预算分配的入口基准。
- [F11 Data Model](./11-data-model.md)：`Program` / `Idl` / `Skill` / `SkillVersion` / `Invocation` / `AuditLog` 的存储与缓存路径决定延迟构成。
- [F5 IDL Registry](./05-idl-registry.md)：Geyser 订阅与 IDL 版本刷新产生持续后台负载与外部订阅费用。
- [F7 NL Planner](./07-nl-planner.md)：`ag_planNL` 的 LLM 调用链是主要可变成本来源。
- [F9 MCP Server](./09-mcp-server.md)：MCP 工具流量与 JSON-RPC 共享后端容量池。

关键假设（用于容量与成本估算）：

1. 读请求（`ag_listSkills` + `ag_getIdl`）占 70–85%。
2. 写/执行请求（`ag_invokeSkill`）占 10–25%，其中 dry-run 比例约 60%。
3. 规划请求（`ag_planNL`）占 3–10%，但单位成本最高。
4. Redis 命中率是控制 p95 与单位成本的第一杠杆。

## Design

### 1) SLO 与性能预算（E.F12.2）

#### 1.1 Endpoint QPS Targets（单区域，steady-state）

| Endpoint | p50 latency target | p95 latency target | Target QPS | Burst QPS (60s) | Error budget |
|---|---:|---:|---:|---:|---:|
| `ag_listSkills` | ≤ 35 ms | ≤ 120 ms | 600 | 1,200 | 0.5% / 30d |
| `ag_getIdl` | ≤ 30 ms | ≤ 100 ms | 500 | 1,000 | 0.5% / 30d |
| `ag_invokeSkill` (dry-run) | ≤ 120 ms | ≤ 350 ms | 180 | 320 | 1.0% / 30d |
| `ag_invokeSkill` (submit) | ≤ 220 ms* | ≤ 700 ms* | 90 | 180 | 1.5% / 30d |
| `ag_planNL` | ≤ 450 ms | ≤ 1,500 ms | 60 | 120 | 2.0% / 30d |

\* `submit` 延迟不含最终链上确认时间，仅指 API 接收、构建、预检、提交上游 RPC 的 server-side 完成时延。

#### 1.2 端到端延迟分解（目标 p95）

| Component | `ag_listSkills` | `ag_getIdl` | `ag_invokeSkill` (dry-run) | `ag_planNL` |
|---|---:|---:|---:|---:|
| Edge/AuthQuota | 15 ms | 15 ms | 20 ms | 20 ms |
| Redis read/write | 20 ms | 20 ms | 30 ms | 25 ms |
| Postgres fallback | 45 ms | 40 ms | 70 ms | 40 ms |
| Planner/LLM | 0 ms | 0 ms | 0–30 ms (rule checks) | 1,150 ms |
| Upstream RPC/Geyser side effects | 0 ms | 0 ms | 180 ms | 120 ms |
| **Total p95 budget** | **120 ms** | **100 ms** | **350 ms** | **1,500 ms** |

### 2) LLM 成本模型与缓存目标（E.F12.3）

#### 2.1 请求分类与 token 预算

| Workload | Model tier | Avg input tokens | Avg output tokens | Token cache hit target |
|---|---|---:|---:|---:|
| IDL semantic tagging（F5/F6 后台） | local-small first, cloud fallback | 1,200 | 250 | ≥ 70%（by `schema_hash`） |
| `ag_planNL` 标准请求 | cloud reasoning | 1,000 | 300 | ≥ 55%（prompt prefix + few-shot） |
| `ag_planNL` 复杂多步请求 | cloud reasoning high | 2,200 | 700 | ≥ 40% |
| 安全解释/失败复盘文本 | local-small | 500 | 180 | ≥ 65% |

#### 2.2 单请求成本公式

定义：

- `Cin` = 输入 token 单价（$/1M token）
- `Cout` = 输出 token 单价（$/1M token）
- `Tin` = 实际输入 token 数
- `Tout` = 实际输出 token 数
- `H` = 缓存命中率（0~1，命中后只计检索/轻量校验成本）
- `Ccache` = 缓存命中时固定成本（默认 $0.00002/request）

则期望单请求 LLM 成本：

`E[cost] = (1 - H) * ((Tin * Cin + Tout * Cout) / 1,000,000) + H * Ccache`

#### 2.3 参考成本区间（2026 Q2 假设价）

> 仅为规划区间，实际由采购合同替换；目标是保证单位经济性可推演。

- cloud reasoning tier: `Cin=$2.0/M`, `Cout=$8.0/M`
- local-small 自托管折算：`$0.20~$0.45/M tokens`（含算力摊销）

示例：

1. `ag_planNL` 标准请求（Tin=1000, Tout=300, H=0.55）  
   `E[cost] ≈ (0.45 * (1000*2 + 300*8)/1e6) + 0.55*0.00002 ≈ $0.0020/request`
2. 复杂请求（Tin=2200, Tout=700, H=0.40）  
   `E[cost] ≈ $0.0054/request`
3. IDL 后台标注（local-first，cloud fallback 15%）  
   期望成本控制在 `$0.0004 ~ $0.0012 / idl_version`

#### 2.4 缓存命中率与成本控制目标

- **Prompt 前缀缓存命中率**：`ag_planNL` ≥ 55%（30d rolling）。
- **Schema-hash 语义标注命中率**：IDL tagging ≥ 70%。
- **成本守卫线**：全量流量下 LLM blended cost ≤ **$0.0018/request**（按总请求数加权）。
- **降级策略触发**：若 1h 窗口 blended cost > $0.0032/request，自动：
  1) 提升 local-small 路由比例；
  2) 缩短上下文、减少 few-shot；
  3) 对 free tier 开启 `plan` 排队或限额收紧。

### 3) Horizontal Scale Plan（E.F12.4）

#### 3.1 架构原则：Stateless Proxy + Sharded Registry

- `stateless proxy`：API 层（含 `RpcPassthrough`、`McpServer` 入口适配、`AuthQuota` 校验）不保存会话持久状态，支持无损横向扩容。
- `sharded registry`：`IdlRegistry` 与 `SkillSynthesizer` 的热数据按 `program_id` 一致性哈希分片，后台消费组与存储分区对齐。
- `NlPlanner`：独立 worker pool（CPU-bound + LLM I/O），与 API front door 解耦，防止长尾请求拖垮读路径。

#### 3.2 扩缩容触发器

| Layer | Scale-out trigger (5 min window) | Scale-in trigger (30 min window) |
|---|---|---|
| API Proxy pods | CPU > 65% 或 p95 > SLO*1.15 或 QPS > 80% capacity | CPU < 35% 且 p95 < SLO*0.8 |
| Planner workers | queue depth > 200 或 `ag_planNL` p95 > 1.5s | queue depth < 40 且 p95 < 1.0s |
| Registry shard workers | Geyser lag > 10s 或 shard backlog > 50k msgs | lag < 2s 且 backlog < 10k |
| Redis cluster | used_memory > 75% 或 evicted_keys > 0 sustained | used_memory < 55% |
| Postgres read replicas | read CPU > 70% / replica lag > 2s | read CPU < 45% / lag < 500ms |

#### 3.3 分片策略

- 主分片键：`program_id`（Pubkey hash）。
- 初始分片数：16（可在线扩展至 32/64，采用虚拟节点减轻重分布抖动）。
- 热点缓解：对超热点 Program 启用“二级桶” `hash(program_id + skill_name)`。
- 数据一致性：`Idl` 与 `SkillVersion` 写路径要求同分片顺序提交（避免版本倒挂）。

#### 3.4 多区域策略（阶段性）

1. **Phase A**：单主区域 + 只读边缘缓存（最低复杂度）。
2. **Phase B**：双活 API + 单写 registry（通过队列回传统一写入）。
3. **Phase C**：按租户/地理分区多主（需全局 ID 和冲突策略）。

### 4) External Subscription Cost（Helius/Triton/Quicknode）

#### 4.1 成本构成

1. **基础订阅费**：Yellowstone gRPC 通道与消息额度。
2. **超额消息费**：按事件条数/数据量计费（不同供应商口径不同）。
3. **网络出口费**：跨区拉流或复制导致的 egress。
4. **冗余链路费**：高可用下多供应商并行订阅（active-active 或 active-standby）。

#### 4.2 预算模型（月度）

`Monthly_Streaming_Cost = Base_Subscription + Overage_Events + CrossRegion_Egress + Redundancy_Premium`

规划守卫线（初期）：

- Base + overage 合计目标：`$3k ~ $12k / month`（取决于网络、事件密度与保留策略）。
- 冗余溢价控制：`Redundancy_Premium <= 40%`（相对单供应商基线）。
- 成本/吞吐指标：`$ per 1M events` 与 `$ per active program` 双维度跟踪。

#### 4.3 供应商策略

- **主供应商 + 备供应商**：默认单主拉流，故障切换到备，避免长期双活双计费。
- **关键窗口双拉流**：仅在升级/高风险期间启用短期双活验证。
- **契约条款建议**：争取 burst 额度、超额阶梯价、故障补偿 SLA。

## Key Decisions & Alternatives

| Decision | Chosen | Alternatives | Trade-offs |
|---|---|---|---|
| API 架构 | Stateless proxy | Sticky sessions | Stateless 更易水平扩展；需外置状态与更强缓存治理 |
| Registry 扩展 | `program_id` 分片 | 按租户分片 | program 分片更贴合链上事件分布；跨租户查询聚合成本更高 |
| Planner 资源池 | 独立 worker pool | 与 API 进程混部 | 隔离长尾与成本波动；部署与调度更复杂 |
| LLM 成本控制 | 命中率目标 + 守卫线降级 | 固定模型固定上下文 | 更灵活可控；需要精细观测与动态策略 |
| Geyser 供应商 | 主备策略 | 长期多供应商双活 | 主备成本更优；切换演练要求更高 |

## Risks & Open Questions

- **黑天鹅流量突发**：链上热点事件可使 `ag_listSkills` 和 registry backlog 同时飙升。  
  - 缓解：预留 2x burst 容量 + 紧急限流策略。  
  - Owner: Runtime/SRE
- **LLM 价格波动**：云端模型调价会直接影响毛利。  
  - 缓解：多模型路由 + 本地模型 fallback + 合同锁价。  
  - Owner: Platform Economics
- **缓存污染导致错误命中**：错误 schema 缓存会放大调用失败。  
  - 缓解：`schema_hash` 强校验 + 版本回滚熔断。  
  - Owner: IdlRegistry + SkillSynthesizer
- **供应商事件口径差异**：Helius/Triton/Quicknode 的计费维度不同，预算对比失真。  
  - 缓解：统一内部事件计量标准（normalized events）。  
  - Owner: FinOps

## References

- [F5 IDL Registry](./05-idl-registry.md)
- [F7 NL Planner](./07-nl-planner.md)
- [F9 MCP Server](./09-mcp-server.md)
- [F10 API](./10-api.md)
- [F11 Data Model](./11-data-model.md)
- [Solana Transaction Confirmation & Expiration](https://solana.com/docs/advanced/confirmation)
- [Redis Eviction Policies](https://redis.io/docs/latest/develop/reference/eviction/)

<!--
assertion-evidence:
  E.F12.1: file exists at docs/design/12-performance-cost.md with required frontmatter fields (doc/title/owner/status/depends-on/updated).
  E.F12.2: section "1) SLO 与性能预算" provides explicit QPS targets and p50/p95 latency budgets per canonical endpoint.
  E.F12.3: section "2) LLM 成本模型与缓存目标" defines per-request cost formula, request-class estimates, and cache hit-rate targets.
  E.F12.4: section "3) Horizontal Scale Plan" specifies stateless proxy plus sharded registry architecture with scaling triggers and shard strategy.
-->
