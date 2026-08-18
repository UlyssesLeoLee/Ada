# Ada 无限画布跨平台数据集成系统 基本設計書

版本：1.0.0
制定日：2026-08-17
文档语言：中文（简体）
密级：内部

---

## 目次

1. はじめに（前言）
2. 設計基本方針（设计基本原则）
3. システムアーキテクチャ（系统架构）
4. マルチテナント設計（多租户设计）
5. データベース設計（数据库设计）
6. セキュリティ設計（安全设计）
7. インタフェース設計（接口设计）
8. デプロイメント戦略（部署策略）
9. 技術スタック推奨（技术栈推荐）
10. リスクと対応（风险与对应）

---

## 1. はじめに（前言）

本文档为 Ada 系统的基本設計書（Basic Design Document），在需求定義書确认合意的基础上，详细阐述系统的整体架构、多租户设计、关键模块的设计思路、技术选型与部署策略。

本文档作为后续详細設計書的上位设计方针，需在项目主要关系人间达成一致理解后方可展开详細設計與开发。

---

## 2. 設計基本方針（设计基本原则）

### 2.1 生体仿生架构坚守

系统严格遵循需求文档第 5 章定义的四层生物仿生模型：
- **骨骼层（节点）**：最小化节点的职责，每个节点仅做一件事
- **血液层（数据流）**：数据包在连线上的流转具有一定自主特性，支持缓存、限速、重放
- **肌肉层（控制流）**：处理并发调度、暂停恢复、限流退避等执行动力
- **神经系统层（编排引擎）**：基于 LangGraph 风格的状态机做决策，不涉及体力活动

### 2.2 多租户优先设计

无论单机本地模式还是 SaaS 部署模式，系统架构均需为多租户就绪：
- 所有数据模型在设计时均需包含 `tenant_id` 字段
- 所有数据库查询、API 请求均需自动注入租户隔离条件
- 租户间计算资源（浏览器实例、并发度、存储空间）需独立配额管理

### 2.3 底层智能平衡

- **底层**：直接操作浏览器渲染层（DOM/网络流量），在无 API 场景下自动降级；API 优先加速采集成功率
- **智能**：编排引擎支持 LLM 集成，自动化节点失败检测与修复提示（如检测选择器失效并提示用户）

### 2.4 开发体验最优化

用户应能通过可视化拖拽完成 80% 常见场景，无需编码。系统提供渐进式复杂度：
- 初级：无限画布 + 内置节点 = 零代码编排
- 中级：自定义转换函数（表达式/轻量脚本）
- 高级：自定义插件 SDK（Rust/WASM）扩展

---

## 3. システムアーキテクチャ（系统架构）

### 3.1 高层架构分层

```
┌───────────────────────────────────────────────────────┐
│                      前端层（Web UI）                    │
│      React/Vue 无限画布编辑器 + 实时协作编辑引擎        │
└───────────────────┬─────────────────────────────────────┘
                    │ HTTP/WebSocket (JWT Token Auth)
┌───────────────────▼─────────────────────────────────────┐
│                   API Gateway 层                        │
│  请求验证・租户上下文注入・限流・请求路由               │
└───────────────────┬─────────────────────────────────────┘
                    │
    ┌───────────────┼───────────────┐
    │               │               │
┌───▼──┐      ┌────▼─────┐    ┌───▼──┐
│ 编排 │      │  数据流   │    │ 节点  │
│ 引擎 │      │  执行器   │    │ 运行时 │
│ API  │      │  (队列+流)│    │ API  │
└──────┘      └──────────┘    └──────┘
    │               │               │
└───────────────────┼───────────────┘
                    │
            ┌───────▼────────┐
            │  存储服务层    │
            │ ┌────┬────────┐│
            │ │数据库│对象存储││
            │ └────┴────────┘│
            └────────────────┘
```

### 3.2 核心模块划分

#### 3.2.1 前端层（Frontend）

**技术选型建议**：React 18 + TypeScript + Redux Toolkit / Zustand

