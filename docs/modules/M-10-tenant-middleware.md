# M-10 多租户中间件（Tenant Middleware）

> **ドキュメントID**：DOC-MOD-010
> **文書分類**：モジュール別設計書
> **バージョン**：v1.2.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §10）、`docs/tests/IT-design.md`（DOC-TST-002 §5）
> **関連文書**：`docs/modules/M-11`（DOC-MOD-011）、`docs/modules/M-14`（DOC-MOD-014）、`docs/modules/M-15`（DOC-MOD-015）、`docs/modules/M-16`（DOC-MOD-016）
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
| v1.2.0 | 2026-08-19 | M-14/M-15/M-16 関連 DDL 追加（§4.3-§4.5）+ PL/pgSQL 存过 5 本（§4.6） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 需求来源（要件定義書）
2. 基本设计（基本設計書）
3. 詳細设计（詳細設計書）
4. 数据库设计
5. 验收要点
6. 用語集
7. 参考文献

---

## 1. 需求来源（要件定義書）

### 1.1 涉及 F-IDs

- **F-17** 多租户与工作空间管理

### 1.2 关联用例

U-09 多团队画布协作与隔离、U-10 多租户 SaaS 部署

### 1.3 非功能需求

- 7.5 安全：多租户数据隔离**必须满足**数据库级、存储级、网络隔离
- 7.3 运用保守性：审计日志保留期可由租户管理员配置

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中的"贯穿所有层的横切关注点"——所有 API 请求必经中间件，事务级注入 `app.current_tenant` 会话变量。

### 2.2 三层隔离模型（basic-design §4.1）

#### 第一层：逻辑隔离（应用层）

所有 API 端点、业务逻辑均需在函数签名处显式传递 `tenant_id`：

```rust
async fn get_canvas(
    tenant_id: TenantId,
    canvas_id: CanvasId,
    req_user: AuthUser,
) -> Result<Canvas> {
    // 显式检查：用户属于 tenant_id 吗？
    if req_user.tenant_id != tenant_id {
        return Err(Unauthorized);
    }
    // 数据库查询自动注入 tenant_id 条件
    db.canvas.get(canvas_id)
        .filter(|c| c.tenant_id == tenant_id)
        .await
}
```

#### 第二层：数据库隔离

**行级安全（RLS，Row-Level Security）** 在 PostgreSQL 中实现：

```sql
-- Canvas 表（会话变量统一命名为 app.current_tenant，与 5.2 节全部建表语句及
-- 詳細設計書 10.1 節 TenantContextMiddleware 的实现保持一致，避免变量名不匹配导致 RLS 静默失效）
CREATE POLICY canvas_tenant_isolation ON canvas
  USING (tenant_id = current_setting('app.current_tenant', true)::uuid);

-- 查询前设置租户上下文
SET app.current_tenant TO 'uuid-of-tenant-1';
SELECT * FROM canvas;  -- 自动过滤
```

**备选方案**（无 RLS 时）：应用层显式过滤。

#### 第三层：物理隔离（部署层）

对于大型企业或高合规性需求（FedRamp/SOC2），支持：

- **Kubernetes 命名空间隔离**：为关键租户分配独立的 Pod/Node
- **专用数据库实例**：超大型租户拥有独立数据库副本
- **数据驻留地限制**：某些租户的数据只能存储在特定地域

### 2.3 多租户计费与配额模型

```rust
struct TenantQuota {
    tenant_id: TenantId,
    
    // 计算资源
    concurrent_canvas_executions: u32,          // 同时执行的画布数
    concurrent_playwright_instances: u32,       // 浏览器实例数
    
    // 存储资源
    total_storage_gb: u64,                      // 总存储容量
    snapshot_retention_days: u32,               // 快照保留天数
    
    // 网络资源
    api_calls_per_hour: u32,                    // 小时 API 调用限额
    webhook_calls_per_hour: u32,                // 出站 Webhook 调用限额
    
    // 可选附加功能
    enable_sso: bool,
    enable_custom_plugins: bool,
    enable_llm_semantic_nodes: bool,
}
```

### 2.4 租户生命周期管理

```
创建 → 激活 → [执行中/暂停] → 删除
       ↓
   [数据保留期 7 天内可恢复]
       ↓
   [7 天后永久清理]
```

删除租户时需自动清理：

- 所有画布、执行记录、快照
- 凭证库中该租户的敏感数据
- 审计日志（保留 1 年或按法规要求）
- 计费/配额记录（保留 7 年）

## 3. 详细设计（詳細設計書）

### 3.1 请求上下文注入中间件（Actix-web Middleware）

```rust
pub struct TenantContextMiddleware;

impl<S> Transform<S, ServiceRequest> for TenantContextMiddleware
where S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>
{
    fn call(&self, req: ServiceRequest) -> Self::Future {
        // 1. 从 JWT Claims 中提取 tenant_id, user_id, roles
        let claims = extract_jwt_claims(&req)?;

        // 2. 若请求路径中也包含 tenant_id（如 /api/v1/tenants/{tenant_id}/...），
        //    校验路径中的 tenant_id 与 Token 中的 tenant_id 是否一致
        if let Some(path_tenant_id) = extract_path_tenant_id(&req) {
            if path_tenant_id != claims.tenant_id {
                return Err(ErrorForbidden("tenant_mismatch"));
            }
        }

        // 3. 将 TenantContext 注入请求扩展，供后续 handler 与数据库层使用
        req.extensions_mut().insert(TenantContext {
            tenant_id: claims.tenant_id,
            user_id: claims.user_id,
            roles: claims.roles,
        });

        self.service.call(req)
    }
}
```

**关键修正：RLS 会话变量的正确设置位置与方式**

