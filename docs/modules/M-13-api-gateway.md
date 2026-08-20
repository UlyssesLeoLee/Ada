# M-13 API Gateway

> **ドキュメントID**：DOC-MOD-013
> **文書分類**：モジュール別設計書
> **バージョン**：v1.2.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/requirements.md`（DOC-REQ-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §13）
> **関連文書追加**：`docs/modules/M-14`（DOC-MOD-014）、`docs/modules/M-16`（DOC-MOD-016）
> **関連文書**：`docs/modules/M-10`（DOC-MOD-010）、`docs/modules/M-11`（DOC-MOD-011）
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
| v1.2.0 | 2026-08-19 | モジュール登録表連携ルーティング + クラスタノード認識追加（§5） | Ada プロジェクトチーム | TBD | TBD |

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

横切关注点，涉及 F-09 免安装分发的入口形式、F-11 权限与多人协作、F-15/F-16 凭证管理、F-17 多租户管理 API。

### 1.2 关联用例

U-01 ~ U-10 全部用例的入口点

### 1.3 接口要件

- I-01 前端 Web UI ↔ Runtime：通过本地 HTTP/WebSocket 通信（单机模式）或 HTTPS/WSS（多租户 SaaS 模式）
- I-06 多租户管理 API：提供租户/工作空间/团队/成员/权限/计费配额的管理 API

### 1.4 非功能需求

- 7.5 安全：所有通信走 HTTPS/WSS（TLS 1.2+）；多租户 SaaS 环境强制 HSTS 头
- 7.3 运用保守性：请求日志、跨域（CORS）与安全头设置

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中"贯穿所有层的横切关注点"——位于前端与后端服务（[M-01](../modules/M-01-acquisition-adapter.md) ~ [M-12](../modules/M-12-canvas-editor-frontend.md)）之间。

### 2.2 主要职责（basic-design §3.2.2）

- JWT/OAuth2 身份验证与授权
- 租户上下文提取与请求上下文绑定
- 速率限制（Per-tenant, Per-user）
- 请求日志与审计
- 跨域（CORS）与安全头设置
- 请求/响应转码（JSON/Protocol Buffers）

### 2.3 认证与安全（basic-design §6.1 + §6.3）

- **认证流程**：用户登录 → JWT Token 生成（包含 `tenant_id`, `user_id`, `roles`）→ 前端请求携带 Token → API Gateway 验证 → Gateway 从 Token 中提取 `tenant_id` 并注入上下文
- **授权检查**：
  - 请求中的 `tenant_id` 与 Token 中的 `tenant_id` 匹配？
  - 用户在该租户中的角色是否有权限？
  - 资源是否属于该租户？
- **数据传输**：所有通信走 HTTPS/WSS（TLS 1.2+），多租户 SaaS 环境强制 HSTS 头，证书由公信 CA 签发并自动轮换

### 2.4 错误处理总入口

所有模块内部 `Error`（[api/error-codes.md §3](../api/error-codes.md) 列出索引）经 API Gateway 层的统一 `From<XxxError> for ApiError` 实现转换为对外 HTTP Error Code（[api/error-codes.md §2](../api/error-codes.md)），避免各 handler 手写重复的映射逻辑。

## 3. 详细设计（詳細設計書）

### 3.1 中间件链

请求在 API Gateway 内的处理顺序：

```
HTTP Request
   │
   ▼
[1] CORS Middleware
   │
   ▼
[2] TLS / HSTS 检查（多租户 SaaS 模式）
   │
   ▼
[3] 请求日志中间件
   │
   ▼
[4] JWT 鉴权中间件
   │  ├─ 失败 → 401 Unauthorized
   │  └─ 成功 → 提取 claims（tenant_id, user_id, roles）
   ▼
[5] [M-10 多租户中间件](../modules/M-10-tenant-middleware.md)
       ├─ 校验路径 tenant_id 与 Token tenant_id 一致
       │  └─ 不一致 → 403 TENANT_MISMATCH
       ├─ 配额检查（QUOTA_EXCEEDED）
       └─ 注入 TenantContext 到请求扩展
   │
   ▼
[6] [M-11 权限中间件](../modules/M-11-rbac-collab.md) RBAC 检查
   │  └─ 权限不足 → 403 Forbidden
   ▼
[7] 路由到具体业务 handler
   │
   ▼
[8] 异常处理（统一 From<XxxError> for ApiError 转换）
   │
   ▼
