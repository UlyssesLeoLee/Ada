# M-01 采集适配器（Acquisition Adapter）

> **ドキュメントID**：DOC-MOD-001
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）、`docs/architecture/00-anatomy-model.md`（DOC-ARCH-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §1）、`docs/tests/IT-design.md`（DOC-TST-002 §1, §3, §4）
> **関連文書**：`docs/modules/M-02`（DOC-MOD-002）、`docs/modules/M-06`（DOC-MOD-006）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章「システム開発プロセス」
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

> 旧版注記：旧版で「上位文档」として `architecture/00-anatomy-model.md` を引用。本版以降は IPA 標準のメタデータ表に統合。

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-18 | 初版制定（DOC-DTL-001 §4 + DOC-BSC-001 §3.2.5 + DOC-REQ-001 F-02/F-15/F-16 集約） | Ada プロジェクトチーム | TBD | TBD |
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

- **F-02** 采集适配器（双模式：API + Playwright）——本模块主功能
- **F-15** API 连接器管理
- **F-16** 通用 CRM／企业系统适配框架

### 1.2 关联用例

U-01 跨平台内容同步、U-02 多源数据聚合看板、U-06 IM 消息双向联动、U-07 项目管理数据同步、U-08 企业系统数据集成

### 1.3 数据要件

- NJSON Schema（8.1 节）中 `source.platform` / `source.adapter_id` / `source.adapter_mode` 字段
- 原始快照 `raw_ref` 默认保留 7 天（8.3 节）

### 1.4 接口要件

- I-02 Runtime ↔ Playwright：通过 CDP（Chrome DevTools Protocol）驱动浏览器实例；多租户环境下浏览器实例池按租户隔离
- I-04 Runtime ↔ 插件（节点）：定义插件 SDK 接口规范

### 1.5 非功能需求

- 7.2 性能：单节点数据处理吞吐 ≥ 100 条/秒（不含浏览器采集类节点）
- 7.5 安全：凭证加密存储、传输 HTTPS/WSS、合规使用边界提示
- 10.1 约束：采取严格反爬机制的平台无法保证 100% 稳定；不承诺突破 CAPTCHA；Playwright 浏览器内核体积大（100~300MB/浏览器）

### 1.6 合规提示（F-02-06）

采集适配器需在**授权合法使用**前提下运行，系统应在文档与界面中明确提示用户遵守目标平台服务条款及所在地法律法规，不得用于绕过访问控制、爬取受版权/隐私保护的非公开数据。

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中的"骨骼层 + 外部环境层（Playwright）"，是节点运行时的具体实现之一（3.2.5 节点运行时）。

### 2.2 双模式策略

"**API 连接器优先，无 API 场景回退 Playwright 浏览器自动化采集**"双模式策略，统一转换为标准化 JSON。运行时按平台能力自动选择，或由用户手动指定优先级。

### 2.3 涉及表

| 表 | 用途 | 权限隔离 |
|---|---|---|
| `credential` | 凭证库（OAuth2 token、API key、Cookie session）加密存储 | RLS by tenant_id |
| `connector_template` | F-16 通用 CRM/企业系统连接器模板 | RLS by tenant_id |
| `connector_sync_state` | F-15 增量同步游标 | RLS by tenant_id |

### 2.4 关键安全设计（basic-design §6.2）

- 所有目标平台的敏感凭证（密码、Token、Cookie）需 AES-256 加密存储
- 加密密钥由 KMS（如 HashiCorp Vault）管理，不硬编码
- 在数据库中仅存储加密后的值，无法反向解密
- 凭证访问需审计日志

## 3. 详细设计（詳細設計書）

### 3.1 职责边界

- **输入**：`NodeDefinition { node_type: Acquisition }` 中的适配器配置
- **输出**：`Stream<NJson>`（异步流）
- **不负责**：字段级清洗映射（由 [M-02](../modules/M-02-normalizer.md) 负责），仅负责"从平台取到数据 + 转换为初步 NJSON 骨架"

### 3.2 双模式适配器 Trait 设计

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

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("该适配器不支持此操作")]
    Unsupported,
    #[error("API 与浏览器模式均不可用")]
    NoAvailableMode,
    #[error("登录态/凭证已过期")]
    AuthExpired,
    #[error("页面选择器未匹配到元素: {selector}")]
    SelectorNotFound { selector: String },
    #[error("浏览器实例池获取超时或配额耗尽")]
    PoolExhausted,
    #[error("上游平台返回错误: {status_code} {message}")]
    UpstreamError { status_code: u16, message: String },
    #[error("网络请求失败: {0}")]
    NetworkError(String),
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