**主要职责**：
- 无限画布编辑引擎（缩放、平移、节点拖拽、连线编辑）
- 实时协作冲突解决（基于 Yjs/Automerge CRDT 库）
- 节点配置面板与参数输入表单生成
- WebSocket 长连接维护，实时推送节点执行状态
- 权限与多人编辑提示（谁在编辑哪个节点）

**特殊考虑**：
- 虚拟化渲染大规模节点（1000+ 节点时）——仅渲染视口内节点
- 本地状态管理 vs 服务端状态同步的平衡

#### 3.2.2 API Gateway 层

**技术选型建议**：Actix-web / Tonic（Rust 生态）

**主要职责**：
- JWT/OAuth2 身份验证与授权
- 租户上下文提取与请求上下文绑定
- 速率限制（Per-tenant, Per-user）
- 请求日志与审计
- 跨域（CORS）与安全头设置
- 请求/响应转码（JSON/Protocol Buffers）

#### 3.2.3 编排引擎（Orchestration Engine）

**技术选型建议**：基于 LangGraph 思想的 Rust 状态机实现

**主要职责**：
- 解析画布配置（DAG/StateGraph）为执行计划
- 状态管理与状态转移规则
- 条件分支、循环、异常捕获判决逻辑
- 执行历史记录与快照持久化
- 支持断点续传（Resume from checkpoint）

**关键设计**：
- 状态不可变性（Immutable State）——每次转移生成新状态，便于回放与调试
- 异步执行计划（Async Execution Plan）——支持并发节点调度
- 中间件系统（Middleware）——便于插入日志、监控、重试等横切关注点

#### 3.2.4 数据流执行器（Data Flow Executor）

**主要职责**：
- 节点出队与入队管理（消息队列驱动）
- 背压处理（Backpressure）——下游处理慢时自动暂停上游
- 数据在连线上的缓存与转发
- 流量监控与可视化数据
- 支持数据包的重放（Replay）用于调试

**实现方案**：
- 基于 async channel（tokio/crossbeam）的事件驱动架构
- 每条连线（Edge）是一个单向队列，支持配置容量与超时

#### 3.2.5 节点运行时（Node Runtime）

**主要职责**：
- 采集适配器（Playwright + API 双模式）
- 转换节点（字段映射、表达式计算、脚本执行）
- 路由节点（条件分支、数据复制）
- 输出节点（文件写入、数据库插入、Webhook 推送）
- 人工介入节点（暂停等待用户确认）

**插件扩展**：
- Rust 原生插件：通过 `libloading` 动态加载，需声明 `NodePlugin` trait
- WASM 插件：通过 `wasmtime` 沙箱执行，限制 CPU/内存/I/O

#### 3.2.6 存储服务层

**数据库选型建议**：
- 主数据库：PostgreSQL（支持行级安全 RLS 用于多租户隔离）
- 缓存：Redis（会话、临时数据、速率限制计数）
- 对象存储：S3/MinIO（快照、采集结果、日志）

**表结构关键字段**：所有表需包含 `tenant_id` + `workspace_id`（多租户模式）或仅 `workspace_id`（本地模式）

---

## 4. マルチテナント設計（多租户设计）

### 4.1 租户隔离的三层模型

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
-- Canvas 表
CREATE POLICY canvas_tenant_isolation ON canvas
  USING (tenant_id = current_setting('app.current_tenant_id')::uuid);

