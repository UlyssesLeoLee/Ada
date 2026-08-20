# 超上流工程ドキュメント（Upstream Documents）

> **本ディレクトリの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) §5.1（超上流プロセス、IPA 工程 01-09）および §5.15（管理プロセス 131-144 の一部）に対応する **8 種類の超上流工程ドキュメント** を提供する。  
> 経営要求確認から新業務設計まで、上流工程の正式な記録を残す。

> **ドキュメントID**：DOC-UP-INDEX
> **文書分類**：上流工程文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)（DOC-ARCH-009）
> **下位文書**：
> - [`docs/upstream/01-pj-charter.md`](01-pj-charter.md)（DOC-UP-001）
> - [`docs/upstream/02-stakeholder-register.md`](02-stakeholder-register.md)（DOC-UP-002）
> - [`docs/upstream/03-as-is-business.md`](03-as-is-business.md)（DOC-UP-003）
> - [`docs/upstream/04-as-is-system.md`](04-as-is-system.md)（DOC-UP-004）
> - [`docs/upstream/05-issue-list.md`](05-issue-list.md)（DOC-UP-005）
> - [`docs/upstream/06-to-be-business.md`](06-to-be-business.md)（DOC-UP-006）
> - [`docs/upstream/07-to-be-system.md`](07-to-be-system.md)（DOC-UP-007）
> - [`docs/upstream/08-initial-risk-assessment.md`](08-initial-risk-assessment.md)（DOC-UP-008）
> **関連文書**：
> - [`docs/requirements/README.md`](../requirements/README.md)（DOC-REQ-INDEX）
> - [`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)
> - [`docs/architecture/07-qa-register.md`](../architecture/07-qa-register.md)
> - [`docs/legacy/requirements.md`](../legacy/requirements.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」第 5 章「プロセス」
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（超上流 8 工程ドキュメント） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 使い方
2. ドキュメント一覧と IPA 工程マッピング
3. 位置付け
4. フェーズ間のデータフロー
5. 用語集
6. 参考文献

---

## 1. 使い方

### 1.1 いつ使うか

| フェーズ | いつ | 担当 |
|---|---|---|
| 経営要求確認（01）〜 企画（04） | プロジェクト検討開始時 | PO + 経営層 |
| PJ 立上げ（05） | システム化計画承認時 | PM 任命 |
| 現行業務/システム調査（06-07） | PJ 開始直後 | Biz + アーキ |
| 課題分析（08） | As-Is 調査完了後 | Biz + PO + アーキ |
| 新業務設計（09） | 課題分析完了後 | Biz + PO + アーキ |

### 1.2 読み順

1. [§1 PJ Charter](01-pj-charter.md) — プロジェクト全体の目的・スコープ・体制
2. [§2 ステークホルダ登録簿](02-stakeholder-register.md) — 関係者の責務・関心事
3. [§3-4 As-Is](03-as-is-business.md) / [§4 As-Is システム](04-as-is-system.md) — 現状把握
4. [§5 課題一覧](05-issue-list.md) — 解決すべき問題
5. [§6-7 To-Be](06-to-be-business.md) / [§7 To-Be システム](07-to-be-system.md) — あるべき姿
6. [§8 初期リスク評価](08-initial-risk-assessment.md) — 想定リスクと対応

### 1.3 派生版の作成

- 各ドキュメントは **1 PJ = 1 派生版**
- 改訂時は **バージョン番号 + 改訂履歴** を更新
- PO + PM 合意で承認

---

## 2. ドキュメント一覧と IPA 工程マッピング

| DOC-ID | ファイル | タイトル | 対応 IPA 工程 | NF 区分 |
|---|---|---|---|---|
| DOC-UP-001 | [01-pj-charter.md](01-pj-charter.md) | プロジェクト憲章 | 05, 131 | 全て |
| DOC-UP-002 | [02-stakeholder-register.md](02-stakeholder-register.md) | ステークホルダ登録簿 | 05, 140 | — |
| DOC-UP-003 | [03-as-is-business.md](03-as-is-business.md) | 現行業務フロー（As-Is） | 06 | — |
| DOC-UP-004 | [04-as-is-system.md](04-as-is-system.md) | 現行システム構成（As-Is） | 07 | — |
| DOC-UP-005 | [05-issue-list.md](05-issue-list.md) | 課題一覧 | 08 | — |
| DOC-UP-006 | [06-to-be-business.md](06-to-be-business.md) | 新業務フロー（To-Be） | 09 | — |
| DOC-UP-007 | [07-to-be-system.md](07-to-be-system.md) | 新システム構成（To-Be） | 09, 22, 24 | [NF-AVA\|PER\|OPS\|SEC] |
| DOC-UP-008 | [08-initial-risk-assessment.md](08-initial-risk-assessment.md) | 初期リスク評価 | 08, 135 | — |

---

## 3. 位置付け

```
[経営要求 01-04]
       ↓
[PJ Charter DOC-UP-001] ← スコープ・体制・期間・予算の正式合意
       ↓
[ステークホルダ DOC-UP-002] ← 関係者マッピング
       ↓
[As-Is 業務 DOC-UP-003] + [As-Is システム DOC-UP-004] ← 現状把握
       ↓
[課題一覧 DOC-UP-005] ← 解決すべき問題
       ↓
[To-Be 業務 DOC-UP-006] + [To-Be システム DOC-UP-007] ← あるべき姿
       ↓
[初期リスク DOC-UP-008] ← 想定リスクと対応
       ↓
[要件定義書 legacy/requirements.md]
       ↓
[要件细分 docs/requirements/]
       ↓
[基本設計 docs/architecture/, docs/modules/]
       ...
```

---

## 4. フェーズ間のデータフロー

| 入力元 | 出力先 | 引き渡しデータ |
|---|---|---|
| 経営要求 | [PJ Charter](01-pj-charter.md) | 経営目標、予算上限、期限制約 |
| [PJ Charter](01-pj-charter.md) | [ステークホルダ](02-stakeholder-register.md) | 想定関係者 |
| [As-Is 業務](03-as-is-business.md) | [課題一覧](05-issue-list.md) | 業務上の非効率・問題 |
| [As-Is システム](04-as-is-system.md) | [課題一覧](05-issue-list.md) | システム制約・技術的負債 |
| [課題一覧](05-issue-list.md) | [To-Be 業務](06-to-be-business.md) | 解決すべき課題 |
| [To-Be 業務](06-to-be-business.md) | [To-Be システム](07-to-be-system.md) | 必要なシステム機能 |
| [To-Be システム](07-to-be-system.md) | [初期リスク](08-initial-risk-assessment.md) | 新規導入リスク |
| 全上流 | 要件定義書 | 確定した PJ スコープ |

---

## 5. 用語集

| 用語 | 説明 | 出典 |
|---|---|---|
| 超上流 | 経営要求から要件定義までの上流工程 | IPA 共通フレーム |
| As-Is | 現状（業務・システム） | Biz モデリング |
| To-Be | あるべき姿（業務・システム） | Biz モデリング |
| ステークホルダ | プロジェクト関係者 | PMBOK |
| PJ Charter | プロジェクト憲章（正式な発足文書） | PMBOK |
| 課題 | 解決すべき問題点 | 本書 |
| 初期リスク | プロジェクト初期に識別されたリスク | PMBOK |
| RACI | Responsible / Accountable / Consulted / Informed | PMBOK |

---

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018 年 3 月
2. PMBOK Guide 第 7 版、Project Management Institute、2021 年
3. BPMN 2.0、OMG、2011 年
4. Ada プロジェクトチーム「[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md)」、2026-08-20
5. Ada プロジェクトチーム「[DOC-REQ-001 要件定義書](../legacy/requirements.md)」、2026-08-18

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
