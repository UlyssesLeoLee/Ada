# エラーコード体系

> **ドキュメントID**：DOC-API-003
> **文書分類**：横断文書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：全モジュール（DOC-MOD-001～013）
> **関連文書**：`docs/api/rest-endpoints.md`（DOC-API-001）、`docs/api/websocket-events.md`（DOC-API-002）
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
2. 命名约定
3. 对外 HTTP Error Code
4. 模块内部 Rust 错误类型
5. 节点级失败 vs 请求级失败
6. 用語集
7. 参考文献

---

## 1. 概要

本文定义模块内部 Error 类型到对外 HTTP Error Code 的映射约定，以及节点级失败与请求级失败的区分。NF セキュリティ：[NF-SEC]【必須】、可用性：[NF-AVA]【必須】。

## 2. 命名约定

模块内部 `Error` 枚举変体名と対外 Error Code の対応関係は `{EnumVariant}` → `{SCREAMING_SNAKE_ERROR_CODE}` の機械的変換に従う（例：`AdapterError::AuthExpired` → `ADAPTER_AUTH_EXPIRED`）。API Gateway 層の統一 `From<XxxError> for ApiError` 実装で変換し、各 handler の手書き重複マッピングを避ける。

## 3. 对外 HTTP Error Code

| Error Code | HTTP Status | 説明 | 対応処理層 | NF タグ |
|---|---|---|---|---|
| `TENANT_MISMATCH` | 403 | 请求路径租户与 Token 租户不一致 | [M-10](../modules/M-10-tenant-middleware.md) | [NF-SEC]【必須】 |
| `QUOTA_EXCEEDED` | 429 | 配额超限 | [M-10](../modules/M-10-tenant-middleware.md) | [NF-PER]【必須】 |
| `ADAPTER_AUTH_EXPIRED` | 401 | 采集适配器凭证过期 | [M-01](../modules/M-01-acquisition-adapter.md) | [NF-SEC]【必須】 |
| `ADAPTER_NO_AVAILABLE_MODE` | 502 | API 与浏览器模式均不可用 | [M-01](../modules/M-01-acquisition-adapter.md) | [NF-AVA]【必須】 |
| `SELECTOR_NOT_FOUND` | 200*（节点级失败，不阻断 HTTP） | 页面选择器未匹配到元素 | [M-01](../modules/M-01-acquisition-adapter.md) | [NF-AVA]【必須】 |
| `SCHEMA_VALIDATION_FAILED` | 200*（节点级失败） | 数据未通过 JsonSchema 校验 | [M-02](../modules/M-02-normalizer.md) | [NF-SEC]【必須】 |
| `PLUGIN_RESOURCE_LIMIT_EXCEEDED` | 200*（节点级失败） | WASM 插件超出 CPU/内存限制 | [M-06](../modules/M-06-node-runtime-plugin-sdk.md) | [NF-PER]【必須】 |
| `EDGE_INCOMPATIBLE_SCHEMA` | 400 | 连线两端节点 Schema 不兼容 | [M-06](../modules/M-06-node-runtime-plugin-sdk.md) | [NF-OPS]【必須】 |
| `EXECUTION_NOT_FOUND` | 404 | 查询的执行记录不存在或不属于当前租户 | [M-04](../modules/M-04-orchestration-engine.md) | [NF-AVA]【必須】 |
| `CONCURRENT_EDIT_CONFLICT` | 409 | 画布版本冲突（非协作模式下的乐观锁冲突） | [M-11](../modules/M-11-rbac-collab.md) | [NF-OPS]【必須】 |

## 4. 模块内部 Rust 错误类型

各模块 `Error` 类型完整定義均集中在各自模块文件的"## 3. 詳細設計"节末尾，**不在本文件重複**。下表は索引のみ。

| 模块 | 内部 Error 类型 | 定義位置 |
|---|---|---|
| M-01 | `AdapterError` | [M-01 §3.2](../modules/M-01-acquisition-adapter.md) |
| M-01 | `PoolError` | detailed-design §14.1 |
| M-04 | `OrchestrationError` / `EvalError` | detailed-design §14.1 |
| M-04 | `StoreError` | detailed-design §14.1 |
| M-06 | `PluginError` | detailed-design §14.1 |
| M-07 | `DebugError` | detailed-design §14.1 |
| M-10 | `QuotaError` / `DeletionError` | detailed-design §14.1 |

所有内部 `Error` 均采用 `thiserror` 派生，由 API Gateway 层統一轉換為第 3 节的 HTTP Error Code。

## 5. 节点级失败 vs 请求级失败

注意第 3 节中有三个 Error Code 标记为 `200*（节点级失败）`：

- `SELECTOR_NOT_FOUND`
- `SCHEMA_VALIDATION_FAILED`
- `PLUGIN_RESOURCE_LIMIT_EXCEEDED`

这三类失败**不阻断 HTTP 请求本身**（仍返回 200），而是作为该节点本次执行的结果写入 [`ExecutionNodeSnapshot` 表](../modules/M-10-tenant-middleware.md)（参见 [M-04 §3.3 异常捕获与重试策略](../modules/M-04-orchestration-engine.md)）。前端通过 [WebSocket `canvas.node.status_changed` 事件](websocket-events.md) 感知节点失败详情。 [NF-AVA]【必須】

## 6. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| Error Code | 对外公開する機械可読エラー識別子 | §2 |
| EnumVariant | Rust 内部エラー型の変種 | §2 |
| HTTP Status | HTTP 応答コード（4xx/5xx） | §3 |
| ノード级失敗 | 個別ノード単位での失敗、HTTP 全体は成功 | §5 |
| リクエスト级失敗 | HTTP リクエスト全体としての失敗 | §3 |
| thiserror | Rust 用 derive macro エラー型生成ライブラリ | §4 |
| 楽観ロック | バージョン番号で競合を検出する制御方式 | §3 |
| 変換マッピング | 内部 Error → 対外 Error Code の対応表 | §2 |
| SCREAMING_SNAKE | 大文字スネークケース（例：TENANT_MISMATCH） | §2 |
| From trait | Rust の型変換トレイト | §2 |

## 7. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. IETF「RFC 7807 — Problem Details for HTTP APIs」
4. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
