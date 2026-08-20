# M-15 中心事件总线（Central Event Bus）

> **ドキュメントID**：DOC-MOD-015
> **文書分類**：モジュール別設計書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/04-atomic-deployment.md`（DOC-ARCH-005）
> **下位文書**：`docs/api/admin-events.md`（DOC-API-005）
> **関連文書**：`docs/modules/M-03`（DOC-MOD-003 画布内数据流）、`docs/modules/M-10`（DOC-MOD-010）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（Pub/Sub + 永続化 + リプレイ） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 需求来源
2. 基本设计
3. 詳細设计
4. 验收要点
5. 用語集
6. 参考文献

---

## 1. 需求来源（要件定義書）

### 1.1 涉及来源

- 上一级需求：[DOC-ARCH-005 §5 中心事件管理](../architecture/04-atomic-deployment.md)
- 既有能力：M-03 数据流引擎是**画布内**数据通道，M-15 是**系统级**事件通道
- 隐含需求：M-11 审计日志需统一为 M-15 事件源

### 1.2 NF 标签要求

- 端到端投递 P95 ≤ 100ms（同集群） [NF-PER]【必須】
- 事件持久化 30 天可重放 [NF-AVA]【必須】
- 审计事件保留 1 年 [NF-SEC]【必須】
- At-least-once 投递不丢 [NF-AVA]【必須】

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/04 §3 总体架构](../architecture/04-atomic-deployment.md) 中的"管理平面+服务平面共有"，是所有模块间**系统级**通信的中枢。

### 2.2 与 M-03 画布数据流的区别

| 维度 | M-03 画布数据流 | M-15 中心事件 |
|---|---|---|
| 范围 | 单画布内 | 跨模块/跨集群 |
| 消费者 | 画布的下一节点 | 任意订阅者 |
| 生命周期 | 随画布执行 | 永久（按 retention） |
| 模式 | 点对点（Edge） | Pub/Sub（topic） |
| 持久化 | 临时缓存 | event_log 表（PL/pgSQL 存过） |

### 2.3 核心职责

- Topic 注册与生命周期
- 事件发布（Publish）
- 事件订阅（Subscribe，含持久订阅 / 临时订阅）
- 事件持久化（event_log 表 + 持久队列）
- 事件重放（指定 `from_offset` 或时间窗口）
- 消费者位点管理（consumer_offset 表）
- 与 [M-11 §3.3 审计日志](../modules/M-11-rbac-collab.md) 整合

### 2.4 涉及表

| 表 | 用途 | 詳細位置 |
|---|---|---|
| `event_log` | 事件持久化（含 RLS） | [M-10 §10 扩展](../modules/M-10-tenant-middleware.md) |
| `event_topic` | Topic 元数据 | 同上 |
| `event_subscription` | 订阅者配置 | 同上 |
| `consumer_offset` | 消费者位点 | 同上 |

## 3. 詳細设计（詳細設計書）

### 3.1 Topic 命名规范

```
<category>.<entity>.<action>

例：
- module.registered
- module.swapped
- cluster.node_joined
- cluster.leader_elected
- canvas.execution.completed
- permission.changed
- credential.accessed
- system.config_changed
```

支持通配符订阅：`*` 匹配单段，`#` 匹配多段（Kafka 风格）。

### 3.2 核心 Trait

```rust
#[async_trait]
pub trait CentralEventBus: Send + Sync {
    /// 注册 topic（首次发布自动注册）
    async fn ensure_topic(&self, topic: &str, config: TopicConfig) -> Result<(), BusError>;

    /// 发布事件
    async fn publish(
        &self,
        topic: &str,
        payload: serde_json::Value,
    ) -> Result<EventId, BusError>;

    /// 订阅（持久订阅：位点持久化；临时订阅：实时推送）
    async fn subscribe(
        &self,
        topic_pattern: &str,
        group_id: &str,
        handler: Box<dyn EventHandler>,
    ) -> Result<SubscriptionId, BusError>;

    /// 重放（指定起始 offset 或时间）
    async fn replay(
        &self,
        topic: &str,
        from: ReplayPosition,
    ) -> Result<EventStream, BusError>;
}
```

### 3.3 事件 schema

```json
{
  "event_id": "uuid v7（按时间排序）",
  "event_seq": 12345,
  "topic": "module.swapped",
  "tenant_id": "uuid（多租户隔离）",
  "payload": { ... },
  "headers": {
    "schema_version": "1.0",
    "trace_id": "uuid",
    "producer": "m14-module-registry",
    "produced_at": "2026-08-19T10:00:00Z"
  }
}
```

### 3.4 发布流程

```
publisher.publish(topic, payload)
   │
   ▼
[1] PL/pgSQL append_event() 原子追加
   │  - 分配全局递增 event_seq（SEQUENCE）
   │  - event_id 写入 event_log
   │  - 触发 NOTIFY 'event_appended'，通知 pg_listening 进程
   ▼
[2] 后台 dispatcher
   │  - 读取 NOTIFY
   │  - 查询匹配 topic_pattern 的订阅者
   │  - 推送到 WebSocket 或入持久队列
   ▼
[3] 消费者 ACK
   │  - 更新 consumer_offset
   │  - 删除持久队列项
```

