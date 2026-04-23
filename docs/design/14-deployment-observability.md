---
doc: F14
title: Deployment & Observability（Compose/K8s 草案、Prometheus、OpenTelemetry、日志规范）
owner: AgentGeyser Core
status: draft
depends-on: [F3, F4, F10, F11, F12, F13]
updated: 2026-04-23
---

## Goals

- 定义 AgentGeyser 在开发/预发/生产环境的部署基线：Docker Compose 草案 + Kubernetes manifest sketch。
- 建立统一可观测性标准：Prometheus 指标命名、OpenTelemetry span 约定、结构化日志 schema。
- 将运行时可用性、性能与安全事件串联到统一 telemetry 语义，支撑 SLO、排障与审计。

## Non-Goals

- 不在本文提供可直接上线的生产级 IaC（如完整 Helm chart、Terraform module）。
- 不覆盖业务逻辑实现细节（IDL 学习、NL 规划算法见 F5/F7）。
- 不替代安全策略文档（见 [F13 安全威胁模型](./13-security.md)）。

## Context

本文件 fulfills `E.F14.1`、`E.F14.2`、`E.F14.3`、`E.F14.4`。  
命名与模块遵循 [F4 模块设计](./04-modules.md)：`IdlRegistry`、`SkillSynthesizer`、`NlPlanner`、`McpServer`、`RpcPassthrough`、`AuthQuota`。  
API 面与实体名称遵循 [F10 API](./10-api.md) 与 [F11 Data Model](./11-data-model.md)。

## Design

### Deployment Topology Overview

```mermaid
flowchart LR
  subgraph Edge[Edge]
    LB[Ingress / API Gateway]
  end

  subgraph App[AgentGeyser Runtime]
    Proxy[agentgeyser-proxy\n(Axum + tonic)]
    Worker[idl-sync-worker]
  end

  subgraph Data[Stateful Services]
    Redis[(Redis)]
    Postgres[(Postgres)]
  end

  subgraph Obs[Observability Stack]
    Prom[Prometheus]
    OTelCol[OpenTelemetry Collector]
    Loki[Loki / Log Backend]
    Jaeger[Tempo/Jaeger]
  end

  subgraph Upstream[External Dependencies]
    Geyser[Yellowstone gRPC]
    SolRpc[Solana RPC Vendors]
    LLM[LLM Providers]
  end

  LB --> Proxy
  Worker --> Geyser
  Proxy --> SolRpc
  Proxy --> LLM
  Proxy --> Redis
  Proxy --> Postgres
  Worker --> Redis
  Worker --> Postgres
  Proxy --> OTelCol
  Worker --> OTelCol
  OTelCol --> Prom
  OTelCol --> Jaeger
  OTelCol --> Loki
```

### 1) Docker Compose Draft（Local/CI Smoke）

> 目标：为本地开发与集成测试提供最小可运行编排；不包含生产 HA 配置。

```yaml
version: "3.9"

services:
  proxy:
    image: ghcr.io/agentgeyser/proxy:dev
    container_name: ag-proxy
    ports:
      - "8080:8080"   # JSON-RPC / REST / MCP-over-HTTP
      - "9464:9464"   # Prometheus scrape endpoint
    environment:
      AG_ENV: "dev"
      AG_HTTP_ADDR: "0.0.0.0:8080"
      AG_METRICS_ADDR: "0.0.0.0:9464"
      AG_LOG_FORMAT: "json"
      AG_REDIS_URL: "redis://redis:6379/0"
      AG_DATABASE_URL: "postgres://ag:ag@postgres:5432/agentgeyser"
      AG_OTEL_EXPORTER_OTLP_ENDPOINT: "http://otel-collector:4317"
    depends_on:
      - redis
      - postgres
      - otel-collector
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://localhost:8080/healthz"]
      interval: 10s
      timeout: 3s
      retries: 6

  idl-sync-worker:
    image: ghcr.io/agentgeyser/idl-sync:dev
    container_name: ag-idl-sync
    environment:
      AG_ENV: "dev"
      AG_REDIS_URL: "redis://redis:6379/0"
      AG_DATABASE_URL: "postgres://ag:ag@postgres:5432/agentgeyser"
      AG_OTEL_EXPORTER_OTLP_ENDPOINT: "http://otel-collector:4317"
    depends_on:
      - redis
      - postgres
      - otel-collector

  redis:
    image: redis:7-alpine
    container_name: ag-redis
    ports:
      - "6379:6379"
    command: ["redis-server", "--save", "", "--appendonly", "no"]

  postgres:
    image: postgres:16-alpine
    container_name: ag-postgres
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: "ag"
      POSTGRES_PASSWORD: "ag"
      POSTGRES_DB: "agentgeyser"
    volumes:
      - pgdata:/var/lib/postgresql/data

  otel-collector:
    image: otel/opentelemetry-collector:0.102.1
    container_name: ag-otel-collector
    command: ["--config=/etc/otelcol/config.yaml"]
    ports:
      - "4317:4317"   # OTLP gRPC
      - "4318:4318"   # OTLP HTTP
    volumes:
      - ./deploy/otel-collector-config.yaml:/etc/otelcol/config.yaml:ro

  prometheus:
    image: prom/prometheus:v2.54.1
    container_name: ag-prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./deploy/prometheus.yml:/etc/prometheus/prometheus.yml:ro
    depends_on:
      - proxy
      - otel-collector

volumes:
  pgdata: {}
```