上一版设计中间件直接对连接池调用 `SET`，存在两个缺陷需要修正：（1）字符串拼接 SQL 是注入反面写法；（2）`SET` 是连接级状态，从池中任取一条连接执行后无法保证请求后续的实际查询复用同一条连接，等于没设置。正确做法是**在每次数据库访问的事务开始处**，用参数化的 `set_config()` 加 `SET LOCAL` 语义（第三参数 `is_local = true`，仅在当前事务内生效，事务结束自动重置，天然避免连接池残留问题）：

```rust
/// 每次数据库操作在获取连接、开启事务后，事务内第一步调用本函数，
/// 而不是在中间件里对连接池整体调用 —— 从根本上避免连接复用导致的 RLS 失效
async fn with_tenant_scope<T>(
    pool: &PgPool,
    tenant_id: TenantId,
    f: impl FnOnce(&mut Transaction<'_, Postgres>) -> BoxFuture<'_, Result<T, DbError>>,
) -> Result<T, DbError> {
    let mut tx = pool.begin().await?;

    // set_config 为参数化函数调用，非字符串拼接；is_local=true 等价于 SET LOCAL，
    // 仅在本事务内生效，COMMIT/ROLLBACK 后自动清除，无需手动 RESET
    sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await?;

    let result = f(&mut tx).await?;
    tx.commit().await?;
    Ok(result)
}
```

由于该方案以事务为作用域自动重置，连接池防御设计不依赖连接归还钩子手动 `RESET`（该方式在钩子被跳过、异常路径未触发时仍有残留风险），而是从设计上让会话变量的生命周期与事务严格绑定（`SET LOCAL`）。这样即使连接池实现遗漏归还钩子，残留也不可能跨越事务边界。单元测试仍需覆盖"连接复用后前一事务的会话变量不可见"这一场景（见 17.2 節 TC-MT-002），作为该设计假设的回归验证，而非依赖钩子本身。

### 3.2 配额检查与限流

```rust
pub struct QuotaEnforcer {
    quota_cache: Arc<DashMap<TenantId, TenantQuota>>,   // 定期从数据库刷新，TTL 60s
}

impl QuotaEnforcer {
    pub async fn check_and_reserve(&self, tenant_id: TenantId, resource: QuotaResource) -> Result<(), QuotaError> {
        let quota = self.get_quota(tenant_id).await?;
        let current_usage = self.get_current_usage(tenant_id, &resource).await?;

        match resource {
            QuotaResource::ConcurrentExecution => {
                if current_usage >= quota.concurrent_canvas_executions {
                    return Err(QuotaError::Exceeded {
                        resource: "concurrent_canvas_executions".into(),
                        limit: quota.concurrent_canvas_executions,
                    });
                }
            }
            QuotaResource::ApiCallsPerHour => { /* 类似检查，基于滑动窗口计数器（Redis） */ }
            QuotaResource::StorageBytes => { /* 类似检查 */ }
        }
        Ok(())
    }
}
```

### 3.3 租户生命周期状态机

```
         create_tenant
              │
              ▼
         ┌─────────┐   suspend()    ┌───────────┐
         │ active  │ ─────────────► │ suspended  │
         └─────────┘ ◄───────────── └───────────┘
              │           resume()
              │ delete_tenant()
              ▼
         ┌──────────────────┐
         │ pending_deletion  │  (软删除，保留期 7 天)
         └──────────────────┘
              │ 定时任务（每日执行）
              │ retention_period_expired
              ▼
         ┌──────────┐
         │ deleted   │  (物理清除全部数据)
         └──────────┘
```

```rust
/// 対応 8.3 節数据保留策略中的租户清理合规性审计要求
pub struct DeletionReport {
    pub tenant_id: TenantId,
    pub deleted_at: DateTime<Utc>,
    pub rows_deleted_by_table: HashMap<String, u64>,   // 表名 → 删除行数，逐表统计供审计
    pub credential_keys_revoked: Vec<String>,           // 已通知 KMS 吊销的密钥引用列表
}

pub async fn hard_delete_tenant_data(tenant_id: TenantId) -> Result<DeletionReport, DeletionError> {
    // 事务性删除，按外键依赖倒序执行：
    // 1. execution_node_snapshot, execution_log
    // 2. canvas_execution
    // 3. canvas_version, canvas
    // 4. credential（凭证库，需先安全擦除加密密钥引用）
    // 5. audit_log（若审计策略允许，否则归档至冷存储后删除）
    // 6. workspace, team, tenant_user
    // 7. tenant
    // 每步删除后写入 DeletionReport 供合规审计
}
```

### 3.4 数据库行级安全（RLS）策略实现细节

对 5.2 节列出的每张多租户表，均需附加以下标准 RLS 策略模板：

```sql
ALTER TABLE {table_name} ENABLE ROW LEVEL SECURITY;

CREATE POLICY {table_name}_tenant_isolation ON {table_name}
  FOR ALL
  USING (tenant_id = current_setting('app.current_tenant', true)::uuid)
  WITH CHECK (tenant_id = current_setting('app.current_tenant', true)::uuid);

-- app.current_tenant 通过 10.1 節 with_tenant_scope 以 SET LOCAL 语义（set_config 第三参数 true）
-- 在事务开始处设置，事务提交/回滚后由 PostgreSQL 自动清除，天然不会跨事务/跨连接归还残留
```

## 4. 数据库设计（核心表 DDL 集中在此模块）

> **拆分约定**：数据库表 DDL 涉及多个模块的读写，集中在 M-10 模块文件中维护；其他模块文件以"涉及表"方式引用。

### 4.1 核心数据模型（ER 图概览）

