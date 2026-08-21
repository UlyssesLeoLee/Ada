# Ada 无限画布跨平台数据集成系统 文档总览

> **本目录按"内容模块"重新组织**。每份内容仅出现在所属模块的 MD 文件中，互不重复。

> **ドキュメントID**：DOC-INDEX-001
> **文書分類**：横断文書
> **バージョン**：v1.9.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：無
> **下位文書**：`docs/template.md`、`docs/CHANGELOG.md`、`docs/legacy/*`、`docs/architecture/*`、`docs/modules/*`、`docs/api/*`、`docs/tests/*`、`docs/templates/*`、`docs/upstream/*`、`docs/requirements/*`、`docs/management/*`、`docs/business/*`、`docs/decisions/*`、`docs/observability/*`
> **関連文書**：無
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018)
> - IPA「非機能要求グレード」(2018)
> - IPA「ソフトウェア開発データ白書」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（模块化索引） | Ada プロジェクトチーム | TBD | TBD |
| v1.1.0 | 2026-08-19 | IPA 準拠化（追加表头/页脚/参考文献/用語集） | Ada プロジェクトチーム | TBD | TBD |
| v1.2.0 | 2026-08-19 | 原子化部署 / 中心事件 / 集群 / 热插拔 大扩展（M-14/15/16 + DOC-ARCH-005/006 + DOC-API-004/005/006） | Ada プロジェクトチーム | TBD | TBD |
| v1.3.0 | 2026-08-19 | Rust 技術スタック選択書（DOC-ARCH-007）追加 | Ada プロジェクトチーム | TBD | TBD |
| v1.4.0 | 2026-08-19 | 实施前 QA 登録簿（DOC-ARCH-008）追加 | Ada プロジェクトチーム | TBD | TBD |
| v1.5.0 | 2026-08-20 | IPA ワークフロー全体俯瞰（DOC-ARCH-009）追加 | Ada プロジェクトチーム | TBD | TBD |
| v1.6.0 | 2026-08-20 | 工程別テンプレート集（`docs/templates/`、DOC-TPL-INDEX + 8 カテゴリ × 62 テンプレート）追加 | Ada プロジェクトチーム | TBD | TBD |
| v1.7.0 | 2026-08-20 | 超上流/要件/管理/業務 4 新ディレクトリ追加（upstream 8 + requirements 10 + management 5 + business 1 = 24 ドキュメント） | Ada プロジェクトチーム | TBD | TBD |
| v1.8.0 | 2026-08-20 | 意思決定ドキュメント（`docs/decisions/`、11 P0 + 15 D-ADR）+ Cargo Workspace 18 crate scaffold 追加 |
| v1.9.0 | 2026-08-20 | Observability Platform 設計（`docs/observability/`、14 ファイル / 210 KB / DOC-OBS-INDEX + 13 + 10 OBS-ADR）追加 | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 概要
2. 适用 IPA 標準
3. 阅读路径
4. 模块索引
5. 横切关注点（architecture/）
6. API 与契约（api/）
7. 测试设计书（tests/）
8. 模板与履历
9. バージョン履歴
10. 拆分约定
11. 用語集
12. 参考文献

---

## 1. 概要

本目录是 Ada 无限画布跨平台数据集成系统的**唯一权威文档源**。所有文档遵循 IPA「共通フレーム2018」(SLCP-JCF2018) 定义的文档标准，并按"内容模块"重新组织：每份内容仅出现在所属模块的 MD 文件中，互不重复。

## 2. 适用 IPA 標準

- **共通フレーム2018 (SLCP-JCF2018)** — 文書ライフサイクル・承認プロセス・文書分類
- **非機能要求グレード2018** — 6 大類 × 必須/推奨 の 2 段階評価
- **ソフトウェア開発データ白書** — 文書メタデータ形式

各ドキュメントの表頭メタデータ・改訂履歴・目次・用語集・参考文献の構成は [`docs/template.md`](template.md) に従う。

## 3. 阅读路径

