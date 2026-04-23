---
doc: F15
title: Reference Skeleton Repository Structure
owner: AgentGeyser Core
status: draft
depends-on: [F3, F4]
updated: 2026-04-23
---

## Goals

定义 AgentGeyser 参考骨架目录，使 Rust workspace、pnpm workspace、CI stub 一次性就位，支持后续 Sprint 实现。

## Non-Goals

不实现任何业务逻辑，不连接外部链上服务，不在本 feature 中执行 CI。

## Context

本结构承接 [F3 Architecture](./03-architecture.md) 与 [F4 Modules](./04-modules.md) 的模块边界。

## Design

`skeleton/` 采用 monorepo：Rust crates 承载服务端模块，`packages/` 提供 SDK 与 MCP client stub，`.github/workflows/ci.yml` 提供静态 CI 骨架。

```text
skeleton/
├── Cargo.toml
├── crates/{proxy,idl-registry,skill-synth,nl-planner,mcp-server}
├── pnpm-workspace.yaml
├── packages/{sdk,mcp-client}
└── .github/workflows/ci.yml
```

## Key Decisions & Alternatives

| Decision | Alternatives | Trade-off |
| --- | --- | --- |
| 单仓管理 Rust+TS | 分仓管理 | 单仓更利于跨模块改动，但 CI 需双工具链 |
| crate 名称与模块一一对应 | 聚合为少量 crate | 一一对应清晰，早期样板文件更多 |
| CI 仅保留 `cargo check` + `pnpm -r build` | 完整测试矩阵 | 当前阶段聚焦骨架，可后续扩展 |

## Risks & Open Questions

- 风险：缺少 `tsconfig.json` 会导致 `pnpm -r build` 在真实执行时失败；当前为 stub 可接受。
- 开放问题：Sprint 1 是否引入 shared TS config package。

## References

- [Rust Cargo Workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [pnpm Workspace](https://pnpm.io/workspaces)
- [GitHub Actions Workflow syntax](https://docs.github.com/actions/using-workflows/workflow-syntax-for-github-actions)

## Local Bootstrap

```bash
cd skeleton
cargo build --workspace
pnpm install
pnpm -r build
```

以上命令用于本地拉起工具链验证；本 mission 不要求执行。
