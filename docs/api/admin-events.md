# Admin API - 事件中心（Central Event Bus）

> **ドキュメントID**：DOC-API-005
> **文書分類**：横断文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/04-atomic-deployment.md`（DOC-ARCH-005）、`docs/modules/M-15`（DOC-MOD-015）
> **下位文書**：無
> **関連文書**：`docs/api/admin-modules.md`（DOC-API-004）、`docs/api/admin-cluster.md`（DOC-API-006）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018)
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（イベント API） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 概要
2. 认证与授权
3. 端点清单
4. 关键端点详解
5. WebSocket 实时流
6. 错误码
7. 用語集
8. 参考文献

---

## 1. 概要

本文定义 `/api/v1/admin/events/*` 命名空间下的 API 端点，承载 [DOC-MOD-015](../modules/M-15-central-event-bus.md) 中心事件总线对外能力（查询、订阅、重放）。发布由各业务模块内部直接调用 PL/pgSQL `append_event()`，不通过本 API 暴露。

## 2. 认证与授权

- **认证**：Bearer JWT
- **授权**：
  - `Owner` / `PlatformAdmin` 可订阅所有 topic
  - 其他角色可订阅 tenant 内的非审计 topic
  - 审计 topic (`permission.*`, `credential.*`, `audit.*`) 仅 `PlatformAdmin` 可订阅

## 3. 端点清单

```
# Topic 管理
GET    /api/v1/admin/events/topics                        # 列出所有 topic
POST   /api/v1/admin/events/topics                        # 注册 topic（首次 publish 自动注册）
GET    /api/v1/admin/events/topics/:topic                 # topic 详情
PATCH  /api/v1/admin/events/topics/:topic                 # 更新 topic 配置（retention 等）

# 事件查询
GET    /api/v1/admin/events                               # 查询事件（带过滤）
GET    /api/v1/admin/events/:event_id                     # 单事件详情
GET    /api/v1/admin/events/topics/:topic/tail            # tail（最近 N 条）

# 订阅管理
GET    /api/v1/admin/events/subscriptions                 # 列出订阅
POST   /api/v1/admin/events/subscriptions                 # 创建订阅
DELETE /api/v1/admin/events/subscriptions/:sub_id        # 删除订阅
GET    /api/v1/admin/events/subscriptions/:sub_id/offsets  # 查询消费者位点
POST   /api/v1/admin/events/subscriptions/:sub_id/offsets/reset  # 重置位点

# Replay
POST   /api/v1/admin/events/replay                        # 启动 replay 任务
GET    /api/v1/admin/events/replay/:replay_id             # replay 进度
POST   /api/v1/admin/events/replay/:replay_id/abort       # 中止 replay
```

## 4. 关键端点详解

### 4.1 GET /api/v1/admin/events - 查询事件

**Query Parameters**：

| 参数 | 类型 | 说明 |
|---|---|---|
| `topic` | string | 精确 topic 或通配符（`*`, `#`） |
| `tenant_id` | UUID | 限定租户 |
| `from_seq` | int | 起始 event_seq |
| `to_seq` | int | 结束 event_seq |
| `from_time` | ISO8601 | 起始时间 |
| `to_time` | ISO8601 | 结束时间 |
| `producer` | string | 生产者模块 ID |
| `limit` | int | 最多返回数量，默认 100，上限 1000 |
| `order` | asc/desc | 排序方向，默认 desc |

**Response 200**：

```json
{
  "events": [
    {
      "event_id": "uuid",
      "event_seq": 12345,
      "topic": "module.swapped",
      "tenant_id": "uuid",
      "payload": { ... },
      "produced_at": "2026-08-19T10:00:00Z",
      "producer": "m14-module-registry"
    }
  ],
  "has_more": true,
  "next_seq": 12350
}
```

### 4.2 POST /api/v1/admin/events/subscriptions - 创建订阅

**Request Body**：

```json
{
  "topic_pattern": "module.*",
  "group_id": "admin-ui-cluster-viewer",
  "delivery_mode": "durable",   // "durable" | "ephemeral"
  "from_position": "latest",    // "earliest" | "latest" | {"event_seq": 1000}
  "filter": {
    "tenant_id": "uuid（可选）"
  }
}
```

**Response 201**：

```json
{
  "subscription_id": "uuid",
  "topic_pattern": "module.*",
  "group_id": "admin-ui-cluster-viewer",
  "delivery_mode": "durable",
  "from_position": "latest",
  "created_at": "2026-08-19T10:00:00Z"
}
```

### 4.3 POST /api/v1/admin/events/replay - 启动 replay

**Request Body**：

```json
{
  "topic": "module.*",
  "from": { "event_seq": 1000 },
  "to": { "event_seq": 2000 },
  "target_subscription_id": "uuid（可选，不指定则仅 dry-run）",
  "rate_limit_per_second": 100,
  "dry_run": true
}
```

**Response 202**：

```json
{
  "replay_id": "uuid",
  "topic": "module.*",
  "event_count_estimated": 1000,
  "status": "Pending"
}
```

## 5. WebSocket 实时流

```
ws://host/api/v1/admin/events/stream?token={jwt}&topic_pattern={pattern}&group_id={id}
```

服务端推送（与 [DOC-API-002](websocket-events.md) 兼容格式）：

```json
{
  "type": "event.delivered",
  "data": {
    "event_id": "uuid",
    "event_seq": 12345,
    "topic": "module.swapped",
    "payload": { ... }
  }
}
```

支持：
- 多 topic 通配符订阅
- 心跳（30s 一次）
- 断线重连后从 consumer_offset 续传

## 6. 错误码

| Error Code | HTTP Status | 説明 | NF タグ |
|---|---|---|---|
| `TOPIC_NOT_FOUND` | 404 | topic 未注册 | [NF-OPS]【必須】 |
| `INVALID_TOPIC_PATTERN` | 400 | 通配符语法错误 | [NF-OPS]【必須】 |
| `SUBSCRIPTION_LIMIT_EXCEEDED` | 429 | 单 group 订阅数超限 | [NF-PER]【必須】 |
| `REPLAY_NOT_ALLOWED` | 403 | 无 replay 权限 | [NF-SEC]【必須】 |
| `REPLAY_RATE_EXCEEDED` | 429 | replay 速率超限 | [NF-PER]【必須】 |
| `EVENT_NOT_FOUND` | 404 | event_id 不存在或超出 retention | [NF-OPS]【必須】 |
| `INVALID_TIME_RANGE` | 400 | 时间范围非法 | [NF-OPS]【必須】 |
| `OFFSET_RESET_FORBIDDEN` | 403 | 审计 topic 禁止重置位点 | [NF-SEC]【必須】 |

## 7. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 事件 API | /api/v1/admin/events/* 端点群 | §1 |
| Topic | 事件分类 | §3 |
| 订阅 Subscription | 消费者配置 | §3 |
| Consumer Group | 同一组消费者只收一次 | §4.2 |
| Replay | 历史事件重放 | §4.3 |
| 持久订阅 | durable subscription | §4.2 |
| 临时订阅 | ephemeral subscription | §4.2 |
| 位点 | consumer offset | §3 |
| 速率限制 | rate limit | §4.3 |
| Dry-run | 干跑模式 | §4.3 |
| Wildcard | 通配符 * 和 # | §3 |
| Heartbeat | 连接心跳 | §5 |
| Tail | 最新 N 条 | §3 |

## 8. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Ada プロジェクトチーム「中央イベントバス v1.0.0」、2026-08-19（[DOC-MOD-015](../modules/M-15-central-event-bus.md)）
4. Ada プロジェクトチーム「原子化部署アーキテクチャ v1.0.0」、2026-08-19（[DOC-ARCH-005](../architecture/04-atomic-deployment.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