- **第一次了解本系统** → 先读 [`architecture/00-anatomy-model.md`](architecture/00-anatomy-model.md) 建立仿生模型心智模型，再扫一遍各模块文件的"## 1. 需求来源"节。
- **按模块开发** → 直接打开 `modules/M-XX-xxx.md`（13 份之一），每份都自带 4 段统一结构：需求来源 → 基本设计 → 详细设计 → 验收要点。
- **做架构/技术选型/部署决策** → [`architecture/`](architecture/) 目录。
- **对接 API 或排查错误码** → [`api/`](api/) 目录。
- **查阅历史版本与变更履历** → [`CHANGELOG.md`](CHANGELOG.md)；原始三份大文件归档在 [`legacy/`](legacy/)。
- **编写新文档** → 先读 [`template.md`](template.md) 掌握 IPA 準拠格式。

## 4. 模块索引

| 编号 | 文档ID | 模块名 | 対応要件 | 文档 |
|---|---|---|---|---|
| M-01 | DOC-MOD-001 | 采集适配器 | F-02 / F-15 / F-16 | [→](modules/M-01-acquisition-adapter.md) |
| M-02 | DOC-MOD-002 | 标准化转换 | F-03 | [→](modules/M-02-normalizer.md) |
| M-03 | DOC-MOD-003 | 数据流引擎 | F-04 | [→](modules/M-03-data-flow-engine.md) |
| M-04 | DOC-MOD-004 | 编排引擎 | F-05 | [→](modules/M-04-orchestration-engine.md) |
| M-05 | DOC-MOD-005 | 控制流执行器 | F-06 | [→](modules/M-05-control-flow-executor.md) |
| M-06 | DOC-MOD-006 | 节点运行时 / 插件 SDK | F-07 | [→](modules/M-06-node-runtime-plugin-sdk.md) |
| M-07 | DOC-MOD-007 | 可视化调试 | F-08 | [→](modules/M-07-debug-service.md) |
| M-08 | DOC-MOD-008 | 定时/事件触发器 | F-13 | [→](modules/M-08-trigger-service.md) |
| M-09 | DOC-MOD-009 | 输出 / 导出 | F-14 | [→](modules/M-09-exporter.md) |
| M-10 | DOC-MOD-010 | 多租户中间件 | F-17 | [→](modules/M-10-tenant-middleware.md) |
| M-11 | DOC-MOD-011 | 权限与协作 | F-11 | [→](modules/M-11-rbac-collab.md) |
| M-12 | DOC-MOD-012 | 前端画布编辑器 | F-01 | [→](modules/M-12-canvas-editor-frontend.md) |
| M-13 | DOC-MOD-013 | API Gateway | 横断 | [→](modules/M-13-api-gateway.md) |
| M-14 | DOC-MOD-014 | 模块注册与生命周期 | DOC-ARCH-005 | [→](modules/M-14-module-registry.md) |
| M-15 | DOC-MOD-015 | 中心事件总线 | DOC-ARCH-005 | [→](modules/M-15-central-event-bus.md) |
| M-16 | DOC-MOD-016 | 集群协调 | DOC-ARCH-005 | [→](modules/M-16-cluster-coordinator.md) |

## 5. 横切关注点（architecture/）

| 文档ID | 文件 | 内容 | 来源 |
|---|---|---|---|
| DOC-ARCH-001 | [00-anatomy-model.md](architecture/00-anatomy-model.md) | 仿生模型总览：骨/血/神经/肌肉四层职责与设计原则 | DOC-REQ-001 §5、DOC-BSC-001 §2.1 |
| DOC-ARCH-002 | [01-tech-stack.md](architecture/01-tech-stack.md) | 各层技术选型与备选 | DOC-BSC-001 §9 |
| DOC-ARCH-003 | [02-deployment.md](architecture/02-deployment.md) | 单机本地 / 多租户 SaaS / 混合三种部署模式 | DOC-BSC-001 §8 |
| DOC-ARCH-004 | [03-cross-cutting-risks.md](architecture/03-cross-cutting-risks.md) | 跨模块风险与应对 | DOC-REQ-001 §11、DOC-BSC-001 §10 |
| DOC-ARCH-005 | [04-atomic-deployment.md](architecture/04-atomic-deployment.md) | 原子化部署 + 中心事件 + 集群 + 热插拔 4 大能力总论 | 2026-08-19 追加 |
| DOC-ARCH-006 | [05-admin-operations-ui.md](architecture/05-admin-operations-ui.md) | 管理员运维界面规范 | 2026-08-19 追加 |
| DOC-ARCH-007 | [06-rust-tech-selection.md](architecture/06-rust-tech-selection.md) | Rust 技術スタック選択書（主要言語、crate 単位の詳細選定 + ADR） | 2026-08-19 追加 |
| DOC-ARCH-008 | [07-qa-register.md](architecture/07-qa-register.md) | 实施前 QA 登録簿（懸念・疑問・未決・仮定・リスクの実装前集約リスト） | 2026-08-19 追加 |
| DOC-ARCH-009 | [08-workflow-overview.md](architecture/08-workflow-overview.md) | IPA 共通フレーム2018 ワークフロー全体俯瞰（150 工程 × 16 カテゴリ × ステータス × RACI × 11 ゲート × 監査ポイント） | 2026-08-20 追加 |

