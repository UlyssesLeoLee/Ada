# Ada 无限画布跨平台数据集成系统 詳細設計書

版本：1.2.0
制定日：2026-08-18
文档语言：中文（简体）
密级：内部
上位文档：`docs/requirements.md`（要件定義書 v1.2.0）、`docs/basic-design.md`（基本設計書 v1.2.0）

---

## 目次

1. はじめに（前言）
2. モジュール構成一覧（模块清单与依赖关系）
3. データ型定義（核心数据结构定义）
4. 採集アダプタ 詳細設計（F-02/F-15/F-16）
5. 標準化変換 詳細設計（F-03）
6. データフローエンジン 詳細設計（F-04）
7. オーケストレーションエンジン 詳細設計（F-05）
8. 制御フロー実行器 詳細設計（F-06）
9. ノードランタイム／プラグイン SDK 詳細設計（F-07）
10. マルチテナント・ミドルウェア 詳細設計（F-17）
11. 権限・協業モジュール 詳細設計（F-11）
12. フロントエンド 詳細設計
13. API 詳細仕様（リクエスト/レスポンス）
14. エラーコード体系
15. 状態遷移設計
16. 並行性・排他制御設計
17. テスト観点（単体・結合）
18. プロジェクト構成（Cargo Workspace 設計）
19. 可観測性設計（ログ・メトリクス・トレース）
20. 用語索引・変更履歴

---

## 1. はじめに（前言）

本文档为 Ada 系统的詳細設計書（Detailed Design Document），在基本設計書确定的架构基础上，对各核心模块给出**类/结构体级别**的设计——数据结构定义、函数签名、算法流程、状态机、并发模型、错误处理规则——作为编码实现与单元测试用例编写的直接依据。

本文档遵循的编写原则：
- 每个模块章节包含：**职责边界 → 数据结构 → 核心算法/时序 → 异常处理 → 与其他模块的接口契约**
- 所有代码示例使用 Rust 伪代码风格，仅用于表达设计意图，非最终实现
- 章节编号与基本設計書 3.2 节的模块划分一一对应

---

## 2. モジュール構成一覧（模块清单与依赖关系）

| モジュール ID | モジュール名 | 対応要件 | 主要言語 | 依存モジュール |
|---|---|---|---|---|
| M-01 | 采集适配器（Acquisition Adapter） | F-02, F-15, F-16 | Rust + Playwright-Rust | M-02, M-10 |
| M-02 | 标准化转换（Normalizer） | F-03 | Rust | なし（純粋関数群） |
| M-03 | 数据流引擎（Data Flow Engine） | F-04 | Rust (tokio) | M-02, M-10 |
| M-04 | 编排引擎（Orchestration Engine） | F-05 | Rust | M-03, M-06 |
| M-05 | 控制流执行器（Control Flow Executor） | F-06 | Rust (tokio) | M-01, M-04, M-09 |
| M-06 | 节点运行时／插件 SDK（Node Runtime） | F-07 | Rust + WASM (wasmtime) | M-01, M-02 |
| M-07 | 可视化调试（Debug Service） | F-08 | Rust | M-03, M-04 |
| M-08 | 定时/事件触发器（Trigger Service） | F-13 | Rust | M-05 |
| M-09 | 输出/导出模块（Exporter） | F-14 | Rust | M-02 |
| M-10 | 多租户中间件（Tenant Middleware） | F-17 | Rust | データベース層 |
| M-11 | 权限与协作模块（RBAC & Collab） | F-11 | Rust + yrs (前端側) | M-10 |
| M-12 | 前端画布编辑器（Canvas Editor） | F-01 | Rust (Bevy + bevy_egui, WASM) + TypeScript(HTML Overlay 胶水层) | API Gateway 経由で全モジュール |
| M-13 | API Gateway | 横断 | Rust (Actix-web) | M-10, M-11 |

### 2.1 モジュール依存図（テキスト表現）

```
M-12 前端画布编辑器
   │  HTTP/WebSocket
   ▼
M-13 API Gateway ── M-10 多租户中间件（全リクエストを通過）
   │                     │
   ├──────────┬──────────┼──────────┐
   ▼          ▼          ▼          ▼
M-04       M-03       M-07       M-08
编排引擎    数据流引擎   调试服务    触发器
   │          │
   ▼          ▼
M-05 ──────► M-01 采集适配器 ──► M-02 标准化转换 ──► M-03（回流）
控制流执行器        │
   │                └──► M-16（API连接器管理，属于M-01子模块）
   ▼
M-06 节点运行时（插件宿主） ──► M-09 输出/导出模块
   │
   ▼
M-11 权限与协作模块（前端 Yjs ⇄ 后端 CRDT 同步服务）
```

---

## 3. データ型定義（核心数据结构定义）

### 3.1 NJSON（标准化数据包）Rust 结构体定义

```rust
/// 标准化 JSON 数据包，全系统数据流转的最小单元
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NJson {
    pub schema_version: String,           // "1.1"
    pub tenant_id: Option<TenantId>,       // 多租户环境必填，本地模式可为 None
    pub workspace_id: Option<WorkspaceId>,
    pub source: SourceInfo,
    pub captured_at: DateTime<Utc>,
    pub captured_by: Option<String>,
    pub payload: Payload,
    pub raw_ref: Option<String>,
    pub trace_id: TraceId,
    pub execution_id: Option<ExecutionId>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceInfo {
    pub platform: String,        // "lark" | "slack" | "jira" | "custom_crm" ...
    pub adapter_id: String,      // 采集适配器唯一标识，如 "lark_im_v1"
    pub url: Option<String>,
    pub adapter_mode: AdapterMode, // Api | Browser
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AdapterMode {
    Api,
    Browser,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Payload {
    pub id: Option<String>,
    pub r#type: String,          // "message" | "post" | "ticket" | "order" ...
    pub fields: serde_json::Map<String, serde_json::Value>,
}

pub type TenantId = uuid::Uuid;
pub type WorkspaceId = uuid::Uuid;
pub type TraceId = uuid::Uuid;
pub type ExecutionId = uuid::Uuid;
```

### 3.2 画布配置（DAG/StateGraph）定义

```rust
/// 画布的静态配置，由前端编辑器序列化后持久化
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CanvasDefinition {
    pub canvas_id: uuid::Uuid,
    pub version: u32,
    pub nodes: Vec<NodeDefinition>,
    pub edges: Vec<EdgeDefinition>,
    pub entry_node_ids: Vec<String>,     // 支持多入口
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDefinition {
    pub node_id: String,                 // 画布内唯一
    pub node_type: NodeType,
    pub display_name: String,
    pub position: (f64, f64),            // 画布坐标，仅前端渲染使用
    pub config: serde_json::Value,       // 节点特有配置（选择器规则、映射规则等）
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub retry_policy: Option<RetryPolicy>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NodeType {
    Acquisition { adapter_id: String },
    Transform { expr: String },
    Condition { predicate: String },
    Loop { collection_expr: String },
    Merge,
    HumanReview,
    Output { exporter_id: String },
    Custom { plugin_id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EdgeDefinition {
    pub edge_id: String,
    pub from_node: String,
    pub to_node: String,
    pub edge_kind: EdgeKind,             // DataFlow(血液) | ControlFlow(肌肉)
    pub condition: Option<String>,       // 条件分支边的判断表达式
    pub buffer_config: BufferConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EdgeKind {
    DataFlow,
    ControlFlow,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BufferConfig {
    pub capacity: usize,          // 连线队列容量，默认 1000
    pub overflow_policy: OverflowPolicy,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum OverflowPolicy {
    Block,      // 背压：上游阻塞等待
    DropOldest,
    DropNewest,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BackoffStrategy {
    Fixed { interval_ms: u64 },
    Exponential { base_ms: u64, max_ms: u64, multiplier: f64 },
}
```