### 2) Kubernetes Manifest Sketch（Staging/Prod）

> 目标：展示生产编排意图（分层、探针、资源、滚动升级）；具体值由 Helm/Kustomize 环境覆盖。

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: agentgeyser-proxy
  labels:
    app.kubernetes.io/name: agentgeyser-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: agentgeyser-proxy
  template:
    metadata:
      labels:
        app: agentgeyser-proxy
    spec:
      containers:
        - name: proxy
          image: ghcr.io/agentgeyser/proxy:stable
          ports:
            - containerPort: 8080
            - containerPort: 9464
          env:
            - name: AG_ENV
              value: "prod"
            - name: AG_OTEL_EXPORTER_OTLP_ENDPOINT
              value: "http://otel-collector.observability.svc.cluster.local:4317"
          envFrom:
            - secretRef:
                name: agentgeyser-runtime-secrets
            - configMapRef:
                name: agentgeyser-runtime-config
          readinessProbe:
            httpGet:
              path: /readyz
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 10
          livenessProbe:
            httpGet:
              path: /healthz
              port: 8080
            initialDelaySeconds: 15
            periodSeconds: 15
          resources:
            requests:
              cpu: "500m"
              memory: "512Mi"
            limits:
              cpu: "2"
              memory: "2Gi"
---
apiVersion: v1
kind: Service
metadata:
  name: agentgeyser-proxy
spec:
  selector:
    app: agentgeyser-proxy
  ports:
    - name: http
      port: 80
      targetPort: 8080
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: agentgeyser-proxy-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: agentgeyser-proxy
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

#### K8s 配套建议（简述）

- `IdlRegistry` worker 建议单独 Deployment，使用 PDB + anti-affinity，避免与 `proxy` 抢占 CPU。
- Redis/Postgres 生产建议托管服务（Managed）或独立 StatefulSet；避免与计算平面同节点。
- 通过 `ServiceMonitor`（Prometheus Operator）抓取 `:9464/metrics`。
- 通过 `OpenTelemetryCollector` CRD 将 traces/logs/metrics 统一转发到后端。

### 3) Prometheus Metrics Conventions（E.F14.3）

命名约定：`agentgeyser_<subsystem>_<metric>_<unit?>`，标签控制在低基数（tenant_id 禁止直接打到高频指标）。

| Metric Name | Type | Labels | Description |
|---|---|---|---|
| `agentgeyser_http_requests_total` | counter | `route`, `method`, `status_class` | 全入口请求计数 |
| `agentgeyser_http_request_duration_seconds` | histogram | `route`, `method` | HTTP 请求时延分布 |
| `agentgeyser_rpc_upstream_requests_total` | counter | `provider`, `rpc_method`, `result` | 上游 Solana RPC 调用量 |
| `agentgeyser_rpc_upstream_latency_seconds` | histogram | `provider`, `rpc_method` | 上游 RPC 时延 |
| `agentgeyser_auth_quota_rejections_total` | counter | `reason`, `surface` | `AuthQuota` 拒绝计数（401/403/429） |
| `agentgeyser_skill_invocations_total` | counter | `skill_name`, `version`, `result` | `ag_invokeSkill` 调用结果 |
| `agentgeyser_nl_plans_total` | counter | `planner_model`, `result` | `ag_planNL` 规划请求计数 |
| `agentgeyser_nl_plan_duration_seconds` | histogram | `planner_model` | NL 规划总耗时 |
| `agentgeyser_idl_updates_total` | counter | `source`, `outcome` | `IdlRegistry` IDL 更新计数 |
| `agentgeyser_cache_hit_ratio` | gauge | `cache_name` | Redis 热缓存命中率 |
| `agentgeyser_queue_depth` | gauge | `queue` | 内部异步队列深度（worker/backpressure） |
| `agentgeyser_otel_export_failures_total` | counter | `signal`, `backend` | OTel 导出失败计数 |