-- 查询前设置租户上下文
SET app.current_tenant_id TO 'uuid-of-tenant-1';
SELECT * FROM canvas;  -- 自动过滤
```

**备选方案**（无 RLS 时）：应用层显式过滤

#### 第三层：物理隔离（部署层）

对于大型企业或高合规性需求（FedRamp/SOC2），支持：
- **Kubernetes 命名空间隔离**：为关键租户分配独立的 Pod/Node
- **专用数据库实例**：超大型租户拥有独立数据库副本
- **数据驻留地限制**：某些租户的数据只能存储在特定地域

### 4.2 多租户计费与配额模型

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

### 4.3 租户生命周期管理

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

---

## 5. データベース設計（数据库设计）

### 5.1 核心数据模型 (ER 图概览)

```
Tenant
  ├─ TenantUser (多对多关系，含角色)
  ├─ Workspace
  │  ├─ WorkspaceUser (多对多)
  │  ├─ Canvas
  │  │  ├─ Node (节点定义)
  │  │  ├─ Edge (连线定义)
  │  │  ├─ CanvasVersion (版本快照)
  │  │  └─ CanvasExecution (执行记录)
  │  │      ├─ ExecutionNodeSnapshot (节点快照)
  │  │      └─ ExecutionLog (日志)
  │  ├─ Team
  │  │  └─ TeamUser
  │  ├─ Credential (凭证库)
  │  └─ ConnectorConfig (连接器配置)
  │
  └─ AuditLog (审计日志)
```

### 5.2 关键表设计

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
  FOR ALL USING (tenant_id = current_setting('app.current_tenant')::uuid);
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

---

## 6. セキュリティ設計（安全设计）

### 6.1 认证与授权

**认证流程**：
1. 用户登录 → JWT Token 生成（包含 `tenant_id`, `user_id`, `roles`）
2. 前端请求携带 Token → API Gateway 验证
3. Gateway 从 Token 中提取 `tenant_id` 并注入上下文

**授权检查**：
- 请求中的 `tenant_id` 与 Token 中的 `tenant_id` 匹配？
- 用户在该租户中的角色是否有权限？
- 资源是否属于该租户？

### 6.2 凭证管理

所有目标平台的敏感凭证（密码、Token、Cookie）需：
- 加密存储（AES-256）
- 加密密钥由 KMS（如 HashiCorp Vault）管理，不硬编码
- 在数据库中仅存储加密后的值，无法反向解密
- 凭证访问需审计日志

### 6.3 数据传输安全

- 所有通信走 HTTPS/WSS（TLS 1.2+）
- 多租户 SaaS 环境强制 HSTS 头
- 证书由公信 CA（如 Let's Encrypt）签发，自动轮换

### 6.4 多租户数据穿透测试

定期执行：
- 验证租户 A 的用户无法读取租户 B 的数据
- 验证租户 A 的 API 调用自动被租户过滤条件拦截
- 验证计费配额真正独立（租户 A 的超配不影响租户 B）

---

## 7. インタフェース設計（接口设计）

### 7.1 核心 REST API 端点

```
[Canvas 管理]
POST   /api/v1/tenants/{tenant_id}/workspaces/{ws_id}/canvases
GET    /api/v1/tenants/{tenant_id}/workspaces/{ws_id}/canvases/{canvas_id}
PUT    /api/v1/tenants/{tenant_id}/workspaces/{ws_id}/canvases/{canvas_id}
DELETE /api/v1/tenants/{tenant_id}/workspaces/{ws_id}/canvases/{canvas_id}

[执行管理]
POST   /api/v1/tenants/{tenant_id}/canvases/{canvas_id}/execute
GET    /api/v1/tenants/{tenant_id}/executions/{exec_id}
GET    /api/v1/tenants/{tenant_id}/executions/{exec_id}/snapshots

[多租户管理]
POST   /api/v1/admin/tenants
PUT    /api/v1/admin/tenants/{tenant_id}
DELETE /api/v1/admin/tenants/{tenant_id}

GET    /api/v1/tenants/{tenant_id}/quota
PUT    /api/v1/tenants/{tenant_id}/quota
GET    /api/v1/tenants/{tenant_id}/audit-logs
```

### 7.2 WebSocket 事件推送

```
[客户端连接]
ws://localhost:8000/ws?token=<jwt>&tenant_id=<uuid>

[服务端推送事件]
{
  "type": "canvas.node.executed",
  "data": {
    "execution_id": "...",
    "node_id": "...",
    "status": "success",
    "output": {...}
  }
}

