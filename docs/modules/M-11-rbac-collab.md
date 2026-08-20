# M-11 权限与协作（RBAC & Collab）

> **ドキュメントID**：DOC-MOD-011
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §11）
> **関連文書**：`docs/modules/M-10`（DOC-MOD-010）、`docs/modules/M-12`（DOC-MOD-012）
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

- **F-11** 权限与多人协作
- **F-17-04** 租户级 SSO / LDAP / AD 集成（由本模块在认证层承接）

### 1.2 关联用例

U-09 多团队画布协作与隔离、U-10 多租户 SaaS 部署

### 1.3 非功能需求

- 7.5 安全：审计日志与核心数据一样隔离

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于横切模块：RBAC 鉴权嵌入到 [M-13 API Gateway](../modules/M-13-api-gateway.md) 的中间件链；实时协作能力由前端 [M-12 §3 实时协作](../modules/M-12-canvas-editor-frontend.md) 与本模块的 WebSocket 中继服务协作承载。

### 2.2 认证流程（basic-design §6.1）

1. 用户登录 → JWT Token 生成（包含 `tenant_id`, `user_id`, `roles`）
2. 前端请求携带 Token → API Gateway 验证
3. Gateway 从 Token 中提取 `tenant_id` 并注入上下文

### 2.3 授权检查

- 请求中的 `tenant_id` 与 Token 中的 `tenant_id` 匹配？
- 用户在该租户中的角色是否有权限？
- 资源是否属于该租户？

## 3. 详细设计（詳細設計書）

### 3.1 RBAC 数据模型

```rust
pub enum Role {
    Owner,       // 租户拥有者，可管理计费/删除租户
    Admin,       // 可管理成员、权限、集成配置
    Editor,      // 可编辑画布
    Executor,    // 可触发画布执行，不可编辑
    Viewer,      // 只读
}

pub struct Permission {
    pub resource_type: ResourceType,   // Canvas | Workspace | Credential
    pub action: Action,                 // Read | Write | Execute | Delete | ShareManage
}

fn role_permissions(role: &Role) -> HashSet<Permission> {
    // 静态映射表，编译期常量，避免运行时重复计算
}
```

### 3.2 实时协作冲突解决（F-11-02）

采用 **CRDT（Conflict-free Replicated Data Type）** 方案，前端集成 Yjs，后端提供 WebSocket 中继与持久化：

```
协作时序：
  用户 A 编辑节点位置          用户 B 同时编辑同一节点的配置
        │                              │
        ▼                              ▼
  Yjs 本地文档更新（Y.Doc）      Yjs 本地文档更新（Y.Doc）
        │                              │
        ▼                              ▼
  生成增量更新（Update）广播 ──────► 后端 WebSocket 中继（M-11）
                                        │
                        ┌───────────────┴───────────────┐
                        ▼                                ▼
                  广播给其他在线协作者              持久化 Y.Doc 快照
                                                    （周期性 Snapshot 至 PostgreSQL）
```

由于 CRDT 的数学性质（可交换、可结合），节点位置移动与配置字段编辑分别落在 Y.Doc 的不同子结构（`Y.Map` 嵌套），天然避免大部分冲突；仅当两用户编辑**同一标量字段**时才需 Yjs 内置的 Last-Write-Wins 语义。

### 3.3 审计追溯（F-11-03）

所有经过 M-11 的写操作在提交前统一调用：

```rust
async fn record_audit_log(
    tenant_id: TenantId,
    user_id: UserId,
    action_type: &str,
    resource: (ResourceType, uuid::Uuid),
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
) {
    // 写入 audit_log 表，before/after 使用 JSON Patch (RFC 6902) 格式压缩存储差异
}
```

`audit_log` 表 DDL 见 [M-10 §4.2](../modules/M-10-tenant-middleware.md)。

### 3.4 协作冲突场景与处理

| 场景 | 处理 |
|---|---|
| 两用户同时编辑不同字段 | CRDT 自动合并，无冲突 |
| 两用户同时编辑同一标量字段 | Last-Write-Wins（LWW），以 Yjs 内部时钟为准 |
| 非协作模式下的版本冲突 | 乐观锁冲突，返回 `CONCURRENT_EDIT_CONFLICT`（HTTP 409）|

### 3.5 凭证访问审计

[M-10 §4.2 credential 表注释](../modules/M-10-tenant-middleware.md) 中明确"凭证访问需强制审计"：应用层在每次读取 `encrypted_payload` 前调用本模块 `record_audit_log`，`action_type = 'credential.access'`，便于合规追溯。

## 4. 验收要点

1. **RBAC 角色齐全**：Owner / Admin / Editor / Executor / Viewer 五种角色权限隔离正确。
2. **多用户实时协作**：≥ 3 用户同时编辑同一画布，前端不会出现状态错乱或丢更新。
3. **审计日志完整**：画布编辑、权限变更、凭证查看、数据导出、画布执行五类操作均记录到 `audit_log`，多租户环境下与核心数据一样隔离。
4. **乐观锁冲突**：非协作模式下，重复保存旧版本画布返回 409 `CONCURRENT_EDIT_CONFLICT`。 [NF-OPS]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| RBAC | Role-Based Access Control | §1、F-11 [NF-SEC]【必須】 |
| 5 种角色 | Owner / Admin / Editor / Executor / Viewer | §3.1 |
| Permission | 资源类型 + 操作的二元组 | §3.1 |
| CRDT | Conflict-free Replicated Data Type | §3.2 [NF-OPS]【必須】 |
| yrs | Yjs 的 Rust 移植版 | §3.2 |
| Y.Doc | yrs 文档实例 | §3.2 |
| LWW | Last-Write-Wins 冲突解决 | §3.2 |
| 审计日志 | 记录所有写操作 | §3.3 [NF-SEC]【必須】 |
| JSON Patch | RFC 6902 差异格式 | §3.3 |
| Awareness | 协作感知（光标/选中状态） | §3.2 |
| 邀请 token | 画布共享链接鉴权 | §3.3 [NF-SEC]【必須】 |
| 乐观锁 | 版本号控制并发写 | §3.4 [NF-OPS]【必須】 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. IETF「RFC 6902 — JavaScript Object Notation (JSON) Patch」
4. Yjs 公式ドキュメント「Yjs — Shared Data Types for Building Collaborative Applications」
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