## 5.5. 工程別テンプレート集（templates/）

> ⚪（未着手）工程を実行するたびに以下のテンプレートを派生して記録する。62 テンプレートで IPA 150 工程の 80 件をカバー。

| ドキュメントID | ファイル | 内容 | テンプレート数 | 対応 IPA 工程 |
|---|---|---|---|---|
| DOC-TPL-INDEX | [templates/README.md](templates/README.md) | テンプレート集総覧 + 使い方 + 命名規約 | — | — |
| DOC-TPL-REV | [templates/01-reviews.md](templates/01-reviews.md) | レビュー記録（RD/BD/DD/UT/ST 完了/UAT/Release/PJ 完了） | 8 | 20, 41, 52, 61, 89, 94, 103, 145 |
| DOC-TPL-TST | [templates/02-tests-execution.md](templates/02-tests-execution.md) | 試験実施ログ（UT/IT/ST/UAT/検収/障害/回帰） | 11 | 60, 62-75, 78-88, 92-95 |
| DOC-TPL-PRC | [templates/03-process-management.md](templates/03-process-management.md) | プロセス管理（WBS/進捗/課題/リスク/会議/工数/コスト） | 7 | 132-142 |
| DOC-TPL-RBK | [templates/04-runbooks.md](templates/04-runbooks.md) | Runbook（開発環境/IT 環境/移行/Deploy/Smoke/Hypercare） | 11 | 53, 68, 97-101, 105-108 |
| DOC-TPL-OPS | [templates/05-operations.md](templates/05-operations.md) | 運用管理（引継ぎ/監視/ジョブ/BK/Capa/Incident/Postmortem/Problem/Support） | 9 | 109-117 |
| DOC-TPL-CHG | [templates/06-change-management.md](templates/06-change-management.md) | 変更・保守管理（CR/影響分析/承認/CM/Patch/Vuln/改修/Hotfix/回帰） | 9 | 118-126 |
| DOC-TPL-QUA | [templates/07-quality.md](templates/07-quality.md) | 品質管理（QA Review/Eval/Audit） | 3 | 128-130 |
| DOC-TPL-CLO | [templates/08-closure.md](templates/08-closure.md) | 終結（引渡し/完了報告/Retrospective/KT/Archive） | 5 | 146-150 |
| **合計** | | | **62** | **80 工程** |

## 5.6. 超上流工程文档（upstream/）

> IPA 工程 01-09 + 131（PJ 計画）+ 135（リスク初期）+ 140（ステークホルダ）に対応。

| ドキュメントID | ファイル | タイトル | 対応 IPA 工程 |
|---|---|---|---|
| DOC-UP-INDEX | [upstream/README.md](upstream/README.md) | 超上流工程総覧 | — |
| DOC-UP-001 | [upstream/01-pj-charter.md](upstream/01-pj-charter.md) | プロジェクト憲章 | 05, 131 |
| DOC-UP-002 | [upstream/02-stakeholder-register.md](upstream/02-stakeholder-register.md) | ステークホルダ登録簿 | 05, 140 |
| DOC-UP-003 | [upstream/03-as-is-business.md](upstream/03-as-is-business.md) | 現行業務フロー（As-Is） | 06 |
| DOC-UP-004 | [upstream/04-as-is-system.md](upstream/04-as-is-system.md) | 現行システム構成（As-Is） | 07 |
| DOC-UP-005 | [upstream/05-issue-list.md](upstream/05-issue-list.md) | 課題一覧 | 08 |
| DOC-UP-006 | [upstream/06-to-be-business.md](upstream/06-to-be-business.md) | 新業務フロー（To-Be） | 09 |
| DOC-UP-007 | [upstream/07-to-be-system.md](upstream/07-to-be-system.md) | 新システム構成（To-Be） | 09, 22, 24 |
| DOC-UP-008 | [upstream/08-initial-risk-assessment.md](upstream/08-initial-risk-assessment.md) | 初期リスク評価 | 08, 135 |

