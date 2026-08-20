# M-08 定时/事件触发器（Trigger Service）

> **ドキュメントID**：DOC-MOD-008
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §8）
> **関連文書**：`docs/modules/M-04`（DOC-MOD-004）、`docs/modules/M-10`（DOC-MOD-010）
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

- **F-13** 定时/事件触发

### 1.2 关联用例

U-01 跨平台内容同步、U-02 多源数据聚合看板、U-03 事件触发式自动化

### 1.3 接口要件

- I-01 前端 Web UI ↔ Runtime：包含触发器配置 API

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于触发器服务，独立模块但与 [M-04 编排引擎](../modules/M-04-orchestration-engine.md) 紧密协作——触发器在事件到达时调用编排引擎的入口创建执行。

### 2.2 主要职责

- Cron 表达式定时触发整个画布或指定入口节点
- 基于 Webhook 的外部事件触发
- 手动触发（来自前端 UI 或 [api/rest-endpoints.md §2.2](../api/rest-endpoints.md)）

## 3. 详细设计（詳細設計書）

### 3.1 触发器类型

```rust
pub enum Trigger {
    /// Cron 定时触发
    Cron {
        schedule: String,                  // 标准 5 段 cron 表达式
        timezone: chrono_tz::Tz,           // 租户级时区
        canvas_id: uuid::Uuid,
        entry_node_id: Option<String>,
    },
    /// Webhook 外部事件触发
    Webhook {
        path: String,                      // e.g. "/api/v1/webhooks/{trigger_id}"
        secret: Option<String>,            // HMAC 签名校验
        canvas_id: uuid::Uuid,
        entry_node_id: Option<String>,
    },
    /// 手动触发
    Manual {
        user_id: UserId,
        canvas_id: uuid::Uuid,
        entry_node_id: Option<String>,
        mock_input: Option<NJson>,
    },
}
```

### 3.2 触发执行

```rust
impl TriggerService {
    /// 统一触发入口，供 Cron / Webhook / Manual 三类触发器共用
    pub async fn dispatch(&self, trigger: Trigger) -> Result<ExecutionId, TriggerError> {
        // 1. 配额检查（[M-10 §3.2 配额检查](../modules/M-10-tenant-middleware.md)）
        // 2. 加载画布定义（CanvasDefinition）
        // 3. 创建 ExecutionState
        // 4. 提交到 M-04 编排引擎的 run() 入口
        // 5. 立即返回 execution_id（不阻塞）
    }
}
```

### 3.3 Cron 调度器实现

- 基于 `tokio-cron-scheduler` 或自研的 cron 表达式解析器
- 每个 Cron 触发器由独立 task 监听，到点调用 §3.2 `dispatch`
- 多租户环境下 cron 调度器按租户隔离，单租户的调度失败不影响其他租户

### 3.4 Webhook 安全

- 路径命名空间：`/api/v1/webhooks/{trigger_id}`
- 可选 HMAC 签名校验（`secret` 字段配置）：校验 `X-Hub-Signature-256` 头
- 防重放：使用 `X-Ada-Webhook-Timestamp` 头与 `timestamp ± 5min` 窗口比对

## 4. 验收要点

1. **Cron 准确性**：在指定时区下 cron 表达式按预期时刻触发。
2. **Webhook 安全**：未携带正确签名的请求返回 401。
3. **触发幂等**：同一 `trigger_id` 在极短时间内收到多个 Webhook 请求，调度器去重不重复执行。
4. **多租户隔离**：A 租户的 Cron 触发器在 B 租户资源紧张时仍按计划执行。
5. **手动触发可定位**：手动触发的 `execution_id` 在审计日志中可追溯到具体用户。 [NF-OPS]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 触发器 | 画布执行的外部驱动 | §1 |
| Cron 表达式 | 5 段定时表达式 | §3.1 |
| Webhook | HTTP 回调触发 | §3.1 |
| 手动触发 | 通过 UI/API 手动启动 | §3.1 |
| 时区 | cron 解析所用时区 | §3.1 |
| HMAC 签名 | Webhook 鉴权签名 | §3.4 [NF-SEC]【必須】 |
| 防重放 | 时间戳 ± 5min 窗口校验 | §3.4 [NF-SEC]【必須】 |
| 配额检查 | 触发时检查租户配额 | §3.2 [NF-PER]【必須】 |
| 触发去重 | 极短时间内多次触发只执行一次 | §3.2 |
| 租户隔离 | 调度器按租户独立 | §3.3 [NF-SEC]【必須】 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. IETF「RFC 6234 — US Secure Hash Algorithms」
4. tokio-cron-scheduler 公式ドキュメント
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