### 3.3 双模式选择算法（F-02-00 对应实现）

```
函数 select_adapter_mode(config, adapter):
    1. 如果 config.forced_mode 已指定（用户手动优先级）:
         返回 config.forced_mode
    2. 否则：
         result = adapter.probe_api_availability(config)
         如果 result == ApiAvailable:
             返回 AdapterMode::Api
         否则：
             如果 adapter.supported_modes() 包含 Browser:
                 记录降级日志（audit_log: "adapter_fallback_to_browser"）
                 返回 AdapterMode::Browser
             否则：
                 返回 错误 AdapterError::NoAvailableMode
```

**探测缓存策略**：`probe_api_availability` 结果按 `(tenant_id, adapter_id)` 缓存于 Redis，TTL 5 分钟，避免每次执行都重新探测导致延迟。

### 3.4 Playwright 浏览器自动化子模块

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

### 3.5 采集频率限流设计（F-02-04）

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

### 3.6 API 连接器管理子模块（F-15）

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

### 3.7 通用 CRM/企业系统适配框架（F-16）

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

### 3.8 异常处理

| 异常场景 | 处理策略 |
|---|---|
| API 探测超时（>3s） | 记录为 `ApiUnavailable`，回退浏览器模式，不阻塞主流程 |
| 页面选择器失效（元素未找到） | 触发 F-12 自愈提示：捕获页面快照，通过启发式规则（文本相似度匹配）尝试推荐新选择器，写入待用户确认队列 |
| 浏览器崩溃 | `BrowserPool` 检测进程退出码，标记该实例不可用并从池中移除，触发节点级重试（依据 `RetryPolicy`） |
| 登录态失效（Cookie/Token 过期） | 抛出 `AdapterError::AuthExpired`，编排引擎捕获该错误并路由至用户通知节点 |

## 4. 验收要点

来自 [architecture/03-cross-cutting-risks.md §4](../architecture/03-cross-cutting-risks.md)：

1. **10 分钟配置**：能够在无官方 API 的示例平台上，通过可视化点选方式在 10 分钟内配置完成一个采集节点，并成功采集并标准化至少 1 条数据。
2. **零代码端到端**：画布可完成"采集 → 转换 → 条件路由 → 输出至本地文件"的端到端流程搭建与执行。
3. **合规提示**：系统在采集节点配置界面明确展示合规使用提示文案（F-02-06）。
4. **双模式选择正确性**：覆盖三种场景——"仅 API 可用"走 API 路径、"仅浏览器可用"自动回退浏览器、"两者都可用"按用户优先级或探测结果选择。
5. **多租户浏览器隔离**：A 租户的浏览器实例耗尽不影响 B 租户的实例可用性。 [NF-SEC]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 采集适配器 | 目标平台接入插件单元 | §1、DOC-REQ-001 §2 |
| 双模式 | API 模式 + 浏览器模式并行策略 | §1.1、§3.3 |
| AcquisitionAdapter Trait | 采集适配器统一接口 | §3.2 |
| BrowserPool | 按租户隔离的浏览器实例池 | §3.4 |
| RateLimiter | 令牌桶算法的采集频率限流 | §3.5 |
| OAuth2 Token | OAuth 2.0 协议的访问令牌 | §3.6 |
| 增量同步 | 基于 cursor 的增量数据拉取 | §3.6 |
| 通用 CRM 模板 | F-16 适配框架的可复用模板 | §3.7 |
| F-12 自愈 | 选择器失效后的自动修复提示 | §3.8 |
| AdapterMode | 适配器工作模式枚举（Api / Browser） | §3.2 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Playwright 公式ドキュメント「Playwright — Reliable end-to-end testing for modern web apps」
4. OAuth 2.0 仕様 (IETF RFC 6749)
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 要件定義書 v1.2.1」、2026-08-18（[DOC-REQ-001](../legacy/requirements.md)）
6. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 基本設計書 v1.3.0」、2026-08-18（[DOC-BSC-001](../legacy/basic-design.md)）
7. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