### 3.3 执行时状态（Execution State）

```rust
/// 编排引擎运行时的可变状态，每次状态迁移生成新版本（不可变数据结构）
#[derive(Clone, Debug)]
pub struct ExecutionState {
    pub execution_id: ExecutionId,
    pub canvas_id: uuid::Uuid,
    pub tenant_id: Option<TenantId>,
    pub node_statuses: im::HashMap<String, NodeStatus>,  // 使用不可变 HashMap (im crate)
    pub variables: im::HashMap<String, serde_json::Value>,
    pub version: u64,                    // 单调递增，用于并发检测与回放定位
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeStatus {
    Pending,
    Running { started_at: DateTime<Utc> },
    Success { output_ref: String, duration_ms: u64 },
    Failed { error: String, attempt: u32 },
    Skipped { reason: String },
}
```

---

## 4. 採集アダプタ 詳細設計（M-01，対応 F-02/F-15/F-16）

### 4.1 職責境界

- 输入：`NodeDefinition { node_type: Acquisition }` 中的适配器配置
- 输出：`Stream<NJson>`（异步流）
- **不负责**：字段级清洗映射（由 M-02 负责），仅负责"从平台取到数据 + 转换为初步 NJSON 骨架"

### 4.2 双模式适配器 Trait 设计

```rust
/// 所有采集适配器必须实现的统一接口
#[async_trait]
pub trait AcquisitionAdapter: Send + Sync {
    fn adapter_id(&self) -> &str;
    fn supported_modes(&self) -> Vec<AdapterMode>;

    /// 运行时能力探测：判断当前配置下 API 模式是否可用
    async fn probe_api_availability(&self, config: &AdapterConfig) -> ProbeResult;

    /// API 模式采集
    async fn fetch_via_api(
        &self,
        config: &AdapterConfig,
        cursor: Option<Cursor>,
    ) -> Result<FetchBatch, AdapterError>;

    /// 浏览器自动化模式采集
    async fn fetch_via_browser(
        &self,
        config: &AdapterConfig,
        browser_ctx: &BrowserContext,
    ) -> Result<FetchBatch, AdapterError>;

    /// IM 类平台的双向发送能力（可选实现，默认返回 Unsupported）
    async fn send_message(&self, config: &AdapterConfig, msg: OutboundMessage)
        -> Result<(), AdapterError> {
        Err(AdapterError::Unsupported)
    }
}

pub struct FetchBatch {
    pub items: Vec<NJson>,
    pub next_cursor: Option<Cursor>,
    pub has_more: bool,
}

pub enum ProbeResult {
    ApiAvailable,
    ApiUnavailable { reason: String },
}
```

### 4.3 双模式选择算法（F-02-00 对应实现）

```
函数 select_adapter_mode(config, adapter):
    1. 如果 config.forced_mode 已指定（用户手动优先级）:
         返回 config.forced_mode
    2. 否则：
         result = adapter.probe_api_availability(config)
         如果 result == ApiAvailable:
             返回 AdapterMode::Api
         否则:
             如果 adapter.supported_modes() 包含 Browser:
                 记录降级日志（audit_log: "adapter_fallback_to_browser"）
                 返回 AdapterMode::Browser
             否则:
                 返回 错误 AdapterError::NoAvailableMode
```

**探测缓存策略**：`probe_api_availability` 结果按 `(tenant_id, adapter_id)` 缓存于 Redis，TTL 5 分钟，避免每次执行都重新探测导致延迟。

### 4.4 Playwright 浏览器自动化子模块

```rust
pub struct BrowserContext {
    pub session_id: String,
    pub cdp_endpoint: String,           // Chrome DevTools Protocol 端点
    pub tenant_id: Option<TenantId>,     // 用于浏览器实例池的租户隔离配额
}

pub struct BrowserPool {
    // 按租户隔离的浏览器实例池，Key = tenant_id（本地模式为固定 "local"）
    pools: DashMap<TenantId, VecDeque<BrowserContext>>,
    max_instances_per_tenant: u32,       // 来自 TenantQuota.concurrent_playwright_instances
}

impl BrowserPool {
    /// 获取一个浏览器实例，若超出租户配额则排队等待
    pub async fn acquire(&self, tenant_id: TenantId) -> Result<BrowserContext, PoolError> {
        // 1. 检查当前租户已占用实例数是否达到配额上限
        // 2. 未达上限：启动新 Playwright 浏览器进程或复用空闲实例
        // 3. 已达上限：进入等待队列（带超时，默认 30s），超时返回 PoolError::QuotaExceeded
    }

    pub async fn release(&self, ctx: BrowserContext) {
        // 归还实例池；若实例累计使用次数超过阈值（如 100 次）则销毁重建，防止内存泄漏
    }
}
```

**选择器点选机制（F-02-01）设计**：前端通过注入到目标页面的一段脚本（经 CDP `Page.addScriptToEvaluateOnNewDocument`）捕获用户点击的 DOM 节点，计算相对稳定的 CSS 选择器（优先级：`data-testid` > `id` > 语义化 `class` 组合 > 相对 XPath 路径），结果通过 CDP 消息回传前端并写入 `AdapterConfig.selector_rules`。

### 4.5 采集频率限流设计（F-02-04）

```rust
pub struct RateLimiter {
    // 令牌桶算法，按 (tenant_id, platform) 维度隔离
    buckets: DashMap<(TenantId, String), TokenBucket>,
}

pub struct TokenBucket {
    capacity: f64,
    tokens: f64,
    refill_rate: f64,       // tokens/秒
    last_refill: Instant,
    jitter_range_ms: (u64, u64),  // 随机延时范围，模拟人类访问节奏
}

impl RateLimiter {
    pub async fn acquire(&self, key: (TenantId, String)) -> Duration {
        // 返回需要等待的时长（含随机抖动）
        // 计算方式：标准令牌桶 + Uniform(jitter_range_ms) 随机抖动
    }
}
```

### 4.6 API 连接器管理子模块（F-15，M-16 概念上属于 M-01 内部子模块）

```rust
pub struct ApiConnectorRegistry {
    connectors: HashMap<String, Box<dyn ApiConnector>>,
}

#[async_trait]
pub trait ApiConnector: Send + Sync {
    fn auth_method(&self) -> AuthMethod;
    async fn authenticate(&self, credential: &EncryptedCredential) -> Result<AuthToken, AuthError>;
    async fn refresh_token(&self, token: &AuthToken) -> Result<AuthToken, AuthError>;
    fn rate_limit_spec(&self) -> RateLimitSpec;
}

pub enum AuthMethod {
    OAuth2 { authorize_url: String, token_url: String, scopes: Vec<String> },
    ApiKey { header_name: String },
    WebhookSignature { algorithm: SignatureAlgorithm },
}

pub struct RateLimitSpec {
    pub requests_per_second: f64,
    pub burst: u32,
}
```

