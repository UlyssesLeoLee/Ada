# M-02 标准化转换（Normalizer）

> **ドキュメントID**：DOC-MOD-002
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §2）
> **関連文書**：`docs/modules/M-01`（DOC-MOD-001）
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

- **F-03** 标准化 JSON 转换

### 1.2 关联用例

U-01 跨平台内容同步、U-02 多源数据聚合看板、U-05 可视化调试、U-06 IM 消息双向联动、U-07 项目管理数据同步

### 1.3 数据要件

- NJSON Schema（8.1 节）——本模块是 NJSON 的产出者
- 7.5 安全：敏感字段脱敏（掩码/哈希）能力

### 1.4 接口要件

I-03 Runtime ↔ 外部输出目标：标准化后的数据通过此接口被下游消费

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/00-anatomy-model.md §3](../architecture/00-anatomy-model.md) 中的"骨骼层"，与 [M-01 采集适配器](../modules/M-01-acquisition-adapter.md) 协作：M-01 负责"取到数据 + 初步 NJSON 骨架"，M-02 负责"按用户配置精细化字段映射、脱敏"。

### 2.2 设计原则

- **数据规范化优先**：所有平台数据进入画布前必须转换为统一 schema 的 JSON，下游节点无感知差异
- **表达式受控**：自定义表达式禁止 I/O 操作（详见 §3.2），防止用户自定义逻辑产生副作用

### 2.3 数据流转位置

数据流转生命周期中第 2 步（requirements §8.2）：

```
采集（Playwright DOM / API）
  → 标准化转换（F-03，本模块）★
  → 数据流引擎入血管（F-04，[M-03](../modules/M-03-data-flow-engine.md)）
  → 编排引擎决策（F-05，[M-04](../modules/M-04-orchestration-engine.md)）
  → 控制流调度执行下游节点（F-06，[M-05](../modules/M-05-control-flow-executor.md)）
  → 输出/二次分发（F-14，[M-09](../modules/M-09-exporter.md)）
```

## 3. 详细设计（詳細設計書）

### 3.1 字段映射引擎

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

### 3.2 表达式引擎设计（F-03-03）

采用嵌入式表达式语言（推荐基于 `rhai` 或自研简化 DSL），语法限制在纯函数式子集内，禁止 I/O 操作，防止用户自定义表达式产生副作用或安全风险：

```
表达式示例：
  upper(payload.fields.author) + "_" + str(payload.fields.timestamp)

沙箱限制：
  - 执行超时：单次表达式求值 ≤ 50ms（超时则该字段置为 null 并记录警告）
  - 内存限制：表达式引擎堆内存 ≤ 8MB
  - 禁止网络/文件系统访问
```

### 3.3 转换流水线执行时序

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

### 3.4 可视化配置工具

- **F-03-02** 可视化字段映射工具：将平台原始字段拖拽映射至标准化字段（由 [M-12 前端画布编辑器](../modules/M-12-canvas-editor-frontend.md) 承载 UI）
- **F-03-03** 自定义转换函数：见 §3.2 表达式引擎

## 4. 验收要点

1. **NJSON 合规性**：所有平台的输出数据均符合 requirements §8.1 NJSON Schema 必填字段（`schema_version` / `source` / `captured_at` / `payload`）。
2. **字段映射正确性**：覆盖 Identity / 大小写转换 / 日期格式化 / 正则替换 / 表达式 / 脱敏 6 类 TransformFn 的单元测试。
3. **表达式沙箱安全性**：
   - 表达式无法进行网络/文件系统访问（黑盒测试验证）
   - 单次求值超时（>50ms）后字段置 null 并记录 warning，不阻塞整体流程
4. **Schema 校验**：input_schema / output_schema 校验失败时路由至异常分支，符合 [M-04 编排引擎 §7.3](../modules/M-04-orchestration-engine.md) 异常捕获与重试策略。
5. **脱敏正确性**：FullMask / PartialMask / Hash 三种 MaskType 在测试样本上输出符合预期。

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| NJSON | 标准化 JSON 数据包 | §1.3、DOC-REQ-001 §8 |
| 字段映射 | 平台原始字段 → 标准化字段的转换规则 | §3.1 |
| 表达式引擎 | 用户自定义转换函数的执行环境（rhai 等） | §3.2 |
| 沙箱限制 | 表达式引擎的安全执行边界 | §3.2 [NF-SEC]【必須】 |
| 脱敏 | 敏感字段的掩码/哈希处理 | §3.1 [NF-SEC]【必須】 |
| Schema 校验 | JsonSchema 输入/输出校验 | §3.3 |
| TransformFn | 6 类字段变换函数枚举 | §3.1 |
| MaskType | FullMask / PartialMask / Hash | §3.1 |
| 字段映射引擎 | 字段转换的核心执行器 | §3.1 |
| 转换流水线 | 输入→校验→映射→脱敏→输出 | §3.3 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. rhai 公式ドキュメント「rhai — Embedded scripting for Rust」
4. JSON Schema 仕様 (Draft 2020-12)
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 要件定義書 v1.2.1」、2026-08-18（[DOC-REQ-001](../legacy/requirements.md)）
6. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
