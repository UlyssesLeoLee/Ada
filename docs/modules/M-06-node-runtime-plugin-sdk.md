# M-06 节点运行时 / 插件 SDK（Node Runtime / Plugin SDK）

> **ドキュメントID**：DOC-MOD-006
> **文書分類**：モジュール別設計書
> **バージョン**：v1.2.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §6）、`docs/modules/M-14`（DOC-MOD-014 热插拔联动）
> **関連文書**：`docs/modules/M-01`（DOC-MOD-001）、`docs/modules/M-02`（DOC-MOD-002）、`docs/modules/M-09`（DOC-MOD-009）
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
| v1.2.0 | 2026-08-19 | モジュールレベル ホットスワップ拡張（§3.4）追加 | Ada プロジェクトチーム | TBD | TBD |

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

- **F-07** 节点库与插件市场
- **F-02-02** 登录态保持（凭证存储由本模块 SDK 暴露接口，实际存储依赖 [M-01 §3.2 credential 表](../modules/M-01-acquisition-adapter.md)）

### 1.2 关联用例

U-01 跨平台内容同步、U-04 数据清洗与人工复核

### 1.3 接口要件

- I-04 Runtime ↔ 插件：定义插件 SDK 接口规范，支持 Rust 原生插件与 WASM 沙箱插件两种形态；多租户环境下插件执行需在租户级别的资源限制内

### 1.4 非功能需求

- 7.2 性能：单节点数据处理吞吐 ≥ 100 条/秒
- 7.3 运用保守性：插件热更新（推奨项，非必須）
- 7.5 安全：多租户隔离

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中的"骨骼层"——具体节点实现（[M-01 采集适配器](../modules/M-01-acquisition-adapter.md)、[M-02 标准化转换](../modules/M-02-normalizer.md)、[M-09 输出导出](../modules/M-09-exporter.md)）的宿主与生命周期管理者。

### 2.2 插件扩展（basic-design §3.2.5 末尾）

- **Rust 原生插件**：通过 `libloading` 动态加载，需声明 `NodePlugin` trait
- **WASM 插件**：通过 `wasmtime` 沙箱执行，限制 CPU/内存/I/O

### 2.3 内置节点分类

按 F-07-01：

- 采集类（由 [M-01](../modules/M-01-acquisition-adapter.md) 实现）
- 转换类（由 [M-02](../modules/M-02-normalizer.md) 实现）
- 路由/条件类（由 [M-04](../modules/M-04-orchestration-engine.md) 实现）
- 输出类（由 [M-09](../modules/M-09-exporter.md) 实现）
- 人工介入类（由本模块管理生命周期）

## 3. 详细设计（詳細設計書）

### 3.1 插件 Trait 与生命周期

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

### 3.2 双形态插件加载机制

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

### 3.3 Schema 校验驱动的连线合法性检查（F-07-03）

前端在用户尝试连接两个节点时，向后端请求 `POST /api/v1/canvas/validate-edge`（参见 [api/rest-endpoints.md §1.4](../api/rest-endpoints.md)），后端执行：

```
函数 validate_edge_compatibility(source_node, target_node):
    source_schema = source_node.output_schema
    target_schema = target_node.input_schema
    返回 json_schema_compatible(source_schema, target_schema)
    // 兼容性判定：target_schema 的必填字段集合 ⊆ source_schema 声明的字段集合
```

兼容性失败时，对应错误码为 [`EDGE_INCOMPATIBLE_SCHEMA`](../api/error-codes.md)（HTTP 400）。

### 3.4 モジュールレベル ホットスワップ拡張（v1.1.0 追加，[DOC-MOD-014 §3.7](../modules/M-14-module-registry.md) 参照）

> 本節は [DOC-ARCH-005 §7 热插拔协议](../architecture/04-atomic-deployment.md) の下位設計。単一プラグインではなく**モジュール全体**のホットスワップを支える。

#### 3.4.1 拡張された PluginRuntime

```rust
pub enum PluginRuntime {
    Single(Box<dyn NodePlugin>),  // 旧版互換
    Module {
        manifest: ModuleManifest,
        plugins: Vec<Box<dyn NodePlugin>>,
        routes: Vec<RouteSpec>,
    },
    WasmModule {                    // WASM モジュール全体
        manifest: ModuleManifest,
        module: wasmtime::Module,
        exports: Vec<WasmExportSpec>,
    },
}
```