```
Tenant
  ├─ TenantUser (与 AppUser 多对多，含角色) ─── AppUser (全局账号，可跨租户)
  ├─ ConnectorTemplate (F-16 通用连接器模板)
  ├─ ConnectorSyncState (F-15 增量同步游标，按 adapter_id 维度)
  ├─ Workspace
  │  ├─ Canvas
  │  │  ├─ Node (节点定义，内嵌于 dag_json)
  │  │  ├─ Edge (连线定义，内嵌于 dag_json)
  │  │  ├─ CanvasVersion (版本快照)
  │  │  └─ CanvasExecution (执行记录)
  │  │      ├─ ExecutionNodeSnapshot (节点快照)
  │  │      └─ ExecutionLog (日志)
  │  ├─ Team
  │  │  └─ TeamUser (与 AppUser 多对多)
  │  └─ Credential (凭证库，加密存储)
  │
  └─ AuditLog (审计日志)
```

> 注：`Node`/`Edge` 未独立建表，而是作为 `CanvasDefinition`（[detailed-design §3.2](../legacy/detailed-design.md)）整体序列化存入 `canvas.dag_json`（JSONB）。选择整体存储而非归一化拆表的理由：画布编辑是"整体读写"场景（前端一次性加载/保存全部节点连线），拆表会引入不必要的 JOIN 与事务复杂度；`CanvasVersion` 表（见下）通过存储每次发布的 `dag_json` 快照来支持版本回滚。

### 4.2 关键表 DDL

#### Tenant 表

```sql
CREATE TABLE tenant (
  id UUID PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  status ENUM('active', 'suspended', 'deleted') DEFAULT 'active',
  created_at TIMESTAMP WITH TIME ZONE,
  updated_at TIMESTAMP WITH TIME ZONE,
  deleted_at TIMESTAMP WITH TIME ZONE,  -- 软删除
  
  -- 计费相关
  plan_type ENUM('free', 'pro', 'enterprise'),
  quota_json JSONB,  -- TenantQuota 序列化
  
  -- 合规相关
  data_residency VARCHAR(100),  -- 数据驻留地，如 'US', 'EU'
  require_mfa BOOLEAN DEFAULT false,
  sso_enabled BOOLEAN DEFAULT false,
  sso_config_json JSONB
);
```

#### Workspace 表

```sql
CREATE TABLE workspace (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenant(id),

  name VARCHAR(255) NOT NULL,
  description TEXT,

  created_by UUID,
  created_at TIMESTAMP WITH TIME ZONE,
  updated_at TIMESTAMP WITH TIME ZONE,
  deleted_at TIMESTAMP WITH TIME ZONE,  -- 软删除

  UNIQUE(tenant_id, id)
);

ALTER TABLE workspace ENABLE ROW LEVEL SECURITY;
CREATE POLICY workspace_rls ON workspace
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);
```

#### User / TenantUser 表

```sql
-- User 为全局账号表（一个自然人可跨多个租户，如同一邮箱受邀加入不同企业租户）
CREATE TABLE app_user (
  id UUID PRIMARY KEY,
  email VARCHAR(255) NOT NULL UNIQUE,
  display_name VARCHAR(255),
  password_hash VARCHAR(255),         -- 若启用 SSO/OAuth 登录则可为空
  mfa_enabled BOOLEAN DEFAULT false,
  created_at TIMESTAMP WITH TIME ZONE,
  last_login_at TIMESTAMP WITH TIME ZONE
);

-- TenantUser 为用户与租户的多对多关系，携带该用户在该租户下的角色（対応 F-11 RBAC）
CREATE TABLE tenant_user (
  tenant_id UUID NOT NULL REFERENCES tenant(id),
  user_id UUID NOT NULL REFERENCES app_user(id),
  role VARCHAR(20) NOT NULL,          -- 'owner' | 'admin' | 'editor' | 'executor' | 'viewer'
  invited_by UUID,
  joined_at TIMESTAMP WITH TIME ZONE,
  status VARCHAR(20) DEFAULT 'active', -- 'invited' | 'active' | 'suspended'

  PRIMARY KEY (tenant_id, user_id)
);

ALTER TABLE tenant_user ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_user_rls ON tenant_user
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);
```

#### Team / TeamUser 表

```sql
CREATE TABLE team (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenant(id),
  workspace_id UUID NOT NULL,

  name VARCHAR(255) NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE,

  FOREIGN KEY (tenant_id, workspace_id) REFERENCES workspace(tenant_id, id)
);

CREATE TABLE team_user (
  team_id UUID NOT NULL REFERENCES team(id),
  tenant_id UUID NOT NULL,            -- 冗余存储，便于 RLS 直接过滤，避免 JOIN
  user_id UUID NOT NULL REFERENCES app_user(id),
  role_override VARCHAR(20),          -- 可选：团队内角色覆盖租户级默认角色

  PRIMARY KEY (team_id, user_id)
);

ALTER TABLE team ENABLE ROW LEVEL SECURITY;
CREATE POLICY team_rls ON team
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);

ALTER TABLE team_user ENABLE ROW LEVEL SECURITY;
CREATE POLICY team_user_rls ON team_user
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);
```

#### Credential 表（凭证库，対応 F-02-02/7.5 安全要件）

```sql
CREATE TABLE credential (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenant(id),
  workspace_id UUID NOT NULL,

  name VARCHAR(255) NOT NULL,          -- 用户可读标识，如"飞书-市场部账号"
  platform VARCHAR(50) NOT NULL,       -- 'lark' | 'slack' | 'jira' | ...
  credential_type VARCHAR(20) NOT NULL, -- 'oauth2_token' | 'api_key' | 'cookie_session'

  -- 加密存储：仅存密文，密钥由 KMS 管理（対応基本設計書 6.2 節）
  encrypted_payload BYTEA NOT NULL,
  encryption_key_id VARCHAR(100) NOT NULL,  -- KMS 中的密钥引用，非密钥本身

  -- OAuth2 场景的过期与刷新
  expires_at TIMESTAMP WITH TIME ZONE,
  refresh_token_encrypted BYTEA,

  created_by UUID,
  created_at TIMESTAMP WITH TIME ZONE,
  last_used_at TIMESTAMP WITH TIME ZONE,
  last_rotated_at TIMESTAMP WITH TIME ZONE,

  FOREIGN KEY (tenant_id, workspace_id) REFERENCES workspace(tenant_id, id)
);

ALTER TABLE credential ENABLE ROW LEVEL SECURITY;
CREATE POLICY credential_rls ON credential
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);

-- 凭证访问需强制审计（应用层在每次读取 encrypted_payload 前调用 M-11 record_audit_log）
```

