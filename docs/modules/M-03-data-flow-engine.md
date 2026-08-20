# M-03 数据流引擎（Data Flow Engine）

> **ドキュメントID**：DOC-MOD-003
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §3）
> **関連文書**：`docs/modules/M-04`（DOC-MOD-004）、`docs/modules/M-07`（DOC-MOD-007）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-18 | 初版制定 | Ada プロジェクトチーム | TBD | TBD |
| v1.1.0 | 2026-08-19 | IPA 準拠メタデータ追加、NF タグ付与 | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 需求来源（要件定義書）
2. 基本设计（基本設計書）
3. 詳細设计（詳細設計書）
4. 验收要点
5. 用語集
6. 参考文献

---

## 1. 需求来源（要件定義書）

### 1.1 涉及 F-IDs

- **F-04** 数据流引擎（血液层）

### 1.2 关联用例

U-01 跨平台内容同步、U-02 多源数据聚合看板、U-05 可视化调试

### 1.3 非功能需求

- 7.2 性能：单节点数据处理吞吐 ≥ 100 条/秒
- 7.1 可用性：Runtime 意外退出后重启，应能基于最近一次持久化状态自动恢复未完成的画布执行（断点续传）

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中的"血液层（データフロー）"，对应 §2 表中"血液"行。

### 2.2 设计核心

数据流（血液）不受编排引擎（神经）的直接强控制，仅依据既定的流转路径（血管，即连线）持续、自主地在节点间搬运标准化 JSON 数据包，类似循环系统的自主调节（自律神经支配之外的物理循环特性）。

### 2.3 主要职责（basic-design §3.2.4）

- 节点出队与入队管理（消息队列驱动）
- 背压处理（Backpressure）——下游处理慢时自动暂停上游
- 数据在连线上的缓存与转发
- 流量监控与可视化数据
- 支持数据包的重放（Replay）用于调试

### 2.4 实现方案

- 基于 async channel（tokio/crossbeam）的事件驱动架构
- 每条连线（Edge）是一个单向队列，支持配置容量与超时

## 3. 详细设计（詳細設計書）

### 3.1 连线（Edge）运行时表示

```rust
/// 每条数据流连线在运行时对应一个异步有界队列
pub struct DataFlowChannel {
    pub edge_id: String,
    pub sender: tokio::sync::mpsc::Sender<NJson>,
    pub receiver: Arc<Mutex<tokio::sync::mpsc::Receiver<NJson>>>,
    pub buffer_config: BufferConfig,
    pub metrics: ChannelMetrics,
}

pub struct ChannelMetrics {
    pub throughput_counter: AtomicU64,   // 累计流经数据包数
    pub current_queue_depth: AtomicUsize,
    pub last_activity: AtomicI64,        // Unix 时间戳，用于堆积检测告警
}
```

### 3.2 背压（Backpressure）机制

```
函数 send_with_backpressure(channel, item):
    根据 channel.buffer_config.overflow_policy 分支：

    Block（默认）:
        channel.sender.send(item).await   // tokio mpsc 的天然背压：队列满时 send 挂起
        // 上游节点的执行协程在此处自动暂停，直到下游消费腾出空间

    DropOldest:
        尝试 try_send(item)
        若队列满：弹出队首最旧元素，再重试 try_send
        记录 metrics.dropped_count += 1，写入审计日志

    DropNewest:
        尝试 try_send(item)
        若队列满：丢弃当前 item，不做重试
        记录 metrics.dropped_count += 1
```

### 3.3 数据缓存与重放（F-04-03）

下游节点暂停（如进入 `HumanReview` 等待用户确认）时，数据不丢失的关键在于：**tokio mpsc 队列本身即为缓存介质**，只要队列容量足够且上游不主动清空，数据天然驻留在内存中。

对于**跨进程重启**场景（Runtime 崩溃重启后仍需恢复未处理的数据），设计**持久化溢出策略**：

```rust
pub struct PersistentOverflowBuffer {
    // 当 in-memory 队列使用率超过 80% 时，触发向 Redis/本地文件溢出
    threshold_ratio: f64,
    backing_store: OverflowBackingStore,  // Redis List 或本地 sled 数据库
}
```

重放（Replay）功能：[M-07 调试服务](../modules/M-07-debug-service.md) 可请求 `DataFlowEngine::replay_from_snapshot(execution_id, node_id)`，从持久化的节点输入快照重新构造 `NJson` 并重新注入对应 Edge 的队列。

### 3.4 流量监控可视化数据源（F-04-04）

`ChannelMetrics` 通过 Prometheus 指标格式暴露：

```
ada_dataflow_throughput_total{tenant_id, canvas_id, edge_id}
ada_dataflow_queue_depth{tenant_id, canvas_id, edge_id}
ada_dataflow_dropped_total{tenant_id, canvas_id, edge_id, reason}
```

前端通过 [WebSocket `canvas.dataflow.metrics` 事件](../api/websocket-events.md) 订阅这些指标的采样推送（默认 1s 间隔），驱动画布上连线的"流光动效"渲染速度与颜色（堆积越多颜色越偏红）。详见 [M-12 §12.5 视口虚拟化渲染性能保证](../modules/M-12-canvas-editor-frontend.md)。

## 4. 验收要点

1. **背压正确性**：下游处理慢时上游自动暂停，不出现 OOM。
2. **持久化溢出**：Runtime 异常重启后未消费的数据可恢复。
3. **流量监控实时性**：1s 采样间隔内，堆积状态能在前端画布上以颜色渐变（绿→红）反映。
4. **重放功能**：节点输入快照可被重放至对应 Edge，调试场景下能复现历史执行。
5. **多租户隔离**：流量指标的 labels 必须包含 `tenant_id`，避免跨租户指标混淆。 [NF-SEC]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 数据流引擎 | 节点间数据流转的"血液层" | §1、DOC-ARCH-001 |
| DataFlowChannel | 连线对应的有界队列 | §3.1 |
| 背压 (Backpressure) | 下游处理慢时上游自动暂停 | §3.2 [NF-PER]【必須】 |
| 溢出策略 | 队列满时的处理方式（Block/DropOldest/DropNewest） | §3.2 |
| 持久化溢出 | 内存队列满时溢出到 Redis/文件 | §3.3 [NF-AVA]【必須】 |
| Replay | 从历史快照重放节点执行 | §3.3 |
| ChannelMetrics | 队列的吞吐量/深度/丢弃统计 | §3.1 |
| Prometheus 指标 | 云原生监控指标格式 | §3.4 |
| 流光动效 | 数据流可视化渲染 | §3.4 |
| 多租户隔离 | tenant_id 标签强制 | §4.5 [NF-SEC]【必須】 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Prometheus 公式ドキュメント「Prometheus — Monitoring system & time series database」
4. tokio 公式ドキュメント「tokio — An asynchronous runtime for Rust」
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
