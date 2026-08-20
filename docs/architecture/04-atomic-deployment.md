# 原子化部署・中心事件管理・App 集群・热插拔架构总论

> **ドキュメントID**：DOC-ARCH-005
> **文書分類**：横断文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/00-anatomy-model.md`（DOC-ARCH-001）
> **下位文書**：
>   - `docs/architecture/05-admin-operations-ui.md`（DOC-ARCH-006）
>   - `docs/modules/M-14-module-registry.md`（DOC-MOD-014）
>   - `docs/modules/M-15-central-event-bus.md`（DOC-MOD-015）
>   - `docs/modules/M-16-cluster-coordinator.md`（DOC-MOD-016）
>   - `docs/api/admin-modules.md`（DOC-API-004）
>   - `docs/api/admin-events.md`（DOC-API-005）
>   - `docs/api/admin-cluster.md`（DOC-API-006）
> **関連文書**：`docs/modules/M-06`（DOC-MOD-006）、`docs/modules/M-10`（DOC-MOD-010）、`docs/modules/M-13`（DOC-MOD-013）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章「システム開発プロセス」・第 7 章「運用・保守プロセス」
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（4 大能力 + 集成 + PL/pgSQL 存过方針） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 背景与目的
2. 4 大能力定义
3. 总体架构
4. 原子化部署
5. 中心事件管理
6. App 集群协调
7. 热插拔协议
8. 4 者关系与协调
9. 管理员运维界面
10. API 与存储设计原则
11. PL/pgSQL 存过策略
12. 验收硬指标
13. 用語集
14. 参考文献

---

## 1. 背景与目的

现有 M-01~M-13 模块设计是**单体能力视角**：每个模块自身完成"采集/转换/编排/导出"等独立功能，但**缺乏**：

- **原子化部署能力**：当前所有模块捆绑在同一个 Runtime 二进制中，无法单独升级某个模块（如只升级 F-02 适配器而保留 M-04 编排引擎不变）。
- **中心事件总线**：现有 M-03 数据流引擎是**画布内**数据通道，跨模块/跨集群的**系统级**事件（部署、配置变更、健康告警）无统一通道。
- **App 集群协调**：M-13 API Gateway 假设单机部署，多副本时缺乏服务发现、领导选举、状态分片机制。
- **热插拔**：M-06 插件 SDK 支持加载，但**模块级**（不只是单个插件）的启停、版本切换、流量灰度缺失。
- **管理员运维界面**：前端 M-12 是面向业务用户（画布设计者）的，缺面向 SRE/平台管理员的可视化运维入口。

**目的**：补充上述 4 大能力 + 1 个管理面，使每个模块可独立升级、系统级事件可观测、集群可水平扩展、模块可热插拔、运维可可视化。

## 2. 4 大能力定义

| 能力 | 英文 | 定义 | 涉及新模块 |
|---|---|---|---|
| 原子化部署 | Atomic Deployment | 每个模块可独立打包、版本化、灰度、升级、回滚 | M-14 |
| 中心事件管理 | Central Event Bus | 跨模块/跨节点的统一事件发布/订阅/持久化/重放 | M-15 |
| App 集群协调 | App Cluster Coordination | 多 Runtime 实例的服务发现、领导选举、心跳、状态分片 | M-16 |
| 热插拔 | Hot-Swap | 模块可在不中断其他模块运行的前提下加载/卸载/切换版本 | M-14 + M-06 扩展 |

## 3. 总体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                     管理平面（Admin Plane）                          │
│  Admin Ops UI（DOC-ARCH-006）                                       │
│  - 模块列表 / 版本 / 健康 / 灰度                                    │
│  - 事件订阅 / 重放 / 审计                                            │
│  - 集群拓扑 / 领导 / 心跳                                            │
└──────────────────────────────┬───────────────────────────────────────┘
                               │ HTTPS / WSS
                               │ （admin-* 命名空间）
┌──────────────────────────────▼───────────────────────────────────────┐
│                     API Gateway（M-13 增强）                        │
│  - 业务路由（/api/v1/*）→ 现有 13 模块                               │
│  - 管理路由（/api/v1/admin/*）→ M-14/M-15/M-16                       │
│  - 模块路由表（registry-aware routing）                              │
└──┬────────────────┬─────────────────┬─────────────────┬─────────────┘
   │                │                 │                 │
┌──▼─────────┐ ┌────▼──────┐  ┌────────▼────────┐  ┌────▼────────┐
│ M-14 模块  │ │ M-15 中心 │  │ M-16 集群       │  │ 现有 M-01~M-13│
│ 注册与生命 │ │ 事件总线  │  │ 协调器         │  │ 业务模块      │
│ 周期       │ │           │  │                 │  │              │
└──────┬─────┘ └─────┬─────┘  └────────┬────────┘  └──────┬───────┘
       │             │                 │                 │
       │    ┌────────┴────────┐ ┌───────┴────────┐  ┌─────┴─────────┐
       │    │ Event Log 表   │ │ cluster_node   │  │ 现有 13 模块  │
       │    │ + 持久化队列   │ │ leader_lease   │  │ 业务表        │
       │    └────────────────┘ │ module_registry│  └───────────────┘
       │                       └─────────────────┘
       │
┌──────▼──────────────────────────────────────────────────────────────┐
│                       PostgreSQL + 共享存储                         │
│  RLS 多租户隔离（M-10）+ 新增表（见 §10）                          │
│  + PL/pgSQL 存过（register_module / atomic_swap / append_event）  │
└─────────────────────────────────────────────────────────────────────┘
```