## 5.7. 要件定義ドキュメント（requirements/）

> IPA 工程 10-19（要件定義）に対応。トレーサビリティ確保のため要件種別ごとに独立文書化。

| ドキュメントID | ファイル | タイトル | 対応 IPA 工程 |
|---|---|---|---|
| DOC-REQ-INDEX | [requirements/README.md](requirements/README.md) | 要件定義総覧 | — |
| DOC-REQ-UR-001 | [requirements/01-ur-user-requirements.md](requirements/01-ur-user-requirements.md) | ユーザー要求定義書（UR） | 10 |
| DOC-REQ-BR-001 | [requirements/02-br-business-requirements.md](requirements/02-br-business-requirements.md) | 業務要件定義書（BR） | 11 |
| DOC-REQ-SR-001 | [requirements/03-sr-system-requirements.md](requirements/03-sr-system-requirements.md) | システム要件定義書（SR） | 12 |
| DOC-REQ-FR-001 | [requirements/04-fr-functional-requirements.md](requirements/04-fr-functional-requirements.md) | 機能要件定義書（FR / F-01〜F-17） | 13 |
| DOC-REQ-NFR-001 | [requirements/05-nfr-non-functional-requirements.md](requirements/05-nfr-non-functional-requirements.md) | 非機能要件定義書（NFR / 6 区分） | 14 |
| DOC-REQ-DATA-001 | [requirements/06-data-requirements.md](requirements/06-data-requirements.md) | データ要件定義書 | 15 |
| DOC-REQ-IF-001 | [requirements/07-external-if-requirements.md](requirements/07-external-if-requirements.md) | 外部 IF 要件定義書 | 16 |
| DOC-REQ-SEC-001 | [requirements/08-security-requirements.md](requirements/08-security-requirements.md) | セキュリティ要件定義書 | 17 |
| DOC-REQ-OPS-001 | [requirements/09-operation-requirements.md](requirements/09-operation-requirements.md) | 運用要件定義書 | 18 |
| DOC-REQ-MIG-001 | [requirements/10-migration-requirements.md](requirements/10-migration-requirements.md) | 移行要件定義書 | 19 |

## 5.7.5. 意思決定ドキュメント（decisions/）

> G4 实施着手判定前に必要な 11 P0 决策 + 15 D-ADR を集約。

| ドキュメントID | ファイル | タイトル | 対応 |
|---|---|---|---|
| DOC-DEC-INDEX | [decisions/README.md](decisions/README.md) | 意思決定総覧 | — |
| DOC-DEC-001 | [decisions/01-p0-decision-matrix.md](decisions/01-p0-decision-matrix.md) | 11 P0 决策矩阵 | UN-P0-01〜11 |
| DOC-DEC-002 | [decisions/02-design-adrs.md](decisions/02-design-adrs.md) | D-01〜15 設計 ADR | D-01〜15 |

## 5.8. プロジェクト管理ドキュメント（management/）

> IPA 工程 138-140, 143-144 に対応。

| ドキュメントID | ファイル | タイトル | 対応 IPA 工程 |
|---|---|---|---|
| DOC-MGT-INDEX | [management/README.md](management/README.md) | プロジェクト管理総覧 | — |
| DOC-MGT-DLV-001 | [management/01-deliverable-list.md](management/01-deliverable-list.md) | 成果物一覧 | 138 |
| DOC-MGT-REV-001 | [management/02-review-schedule.md](management/02-review-schedule.md) | レビュー管理表 | 139 |
| DOC-MGT-SCP-001 | [management/03-scope-statement.md](management/03-scope-statement.md) | スコープベースライン | 143, 144 |
| DOC-MGT-COM-001 | [management/04-communication-plan.md](management/04-communication-plan.md) | コミュニケーション計画 | 140 |

## 5.9. 業務ドキュメント（business/）

> 業務シナリオ試験（IPA 工程 93）のインプット。

| ドキュメントID | ファイル | タイトル | 対応 IPA 工程 |
|---|---|---|---|
| DOC-BIZ-SCN-001 | [business/01-scenario-catalog.md](business/01-scenario-catalog.md) | 業務シナリオ集 | 93 |