#### ConnectorTemplate 表（対応 F-16 通用 CRM/企业系统适配框架）

```sql
CREATE TABLE connector_template (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenant(id),

  name VARCHAR(255) NOT NULL,
  base_url VARCHAR(500) NOT NULL,
  auth_method_json JSONB NOT NULL,     -- AuthMethod 序列化（[M-01 §3.6](../modules/M-01-acquisition-adapter.md)）
  endpoints_json JSONB NOT NULL,        -- Vec<EndpointSpec> 序列化
  field_mapping_json JSONB NOT NULL,    -- FieldMappingRules 序列化

  created_by UUID,
  created_at TIMESTAMP WITH TIME ZONE,
  updated_at TIMESTAMP WITH TIME ZONE
);

ALTER TABLE connector_template ENABLE ROW LEVEL SECURITY;
CREATE POLICY connector_template_rls ON connector_template
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);
```

#### ConnectorSyncState 表（対応 F-15-03 增量同步）

```sql
CREATE TABLE connector_sync_state (
  tenant_id UUID NOT NULL,
  adapter_id VARCHAR(100) NOT NULL,
  credential_id UUID REFERENCES credential(id),

  cursor_json JSONB,                    -- Cursor 序列化，NULL 表示尚未首次同步
  last_synced_at TIMESTAMP WITH TIME ZONE,
  sync_status VARCHAR(20) DEFAULT 'idle', -- 'idle' | 'syncing' | 'error'
  last_error TEXT,

  PRIMARY KEY (tenant_id, adapter_id, credential_id)
);

ALTER TABLE connector_sync_state ENABLE ROW LEVEL SECURITY;
CREATE POLICY connector_sync_state_rls ON connector_sync_state
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);
```

#### Canvas 表

```sql
CREATE TABLE canvas (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenant(id),
  workspace_id UUID NOT NULL,
  
  name VARCHAR(255),
  description TEXT,
  
  -- 画布定义
  dag_json JSONB,  -- DAG/StateGraph 定义
  
  -- 版本与状态
  -- current_version_id 有意不声明 FK：与下方 canvas_version.canvas_id → canvas.id 互相引用会形成循环外键，
  -- 建表顺序上 canvas_version 表此时还不存在。一致性由应用层保证（写入前必须先插入 canvas_version 行），
  -- 如需数据库级强制，可在两表都建好后执行 ALTER TABLE canvas ADD CONSTRAINT ... DEFERRABLE INITIALLY DEFERRED
  current_version_id UUID,
  status ENUM('draft', 'published', 'archived'),
  
  created_by UUID,  -- 创建者 user_id
  updated_by UUID,
  
  created_at TIMESTAMP WITH TIME ZONE,
  updated_at TIMESTAMP WITH TIME ZONE,
  
  -- 多租户隔离
  UNIQUE(tenant_id, id),
  FOREIGN KEY (tenant_id, workspace_id) REFERENCES workspace(tenant_id, id)
);

-- 启用行级安全
ALTER TABLE canvas ENABLE ROW LEVEL SECURITY;
CREATE POLICY canvas_rls ON canvas 
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);
```

#### CanvasVersion 表（対応 F-10 版本管理与回滚）

```sql
CREATE TABLE canvas_version (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  canvas_id UUID NOT NULL,

  version_number INT NOT NULL,
  dag_json JSONB NOT NULL,             -- 该版本的完整画布快照（整体存储，理由见 5.1 節注）
  change_summary TEXT,                  -- 可选的人工/自动生成变更摘要

  created_by UUID,
  created_at TIMESTAMP WITH TIME ZONE,

  UNIQUE(tenant_id, canvas_id, version_number),
  FOREIGN KEY (tenant_id, canvas_id) REFERENCES canvas(tenant_id, id)
);

ALTER TABLE canvas_version ENABLE ROW LEVEL SECURITY;
CREATE POLICY canvas_version_rls ON canvas_version
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);

-- canvas.current_version_id 指向本表，回滚操作即将 current_version_id 指回历史版本
-- 并将该历史版本的 dag_json 复制为新的 version_number（回滚本身也生成一条新版本记录，保留完整历史）
```

#### CanvasExecution 表

```sql
CREATE TABLE canvas_execution (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  workspace_id UUID NOT NULL,
  canvas_id UUID NOT NULL,
  
  -- 执行信息
  triggered_by UUID,  -- 触发者 user_id
  triggered_at TIMESTAMP WITH TIME ZONE,
  started_at TIMESTAMP WITH TIME ZONE,
  completed_at TIMESTAMP WITH TIME ZONE,
  
  -- 执行状态与结果
  status ENUM('pending', 'running', 'success', 'failure', 'aborted'),
  error_message TEXT,
  
  -- 统计
  total_nodes_executed INT,
  failed_nodes INT,
  duration_ms INT,
  
  -- 数据量统计
  records_processed INT,
  records_failed INT,
  
  FOREIGN KEY (tenant_id) REFERENCES tenant(id),
  FOREIGN KEY (tenant_id, canvas_id) REFERENCES canvas(tenant_id, id)
);

ALTER TABLE canvas_execution ENABLE ROW LEVEL SECURITY;
CREATE POLICY canvas_execution_rls ON canvas_execution
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);
```

#### ExecutionNodeSnapshot 表（対応 F-08 可视化调试）