## 4. 原子化部署

### 4.1 模块清单（Module Manifest）

每个模块以 `Module.toml` 形式声明自身元数据，存储于 `module_registry` 表：

```toml
# Module.toml 示例
[meta]
module_id = "m01-acquisition"
version = "1.4.2"  # 遵循 SemVer
display_name = "采集适配器"
description = "..."
authors = ["ada-team"]

[deps]
required = ["m03-dataflow >= 1.0, < 2.0", "m06-runtime >= 1.2"]
optional = ["m15-events"]

[entry]
type = "wasm"  # 或 "native" / "docker"
artifact = "m01-acquisition-v1.4.2.wasm"
sha256 = "..."

[api]
routes = ["/api/v1/canvases/{id}/acquire"]

[resources]
cpu = "500m"
memory = "256Mi"
storage = "100Mi"

[state]
kind = "stateless"  # 或 "stateful" / "leader-elected"

[compatibility]
min_runtime = "1.0"
replaces = ["m01-acquisition:1.3.x"]
migration_notes = "..."
```

### 4.2 部署状态机

```
Discovered → Registered → Downloading → Loaded → Active
                ↓                              ↓
            Failed                          Draining
                ↑                              ↓
            Rejected                       Drained → Unloading → Unloaded
                                                ↓
                                            Failed
```

- **Discovered**：Runtime 启动时扫描 artifacts 目录，发现新模块
- **Registered**：模块元数据写入 `module_registry` 表（PL/pgSQL 存过 `register_module()`）
- **Downloading**：从 registry 拉取 artifact，校验 sha256
- **Loaded**：加载到进程/容器，未对外暴露路由
- **Active**：注册到 API Gateway 路由表，开始接收流量
- **Draining**：停止接收新流量，等待进行中请求完成
- **Draining → Drained**：进行中请求归零
- **Unloading → Unloaded**：释放资源、卸载插件
- **Failed**：任一阶段失败，回滚到上一个 Active 版本

### 4.3 升级策略

| 策略 | 描述 | 适用 |
|---|---|---|
| Recreate | 停老版本，启动新版本 | 离线批处理模块 |
| Rolling | 逐副本替换 | 无状态服务 |
| Blue-Green | 双倍资源，瞬间切换 | 关键路径 |
| Canary | 5% → 25% → 50% → 100% 灰度 | 高风险变更 |

## 5. 中心事件管理

### 5.1 事件分类

| 类别 | 事件示例 | 持久化 |
|---|---|---|
| 系统事件 | `module.registered` / `cluster.leader_elected` | 7 天 |
| 业务事件 | `canvas.executed` / `execution.failed` | 30 天 |
| 审计事件 | `permission.changed` / `credential.accessed` | 1 年 |
| 数据事件 | `dataset.ingested` / `dataset.transformed` | 90 天 |

### 5.2 Pub/Sub 语义

- **Topic**：每个事件类别对应一个 topic（可通配订阅 `module.*`）
- **At-least-once delivery**：消费者至少收到一次（可能重复）
- **Consumer group**：同一 topic 多个消费者实例只接收一次
- **Replay**：消费者可指定 `from_offset` 重放历史

### 5.3 持久化模型