**增量同步（F-15-03）设计**：每个 API 连接器需实现 `Cursor` 类型的序列化/反序列化，`Cursor` 持久化于 `connector_sync_state` 表，字段为 `(tenant_id, adapter_id, cursor_json, updated_at)`。执行时优先读取上次 cursor，若无则执行全量首次同步。

### 4.7 通用 CRM/企业系统适配框架（F-16）

```rust
pub struct GenericConnectorTemplate {
    pub template_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub base_url: String,
    pub auth_method: AuthMethod,
    pub endpoints: Vec<EndpointSpec>,
    pub field_mapping: FieldMappingRules,
}

pub struct EndpointSpec {
    pub name: String,          // "list_orders", "get_customer" 等用户自定义命名
    pub method: HttpMethod,
    pub path_template: String, // 支持 {{variable}} 插值
    pub pagination: Option<PaginationSpec>,
}
```

该模板可保存为可复用配置（F-16-03），存储于 `connector_template` 表，`tenant_id` 隔离，同租户下其他画布可直接引用同一模板。

### 4.8 異常処理

| 异常场景 | 处理策略 |
|---|---|
| API 探测超时（>3s） | 记录为 `ApiUnavailable`，回退浏览器模式，不阻塞主流程 |
| 页面选择器失效（元素未找到） | 触发 F-12 自愈提示：捕获页面快照，通过启发式规则（文本相似度匹配）尝试推荐新选择器，写入待用户确认队列 |
| 浏览器崩溃 | `BrowserPool` 检测进程退出码，标记该实例不可用并从池中移除，触发节点级重试（依据 `RetryPolicy`） |
| 登录态失效（Cookie/Token 过期） | 抛出 `AdapterError::AuthExpired`，编排引擎捕获该错误并路由至用户通知节点 |

---

## 5. 標準化変換 詳細設計（M-02，対応 F-03）

### 5.1 字段映射引擎

```rust
pub struct FieldMappingRules {
    pub mappings: Vec<FieldMapping>,
}

pub struct FieldMapping {
    pub source_path: JsonPath,      // 如 "$.fields.author.name"
    pub target_path: JsonPath,      // 如 "$.fields.sender"
    pub transform: Option<TransformFn>,
}

pub enum TransformFn {
    Identity,
    ToUpperCase,
    ToLowerCase,
    DateFormat { from: String, to: String },
    Regex { pattern: String, replacement: String },
    Expression(String),   // 用户自定义表达式，见 5.2
    MaskSensitive { mask_type: MaskType },  // 敏感字段脱敏
}

pub enum MaskType {
    FullMask,             // ****
    PartialMask { keep_prefix: u8, keep_suffix: u8 },  // 138****1234
    Hash { algorithm: HashAlgorithm },
}
```

### 5.2 表达式引擎设计（F-03-03）

采用嵌入式表达式语言（推荐基于 `rhai` 或自研简化 DSL），语法限制在纯函数式子集内，禁止 I/O 操作，防止用户自定义表达式产生副作用或安全风险：

```
表达式示例：
  upper(payload.fields.author) + "_" + str(payload.fields.timestamp)

沙箱限制：
  - 执行超时：单次表达式求值 ≤ 50ms（超时则该字段置为 null 并记录警告）
  - 内存限制：表达式引擎堆内存 ≤ 8MB
  - 禁止网络/文件系统访问
```

### 5.3 转换流水线执行时序

```
输入 NJson(raw)
  │
  ▼
[1] Schema 校验（若 NodeDefinition.input_schema 已定义）
  │  失败 → 路由至异常分支，记录 ValidationError
  ▼
[2] 依 FieldMapping 列表逐条执行映射（并行执行，字段间无依赖）
  │
  ▼
[3] 敏感字段脱敏（若配置）
  │
  ▼
[4] Schema 校验（output_schema，若已定义）
  │
  ▼
输出 NJson(normalized)
```

---

## 6. データフローエンジン 詳細設計（M-03，対応 F-04）

### 6.1 连线（Edge）运行时表示

```rust
/// 每条数据流连线在运行时对应一个异步有界队列
pub struct DataFlowChannel {
    pub edge_id: String,
    pub sender: tokio::sync::mpsc::Sender<NJson>,
    pub receiver: Arc<Mutex<tokio::sync::mpsc::Receiver<NJson>>>,
    pub buffer_config: BufferConfig,
    pub metrics: ChannelMetrics,
}

pub struct ChannelMetrics {
    pub throughput_counter: AtomicU64,   // 累计流经数据包数
    pub current_queue_depth: AtomicUsize,
    pub last_activity: AtomicI64,        // Unix 时间戳，用于堆积检测告警
}
```

### 6.2 背压（Backpressure）机制

```
函数 send_with_backpressure(channel, item):
    根据 channel.buffer_config.overflow_policy 分支：

    Block（默认）:
        channel.sender.send(item).await   // tokio mpsc 的天然背压：队列满时 send 挂起
        // 上游节点的执行协程在此处自动暂停，直到下游消费腾出空间

    DropOldest:
        尝试 try_send(item)
        若队列满: 弹出队首最旧元素，再重试 try_send
        记录 metrics.dropped_count += 1，写入审计日志

    DropNewest:
        尝试 try_send(item)
        若队列满: 丢弃当前 item，不做重试
        记录 metrics.dropped_count += 1
```

### 6.3 数据缓存与重放（F-04-03）

下游节点暂停（如进入 `HumanReview` 等待用户确认）时，数据不丢失的关键在于：**tokio mpsc 队列本身即为缓存介质**，只要队列容量足够且上游不主动清空，数据天然驻留在内存中。

对于**跨进程重启**场景（Runtime 崩溃重启后仍需恢复未处理的数据），设计**持久化溢出策略**：

```rust
pub struct PersistentOverflowBuffer {
    // 当 in-memory 队列使用率超过 80% 时，触发向 Redis/本地文件溢出
    threshold_ratio: f64,
    backing_store: OverflowBackingStore,  // Redis List 或本地 sled 数据库
}
```

重放（Replay）功能：`M-07 调试服务` 可请求 `DataFlowEngine::replay_from_snapshot(execution_id, node_id)`，从持久化的节点输入快照重新构造 `NJson` 并重新注入对应 Edge 的队列。

### 6.4 流量监控可视化数据源（F-04-04）

`ChannelMetrics` 通过 Prometheus 指标格式暴露：

```
ada_dataflow_throughput_total{tenant_id, canvas_id, edge_id}
ada_dataflow_queue_depth{tenant_id, canvas_id, edge_id}
ada_dataflow_dropped_total{tenant_id, canvas_id, edge_id, reason}
```

前端通过 WebSocket 订阅这些指标的采样推送（默认 1s 间隔），驱动画布上连线的"流光动效"渲染速度与颜色（堆积越多颜色越偏红）。

---

## 7. オーケストレーションエンジン 詳細設計（M-04，対応 F-05）

### 7.1 状态机核心循环

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

### 7.2 条件分支/循环/汇聚节点的语义

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

### 7.3 异常捕获与重试策略（F-05-02, F-05-03）

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

