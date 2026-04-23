# Design Documents Index

This index provides a fast reading path across the 16 AgentGeyser design docs. Start with vision and architecture (F1–F4), then move through core subsystems (F5–F9), API/data contracts (F10–F11), non-functional and operations design (F12–F14), and finish with skeleton and roadmap (F15–F16). The dependency graph captures declared `depends-on` relations from each doc frontmatter so you can follow prerequisite context before deep-diving a document.

## Document Index

| Doc | Title | Status | Path |
| --- | --- | --- | --- |
| F1 | Vision & Problem Statement | draft | [docs/design/01-vision.md](./01-vision.md) |
| F2 | Competitive Landscape & Positioning | draft | [docs/design/02-competitive-landscape.md](./02-competitive-landscape.md) |
| F3 | High-level Architecture & Data Flow | draft | [docs/design/03-architecture.md](./03-architecture.md) |
| F4 | Module Decomposition & Responsibility Boundaries | draft | [docs/design/04-modules.md](./04-modules.md) |
| F5 | IDL Registry & Continuous Learning Pipeline | draft | [docs/design/05-idl-registry.md](./05-idl-registry.md) |
| F6 | Skill Synthesizer (IDL → Semantic Skills) | draft | [docs/design/06-skill-synthesizer.md](./06-skill-synthesizer.md) |
| F7 | NL → Transaction Planner | draft | [docs/design/07-nl-planner.md](./07-nl-planner.md) |
| F8 | Dynamic TypeScript SDK 设计（Proxy Dispatch / Ambient Typings / Version Pinning / Offline Fallback） | draft | [docs/design/08-sdk.md](./08-sdk.md) |
| F9 | MCP Server 设计（Tools / Resources / Prompts, Auth, Rate Limits） | draft | [docs/design/09-mcp-server.md](./09-mcp-server.md) |
| F10 | 对外 API 规范（JSON-RPC ag_* / REST 管理面 / MCP Endpoint） | draft | [docs/design/10-api.md](./10-api.md) |
| F11 | 数据模型设计（ER / Postgres DDL / Redis Key-space） | draft | [docs/design/11-data-model.md](./11-data-model.md) |
| F12 | 性能、可扩展性与成本模型（QPS / Latency / LLM Cost / Scale Plan） | draft | [docs/design/12-performance-cost.md](./12-performance-cost.md) |
| F13 | 安全威胁模型（STRIDE / Non-Custodial Invariant / Audit & Compliance Hooks） | draft | [docs/design/13-security.md](./13-security.md) |
| F14 | Deployment & Observability（Compose/K8s 草案、Prometheus、OpenTelemetry、日志规范） | draft | [docs/design/14-deployment-observability.md](./14-deployment-observability.md) |
| F15 | Reference Skeleton Repository Structure | draft | [docs/design/15-skeleton.md](./15-skeleton.md) |
| F16 | 分阶段路线图与商业化（Spike → MVP → Beta → GA） | draft | [docs/design/16-roadmap.md](./16-roadmap.md) |

## Dependency Graph

```mermaid
flowchart LR
  F1["F1: Vision & Problem Statement"]
  F2["F2: Competitive Landscape & Positioning"]
  F3["F3: High-level Architecture & Data Flow"]
  F4["F4: Module Decomposition & Responsibility Boundaries"]
  F5["F5: IDL Registry & Continuous Learning Pipeline"]
  F6["F6: Skill Synthesizer (IDL → Semantic Skills)"]
  F7["F7: NL → Transaction Planner"]
  F8["F8: Dynamic TypeScript SDK 设计（Proxy Dispatch / Ambient Typings / Version Pinning / Offline Fallback）"]
  F9["F9: MCP Server 设计（Tools / Resources / Prompts, Auth, Rate Limits）"]
  F10["F10: 对外 API 规范（JSON-RPC ag_* / REST 管理面 / MCP Endpoint）"]
  F11["F11: 数据模型设计（ER / Postgres DDL / Redis Key-space）"]
  F12["F12: 性能、可扩展性与成本模型（QPS / Latency / LLM Cost / Scale Plan）"]
  F13["F13: 安全威胁模型（STRIDE / Non-Custodial Invariant / Audit & Compliance Hooks）"]
  F14["F14: Deployment & Observability（Compose/K8s 草案、Prometheus、OpenTelemetry、日志规范）"]
  F15["F15: Reference Skeleton Repository Structure"]
  F16["F16: 分阶段路线图与商业化（Spike → MVP → Beta → GA）"]
  F1 --> F2
  F1 --> F3
  F2 --> F3
  F3 --> F4
  F3 --> F5
  F4 --> F5
  F4 --> F6
  F5 --> F6
  F4 --> F7
  F6 --> F7
  F3 --> F8
  F4 --> F8
  F5 --> F8
  F6 --> F8
  F7 --> F8
  F3 --> F9
  F4 --> F9
  F6 --> F9
  F8 --> F9
  F4 --> F10
  F8 --> F10
  F9 --> F10
  F5 --> F11
  F6 --> F11
  F7 --> F11
  F10 --> F11
  F5 --> F12
  F7 --> F12
  F9 --> F12
  F10 --> F12
  F11 --> F12
  F4 --> F13
  F5 --> F13
  F7 --> F13
  F9 --> F13
  F10 --> F13
  F11 --> F13
  F3 --> F14
  F4 --> F14
  F10 --> F14
  F11 --> F14
  F12 --> F14
  F13 --> F14
  F3 --> F15
  F4 --> F15
  F1 --> F16
  F2 --> F16
  F3 --> F16
  F4 --> F16
  F10 --> F16
  F12 --> F16
  F13 --> F16
  F14 --> F16
  F15 --> F16
```

<!-- assertion-evidence: X.1: this file exists and links all 16 docs -->
