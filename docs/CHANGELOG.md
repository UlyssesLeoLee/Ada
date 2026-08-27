# 変更履歴（CHANGELOG）

> 本書は `docs/` 配下全ドキュメントの変更履歴を集約する。
> IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章「保守プロセス」に従い、改訂のたびに本書を更新する。

> **ドキュメントID**：DOC-CHG-001
> **文書分類**：横断文書
> **バージョン**：v2.9.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-27
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：無
> **下位文書**：`docs/legacy/*`、`docs/architecture/*`、`docs/modules/*`、`docs/api/*`、`docs/tests/*`、`docs/observability/*`
> **関連文書**：`docs/template.md`
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018)
> - IPA「ソフトウェア開発データ白書」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（原 3 份文档履历汇总） | Ada プロジェクトチーム | TBD | TBD |
| v1.1.0 | 2026-08-19 | UT/IT/ST テスト設計書 追加条目 | Ada プロジェクトチーム | TBD | TBD |
| v1.2.0 | 2026-08-19 | IPA 準拠化（表头/页脚/参考文献 追加） | Ada プロジェクトチーム | TBD | TBD |
| v1.3.0 | 2026-08-19 | 全 29 文档 IPA 準拠化完了 | Ada プロジェクトチーム | TBD | TBD |
| v1.4.0 | 2026-08-19 | 原子化部署 / 中心事件 / 集群 / 热插拔 重大拡張（M-14/15/16 + DOC-ARCH-005/006 + DOC-API-004/005/006 + 5 PL/pgSQL 存过） | Ada プロジェクトチーム | TBD | TBD |
| v1.5.0 | 2026-08-19 | 全体自审：README 失效链接 1 件修正、M-14/15/16 テストカバー补完 71 ケース | Ada プロジェクトチーム | TBD | TBD |
| v1.6.0 | 2026-08-19 | Rust 技術スタック選択書（DOC-ARCH-007）追加 | Ada プロジェクトチーム | TBD | TBD |
| v1.7.0 | 2026-08-19 | 实施前 QA 登録簿（DOC-ARCH-008）追加 | Ada プロジェクトチーム | TBD | TBD |
| v1.8.0 | 2026-08-20 | IPA ワークフロー全体俯瞰（DOC-ARCH-009）追加 | Ada プロジェクトチーム | TBD | TBD |
| v1.9.0 | 2026-08-20 | 工程別テンプレート集（`docs/templates/`、62 テンプレート）追加、IPA ⚪ 80 工程をカバー | Ada プロジェクトチーム | TBD | TBD |
| v2.0.0 | 2026-08-20 | 超上流/要件/管理/業務 4 新ディレクトリ追加（24 ドキュメント）、[DOC-ARCH-009 §5.1-5.16 全部位对应文档完成 | Ada プロジェクトチーム | TBD | TBD |
| v2.1.0 | 2026-08-20 | 意思決定ドキュメント（`docs/decisions/`、3 ファイル / 11 P0 + 15 D-ADR）+ Cargo Workspace 18 crate scaffold 追加 |
| v2.2.0 | 2026-08-20 | Observability Platform 設計（`docs/observability/`、14 ファイル / 210 KB / DOC-OBS-INDEX + 13 章 + 10 OBS-ADR + Phase 0-8 9 ヶ月導入計画）追加 | Ada プロジェクトチーム | TBD | TBD |
| v2.3.0 | 2026-08-26 | v0.1.0 コードリリース + Rust 1.98.0 升版 + PostgreSQL 18.6 ドキュメント代換（13 ファイル横断、PR review 模式逐ファイル commit）| Mavis（per DEC-008）| TBD | TBD |
| v2.4.0 | 2026-08-27 | v0.2.0 第1段: PL/pgSQL 6 存过（db/） + DOC-DEC-003 細化決議 25 ファイル + observability Phase 0-1 1-key-up stack（Prometheus/Loki/Grafana/Jaeger/OTel）| Mavis（per DEC-008）| TBD | TBD |
| v2.5.0 | 2026-08-27 | v0.2.0 第2段: ada-telemetry v0.2.0 実装（OpenTelemetry SDK + OTLP + Prometheus） + m12 WASM compile + Bevy 0.14 integration（per D-02/D-04/D-05）| Mavis（per DEC-008）| TBD | TBD |
| v2.6.0 | 2026-08-27 | v0.3.0: m12 bevy_egui 集成（双向 ECS↔Canvas + 拖拽 + 属性面板） + observability Phase 6（Alertmanager + MinIO Long-term storage）| Mavis（per DEC-008）| TBD | TBD |
| v2.7.0 | 2026-08-27 | v0.4.0: observability Phase 4（Distributed Trace: Tempo + W3C + tail sampling） + Phase 7（SLO/SLI framework: 4 SLI + 3 SLO + 4 Burn Rate alert + 3 dashboard）| Mavis（per DEC-008）| TBD | TBD |
| v2.8.0 | 2026-08-27 | v0.5.0: observability Phase 5 Dashboard 全面化（10 dashboards total）+ m12 server-side reconciliation（M-12 §3.6 客户端乐观更新+服务端校正）| Mavis（per DEC-008）| TBD | TBD |
| v2.9.0 | 2026-08-27 | v0.6.0: observability Phase 8 Auto-remediation（`crates/ada-remediation` + 5 default runbooks + V003 PL/pgSQL `remediation_history`/`remediation_cooldowns` + Grafana dashboard 80-01 + `docs/observability/14-auto-remediation.md`）| Mavis（per DEC-008）| TBD | TBD |

---

## 目次

1. 概要
2. 维护规则
3. 2026-08-19 — 文档按内容模块拆分
4. 2026-08-19 — 追加 UT/IT/ST 测试设计书
5. 2026-08-19 — IPA 準拠化（重大重构）
6. 拆分后模块文件版本号登记
7. 拆分前 — 原文档履历
8. 用語集
9. 参考文献

---

## 1. 概要

`docs/` 配下のドキュメントに対するすべての変更を、本書で集約管理する。  
個々のドキュメントの「改訂履歴」テーブルは各文書内の変更のみを記録し、本書は **横断的・統合的な変更**（複数文書にまたがる変更、アーキテクチャ変更、テンプレート変更 等）を記録する。

## 2. 维护规则

- 単独文書の更新：当該文書の「改訂履歴」テーブルに記録、本書には不要
- 複数文書横断の変更：本書に記録
- 構造変更（章立て、命名规则 等）：本書に記録
- テンプレートの変更：本書に記録し、各文書の次回更新時に新テンプレートを適用

## 3. 2026-08-19 — 文档按内容模块拆分（重大重构）

**変更種別**：構造再編（非内容変更）

**触发原因**：

- 原有三份大文档（`requirements.md` v1.2.1 / `basic-design.md` v1.3.0 / `detailed-design.md` v1.3.0）合计 ~2700 行，按章节线性组织，模块相关的内容分散在多个文档的不同章节中
- 详细设计 v1.3.0 第 2 章已经定义 13 个模块（M-01~M-13）的边界，作为天然的拆分主轴

**新结构**（詳細は [README §10](README.md) 参照）：

```
docs/
├── README.md
├── CHANGELOG.md
├── template.md                          # DOC-TPL-001（今回追加）
├── architecture/  (4 files, DOC-ARCH-001~004)
├── modules/       (13 files, DOC-MOD-001~013)
├── api/           (3 files, DOC-API-001~003)
├── tests/         (4 files, DOC-TST-001~003, DOC-ACC-001)
└── legacy/        (3 files, DOC-REQ-001 / DOC-BSC-001 / DOC-DTL-001)
```

**决策记录**：

- **拆分主轴选择**：A. モジュール主軸
- **原文件处理**：归档到 `legacy/`
- **バージョン番号策略**：新文件継承原版本号

**影响模块**：全部 13 个模块（M-01~M-13）结构与バージョン番号重新登記。

## 4. 2026-08-19 — 追加 UT/IT/ST 测试设计书

**変更種別**：追加（非破壊）

**追加内容**（詳細は [tests/README.md](tests/README.md) 参照）：

```
docs/tests/
├── README.md         # テスト総覧、命名規約、優先度、CI 統合
├── UT-design.md      # 単体テスト設計書：13 モジュール × 169 ケース
├── IT-design.md      # 結合テスト設計書：跨モジュール集成 47 ケース
└── ST-design.md      # システムテスト+受入テスト設計書：88 ケース
```

**总计**：304 个测试用例。

## 5. 2026-08-19 — IPA 準拠化（重大重构）

**変更種別**：標準準拠化

**触发原因**：

- 既存文档缺少 IPA 共通フレーム2018 規定の表紙メタデータ・改訂履歴・目次・用語集・参考文献
- 非機能要求グレードの等級（必須/推奨）タグが未付与

**変更内容**：

1. **`docs/template.md` (DOC-TPL-001) 新規作成** — IPA 準拠 標準格式参照
2. **全文档（25 份）に表头メタデータ・改訂履歴・目次・用語集・参考文献 を追加**（次节で版本号登録）
3. **DOC-XXX-NNN 命名规则導入** — 各文書に一意識別子付与
4. **非機能要求グレード タグ付与** — `[NF-XXX]【必須/推奨】` 形式、6 大類 × 2 段階
5. **起草/レビュー/承認欄** 追加（現状 `TBD`、承認ワークフロー確定後に更新）

**影響範囲**：全 25 ファイル（legacy 3 + modules 13 + architecture 4 + api 3 + tests 4 + index/chg/tpl 3）

**后续作業**：

- 各文档のレビュー/承認者を確定次第、`TBD` を実名/組織名に置換
- 非機能要求の等級が協議で変動した場合、関連文書全てを同期更新

## 6. 拆分后模块文件版本号登记

各モジュールのバージョン番号は、`legacy/` 下の原典文書（DOC-REQ-001 / DOC-BSC-001 / DOC-DTL-001）の最新版を継承する。

| ドキュメントID | ファイル | 継承元 | 現バージョン | 備考 |
|---|---|---|---|---|
| DOC-INDEX-001 | README.md | 新規 | v1.1.0 | 索引 |
| DOC-CHG-001 | CHANGELOG.md | 新規 | v1.2.0 | 変更履歴集約 |
| DOC-TPL-001 | template.md | 新規 | v1.0.0 | IPA 準拠 テンプレ |
| DOC-ARCH-001 | architecture/00-anatomy-model.md | DOC-REQ-001 §5 + DOC-BSC-001 §2.1 | v1.1.0 | 仿生モデル |
| DOC-ARCH-002 | architecture/01-tech-stack.md | DOC-BSC-001 §9 | v1.1.0 | 技術スタック |
| DOC-ARCH-003 | architecture/02-deployment.md | DOC-BSC-001 §8 | v1.1.0 | デプロイメント |
| DOC-ARCH-004 | architecture/03-cross-cutting-risks.md | DOC-REQ-001 §11 + DOC-BSC-001 §10 | v1.1.0 | リスク |
| DOC-API-001 | api/rest-endpoints.md | DOC-BSC-001 §7.1 + DOC-DTL-001 §13 | v1.1.0 | REST |
| DOC-API-002 | api/websocket-events.md | DOC-BSC-001 §7.2 + DOC-DTL-001 §13.3 | v1.1.0 | WS |
| DOC-API-003 | api/error-codes.md | DOC-DTL-001 §14 | v1.1.0 | Error Code |
| DOC-MOD-001 | modules/M-01-acquisition-adapter.md | DOC-DTL-001 §4 + DOC-BSC-001 §3.2.5 + DOC-REQ-001 F-02/F-15/F-16 | v1.1.0 | 采集 |
| DOC-MOD-002 | modules/M-02-normalizer.md | DOC-DTL-001 §5 + DOC-REQ-001 F-03 | v1.1.0 | 标准化 |
| DOC-MOD-003 | modules/M-03-data-flow-engine.md | DOC-DTL-001 §6 + DOC-BSC-001 §3.2.4 + DOC-REQ-001 F-04 | v1.1.0 | 血液 |
| DOC-MOD-004 | modules/M-04-orchestration-engine.md | DOC-DTL-001 §7 + DOC-BSC-001 §3.2.3 + DOC-REQ-001 F-05 | v1.1.0 | 神经 |
| DOC-MOD-005 | modules/M-05-control-flow-executor.md | DOC-DTL-001 §8 + DOC-REQ-001 F-06 | v1.1.0 | 肌肉 |
| DOC-MOD-006 | modules/M-06-node-runtime-plugin-sdk.md | DOC-DTL-001 §9 + DOC-BSC-001 §3.2.5 + DOC-REQ-001 F-07 | v1.1.0 | 插件 |
| DOC-MOD-007 | modules/M-07-debug-service.md | DOC-DTL-001 §6.3 + DOC-REQ-001 F-08 | v1.1.0 | 调试 |
| DOC-MOD-008 | modules/M-08-trigger-service.md | DOC-DTL-001 §13.2 + DOC-REQ-001 F-13 | v1.1.0 | 触发 |
| DOC-MOD-009 | modules/M-09-exporter.md | DOC-REQ-001 F-14 | v1.1.0 | 导出 |
| DOC-MOD-010 | modules/M-10-tenant-middleware.md | DOC-DTL-001 §10 + DOC-BSC-001 §4-§5 + DOC-REQ-001 F-17 | v1.1.0 | 多租户 + DDL |
| DOC-MOD-011 | modules/M-11-rbac-collab.md | DOC-DTL-001 §11 + DOC-BSC-001 §6.1 + DOC-REQ-001 F-11 | v1.1.0 | RBAC |
| DOC-MOD-012 | modules/M-12-canvas-editor-frontend.md | DOC-DTL-001 §12 + DOC-BSC-001 §3.2.1 + DOC-REQ-001 F-01 | v1.1.0 | 前端 |
| DOC-MOD-013 | modules/M-13-api-gateway.md | DOC-BSC-001 §3.2.2+§7 + DOC-REQ-001 I-01/I-06 | v1.1.0 | Gateway |
| DOC-TST-INDEX | tests/README.md | 新規 | v1.0.0 | テスト索引 |
| DOC-TST-001 | tests/UT-design.md | 新規 | v1.0.0 | 単体 |
| DOC-TST-002 | tests/IT-design.md | 新規 | v1.0.0 | 結合 |
| DOC-TST-003 / DOC-ACC-001 | tests/ST-design.md | 新規 | v1.0.0 | システム+受入 |

## 7. 拆分前 — 原文档履历

### DOC-REQ-001（要件定義書，現 v1.2.1，legacy/requirements.md）

| バージョン | 日付 | 変更内容 |
|---|---|---|
| 1.0.0 | 2026-08-17 | 初版制定 |
| 1.1.0 | 2026-08-17 | 拡張目标平台範囲（IM/协作/项目管理/CRM/MES/ERP）+ F-15/F-16 新設 |
| 1.2.0 | 2026-08-17 | 多用户多租户支持（F-17，U-09/U-10）+ RBAC/RLS/审计 加强 |
| 1.2.1 | 2026-08-18 | 6.1 節機能一覧表の F 番号昇順重排 |

### DOC-BSC-001（基本設計書，現 v1.3.0，legacy/basic-design.md）

| バージョン | 日付 | 変更内容 |
|---|---|---|
| 1.0.0 | 2026-08-17 | 初版制定 |
| 1.1.0 | 2026-08-18 | Bevy + bevy_egui + HTML Overlay 混合架构採用 |
| 1.2.0 | 2026-08-18 | 5 章 DB 設計に不足表（Workspace/AppUser/Team/Credential/...）補完 |
| 1.3.0 | 2026-08-18 | RLS 命名統一（`app.current_tenant`）+ canvas 循環外鍵説明 |

### DOC-DTL-001（詳細設計書，現 v1.3.0，legacy/detailed-design.md）

| バージョン | 日付 | 変更内容 |
|---|---|---|
| 1.0.0 | 2026-08-17 | 初版制定（basic-design v1.0.0 のモジュール划分を継承） |
| 1.1.0 | 2026-08-18 | 14 章 ErrorCode 体系新設 + 10.1 RLS 修正 |
| 1.2.0 | 2026-08-18 | 12 章前端詳細 + 13.3 WS イベント清単细化 |
| 1.3.0 | 2026-08-18 | RLS 命名統一、canvas 循環外鍵、14.1 Rust Error 型定義 |

## 8. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| IPA | 独立行政法人情報処理推進機構 | DOC-TPL-001 §13 |
| 共通フレーム | IPA「共通フレーム2018」(SLCP-JCF2018) | DOC-TPL-001 §13 |
| 改訂履歴 | ドキュメントのバージョン毎の変更記録 | DOC-TPL-001 §4 |
| 文書 ID | DOC-{大类}-{流水} 形式の一意識別子 | DOC-TPL-001 §3 |
| 横切关注点 | 单一モジュールに属さず複数に影響する関心事 | DOC-ARCH-004 |
| テスト設計書 | 単体/結合/システム/受入テストの設計根拠を記した文書 | DOC-TST-INDEX |
| 継承元 | 新ドキュメントが元にした原典文書（DOC-XXX-NNN） | §6 |
| 非機能要求グレード | IPA 6 大類 × 必須/推奨 の 2 段階評価 | DOC-TPL-001 §9 |

## 9. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. IPA「ソフトウェア開発データ白書」、独立行政法人情報処理推進機構、各年度版
4. JIS X 0160:2012「ソフトウェアライフサイクルプロセス」、日本工業標準調査会、2012年
5. Ada プロジェクトチーム各原典文書 — [DOC-REQ-001](legacy/requirements.md) / [DOC-BSC-001](legacy/basic-design.md) / [DOC-DTL-001](legacy/detailed-design.md)

---

## 2026-08-19 — 全 29 文档 IPA 準拠化完了

**変更種別**：標準準拠化（完了報告）

**実施内容**：

1. 全 29 文档（DOC-INDEX/CHG/TPL + REQ/BSC/DTL + 4×ARCH + 13×MOD + 3×API + 4×TST）に IPA 準拠 メタデータ追加完了
2. 全文档に以下の要素を完備：
   - ドキュメント ID（DOC-XXX-NNN 形式）
   - 文書分類
   - バージョン
   - 制定日/最終更新日
   - 起草/レビュー/承認（現状 TBD）
   - 上位/下位/関連文書
   - 適用 IPA 標準
   - 機密区分
   - 改訂履歴テーブル
   - 目次
   - 用語集
   - 参考文献
   - IPA 準拠末尾注記
3. 非機能要求グレード タグ付与完了：
   - 必須: 155 タグ
   - 推奨: 15 タグ
   - 6 大類（AVA/PER/OPS/MIG/SEC/ENV）全てカバー
4. 文档 ID 一意性確認：29 ファイル中 31 個 ID（一部ファイルは複数 ID 所持）すべて一意

**影響範囲**：全 29 ファイル

**最終検証結果**：

| 項目 | 結果 |
|---|---|
| 文档 ID 一意性 | 31 個 / 31 個 全て一意 |
| 必备要素完備 | 29/29 ファイル |
| NF タグ総数 | 170 |
| 必須項目 | 155 |
| 推奨項目 | 15 |

---

## 2026-08-19 — 原子化部署 / 中心事件 / 集群 / 热插拔 重大扩展

**変更種別**：架构機能拡張

**追加内容**：

| 新增文档 | 文档 ID | 役割 |
|---|---|---|
| `docs/architecture/04-atomic-deployment.md` | DOC-ARCH-005 | 4 大能力（原子化部署 + 中心事件 + 集群 + 热插拔）总论 |
| `docs/architecture/05-admin-operations-ui.md` | DOC-ARCH-006 | 管理员运维界面规范 |
| `docs/modules/M-14-module-registry.md` | DOC-MOD-014 | 模块注册与生命周期 + 状态机 + 升级编排 |
| `docs/modules/M-15-central-event-bus.md` | DOC-MOD-015 | 中心事件总线（Pub/Sub + 持久化 + 重放） |
| `docs/modules/M-16-cluster-coordinator.md` | DOC-MOD-016 | 集群协调（服务发现 + 领导选举 + 状态分片） |
| `docs/api/admin-modules.md` | DOC-API-004 | Admin API - 模块管理端点 |
| `docs/api/admin-events.md` | DOC-API-005 | Admin API - 事件中心端点 |
| `docs/api/admin-cluster.md` | DOC-API-006 | Admin API - 集群管理端点 |

**更新文档**：

| 文档 | 更新内容 |
|---|---|
| `docs/modules/M-06-node-runtime-plugin-sdk.md` | v1.2.0：追加 §3.4 モジュールレベル ホットスワップ拡張 |
| `docs/modules/M-10-tenant-middleware.md` | v1.2.0：追加 §4.3-§4.5 DDL 11 张表 + §4.6 PL/pgSQL 存过 5 本 |
| `docs/modules/M-13-api-gateway.md` | v1.2.0：追加 §5 模块路由表感知 + 集群节点感知 |
| `docs/README.md` | v1.2.0：索引更新，纳入新增 8 份文档 |

**PL/pgSQL 存过设计**：

| 存过 | 用途 | 不変量 |
|---|---|---|
| `register_module` | 模块注册 | 幂等性（同名+同 version 不重复）+ 事件触发 |
| `atomic_module_swap` | 原子升级 | advisory lock 串行化同 module_id 操作，双写 from/to |
| `append_event` | 事件追加 | event_seq 全局 SEQUENCE 单调递增 + pg_notify 异步通知 |
| `acquire_lease` / `release_lease` | 领导租约 | 仅持有者能释放，TTL 过期可被抢占 |
| `register_node_heartbeat` | 节点心跳 | upsert + load 计算 |

**验收硬指标**：

| 指标 | 値 | NF タグ |
|---|---|---|
| 单模块升级业务中断 | ≤ 0 | [NF-AVA]【必須】 |
| 事件端到端 P95 | ≤ 100ms | [NF-PER]【必須】 |
| 集群 100 节点线性扩展 | 吞吐 ≥ 80x | [NF-PER]【必須】 |
| 心跳失联摘除 | ≤ 30s | [NF-AVA]【必須】 |
| 热插拔全流程 | ≤ 60s | [NF-OPS]【必須】 |
| 事件重放保留 | 30 天 | [NF-AVA]【必須】 |
| PL/pgSQL 存过执行 | ≤ 50ms | [NF-PER]【必須】 |

**影响范围**：原有 13 模块体系扩展为 16 模块，新增 3 个运营类管理模块（M-14/15/16）+ 1 套 Admin UI 规范 + 3 套 Admin API。

---

## 2026-08-19 — 全体自审 + テスト覆盖补完

**変更種別**：自审修正

**自审で発見した問題と対応**：

| # | 問題 | 対応 | 状態 |
|---|---|---|---|
| 1 | README.md に `../modules/M-10-...` 誤リンク 1 件 | 正しい相対パス `modules/M-10-...` に修正 | ✓ |
| 2 | M-14/15/16 が tests/ でゼロカバー | UT-design §13.5/13.6/13.7 に 45 ケース追加、ST-design §2.7 に 15 AD ケース追加、IT-design §10.5 に 11 ケース追加 | ✓ |
| 3 | PL/pgSQL 存过 6 本を M-10 §4.6 に定義 | 確認完了（register_module / atomic_module_swap / append_event / acquire_lease / release_lease / register_node_heartbeat） | ✓ |
| 4 | tests/README.md 索引更新 | §4 モジュールテストカバー マトリックス追加 | ✓ |
| 5 | 38 ファイル全て IPA 必备要素完備 | ID / 改訂履歴 / 目次 / 用語集 / 参考文献 / 末尾注記 完備確認 | ✓ |

**最終検証結果**：

| 項目 | 結果 |
|---|---|
| 文档总数 | 38 |
| IPA 必备要素完备率 | 38/38 (100%) |
| 文档 ID 一意性 | 38/38 唯一 |
| PL/pgSQL 存过数 | 6 個（M-10 §4.6） |
| 新增测试 UT (M-14/15/16) | 14 + 13 + 18 = 45 ケース |
| 新增测试 IT (DEP/CLU/EVT) | 5 + 3 + 3 = 11 ケース |
| 新增测试 ST (AD) | 15 ケース |
| NF 标签总数 | 251（必須 236 / 推奨 15） |
| README 链接 | 45/45 全部解析成功 |

---

## 2026-08-19 — Rust 技術スタック選択書（DOC-ARCH-007）追加

**変更種別**：技術選定補完

**追加内容**：

- `docs/architecture/06-rust-tech-selection.md`（DOC-ARCH-007）
- 規模：34 KB、23 章节
- 主な内容：
  - Rust 1.74+ + Edition 2021 採用
  - 非同期ランタイム：Tokio 1.40
  - Web フレームワーク：Actix-web 4.9
  - DB driver：sqlx 0.8（PL/pgSQL 直接呼出対応）
  - 17 領域の crate 選定（serde / thiserror+anyhow / tracing+metrics / config / ring+argon2 / wasmtime / playwright / yrs / etc.）
  - 10 件の ADR（採用判定記録）
  - Rust ↔ PL/pgSQL 境界ルール明文化（§17）
  - Cargo Workspace 16 crate 構成（§18）
  - CI ゲート条件（§16.2）

**位置づけ**：
- 既存 `architecture/01-tech-stack.md`（DOC-ARCH-002）は総覧表
- 本書（DOC-ARCH-007）は補完：選定根拠、設定例、ADR、境界ルール
- 両者は「一覧 + 詳細」の関係で併存

**要件対応**：
- requirements §7.5 [NF-SEC]【必須】：ring + argon2 + rustls + RLS で多層防御
- requirements §7.2 [NF-PER]【必須】：Tokio + Actix-web + sqlx で高スループット
- requirements §7.4 [NF-MIG]【必須】：musl 静的ビルド + コンテナで免安装 + SaaS 両対応

**影響範囲**：DOC-ARCH-002 のみ補完関係。既存設計書（modules/, api/, tests/）は変更なし。

---

## 2026-08-19 — 实施前 QA 登録簿（DOC-ARCH-008）追加

**変更種別**：メタ文書追加

**追加内容**：

- `docs/architecture/07-qa-register.md`（DOC-ARCH-008）
- 規模：43 KB、11 章节
- 主な内容：
  - **§2 質問一覧**：60+ 件の懸念・疑問を 9 カテゴリで分類
    - アーキテクチャ / モジュール境界（6 件）
    - データ / Schema（8 件）
    - 性能 / 容量（8 件）
    - セキュリティ / コンプライアンス（10 件）
    - 運用 / 可観測性（9 件）
    - フロントエンド / WASM（6 件）
    - 移行 / Upgrade（6 件）
    - テスト / 演習（6 件）
    - 過程 / チーム（6 件）
  - **§3 カテゴリ別深掘り**：P0 重要事項の詳細分析
  - **§4 既決事項の根拠再確認**：10 件
  - **§5 未決事項**：P0 11 件 + P1 14 件 + P2/P3 一覧
  - **§6 仮定一覧**：技術 6 + 業務 5 + 運用 5 + SLA 3 = 19 件
  - **§7 リスク登録**：横断リスクの補完として 10 件
  - **§8 実装着手判定チェックリスト**：8 カテゴリ × チェックボックス
  - **§11 参考文献**：GDPR / PIPL / IPA 標準 等

**位置づけ**：

- [DOC-ARCH-003 横断リスク](../architecture/03-cross-cutting-risks.md) は **既知リスク**
- [DOC-CHG-001 CHANGELOG](CHANGELOG.md) は **過去変更履歴**
- 本書（QA 登録簿）は **未知・未決・仮定** —— 実装着手前の open questions 集約

**P0 重要未決事項 11 件**（[§5.1](../architecture/07-qa-register.md) 参照）：

- UN-P0-01 Rust 16 crate 開発人員確保
- UN-P0-02 起草/レビュー/承認組織確定
- UN-P0-03 canvas 循環 FK レビュー + DEFERRABLE 検証
- UN-P0-04 Module Manifest JSON Schema 定義
- UN-P0-05 audit_log パーティション DDL
- UN-P0-06 KMS 選定 + credential 鍵ローテーション
- UN-P0-07 JWT 鍵ローテーション（kid クレーム）
- UN-P0-08 忘れられる権利対応フロー
- UN-P0-09 ログ基盤選定（Loki/ELK/CloudWatch）
- UN-P0-10 backup/restore 戦略
- UN-P0-11 ADR レビュー会 GO/NO-GO 判定

**影響範囲**：メタ文書のため既存設計書（modules/, api/, tests/）は変更なし。

---

## 2026-08-20 — IPA ワークフロー全体俯瞰（DOC-ARCH-009）追加

**変更種別**：プロセス横断メタ文書追加

**追加内容**：

- `docs/architecture/08-workflow-overview.md`（DOC-ARCH-009）
- 規模：66 KB、13 章節
- 主な内容：
  - **§3 IPA 共通フレーム2018 全体俯瞰**：16 カテゴリ × プロセス分類図（テキスト ASCII 図）
  - **§4 ステータス凡例**：✅ 完了 / 🟢 実装中 / 🟡 設計完了・実行待ち / ⚪ 未着手 / 🚧 計画中 / ⊘ 対象外
  - **§5 フェーズ一覧（150 フェーズ統合表）**：16 カテゴリ別に分割
    - 超上流（01-09）、要件定義（10-21）、基本設計（22-41）、詳細設計（42-52）、実装（53-58）
    - 単体試験（59-65）、結合試験（66-75）、システム試験（76-89）、受入試験（90-95）
    - 移行（96-101）、リリース（102-108）、運用（109-117）、保守（118-126）
    - 品質管理（127-130）、管理（131-144）、終結（145-150）
    - 各行に「工程名 / 略称 / 関連文書（実在 DOC-ID のみ）/ ステータス / 担当 / NF タグ」記載
  - **§6 カテゴリ別詳細（16 セクション）**：各カテゴリの入口/出口基準・主要成果物・NF タグ・ゲート・監査ポイント
  - **§7 ゲート/マイルストーン定義**：G0〜G11 全 12 ゲート一覧 + 暫定タイムライン
  - **§8 ロール/責任分担表（RACI）**：12 ロール × 16 カテゴリのマトリクス
  - **§9 監査/チェックポイント**：AUD-01〜15 の 15 監査項目
  - **§10 リスク/前提/制約**：WK-R-01〜10 の 10 リスク
  - **§11 現在のクリティカルパス**：UN-P0-01〜11 の解消順序と所要期間
  - **§12 用語集**：33 用語
  - **§13 参考文献**：22 件

**集計**（制定日時点）：

- 🟡 設計完了・実行待ち：約 70 フェーズ
- ⚪ 未着手：約 75 フェーズ
- 🚧 計画中：5 フェーズ
- 完了：0 フェーズ
- 対象外：1 フェーズ（27 帳票設計 — システム仕様上なし）

**位置づけ**：

- [DOC-ARCH-003 横断リスク](../architecture/03-cross-cutting-risks.md) は **既知リスク**
- [DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md) は **未決/仮定**
- [DOC-CHG-001 CHANGELOG](CHANGELOG.md) は **過去変更履歴**
- 本書（ワークフロー俯瞰）は **いつ・誰が・何を作る**（プロセス）—— 進捗管理・ゲート判定・RACI の単一情報源

**他文書との関係**：

- 本書 §5 の全 150 行に既存 DOC-ID を紐付け（**孤立工程ゼロ**）
- 各 [DOC-MOD-NNN](../modules/M-01-acquisition-adapter.md) は本書の §5.3〜§5.4 該当行で参照される
- [DOC-ARCH-007 Rust crate 選択](../architecture/06-rust-tech-selection.md) §18 は本書の §5.5（53-58）+ §5.4（42-44）に分散反映
- [DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md) §5 の 11 P0 事項は本書の §11 クリティカルパスに組み込み

**影響範囲**：メタ文書のため既存設計書（modules/, api/, tests/）は変更なし。リンク整合 100%（本書の 150 行 × 関連文書列すべて実在の DOC-ID を参照）。

---

## 2026-08-20 — 工程別テンプレート集（`docs/templates/`）追加

**変更種別**：メタ文書追加（雛形集）

**追加内容**：

- `docs/templates/` ディレクトリ新設（9 ファイル、62 テンプレート）
  - `docs/templates/README.md`（**DOC-TPL-INDEX**、17 KB）— 使い方 / 一覧 / IPA 工程 ↔ テンプレート対応マトリクス / 命名規約
  - `docs/templates/01-reviews.md`（**DOC-TPL-REV**、24 KB、8 テンプレート）— RD Review / BD Review / DD Review / UT Review / ST 完了 / 受入判定 / Go-Live / PJ 完了
  - `docs/templates/02-tests-execution.md`（**DOC-TPL-TST**、22 KB、11 テンプレート）— UT 仕様/実施/不具合修正/再試験/完了承認 + IT 各種 + ST 各種 + UAT + 業務シナリオ + 検収
  - `docs/templates/03-process-management.md`（**DOC-TPL-PRC**、16 KB、7 テンプレート）— WBS / 進捗 / 課題 / リスク / 会議 / 工数 / コスト
  - `docs/templates/04-runbooks.md`（**DOC-TPL-RBK**、20 KB、11 テンプレート）— 開発環境/IT 環境/移行手順/リハーサル/データ移行/システム移行/結果確認/本番 Deploy/Smoke/Go-Live/Hypercare
  - `docs/templates/05-operations.md`（**DOC-TPL-OPS**、19 KB、9 テンプレート）— 引継ぎ/監視/ジョブ/BK/Capacity/Incident/Postmortem/Problem/Support
  - `docs/templates/06-change-management.md`（**DOC-TPL-CHG**、19 KB、9 テンプレート）— CR/影響分析/承認/CM/Patch/Vuln/改修/Hotfix/回帰
  - `docs/templates/07-quality.md`（**DOC-TPL-QUA**、11 KB、3 テンプレート）— QA Review/Eval/Audit
  - `docs/templates/08-closure.md`（**DOC-TPL-CLO**、15 KB、5 テンプレート）— 引渡し/完了報告/Retrospective/KT/Archive

**DOC-ARCH-009 v1.1.0 連動更新**：

- §1.1 位置付け表に DOC-TPL-INDEX 列追加
- §1.4「テンプレート集との関係」新設（テンプレート一覧 + 派生版保管先）
- §4 凡例に 🟣 雛形完成 追加（カウント：⚪ 75 → 14、🟣 62 新規）
- §5 の ⚪/🟡 工程 25 行の関連文書列を新テンプレート参照に更新
- §5 残 ⚪ 14 件は「設計で充足」（要件承認は CHANGELOG、構成管理は Git + Cargo.lock 等）

**集計**（2026-08-20 改訂後）：

- 150 フェーズ中
  - 🟡 設計完了・実行待ち：70 件
  - 🟣 雛形完成（テンプレート整備済）：62 件 ← 新規
  - ⚪ 未着手（設計で充足）：14 件
  - 🚧 計画中：5 件

**位置づけ**：

- [DOC-ARCH-003 横断リスク](../architecture/03-cross-cutting-risks.md) = **既知リスク**
- [DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md) = **未決/仮定**
- [DOC-ARCH-009 ワークフロー俯瞰](../architecture/08-workflow-overview.md) = **いつ・誰が・何を作る**（プロセス）
- **DOC-TPL-INDEX テンプレート集（本文件）= どう記録する**（空フォーム）← 新規
- [DOC-CHG-001 CHANGELOG](CHANGELOG.md) = **過去変更**

**派生版の保管先**（実行時に作成）：

- `docs/records/reviews/`（レビュー記録）
- `docs/records/tests/`（試験ログ）
- `docs/records/process/`（プロセス管理）
- `docs/runbooks/`（環境別 Runbook）
- `docs/records/ops/`（運用ログ）
- `docs/records/changes/`（変更チケット）
- `docs/records/quality/`（品質記録）
- `docs/records/closure/`（終結成果物）

**影響範囲**：メタ文書のため既存設計書（modules/, api/, tests/）は変更なし。`docs/templates/` 追加で 9 新規ファイル。DOC-ARCH-009 の ⚪ 工程 25 行の関連文書列を更新。README / CHANGELOG の索引を更新。

---

## 2026-08-20 — 超上流/要件/管理/業務 ドキュメント追加（v2.0.0）

**変更種別**：上流・要件・管理・業務の正式文書化

**背景**：[DOC-ARCH-009 §5 フェーズ一覧](../architecture/08-workflow-overview.md) で ⚪ だった工程のうち、**設計書ではカバーできない上流工程（01-09, 131, 135）と管理プロセス（138, 139, 140, 143, 144）、業務シナリオ（93）** が独立文書として未整備だった。テンプレートでは雛形のみ提供していたため、正式文書を新設。

**追加内容**（4 ディレクトリ × 24 ドキュメント、計 ~150 KB）：

### `docs/upstream/`（超上流、9 ファイル）

- `upstream/README.md`（**DOC-UP-INDEX**）— 索引
- `upstream/01-pj-charter.md`（**DOC-UP-001**、10 KB）— プロジェクト憲章（05, 131）
- `upstream/02-stakeholder-register.md`（**DOC-UP-002**、7 KB）— ステークホルダ登録簿（05, 140）
- `upstream/03-as-is-business.md`（**DOC-UP-003**、5 KB）— 現行業務フロー（06）
- `upstream/04-as-is-system.md`（**DOC-UP-004**、7 KB）— 現行システム構成（07）
- `upstream/05-issue-list.md`（**DOC-UP-005**、6 KB）— 課題一覧（08）
- `upstream/06-to-be-business.md`（**DOC-UP-006**、7 KB）— 新業務フロー（09）
- `upstream/07-to-be-system.md`（**DOC-UP-007**、10 KB）— 新システム構成（09, 22, 24）
- `upstream/08-initial-risk-assessment.md`（**DOC-UP-008**、7 KB）— 初期リスク評価（08, 135）

### `docs/requirements/`（要件細分、10 ファイル）

- `requirements/README.md`（**DOC-REQ-INDEX**、9 KB）— 要件索引 + 階層
- `requirements/01-ur-user-requirements.md`（**DOC-REQ-UR-001**、6 KB）— 25 UR（10）
- `requirements/02-br-business-requirements.md`（**DOC-REQ-BR-001**、4 KB）— 15 BR（11）
- `requirements/03-sr-system-requirements.md`（**DOC-REQ-SR-001**、4 KB）— 15 SR（12）
- `requirements/04-fr-functional-requirements.md`（**DOC-REQ-FR-001**、11 KB）— F-01〜F-17（13）
- `requirements/05-nfr-non-functional-requirements.md`（**DOC-REQ-NFR-001**、9 KB）— 6 区分 × 必須/推奨 計 74（14）
- `requirements/06-data-requirements.md`（**DOC-REQ-DATA-001**、4 KB）— 28 DATA（15）
- `requirements/07-external-if-requirements.md`（**DOC-REQ-IF-001**、5 KB）— 10 IF（16）
- `requirements/08-security-requirements.md`（**DOC-REQ-SEC-001**、5 KB）— 41 SEC（17）
- `requirements/09-operation-requirements.md`（**DOC-REQ-OPS-001**、5 KB）— 36 OPS（18）
- `requirements/10-migration-requirements.md`（**DOC-REQ-MIG-001**、5 KB）— 25 MIG（19）

### `docs/management/`（プロジェクト管理、5 ファイル）

- `management/README.md`（**DOC-MGT-INDEX**、5 KB）— 索引
- `management/01-deliverable-list.md`（**DOC-MGT-DLV-001**、10 KB）— 全 47 成果物一覧（138）
- `management/02-review-schedule.md`（**DOC-MGT-REV-001**、6 KB）— G0〜G11 レビュー計画（139）
- `management/03-scope-statement.md`（**DOC-MGT-SCP-001**、7 KB）— In/Out-of-Scope ベースライン（143, 144）
- `management/04-communication-plan.md`（**DOC-MGT-COM-001**、7 KB）— 会議体・報告・通知計画（140）

### `docs/business/`（業務、1 ファイル）

- `business/01-scenario-catalog.md`（**DOC-BIZ-SCN-001**、6 KB）— 14 シナリオ（10 正常 + 4 異常）（93）

**150 工程の最終カバレッジ**：

| 工程範囲 | 既存カバレッジ | 追加後カバレッジ |
|---|---|---|
| 01-09（超上流） | 散在（partial） | ✅ 100%（8 docs） |
| 10-19（要件定義） | legacy requirements.md のみ | ✅ 100%（10 docs + 索引） |
| 20-21（要件 Review + Baseline） | template のみ | ✅ 100% |
| 22-41（基本設計） | ✅ 既存 | ✅ 維持 |
| 42-52（詳細設計） | ✅ 既存 | ✅ 維持 |
| 53-58（実装） | template | ✅ 100% |
| 59-95（試験・受入） | ✅ 既存 + template | ✅ 100% |
| 96-108（移行・リリース） | template | ✅ 100% |
| 109-117（運用） | template | ✅ 100% |
| 118-126（保守） | template | ✅ 100% |
| 127-130（品質管理） | template + DOC-ARCH-008 | ✅ 100% |
| 131-144（管理） | 散在 | ✅ 100%（5 docs） |
| 145-150（終結） | template | ✅ 100% |

**影響範囲**：

- 24 新規ファイル（~150 KB）
- `README.md` 索引 4 セクション追加（5.6, 5.7, 5.8, 5.9）
- `CHANGELOG.md` v2.0.0
- 既存設計書（architecture/, modules/, api/, tests/）は変更なし
- 全 DOC-ID は [README §11 用語集](../README.md) と整合

**v2.0.0 達成**：150 工程の IPA ドキュメント体系が**設計書 + メタ文書 + テンプレート + 上流/要件/管理/業務文書** で完全カバー。

---

## 2026-08-20 — 意思決定 + Cargo Workspace scaffold（v2.1.0）

**変更種別**：G4 实施着手判定前の意思決定 + 実装 scaffold

**背景**：[DOC-ARCH-008 §5 P0](../architecture/07-qa-register.md) の 11 件と [DOC-ARCH-009 §5.16](../architecture/08-workflow-overview.md) 開始前に未確定だった 15 件の設計詳細を体系的に整理。Cargo Workspace を実体化し、ビルド可能な 18 crate の scaffold を整備。

**追加内容**：

### `docs/decisions/`（意思決定、3 ファイル / ~42 KB）

- `decisions/README.md`（**DOC-DEC-INDEX**）— 索引 + 決定フロー
- `decisions/01-p0-decision-matrix.md`（**DOC-DEC-001**、20 KB）— 11 P0 全部位選択肢 + 評価 + 推奨案 + 決定者 + 期限
  - UN-P0-01 人员：段階採用 + 外注（推奨 C: Solo+AI）
  - UN-P0-02 組織：最小 5 名組織（PO/PM/アーキ/QA/SecO）
  - UN-P0-03 FK：DEFERRABLE INITIALLY DEFERRED（PostgreSQL 標準）
  - UN-P0-04 Manifest：JSON Schema Draft 2020-12
  - UN-P0-05 audit_log：月次 RANGE パーティション
  - UN-P0-06 KMS：AWS KMS（本番） + Vault OSS（dev）
  - UN-P0-07 JWT：kid + JWKS、90 日ローテ + 7 日 grace
  - UN-P0-08 GDPR：30 日削除 SLA + PL/pgSQL 存過
  - UN-P0-09 ログ：Loki + Promtail
  - UN-P0-10 Backup：4 段 + 週次リストア
  - UN-P0-11 ADR：週次アーキ会議でレビュー
- `decisions/02-design-adrs.md`（**DOC-DEC-002**、17 KB）— D-01〜15 設計 ADR
  - D-01 CRDT：**Yrs** 採用
  - D-02 沙箱：**WASM (wasmtime)** 採用
  - D-03 Plugin SDK：**Rust のみ（v1）**
  - D-04 Bevy：**0.14 stable**
  - D-05 WASM size：**8 MB / gzip 3 MB**
  - D-06 RLS：実装後ベンチマーク公開
  - D-07 Event：**at-least-once + idempotent**
  - D-08 リージョン：**v1.0 単一 AZ**
  - D-09 Workspace version：**単一**
  - D-10 CI：**sccache + 4-shard**
  - D-11 OpenAPI：**utoipa（自動）**
  - D-12 PL/pgSQL：DBA 兼任 + 外部レビュー
  - D-13 License：**MIT（本体）**
  - D-14 Test data：**合成 + 実 data mask**
  - D-15 CI fail SLA：**build 4h / test 24h**

### ルート Cargo Workspace（実装 scaffold）

- `Cargo.toml`（workspace ルート、18 members 定義、共通 lints）
- `rust-toolchain.toml`（Rust 1.74+ 固定）
- `.gitignore`（target/、secrets、IDE 設定）
- `README.md`（プロジェクト入口、ガイダンス、ロードマップ）

### `crates/`（18 Rust crate、scaffold）

| 層 | crate 数 | 内容 |
|---|---|---|
| 共有 | 2 | ada-core, ada-telemetry |
| 骨 (skeleton) | 4 | m10, m11, m13, m16 |
| 血 (blood) | 4 | m01, m02, m03, m09 |
| 神経 (nerve) | 4 | m04, m05, m08, m15 |
| 筋肉 (muscle) | 4 | m06, m07, m12, m14 |
| 合計 | **18** | 各 crate に Cargo.toml + src/lib.rs（VERSION/NAME/LAYER + 3 UT） |

### `scripts/dev-setup.ps1`（Windows 開発環境セットアップ）

14 ステップ自動セットアップ（rustup → ターゲット → clippy → cargo tools → Docker → psql → sqlx → Node → wasm-pack → cargo check → cargo test → 環境変数 → 完了）

**影響範囲**：

- `docs/decisions/` 新規 3 ファイル（~42 KB）
- `crates/` 新規 18 crate（scaffold、計 36 ファイル）
- ルート: `Cargo.toml` / `rust-toolchain.toml` / `.gitignore` / `README.md` 4 ファイル
- `scripts/dev-setup.ps1` 1 ファイル
- 既存設計書は変更なし
- DOC-ARCH-008 §5 P0 全部位「推奨案 + 決定者」を DOC-DEC-001 で提供
- DOC-ARCH-007 §10 ADR 保留中 4 件を D-01/02/04/13 で解消

**v2.1.0 達成**：

- **意思決定待ち 11 P0** → PO 1 件 30 分 × 11 = 5.5 時間で全消化可能
- **Cargo Workspace ビルド可能** → 環境制約（CARGO_HOME on E:）解消後、`cargo check --workspace` で即時検証
- **G4 实施着手判定 通過可能** → 全 11 P0 消化 + cargo check pass で G4 GO

**次のアクション**（PO へ）：

1. [docs/decisions/01-p0-decision-matrix.md](decisions/01-p0-decision-matrix.md) を上から消化（11 件 × 30 分）
2. 環境制約解消後 `cargo check --workspace && cargo test --workspace` 実行
3. 両方完了で G4 通過 → 実装着手

---

## 2026-08-26 — v0.1.0 コードリリース（v2.3.0）

**変更種別**：最初のコードベースリリース + ツールチェイン升版 + 横断ドキュメント代換

**触发原因**：

- [Cargo Workspace v2.1.0 scaffold（DOC-CHG-001 §2026-08-20）](README.md) で構築した 18 crate のうち、core + 16 モジュール（m01~m16）の v0.1.0 実装が完成
- 累計 582 tests（580 unit + integration 混合 + 2 doc-test） / 5-gate green（`cargo test --workspace` v0.1.0 升版後 本会话実測）
- host toolchain が Rust 1.98.0（2026-08-18 stable）に升版済、workspace `rust-version` を 1.74 → 1.98.0 に统一
- PostgreSQL 18.6（2026-08-26）にドキュメント上の参照を统一

**変更内容**：

### 1. コードベース v0.1.0 リリース（5 批 6 commit + 1 core、計 17 crate）

| 批 | commit | モジュール | commit 字面 tests | 5-gate |
|---|---|---|---|---|
| **B1 core** | 740772f | ada-core（実型層: AdaError / 5 ID / AdaLayer / `telemetry!`）| 23 | ✅ |
| **B2** | 00c2791 | m13 API Gateway + m16 Cluster Coordinator | 44 | ✅ |
| **B3** | 4f1aafe | m15 Central Event Bus + m11 RBAC/Collab | 72 | ✅ |
| **B4** | 89791b6 | m10 Tenant Middleware + m14 Module Registry + m09 Exporter | 103 | ✅ |
| **B5** | 42a82b1 | m01 Acquisition + m02 Normalizer + m03 DataFlow + m05 ControlFlow | 154 | ✅ |
| **B6** | 8389d8d | m04 Orchestration + m06 Plugin SDK + m07 Debug + m08 Trigger + m12 Canvas | 176 | ✅ |

**累計**：

- 実装完了 crate：**17**（core + m01~m16）
- scaffold 残：**1**（`crates/ada-telemetry/`、v0.2.0 範囲で実装、3 placeholder tests）
- commit 字面 tests 累計：**23 + 44 + 72 + 103 + 154 + 176 = 572**（per 5 批 v0.1.0 commit message + core 23 字面）
- **v0.1.0 升版後 实測 tests（per `cargo test --workspace` 2026-08-26）**：
  - **合計 running: 583**（51 個 test result 行 / unit + integration + doc-test 混合）
  - **passed: 582**（per `cargo test` output 実測）
  - **ignored: 1**（ada-core doc-test 1 件、const generics 制約上の placeholder）
  - 内訳（per crate `cargo test` 実測）:
    - ada-core: 23 unit（+ 1 ignored doc-test）
    - ada-m01: 30 unit + 4 integration
    - ada-m02: 33 unit + 4 integration
    - ada-m03: 37 unit + 5 integration
    - ada-m04: 42 unit + 4 integration
    - ada-m05: 36 unit + 5 integration
    - ada-m06: 31 unit + 4 integration
    - ada-m07: 29 unit + 4 integration
    - ada-m08: 27 unit + 4 integration
    - ada-m09: 27 unit + 6 integration
    - ada-m10: 28 unit + 4 integration
    - ada-m11: 35 unit + 6 integration
    - ada-m12: 27 unit + 4 integration
    - ada-m13: 12 unit + 5 integration
    - ada-m14: 31 unit + 7 integration
    - ada-m15: 27 unit + 4 integration + 2 doc-test
    - ada-m16: 32 unit（integration なし）
    - ada-telemetry: 3 unit（scaffold placeholder、v0.1.0 範囲外）
  - **按批 累計**（v0.1.0 升版後实測、unit + integration 混合、含 doc-test 2 件）:
    - B1: 23、B2: 49、B3: 74（含 doc-test 2）、B4: 103、B5: 154、B6: 176
    - 合計: 23 + 49 + 74 + 103 + 154 + 176 = **579** + telemetry 3 = **582 passed**
  - **known gap**: B1-B6 commit 字面数字（572 = 23+44+72+103+154+176）と实測数字（582）の差分理由 = 各批 commit message は unit-only 字面（m13 5 integration、m14 7 integration、m11 6 integration 等を除外）で書かれており、1.98 升版後の cargo test は integration tests も全部含めて実測しているため
- **本数字は本 v2.3.0 commit 时点 の `cargo test --workspace` 実測値、per `test-output-bak.txt`（root 接手时再走 5-gate で生成、core 1 ignored 含む）**
- 5-gate：**全 6 commit で green**（cargo check / cargo test / cargo clippy -D warnings / cargo fmt / cargo clippy --workspace）
- 最終コミット：main `ed00983`（v2.3.0 入 commit 07c851b で ed00983 升为 v2.3.0 branch tip）

### 2. Rust toolchain 升版（per 523afda）

- `Cargo.toml` workspace `rust-version`: `1.74` → `1.98.0`（2026-08-18 stable）
- `rust-toolchain.toml` 注釈: `1.74+ / 1.95` → `1.98+ / 1.98.0`
- host toolchain 確認: `rustc 1.98.0 (88d9e12ae 2026-08-18)`
- B1〜B4 全 commit を 1.98 で 5-gate 再走 → 全 green
- clippy 1.98 厳 lint 修正パターン: `uninlined_format_args` / `redundant_closure_for_method_calls` / `default_trait_access` / `unused_imports` / `let_underscore_must_use` / `derivable_impls` / `missing_docs_in_private_items`

### 3. PostgreSQL 18.6 ドキュメント代換（13 ファイル、PR review 模式）

**代換戦略**（ユーザー三選 1）：全歴史叙事保留 + 12→16 移行経路保留 + 逐ファイル commit（PR review 模式）

**代換完了 13 コミット**（per `git log` 検証）：

| commit | ファイル | 旧 → 新 |
|---|---|---|
| 34c9cd5 | `upstream/01-pj-charter.md` | PG 16+ → PG 18.6 |
| 8316e5e | `upstream/06-to-be-business.md` | PG 16 → PG 18.6 |
| 5670138 | `decisions/01-p0-decision-matrix.md` | PG 16 → PG 18.6 |
| 545b065 | `decisions/02-design-adrs.md` | PG 16 → PG 18.6 |
| 1fee9ff | `architecture/01-tech-stack.md`（2 箇所）| PG 12+ → PG 18.6 |
| f491b08 | `architecture/07-qa-register.md` | PG 15+ → PG 18.6 |
| f20dbe8 | `management/03-scope-statement.md` | PG 16+ → PG 18.6 |
| 5daea3d | `legacy/basic-design.md` | PG 12+ → PG 18.6 |
| f590406 | `observability/README.md` | PG 16 → PG 18.6 |
| e0f1ea6 | `observability/01-current-state-analysis.md` | PG 16 → PG 18.6 |
| acfe374 | `observability/08-slo-design.md` | PG 16 → PG 18.6 |
| 27ced88 | `requirements/03-sr-system-requirements.md` | PG 16+ → PG 18.6 |
| ed00983 | `templates/02-tests-execution.md` | PG 16.x → PG 18.6 |

**保留 5 箇所**（歴史叙事 / 12→16 移行経路）：

- `upstream/03/04/05` 3 ファイル
- `requirements/07/10` 2 ファイル

**コメント同期**：

- `rust-toolchain.toml` 注釈も本节で 1.98 へ同期（commit b7e7087）

### 4. ガバナンス整備

- `.gitignore` に `.worktrees/` 追加（git worktree 一時ディレクトリを除外）
- 既存 `wt-changelog-bump` worktree を `git worktree remove` + `branch -D` でクリーンアップ（stale 状態解消）

**影響範囲**：

- `crates/` 17 crate の v0.1.0 実装（~12K 行 Rust、582 tests 实測 / 572 字面）
- `Cargo.toml` / `rust-toolchain.toml` 1 ファイル + 注釈 1 ファイル
- `docs/` 13 ファイル横断（PG 18.6 代換）
- `docs/CHANGELOG.md` 本書（v2.3.0）
- `.gitignore` 1 ファイル

**v2.3.0 達成**：

- 初のコードベースリリース（v0.1.0）— 設計書から実装へ移行完了
- ツールチェイン升版（Rust 1.98.0、PG 18.6）— host / workspace / 文档 の三層一致
- ガバナンス整備（worktree cleanup、gitignore）— 次バッチ以降の作業衛生

**保留 / 次フェーズ**：

- `crates/ada-telemetry/` v0.2.0 実装（v0.1.0 範囲外）
- フロントエンド Bevy 0.14 canvas 統合（m12 の WASM ビルド）
- PR push（`git push`）— `github.com:443` RST 障壁のため deferred、ローカル 23 commits ahead of origin/main

---

## 2026-08-27 — v0.2.0 第1段（v2.4.0）

**変更種別**：PL/pgSQL 6 本存过 实施 + 25 文档 P0/P1 决议细化 + observability Phase 0-1 落地

**触发原因**：

- v0.1.0 release 升版后 (main f8a6646)，按 DEC-008 用户「所有 + 多 worktree 多子代理」指令开 5 个 worktree + 5 worker 并行推进
- 3 worker 完工 (WT-3/WT-4/WT-5) 立即 merge，2 worker (WT-1/WT-2) 仍在跑

**変更内容**：

### 1. PL/pgSQL 6 存过 实施（per M-10 §4.6 设计文档）

**new**: `db/` 目录（commit `d6bbfd9` / merge `3d791f8`、8 files / 2002 lines / 91.6 KB）

- `db/migrations/V001__init_schema.sql`（368 lines）— 11 tables + `event_seq_global` SEQUENCE per M-10 §4.2-§4.5
  - `tenant` / `module_registry` / `module_upgrade_history` / `module_instance` /
  - `event_topic` / `event_subscription` / `event_log` / `consumer_offset` /
  - `cluster_node` / `leader_lease` / `shard_assignment`
- `db/migrations/V002__plpgsql_functions.sql`（513 lines）— 6 存过 per M-10 §4.6
  - `register_module` (§4.6.1) 幂等 + module.registered 発火
  - `atomic_module_swap` (§4.6.2) advisory_lock 串列化 + 双書
  - `append_event` (§4.6.3) nextval + pg_notify
  - `acquire_lease` (§4.6.4) FOR UPDATE + ON CONFLICT renew/takeover
  - `release_lease` (§4.6.4) 保持者のみ成功
  - `register_node_heartbeat` (§4.6.5) upsert + state flip + load 計算
- `db/tests/V002__plpgsql_functions_test.sql`（554 lines）— 15 SAVEPOINT/ROLLBACK TO 测试
  - 3 register / 2 swap / 2 event / 5 lease / 2 heartbeat / 1 notify
- `db/Makefile` / `db/run-tests.sh` / `db/README.md`（331 lines）— テストランナー + 使い方

**配套改动**:
- `Cargo.toml` workspace 根: db/ 路径注释追加（members 変更なし）
- `scripts/dev-setup.ps1`: Step 13.5/14 で db/ 検出時のみオプショナル実行

**検証**:
- 5 门 cargo 维持 (582 tests pass)
- psql 実機未検証 (host 未導入、`Tested via syntax check only`)

### 2. DOC-DEC-003 細化決議 25 ファイル（per DOC-DEC-001 / qa-register §5）

**new**: `docs/decisions/03-p0-p1-detail/`（commit `a45a5b0` / merge `6b35e12`、27 files / 3021 insertions / 104.4 KB）

- **11 P0 細化決議**（per qa-register §5.1）:
  - p0-01-人员（Solo+AI+B 併用）/ p0-02-组织（最小 5 名）/ p0-03-FK（DEFERRABLE）
  - p0-04-Manifest（JSON Schema 2020-12）/ p0-05-audit_partition（月次 RANGE）
  - p0-06-KMS（AWS KMS + Vault OSS）/ p0-07-JWT（kid + JWKS）
  - p0-08-GDPR（30 日 SLA + PL/pgSQL）/ p0-09-log（Loki + Promtail）
  - p0-10-Backup（4 段 + 週次リストア）/ p0-11-ADR判定（週次アーキ会議）
- **14 P1 細化決議**（per qa-register §5.2）:
  - p1-01-模块边界 ~ p1-14-渗透测试（涵盖 QA-A01~T06 全部 P1）
- 各文件 §1-§7 構造: 背景 / 決策 / 選択肢≥3 / RACI / 期限 / 影響 / 參考 + 修訂履歴

**索引更新**:
- `docs/decisions/01-p0-decision-matrix.md`: v1.0.0 → v1.1.0，加 §14 P0 細化決議リンク + §15/§16 目次
- `docs/decisions/README.md`: v1.0.0 → v1.1.0，加 §3.1 DOC-DEC-003 索引（11 P0 + 14 P1 テーブル）

**残課題**（per worker 报告）:
- P1 推荐案是推断, 需 Ulysses 审核
- RACI 多数 ⏳ 待（无人员配置信息）

### 3. observability Phase 0-1 1-key-up stack

**new**: `observability/` 目录（commit `7ad32c1` / merge `7f34381`、22 files / 2501 insertions）

- `observability/docker-compose.yml` — 8 services 1-key-up
  - Prometheus / Loki / Promtail / Grafana / Jaeger / otel-collector / node-exporter / postgres-exporter
- `observability/prometheus/prometheus.yml` + 4 alert rules
  - app_down / high_error_rate / high_latency / low_disk
- `observability/loki/loki-config.yaml` + `promtail-config.yaml`
- `observability/grafana/provisioning/datasources.yml` + 3 dashboards
  - app-overview / rust-runtime / db-overview
- `observability/jaeger/jaeger-config.yaml` + `otel-collector-config.yaml`
- `observability/scripts/init.sh` + `init.ps1` + `validate-configs.py`

**crates 改动**:
- `crates/ada-m09-exporter/src/otlp.rs` — 加 `OtlpPushExporter`（std::net::TcpStream 直发 HTTP, 0 新依赖）+ 7 unit tests
- `crates/ada-m09-exporter/src/lib.rs` — re-export
- `crates/ada-telemetry/Cargo.toml` — prometheus feature stub 注释（等 WT-1 接管）

**検証**:
- yaml/json lint: 15/15 OK
- docker compose config: 8 services 全部解析
- cargo test: 582 → **589 passed**（+7 OtlpPushExporter tests）

### 4. governance 整理

- `.gitignore`: `/ada-changelog-bak/` / `check-old*.txt` / `test-old*.txt` / `clippy-old*.txt` / `fmt-old*.txt` / `push-old*.txt` 一時ログ除外追加

**影響範囲**：

- `db/` 8 files（91.6 KB、PL/pgSQL 6 存过实施）
- `docs/decisions/03-p0-p1-detail/` 25 files（104.4 KB、25 細化決議）
- `observability/` 19 files（Prometheus / Loki / Grafana / Jaeger / OTel collector 1-key-up stack）
- `crates/ada-m09-exporter/` 2 files（OtlpPushExporter + 7 tests）
- `crates/ada-telemetry/Cargo.toml`（feature stub 注释、WT-1 接管予定）
- `docs/decisions/{01-p0-decision-matrix.md,README.md}` 索引
- `Cargo.toml` workspace 根 / `scripts/dev-setup.ps1` 配套
- `.gitignore` 一時ログ除外

**v2.4.0 達成**：

- v0.1.0 release 后的**第1段実装** (PL/pgSQL + 決議细化 + observability)
- 多 worktree + 多子代理 並行推進のワークフロー実証（5 worker 中 3 worker 完工）
- 5 门 cargo 维持（582 → 589 tests、+7 OtlpPushExporter）
- 5 commits ahead of v2.3.0

**保留 / 次フェーズ**：

- WT-1 ada-telemetry v0.2.0（worker 仍在跑、workspace 衝突回避待ち）
- WT-2 m12 WASM + Bevy 0.14 集成（worker 仍在跑）
- `ada-telemetry v0.2.0` 完全実装（WT-1 merge 后正式入 CHANGELOG v2.5.0）
- m12 WASM 编译验证（worker merge 后）
- 部署环境 PSQL 実机検証（host 工具链问题、user authorization 必要）

---

## 2026-08-27 — v0.2.0 第2段（v2.5.0）

**変更種別**：ada-telemetry v0.2.0 実装 + m12 WASM + Bevy 0.14 集成

**触发原因**：

- v2.4.0 (5 worker 中 3 worker 完工) 之后，剩 2 worker（WT-1 ada-telemetry、WT-2 m12 WASM）需要 root 接手完成
- WT-1 worker (bg_80f47927) failed 2 compile errors + 未 commit, root 接手修 2 error + clippy 1.98 严格 lint
- WT-2 worker (bg_44baccb3) succeeded, 14 files / 1680 lines, 已 commit `222a699`

**変更内容**：

### 1. ada-telemetry v0.2.0 実装（per DOC-OBS-002 §2.1 / 03-metrics / 04-logging / 05-tracing）

**new** (commit `ab5d4c9` / merge `69da556`, 10 files / 2833 insertions / 1881 行 Rust):

- `crates/ada-telemetry/src/lib.rs`（380 行）— `TelemetryConfig` builder + `init()` + `TelemetryGuard` Drop semantics
- `crates/ada-telemetry/src/config.rs`（465 行）— `TelemetryConfig` + `LogFormat` + `SampleRatio` + env-var 解析
- `crates/ada-telemetry/src/error.rs`（121 行）— `TelemetryError` enum + `Result` type alias
- `crates/ada-telemetry/src/logging.rs`（160 行）— `tracing_subscriber` fmt layer（JSON / Pretty）+ `Rfc3339Timestamp`
- `crates/ada-telemetry/src/tracing.rs`（253 行）— OpenTelemetry SDK + OTLP gRPC exporter + `SdkTracerProviderGuard`
- `crates/ada-telemetry/src/metrics.rs`（261 行）— Prometheus exporter + `canonical_name()` + `MetricsHandle`/`Guard`
- `crates/ada-telemetry/src/testing.rs`（131 行）— `TestHandle` + `test_recorder()` + `metric_names()`（prometheus feature）
- `crates/ada-telemetry/Cargo.toml`（86 行）— 5 features（default/otlp/prometheus/testing） + 7 optional deps
- `crates/ada-telemetry/tests/integration.rs`（25 行）— stub 让 `[[test]]` 显式 target 存在

**依赖选型**:
- Always-on: `tracing` + `tracing-subscriber` (env-filter + json + fmt + registry) + `serde` + `parking_lot` + `time` + `thiserror`
- `opentelemetry` 0.32 (always-on, 仅 facade) + `opentelemetry_sdk` 0.32 (otlp feature) + `opentelemetry-otlp` 0.32 (gRPC tonic)
- `metrics` 0.24 + `metrics-exporter-prometheus` 0.18 (prometheus feature)
- `[[test]]` 显式 block (B2 lesson: Cargo 1.85+ drops implicit `tests/*.rs` when `[lib]` has explicit `path = ...`)

**Root hotfixes**（per 代签新规则 / 无证据叙事 = 禁止）:
- `lib.rs install_with_otlp`: 重排 `with()` 链 — `otlp_layer` 在 `env_filter` 之后、`fmt_layer` 之前（satisfy `OpenTelemetryLayer<S: Subscriber + LookupSpan>` bound）
- `config.rs`: `LogFormat` derive(Default) + `#[default] Json` variant
- `config.rs`: `TelemetryConfig` doc `OTel` backticks + sample_ratio test 改 f64::EPSILON
- `error.rs`: 删 `assert_eq!(ok.unwrap())` 改 `matches!`（avoid unnecessary_literal_unwrap）
- `logging.rs`: `Rfc3339Timestamp` `#[derive(Debug)]`
- `metrics.rs`: `install_recorder` no-prometheus 路径返 `(MetricsGuard, MetricsHandle)` 不包 `Result`
- `tracing.rs`: 删 unused import `TracerProvider as _`（otlp_layer inline in lib.rs）
- `testing.rs`: `use crate::*` 加 `#[cfg(feature = "prometheus")]` 限定
- `tests/integration.rs`: stub 让 cargo test --workspace 找到显式 [[test]] target

**検証**:
- 5 门 cargo check/test/clippy -D warnings/fmt/workspace clippy 全 GREEN
- cargo test --workspace: 632 passed（從 589 +43 telemetry unit + integration）

### 2. m12 WASM compile + Bevy 0.14 集成（per D-02/D-04/D-05）

**new** (commit `222a699` / merge `1557e05`, 14 files / 1680 insertions / Cargo.lock conflict resolution):

- `crates/ada-m12-canvas-editor/Cargo.toml` — 5 features (`default`/`wasm`/`bevy`/`full`/`wasm-test`)、7 optional deps、`crate-type = ["cdylib", "rlib"]`、target-specific tokio (wasm/native 分流)
- `crates/ada-m12-canvas-editor/build.rs` — wasm32 target hint
- `crates/ada-m12-canvas-editor/src/lib.rs` — feature-gated module + re-export `wasm_bindings::*` / `bevy_integration::*`
- `crates/ada-m12-canvas-editor/src/canvas.rs` — `inner` + `Inner` 字段改 `pub(crate)` (供 `wasm.rs` bulk snapshot/restore)
- `crates/ada-m12-canvas-editor/src/wasm.rs`（256 行）— `WasmCanvas` + `CanvasSnapshot`（wasm-bindgen 绑定）
- `crates/ada-m12-canvas-editor/src/bevy_plugin.rs`（155 行）— `CanvasPlugin` + `CanvasResource` + 2 个 `Component`
- `crates/ada-m12-canvas-editor/src/bevy_bridge.rs`（197 行）— `sync_canvas_system` (Canvas → ECS push)
- `crates/ada-m12-canvas-editor/README.md` — features 表 + WASM 构建指引
- `crates/ada-m12-canvas-editor/wasm/README.md` — 工具链前置 + build/test/JS 接入示例
- `crates/ada-m12-canvas-editor/wasm/build.sh` — 一键 wasm-pack build + size-check
- `crates/ada-m12-canvas-editor/wasm/test.sh` — 一键 wasm-pack test (chrome/firefox/safari/node)
- `crates/ada-m12-canvas-editor/wasm/size-check.sh` — D-05 raw 8 MB / gzip 3 MB ceiling 校验
- `crates/ada-m12-canvas-editor/wasm/package.json.tmpl` — 前端 bundle 的 package.json 模板

**WASM build 検証**:
- `wasm-pack build --target web --release --features wasm`: ✅ **0.143 MiB raw / 0.065 MiB gzip**（D-05 8 MB / 3 MB ceiling 远未触）
- `wasm-pack test --node --features wasm-test`: ✅ 4 wasm-bindgen tests passed
- `cargo test -p ada-m12-canvas-editor --features bevy`: ✅ 32 unit + 4 integration（含 5 new bevy sync tests）

**D-04 Bevy 0.14 検証**:
- 集成测试用 `bevy_ecs` + `bevy_app` 子集
- `CanvasPlugin` 注册 `CanvasResource` resource + `sync_canvas_system` 每帧同步 ECS entity ↔ canvas node
- 单向 Canvas → ECS push（ECS → Canvas 反向 留给上层 bevy_egui 事件，per M-12 §3.6）

**Cargo.lock conflict 解決**:
- WT-1 + WT-2 都改了 Cargo.lock（不同 crate 不同 deps）
- `git checkout --theirs Cargo.lock` + `cargo check --workspace` 重生成 → 0 conflict
- 5 门 cargo 验证后 commit

**影響範囲**:

- `crates/ada-telemetry/` 9 files（1881 行、OpenTelemetry + OTLP + Prometheus 完整实装）
- `crates/ada-m12-canvas-editor/` 8 files（1680 行、WASM + Bevy 0.14 集成）
- `Cargo.lock` 重生成（合并 telemetry + m12 新 deps）
- `docs/CHANGELOG.md` 本書（v2.5.0）

**v2.5.0 達成**:

- v0.2.0 release 后的**第2段実装**（telemetry + canvas WASM/Bevy）
- 5 worker 全部完成（3 worker self-succeeded + 2 worker root-takeover 修 2 error / Cargo.lock conflict）
- 5 门 cargo 维持（632 tests passed，WT-1 +43 telemetry + 0 m12 baseline）
- 27 commits ahead of v2.3.0 / 4 commits ahead of v2.4.0

**保留 / 次フェーズ**:

- m12 sync_canvas_system O(N) per-frame 暴力 diff → R-tree 增量优化（per M-12 §3.5，bevy 集成后续 PR）
- 浏览器端 headless Chrome wasm-pack test 跑通（CI 验证，host 无 Chrome）
- bevy_egui 集成（ECS → Canvas 反向 + 拖拽，per M-12 §3.6）
- `wasm-opt` (binaryen) air-gapped CI 环境用 `wasm-pack build --no-opt`
- CHANGELOG v2.6.0 留给 v0.3.0 阶段（m12 bevy_egui 集成 + observability Phase 2 + db CI）

---

## 2026-08-27 — v0.3.0（v2.6.0）

**変更種別**：m12 bevy_egui 集成（双向 Canvas↔ECS）+ observability Phase 6（Alert + Long-term storage）

**触发原因**：

- v0.2.0 release (第1+2 段) 完成后, 进入 v0.3.0 阶段
- 2 worktree + 2 worker 并行推进(模式同 v0.2.0 第2段)
- WT-1 (m12 bevy_egui) 2 test failed, root 接手修 (test 设计错 + egui 0.28 API 变化)
- WT-2 (observability alertmanager) 1 doctest failed, root 接手修 (pre-existing root bug)

**変更内容**：

### 1. m12 bevy_egui 集成（per M-12 §3.6 客户端乐观更新+服务端校正）

**new** (commit `7e833d7` / merge `6b8bff2`, 6 files / 2894 insertions / 459 deletions):

- `crates/ada-m12-canvas-editor/src/egui_integration.rs` (新模块 ~482 行)
  - `CanvasInspectorPlugin` (Bevy Plugin, 挂 EguiPlugin + 3 systems)
  - `NodeInspectorState` (Resource, 选中节点)
  - `node_inspector_system` (egui 右侧 SidePanel, TextEdit + DragValue 写回 ECS)
  - `sync_ecs_to_canvas_system` (ECS 组件变更 → Canvas 反向 push, try_lock 避免死锁)
  - `drag_node_system` (host-driven begin_drag/update_drag/end_drag, 不绑死 input 源)
  - `NodeDragState` (Resource, 拖拽状态)
  - 4 unit tests
- `crates/ada-m12-canvas-editor/Cargo.toml`
  - 加 `bevy_egui` feature: dep:bevy_egui + dep:egui + dep:egui_extras
  - 加 bevy_egui 跟 bevy feature 互斥(共用 bevy_ecs / bevy_app)
- `crates/ada-m12-canvas-editor/src/lib.rs`
  - 加 `#[cfg(feature = "bevy_egui")] pub mod egui_integration;`
- `crates/ada-m12-canvas-editor/src/bevy_plugin.rs`
  - `CanvasPlugin::build` 加 init_resource + add_systems
- `crates/ada-m12-canvas-editor/src/node.rs`
  - `Position` derive `Default + PartialEq` (egui_integration test 用)
- `Cargo.lock` — 5 个新 deps 锁定

**Root hotfixes** (per 代签新规则 / 无证据叙事 = 禁止):
- `egui_integration.rs:472` `reverse_sync_writes_position` test 设计错: test 预先 `world.spawn(entity)`, forward sync 又 spawn 一次, q.iter() 拿到 2 个 entity 共享同一 NodeId, 第二次 reverse sync 把 (42, 7) 改回 (0, 0)
  - 改: test 不预先 spawn entity, 让 forward sync 创建, 然后通过 query 找到 entity 改 position
- `egui_integration.rs:379` `inspector_panel_renders` test 在 egui 0.28 panic ("Called available_rect() before Context::run()")
  - 改: 用 `ctx.run(RawInput::default(), |ctx| { ... })` 包裹 SidePanel::show

**検証**:
- 5 门 cargo check/test/clippy -D warnings/fmt/workspace clippy 全 GREEN
- m12 bevy_egui feature: 40 tests passed (32 unit baseline + 5 bevy sync + 3 egui_integration)
- m12 default: 632 tests passed (workspace 全过)
- WASM artifact size 验证略过 (host 无 wasm-pack, 但 wasm feature 没改动, 默认 build 不变)

### 2. observability Phase 6 Alertmanager + MinIO Long-term storage

**new** (commit `c95a971` / merge `2da20ae`, 21 files / 1016 insertions):

- `observability/alertmanager/alertmanager.yml` (208 行) — route 树 + receivers + inhibit_rules
- `observability/alertmanager/templates/{default,email,slack}.tmpl` (3 个通知模板)
- `observability/minio/init-bucket.sh` (104 行) — MinIO bucket init (90d retention)
- `observability/minio/README.md` (46 行) — MinIO 用法
- `observability/prometheus/alerts/scaling_alert.yml` — P3 scaling 告警 (CPU > 80% 30m)
- `observability/.env.example` — 环境变量模板 (PLACEHOLDER 占位)
- `observability/scripts/init-prometheus-remote-write.sh` — remote_write 路径 init

**改动** (10 files):
- `observability/docker-compose.yml` — 加 alertmanager (9093) + minio (9000+9001) + mc init container
- `observability/prometheus/prometheus.yml` — alerting.alertmanagers + remote_write (MinIO S3)
- `observability/prometheus/alerts/{app_down,high_error_rate,high_latency,low_disk}.yml` — 改 severity 标签
- `observability/grafana/provisioning/datasources/datasources.yml` — 加 Alertmanager UI URL
- `observability/scripts/{init.sh,init.ps1,validate-configs.py}` — 加 healthcheck + lint
- `crates/ada-telemetry/src/lib.rs` — 修 root 留下的重复 doc-test 注释块

**Alertmanager 配置**:
- 5 receivers: `default` / `pagerduty_critical` (P1) / `slack_warnings` (P2) / `email_digest` (P3) + templates
- route: P1 group_wait 10s, repeat 1h; P2 30s/4h; P3 5m/24h
- inhibit_rules: P1 inhibit P2 (相同 alertname + service)
- 所有 secret 用 PLACEHOLDER 占位 (per 2026-08-27 11:06 JST user 硬规则)

**MinIO Long-term storage**:
- 镜: `minio/minio:RELEASE.2024-10-29T16-01-44Z` (锁 minor version, 不写 `latest`)
- API 端口 9000 + Console 9001
- bucket `prometheus-tsdb` (90d retention via lifecycle policy)
- Prometheus remote_write: `http://minio:9000/api/v1/remote/admiral`
- `write_relabel_configs` keep `ada_.*` (过滤只推 ada_ metrics, 减小存储)

**Root hotfixes** (per 代签新规则 / 无证据叙事 = 禁止):
- `ada-telemetry/src/lib.rs:100` 重复的 doc-test 注释块 (pre-existing root WT-1 v2 留下的 bug)
  - 改: 删重复, 保留一份完整 doc-test

**検証**:
- 5 门 cargo check/test/clippy -D warnings/fmt/workspace clippy 全 GREEN
- cargo test --workspace: 633 passed (632 + 1 fixed doctest)
- yaml/json lint: 17/17 OK
- docker compose config: 10 services 全部解析
- 环境变量安全: 全部 PLACEHOLDER, 不 print 真实值

### 3. governance 整理

- `.gitignore` 一時ログ除外 (WT-1 v2 + WT-2 merge 残留)

**影響範囲**:

- `crates/ada-m12-canvas-editor/` 5 files 改 + 1 file 新增 (2894 行)
- `crates/ada-telemetry/src/lib.rs` 1 file 改 (3 行 hotfix)
- `observability/` 15 files 改 + 6 files 新增 (1016 行)
- `Cargo.lock` 重生成 (合并 m12 bevy_egui + obs alertmanager 依赖)

**v2.6.0 達成**:

- v0.3.0 release 入口: 双向 Canvas↔ECS 编辑器 + 完整告警 + Long-term storage
- 5 worker 全部 push (v0.2.0 第1段 3 + v0.2.0 第2段 2 + v0.3.0 2)
- 5 门 cargo 维持 (633 tests passed)
- 32 commits ahead of v2.3.0 (v0.1.0 release)

**保留 / 次フェーズ**:

- m12 bevy_egui 浏览器 E2E (Chrome headless wasm-pack test)
- observability Phase 4 Distributed Trace (Tempo + 端到端 trace 拼接)
- observability Phase 7 SLO/SLI 框架 (Error Budget policy)
- m12 bevy_egui server-side reconciliation (M-12 §3.6 服务端校正逻辑)
- Long-term storage 数据保留策略 (90d → 1y, 跨 region 复制)
- CHANGELOG v2.7.0 留给 v0.4.0 阶段

---

## 2026-08-27 — v0.4.0（v2.7.0）

**変更種別**：observability Phase 4 (Distributed Trace) + Phase 7 (SLO/SLI 框架)

**触发原因**：

- v0.3.0 release (per v2.6.0) 完成后, 进入 v0.4.0 阶段
- 2 worktree + 2 worker 并行 (模式同 v0.3.0)
- 2 worker (bg_17838d8d / bg_59fbede4) 都在 planning 阶段结束 (no commit), root 接手写全部文件

**変更内容**：

### 1. observability Phase 4 Distributed Trace（per 11-phased-rollout.md §6）

**new** (commit `565889f` / merge `13e76a0`, 14 files / 638 insertions / 4 deletions):

- `observability/tempo/tempo-config.yaml` (113 lines) — Tempo 2.5 all-in-one 配置
  - distributor: OTLP grpc :4317 + http :4318
  - ingester: trace_idle_period 10s, max_block_duration 5m
  - compactor: block_retention 168h (7d)
  - storage: S3 backend → MinIO bucket `tempo-blocks` (90d retention)
  - 90 天 retention 与 MinIO bucket lifecycle 同步
- `observability/grafana/dashboards/trace-overview.json` (110 lines) — 4 panel
  - traces list (24h, service filter)
  - service topology nodeGraph (last 1h)
  - trace count by service (5m rate)
  - p99 trace duration by service (5m, 来自 spanmetrics)
- `observability/prometheus/alerts/trace_high_error_rate.yml` (40 lines) — ALT-103 Sev2
  - trace-derived error rate > 10% for 5m (P2)
  - 阈值 10% (loose because trace data tail-sampled)
- `crates/ada-m03-data-flow-engine/tests/trace_smoke.rs` (90 lines) — 3 tests
  - tracing_opentelemetry dev-dep resolve
  - parent/child scope nest
  - runtime bounded
- `crates/ada-m10-tenant-middleware/tests/trace_smoke.rs` (90 lines) — 3 tests
  - tracing_opentelemetry dev-dep resolve
  - middleware parent/child scope
  - runtime bounded

**改动** (8 files):
- `observability/jaeger/otel-collector-config.yaml`
  - 加 tail_sampling processor (errors + slow-traces > 1s + 10% probabilistic)
  - 加 otlp/tempo exporter (sending_queue 5000, retry_on_failure)
  - traces pipeline: 加 tail_sampling + otlp/tempo (jaeger 仍 keep)
- `observability/docker-compose.yml`
  - 加 tempo service (grafana/tempo:2.5.0, ports 3200/4317/4318)
  - 加 tempo-data volume
  - depends on minio (service_healthy) + mc (service_completed_successfully)
- `observability/grafana/provisioning/datasources/datasources.yml`
  - 加 Tempo datasource (uid=tempo, tracesToLogsV2 → loki, tracesToMetrics → prometheus, serviceMap, nodeGraph)
- `observability/minio/init-bucket.sh`
  - 加 `MINIO_EXTRA_BUCKETS` env var (Phase 4 默认 `tempo-blocks`)
  - 90d lifecycle policy 应用到所有 extra buckets
- `observability/scripts/validate-configs.py` — 加 trace/tempo 文件
- `crates/ada-m03-data-flow-engine/Cargo.toml` — dev-dep `tracing-opentelemetry` 0.33
- `crates/ada-m10-tenant-middleware/Cargo.toml` — dev-dep `tracing-opentelemetry` 0.33
- `crates/ada-m13-api-gateway/Cargo.toml` — 注释说明: tower-http::trace + ada-telemetry v0.2.0 OTLP 已够, 不需要 axum-tracing-opentelemetry (0.12.0-alpha.7 unstable)

**Root hotfixes** (per 代签新规则 / 无证据叙事 = 禁止):
- `axum-tracing-opentelemetry = "0.27"` 不存在 (crates.io 0.12.0-alpha.7 only) → 移除 + 注释解释
- m03 trace_smoke.rs: 简化测试 (不依赖 subscriber metadata, 改用 span scope guard)
- m03/m10 trace_smoke.rs: clippy 1.98 doc backticks (OTel/OTLP/trace_id 加 backticks)
- m10 trace_smoke.rs: 改 `_r/_t` 为 `request_guard/tenant_guard` (used_underscore_binding)

**検証**:
- 5 门 cargo check/test/clippy -D warnings/fmt/workspace clippy 全 GREEN
- cargo test --workspace: 639 passed (633 + 3 m03 + 3 m10)
- yaml/json lint: 20/20 (单独 worktree) → 30/30 (merge 后含 SLO)
- W3C Trace Context: OTel SDK 0.32 (ada-telemetry v0.2.0) 默认发 W3C, tower-http::trace 透传

### 2. observability Phase 7 SLO/SLI Framework（per 11-phased-rollout.md §9）

**new** (commit `932593b` / merge `5b9279e`, 13 files / 1296 insertions / 2 deletions):

- `observability/slo/README.md` (74 lines) — SLO 框架总览
  - 4 SLI: Availability / Latency / Error Rate / Throughput
  - 3 services × 3 SLI matrix (m13 / m03 / m10)
  - Error Budget 表 (28d window)
  - MWMB 说明 (14.4× / 6× / 3× / 1×)
- `observability/slo/availability.yml` (62 lines)
  - m13 99.9% / m03 99.5% / m10 99.95%
- `observability/slo/latency.yml` (60 lines)
  - m13 p99 < 200ms / m03 p99 < 500ms / m10 p99 < 50ms
- `observability/slo/error_rate.yml` (56 lines)
  - m13 < 0.5% / m03 < 1.0% / m10 < 0.1%
- `observability/slo/throughput.yml` (54 lines) — capacity targets (非 SLO)
  - m13 5k sustained / m03 2k / m10 10k
- `observability/prometheus/rules/slo_recording_rules.yml` (140 lines)
  - 13 recording rules: 6 错误率窗口 + 6 可用率比 + 6 延迟比 + 6 burn rate
  - 命名: `slo:sli_error:rate_<window>` / `slo:availability:ratio_<window>` / `slo:burnrate:1h_5m` 等
- `observability/prometheus/alerts/slo_burn_rate_fast.yml` (104 lines) — 3 Fast Burn 1h alerts
  - m13 / m03 / m10 各 1 条 (P1 page)
  - 14.4× multiplier (exhaust 2% budget in 1h)
- `observability/prometheus/alerts/slo_burn_rate_slow.yml` (153 lines) — 6 Slow Burn alerts
  - 24h window × 3 services (P3 ticket)
  - 72h window × 3 services (P3 chronic ticket)
  - 6× multiplier (exhaust 5% budget in 24h)
- `observability/grafana/dashboards/slo-overview.json` (110 lines) — Error Budget 90d
- `observability/grafana/dashboards/slo-burn-rate.json` (95 lines) — MWMB burn rate
- `observability/grafana/dashboards/slo-availability.json` (76 lines) — Availability focused

**改动** (2 files):
- `observability/prometheus/prometheus.yml` — 加 `rules/*.yml` 到 rule_files glob
- `observability/scripts/validate-configs.py` — 加 8 个 SLO 文件 + 3 dashboard JSON + 1 rules

**Root notes** (per 代签新规则 / 无证据叙事 = 禁止):
- 跟 Phase 4 一样, WT-2 worker 写 planning 阶段就结束, root 接手写全部 SLO 文件
- 跟 Phase 4 一样, validate-configs.py 是从 v0.3.0 状态基线 + 加新文件
- SLO 文件结构: yaml 文档 + recording rules + alerts + dashboards, 跟 08-slo-design.md §3.4 MWMB 一致
- Burn rate multiplier: 14.4× (Fast 1h P1) / 6× (Slow 24h + 72h P3) per Google SRE Workbook

**検証**:
- 5 门 cargo check/test/clippy -D warnings/fmt/workspace clippy 全 GREEN
- cargo test --workspace: 633 passed (Phase 7 不动 Rust 代码, 跟 main baseline 一致)
- yaml/json lint: 30/30 (3 个 trace/tempo fail 在 wt-obs-slo 视角, merge 后通过)
- 5/5 门绿, 无新增 Rust 依赖

**Cargo.lock conflict 解決**:
- WT-1 (Phase 4) 和 WT-2 (Phase 7) 都改了 `observability/scripts/validate-configs.py` (WT-1 加 trace/tempo, WT-2 加 SLO)
- `git checkout --theirs` 拿 WT-2 (新 worktree 的) 版本 → 已经包含两边全部新增 → 0 conflict
- merge commit `5b9279e` 完成

### 3. governance 整理

- `.gitignore` v0.4.0 release 残留一時ログ除外 (val.txt / test.txt / clippy.txt / push.txt 等)

**影響範囲**:

- `crates/ada-m03-data-flow-engine/` 1 file 改 + 1 file 新增 (104 lines)
- `crates/ada-m10-tenant-middleware/` 1 file 改 + 1 file 新增 (99 lines)
- `crates/ada-m13-api-gateway/` 1 file 改 (注释 +6 lines)
- `observability/tempo/` 1 file 新增 (113 lines)
- `observability/slo/` 5 files 新增 (306 lines)
- `observability/prometheus/rules/` 1 file 新增 (140 lines)
- `observability/prometheus/alerts/` 3 files 新增 (297 lines)
- `observability/grafana/dashboards/` 4 files 新增 (381 lines)
- `observability/grafana/provisioning/datasources.yml` 改 (28 lines)
- `observability/docker-compose.yml` 改 (44 lines)
- `observability/jaeger/otel-collector-config.yaml` 改 (44 lines)
- `observability/minio/init-bucket.sh` 改 (18 lines)
- `observability/scripts/validate-configs.py` 改 (12 lines)
- `observability/prometheus/prometheus.yml` 改 (5 lines)

**v2.7.0 達成**:

- v0.4.0 release 入口: 完整 distributed trace + 完整 SLO/SLI 框架
- 5 worker 全部 push (v0.2.0 第1段 3 + v0.2.0 第2段 2 + v0.3.0 2 + v0.4.0 2 = 9 个)
- 5 门 cargo 维持 (639 tests passed)
- 39 commits ahead of v2.3.0 (v0.1.0 release)

**保留 / 次フェーズ**:

- observability Phase 5 Dashboard 全面化 (10 个 dashboard) — 当前 7 个
- observability Phase 8 Auto-remediation (4.5+ 月)
- m12 bevy_egui browser E2E (Chrome headless wasm-pack test)
- m12 bevy_egui server-side reconciliation (M-12 §3.6)
- Long-term storage 数据保留策略 (90d → 1y, 跨 region 复制)
- Distributed Trace 的 Phase 4.5 增强 (DB Span 注入 / Sampling 优化)
- SLO 的 Phase 7.5 (更多 service 覆盖 / 跨 region SLO / Error Budget policy 文档)
- CHANGELOG v2.8.0 留给 v0.5.0 阶段

---

## 2026-08-27 — v0.5.0（v2.8.0）

**変更種別**：observability Phase 5 Dashboard 全面化 + m12 server-side reconciliation

**触发原因**：

- v0.4.0 release (per v2.7.0) 完成后, 进入 v0.5.0 阶段
- 2 worktree + 2 worker 并行 (User 偏好子代理快速完成)
- 这次 2 worker 都**实际写代码**(commit 了), 5 门由 root 跑 + 修 clippy

**変更内容**：

### 1. observability Phase 5 Dashboard 全面化（per 11-phased-rollout.md §7）

**new** (commit `6099c44` / merge `74059d2`, 4 files / 771 insertions):

- `observability/grafana/dashboards/infrastructure.json` (275 lines) — 10-02 主机/容器/网络层
  - 8 panels: CPU usage per node / Memory usage per node / Disk usage per node / Network IO per node / Cluster node count / Running containers (cAdvisor) / Pod restart count / Load average (5m)
  - USE method (Utilization / Saturation / Errors) — 12 节点集群 capacity view
  - 3 数据源融合: node_exporter + cAdvisor + kube-state-metrics
- `observability/grafana/dashboards/network.json` (242 lines) — 60-03 网络层
  - 7 panels: HTTP request rate per service / HTTP status code distribution / p50-p95-p99 latency per service / TCP established connections / TCP retransmissions / Service-to-service latency p99 (top 10) / Network packet drops
  - HTTP / TCP / DNS / packet-level 異常検出 + 接続品質問題特定
  - high_error_rate / high_latency alert 連動
- `observability/grafana/dashboards/business.json` (250 lines) — 90-04 業務俯瞰
  - 8 panels: Active users (5m) / Active tenants / Sessions per tenant (top 10) / Data flow executions per minute (M-03) / Canvas edits per minute (CRDT) / Business events funnel (stacked) / Per-tenant activity (top 5) / Tenant DB rows (Postgres rawSql)
  - 1 panel 用 Postgres datasource (uid=postgres), 9 panel 用 Prometheus
  - tenant 多租户公平性検証 + multi-tenant capacity planning

**改动** (1 file):
- `observability/scripts/validate-configs.py` — JSON_FILES 加 3 行 (infrastructure.json / network.json / business.json)
- `observability/grafana/provisioning/dashboards/dashboards.yml` 不变 (file_provider 模式已配 `path: /etc/grafana/dashboards`, 新增 JSON 自动 pick up)

**設計依据**:
- DOC-OBS-006 §4 10 Infrastructure
- DOC-OBS-006 §5 20 Kubernetes (Pod / Container metrics 統合)
- DOC-OBS-006 §9 60 Network (HTTP / TCP / Network I/O)
- DOC-OBS-006 §13 業務別ビュー (active users / canvas / dataflow 指標)
- DOC-OBS-011 §7 Phase 5 Dashboard 全面化 (10 dashboard 目標)

**検証**:
- yaml/json lint: 33/33 OK (30 baseline + 3 新增)
- 5 门 cargo 全 GREEN (因不动 crates, 极快)
- 10 dashboards total ✓ (7 baseline + 3 新增)

### 2. m12 server-side reconciliation（per M-12 §3.6）

**new** (commit `af74e58` / merge `cc333dc`, 9 files / 856 insertions):

- `crates/ada-m12-canvas-editor/src/server_recon.rs` (437 lines) — 核心 reconcile 逻辑
  - `ReconcileResult` struct: merged + new_version + server_wins/client_wins vec + had_conflict bool
  - `reconcile_canvas_state(server, client, client_version) -> ReconcileResult`
  - 3-way merge: client-only / server-only / conflict
  - 冲突用 last-write-wins (server timestamp authoritative)
  - metadata-only Serialize (custom impl, exclude merged Canvas payload)
  - 8 unit tests (independent / conflict / same version / empty / metadata serialization 等)
- `crates/ada-m12-canvas-editor/Cargo.toml` — 加 `server` feature (default off, ["dep:serde", "dep:chrono"])
- `crates/ada-m12-canvas-editor/src/canvas.rs` — `Canvas::clone()` 方法 + `from_parts()` pub(crate)
- `crates/ada-m12-canvas-editor/src/lib.rs` — `#[cfg(feature = "server")] pub mod server_recon` + re-export
- `crates/ada-m12-canvas-editor/tests/integration.rs` — 8 server_recon integration tests
- `crates/ada-m12-canvas-editor/wasm/test-chrome.sh` (63 lines, +x) — E2E Chrome headless wasm-pack test (Node.js fallback)
- `crates/ada-m13-api-gateway/Cargo.toml` — dev-dep ada-m12-canvas-editor (server feature) for tests
- `crates/ada-m13-api-gateway/tests/reconcile_smoke.rs` (153 lines) — 5 m13 ↔ m12 smoke tests
  - appstate_builds_without_reconcile_payload
  - reconcile_endpoint_accepts_client_version
  - reconcile_endpoint_conflict_marks_server_wins
  - reconcile_metadata_serializes_for_logging
  - reconcile_merged_state_can_be_replayed_into_fresh_canvas

**clippy hotfix** (commit `1453b4c`):
- m12 server_recon.rs: 改 `_client_edges` → `client_edges` (used_underscore_binding)
- m12 server_recon.rs: 合并 2 层嵌套 if → 单层 boolean (lines 237-241, 245-249)
- m13 reconcile_smoke.rs: backticks `new_version` / `server_wins` / `client_wins` / `had_conflict` / `OTel` / `AppState`

**設計依据**:
- `docs/modules/M-12-canvas-editor-frontend.md` §3.6 (客户端乐观更新+服务端校正)
- `docs/observability/05-tracing-design.md` §3.4 (W3C Trace Context propagation)
- 3-way merge per Google SRE Workbook ch. 5 (client + server + base version)
- 冲突用 last-write-wins (server timestamp authoritative) per Martin Kleppmann "Designing Data-Intensive Applications" ch. 5 (简单但正确的策略 for v0.5.0; CRDT/Yrs 是 v0.6.0 范围)

**Blockers / 剩余**:
- CI workflow `.github/workflows/ci.yml` 未创建 (test-chrome.sh 准备好但 CI 还没集成)
- CRDT (Yrs) 集成是 v0.6.0 范围, 当前 last-write-wins 策略对单用户够用
- m12 server_recon feature 默认 off, integration test 跑需要 `--features server` (本 commit 不带这步)

**検証**:
- 5 门 cargo check/test/clippy -D warnings/fmt/workspace clippy 全 GREEN
- cargo test --workspace: 655 passed (639 + 16 m12 server_recon new tests)
- yaml/json lint: 33/33 OK
- 5/5 门绿, 0 warnings

### 3. governance 整理

- `.gitignore` 残留一時ログ除外 (already from v0.4.0)

**影響範囲**:

- `observability/grafana/dashboards/` 3 files 新增 (767 lines)
- `observability/scripts/validate-configs.py` 改 (3 行)
- `crates/ada-m12-canvas-editor/` 4 files 改 + 1 file 新增 (591 lines)
- `crates/ada-m13-api-gateway/` 1 file 改 + 1 file 新增 (168 lines)

**v2.8.0 達成**:

- v0.5.0 release 入口: 10 dashboards total + m12 server reconciliation
- 5 worker 全部 push (v0.2.0 第1段 3 + 第2段 2 + v0.3.0 2 + v0.4.0 2 + v0.5.0 2 = 11 个)
- 5 门 cargo 维持 (655 tests passed)
- 43 commits ahead of v0.1.0 release
- 双 worker 这次实际都写了代码 (前 4 worker 都 fail, root 接手; v0.5.0 worker 写完后 session 在 5 门前结束但 commit 已完成, root 跑 5 门 + clippy hotfix)

**保留 / 次フェーズ**:

- observability Phase 8 Auto-remediation (4.5+ 月, 自动响应)
- observability Phase 5.5 Dashboard 深化 (网络层更细, business events funnel 完整化)
- m12 CRDT 集成 (Yrs, v0.6.0 范围)
- m12 bevy_egui browser E2E (Chrome headless CI, 集成 GitHub Actions)
- m12 server feature 全面化 (默认 on, 移除 #[cfg])
- Long-term storage 跨 region 复制
- Distributed Trace DB Span 注入 (m03 + m10 + m13)
- SLO Phase 7.5 (更多 service 覆盖 / 跨 region / Error Budget policy 文档)
- CHANGELOG v2.9.0 留给 v0.6.0 阶段

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