### 7.4 LLM 语义决策节点设计（F-05-04）

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

### 7.5 状态持久化与断点续传（対応 7.1 可用性要件）

`StateStore` 接口的 PostgreSQL 实现每次 `checkpoint` 写入 `canvas_execution` 表的增量字段（`node_statuses` 以 JSONB 存储），Runtime 重启后通过 `ExecutionId` 恢复：

```rust
#[async_trait]
pub trait StateStore: Send + Sync {
    async fn checkpoint(&self, state: &ExecutionState) -> Result<(), StoreError>;
    async fn load_latest(&self, execution_id: ExecutionId) -> Result<Option<ExecutionState>, StoreError>;
}
```

---

## 8. 制御フロー実行器 詳細設計（M-05，対応 F-06）

### 8.1 并发调度器

```rust
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

### 8.2 暂停/恢复/终止语义

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

### 8.3 单节点手动触发（F-06-03，调试用）

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

---

## 9. ノードランタイム／プラグイン SDK 詳細設計（M-06，対応 F-07）

### 9.1 插件 Trait 与生命周期

```rust
#[async_trait]
pub trait NodePlugin: Send + Sync {
    fn plugin_id(&self) -> &str;
    fn input_schema(&self) -> JsonSchema;
    fn output_schema(&self) -> JsonSchema;

    async fn on_load(&mut self, ctx: &PluginContext) -> Result<(), PluginError>;
    async fn execute(&self, input: NJson, config: &serde_json::Value) -> Result<NJson, PluginError>;
    async fn on_unload(&mut self) -> Result<(), PluginError>;
}

pub struct PluginContext {
    pub tenant_id: Option<TenantId>,
    pub resource_limits: ResourceLimits,
}

pub struct ResourceLimits {
    pub max_memory_mb: u32,
    pub max_cpu_time_ms: u32,
    pub network_access: NetworkAccessPolicy,
}
```

### 9.2 双形态插件加载机制

| 插件形态 | 加载方式 | 隔离级别 | 适用场景 |
|---|---|---|---|
| Rust 原生插件 | `libloading` 动态库加载，需匹配 ABI 版本 | 进程内，无沙箱隔离 | 官方内置节点、性能敏感场景 |
| WASM 插件 | `wasmtime` 沙箱执行，通过 WASI 受限接口访问资源 | 沙箱隔离，CPU/内存受限 | 第三方/用户自定义插件 |

```rust
pub enum PluginRuntime {
    Native(Box<dyn NodePlugin>),
    Wasm { engine: wasmtime::Engine, module: wasmtime::Module, limits: ResourceLimits },
}

impl PluginRuntime {
    pub async fn execute(&self, input: NJson, config: &serde_json::Value) -> Result<NJson, PluginError> {
        match self {
            PluginRuntime::Native(p) => p.execute(input, config).await,
            PluginRuntime::Wasm { engine, module, limits } => {
                // 1. 创建带 fuel（燃料计量，限制执行步数防止死循环）的 Store
                // 2. 序列化 input 为 WASM 线性内存可读的字节流
                // 3. 调用导出函数 `execute`
                // 4. 超时/超内存则强制中断并返回 PluginError::ResourceLimitExceeded
            }
        }
    }
}
```

### 9.3 Schema 校验驱动的连线合法性检查（F-07-03）

前端在用户尝试连接两个节点时，向后端请求 `POST /api/v1/canvas/validate-edge`，后端执行：

```
函数 validate_edge_compatibility(source_node, target_node):
    source_schema = source_node.output_schema
    target_schema = target_node.input_schema
    返回 json_schema_compatible(source_schema, target_schema)
    // 兼容性判定：target_schema 的必填字段集合 ⊆ source_schema 声明的字段集合
```

---

## 10. マルチテナント・ミドルウェア 詳細設計（M-10，対応 F-17）

### 10.1 请求上下文注入中间件（Actix-web Middleware）

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

        // 4. 设置数据库连接的 RLS session 变量（关键：数据库层兜底隔离）
        db_pool.execute(&format!("SET app.current_tenant = '{}'", claims.tenant_id)).await?;

        self.service.call(req)
    }
}
```

### 10.2 配额检查与限流

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

### 10.3 租户生命周期状态机

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

### 10.4 数据库行级安全（RLS）策略实现细节

对基本設計書 5.2 节列出的每张多租户表，均需附加以下标准 RLS 策略模板：

```sql
ALTER TABLE {table_name} ENABLE ROW LEVEL SECURITY;

CREATE POLICY {table_name}_tenant_isolation ON {table_name}
  FOR ALL
  USING (tenant_id = current_setting('app.current_tenant', true)::uuid)
  WITH CHECK (tenant_id = current_setting('app.current_tenant', true)::uuid);

-- 关键：数据库连接池的每个连接在归还前必须 RESET app.current_tenant，
-- 防止连接复用时残留上一租户的会话变量导致隔离失效
```

**连接池防御设计**：使用 `deadpool-postgres` 时，在连接归还钩子（`RecycleMethod`）中显式执行 `RESET app.current_tenant`，并在单元测试中加入"连接复用后残留状态检测"用例（见第 17 章）。

---

## 11. 権限・協業モジュール 詳細設計（M-11，対応 F-11）

### 11.1 RBAC 数据模型

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

### 11.2 实时协作冲突解决（F-11-02）

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

### 11.3 审计追溯（F-11-03）

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

---

## 12. フロントエンド 詳細設計（M-12）

### 12.1 混合渲染架构总览

前端采用 **Bevy（ECS 游戏引擎）+ bevy_egui（即时模式 GUI）渲染画布本体，HTML Overlay（原生 DOM）承载中文输入密集的表单**，两套渲染管线在同一浏览器页面内并存，通过共享的视口变换状态保持视觉对齐。选型理由与风险应对详见基本設計書 3.2.1 节。

```
浏览器页面
 ├─ <canvas id="bevy-canvas">                    // Bevy WASM 应用挂载点，占满视口
 │   （由 Bevy ECS 系统驱动，非 DOM 树，以下为 ECS 实体/系统结构，非 HTML 组件）
 │
 └─ <div id="html-overlay-root">                 // 与 bevy-canvas 同尺寸叠加的 DOM 层，pointer-events 默认穿透
     ├─ <NodeConfigForm />                        // 节点配置表单，仅当前选中节点打开配置时挂载
     ├─ <ExpressionEditor />                      // 表达式/JSON 映射规则编辑器（CodeMirror）
     ├─ <DebugPanel />                             // 执行日志时间轴、数据快照 JSON 树
     └─ <TenantWorkspaceSwitcher />                // 工作空间/租户切换器（顶部固定栏，非画布跟随）
```

### 12.2 Bevy ECS 画布数据模型

