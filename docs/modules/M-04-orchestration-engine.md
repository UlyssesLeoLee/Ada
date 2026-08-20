# M-04 编排引擎（Orchestration Engine）

> **ドキュメントID**：DOC-MOD-004
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §4）、`docs/tests/IT-design.md`（DOC-TST-002 §2）
> **関連文書**：`docs/modules/M-05`（DOC-MOD-005）、`docs/modules/M-07`（DOC-MOD-007）
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

- **F-05** 编排引擎（神经系统层）

### 1.2 关联用例

U-01 跨平台内容同步、U-03 事件触发式自动化、U-04 数据清洗与人工复核、U-06 IM 消息双向联动

### 1.3 非功能需求

- 7.1 可用性：断点续传（Runtime 意外退出后重启应能基于最近一次持久化状态自动恢复未完成的画布执行）
- 7.3 运用保守性：结构化日志（JSON Lines）、节点/画布/时间过滤

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中的"神经系统层（LangGraph）"，对应 §2 表中"神经系统"行。

### 2.2 职责定位

编排引擎只做**决策**，不做体力劳动。依据 §2.1 设计原则："神经系统只做决策，不做体力劳动"——编排引擎（LangGraph 层）只负责状态判断与路径选择，实际的节点执行与并发调度下放给 [M-05 控制流执行器（肌肉层）](../modules/M-05-control-flow-executor.md)。

### 2.3 主要职责（basic-design §3.2.3）

- 解析画布配置（DAG/StateGraph）为执行计划
- 状态管理与状态转移规则
- 条件分支、循环、异常捕获判决逻辑
- 执行历史记录与快照持久化
- 支持断点续传（Resume from checkpoint）

### 2.4 关键设计

- **状态不可变性（Immutable State）**：每次转移生成新状态，便于回放与调试
- **异步执行计划（Async Execution Plan）**：支持并发节点调度
- **中间件系统（Middleware）**：便于插入日志、监控、重试等横切关注点

## 3. 详细设计（詳細設計書）

### 3.1 状态机核心循环

```rust
pub struct OrchestrationEngine {
    canvas_def: CanvasDefinition,
    state_store: Arc<dyn StateStore>,     // 持久化后端（PostgreSQL）
}

impl OrchestrationEngine {
    pub async fn run(&self, initial_state: ExecutionState) -> Result<ExecutionState, OrchestrationError> {
        let mut state = initial_state;

        loop {
            // 1. 计算当前可执行的节点集合（依赖已满足且状态为 Pending）
            let runnable = self.compute_runnable_nodes(&state);

            if runnable.is_empty() {
                if self.all_terminal(&state) {
                    break;  // 执行完成
                }
                // 等待外部事件（如 HumanReview 节点的用户确认）
                state = self.wait_for_external_event(state).await?;
                continue;
            }

            // 2. 交由控制流执行器（M-05）并发调度执行这些节点
            let results = self.control_flow_executor
                .dispatch(&runnable, &state).await;

            // 3. 依据执行结果做状态迁移（生成新的不可变 ExecutionState）
            state = self.transition(state, results)?;

            // 4. 持久化状态快照（用于断点续传）
            self.state_store.checkpoint(&state).await?;
        }

        Ok(state)
    }
}
```

### 3.2 条件分支/循环/汇聚节点的语义

```rust
/// 条件判断节点求值逻辑
fn evaluate_condition(node: &NodeDefinition, state: &ExecutionState) -> Result<String, EvalError> {
    // node.config 中存储条件表达式与分支映射: { "expr": "...", "branches": {"true": "node_b", "false": "node_c"} }
    let predicate_result: bool = expression_engine::eval_bool(&node.config["predicate"], &state.variables)?;
    let branch_key = if predicate_result { "true" } else { "false" };
    Ok(node.config["branches"][branch_key].as_str().unwrap().to_string())
}

/// 循环节点：对 collection_expr 求值得到的集合逐一生成子执行上下文
fn expand_loop_node(node: &NodeDefinition, state: &ExecutionState) -> Vec<LoopIteration> {
    let items: Vec<serde_json::Value> = expression_engine::eval_array(&node.config["collection_expr"], &state.variables).unwrap_or_default();
    items.into_iter().enumerate()
        .map(|(idx, item)| LoopIteration { index: idx, item, parent_node: node.node_id.clone() })
        .collect()
}

/// 汇聚（Join/Merge）节点：等待所有上游分支到达后才继续
fn is_merge_ready(node: &NodeDefinition, state: &ExecutionState, upstream_edges: &[EdgeDefinition]) -> bool {
    upstream_edges.iter().all(|e| {
        matches!(state.node_statuses.get(&e.from_node), Some(NodeStatus::Success{..}) | Some(NodeStatus::Skipped{..}))
    })
}
```