```
publish(topic, payload)
   │
   ▼
[1] PL/pgSQL append_event() 原子追加
   │  - 分配全局递增 event_seq
   │  - 写入 event_log 表
   │  - 写入持久化队列（避免消费者 lag 时内存堆积）
   ▼
[2] 通知订阅者
   │  - WebSocket push
   │  - 持久队列轮询（HTTP）
   ▼
[3] 消费者 ACK
   │  - 更新 consumer_offset
   │  - 删除已 ACK 的持久队列项（按 retention）
```

## 6. App 集群协调

### 6.1 节点身份

每个 Runtime 启动时：

1. 读取本机 hostname + 启动时间戳 + 随机 nonce → 生成 `node_id`（UUID v5）
2. 向 `cluster_node` 表注册（含 `advertised_addr:port`、标签、capacity）
3. 周期性发送心跳（默认 5s）

### 6.2 服务发现

API Gateway 通过 `cluster_node` 表查询"运行中且健康的 + 提供某类路由"的节点列表，按 round-robin / least-loaded 策略负载均衡。

### 6.3 领导选举

使用 `leader_lease` 表（PL/pgSQL `acquire_lease()` / `release_lease()`）：

```sql
-- 每 10s 续约，否则 30s 视为失联
SELECT * FROM acquire_lease('m04-orchestrator-singleton', $node_id, 30);
```

只有 leader 副本运行 Singleton 角色（如全局调度器）；其他副本为 Hot Standby。

### 6.4 状态分片

带状态模块（如 M-04 编排引擎）的状态按 `tenant_id` hash 分片到不同节点：

```
shard_id = hash(tenant_id) % N
```

新增节点 → 自动 rebalance；移除节点 → 状态迁移到剩余节点。

## 7. 热插拔协议

### 7.1 完整生命周期

```
register → resolve_deps → download → verify_hash → instantiate
   ↓                                                ↓
validate_compat                               init_runtime
   ↓                                                ↓
register_routes ←────────── activate ←─────── health_check
                                   ↓
                              receive_traffic
                                   ↓
                  ┌──────── drain ──────┐
                  ↓                      ↓
            stop_new_traffic      wait_inflight_to_zero
                  ↓                      ↓
              active=0  ←──────── inflight=0
                  ↓
              unregister_routes
                  ↓
              destroy_runtime
                  ↓
              unload_artifact
                  ↓
              unregistered
```

### 7.2 零停机保证

- **Activate 之前** 不修改路由表，新模块对流量不可见
- **Drain 期间** 旧模块继续服务，进行中请求自然结束
- **双版本共存窗口**：激活新版本 → 旧版本 drain 期间，两者都注册
- **失败回滚**：drain 超时或新模块 health_check 失败 → 立即 unregister_routes → 旧版本仍在

## 8. 4 者关系与协调

| 场景 | 原子化部署 | 中心事件 | 集群协调 | 热插拔 |
|---|---|---|---|---|
| 升级 m01-acquisition 到 1.5.0 | 主流程 | 触发 `module.registered` | 通知所有节点 | drain → activate |
| 新增节点加入集群 | - | 触发 `cluster.node_joined` | 注册 + 心跳 | - |
| 选举 m04-singleton leader | - | `cluster.leader_elected` | 抢租约 | - |
| 紧急回滚 | 主流程 | `module.rolled_back` | - | 旧版本重新 activate |
| 模块崩溃 | - | `module.crashed` | 失联检测 | 自动重启或 leader 接管 |

**关键时序**：所有部署/集群事件先经 `M-15 中心事件总线`，再由各节点异步消费 → 保证集群状态最终一致。

## 9. 管理员运维界面

详见 [DOC-ARCH-006](05-admin-operations-ui.md)。

## 10. API 与存储设计原则

### 10.1 API 原则

- 管理面 API 命名空间：`/api/v1/admin/{modules,events,cluster}/*`
- 必须要求 `Owner` / `PlatformAdmin` 角色
- 所有写操作经 `M-13 §3.1` 中间件链 + 强制审计日志
- 大批量操作（如全集群广播）异步化，返回 `202 Accepted` + operation_id

### 10.2 存储原则

- 新增 4 张核心表（详见 [M-10 §10 扩展](../modules/M-10-tenant-middleware.md)）：
  - `module_registry` - 模块清单
  - `module_version` - 模块多版本
  - `event_log` - 事件持久化（含 RLS by tenant）
  - `cluster_node` - 集群节点
  - `leader_lease` - 领导租约
  - `consumer_offset` - 消费者位点
- RLS 策略延续 M-10 的 `tenant_id` 隔离模式
- 关键路径使用 PL/pgSQL 存过保证原子性（详见 §11）