```rust
// ===== Component 定义 =====

#[derive(Component)]
struct CanvasNode {
    node_id: String,
    node_type: NodeType,
}

#[derive(Component)]
struct CanvasPosition(Vec2);          // 画布逻辑坐标系（非屏幕像素坐标）

#[derive(Component)]
struct NodeVisualState {
    status: NodeStatus,               // 驱动节点边框颜色（Pending灰/Running蓝/Success绿/Failed红）
    selected: bool,
}

#[derive(Component)]
struct CanvasEdge {
    edge_id: String,
    from_entity: Entity,
    to_entity: Entity,
    edge_kind: EdgeKind,              // DataFlow(血液，实线流光) | ControlFlow(肌肉，虚线箭头)
}

#[derive(Component)]
struct DataFlowAnimation {
    throughput: f32,                  // 来自 ChannelMetrics，驱动流光粒子速度
    queue_pressure: f32,              // 0.0~1.0，驱动连线颜色渐变（绿→红）
}

// ===== Resource（全局单例状态）=====

#[derive(Resource)]
struct ViewportTransform {
    pan: Vec2,
    zoom: f32,                        // 限制范围 0.1 ~ 10.0（对应需求 10%～1000%）
}

#[derive(Resource)]
struct SpatialIndex {
    // R-tree 空间索引，加速视锥裁剪查询与框选命中检测
    tree: rstar::RTree<IndexedNode>,
}

#[derive(Resource)]
struct SelectionState {
    selected_node_ids: HashSet<String>,
}

#[derive(Resource)]
struct OpenConfigPanel {
    // 非 None 时，通知 HTML Overlay 层挂载对应节点的配置表单
    node_id: Option<String>,
    anchor_screen_pos: Vec2,           // 用于 Overlay 定位
}
```

### 12.3 核心 ECS 系统（Systems）

```rust
/// 每帧执行：将画布逻辑坐标转换为屏幕像素坐标，驱动 Bevy Transform
fn sync_node_screen_position(
    viewport: Res<ViewportTransform>,
    mut query: Query<(&CanvasPosition, &mut Transform), With<CanvasNode>>,
) {
    for (canvas_pos, mut transform) in query.iter_mut() {
        let screen_pos = (canvas_pos.0 - viewport.pan) * viewport.zoom;
        transform.translation = screen_pos.extend(0.0);
    }
}

/// 视锥裁剪：仅渲染视口 + buffer 区域内的节点，非可见节点组件标记为休眠
fn frustum_culling_system(
    viewport: Res<ViewportTransform>,
    spatial_index: Res<SpatialIndex>,
    mut query: Query<(&CanvasNode, &mut Visibility)>,
) {
    let buffer_margin = 200.0; // 画布单位，视口外缓冲区，避免快速平移时的闪烁
    let visible_bounds = expand_bounds(viewport.visible_bounds(), buffer_margin);
    let visible_ids: HashSet<String> = spatial_index.tree
        .locate_in_envelope(&visible_bounds)
        .map(|n| n.node_id.clone())
        .collect();

    for (node, mut visibility) in query.iter_mut() {
        *visibility = if visible_ids.contains(&node.node_id) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// 双击节点时，向 OpenConfigPanel Resource 写入请求，由 JS 胶水代码桥接挂载 HTML Overlay 表单
fn handle_node_double_click(
    mut open_panel: ResMut<OpenConfigPanel>,
    mut click_events: EventReader<NodeDoubleClickEvent>,
    query: Query<(&CanvasNode, &GlobalTransform)>,
) {
    for ev in click_events.read() {
        if let Ok((node, transform)) = query.get(ev.entity) {
            open_panel.node_id = Some(node.node_id.clone());
            open_panel.anchor_screen_pos = transform.translation().truncate();
        }
    }
}

/// 数据流吞吐指标驱动连线流光动效速度与颜色（対応 6.4 节 ChannelMetrics）
fn update_dataflow_animation(
    metrics: Res<DataFlowMetricsCache>,   // WebSocket 推送更新
    mut query: Query<(&CanvasEdge, &mut DataFlowAnimation)>,
) {
    for (edge, mut anim) in query.iter_mut() {
        if let Some(m) = metrics.get(&edge.edge_id) {
            anim.throughput = m.throughput as f32;
            anim.queue_pressure = (m.current_queue_depth as f32 / m.capacity as f32).clamp(0.0, 1.0);
        }
    }
}
```

### 12.4 HTML Overlay 桥接机制（中文输入表单）

```rust
// Bevy 侧：通过 wasm-bindgen 暴露状态变化事件给 JS 胶水层
#[wasm_bindgen]
pub fn get_open_config_panel_state() -> JsValue {
    // 序列化 OpenConfigPanel Resource 当前状态，供 JS 端轮询或由 Bevy 主动 postMessage
}

#[wasm_bindgen]
pub fn submit_config_panel_form(node_id: String, config_json: String) {
    // HTML 表单提交后回调此函数，将配置写回 ECS World 中对应 NodeDefinition.config
    // 并触发画布重渲染 + 通过 WebSocket 同步至后端持久化
}
```

```typescript
// JS/TS 胶水层：监听 Bevy 侧的面板打开请求，动态挂载/卸载 React 组件到 Overlay 层
bevyModule.onConfigPanelOpen((state: OpenConfigPanelState) => {
  if (state.node_id) {
    mountConfigForm(state.node_id, state.anchor_screen_pos); // ReactDOM.createRoot 挂载
  } else {
    unmountConfigForm();  // 关闭面板时立即卸载，避免常驻 DOM 增加开销
  }
});

// 表单提交时调用回 Bevy WASM 导出函数，写回 ECS 状态
function onFormSubmit(nodeId: string, configJson: string) {
  bevyModule.submit_config_panel_form(nodeId, configJson);
}
```

**坐标同步细节**：Overlay 表单容器使用 `position: absolute`，其 `left/top` 由 `OpenConfigPanel.anchor_screen_pos` 决定；Bevy 侧 `ViewportTransform` 每帧变化时，通过同一份共享状态（`SharedArrayBuffer` 或高频 `postMessage`，视浏览器兼容性选择）同步给 JS 层，Overlay 位置随画布平移/缩放实时跟随，视觉上表现为"挂在节点旁边的浮动面板"。

**技术选型**：Overlay 表单内的富文本/表达式编辑场景使用 CodeMirror 6（原生支持 IME 组合输入事件 `compositionstart`/`compositionupdate`/`compositionend`），普通字段使用原生 `<input>`/`<textarea>`。

### 12.5 视口虚拟化渲染性能保证（対応 7.2 性能要件：1000 节点 30fps）

| 优化点 | 实现方式 |
|---|---|
| 视锥裁剪 | R-tree 空间索引（12.3 节 `frustum_culling_system`），节点位置变更时仅更新局部索引节点，不整体重建 |
| 渲染批处理 | Bevy 的 Sprite Batching：同材质节点卡片合批绘制，减少 draw call |
| 连线渲染 | 使用 Bevy 的 `Gizmos` 或自定义 Mesh 而非逐帧生成新几何体，流光动效通过 Shader uniform 参数驱动（避免 CPU 端逐帧重算顶点） |
| ECS 查询优化 | 高频系统（如 `sync_node_screen_position`）使用 `Changed<T>` 过滤器，仅处理本帧发生变化的实体 |
| WASM 包体积 | 裁剪 Bevy 默认 feature（禁用 3D、音频、动画等未使用模块），启用 `wasm-opt -O3` |

### 12.6 状态管理架构