```sql
CREATE TABLE execution_node_snapshot (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  execution_id UUID NOT NULL,
  node_id VARCHAR(100) NOT NULL,       -- 对应 CanvasDefinition 内的 node_id，非全局 UUID

  attempt INT NOT NULL DEFAULT 1,       -- 重试次数编号
  status VARCHAR(20) NOT NULL,          -- 対応詳細設計書 NodeStatus

  input_ref VARCHAR(500),               -- 输入数据的对象存储引用（大体积数据不入库，仅存引用）
  output_ref VARCHAR(500),
  error_message TEXT,

  started_at TIMESTAMP WITH TIME ZONE,
  completed_at TIMESTAMP WITH TIME ZONE,
  duration_ms INT,

  FOREIGN KEY (tenant_id) REFERENCES tenant(id),
  FOREIGN KEY (tenant_id, execution_id) REFERENCES canvas_execution(tenant_id, id)
);

ALTER TABLE execution_node_snapshot ENABLE ROW LEVEL SECURITY;
CREATE POLICY execution_node_snapshot_rls ON execution_node_snapshot
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);

-- 対応 F-08-01：每个节点仅保留最近 N 次（默认 20 次）快照，超出部分由定时任务清理
CREATE INDEX idx_exec_node_snapshot_retention
  ON execution_node_snapshot (tenant_id, execution_id, node_id, started_at DESC);
```

#### ExecutionLog 表

```sql
CREATE TABLE execution_log (
  id BIGSERIAL PRIMARY KEY,            -- 高频写入场景，用自增而非 UUID 降低索引开销
  tenant_id UUID NOT NULL,
  execution_id UUID NOT NULL,
  node_id VARCHAR(100),                 -- 可为空：整体执行级别的日志

  log_level VARCHAR(10) NOT NULL,       -- 'debug' | 'info' | 'warn' | 'error'
  message TEXT NOT NULL,
  logged_at TIMESTAMP WITH TIME ZONE DEFAULT now(),

  FOREIGN KEY (tenant_id) REFERENCES tenant(id),
  FOREIGN KEY (tenant_id, execution_id) REFERENCES canvas_execution(tenant_id, id)
);

ALTER TABLE execution_log ENABLE ROW LEVEL SECURITY;
CREATE POLICY execution_log_rls ON execution_log
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);

-- 対応 8.1 節（要件定義書）日志结构化要求；生产环境建议按月分区（PARTITION BY RANGE (logged_at)）
-- 以支撑 7.5 節审计日志保留期策略的高效批量清理
```

#### AuditLog 表

```sql
CREATE TABLE audit_log (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  
  -- 操作者与时间
  user_id UUID,
  action_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  
  -- 操作内容
  action_type VARCHAR(50),  -- 'create_canvas', 'edit_canvas', 'delete_canvas', ...
  resource_type VARCHAR(50),  -- 'canvas', 'workflow', 'credential', ...
  resource_id UUID,
  
  -- 变更内容（JSON Patch 格式）
  before_state JSONB,
  after_state JSONB,
  
  -- 结果
  success BOOLEAN,
  error_message TEXT,
  
  FOREIGN KEY (tenant_id) REFERENCES tenant(id),
  INDEX (tenant_id, action_at DESC)
);
```

### 4.3 模块注册与生命周期相关表（[DOC-MOD-014](../modules/M-14-module-registry.md) 配套）

#### module_registry 表

```sql
CREATE TABLE module_registry (
  id UUID PRIMARY KEY,
  module_id VARCHAR(100) NOT NULL,           -- 如 'm01-acquisition'
  version VARCHAR(50) NOT NULL,              -- SemVer，如 '1.5.0'
  manifest JSONB NOT NULL,                   -- Module.toml 全文
  artifact_url TEXT,                          -- s3://bucket/path
  artifact_sha256 VARCHAR(64),                -- 64 字符十六进制
  
  state VARCHAR(30) NOT NULL DEFAULT 'Registered',  -- 'Registered' | 'Downloading' | 'Loaded' | 'Active' | 'Draining' | 'Drained' | 'Unloading' | 'Unloaded' | 'Failed' | 'Rejected'
  active BOOLEAN NOT NULL DEFAULT FALSE,      -- 当前激活版本
  retired_at TIMESTAMP WITH TIME ZONE,
  activated_at TIMESTAMP WITH TIME ZONE,
  
  registered_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  registered_by UUID,
  
  -- 多租户隔离：模块为系统级资源，但归属某个 tenant 的部署
  tenant_id UUID NOT NULL,
  
  UNIQUE (tenant_id, module_id, version),
  UNIQUE (tenant_id, module_id) WHERE active = TRUE  -- 同 module_id 仅 1 active
);

ALTER TABLE module_registry ENABLE ROW LEVEL SECURITY;
CREATE POLICY module_registry_rls ON module_registry
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);

CREATE INDEX idx_module_registry_lookup
  ON module_registry (tenant_id, module_id, version);
```

#### module_upgrade_history 表

```sql
CREATE TABLE module_upgrade_history (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  module_id VARCHAR(100) NOT NULL,
  from_version VARCHAR(50),
  to_version VARCHAR(50) NOT NULL,
  strategy VARCHAR(20) NOT NULL,              -- 'rolling' | 'blue-green' | 'canary' | 'recreate'
  
  plan_id UUID NOT NULL,
  status VARCHAR(20) NOT NULL,                -- 'Pending' | 'InProgress' | 'Succeeded' | 'Failed' | 'Aborted'
  
  started_at TIMESTAMP WITH TIME ZONE,
  completed_at TIMESTAMP WITH TIME ZONE,
  total_nodes INT,
  completed_nodes INT DEFAULT 0,
  failed_nodes INT DEFAULT 0,
  
  rolled_back BOOLEAN DEFAULT FALSE,
  error_message TEXT,
  
  FOREIGN KEY (tenant_id) REFERENCES tenant(id)
);

ALTER TABLE module_upgrade_history ENABLE ROW LEVEL SECURITY;
CREATE POLICY module_upgrade_history_rls ON module_upgrade_history
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);
```

#### module_instance 表