### 3.5 PL/pgSQL 存过：append_event

```sql
CREATE OR REPLACE FUNCTION append_event(
    p_topic    TEXT,
    p_payload  JSONB,
    p_tenant_id UUID DEFAULT NULL
) RETURNS TABLE(event_id UUID, event_seq BIGINT)
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_seq BIGINT;
    v_id UUID;
    v_tenant_id UUID;
BEGIN
    -- 1. 解析 tenant_id（从会话变量或入参）
    v_tenant_id := COALESCE(p_tenant_id,
        current_setting('app.current_tenant', true)::UUID);
    
    -- 2. 分配 event_seq（全局 SEQUENCE）
    v_seq := nextval('event_seq_global');
    
    -- 3. 生成 event_id（UUID v7 或 v4）
    v_id := gen_random_uuid();
    
    -- 4. 插入 event_log（RLS 自动过滤）
    INSERT INTO event_log (
        id, event_seq, topic, tenant_id, payload, 
        produced_at, producer
    ) VALUES (
        v_id, v_seq, p_topic, v_tenant_id, p_payload,
        now(), current_setting('app.current_service', true)
    );
    
    -- 5. NOTIFY 监听者
    PERFORM pg_notify('event_appended', json_build_object('seq', v_seq, 'topic', p_topic)::TEXT);
    
    RETURN QUERY SELECT v_id, v_seq;
END;
$$;
```

### 3.6 持久化策略

| 类别 | retention | 索引 | 备注 |
|---|---|---|---|
| 系统事件 (`module.*`, `cluster.*`) | 7 天 | event_seq, topic | 可重放 |
| 业务事件 (`canvas.*`, `execution.*`) | 30 天 | event_seq, topic, tenant_id | 可重放 |
| 审计事件 (`permission.*`, `credential.*`, `audit.*`) | 1 年 | event_seq, actor, resource | 合规要求 |
| 数据事件 (`dataset.*`) | 90 天 | event_seq, dataset_id | 衍生数据重建 |

后台任务每日按 retention 清理。

### 3.7 订阅模式

#### 持久订阅（Durable Subscription）
- consumer_offset 持久化在 PostgreSQL
- 消费者重启后从上次 ACK 位点继续
- 适用：业务模块、运维界面

#### 临时订阅（Ephemeral Subscription）
- 仅 WebSocket 实时推送
- 消费者断开后位点丢失
- 适用：运维界面实时面板

#### 消费者组（Consumer Group）
- 同一 group_id 多个消费者，事件只被一个消费
- 实现：dispatcher 按轮询/最少堆积分配

### 3.8 Replay 机制

```
replay(topic, from=event_seq=1000, limit=1000)
   │
   ▼
[1] SELECT FROM event_log
   │  WHERE topic = $topic AND event_seq >= 1000
   │  ORDER BY event_seq LIMIT 1000
   ▼
[2] 投递到指定订阅者（指定 replay 模式，不影响生产 consumer_offset）
```

支持速率限制（防打爆下游）、干跑模式（仅记录不投递）。

### 3.9 性能保证

- 单 topic 顺序投递（同一 topic 内 event_seq 单调递增）
- 跨 topic 不保证顺序
- 批量订阅支持（订阅者按 batch 拉取）
- 持久队列使用 Redis List（DB 写完后入队，避免消费者拉空）

## 4. 验收要点

1. **投递延迟**：P95 ≤ 100ms（同集群，topic 流量 1000 events/s）。 [NF-PER]【必須】
2. **持久化不丢**：发布成功后事件至少保留 30 天（按类别）。 [NF-AVA]【必須】
3. **重放正确性**：重放后 consumer_offset 不变（不影响生产消费进度）。 [NF-AVA]【必須】
4. **PL/pgSQL 原子性**：并发 `append_event` 调用 event_seq 唯一且单调递增。 [NF-SEC]【必須】
5. **多租户隔离**：A 租户订阅者仅收到 tenant_id=A 的事件。 [NF-SEC]【必須】
6. **At-least-once**：消费者处理失败时不更新 consumer_offset，重启后重新消费。 [NF-AVA]【必須】

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 中心事件总线 | 跨模块/集群事件通道 | §1 |
| Pub/Sub | 发布/订阅模式 | §2.2 |
| Topic | 事件分类标识 | §3.1 |
| Consumer Group | 同一组消费者只收一次 | §3.7 |
| At-least-once | 至少一次投递 | §3.7 |
| Event Seq | 全局递增事件序列号 | §3.3 |
| Replay | 历史事件重放 | §3.8 |
| Retention | 事件保留期 | §3.6 |
| Persistent Subscription | 持久订阅 | §3.7 |
| Append-only | 仅追加不可修改 | §3.5 |
| NOTIFY/LISTEN | PostgreSQL 异步通知机制 | §3.4 |
| Dispatcher | 事件分发器 | §3.4 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. PostgreSQL Global Development Group「PostgreSQL Documentation — NOTIFY / LISTEN」
4. Apache Kafka 公式「Kafka — Distributed event streaming platform」
5. Ada プロジェクトチーム「原子化部署アーキテクチャ v1.0.0」、2026-08-19（[DOC-ARCH-005](../architecture/04-atomic-deployment.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