HTTP Response
```

### 3.2 REST API 端点

完整端点清单见 [api/rest-endpoints.md](../api/rest-endpoints.md)。本节列举按业务域分类的概览：

| 业务域 | 端点数 | 鉴权要求 |
|---|---|---|
| 画布管理 | 4 | Editor 及以上 |
| 执行管理 | 3 | Executor 及以上（GET 路径）/ Editor 及以上（POST 路径） |
| 多租户管理 | 6 | Owner / Admin |
| 插件连线校验 | 1 | Editor 及以上 |

### 3.3 WebSocket 端点

```
ws://host/ws?token={jwt}&tenant_id={uuid}
```

事件协议详见 [api/websocket-events.md](../api/websocket-events.md)。WebSocket 鉴权失败返回 `4401 Unauthorized` 后关闭连接。

### 3.4 GraphQL（可选）

basic-design §7.3：用于前端复杂查询，支持字段按需获取，减少网络开销。本期不强制实现。

### 3.5 多租户管理 API（I-06）

提供租户/工作空间/团队/成员/权限/计费配额的管理 API，支持 REST 与 GraphQL 查询接口（GraphQL 可选）。具体端点见 [api/rest-endpoints.md §1.3](../api/rest-endpoints.md)。

## 4. 验收要点

1. **认证正确性**：JWT Token 过期/伪造/篡改的请求均被拒绝。
2. **多租户上下文强制**：未携带 Token 或 Token 不含 tenant_id 的请求被拒绝。
3. **错误码转换正确**：各模块内部 Error 经 API Gateway 转换后，对外 HTTP Error Code 与 [api/error-codes.md §2](../api/error-codes.md) 完全一致。
4. **限流生效**：Per-tenant / Per-user 限流在测试场景下正确触发（HTTP 429）。
5. **HTTPS/WSS 强制**：多租户 SaaS 模式下所有通信走 TLS 1.2+。
6. **审计日志**：所有通过 API Gateway 的请求记录到 [`audit_log`](../modules/M-10-tenant-middleware.md)，多租户环境下与核心数据一样隔离。 [NF-SEC]【必須】
7. **模块路由表动态更新**：m01-acquisition 升级激活后，新版本提供的路由立即可见，旧版本 drain 期间双版本共存。 [NF-AVA]【必須】
8. **集群节点感知**：A 节点失联 30s 内从路由表移除，请求 0 失败。 [NF-AVA]【必須】

## 5. v1.2.0 追加：模块路由 + 集群感知（[DOC-MOD-014 / 016](../modules/M-14-module-registry.md) 配套）

### 5.1 模块注册表感知路由

API Gateway 启动时从 [`module_registry`](../modules/M-10-tenant-middleware.md) 加载所有 `active=TRUE` 的模块，构造路由表：

```
GET /api/v1/canvases/{id}/acquire
  → m01-acquisition
  → 当前 active 版本
  → 在提供此路由的节点中按 load_factor 升序选择
```

模块升级时（[DOC-MOD-014 §3.3](../modules/M-14-module-registry.md)）：
1. `atomic_module_swap` 在 DB 中切换 active
2. 触发 `module.swapped` 事件
3. 各节点 API Gateway 订阅该事件，更新本地路由表
4. 新版本路由立即可用，旧版本在 drain 完成后从路由表移除

### 5.2 集群节点感知

启动时通过 `SELECT * FROM cluster_node WHERE state='Active' AND last_heartbeat_at > now() - interval '30 seconds'` 获取健康节点列表。

每个请求处理流程：

```
1. 中间件链鉴权 + 注入 TenantContext
2. 解析目标路由
3. 在 [DOC-MOD-016 §3.3 服务发现](../modules/M-16-cluster-coordinator.md) 返回的节点列表中
   按 load_factor 升序选 1 个
4. HTTP 代理或 WebSocket 转发
5. 失败 → 选次优节点重试（最多 2 次）
6. 全部失败 → 503 + 触发节点健康检查事件
```

### 5.3 与 M-15 事件联动

API Gateway 进程订阅 M-15 事件总线的以下 topic：

| Topic | 触发 | Gateway 动作 |
|---|---|---|
| `module.swapped` | 模块升级 | 更新本地路由表 |
| `module.unloaded` | 模块卸载 | 移除路由 |
| `cluster.node_removed` | 节点摘除 | 从负载均衡池移除 |
| `cluster.leader_elected` | Leader 变化 | 刷新 singleton 路由 |

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| API Gateway | 系统入口中间件集合 | §1、M-13 |
| 中间件链 | CORS→日志→JWT→tenant→RBAC→handler | §3.1 |
| JWT | JSON Web Token | §3.1 [NF-SEC]【必須】 |
| TenantContext | 注入请求的租户上下文 | §3.1 |
| 限流 | Per-tenant / Per-user | §3.5 [NF-PER]【必須】 |
| TLS 1.2+ | 传输加密 | §2.3 [NF-SEC]【必須】 |
| HSTS | HTTP Strict Transport Security | §2.3 [NF-SEC]【必須】 |
| CORS | Cross-Origin Resource Sharing | §3.1 [NF-SEC]【必須】 |
| GraphQL | 可选查询语言 | §3.4 |
| 多租户管理 API | I-06 端点 | §3.5 [NF-SEC]【必須】 |
| 错误码转换 | From<XxxError> for ApiError | §2.4 |
| 审计日志 | 全部请求记录 | §4.6 [NF-SEC]【必須】 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. IETF「RFC 7519 — JSON Web Token (JWT)」
4. IETF「RFC 6797 — HTTP Strict Transport Security (HSTS)」
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 基本設計書 v1.3.0」、2026-08-18（[DOC-BSC-001](../legacy/basic-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
