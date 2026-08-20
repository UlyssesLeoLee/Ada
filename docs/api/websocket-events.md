# WebSocket イベント推送协议

> **ドキュメントID**：DOC-API-002
> **文書分類**：横断文書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/modules/M-11`（DOC-MOD-011）、`docs/modules/M-12`（DOC-MOD-012）
> **関連文書**：`docs/api/rest-endpoints.md`（DOC-API-001）、`docs/api/error-codes.md`（DOC-API-003）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018)
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

1. 概要
2. 接続仕様
3. サーバー → クライアント イベント一覧
4. イベント schema 例
5. クライアント → サーバー（CRDT 同期）
6. 配额预警
7. 用語集
8. 参考文献

---

## 1. 概要

本文定义 WebSocket 长连接的事件推送协议，覆盖执行状态、协作同步、配额预警等实时场景。NF 性能要求：[NF-PER]【必須】、セキュリティ：[NF-SEC]【必須】。

## 2. 接続仕様

```
ws://host/ws?token={jwt}&tenant_id={uuid}
```

- 鉴权：接続時に JWT と tenant_id を携带；服务端按 [M-10 §3.1](../modules/M-10-tenant-middleware.md) 注入租户コンテキスト。
- 鉴权失敗：服务端返回 `4401 Unauthorized` 后关闭。 [NF-SEC]【必須】

## 3. サーバー → クライアント イベント一覧

| type | 触发时机 | NF タグ |
|---|---|---|
| `canvas.node.status_changed` | 节点状态变更（Pending→Running→Success） | [NF-PER]【必須】 |
| `canvas.dataflow.metrics` | 每 1s 推送一次连线吞吐量指标 | [NF-PER]【必須】 |
| `canvas.execution.completed` | 整个画布执行完成 | [NF-AVA]【必須】 |
| `canvas.execution.failed` | 执行失败且无法自动恢复 | [NF-AVA]【必須】 |
| `collab.awareness_update` | 其他协作者光标/选中状态变化 | [NF-OPS]【推奨】 |
| `collab.doc_update` | Yjs CRDT 增量更新 | [NF-OPS]【必須】 |
| `tenant.quota_warning` | 配额使用达到 80% 阈值 | [NF-OPS]【必須】 |

## 4. イベント schema 例

### 4.1 节点実行完了

```json
{
  "type": "canvas.node.executed",
  "data": {
    "execution_id": "uuid",
    "node_id": "node_001",
    "status": "success",
    "output": { ... }
  }
}
```

### 4.2 データフロー指標

驱动画布上连线的"流光动效"渲染速度与颜色（堆积越多颜色越偏红）。指标来源参见 [M-03 §3.4](../modules/M-03-data-flow-engine.md)：

```
ada_dataflow_throughput_total{tenant_id, canvas_id, edge_id}
ada_dataflow_queue_depth{tenant_id, canvas_id, edge_id}
ada_dataflow_dropped_total{tenant_id, canvas_id, edge_id, reason}
```

### 4.3 协作感知

```json
{
  "type": "canvas.user_editing",
  "data": {
    "user_id": "uuid",
    "node_id": "node_002",
    "change_type": "selected"
  }
}
```

## 5. クライアント → サーバー（CRDT 同期）

`collab.doc_update` 方向相反，客户端发送 Yjs 增量更新，服务端：

1. 验证客户端 tenant 与该画布 tenant 一致（[M-11 §3.2](../modules/M-11-rbac-collab.md)）；[NF-SEC]【必須】
2. 应用到服务端 Y.Doc 实例；
3. 中继给同画布的其他在线协作者；[NF-PER]【必須】
4. 周期性 Snapshot 至 PostgreSQL。

## 6. 配额预警

`tenant.quota_warning` 在 [M-10 §3.2 配额检查](../modules/M-10-tenant-middleware.md) 检测到使用率 ≥ 80% 时主动推送，便于前端提前提示用户。

## 7. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| WebSocket | 双方向フルデュプレックス通信プロトコル | §1 |
| イベント | サーバーがクライアントに通知する JSON メッセージ | §3 |
| CRDT | Conflict-free Replicated Data Type、競合解決不要の分散データ型 | §5 |
| Yjs / yrs | JavaScript / Rust 実装 CRDT フレームワーク | §5 |
| Awareness | 协作中の存在情報（カーソル・選択状態等） | §3 |
| snapshot | ある時点の CRDT 状態永続化 | §5.4 |
| 流光动效 | データフロー量を視覚化するアニメーション | §4.2 |
| 配额 | 多租户场景でのリソース上限 | §6 |
| WSS | WebSocket over TLS、暗号化された WebSocket | §2 |
| JWT | JSON Web Token、署名付き認証トークン | §2 |

## 8. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. IETF「RFC 6455 — The WebSocket Protocol」
4. Yjs 公式ドキュメント「Yjs — Shared Data Types for Building Collaborative Applications」
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 基本設計書 v1.3.0」、2026-08-18（[DOC-BSC-001](../legacy/basic-design.md)）
6. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
