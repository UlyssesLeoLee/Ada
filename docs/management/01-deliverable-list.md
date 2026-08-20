# 成果物一覧（Deliverable List）

> **本文件の目的**：プロジェクトで作成・維持する**全成果物の正式なリスト**。バージョン、保管場所、責任者を記録する。  
> 関連 IPA 工程: 138（成果物管理）。

> **ドキュメントID**：DOC-MGT-DLV-001
> **文書分類**：管理文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/management/README.md`](README.md)
> **関連文書**：
> - [`docs/CHANGELOG.md`](../CHANGELOG.md)
> - [`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)
> - [`docs/README.md`](../README.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 138 に対応、47 成果物） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. メタ文書
2. アーキテクチャ文書
3. モジュール文書
4. API 文書
5. テスト文書
6. テンプレート
7. 上流・要件・管理・業務文書
8. コード成果物
9. 運用成果物
10. 完了基準
11. 用語集
12. 参考文献

---

## 1. メタ文書（3 件）

| ID | ファイル | v | 担当 | 保管 |
|---|---|---|---|---|
| DOC-INDEX-001 | [README.md](../README.md) | v1.6.0 | PM | docs/ |
| DOC-CHG-001 | [CHANGELOG.md](../CHANGELOG.md) | v1.9.0 | PM | docs/ |
| DOC-TPL-001 | [template.md](../template.md) | v1.0.0 | PM | docs/ |

## 2. アーキテクチャ文書（9 件）

| ID | ファイル | v | 担当 | 保管 |
|---|---|---|---|---|
| DOC-ARCH-001 | [00-anatomy-model.md](../architecture/00-anatomy-model.md) | v1.0.0 | アーキ | docs/architecture/ |
| DOC-ARCH-002 | [01-tech-stack.md](../architecture/01-tech-stack.md) | v1.0.0 | アーキ | docs/architecture/ |
| DOC-ARCH-003 | [02-deployment.md](../architecture/02-deployment.md) | v1.0.0 | SRE | docs/architecture/ |
| DOC-ARCH-004 | [03-cross-cutting-risks.md](../architecture/03-cross-cutting-risks.md) | v1.0.0 | SecO | docs/architecture/ |
| DOC-ARCH-005 | [04-atomic-deployment.md](../architecture/04-atomic-deployment.md) | v1.0.0 | SRE | docs/architecture/ |
| DOC-ARCH-006 | [05-admin-operations-ui.md](../architecture/05-admin-operations-ui.md) | v1.0.0 | FE | docs/architecture/ |
| DOC-ARCH-007 | [06-rust-tech-selection.md](../architecture/06-rust-tech-selection.md) | v1.0.0 | アーキ | docs/architecture/ |
| DOC-ARCH-008 | [07-qa-register.md](../architecture/07-qa-register.md) | v1.0.0 | QA | docs/architecture/ |
| DOC-ARCH-009 | [08-workflow-overview.md](../architecture/08-workflow-overview.md) | v1.1.0 | PM | docs/architecture/ |

## 3. モジュール文書（16 件）

| ID | ファイル | v | 担当 | 保管 |
|---|---|---|---|---|
| DOC-MOD-001 | [M-01-acquisition-adapter.md](../modules/M-01-acquisition-adapter.md) | v1.2.0 | Dev | docs/modules/ |
| DOC-MOD-002 | [M-02-normalizer.md](../modules/M-02-normalizer.md) | v1.1.0 | Dev | docs/modules/ |
| DOC-MOD-003 | [M-03-data-flow-engine.md](../modules/M-03-data-flow-engine.md) | v1.1.0 | Dev | docs/modules/ |
| DOC-MOD-004 | [M-04-orchestration-engine.md](../modules/M-04-orchestration-engine.md) | v1.1.0 | Dev | docs/modules/ |
| DOC-MOD-005 | [M-05-control-flow-executor.md](../modules/M-05-control-flow-executor.md) | v1.1.0 | Dev | docs/modules/ |
| DOC-MOD-006 | [M-06-node-runtime-plugin-sdk.md](../modules/M-06-node-runtime-plugin-sdk.md) | v1.1.0 | Dev | docs/modules/ |
| DOC-MOD-007 | [M-07-debug-service.md](../modules/M-07-debug-service.md) | v1.1.0 | Dev | docs/modules/ |
| DOC-MOD-008 | [M-08-trigger-service.md](../modules/M-08-trigger-service.md) | v1.1.0 | Dev | docs/modules/ |
| DOC-MOD-009 | [M-09-exporter.md](../modules/M-09-exporter.md) | v1.1.0 | Dev | docs/modules/ |
| DOC-MOD-010 | [M-10-tenant-middleware.md](../modules/M-10-tenant-middleware.md) | v1.2.0 | DBA | docs/modules/ |
| DOC-MOD-011 | [M-11-rbac-collab.md](../modules/M-11-rbac-collab.md) | v1.1.0 | SecO | docs/modules/ |
| DOC-MOD-012 | [M-12-canvas-editor-frontend.md](../modules/M-12-canvas-editor-frontend.md) | v1.1.0 | FE | docs/modules/ |
| DOC-MOD-013 | [M-13-api-gateway.md](../modules/M-13-api-gateway.md) | v1.1.0 | Dev | docs/modules/ |
| DOC-MOD-014 | [M-14-module-registry.md](../modules/M-14-module-registry.md) | v1.2.0 | Dev | docs/modules/ |
| DOC-MOD-015 | [M-15-central-event-bus.md](../modules/M-15-central-event-bus.md) | v1.2.0 | Dev | docs/modules/ |
| DOC-MOD-016 | [M-16-cluster-coordinator.md](../modules/M-16-cluster-coordinator.md) | v1.2.0 | Dev | docs/modules/ |

## 4. API 文書（6 件）

| ID | ファイル | v | 担当 | 保管 |
|---|---|---|---|---|
| DOC-API-001 | [rest-endpoints.md](../api/rest-endpoints.md) | v1.0.0 | Dev | docs/api/ |
| DOC-API-002 | [websocket-events.md](../api/websocket-events.md) | v1.0.0 | Dev | docs/api/ |
| DOC-API-003 | [error-codes.md](../api/error-codes.md) | v1.0.0 | Dev | docs/api/ |
| DOC-API-004 | [admin-modules.md](../api/admin-modules.md) | v1.0.0 | Dev | docs/api/ |
| DOC-API-005 | [admin-events.md](../api/admin-events.md) | v1.0.0 | Dev | docs/api/ |
| DOC-API-006 | [admin-cluster.md](../api/admin-cluster.md) | v1.0.0 | Dev | docs/api/ |

## 5. テスト文書（4 件）

| ID | ファイル | v | 担当 | 保管 |
|---|---|---|---|---|
| DOC-TST-INDEX | [tests/README.md](../tests/README.md) | v1.0.0 | QA | docs/tests/ |
| DOC-TST-001 | [UT-design.md](../tests/UT-design.md) | v1.0.0 | QA | docs/tests/ |
| DOC-TST-002 | [IT-design.md](../tests/IT-design.md) | v1.0.0 | QA | docs/tests/ |
| DOC-TST-003 / DOC-ACC-001 | [ST-design.md](../tests/ST-design.md) | v1.0.0 | QA | docs/tests/ |

## 6. テンプレート（9 件）

| ID | ファイル | 担当 | 保管 |
|---|---|---|---|
| DOC-TPL-INDEX | [templates/README.md](../templates/README.md) | PM | docs/templates/ |
| DOC-TPL-REV | [01-reviews.md](../templates/01-reviews.md) | PM | docs/templates/ |
| DOC-TPL-TST | [02-tests-execution.md](../templates/02-tests-execution.md) | QA | docs/templates/ |
| DOC-TPL-PRC | [03-process-management.md](../templates/03-process-management.md) | PM | docs/templates/ |
| DOC-TPL-RBK | [04-runbooks.md](../templates/04-runbooks.md) | SRE | docs/templates/ |
| DOC-TPL-OPS | [05-operations.md](../templates/05-operations.md) | SRE | docs/templates/ |
| DOC-TPL-CHG | [06-change-management.md](../templates/06-change-management.md) | Dev | docs/templates/ |
| DOC-TPL-QUA | [07-quality.md](../templates/07-quality.md) | QA | docs/templates/ |
| DOC-TPL-CLO | [08-closure.md](../templates/08-closure.md) | PM | docs/templates/ |

## 7. 上流・要件・管理・業務文書（23 件）

### 7.1 上流（8 件）

| ID | ファイル | 担当 | 保管 |
|---|---|---|---|
| DOC-UP-INDEX | [upstream/README.md](../upstream/README.md) | PM | docs/upstream/ |
| DOC-UP-001〜008 | [upstream/01〜08](../upstream/01-pj-charter.md) | PM / Biz / アーキ | docs/upstream/ |

### 7.2 要件（10 件）

| ID | ファイル | 担当 | 保管 |
|---|---|---|---|
| DOC-REQ-INDEX | [requirements/README.md](../requirements/README.md) | アーキ | docs/requirements/ |
| DOC-REQ-UR-001〜MIG-001 | [requirements/01〜10](../requirements/01-ur-user-requirements.md) | アーキ / PO | docs/requirements/ |

### 7.3 管理（4 件）

| ID | ファイル | 担当 | 保管 |
|---|---|---|---|
| DOC-MGT-INDEX | [management/README.md](README.md) | PM | docs/management/ |
| DOC-MGT-DLV-001 | [01-deliverable-list.md](01-deliverable-list.md) | PM | docs/management/ |
| DOC-MGT-REV-001 | [02-review-schedule.md](02-review-schedule.md) | PM | docs/management/ |
| DOC-MGT-SCP-001 | [03-scope-statement.md](03-scope-statement.md) | PM | docs/management/ |
| DOC-MGT-COM-001 | [04-communication-plan.md](04-communication-plan.md) | PM | docs/management/ |

### 7.4 業務（1 件）

| ID | ファイル | 担当 | 保管 |
|---|---|---|---|
| DOC-BIZ-SCN-001 | [business/01-scenario-catalog.md](../business/01-scenario-catalog.md) | Biz | docs/business/ |

## 8. コード成果物

| 成果物 | 内容 | 担当 | 保管 |
|---|---|---|---|
| 16 crate + 2 共通 | Rust ソース | Dev × 16 | GitHub |
| 11 テーブル DDL | PostgreSQL | DBA | migrations/ |
| 6 PL/pgSQL 存過 | 関数 | DBA | migrations/ |
| 18 Docker イメージ | コンテナ | SRE | Container Registry |
| K8s manifest | YAML | SRE | manifests/ |
| Terraform | HCL | SRE | terraform/ |

## 9. 運用成果物

| 成果物 | 内容 | 担当 | 保管 |
|---|---|---|---|
| 全 Runbook 派生版 | 11 環境別 | SRE | docs/runbooks/ |
| 監視設定 | Prometheus | SRE | Prometheus |
| ログ基盤 | Loki / ELK | SRE | — |
| アラート設定 | Alertmanager | SRE | Prometheus |
| Backup 設定 | pg_dump + WAL | DBA | S3 |
| 訓練記録 | DR 訓練、Incident 訓練 | SRE | docs/records/ |

## 10. 完了基準

- 全成果物に ID / バージョン / 担当 / 保管場所が定義
- 100% IPA 準拠（[template.md](../template.md) 参照）
- Git 管理（[DOC-ARCH-007 §16.4](../architecture/06-rust-tech-selection.md)）

## 11. 用語集

| 用語 | 説明 |
|---|---|
| Deliverable | 成果物（顧客・運用に引き渡す物） |
| 中間成果物 | 開発過程で作成するが、最終的に破棄される物 |

## 12. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
2. PMBOK Guide 第 7 版、Project Management Institute、2021 年

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