## 11. PL/pgSQL 存过策略

依据 [DOC-ARCH-001 §设计原则](../architecture/00-anatomy-model.md) 中"系统稳定性优先"原则，**关键状态变更使用 PL/pgSQL 存过**，避免应用层竞态：

| 存过 | 用途 | 关键不变量 |
|---|---|---|
| `register_module(manifest jsonb)` | 模块注册 | module_id 唯一、version 唯一约束 |
| `atomic_module_swap(module_id, from_version, to_version, strategy text)` | 原子升级 | 双写元数据一致性、原子切换、失败回滚 |
| `append_event(topic, payload jsonb, tenant_id uuid)` | 事件追加 | event_seq 全局递增、幂等（同 event_id 去重） |
| `acquire_lease(lease_key, node_id, ttl_seconds int)` | 抢租约 | 同一 lease_key 仅一节点持有 |
| `release_lease(lease_key, node_id)` | 释放租约 | 仅持有者可释放 |
| `register_node_heartbeat(node_id, status jsonb)` | 心跳+状态 | upsert、TTL 自动失效 |

每个存过使用 `LANGUAGE plpgsql SECURITY DEFINER` 配合 `SET search_path` 锁定 schema，并加注释说明不变量。

## 12. 验收硬指标

| 编号 | 指标 | NF タグ |
|---|---|---|
| AD-01 | 单模块升级过程业务中断 ≤ 0（蓝绿/灰度验证） | [NF-AVA]【必須】 |
| AD-02 | 中心事件端到端投递延迟 P95 ≤ 100ms（同集群） | [NF-PER]【必須】 |
| AD-03 | 集群规模 100 节点线性扩展（吞吐提升 ≥ 80x） | [NF-PER]【必須】 |
| AD-04 | 心跳失联检测 ≤ 30s 自动摘除 | [NF-AVA]【必須】 |
| AD-05 | 模块热插拔全过程 ≤ 60s（含 drain） | [NF-OPS]【必須】 |
| AD-06 | 事件持久化存储可重放 30 天 | [NF-AVA]【必須】 |
| AD-07 | PL/pgSQL 存过执行时间 ≤ 50ms | [NF-PER]【必須】 |
| AD-08 | 管理面 API 仅 Owner/PlatformAdmin 可访问 | [NF-SEC]【必須】 |
| AD-09 | 审计事件保留 1 年、合规可追溯 | [NF-SEC]【必須】 |
| AD-10 | 集群节点时间同步偏差 ≤ 100ms（NTP） | [NF-OPS]【必須】 |

## 13. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 原子化部署 | 模块独立打包/升级/回滚 | §4 |
| 中心事件总线 | 跨模块/集群事件通道 | §5 |
| App 集群 | 多 Runtime 副本集合 | §6 |
| 热插拔 | 运行时加载/卸载模块 | §7 |
| Module Manifest | 模块元数据声明文件 | §4.1 |
| 双写 | 新旧版本元数据并存 | §7.1 |
| Drain | 排空进行中请求 | §4.2 |
| Canary | 5% 灰度发布 | §4.3 |
| Pub/Sub | 发布/订阅模式 | §5.2 |
| At-least-once | 至少一次投递 | §5.2 |
| Leader Election | 集群中选出一个主 | §6.3 |
| 心跳 (Heartbeat) | 周期性存活信号 | §6.1 |
| RLS | Row-Level Security | §10.2 |
| PL/pgSQL | PostgreSQL 存储过程语言 | §11 |
| Manifest | 元数据清单 | §4.1 |
| 灰度发布 | 渐进式流量切换 | §4.3 |
| 蓝绿部署 | 双版本并存切换 | §4.3 |
| 状态分片 | 按 key 分布状态到节点 | §6.4 |

## 14. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. PostgreSQL Global Development Group「PostgreSQL Documentation — PL/pgSQL」
4. Kubernetes 公式ドキュメント「Deployments — Rolling Update Strategy」
5. NATS 公式ドキュメント「NATS — Cloud Native Application Connectivity」
6. SemVer 公式「Semantic Versioning 2.0.0」
7. CNCF「Cloud Native Definition v1.0」
8. Ada プロジェクトチーム各設計書 — [DOC-ARCH-001](00-anatomy-model.md) / [DOC-MOD-006](../modules/M-06-node-runtime-plugin-sdk.md) / [DOC-MOD-010](../modules/M-10-tenant-middleware.md) / [DOC-MOD-013](../modules/M-13-api-gateway.md)

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
