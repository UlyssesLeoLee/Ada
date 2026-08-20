# M-07 可视化调试（Debug Service）

> **ドキュメントID**：DOC-MOD-007
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §7）
> **関連文書**：`docs/modules/M-03`（DOC-MOD-003）、`docs/modules/M-04`（DOC-MOD-004）
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

- **F-08** 可视化调试

### 1.2 关联用例

U-05 可视化调试与追溯

### 1.3 数据要件

- 8.3 数据保留策略：节点执行快照（调试用）默认保留最近 20 次，可配置

## 2. 基本设计（基本設計書）

### 2.1 架构位置

横跨 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中的"血液层（Replay）"与"神经系统层（快照）"——既消费 [M-03 数据流引擎](../modules/M-03-data-flow-engine.md) 的快照能力，也消费 [M-04 编排引擎](../modules/M-04-orchestration-engine.md) 的 checkpoint 能力。

### 2.2 涉及表

| 表 | 用途 |
|---|---|
| `execution_node_snapshot` | 节点输入/输出快照、错误信息、耗时 |
| `execution_log` | 结构化日志（debug/info/warn/error） |

详见 [M-10 §4 数据库设计](../modules/M-10-tenant-middleware.md)。

### 2.3 主要能力

- 查看每个节点的输入/输出快照、执行日志、耗时
- 节点数据快照 JSON 树形展示
- 执行日志时间轴视图

## 3. 详细设计（詳細設計書）

### 3.1 快照保留策略（F-08-01）

- 每个节点执行后保留最近 N 次（默认 20 次，可配置）的输入/输出快照
- 超出部分由定时任务清理
- 大体积数据（>1MB）不入库，仅在对象存储中保留 `input_ref` / `output_ref` 引用

### 3.2 数据流引擎 Replay 联动

`M-07 调试服务` 可请求 `DataFlowEngine::replay_from_snapshot(execution_id, node_id)`，从持久化的节点输入快照重新构造 `NJson` 并重新注入对应 Edge 的队列。详见 [M-03 §3.3 数据缓存与重放](../modules/M-03-data-flow-engine.md)。

### 3.3 前端展示

调试面板 UI 由 [M-12 前端画布编辑器](../modules/M-12-canvas-editor-frontend.md) 承载，采用 HTML Overlay 形式（执行日志、数据快照 JSON 树选中/复制/搜索依赖浏览器原生能力更可靠，参见 [architecture/01-tech-stack.md §关键选型理由](../architecture/01-tech-stack.md)）。

```typescript
// 简化伪代码：调试面板订阅 WebSocket 事件
ws.on('canvas.node.status_changed', (event) => {
  debugPanel.appendEvent(event);  // 追加到时间轴
  if (event.status === 'success' || event.status === 'failed') {
    debugPanel.fetchSnapshot(event.execution_id, event.node_id)
      .then(snap => debugPanel.showSnapshotTree(snap));
  }
});
```

### 3.4 单节点手动触发

调试时由 [M-05 §3.3 单节点手动触发](../modules/M-05-control-flow-executor.md) 提供 `trigger_single_node` 接口，本模块负责展示触发前后的输入/输出对比。

## 4. 验收要点

1. **快照保留数量**：每个节点默认保留最近 20 次快照，超出部分被定时清理。
2. **快照可视化**：画布上可直接查看某次执行的数据快照 JSON 树形展示。
3. **时间轴视图**：执行日志时间轴视图，标注每个节点的开始/结束时间、耗时、状态（成功/失败/跳过）。
4. **Replay 可复现**：从历史快照 Replay 后的输出与历史一致（确定性的前提下）。
5. **大体积数据处理**：>1MB 的快照只存 `*_ref` 引用，调试面板通过单独 API 拉取实际数据。 [NF-OPS]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 可视化调试 | 节点输入输出快照查看、时间轴 | §1、F-08 |
| 快照保留策略 | 默认保留最近 20 次 | §3.1 |
| input_ref / output_ref | 对象存储引用，不入库 | §3.1 [NF-ENV]【必須】 |
| ExecutionNodeSnapshot | 节点快照表 | §2.2 [NF-OPS]【必須】 |
| ExecutionLog | 执行日志表 | §2.2 [NF-OPS]【必須】 |
| Replay | 从历史快照重放 | §3.2 [NF-AVA]【必須】 |
| NoSnapshotAvailable | 无快照错误 | §3.3 |
| 时间轴视图 | 各节点起止/耗时/状态 | §3.3 [NF-OPS]【必須】 |
| JSON 树 | 快照数据可视化 | §3.3 |
| 调试面板 | 前端调试 UI（HTML Overlay） | §3.3 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