```sql
CREATE TABLE module_instance (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  node_id UUID NOT NULL,                       -- 来自 cluster_node.node_id
  module_id VARCHAR(100) NOT NULL,
  version VARCHAR(50) NOT NULL,
  
  state VARCHAR(20) NOT NULL DEFAULT 'Loading',  -- 'Loading' | 'Loaded' | 'Active' | 'Draining' | 'Drained' | 'Unloading' | 'Terminated' | 'Failed'
  state_changed_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  
  resource_usage JSONB,                        -- {cpu, memory, ...} 实时上报
  last_health_at TIMESTAMP WITH TIME ZONE,
  
  FOREIGN KEY (tenant_id, module_id, version) REFERENCES module_registry(tenant_id, module_id, version),
  FOREIGN KEY (tenant_id, node_id) REFERENCES cluster_node(tenant_id, node_id),
  UNIQUE (node_id, module_id, version)
);

ALTER TABLE module_instance ENABLE ROW LEVEL SECURITY;
CREATE POLICY module_instance_rls ON module_instance
  FOR ALL USING (tenant_id = current_setting('app.current_tenant', true)::uuid);
```

### 4.4 中心事件总线表（[DOC-MOD-015](../modules/M-15-central-event-bus.md) 配套）

#### event_log 表

```sql
CREATE TABLE event_log (
  id UUID PRIMARY KEY,
  event_seq BIGINT NOT NULL,                   -- 全局递增 SEQUENCE
  topic VARCHAR(200) NOT NULL,                 -- 'module.registered' 等
  tenant_id UUID,
  payload JSONB NOT NULL,
  headers JSONB NOT NULL DEFAULT '{}',         -- schema_version, trace_id, producer, ...
  
  produced_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  producer VARCHAR(100),                      -- 生产者模块 ID
  
  UNIQUE (event_seq)
);

CREATE SEQUENCE event_seq_global START 1 INCREMENT 1 CACHE 100;

ALTER TABLE event_log ENABLE ROW LEVEL SECURITY;
CREATE POLICY event_log_rls ON event_log
  FOR ALL USING (tenant_id IS NULL OR tenant_id = current_setting('app.current_tenant', true)::uuid);

-- 索引：按 topic + 时间范围查询
CREATE INDEX idx_event_log_topic_time ON event_log (topic, produced_at DESC);
CREATE INDEX idx_event_log_tenant_time ON event_log (tenant_id, produced_at DESC);
```

#### event_topic 表

```sql
CREATE TABLE event_topic (
  topic VARCHAR(200) PRIMARY KEY,
  category VARCHAR(50) NOT NULL,                -- 'system' | 'business' | 'audit' | 'data'
  retention_days INT NOT NULL DEFAULT 30,
  description TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);
```

#### event_subscription 表

```sql
CREATE TABLE event_subscription (
  id UUID PRIMARY KEY,
  topic_pattern VARCHAR(200) NOT NULL,          -- 支持通配符 * #
  group_id VARCHAR(100) NOT NULL,
  delivery_mode VARCHAR(20) NOT NULL,           -- 'durable' | 'ephemeral'
  filter JSONB DEFAULT '{}',                    -- 过滤条件
  from_position JSONB NOT NULL,                 -- 'earliest' | 'latest' | {event_seq: 1000}
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  UNIQUE (topic_pattern, group_id)
);
```

#### consumer_offset 表

```sql
CREATE TABLE consumer_offset (
  subscription_id UUID REFERENCES event_subscription(id),
  topic VARCHAR(200) NOT NULL,
  consumer_id VARCHAR(200) NOT NULL,            -- 通常为 'group_id:instance_id'
  last_acked_event_seq BIGINT NOT NULL DEFAULT 0,
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  PRIMARY KEY (subscription_id, topic, consumer_id)
);
```

### 4.5 集群协调表（[DOC-MOD-016](../modules/M-16-cluster-coordinator.md) 配套）

#### cluster_node 表

```sql
CREATE TABLE cluster_node (
  node_id UUID PRIMARY KEY,
  tenant_id UUID,                               -- NULL 表示系统级节点
  hostname VARCHAR(255) NOT NULL,
  advertised_addr VARCHAR(255) NOT NULL,        -- '10.0.1.5:8000'
  labels JSONB NOT NULL DEFAULT '{}',            -- {zone, role, ...}
  
  state VARCHAR(20) NOT NULL DEFAULT 'Registering',  -- 'Registering' | 'Active' | 'Unhealthy' | 'Draining' | 'Removed'
  capacity INT NOT NULL DEFAULT 100,            -- 可承载 module instance 数
  
  last_heartbeat_at TIMESTAMP WITH TIME ZONE,
  status JSONB,                                 -- 最新心跳上报的健康/资源信息
  current_load NUMERIC(5, 2),                  -- 0.00 ~ 1.00
  runtime_version VARCHAR(50),
  started_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);

ALTER TABLE cluster_node ENABLE ROW LEVEL SECURITY;
CREATE POLICY cluster_node_rls ON cluster_node
  FOR ALL USING (tenant_id IS NULL OR tenant_id = current_setting('app.current_tenant', true)::uuid);
```

#### leader_lease 表

```sql
CREATE TABLE leader_lease (
  lease_key VARCHAR(200) PRIMARY KEY,            -- 'm04-orchestrator-singleton'
  holder_node_id UUID NOT NULL REFERENCES cluster_node(node_id),
  acquired_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
  expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
  renew_count INT NOT NULL DEFAULT 0,
  metadata JSONB DEFAULT '{}'
);

CREATE INDEX idx_leader_lease_expires ON leader_lease (expires_at);
```

#### shard_assignment 表

```sql
CREATE TABLE shard_assignment (
  shard_id INT NOT NULL,
  tenant_id UUID NOT NULL,
  node_id UUID NOT NULL REFERENCES cluster_node(node_id),
  assigned_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  PRIMARY KEY (shard_id, tenant_id)
);
```

