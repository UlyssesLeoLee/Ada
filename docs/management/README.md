# プロジェクト管理ドキュメント（Project Management Documents）

> **本ディレクトリの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) §5.15（管理プロセス、IPA 工程 131-144）に対応する **4 種類の管理ドキュメント** を提供する。  
> プロジェクト計画、成果物管理、レビュー管理、スコープ管理、コミュニケーション計画を一元化する。

> **ドキュメントID**：DOC-MGT-INDEX
> **文書分類**：管理文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)
> **下位文書**：
> - [`docs/management/01-deliverable-list.md`](01-deliverable-list.md)（DOC-MGT-DLV-001）— 工程 138
> - [`docs/management/02-review-schedule.md`](02-review-schedule.md)（DOC-MGT-REV-001）— 工程 139
> - [`docs/management/03-scope-statement.md`](03-scope-statement.md)（DOC-MGT-SCP-001）— 工程 143
> - [`docs/management/04-communication-plan.md`](04-communication-plan.md)（DOC-MGT-COM-001）— 工程 140
> **関連文書**：
> - [`docs/upstream/01-pj-charter.md`](../upstream/01-pj-charter.md)（DOC-UP-001）
> - [`docs/CHANGELOG.md`](../CHANGELOG.md)
> - [`docs/templates/03-process-management.md`](../templates/03-process-management.md)（DOC-TPL-PRC）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - PMBOK Guide 第 7 版

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（管理 4 ドキュメント） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. ドキュメント一覧
2. IPA 工程マッピング
3. 関連テンプレート
4. 用語集
5. 参考文献

---

## 1. ドキュメント一覧

| DOC-ID | ファイル | タイトル | 内容 |
|---|---|---|---|
| DOC-MGT-DLV-001 | [01-deliverable-list.md](01-deliverable-list.md) | 成果物一覧 | 設計書・コード・運用物の全リスト |
| DOC-MGT-REV-001 | [02-review-schedule.md](02-review-schedule.md) | レビュー管理表 | 全 Gate と関係者、期日 |
| DOC-MGT-SCP-001 | [03-scope-statement.md](03-scope-statement.md) | スコープベースライン | In/Out-of-Scope 正式版 |
| DOC-MGT-COM-001 | [04-communication-plan.md](04-communication-plan.md) | コミュニケーション計画 | 会議体、頻度、参加者 |

## 2. IPA 工程マッピング

| ドキュメント | IPA 工程 | 役割 |
|---|---|---|
| 01-deliverable-list | 138 成果物管理 | 全成果物の追跡 |
| 02-review-schedule | 139 レビュー管理 | 全 Gate レビューの計画 |
| 03-scope-statement | 143 スコープ管理 | 正式なスコープベースライン |
| 04-communication-plan | 140 会議・報告 | ステークホルダとの接点 |

## 3. 関連テンプレート

| テンプレート | 用途 |
|---|---|
| [DOC-TPL-PRC §A.1 WBS](../templates/03-process-management.md#a1-wbs-テンプレートipa-工程-132) | 詳細 WBS |
| [DOC-TPL-PRC §A.2 進捗レポート](../templates/03-process-management.md#a2-進捗レポートテンプレートipa-工程-133) | 週次進捗 |
| [DOC-TPL-PRC §A.3 課題管理台帳](../templates/03-process-management.md#a3-課題管理台帳ipa-工程-134) | 課題追跡 |
| [DOC-TPL-PRC §A.4 リスク管理台帳](../templates/03-process-management.md#a4-リスク管理台帳ipa-工程-135) | リスク追跡 |
| [DOC-TPL-PRC §A.5 会議議事録](../templates/03-process-management.md#a5-会議アジェンダ--議事録テンプレートipa-工程-140) | 議事録 |
| [DOC-TPL-PRC §A.6 工数管理表](../templates/03-process-management.md#a6-工数管理表ipa-工程-141) | 工数追跡 |
| [DOC-TPL-PRC §A.7 コスト管理表](../templates/03-process-management.md#a7-コスト管理表ipa-工程-142) | コスト追跡 |
| [DOC-TPL-CHG §A.1 CR](../templates/06-change-management.md#a1-変更要求チケットipa-工程-118) | 変更要求 |

## 4. 用語集

| 用語 | 説明 |
|---|---|
| Deliverable | 成果物 |
| Scope | スコープ（作業範囲） |
| Gate | フェーズ通過判定の節目 |
| RACI | Responsible / Accountable / Consulted / Informed |
| Baseline | ベースライン（基準点） |

## 5. 参考文献

1. PMBOK Guide 第 7 版、Project Management Institute、2021 年
2. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
3. Ada プロジェクトチーム「[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md)」、2026-08-20

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
