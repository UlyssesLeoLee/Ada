# M-05 控制流执行器（Control Flow Executor）

> **ドキュメントID**：DOC-MOD-005
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §5）
> **関連文書**：`docs/modules/M-04`（DOC-MOD-004）、`docs/modules/M-10`（DOC-MOD-010）
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

- **F-06** 控制流执行器（肌肉层）

### 1.2 关联用例

U-01 跨平台内容同步、U-03 事件触发式自动化、U-04 数据清洗与人工复核

### 1.3 非功能需求

- 7.2 性能：采集并发支持多个 Playwright 浏览器实例并发运行
- 7.5 安全：多租户数据隔离（影响并发度调度时按租户分配）

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中的"肌肉层（制御フロー）"，对应 §2 表中"肌肉"行。是连接"决策"（神经系统）与"结构"（骨骼）之间的执行动力层。

### 2.2 职责定位

神经系统下达的指令被转化为具体的"动作"——即节点的实际触发、暂停、跳过、并行/串行调度。控制流执行器**只做执行动力**，不做决策（决策由 [M-04 编排引擎](../modules/M-04-orchestration-engine.md) 负责）。

### 2.3 主要职责（basic-design §3.2 引用）

- 节点触发、并发调度、暂停恢复、限流退避
- 节点级并发度配置（串行/并行/限流并行）
- 整体流程的暂停、恢复、终止
- 单节点的手动触发（用于调试）

## 3. 详细设计（詳細設計書）

### 3.1 并发调度器

```rust
/// 单个节点一次执行尝试的结果，供编排引擎（M-04 transition）与调试服务（M-07）消费
pub struct NodeExecutionResult {
    pub node_id: String,
    pub attempt: u32,
    pub outcome: NodeExecutionOutcome,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,
}

pub enum NodeExecutionOutcome {
    Success { output: NJson },
    Failure { error: AdapterError },   // 或其他模块自身的 Error 类型，经统一 trait 收敛
    Aborted { reason: String },
}

pub struct ControlFlowExecutor {
    // 按租户隔离的并发度限制信号量
    tenant_semaphores: DashMap<TenantId, Arc<Semaphore>>,
    node_semaphores: DashMap<String, Arc<Semaphore>>,  // 节点级并发度配置
}

impl ControlFlowExecutor {
    pub async fn dispatch(&self, nodes: &[NodeDefinition], state: &ExecutionState)
        -> Vec<NodeExecutionResult>
    {
        let tenant_permit = self.tenant_semaphores
            .entry(state.tenant_id.unwrap_or_default())
            .or_insert_with(|| Arc::new(Semaphore::new(DEFAULT_TENANT_CONCURRENCY)))
            .clone();

        let futures = nodes.iter().map(|node| {
            let permit = tenant_permit.clone();
            async move {
                let _guard = permit.acquire().await.unwrap();  // 租户级限流
                self.execute_single_node(node, state).await
            }
        });

        futures::future::join_all(futures).await
    }
}
```

### 3.2 暂停/恢复/终止语义

```rust
pub enum ExecutionControlSignal {
    Pause,
    Resume,
    Abort,
}

pub struct ExecutionControlHandle {
    signal_tx: tokio::sync::watch::Sender<ExecutionControlSignal>,
}
```

每个节点执行协程在关键检查点（节点执行前、每次数据包处理前）轮询 `watch::Receiver`，若收到 `Pause` 则在当前操作完成后挂起等待 `Resume`；若收到 `Abort` 则立即终止并将节点状态置为 `Skipped { reason: "aborted_by_user" }`。

### 3.3 单节点手动触发（F-06-03，调试用）

```rust
pub async fn trigger_single_node(
    canvas_id: uuid::Uuid,
    node_id: String,
    mock_input: Option<NJson>,
) -> Result<NodeExecutionResult, DebugError> {
    // 不经过完整编排引擎状态机，直接构造最小化单节点执行上下文
    // mock_input 为 None 时，使用该节点最近一次成功执行的输入快照（来自 M-07）
}
```

## 4. 验收要点

1. **并发度可配置**：节点级与租户级并发度可独立配置，互不干扰。
2. **暂停/恢复可恢复**：暂停后从同一状态机位置继续，不丢数据。
3. **终止后状态正确**：被 `Abort` 的节点状态为 `Skipped { reason: "aborted_by_user" }`，不污染执行记录。
4. **租户隔离正确性**：A 租户并发占满不影响 B 租户执行。
5. **单节点调试**：通过 `trigger_single_node` 可独立验证节点逻辑，无需启动完整画布。 [NF-OPS]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 控制流执行器 | 系统的"肌肉层" | §1、DOC-ARCH-001 |
| ControlFlowExecutor | 并发调度器结构体 | §3.1 |
| 租户级并发度 | per-tenant 信号量 | §3.1 [NF-PER]【必須】 |
| 节点级并发度 | per-node 信号量 | §3.1 |
| Pause/Resume/Abort | 流程控制信号 | §3.2 [NF-AVA]【必須】 |
| ExecutionControlHandle | 控制信号发送句柄 | §3.2 |
| trigger_single_node | 单节点手动触发接口 | §3.3 |
| NodeExecutionResult | 节点执行结果结构体 | §3.1 |
| 调度并发 | futures::join_all 并发执行 | §3.1 |
| watch 通道 | tokio 跨任务信号通知原语 | §3.2 |

## 6. 参考文献

1. IPA「共通フレーム2018」(SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. tokio 公式ドキュメント「tokio — An asynchronous runtime for Rust」
4. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
