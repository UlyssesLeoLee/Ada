# レビュー管理表（Review Schedule）

> **本文件の目的**：[DOC-ARCH-009 §7 ゲート](../architecture/08-workflow-overview.md) で定義した **G0〜G11 の全レビュー** の計画表。参加者、期日、判定基準を一覧化。  
> 関連 IPA 工程: 139（レビュー管理）。

> **ドキュメントID**：DOC-MGT-REV-001
> **文書分類**：管理文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/management/README.md`](README.md)
> **下位文書**：各 [DOC-TPL-REV §A.X](../templates/01-reviews.md)
> **関連文書**：[`docs/architecture/08-workflow-overview.md` §7](../architecture/08-workflow-overview.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 139 に対応、12 ゲート計画） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. レビューゲート一覧
2. 詳細スケジュール
3. 参加者
4. 完了基準
5. 監査ポイント
6. 用語集
7. 参考文献

---

## 1. レビューゲート一覧

| Gate | 名称 | IPA 工程 | テンプレート | 現在の状態 |
|---|---|---|---|---|
| G0 | PJ 立上げ判定 | 05 | — | 🟡 |
| G1 | 要件ベースライン化 | 20, 21 | [§A.1 RD Review](../templates/01-reviews.md#a1-要件レビューチェックリストipa-工程-20--g1) | 🟡 |
| G2 | BD Review | 41 | [§A.2 BD Review](../templates/01-reviews.md#a2-基本設計レビューチェックリストipa-工程-41--g2) | ⚪ |
| G3 | DD Review | 52 | [§A.3 DD Review](../templates/01-reviews.md#a3-詳細設計レビューチェックリストipa-工程-52--g3) | ⚪ |
| G4 | 実装着手判定 | 53-58 | [§A.7 GO/NO-GO](../templates/01-reviews.md#a7-リリース-gono-go-判定書ipa-工程-103--g10) (流用) | ⚪ |
| G5 | UT 完了 | 65 | [§A.5 ST 完了](../templates/01-reviews.md#a5-システム試験完了承認書ipa-工程-89--g7) (流用) | ⚪ |
| G6 | IT 完了 | 75 | — | ⚪ |
| G7 | ST 完了 | 89 | [§A.5](../templates/01-reviews.md#a5-システム試験完了承認書ipa-工程-89--g7) | ⚪ |
| G8 | 受入判定 | 94 | [§A.6 受入判定](../templates/01-reviews.md#a6-受入判定書ipa-工程-94--g8) | ⚪ |
| G9 | 移行判定 | 101 | — | ⚪ |
| G10 | Go-Live | 103 | [§A.7 GO/NO-GO](../templates/01-reviews.md#a7-リリース-gono-go-判定書ipa-工程-103--g10) | ⚪ |
| G11 | PJ 完了 | 145 | [§A.8 PJ 完了](../templates/01-reviews.md#a8-プロジェクト完了判定書ipa-工程-145--g11) | ⚪ |

## 2. 詳細スケジュール

| Gate | 計画日 | 参加者 | 必要日数 | 議題 | 判定基準 |
|---|---|---|---|---|---|
| G0 | 2026-08-19 | PM, PO, 経営 | 0.5 | PJ 立上げ合意 | システム化計画承認、予算確保、PM 任命 |
| G1 | 2026-08-19 | PO, PM, アーキ, SecO | 1 | 要件 RD Review + Baseline | 全 12 カテゴリ Pass、NF 網羅 100% |
| G2 | TBD | アーキ, PM, 外部有識者 | 2 | 基本設計 Review | 14 カテゴリ Pass、NF ≥ 90% |
| G3 | TBD | アーキ, テックリード, PM | 2 | 詳細設計 Review | 13 カテゴリ Pass、NF ≥ 95% |
| G4 | TBD | PM, アーキ, テックリード | 1 | 実装着手判定 | [DOC-ARCH-008 §8](../architecture/07-qa-register.md) 全 Pass |
| G5 | TBD | QA, テックリード | 1 | UT 完了 | カバレッジ ≥ 80% |
| G6 | TBD | QA, アーキ | 2 | IT 完了 | 47 ケース全合格 |
| G7 | TBD | QA, PM, SRE | 3 | ST 完了 | 100 ケース全合格、SLA 99.9% |
| G8 | TBD | PO, Biz 代表 | 2 | 受入判定 | UAT 8 ケース全合格 |
| G9 | TBD | PM, SRE, PO | 1 | 移行判定 | リハーサル 2 回成功 |
| G10 | TBD | PM, PO, SRE | 0.5 | Go-Live 判定 | 全 Smoke 合格 |
| G11 | TBD | PO, PM, 経営 | 1 | PJ 完了判定 | 全 10 カテゴリ Pass |

## 3. 参加者（マトリクス）

| Gate | PO | PM | アーキ | テック | Dev | DBA | SRE | SecO | QA | 外部 |
|---|---|---|---|---|---|---|---|---|---|---|
| G0 | A | R | C | I | I | I | I | I | I | I |
| G1 | A | R | R | C | I | C | C | R | C | I |
| G2 | C | A | R | R | C | C | C | C | C | A |
| G3 | I | A | R | R | C | C | C | C | C | I |
| G4 | A | R | R | R | I | I | I | I | I | I |
| G5 | I | C | C | A | R | I | I | I | R | I |
| G6 | I | C | C | A | C | C | C | C | R | I |
| G7 | I | A | C | A | C | C | R | R | R | C |
| G8 | A | R | C | C | C | I | I | I | C | I |
| G9 | A | R | C | C | C | R | R | I | C | I |
| G10 | A | R | C | C | C | C | R | C | C | I |
| G11 | A | R | C | C | I | I | I | I | I | I |

凡例：A = Accountable、R = Responsible、C = Consulted、I = Informed

## 4. 完了基準

- 全 12 ゲートの計画が立つ
- 参加者確定
- テンプレート整備済（[DOC-TPL-REV §A.1〜§A.8](../templates/01-reviews.md)）
- 各ゲート前にプリチェック実施

## 5. 監査ポイント

- 全 Gate レビュー記録を [DOC-TPL-REV §A.X](../templates/01-reviews.md) で保管
- 月次でスケジュール見直し
- 遅延時の代替案を事前準備

## 6. 用語集

| 用語 | 説明 |
|---|---|
| Gate | フェーズ通過判定の節目 |
| 参加者マトリクス | RACI マトリクス |
| プリチェック | レビュー前の事前確認 |

## 7. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
2. PMBOK Guide 第 7 版、Project Management Institute、2021 年
3. Ada プロジェクトチーム「[DOC-ARCH-009 §7 ゲート](../architecture/08-workflow-overview.md)」、2026-08-20

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