```rust
// Bevy Resource 承担传统前端框架中"全局 Store"的角色，按关注点分离为多个 Resource

#[derive(Resource, Default)]
struct CanvasState {
    canvas_id: Option<String>,
    version: u32,
}

#[derive(Resource, Default)]
struct ExecutionState {
    execution_id: Option<String>,
    node_statuses: HashMap<String, NodeStatus>,   // WebSocket 推送实时更新
}

#[derive(Resource, Default)]
struct TenantState {
    current_tenant_id: String,
    current_workspace_id: String,
    user_role: Role,
    quota: TenantQuotaView,
}

#[derive(Resource, Default)]
struct CollaborationState {
    // yrs（Yjs Rust 移植）文档句柄，协作者感知状态（Awareness）
    doc: Option<yrs::Doc>,
    remote_cursors: HashMap<String, RemoteCursor>,  // 驱动其他协作者光标实体的渲染
}
```

**本地状态与服务端权威状态的一致性策略**：客户端对节点位置拖拽等高频操作采用乐观更新（本地 ECS 立即响应），通过 WebSocket 异步同步至服务端；服务端为最终权威源，若同步失败或与其他协作者的 CRDT 合并结果冲突，客户端接收服务端校正后的状态并静默纠正本地渲染（不打断用户当前操作）。

---

## 13. API 詳細仕様（リクエスト/レスポンス）

### 13.1 画布创建

```
POST /api/v1/tenants/{tenant_id}/workspaces/{workspace_id}/canvases

Request Body:
{
  "name": "跨平台消息同步",
  "description": "...",
  "dag_json": { "nodes": [...], "edges": [...] }
}

Response 201:
{
  "canvas_id": "uuid",
  "version": 1,
  "created_at": "2026-08-18T10:00:00Z"
}

Response 403 (租户配额超限):
{
  "error_code": "QUOTA_EXCEEDED",
  "resource": "canvas_count",
  "limit": 50,
  "current": 50
}
```

### 13.2 画布执行触发

```
POST /api/v1/tenants/{tenant_id}/canvases/{canvas_id}/execute

Request Body:
{
  "trigger_type": "manual",       // manual | cron | webhook
  "entry_node_id": "node_001",    // 可选，指定入口节点
  "mock_input": null
}

Response 202:
{
  "execution_id": "uuid",
  "status": "pending"
}
```

### 13.3 WebSocket 事件推送协议

```
连接：ws://host/ws?token={jwt}&tenant_id={uuid}

服务端 → 客户端 事件类型清单：
┌─────────────────────────────┬──────────────────────────────────┐
│ type                          │ 触发时机                            │
├─────────────────────────────┼──────────────────────────────────┤
│ canvas.node.status_changed    │ 节点状态变更（Pending→Running→Success）│
│ canvas.dataflow.metrics       │ 每 1s 推送一次连线吞吐量指标          │
│ canvas.execution.completed    │ 整个画布执行完成                     │
│ canvas.execution.failed       │ 执行失败且无法自动恢复                │
│ collab.awareness_update       │ 其他协作者光标/选中状态变化           │
│ collab.doc_update              │ Yjs CRDT 增量更新                   │
│ tenant.quota_warning          │ 配额使用达到 80% 阈值                │
└─────────────────────────────┴──────────────────────────────────┘
```

---

## 14. エラーコード体系

| Error Code | HTTP Status | 説明 | 対応する処理層 |
|---|---|---|---|
| `TENANT_MISMATCH` | 403 | 请求路径租户与 Token 租户不一致 | M-10 |
| `QUOTA_EXCEEDED` | 429 | 配额超限（并发数/存储/API 调用次数） | M-10 |
| `ADAPTER_AUTH_EXPIRED` | 401 | 采集适配器凭证过期 | M-01 |
| `ADAPTER_NO_AVAILABLE_MODE` | 502 | API 与浏览器模式均不可用 | M-01 |
| `SELECTOR_NOT_FOUND` | 200*（节点级失败，不阻断 HTTP） | 页面选择器未匹配到元素 | M-01 |
| `SCHEMA_VALIDATION_FAILED` | 200*（节点级失败） | 数据未通过 JsonSchema 校验 | M-02 |
| `PLUGIN_RESOURCE_LIMIT_EXCEEDED` | 200*（节点级失败） | WASM 插件超出 CPU/内存限制 | M-06 |
| `EDGE_INCOMPATIBLE_SCHEMA` | 400 | 连线两端节点 Schema 不兼容 | M-06 |
| `EXECUTION_NOT_FOUND` | 404 | 查询的执行记录不存在或不属于当前租户 | M-04 |
| `CONCURRENT_EDIT_CONFLICT` | 409 | 画布版本冲突（非协作模式下的乐观锁冲突） | M-11 |

---

## 15. 状態遷移設計

### 15.1 节点执行状态机（对应第 3.3 节 NodeStatus）

```
Pending ──execute()──► Running ──success──► Success
                          │
                          └──failure(未达重试上限)──► Pending（重新排队重试）
                          │
                          └──failure(已达重试上限)──► Failed
                          │
                          └──abort()──► Skipped
```

### 15.2 画布执行整体状态机

```
pending ──dispatch()──► running ──all_nodes_terminal()──► success
                          │
                          ├──any_unrecovered_failure()──► failure
                          │
                          ├──pause()──► paused ──resume()──► running
                          │
                          └──abort()──► aborted
```

---

## 16. 並行性・排他制御設計

### 16.1 并发安全设计要点

| 设计点 | 采用机制 | 理由 |
|---|---|---|
| `ExecutionState` 并发读写 | 不可变数据结构（`im` crate 持久化数据结构）+ 单写者模型 | 避免锁竞争，编排引擎主循环是唯一写者，其余模块只读订阅 |
| 数据流 Channel | `tokio::sync::mpsc` 有界队列 | 天然提供背压，无需额外锁 |
| 租户配额计数 | Redis `INCR` + Lua 脚本（原子操作） | 避免竞态条件下的配额超发 |
| 画布配置乐观锁 | `canvas.version` 字段 + `WHERE version = ?` 条件更新 | 非实时协作场景下防止并发覆盖 |
| 浏览器实例池 | `tokio::sync::Semaphore` | 限制并发数且支持异步等待 |

### 16.2 画布配置并发写入冲突处理

```
用户 A（version=5）提交编辑
   │
   ▼
UPDATE canvas SET dag_json = ?, version = 6 WHERE canvas_id = ? AND version = 5
   │
   ├─ 影响行数 = 1 → 成功
   │
   └─ 影响行数 = 0（说明已被其他请求更新至更高 version）
        → 返回 409 CONCURRENT_EDIT_CONFLICT
        → 前端提示用户刷新并重新应用变更（非实时协作路径下的兜底）
```

注：处于**实时协作模式**（F-11-02，Yjs 已接管）时不会触发此冲突，因为所有编辑均通过 CRDT 合并；此机制仅用于非协作单用户编辑场景的数据完整性保护。

---

## 17. テスト観点（単体・結合）

### 17.1 单体测试重点

| 模块 | 测试重点 |
|---|---|
| M-01 采集适配器 | 双模式选择算法在 API 不可用时正确降级；限流令牌桶计算准确性；选择器失效检测 |
| M-03 数据流引擎 | 背压策略三种 `OverflowPolicy` 的行为验证；队列容量边界条件 |
| M-04 编排引擎 | 状态机在条件分支/循环/汇聚场景下的正确迁移；重试退避延迟计算 |
| M-06 插件 SDK | WASM 沙箱超时/超内存强制中断；Schema 兼容性校验的边界用例 |
| M-10 多租户中间件 | **连接池复用后残留租户会话变量检测**（关键安全测试）；配额检查的并发竞态测试 |