### 4.6 关键 PL/pgSQL 存过（原子化部署 / 事件总线 / 集群协调）

依据 [DOC-ARCH-005 §11 PL/pgSQL 存过策略](../architecture/04-atomic-deployment.md)，以下存过在事务内保证关键状态变更的原子性。

#### 4.6.1 register_module（[DOC-MOD-014 §3.4](../modules/M-14-module-registry.md)）

```sql
CREATE OR REPLACE FUNCTION register_module(
    p_module_id       TEXT,
    p_version         TEXT,
    p_manifest        JSONB,
    p_artifact_url    TEXT,
    p_artifact_sha256 TEXT
) RETURNS TABLE(success BOOLEAN, module_instance_id UUID, error_msg TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_existing_id UUID;
    v_new_id UUID;
BEGIN
    -- 1. manifest 必填字段校验
    IF p_manifest->'meta'->>'module_id' IS NULL THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, 'manifest.meta.module_id 必填';
        RETURN;
    END IF;
    IF p_manifest->'meta'->>'version' IS NULL THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, 'manifest.meta.version 必填';
        RETURN;
    END IF;
    
    -- 2. 幂等性：同 module_id + version 不重复
    SELECT id INTO v_existing_id
        FROM module_registry
        WHERE module_id = p_module_id AND version = p_version
        AND tenant_id = current_setting('app.current_tenant', true)::UUID;
    IF FOUND THEN
        RETURN QUERY SELECT TRUE, v_existing_id, NULL::TEXT;
        RETURN;
    END IF;
    
    -- 3. 插入新版本
    INSERT INTO module_registry (
        id, module_id, version, manifest, artifact_url, artifact_sha256,
        state, registered_at, registered_by, tenant_id
    ) VALUES (
        gen_random_uuid(), p_module_id, p_version, p_manifest,
        p_artifact_url, p_artifact_sha256,
        'Registered', now(),
        current_setting('app.current_user_id', true)::UUID,
        current_setting('app.current_tenant', true)::UUID
    )
    RETURNING id INTO v_new_id;
    
    -- 4. 触发 module.registered 事件（[DOC-MOD-015](../modules/M-15-central-event-bus.md)）
    PERFORM append_event('module.registered', jsonb_build_object(
        'module_id', p_module_id, 'version', p_version, 'instance_id', v_new_id
    ));
    
    RETURN QUERY SELECT TRUE, v_new_id, NULL::TEXT;
END;
$$;
```

#### 4.6.2 atomic_module_swap（[DOC-MOD-014 §3.5](../modules/M-14-module-registry.md)）

```sql
CREATE OR REPLACE FUNCTION atomic_module_swap(
    p_module_id    TEXT,
    p_from_version TEXT,
    p_to_version   TEXT
) RETURNS TABLE(success BOOLEAN, error_msg TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_from_id UUID;
    v_to_id   UUID;
BEGIN
    -- 串行化同 module_id 的 swap 操作
    PERFORM pg_advisory_xact_lock(hashtext('module_swap:' || p_module_id));
    
    -- 1. 校验两边都存在
    SELECT id INTO v_from_id FROM module_registry
        WHERE module_id = p_module_id AND version = p_from_version
        AND tenant_id = current_setting('app.current_tenant', true)::UUID;
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'from_version not found';
        RETURN;
    END IF;
    SELECT id INTO v_to_id FROM module_registry
        WHERE module_id = p_module_id AND version = p_to_version
        AND tenant_id = current_setting('app.current_tenant', true)::UUID;
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'to_version not found';
        RETURN;
    END IF;
    
    -- 2. 双写：同一事务内 from=inactive + to=active
    UPDATE module_registry SET active = FALSE, retired_at = now()
        WHERE id = v_from_id;
    UPDATE module_registry SET active = TRUE, activated_at = now()
        WHERE id = v_to_id;
    
    -- 3. 写升级历史
    INSERT INTO module_upgrade_history (
        id, tenant_id, module_id, from_version, to_version,
        strategy, plan_id, status, started_at, completed_at
    ) VALUES (
        gen_random_uuid(),
        current_setting('app.current_tenant', true)::UUID,
        p_module_id, p_from_version, p_to_version,
        'atomic_swap', gen_random_uuid(), 'Succeeded', now(), now()
    );
    
    -- 4. 事件
    PERFORM append_event('module.swapped', jsonb_build_object(
        'module_id', p_module_id, 'from', p_from_version, 'to', p_to_version
    ));
    
    RETURN QUERY SELECT TRUE, NULL::TEXT;
END;
$$;
```

#### 4.6.3 append_event（[DOC-MOD-015 §3.5](../modules/M-15-central-event-bus.md)）

```sql
CREATE OR REPLACE FUNCTION append_event(
    p_topic    TEXT,
    p_payload  JSONB
) RETURNS TABLE(event_id UUID, event_seq BIGINT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_seq BIGINT;
    v_id UUID;
    v_tenant_id UUID;
BEGIN
    v_tenant_id := current_setting('app.current_tenant', true)::UUID;
    v_seq := nextval('event_seq_global');
    v_id := gen_random_uuid();
    
    INSERT INTO event_log (
        id, event_seq, topic, tenant_id, payload, produced_at, producer
    ) VALUES (
        v_id, v_seq, p_topic, v_tenant_id, p_payload, now(),
        current_setting('app.current_service', true)
    );
    
    -- 异步通知 dispatcher
    PERFORM pg_notify('event_appended',
        json_build_object('seq', v_seq, 'topic', p_topic)::TEXT);
    
    RETURN QUERY SELECT v_id, v_seq;
END;
$$;
```

#### 4.6.4 acquire_lease / release_lease（[DOC-MOD-016 §3.4](../modules/M-16-cluster-coordinator.md)）

