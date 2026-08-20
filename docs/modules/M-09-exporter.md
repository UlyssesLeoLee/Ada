# M-09 输出/导出（Exporter）

> **ドキュメントID**：DOC-MOD-009
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §9）
> **関連文書**：`docs/modules/M-01`（DOC-MOD-001）、`docs/modules/M-10`（DOC-MOD-010）
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

- **F-14** 数据导出与二次分发

### 1.2 关联用例

U-01 跨平台内容同步、U-02 多源数据聚合看板、U-06 IM 消息双向联动、U-07 项目管理数据同步

### 1.3 接口要件

- I-03 Runtime ↔ 外部输出目标：支持 HTTP Webhook、数据库连接（PostgreSQL/MySQL/SQLite 驱动）、本地文件写入

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中的"骨骼层"末端——数据流转生命周期的最后一环（requirements §8.2）：

```
采集 → 标准化转换 → 数据流引擎入血管 → 编排引擎决策 → 控制流调度执行下游节点 → 输出/二次分发（本模块）★
```

### 2.2 内置输出节点

F-14-01 内置输出节点：

- **本地文件**：JSON / CSV
- **数据库**：SQLite / PostgreSQL / MySQL
- **HTTP Webhook 推送**
- **写回目标平台**：若目标平台支持表单提交类操作，含 IM 消息发送、Jira 工单创建等

## 3. 详细设计（詳細設計書）

### 3.1 输出节点 Trait

```rust
#[async_trait]
pub trait Exporter: Send + Sync {
    fn exporter_id(&self) -> &str;
    /// 将 NJson 数据包按目标格式输出
    async fn export(&self, items: Vec<NJson>, config: &ExporterConfig) -> Result<ExportResult, ExportError>;
}

pub enum ExporterConfig {
    File { path: String, format: FileFormat },                  // JSON | CSV
    Database { dsn: String, table: String, write_mode: WriteMode },
    Webhook { url: String, headers: HashMap<String, String>, retry_policy: RetryPolicy },
    PlatformWrite { platform: String, action: PlatformAction }, // 复用 M-01 的 send_message 等
}
```

### 3.2 数据库输出

- **DSN 加密存储**：连接字符串包含密码等敏感信息，需通过 [M-10 §4.2 credential 表](../modules/M-10-tenant-middleware.md) 加密存储
- **多租户模式支持**：写入数据库时若使用本系统的 PostgreSQL/SQLite，需走 [M-10 §3.1 `with_tenant_scope`](../modules/M-10-tenant-middleware.md) 自动注入租户隔离
- **写模式**：
  - `Insert`：纯插入
  - `Upsert`：按主键存在则更新
  - `Append`：追加到表尾

### 3.3 平台写回（与 M-01 协作）

平台写回（IM 消息发送、Jira 工单创建等）复用 [M-01 §3.2 `send_message`](../modules/M-01-acquisition-adapter.md) 的双向能力，无需在本模块重复实现。

```rust
// 伪代码：ExporterConfig::PlatformWrite 分支
match config {
    ExporterConfig::PlatformWrite { platform, action } => {
        let adapter = registry.get_adapter(platform)?;
        adapter.send_message(adapter_config, action.into_outbound_message()).await?;
    }
    // ...
}
```

## 4. 验收要点

1. **四类输出节点可用**：本地文件（JSON/CSV）、数据库（SQLite/PostgreSQL/MySQL）、Webhook、平台写回四类均能正常输出。
2. **数据库连接安全**：DSN 加密存储，凭证访问写入审计日志。
3. **多租户隔离**：写入本系统数据库时自动带上 `tenant_id`，符合 RLS 策略。
4. **重试与退避**：Webhook 输出可配置重试策略（与 [M-04 §3.3 异常捕获与重试](../modules/M-04-orchestration-engine.md) 一致）。
5. **大体积数据**：文件输出支持流式写入，不因一次写满内存导致 OOM。 [NF-PER]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 输出节点 | 数据流转末端 | §1 |
| Exporter Trait | 输出节点统一接口 | §3.1 |
| ExporterConfig | 4 类输出配置枚举 | §3.1 |
| DSN | 数据库连接字符串（加密存储） | §3.2 [NF-SEC]【必須】 |
| Upsert | 主键冲突时更新 | §3.2 |
| 平台写回 | 复用 M-01 send_message 写回 IM/工单系统 | §3.3 |
| 流式写入 | 不一次性加载全部数据 | §3.1 [NF-PER]【必須】 |
| 重试与退避 | 与 RetryPolicy 一致 | §3.1 |
| 多租户隔离 | 自动注入 tenant_id | §3.2 [NF-SEC]【必須】 |
| Webhook 输出 | HTTP POST 推送 | §3.1 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