### 17.2 关键结合测试：多租户数据穿透测试用例

```
测试用例 TC-MT-001：
  前置条件：租户 A 与租户 B 各创建一个同名画布 "test_canvas"
  步骤：
    1. 以租户 A 的 Token 请求 GET /api/v1/tenants/{A}/canvases（应仅返回 A 的画布）
    2. 以租户 A 的 Token 但路径参数改为 tenant_id={B} 请求同一接口
    3. 直接使用租户 B 的 canvas_id，以租户 A 的 Token 请求 GET /canvases/{B的canvas_id}
  期望结果：
    步骤 2 → 403 TENANT_MISMATCH
    步骤 3 → 404（RLS 层面查询不到，不应用 403 以避免信息泄露资源存在性）

测试用例 TC-MT-002（连接池残留状态）：
  步骤：
    1. 连接池连接 C1 处理租户 A 的请求（SET app.current_tenant = 'A'）
    2. C1 归还连接池
    3. 若归还钩子未正确 RESET，则 C1 被复用于租户 B 的请求时会残留 'A' 的会话变量
  期望结果：C1 复用前必须观测到 app.current_tenant 已被 RESET 为默认值
```

### 17.3 性能测试基准（対応基本設計書 7.2 節）

| 测试项 | 目标值 | 测试方法 |
|---|---|---|
| 单画布 1000 节点渲染帧率 | ≥30fps | 前端 Performance API 采样，Chrome DevTools Protocol 自动化脚本 |
| 单节点吞吐（非采集类） | ≥100 条/秒 | 构造合成数据流，测量端到端处理延迟 |
| 采集并发浏览器实例数 | 依租户配额动态限制，超限排队而非失败 | 并发请求压测，验证 `BrowserPool` 排队行为 |

---

## 18. プロジェクト構成（Cargo Workspace 設計）

### 18.1 Workspace 目录结构与 crate 划分

模块清单（第 2 章）中的 M-01～M-13 对应到实际代码库，采用 Cargo Workspace 组织为多个独立 crate，边界与模块边界一一对应，便于独立编译、独立测试、未来可选的独立部署（如将 M-01 采集适配器拆为独立微服务）：

```
ada/
├─ Cargo.toml                      # workspace 根，members 列出所有子 crate
├─ crates/
│  ├─ ada-core-types/              # 対応第 3 章数据类型：NJson, CanvasDefinition, ExecutionState 等
│  │                                 # 被几乎所有其他 crate 依赖，故独立为最底层 crate，避免循环依赖
│  ├─ ada-acquisition/             # M-01 采集适配器（AcquisitionAdapter trait、Playwright 绑定、API 连接器）
│  ├─ ada-normalizer/              # M-02 标准化转换（字段映射、表达式引擎）
│  ├─ ada-dataflow/                # M-03 数据流引擎（Channel、背压、持久化溢出缓存）
│  ├─ ada-orchestration/           # M-04 编排引擎（状态机核心循环、StateStore trait）
│  ├─ ada-control-flow/            # M-05 控制流执行器（并发调度、暂停恢复）
│  ├─ ada-node-runtime/            # M-06 节点运行时/插件 SDK（NodePlugin trait、WASM 沙箱宿主）
│  ├─ ada-debug/                   # M-07 可视化调试服务
│  ├─ ada-trigger/                 # M-08 定时/事件触发器
│  ├─ ada-exporter/                # M-09 输出/导出模块
│  ├─ ada-tenant/                  # M-10 多租户中间件（TenantContext、QuotaEnforcer、RLS 会话管理）
│  ├─ ada-collab/                  # M-11 权限与协作模块（RBAC、yrs 集成、审计日志写入）
│  ├─ ada-frontend/                # M-12 前端（Bevy + bevy_egui，独立 wasm32-unknown-unknown 编译目标）
│  ├─ ada-api-gateway/             # M-13 API Gateway（Actix-web，二进制 crate，依赖以上除 ada-frontend 外的全部）
│  └─ ada-plugin-sdk/              # 面向第三方插件开发者发布的独立 SDK crate（対応 F-07-02），
│                                    # 仅包含 NodePlugin trait 与必要类型，不携带内部实现细节，可独立发版至 crates.io 或私有 registry
│
├─ plugins/                        # 官方内置插件的示例实现（各自独立 crate，依赖 ada-plugin-sdk）
│  ├─ plugin-lark-adapter/
│  ├─ plugin-slack-adapter/
│  └─ plugin-jira-connector/
│
├─ migrations/                     # sqlx/refinery 数据库迁移脚本，対応基本設計書第 5 章表结构
├─ deploy/                         # Kubernetes manifests / Helm chart / docker-compose（対応基本設計書 8 章部署策略）
└─ xtask/                          # 自定义构建脚本 crate（如 WASM 打包、wasm-opt 压缩流水线）
```

### 18.2 依赖方向约束

为避免模块边界在实现阶段被腐蚀（如 M-01 直接依赖 M-04 的内部类型），workspace 强制以下依赖方向：

```
ada-core-types  ← 被所有 crate 依赖，自身不依赖任何业务 crate

ada-acquisition, ada-normalizer, ada-exporter
      ↓ 依赖
ada-dataflow
      ↓ 依赖
ada-orchestration ← ada-control-flow（双向协作，但通过 trait 抽象解耦，
                                       ada-orchestration 只依赖 ada-control-flow 暴露的 trait，不依赖具体实现）
      ↓ 依赖
ada-node-runtime

ada-tenant, ada-collab  ← 横切关注点，被 ada-api-gateway 与各业务 crate 依赖，
                            但 ada-tenant/ada-collab 自身不反向依赖业务 crate

ada-api-gateway  ← 唯一的二进制 crate 入口，依赖除 ada-frontend 外的全部
ada-frontend     ← 独立 wasm32 编译目标，仅依赖 ada-core-types（数据结构共享），
                     不依赖任何服务端 crate，通过 HTTP/WebSocket 与 ada-api-gateway 通信而非直接函数调用
```

**CI 强制检查**：在 `xtask` 中实现一个依赖方向校验命令（基于 `cargo metadata` 解析依赖图），在 CI 流水线中作为强制检查项，禁止违反上述方向的新增依赖被合入主干。

### 18.3 关键 crate 的 Cargo.toml 特性标记（feature flags）

```toml
# crates/ada-node-runtime/Cargo.toml
[features]
default = ["native-plugins"]
native-plugins = ["libloading"]        # Rust 原生插件加载能力，服务端场景启用
wasm-plugins = ["wasmtime"]            # WASM 沙箱插件能力，两者可同时启用

# crates/ada-frontend/Cargo.toml
[features]
default = []
dev-inspector = ["bevy-inspector-egui"]  # 仅开发环境启用的 ECS 状态调试面板，生产构建禁用以控制包体积
```

---

## 19. 可観測性設計（ログ・メトリクス・トレース）

### 19.1 三大支柱与技术选型