```sql
CREATE OR REPLACE FUNCTION acquire_lease(
    p_lease_key    TEXT,
    p_node_id      UUID,
    p_ttl_seconds  INT DEFAULT 30
) RETURNS TABLE(acquired BOOLEAN, lease_id UUID, expires_at TIMESTAMPTZ)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_existing_node UUID;
    v_existing_expires TIMESTAMPTZ;
    v_new_expires TIMESTAMPTZ;
BEGIN
    v_new_expires := now() + (p_ttl_seconds || ' seconds')::INTERVAL;
    
    SELECT holder_node_id, expires_at INTO v_existing_node, v_existing_expires
        FROM leader_lease WHERE lease_key = p_lease_key FOR UPDATE;
    
    IF v_existing_node IS NULL
       OR v_existing_expires < now()
       OR v_existing_node = p_node_id THEN
        INSERT INTO leader_lease (lease_key, holder_node_id, acquired_at, expires_at, renew_count)
        VALUES (p_lease_key, p_node_id, now(), v_new_expires, 1)
        ON CONFLICT (lease_key) DO UPDATE
            SET holder_node_id = p_node_id, acquired_at = now(),
                expires_at = v_new_expires, renew_count = leader_lease.renew_count + 1;
        RETURN QUERY SELECT TRUE, NULL::UUID, v_new_expires;
    ELSE
        RETURN QUERY SELECT FALSE, NULL::UUID, v_existing_expires;
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION release_lease(
    p_lease_key TEXT,
    p_node_id   UUID
) RETURNS TABLE(released BOOLEAN)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_existing_node UUID;
BEGIN
    SELECT holder_node_id INTO v_existing_node
        FROM leader_lease WHERE lease_key = p_lease_key;
    IF v_existing_node IS NULL OR v_existing_node != p_node_id THEN
        RETURN QUERY SELECT FALSE;
        RETURN;
    END IF;
    DELETE FROM leader_lease WHERE lease_key = p_lease_key;
    RETURN QUERY SELECT TRUE;
END;
$$;
```

#### 4.6.5 register_node_heartbeat（[DOC-MOD-016 §3.6](../modules/M-16-cluster-coordinator.md)）

```sql
CREATE OR REPLACE FUNCTION register_node_heartbeat(
    p_node_id UUID,
    p_status  JSONB
) RETURNS TABLE(healthy BOOLEAN, current_load NUMERIC)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_load NUMERIC;
    v_healthy BOOLEAN;
BEGIN
    INSERT INTO cluster_node (node_id, last_heartbeat_at, status, state)
    VALUES (p_node_id, now(), p_status, 'Active')
    ON CONFLICT (node_id) DO UPDATE
        SET last_heartbeat_at = now(), status = p_status;
    
    SELECT (COUNT(*) FILTER (WHERE state = 'Active')::NUMERIC / GREATEST(capacity, 1))::NUMERIC
        INTO v_load
        FROM module_instance WHERE node_id = p_node_id;
    
    v_healthy := COALESCE((p_status->>'health')::BOOLEAN, FALSE);
    
    RETURN QUERY SELECT v_healthy, v_load;
END;
$$;
```

### 4.7 后台清理任务

- **事件保留期清理**：每日 cron 删除 `event_log` 中 `produced_at < now() - retention_days` 的行
- **节点失联清理**：每分钟检查 `cluster_node.last_heartbeat_at < now() - 60s` 的节点，置 `state='Removed'`
- **租约过期清理**：每分钟检查 `leader_lease.expires_at < now()` 的租约，发布 `cluster.leader_election_pending` 事件

## 5. 验收要点

1. **三层隔离落地**：逻辑隔离（应用层显式 tenant_id）+ 数据库隔离（RLS 强制）+ 物理隔离（命名空间）三层均按场景实施。
2. **RLS 不依赖连接归还钩子**：跨事务边界的会话变量不残留（单元测试 TC-MT-002）。
3. **配额超限拒绝**：超 `concurrent_canvas_executions` / `api_calls_per_hour` / `storage_bytes` 的请求返回 `QUOTA_EXCEEDED` 错误码。
4. **租户清理完整**：删除租户后 7 天保留期内可恢复，超期后 `DeletionReport` 包含逐表删除行数与吊销的密钥列表。
5. **审计日志完整**：所有用户操作（画布编辑、权限变更、凭证查看、数据导出、画布执行）记录到 `audit_log`，多租户环境下与核心数据一样隔离。 [NF-SEC]【必須】

---

## 6. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 多租户 | 单一系统实例服务多个租户 | §1、F-17 |
| 三层隔离 | 逻辑/数据库/物理隔离 | §2.2 |
| TenantContextMiddleware | 注入租户上下文的中间件 | §3.1 [NF-SEC]【必須】 |
| with_tenant_scope | 事务级 RLS 会话变量注入 | §3.1 [NF-SEC]【必須】 |
| SET LOCAL | 事务作用域会话变量 | §3.1 |
| RLS | PostgreSQL Row-Level Security | §3.4 [NF-SEC]【必須】 |
| TenantQuota | 租户资源配额模型 | §2.3 |
| QuotaEnforcer | 配额强制执行器 | §3.2 [NF-PER]【必須】 |
| DeletionReport | 租户硬删除审计报告 | §3.3 [NF-SEC]【必須】 |
| 凭证库 | credential 表，加密存储 | §4.2 [NF-SEC]【必須】 |
| audit_log | 审计日志表 | §4.2 [NF-SEC]【必須】 |
| 软删除 | 标记删除，保留期 7 天 | §2.4 |
| KMS | 密钥管理服务 | §4.2 |
| 跨租户数据穿透 | 严重安全漏洞 | §2.2 [NF-SEC]【必須】 |

## 7. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. PostgreSQL Global Development Group「PostgreSQL Documentation — Row Security Policies」
4. PostgreSQL Global Development Group「PostgreSQL Documentation — SET LOCAL / set_config」
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