### 3.3 异常捕获与重试策略（F-05-02, F-05-03）

```
节点执行失败时的处理流程：

节点执行 → 返回 Err(e)
   │
   ▼
[1] 查询该节点的 RetryPolicy
   │
   ├─ 未超过 max_attempts → 依据 BackoffStrategy 计算延迟 → 调度重试
   │
   └─ 已达 max_attempts:
        │
        ├─ 若该节点存在"异常分支"出边（EdgeDefinition.condition == "on_error"）:
        │     路由至异常处理节点，状态置为 Failed，但整体执行继续
        │
        └─ 否则:
              整体 ExecutionState 标记为失败，触发上层通知（Webhook/UI 提示）
```

指数退避计算公式：

```
delay(attempt) = min(base_ms * multiplier^(attempt-1), max_ms) + random_jitter(0, base_ms * 0.1)
```

### 3.4 LLM 语义决策节点设计（F-05-04）

```rust
pub struct LlmDecisionNode {
    pub prompt_template: String,      // 支持 {{payload.fields.xxx}} 插值
    pub llm_endpoint: LlmEndpointConfig,
    pub output_branches: Vec<String>, // 期望 LLM 从这些候选分支中选择一个
    pub fallback_branch: String,      // LLM 调用失败或返回非法值时的兜底分支
}

async fn evaluate_llm_decision(node: &LlmDecisionNode, njson: &NJson) -> String {
    let prompt = render_template(&node.prompt_template, njson);
    match call_llm(&node.llm_endpoint, &prompt, &node.output_branches).await {
        Ok(branch) if node.output_branches.contains(&branch) => branch,
        _ => node.fallback_branch.clone(),
    }
}
```

### 3.5 状态持久化与断点续传（对应 7.1 可用性要件）

`StateStore` 接口的 PostgreSQL 实现每次 `checkpoint` 写入 `canvas_execution` 表的增量字段（`node_statuses` 以 JSONB 存储），Runtime 重启后通过 `ExecutionId` 恢复：

```rust
#[async_trait]
pub trait StateStore: Send + Sync {
    async fn checkpoint(&self, state: &ExecutionState) -> Result<(), StoreError>;
    async fn load_latest(&self, execution_id: ExecutionId) -> Result<Option<ExecutionState>, StoreError>;
}
```

## 4. 验收要点

1. **三种控制逻辑**：编排引擎能正确执行条件分支、循环、异常重试三类基础控制逻辑的测试用例（[architecture/03-cross-cutting-risks.md §4.5](../architecture/03-cross-cutting-risks.md)）。
2. **断点续传**：Runtime 进程被 kill -9 后重启，相同 `execution_id` 能从最近一次 checkpoint 恢复，未完成节点继续执行。
3. **状态不可变**：每次状态迁移生成新版本号，调试时能按 version 定位历史状态。
4. **LLM 决策兜底**：LLM 调用失败或返回非法值时，自动走 `fallback_branch` 不阻塞主流程。
5. **执行历史可追溯**：`ExecutionLog` 表记录每个节点开始/结束时间、耗时、状态，配合 [M-07 调试服务](../modules/M-07-debug-service.md) 提供时间轴视图。 [NF-OPS]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 编排引擎 | 系统的"决策中枢"，基于状态图 | §1、DOC-ARCH-001 |
| StateGraph | 状态图模型 | §3.1 |
| ExecutionState | 运行时不可变状态 | §3.1 |
| 条件分支 | 节点条件判断与多路径选择 | §3.2 [NF-OPS]【必須】 |
| 循环节点 | 对集合迭代执行 | §3.2 |
| 汇聚节点 | 等待所有上游到达才继续 | §3.2 |
| 异常分支 | 节点失败时的 on_error 路径 | §3.3 [NF-AVA]【必須】 |
| RetryPolicy | 重试策略（max_attempts + 退避） | §3.3 [NF-AVA]【必須】 |
| 指数退避 | 退避算法（base_ms × multiplier^n） | §3.3 |
| LLM 决策节点 | 基于大模型语义判断的分支节点 | §3.4 |
| 断点续传 | Runtime 重启后从 checkpoint 恢复 | §3.5 [NF-AVA]【必須】 |

## 6. 参考文献

1. IPA「共通フレーム2018」(SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. LangGraph 公式ドキュメント「LangGraph — Stateful Multi-Actor Applications with LLMs」
4. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