Histogram buckets 建议：

- `http_request_duration_seconds`: `0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2, 5`
- `nl_plan_duration_seconds`: `0.1, 0.25, 0.5, 1, 2, 4, 8, 15`
- `rpc_upstream_latency_seconds`: `0.01, 0.03, 0.1, 0.3, 0.8, 2, 5`

### 4) OpenTelemetry Span Conventions（E.F14.3）

Trace 命名采用 `agentgeyser.<surface>.<operation>`：

| Span Name | Kind | Key Attributes |
|---|---|---|
| `agentgeyser.http.request` | SERVER | `http.method`, `http.route`, `user_agent.original`, `enduser.id?` |
| `agentgeyser.auth.authorize` | INTERNAL | `auth.surface`, `auth.result`, `tenant.tier` |
| `agentgeyser.rpc.call` | CLIENT | `rpc.system=solana-jsonrpc`, `rpc.method`, `net.peer.name` |
| `agentgeyser.idl.ingest_event` | CONSUMER | `program.id`, `slot`, `idl.source`, `idl.version` |
| `agentgeyser.skill.synthesize` | INTERNAL | `program.id`, `skill.count`, `synth.model` |
| `agentgeyser.nl.plan` | INTERNAL | `planner.model`, `skill.candidates`, `risk.flags.count` |
| `agentgeyser.tx.simulate` | CLIENT | `solana.cluster`, `tx.account_count`, `tx.compute_units_est` |
| `agentgeyser.tx.broadcast` | CLIENT | `provider`, `tx.signature?`, `retry.count` |
| `agentgeyser.auditlog.persist` | INTERNAL | `entity=AuditLog`, `write.result` |

Span 传播要求：

1. 入口强制生成/继承 `trace_id`（W3C TraceContext）。
2. `Invocation` 与 `AuditLog` 必须保存 `trace_id` / `span_id` 关联字段，便于跨系统追踪。
3. 对 LLM 调用增加 `llm.vendor`, `llm.model`, `llm.prompt_tokens`, `llm.completion_tokens`（避免记录原始 prompt）。

### 5) Structured Log Schema（E.F14.4）

日志格式：单行 JSON，UTC ISO-8601 时间，禁止多行堆叠字段（异常堆栈可放 `error.stack` 字段文本）。

#### 5.1 Required Fields

| Field | Type | Required | Notes |
|---|---|---|---|
| `timestamp` | string | yes | RFC3339 UTC |
| `level` | string | yes | `trace`/`debug`/`info`/`warn`/`error`/`fatal` |
| `service` | string | yes | `agentgeyser-proxy` / `idl-sync-worker` |
| `environment` | string | yes | `dev`/`staging`/`prod` |
| `message` | string | yes | 人类可读摘要 |
| `trace_id` | string | yes | 与 OTel 一致 |
| `span_id` | string | yes | 与 OTel 一致 |
| `request_id` | string | conditional | 有外部请求时必填 |
| `tenant_id` | string | conditional | 经脱敏/映射后的租户 ID |
| `module` | string | yes | canonical 模块名 |
| `event` | string | yes | 事件名（如 `rpc.call.completed`） |
| `outcome` | string | yes | `success`/`failure`/`denied` |
| `duration_ms` | number | recommended | 操作耗时 |
| `error.code` | string | conditional | 失败时必填 |
| `error.message` | string | conditional | 失败时必填，脱敏 |
| `security.flags` | array | optional | `prompt_injection_suspected` 等 |

#### 5.2 Log Levels Policy