#### 3.4.2 モジュールライフサイクル

```
register(deps) → instantiate → health_check → register_routes
   ↓
activate (旧版との dual-running 開始)
   ↓
drain_old (旧版の進行中リクエストを待ってから deactivate)
   ↓
unload_old
```

各状態遷移は [DOC-MOD-014 §3.2 状態機](../modules/M-14-module-registry.md) に従い、PL/pgSQL `register_module_state_transition()` で永続化 + `module.state_changed` イベント発火。

#### 3.4.3 ロード時間検証

`on_load` で SHA256 検証を実施し、ロード済みアーティファクトの整合性を保証：

```rust
pub async fn verify_and_load(
    artifact_url: &str,
    expected_sha256: &str,
) -> Result<ModuleArtifact, LoadError> {
    let bytes = download(artifact_url).await?;
    let actual = sha256(&bytes);
    if actual != expected_sha256 {
        return Err(LoadError::HashMismatch { expected: expected_sha256.to_string(), actual });
    }
    Ok(ModuleArtifact { bytes, sha256: actual })
}
```

#### 3.4.4 0 ダウンタイム保証

- `activate` 前にルートテーブルを変更しない（新規モジュールは不可視）
- `drain` 中、旧モジュールは処理中リクエストを自然完了させる
- 両バージョン共存ウィンドウ：新規 activate → 旧版 drain 中、両者登録状態
- 失敗時：drain タイムアウト or health_check 失敗 → 旧版を即座に再 activate

#### 3.4.5 旧版 SDK との互換性

- `PluginRuntime::Single` 経路は旧版そのまま動作（v1.0 互換）
- 新規コードは `PluginRuntime::Module` への移行を推奨
- Manifest 不在の旧プラグインも読み込み可能だが、ライフサイクル管理は [DOC-MOD-014](../modules/M-14-module-registry.md) の対象外

## 4. 验收要点

1. **双形态插件可加载**：内置节点以 Rust 原生形式加载，第三方插件以 WASM 形式加载，均能正常执行。
2. **资源限制生效**：WASM 插件超出 CPU/内存限制时返回 `PLUGIN_RESOURCE_LIMIT_EXCEEDED` 节点级错误，不影响 Runtime 主进程。
3. **连线校验**：节点间 Schema 不兼容时拒绝连线，给出明确错误码。
4. **生命周期管理**：`on_load` / `execute` / `on_unload` 三阶段按序触发，异常时不留下半初始化状态。
5. **多租户资源隔离**：插件执行受 `tenant_id` 对应的 ResourceLimits 约束。 [NF-SEC]【必須】
6. **模块级热插拔零停机**：通过 rolling 策略升级 m01-acquisition 到 1.5.0 期间，旧版本仍服务进行中请求，业务不中断。 [NF-AVA]【必須】
7. **SHA256 校验**：加载阶段校验失败时立即拒绝，不进入 Loaded 状态。 [NF-SEC]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 节点运行时 | 节点宿主与生命周期管理 | §1、DOC-ARCH-001 |
| NodePlugin Trait | 插件统一接口 | §3.1 |
| libloading | Rust 原生插件动态加载 | §3.2 |
| wasmtime | WASM 沙箱执行引擎 | §3.2 [NF-SEC]【必須】 |
| PluginRuntime | 插件运行时枚举（Native/Wasm） | §3.2 |
| ResourceLimits | 资源限制（CPU/内存/网络） | §3.2 |
| validate_edge_compatibility | 连线两端 Schema 兼容性判定 | §3.3 |
| EDGE_INCOMPATIBLE_SCHEMA | 兼容性失败的错误码 | §3.3 |
| 沙箱隔离 | WASM 进程级隔离 | §3.2 [NF-SEC]【必須】 |
| 插件热更新 | 节点插件无重启生效 | §3.1 [NF-OPS]【推奨】 |
| 模块热插拔 | 整个模块不停机升级 | §3.5 [NF-AVA]【必須】 |
| Manifest | 模块元数据 | §3.5.1 [NF-OPS]【必須】 |
| 双版本共存 | activate-drain 重叠期 | §3.5.4 [NF-AVA]【必須】 |

## 6. 参考文献

1. IPA「共通フレーム2018」(SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. wasmtime 公式ドキュメント「wasmtime — A fast and secure runtime for WebAssembly」
4. libloading 公式ドキュメント「libloading — A safer binding for dynamic loading of libraries」
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