| 支柱 | 技术选型 | 用途 |
|---|---|---|
| 结构化日志 | `tracing` + `tracing-subscriber`（JSON Lines 输出） | 対応要件文档 7.3 節日志要求，按节点/画布/租户过滤 |
| 指标（Metrics） | `metrics` crate + Prometheus exporter | 対応第 6.4 節 `ChannelMetrics` 等运行时指标暴露 |
| 分布式追踪（Tracing） | `tracing-opentelemetry` + OTLP exporter → Jaeger/Tempo | 跨模块调用链追踪，定位跨 M-01→M-02→M-03→M-04 的延迟瓶颈 |

### 19.2 统一追踪上下文传播

每个画布执行从触发入口起生成一个 `trace_id`（复用第 3.1 節 `NJson.trace_id` 的设计，二者共享同一追踪体系），贯穿编排引擎、数据流引擎、节点运行时的所有跨 crate 调用：

```rust
#[tracing::instrument(
    skip(state, control_flow_executor),
    fields(tenant_id = %state.tenant_id.unwrap_or_default(), execution_id = %state.execution_id)
)]
pub async fn run_orchestration_step(
    state: &ExecutionState,
    control_flow_executor: &ControlFlowExecutor,
) -> Result<ExecutionState, OrchestrationError> {
    // tracing::instrument 宏自动创建 Span，并将 tenant_id/execution_id 作为结构化字段附加
    // 所有该函数内部产生的日志与子 Span 自动继承这些字段，无需手动透传
}
```

**跨进程边界传播**（如 API Gateway → 通过消息队列异步触发的 Worker）：追踪上下文按 W3C Trace Context 标准（`traceparent` header）序列化，写入队列消息的 metadata 字段，Worker 端反序列化后恢复 Span 链路。

### 19.3 关键指标清单

```
# 対応需求文档 7.2 節性能要件的可观测化
ada_canvas_execution_duration_seconds{tenant_id, canvas_id, status}          (Histogram)
ada_node_execution_duration_seconds{tenant_id, node_type, status}            (Histogram)
ada_dataflow_throughput_total{tenant_id, canvas_id, edge_id}                 (Counter，已见 6.4 節)
ada_dataflow_queue_depth{tenant_id, canvas_id, edge_id}                      (Gauge)
ada_browser_pool_active_instances{tenant_id}                                 (Gauge，対応 4.4 節 BrowserPool)
ada_browser_pool_wait_duration_seconds{tenant_id}                            (Histogram)
ada_tenant_quota_usage_ratio{tenant_id, resource}                            (Gauge，対応 10.2 節 QuotaEnforcer)
ada_plugin_execution_errors_total{plugin_id, error_type}                     (Counter)
ada_websocket_active_connections{tenant_id}                                  (Gauge)
```

**告警规则示例**（Prometheus AlertManager）：

```yaml
- alert: TenantQuotaNearLimit
  expr: ada_tenant_quota_usage_ratio > 0.8
  for: 5m
  labels: { severity: warning }
  annotations:
    summary: "租户 {{ $labels.tenant_id }} 的 {{ $labels.resource }} 配额使用超过 80%"
    # 対応需求文档 7.5 節 tenant.quota_warning WebSocket 事件的服务端触发源

- alert: DataflowQueueBacklog
  expr: ada_dataflow_queue_depth / ada_dataflow_queue_capacity > 0.9
  for: 2m
  labels: { severity: critical }
  annotations:
    summary: "画布 {{ $labels.canvas_id }} 连线 {{ $labels.edge_id }} 数据积压严重，可能触发背压阻塞"
```

### 19.4 日志脱敏与多租户隔离

结构化日志中禁止直接记录 `NJson.payload.fields` 的原始内容（可能含用户敏感数据），仅记录元信息（`trace_id`、`node_type`、`record_count`、耗时）；若调试场景确需记录数据内容，须先经过与第 5.1 節 `MaskSensitive` 相同的脱敏规则处理。日志系统本身也需按 `tenant_id` 做访问隔离——日志查询 API 强制注入租户过滤条件，与第 10 章 M-10 多租户中间件的 RLS 思路一致（日志存储若使用 Loki，则通过 label `tenant_id` 实现等效隔离）。

---

## 20. 用語索引・変更履歴

### 20.1 本文档特有术语补充

| 用語 | 定義 |
|---|---|
| CRDT | Conflict-free Replicated Data Type，一种支持多副本无冲突合并的数据结构，本系统用于实时协作编辑 |
| RLS | Row-Level Security，PostgreSQL 提供的行级访问控制机制，本系统用作多租户隔离的数据库层防线 |
| 令牌桶（Token Bucket） | 一种限流算法，本系统用于控制采集适配器的访问频率 |
| 燃料计量（Fuel Metering） | WASM 沙箱中用于限制代码执行步数的机制，防止插件死循环耗尽资源 |
| 乐观锁（Optimistic Lock） | 基于版本号比对的并发控制方式，用于非实时协作场景下的画布配置更新 |
| ECS | Entity-Component-System，Bevy 的核心架构模式：实体（Entity）仅为 ID，组件（Component）为纯数据，系统（System）为无状态函数，本系统用其表达画布中的节点/连线/视口等运行时对象 |
| HTML Overlay | 叠加在 Bevy WASM Canvas 之上的独立 DOM 层，用于承载依赖浏览器原生能力（尤其中文输入法 IME）的表单类交互 |
| 视锥裁剪（Frustum Culling） | 仅渲染可视区域内实体的优化技术，源自游戏引擎术语，本系统用于大规模节点画布的性能优化 |
| OTLP | OpenTelemetry Protocol，分布式追踪数据的标准传输协议，本系统用于跨模块调用链追踪的数据导出 |
| W3C Trace Context | 跨进程/跨服务传播追踪上下文的标准 HTTP Header 格式（`traceparent`），本系统用于消息队列场景下的追踪链路延续 |

### 20.2 变更履历

| バージョン | 日付 | 変更内容 | 作成者 |
|---|---|---|---|
| 1.0.0 | 2026-08-18 | 初版制定：基于要件定義書 v1.2.0 与基本設計書 v1.0.0，完成 M-01～M-13 全模块详细设计，涵盖数据结构、核心算法、状态机、并发控制、API 规格、错误码体系与测试观点 | Ada プロジェクトチーム |
| 1.1.0 | 2026-08-18 | 前端（M-12）详细设计改为 Bevy + bevy_egui（画布 ECS 渲染）+ HTML Overlay（中文输入表单）混合架构：新增 ECS 数据模型、核心系统（视锥裁剪/坐标同步/流光动效）、Overlay 桥接机制（wasm-bindgen 双向调用）、状态管理架构；协作模块技术栈由 Yjs 调整为 yrs（Rust 移植版） | Ada プロジェクトチーム |
| 1.2.0 | 2026-08-18 | 新增第 18 章 Cargo Workspace 工程结构设计（crate 划分、依赖方向约束、feature flags）；新增第 19 章可观测性设计（tracing/metrics/OpenTelemetry 三支柱、统一追踪上下文传播、关键指标清单与告警规则、日志脱敏与多租户隔离）；补充术语索引 | Ada プロジェクトチーム |

---

*本文档为詳細設計書，是编码实现与单元测试用例编写的直接依据。若后续需求或基本设计发生变更，须同步更新本文档相应章节并递增版本号。*