{
  "type": "canvas.user_editing",
  "data": {
    "user_id": "...",
    "node_id": "...",
    "change_type": "selected"
  }
}
```

### 7.3 GraphQL API（可选）

用于前端复杂查询，支持字段按需获取，减少网络开销

---

## 8. デプロイメント戦略（部署策略）

### 8.1 单机本地模式

目标用户：个人开发者、小团队

```
User PC
  ├─ Ada Runtime (单一可执行文件)
  │  ├─ Frontend Web Server (静态资源)
  │  ├─ API Server (本地 HTTP)
  │  ├─ Orchestration Engine
  │  └─ SQLite 数据库
  │
  └─ 浏览器 (访问 http://localhost:8000)
```

特点：
- 零安装，零依赖（除浏览器内核）
- 数据本地存储，隐私优先
- 支持数据导出为 JSON/CSV 备份

### 8.2 多租户 SaaS 模式

目标用户：企业、SaaS 服务商

```
互联网
  └─ CDN (前端静态资源)
      └─ API Gateway (Nginx/HAProxy)
          └─ Kubernetes 集群
              ├─ Pod: API Server (副本集)
              ├─ Pod: Orchestration Engine (副本集)
              ├─ Pod: Node Runtime Pool (自动扩容)
              ├─ Pod: WebSocket Gateway
              │
              └─ 存储
                  ├─ PostgreSQL (RDS)
                  ├─ Redis (缓存)
                  └─ S3 (对象存储)
```

特点：
- 自动扩容缩容（基于 CPU/内存/队列长度）
- 多租户隔离（命名空间、网络策略）
- 高可用（多副本、健康检查、自动故障转移）

### 8.3 混合部署

支持企业内网部署：
- 私有 Docker 镜像库
- 离线安装包（包含依赖）
- Air-gapped 环境部署指南

---

## 9. 技術スタック推奨（技术栈推荐）

| 层级 | 推荐选技术 | 备选方案 |
|---|---|---|
| 前端 | React 18 + TypeScript + Zustand | Vue 3, Svelte |
| 无限画布库 | Konva.js / Pixi.js | Three.js, Babylon.js |
| 实时协作 | Yjs + y-websocket | Automerge, Apache OT |
| 后端 API | Actix-web (Rust) | Tokio + Tonic (gRPC), Axum |
| 编排引擎 | Rust 自研状态机 | LangGraph (Python) 移植 |
| 浏览器自动化 | Playwright Rust 绑定 | Puppeteer (Node.js) |
| 数据库 | PostgreSQL 12+ | MySQL 8.0+, MariaDB |
| 缓存 | Redis 6+ | Memcached, Apache Druid |
| 对象存储 | AWS S3 / MinIO | Azure Blob, Google Cloud Storage |
| 消息队列 | Tokio Channel / crossbeam | RabbitMQ, Apache Kafka |
| 容器化 | Docker + Kubernetes | Docker Swarm, Nomad |
| 监控日志 | Prometheus + Grafana + ELK | DataDog, New Relic |

---

## 10. リスクと対応（风险与对应）

| 风险 | 影响度 | 对应方案 |
|---|---|---|
| 多租户某个大计算任务（Playwright 采集）耗尽共享资源，影响其他租户 | 高 | 实现严格的配额隔离、Kubernetes 资源限制、优先级队列 |
| 数据库行级安全（RLS）配置错误导致租户数据穿透 | 严重 | 定期安全审计、多租户穿透测试（Penetration Testing）、实现应用层"双重检查" |
| Playwright 采集过程中浏览器崩溃导致租户挂起 | 中 | 进程监控重启、自动故障转移、用户可手动中止执行 |
| 编排引擎状态序列化/反序列化性能问题（大型画布） | 中 | 增量状态存储（仅存储变更）、异步序列化、分片存储 |

---

*本文档为基本設計書，后续需制定各模块的詳細設計書与具体编码规范。*