## 5.10. Observability Platform（observability/）

> IPA 工程 116-117（運用・監視）+ 109（サービスレベル管理）+ 110-115（キャパシティ・障害・問題）に対応。  
> 既存プラットフォームに**最小侵襲**で可観測性能力を追加し、Observe → Detect → Correlate → Diagnose → Alert → Recover の閉ループを実現する。

| ドキュメントID | ファイル | タイトル | 規模 |
|---|---|---|---|
| DOC-OBS-INDEX | [observability/README.md](observability/README.md) | Observability Platform 設計総覧（13 章構成 + 10 OBS-ADR + 1 段階的導入計画） | 14 KB |
| DOC-OBS-001 | [observability/01-current-state-analysis.md](observability/01-current-state-analysis.md) | 現状分析（Phase 0、サービストポロジ、ギャップ分析） | 11 KB |
| DOC-OBS-002 | [observability/02-architecture.md](observability/02-architecture.md) | 全体アーキテクチャ（OTel + Prometheus/Mimir + Loki + Tempo + Grafana） | 13 KB |
| DOC-OBS-003 | [observability/03-metrics-design.md](observability/03-metrics-design.md) | Metrics 設計（RED/USE フレームワーク、18 crate × 4 次元） | 18 KB |
| DOC-OBS-004 | [observability/04-logging-design.md](observability/04-logging-design.md) | Logging 設計（構造化 JSON + PII 自動 redaction） | 12 KB |
| DOC-OBS-005 | [observability/05-tracing-design.md](observability/05-tracing-design.md) | Tracing 設計（W3C Trace Context + OTel Span） | 13 KB |
| DOC-OBS-006 | [observability/06-dashboard-catalog.md](observability/06-dashboard-catalog.md) | Dashboard カタログ（10 個、Grafana） | 19 KB |
| DOC-OBS-007 | [observability/07-alert-policy.md](observability/07-alert-policy.md) | Alert Policy（4 段階 Sev + SLO Burn Rate） | 12 KB |
| DOC-OBS-008 | [observability/08-slo-design.md](observability/08-slo-design.md) | SLO/SLI 設計（Error Budget + Multi-window Burn Rate） | 13 KB |
| DOC-OBS-009 | [observability/09-security-design.md](observability/09-security-design.md) | セキュリティ設計（RBAC + mTLS + NetworkPolicy + GDPR/PIPL） | 16 KB |
| DOC-OBS-010 | [observability/10-deployment-design.md](observability/10-deployment-design.md) | デプロイ設計（Helm + GitOps + ArgoCD） | 20 KB |
| DOC-OBS-011 | [observability/11-phased-rollout.md](observability/11-phased-rollout.md) | 段階的導入計画（Phase 0-8 / 9 ヶ月 / 各 Phase GATE 判定） | 19 KB |
| DOC-OBS-012 | [observability/12-code-impact.md](observability/12-code-impact.md) | コード影響分析（18 crate × Low/Med/High + 49 人日） | 16 KB |
| DOC-OBS-013 | [observability/13-self-audit.md](observability/13-self-audit.md) | アーキテクチャ自審（11 カテゴリ × 48 チェック + Revision 2 計画） | 16 KB |
| **合計** | | | **210 KB / 14 ファイル** |

## 6. API 与契约（api/）

| 文档ID | 文件 | 内容 | 来源 |
|---|---|---|---|
| DOC-API-001 | [rest-endpoints.md](api/rest-endpoints.md) | REST API 端点清单（业务面） | DOC-BSC-001 §7.1、DOC-DTL-001 §13 |
| DOC-API-002 | [websocket-events.md](api/websocket-events.md) | WebSocket 事件推送协议 | DOC-BSC-001 §7.2、DOC-DTL-001 §13.3 |
| DOC-API-003 | [error-codes.md](api/error-codes.md) | 错误码体系（模块内部 Error → 对外 HTTP Code） | DOC-DTL-001 §14 |
| DOC-API-004 | [admin-modules.md](api/admin-modules.md) | Admin API - 模块管理 | DOC-ARCH-005 |
| DOC-API-005 | [admin-events.md](api/admin-events.md) | Admin API - 事件中心 | DOC-ARCH-005 |
| DOC-API-006 | [admin-cluster.md](api/admin-cluster.md) | Admin API - 集群管理 | DOC-ARCH-005 |

