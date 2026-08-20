# Admin API - 集群管理（Cluster Management）

> **ドキュメントID**：DOC-API-006
> **文書分類**：横断文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/04-atomic-deployment.md`（DOC-ARCH-005）、`docs/modules/M-16`（DOC-MOD-016）
> **下位文書**：無
> **関連文書**：`docs/api/admin-modules.md`（DOC-API-004）、`docs/api/admin-events.md`（DOC-API-005）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 7 章
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（クラスタ管理 API） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 概要
2. 认证与授权
3. 端点清单
4. 关键端点详解
5. 错误码
6. 用語集
7. 参考文献

---

## 1. 概要

本文定义 `/api/v1/admin/cluster/*` 命名空间下的 API 端点，承载 [DOC-MOD-016](../modules/M-16-cluster-coordinator.md) 集群协调能力（节点管理、领导选举、状态分片）。

## 2. 认证与授权

- **认证**：Bearer JWT
- **授权**：
  - `PlatformAdmin` 可执行所有操作
  - `Owner` 仅可查询本租户相关节点（受限视图）
  - 节点注册/心跳由 Runtime 进程内部调用，使用 Service Token（不是用户 JWT）

## 3. 端点清单

```
# 节点清单
GET    /api/v1/admin/cluster/nodes                        # 列出所有节点
GET    /api/v1/admin/cluster/nodes/:node_id               # 节点详情
GET    /api/v1/admin/cluster/nodes/:node_id/health        # 健康历史
GET    /api/v1/admin/cluster/nodes/:node_id/metrics       # 节点指标（CPU/内存等）
POST   /api/v1/admin/cluster/nodes/:node_id/drain         # 排空
POST   /api/v1/admin/cluster/nodes/:node_id/remove        # 摘除
POST   /api/v1/admin/cluster/nodes/:node_id/rejoin        # 重新加入

# 领导选举
GET    /api/v1/admin/cluster/leaders                      # 列出所有 leader
GET    /api/v1/admin/cluster/leaders/:lease_key           # 单 leader 详情
POST   /api/v1/admin/cluster/leaders/:lease_key/force-yield  # 强制让位

# 状态分片
GET    /api/v1/admin/cluster/shards                       # 列出分片
GET    /api/v1/admin/cluster/shards/:shard_id             # 分片详情
POST   /api/v1/admin/cluster/shards/rebalance             # 触发 rebalance

# Runtime 内部调用（Service Token）
POST   /api/v1/internal/cluster/nodes/:node_id/heartbeat  # 心跳
POST   /api/v1/internal/cluster/leases/:key/renew         # 续约
```

## 4. 关键端点详解

### 4.1 GET /api/v1/admin/cluster/nodes - 列出节点

**Query Parameters**：

| 参数 | 类型 | 说明 |
|---|---|---|
| `state` | string | `Active` / `Unhealthy` / `Draining` / `Removed` |
| `label_selector` | string | 如 `zone=us-east-1a,role=worker` |
| `healthy_only` | bool | 仅返回健康节点 |
| `include_removed` | bool | 默认 false |

**Response 200**：

```json
{
  "nodes": [
    {
      "node_id": "uuid",
      "hostname": "node-1.ada.example.com",
      "advertised_addr": "10.0.1.5:8000",
      "labels": { "zone": "us-east-1a", "role": "worker" },
      "state": "Active",
      "started_at": "2026-08-19T08:00:00Z",
      "last_heartbeat_at": "2026-08-19T10:00:05Z",
      "health": "Healthy",
      "current_load": 0.45,
      "capacity": 1.0,
      "active_modules": ["m01-acquisition", "m02-normalizer"]
    }
  ],
  "total": 12,
  "healthy": 11
}
```

### 4.2 GET /api/v1/admin/cluster/nodes/:node_id/health - 健康历史

**Query Parameters**：

| 参数 | 类型 | 说明 |
|---|---|---|
| `window` | string | `5m` / `1h` / `24h`（默认 1h） |
| `granularity` | string | `1s` / `10s` / `1m`（默认 10s） |

**Response 200**：

```json
{
  "node_id": "uuid",
  "window": "1h",
  "granularity": "10s",
  "samples": [
    { "at": "2026-08-19T09:00:00Z", "health": "Healthy", "cpu": 0.42, "memory": 0.65 },
    { "at": "2026-08-19T09:00:10Z", "health": "Healthy", "cpu": 0.45, "memory": 0.66 }
  ]
}
```

### 4.3 POST /api/v1/admin/cluster/nodes/:node_id/drain - 排空

**Request Body**：

```json
{
  "timeout_seconds": 300,
  "force": false
}
```

**Response 202**：

```json
{
  "drain_id": "uuid",
  "node_id": "uuid",
  "state": "Draining",
  "started_at": "2026-08-19T10:00:00Z"
}
```

执行后节点状态变为 `Draining`，停止接收新流量，等待进行中请求完成或超时。完成后自动变 `Active`（带 `drained` 标记）或 `Removed`（若 `force=true`）。

### 4.4 POST /api/v1/admin/cluster/nodes/:node_id/remove - 摘除

**Response 202**：

```json
{
  "node_id": "uuid",
  "state": "Removed",
  "removed_at": "2026-08-19T10:00:00Z",
  "affected_modules": ["m01-acquisition"],
  "shard_rebalance_triggered": true
}
```

执行后：
1. 节点状态置 `Removed`，从负载均衡池移除
2. 触发 `cluster.node_removed` 事件
3. 自动触发 shard rebalance（如该节点有负责的 shard）
4. 节点上的 module_instance 状态置 `Terminated`

### 4.5 GET /api/v1/admin/cluster/leaders - 列出 Leader

**Response 200**：

```json
{
  "leaders": [
    {
      "lease_key": "m04-orchestrator-singleton",
      "holder_node_id": "uuid",
      "acquired_at": "2026-08-19T09:00:00Z",
      "expires_at": "2026-08-19T10:00:30Z",
      "renew_count": 540,
      "ttl_seconds": 30
    }
  ]
}
```

### 4.6 POST /api/v1/admin/cluster/leaders/:lease_key/force-yield - 强制让位

**Request Body**：

```json
{
  "reason": "scheduled maintenance",
  "new_target_node_id": "uuid（可选，不指定则由选举产生）"
}
```

**Response 202**：

```json
{
  "lease_key": "m04-orchestrator-singleton",
  "previous_holder": "uuid",
  "new_holder": "uuid",
  "yielded_at": "2026-08-19T10:00:00Z"
}
```

强制释放当前 leader 的租约，触发 `cluster.leader_elected` 事件。

### 4.7 POST /api/v1/admin/cluster/shards/rebalance - 触发 rebalance

**Request Body**：

```json
{
  "strategy": "minimal_disruption",   // "minimal_disruption" | "even_load"
  "max_concurrent_migrations": 5
}
```

**Response 202**：

```json
{
  "rebalance_id": "uuid",
  "total_shards": 10,
  "estimated_duration_seconds": 120,
  "status": "InProgress"
}
```

### 4.8 POST /api/v1/internal/cluster/nodes/:node_id/heartbeat - 内部心跳

**Request Body**：

```json
{
  "status": {
    "health": "Healthy",
    "cpu_usage": 0.42,
    "memory_usage": 0.65,
    "active_executions": 12
  }
}
```

**Response 200**：

```json
{
  "healthy": true,
  "current_load": 0.45,
  "next_heartbeat_in_seconds": 5
}
```

由 Runtime 进程内部每 5s 调用一次，使用 Service Token（非用户 JWT）。

## 5. 错误码

| Error Code | HTTP Status | 説明 | NF タグ |
|---|---|---|---|
| `NODE_NOT_FOUND` | 404 | 节点不存在 | [NF-OPS]【必須】 |
| `NODE_ALREADY_DRAINING` | 409 | 节点已在排空中 | [NF-AVA]【必須】 |
| `NODE_ALREADY_REMOVED` | 409 | 节点已摘除 | [NF-AVA]【必須】 |
| `LEADER_NOT_FOUND` | 404 | lease_key 不存在 | [NF-OPS]【必須】 |
| `LEADER_HELD_BY_OTHERS` | 409 | 强制让位但非持有者（异常） | [NF-SEC]【必須】 |
| `REBALANCE_IN_PROGRESS` | 409 | 已有 rebalance 进行中 | [NF-AVA]【必須】 |
| `INTERNAL_TOKEN_REQUIRED` | 401 | 内部端点需 Service Token | [NF-SEC]【必須】 |
| `INSUFFICIENT_PERMISSIONS` | 403 | 缺 PlatformAdmin 角色 | [NF-SEC]【必須】 |
| `DRAIN_TIMEOUT` | 504 | 排空超时 | [NF-AVA]【必須】 |
| `REBALANCE_FAILED` | 500 | rebalance 失败 | [NF-AVA]【必須】 |

## 6. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 集群管理 API | /api/v1/admin/cluster/* 端点群 | §1 |
| Service Token | 内部调用专用的服务级 JWT | §2 |
| 节点 (Node) | Runtime 实例 | §3 |
| 领导 (Leader) | 集群中持有租约的主 | §3 |
| 租约 (Lease) | 持有者声明有效性的时间窗口 | §3 |
| 让位 (Yield) | 主动放弃 leader | §4.6 |
| 排空 (Drain) | 停止接收新流量 | §4.3 |
| 摘除 (Remove) | 从集群移除节点 | §4.4 |
| 状态分片 (Shard) | 按 key 分布状态 | §3 |
| Re-balance | 重新分布分片 | §4.7 |
| 内部端点 | /api/v1/internal/* 服务间调用 | §3 |
| 心跳 (Heartbeat) | 周期性存活信号 | §4.8 |

## 7. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Ada プロジェクトチーム「クラスタコーディネーター v1.0.0」、2026-08-19（[DOC-MOD-016](../modules/M-16-cluster-coordinator.md)）
4. Ada プロジェクトチーム「原子化部署アーキテクチャ v1.0.0」、2026-08-19（[DOC-ARCH-005](../architecture/04-atomic-deployment.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