- `info`: 生命周期事件、成功关键路径（请求结束、任务完成）。
- `warn`: 可恢复异常、策略拒绝、上游抖动重试。
- `error`: 请求失败或数据写入失败，需要告警。
- `debug/trace`: 仅在临时排障开启，生产默认关闭或采样。

#### 5.3 Example Log Event

```json
{
  "timestamp": "2026-04-23T16:42:10.120Z",
  "level": "warn",
  "service": "agentgeyser-proxy",
  "environment": "staging",
  "message": "NL plan denied by policy post-check",
  "trace_id": "4f8a1e55b229ce6d93fd958f6bf7f85e",
  "span_id": "6c1f5a55b03c2d18",
  "request_id": "req_01J8Y6M3Q6H6",
  "tenant_id": "tnt_a12f",
  "module": "NlPlanner",
  "event": "nl.plan.policy_denied",
  "outcome": "denied",
  "duration_ms": 184,
  "error.code": "POLICY_DENY_HIGH_RISK_ROUTE",
  "error.message": "route requires restricted tool scope",
  "security.flags": ["prompt_injection_suspected"]
}
```

### 6) Operational SLO & Alert Hints

- Availability SLO: `ag_*` 入口月可用性 ≥ 99.9%。
- p95 latency（参考 F12）：
  - `ag_listSkills` < 150ms
  - `ag_getIdl` < 250ms
  - `ag_invokeSkill` < 1200ms（不含链上最终确认）
  - `ag_planNL` < 2500ms（命中缓存时）
- 告警建议：
  - `agentgeyser_http_request_duration_seconds{route="/ag_planNL"} p95 > 3s` 持续 10 分钟
  - `agentgeyser_otel_export_failures_total` 5 分钟增量 > 50
  - `agentgeyser_auth_quota_rejections_total{reason="unexpected"} > baseline x3`

## Key Decisions & Alternatives

| Decision | Chosen | Alternatives | Trade-offs |
|---|---|---|---|
| Runtime deployment split | `proxy` 与 `idl-sync-worker` 拆分部署 | 单进程 all-in-one | 拆分利于扩缩容与故障隔离；运维对象更多 |
| Observability pipeline | OTel Collector 统一汇聚 metrics/traces/logs | 各信号直接写后端 | Collector 可解耦后端与采样策略；增加一个组件 |
| Metrics labels | 低基数标签优先 | 细粒度高基数标签（含 wallet/tenant） | 保持 Prom 成本可控；细节排障需借助日志/trace |
| Log format | 强制 JSON 结构化日志 | 纯文本日志 | 机器可检索性更强；人工阅读门槛略高 |
| K8s scaling signal | HPA 基于 CPU +（后续）自定义指标 | 固定副本数 | 更好应对波峰；自动扩缩容策略更复杂 |

## Risks & Open Questions

- **Risk**：高峰期 NL planning traces 体量大，可能压垮 telemetry backend。  
  **Mitigation**：对低价值 spans 采样；保留 error spans 全量。
- **Risk**：指标与日志字段命名漂移导致跨团队 dashboard 失效。  
  **Mitigation**：将命名约定纳入 CI lint（schema check）。
- **Open Question**：生产环境是否统一采用 Tempo 还是 Jaeger 作为 trace backend？
- **Open Question**：是否需要将安全事件日志单独路由至 SIEM（如 Datadog/Splunk）并延长保留期？

## References

- [F3 High-level Architecture](./03-architecture.md)
- [F4 Modules](./04-modules.md)
- [F10 External API](./10-api.md)
- [F11 Data Model](./11-data-model.md)
- [F12 Performance/Cost](./12-performance-cost.md)
- [F13 Security](./13-security.md)
- [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)
- [Prometheus Naming Best Practices](https://prometheus.io/docs/practices/naming/)
- [Kubernetes Deployment](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/)

<!--
assertion-evidence:
  E.F14.1: frontmatter at top of file includes doc/title/owner/status/depends-on/updated.
  E.F14.2: section "1) Docker Compose Draft" provides compose YAML; section "2) Kubernetes Manifest Sketch" provides Deployment/Service/HPA YAML.
  E.F14.3: section "3) Prometheus Metrics Conventions" enumerates concrete metric names; section "4) OpenTelemetry Span Conventions" defines span naming and attributes.
  E.F14.4: section "5) Structured Log Schema" documents JSON fields, requiredness, levels policy, and example log event.
-->
