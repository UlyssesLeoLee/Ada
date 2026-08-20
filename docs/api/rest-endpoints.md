# REST API 端点清单

> **ドキュメントID**：DOC-API-001
> **文書分類**：横断文書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/modules/M-13`（DOC-MOD-013）
> **関連文書**：`docs/api/websocket-events.md`（DOC-API-002）、`docs/api/error-codes.md`（DOC-API-003）
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
2. 端点清单
3. 关键端点请求/响应示例
4. 认证与多租户上下文
5. GraphQL（可选）
6. 用語集
7. 参考文献

---

## 1. 概要

本文定义 Ada 系统的 REST API 端点规范。所有端点须经 API Gateway 鉴权与租户上下文注入（[DOC-MOD-010](../modules/M-10-tenant-middleware.md)），错误码遵循 [DOC-API-003](error-codes.md)。安全性 NF 标签：[NF-SEC]【必須】。

## 2. 端点清单

### 2.1 画布管理

```
POST   /api/v1/tenants/{tenant_id}/workspaces/{ws_id}/canvases
GET    /api/v1/tenants/{tenant_id}/workspaces/{ws_id}/canvases/{canvas_id}
PUT    /api/v1/tenants/{tenant_id}/workspaces/{ws_id}/canvases/{canvas_id}
DELETE /api/v1/tenants/{tenant_id}/workspaces/{ws_id}/canvases/{canvas_id}
```

### 2.2 执行管理

```
POST   /api/v1/tenants/{tenant_id}/canvases/{canvas_id}/execute
GET    /api/v1/tenants/{tenant_id}/executions/{exec_id}
GET    /api/v1/tenants/{tenant_id}/executions/{exec_id}/snapshots
```

### 2.3 多租户管理（仅租户 Owner / Admin）

```
POST   /api/v1/admin/tenants
PUT    /api/v1/admin/tenants/{tenant_id}
DELETE /api/v1/admin/tenants/{tenant_id}

GET    /api/v1/tenants/{tenant_id}/quota
PUT    /api/v1/tenants/{tenant_id}/quota
GET    /api/v1/tenants/{tenant_id}/audit-logs
```

### 2.4 插件连线校验

```
POST   /api/v1/canvas/validate-edge
```

请求体：

```json
{
  "source_node_id": "node_a",
  "target_node_id": "node_b"
}
```

实现：调用 [M-06 §3.3](../modules/M-06-node-runtime-plugin-sdk.md) 的 `validate_edge_compatibility` 判定两端 schema 兼容性。

## 3. 关键端点请求/响应示例

### 3.1 画布创建

```
POST /api/v1/tenants/{tenant_id}/workspaces/{workspace_id}/canvases
```

**Request Body**：

```json
{
  "name": "跨平台消息同步",
  "description": "...",
  "dag_json": { "nodes": [...], "edges": [...] }
}
```

**Response 201**：

```json
{
  "canvas_id": "uuid",
  "version": 1,
  "created_at": "2026-08-18T10:00:00Z"
}
```

**Response 403（租户配额超限）**：

```json
{
  "error_code": "QUOTA_EXCEEDED",
  "resource": "canvas_count",
  "limit": 50,
  "current": 50
}
```

### 3.2 画布执行触发

```
POST /api/v1/tenants/{tenant_id}/canvases/{canvas_id}/execute
```

**Request Body**：

```json
{
  "trigger_type": "manual",
  "entry_node_id": "node_001",
  "mock_input": null
}
```

**Response 202**：

```json
{
  "execution_id": "uuid",
  "status": "pending"
}
```

## 4. 认证与多租户上下文

- 所有端点需在请求头携带 `Authorization: Bearer <JWT>`，JWT Claims 包含 `tenant_id` / `user_id` / `roles`。
- 路径中含 `{tenant_id}` 时，[M-10 §3.1](../modules/M-10-tenant-middleware.md) 会校验路径 tenant 与 Token tenant 一致性，不一致返回 `403 TENANT_MISMATCH`。
- 详见 [DOC-API-003](error-codes.md)。

## 5. GraphQL（可选）

用于前端复杂查询，支持字段按需获取，减少网络开销（[DOC-BSC-001 §7.3](../legacy/basic-design.md)）。本期不强制实现。

## 6. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| REST | Representational State Transfer、HTTP ベース API 設計スタイル | §1 |
| JWT | JSON Web Token、署名付き認証トークン | §4 |
| 租户コンテキスト | リクエストに紐づくテナント ID 等の情報 | §4 |
| GraphQL | 必要フィールドだけ取得できるクエリ言語 | §5 |
| エンドポイント | API の URI 単位 | §2 |
| TLS 1.2+ | Transport Layer Security 1.2 以上 | §4（间接） |
| RBAC | Role-Based Access Control | §2.3 |
| 鉴权 (Authentication) | ユーザー身元確認 | §4 |
| 認可 (Authorization) | リソースアクセス権限の判定 | §4 |

## 7. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. IETF「RFC 7519 — JSON Web Token (JWT)」
4. GraphQL Foundation「GraphQL Specification」
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 基本設計書 v1.3.0」、2026-08-18（[DOC-BSC-001](../legacy/basic-design.md)）
6. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