## 7. 测试设计书（tests/）

| 文档ID | 文件 | 範囲 | 用例数 |
|---|---|---|---|
| DOC-TST-INDEX | [tests/README.md](tests/README.md) | 测试总览与命名约定 | — |
| DOC-TST-001 | [tests/UT-design.md](tests/UT-design.md) | 单元测试：13 模块 × 平均 8-12 用例 | ~169 |
| DOC-TST-002 | [tests/IT-design.md](tests/IT-design.md) | 集成测试：模块间接口、DB 集成、前后端集成 | ~47 |
| DOC-TST-003 / DOC-ACC-001 | [tests/ST-design.md](tests/ST-design.md) | 系统测试 + 受入测试：E2E + NFR + ACC + SMK | ~88 |

## 8. 模板与履历

| 文档ID | 文件 | 内容 |
|---|---|---|
| DOC-TPL-001 | [template.md](template.md) | IPA 準拠 文档标准格式模板（所有文档的格式参照） |
| DOC-CHG-001 | [CHANGELOG.md](CHANGELOG.md) | 全部文档的变更履历 |
| DOC-INDEX-001 | [README.md](README.md) | 本文件（文档总览） |

## 9. バージョン履歴

- 当前各模块文件版本号继承自 `legacy/` 下三份原文档中影响该模块的最新版本（详见 [CHANGELOG.md](CHANGELOG.md)）。
- 三份原文档本身（`requirements.md` v1.2.1、`basic-design.md` v1.3.0、`detailed-design.md` v1.3.0）已归档到 [`legacy/`](legacy/)，仅作为权威版本历史保留，不再修改。

## 10. 拆分约定

- **每份 M-XX 文件统一 4 段结构**：`## 1. 需求来源` → `## 2. 基本设计` → `## 3. 详细设计` → `## 4. 验收要点`。
- **横切关注点不进入模块文件**：仿生模型、技术栈、部署、跨模块风险统一放 `architecture/`；REST/WS/ErrorCode 统一放 `api/`。
- **多对一 F-IDs 归并**：当一个 F-ID 跨多个模块时（如 F-02 同时影响 M-01 与 M-06），在每个相关模块文件"## 1. 需求来源"节中标注该 F-ID 的子条目归属。
- **关联表分散**：数据库表与模块非 1:1（一张表可能被多模块读写），所有 DDL 集中在 [M-10 §4](modules/M-10-tenant-middleware.md)。
- **格式準拠**：所有文档按 [`template.md`](template.md) 编写，遵守 IPA「共通フレーム2018」+「非機能要求グレード」。

## 11. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 无限画布 | 一种无固定边界、支持自由缩放与平移的二维可视化操作界面 | DOC-REQ-001 §2 |
| 节点 | 画布上代表一个独立处理单元的可视化元素 | DOC-REQ-001 §2 |
| 标准化 JSON | 本系统内部统一的数据交换 schema（NJSON） | DOC-REQ-001 §8 |
| 仿生模型 | 骨/血/神经/肌肉 四层架构隐喻 | DOC-ARCH-001 |
| Runtime | 基于 Rust 编写的本系统核心执行引擎 | DOC-REQ-001 §2 |
| 免安装 | 用户无需经过传统安装程序即可获取并运行 | DOC-REQ-001 §2、F-09 |
| 多租户 | 单一实例服务多个独立租户（tenant） | DOC-REQ-001 §3.3、F-17 |
| CRDT | Conflict-free Replicated Data Type，协作冲突解决算法 | DOC-MOD-011 §3.2 |
| RLS | Row-Level Security，PostgreSQL 行级安全 | DOC-MOD-010 §2.2 |
| モジュール | システムを構成する機能単位（本書では M-01~M-13） | DOC-DTL-001 §2 |

## 12. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. IPA「ソフトウェア開発データ白書」、独立行政法人情報処理推進機構、各年度版
4. JIS X 0160:2012「ソフトウェアライフサイクルプロセス」、日本工業標準調査会、2012年
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 要件定義書 v1.2.1」、2026-08-18（[DOC-REQ-001](legacy/requirements.md)）
6. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 基本設計書 v1.3.0」、2026-08-18（[DOC-BSC-001](legacy/basic-design.md)）
7. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
