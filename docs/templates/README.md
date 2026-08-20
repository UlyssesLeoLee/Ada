# 工程別テンプレート集（Engineering Templates Compendium）

> **本ディレクトリの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) の ⚪（未着手）/🚧（計画中）工程で必要となる **レビュー記録・試験ログ・運用 Runbook・変更管理記録・終結成果物** のテンプレートを一元提供する。  
> テンプレートは **再利用可能な空フォーム** であり、実施時に記入して具体的な証跡とする。  
> 開発着手・運用開始・保守実行のたびに本ディレクトリのテンプレートから派生して証跡を作成する。

> **ドキュメントID**：DOC-TPL-INDEX
> **文書分類**：テンプレート集
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)（DOC-ARCH-009）、[`docs/template.md`](../template.md)（DOC-TPL-001、IPA 準拠のドキュメント標準テンプレート）
> **下位文書**：
> - `docs/templates/01-reviews.md`（DOC-TPL-REV）
> - `docs/templates/02-tests-execution.md`（DOC-TPL-TST）
> - `docs/templates/03-process-management.md`（DOC-TPL-PRC）
> - `docs/templates/04-runbooks.md`（DOC-TPL-RBK）
> - `docs/templates/05-operations.md`（DOC-TPL-OPS）
> - `docs/templates/06-change-management.md`（DOC-TPL-CHG）
> - `docs/templates/07-quality.md`（DOC-TPL-QUA）
> - `docs/templates/08-closure.md`（DOC-TPL-CLO）
> **関連文書**：
> - 全モジュール文書（DOC-MOD-001〜016）
> - 全 API 文書（DOC-API-001〜006）
> - 全テスト文書（DOC-TST-001〜003 / DOC-ACC-001）
> - [`docs/architecture/07-qa-register.md`](../architecture/07-qa-register.md)（DOC-ARCH-008）
> - [`docs/CHANGELOG.md`](../CHANGELOG.md)（DOC-CHG-001）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - IPA「非機能要求グレード2018」
> - JIS X 0160:2012
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（DOC-ARCH-009 の ⚪/🚧 工程に対応する 50+ テンプレートを 8 カテゴリで集約） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 使い方
2. テンプレート一覧
3. IPA 工程 ↔ テンプレート対応マトリクス
4. 命名・保管・版管理ルール
5. 用語集
6. 参考文献

---

## 1. 使い方

### 1.1 3 つの使い方

1. **新規レビュー・試験の開始時**：[§3 マトリクス](#3-ipa-工程--テンプレート対応マトリクス) で該当テンプレートを選び、コピーして記入
2. **新規運用 Runbook の作成時**：[`04-runbooks.md`](04-runbooks.md) から該当パターンを複製して環境固有値を埋める
3. **変更管理・保守記録時**：[`06-change-management.md`](06-change-management.md) のテンプレートを 1 件 = 1 チケットで対応

### 1.2 カスタマイズ指針

- **列は追加可**：組織固有の項目（承認者、コストコード 等）は自由に追加
- **列は削除不可**：IPA 監査・NF タグ・関連文書欄は **必ず残す**（監査証跡）
- **テンプレートの改変禁止**：本ディレクトリ内のテンプレート自体を改変する場合は [DOC-ARCH-009](../architecture/08-workflow-overview.md) §7 の **G2 レビュー相当の承認** を経る
- **派生版の保管**：テンプレートから派生した実記録は `docs/records/` 配下に保存（将来ディレクトリ）

### 1.3 記入時の必須項目

| 項目 | 説明 | 必須/任意 |
|---|---|---|
| 文書 ID | 派生版の DOC-ID（テンプレ DOC-ID + 連番） | 必須 |
| 起票日 / 起票者 | いつ誰が起票したか | 必須 |
| 関連 IPA 工程 | [DOC-ARCH-009](../architecture/08-workflow-overview.md) の 150 工程のうち該当する番号 | 必須 |
| 関連要件（F-ID） | トレーサビリティ確保のため [DOC-REQ-001](../legacy/requirements.md) §9 の F-ID を引用 | 必須 |
| 関連 NF 区分 | [NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV] のうち該当区分 | 必須 |
| 関連リスク ID | [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) または [DOC-ARCH-008](../architecture/07-qa-register.md) のリスク/Q-A ID | 推奨 |
| 関連モジュール (M-ID) | DOC-MOD-001〜016 のいずれか | 推奨 |
| ステータス | 記入中 / レビュー中 / 承認済 / 却下 | 必須 |
| 承認者 | IPA ゲート判定者 | 必須 |
| 監査証跡リンク | GitHub PR / Slack ログ / 議事録リンク | 推奨 |

---

## 2. テンプレート一覧

### 2.1 カテゴリ別件数

| カテゴリ | ファイル | DOC-ID | テンプレート数 | 対応 IPA 工程 |
|---|---|---|---|---|
| ① レビュー記録 | [`01-reviews.md`](01-reviews.md) | DOC-TPL-REV | 8 | 20, 41, 52, 61, 89, 94, 103, 145 |
| ② 試験実施ログ | [`02-tests-execution.md`](02-tests-execution.md) | DOC-TPL-TST | 11 | 60, 62, 63, 64, 65, 68, 69, 70, 71, 72, 73, 74, 75, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 92, 93, 95 |
| ③ プロセス管理 | [`03-process-management.md`](03-process-management.md) | DOC-TPL-PRC | 7 | 132, 133, 134, 135, 140, 141, 142 |
| ④ Runbook / 構築 | [`04-runbooks.md`](04-runbooks.md) | DOC-TPL-RBK | 10 | 53, 97, 98, 99, 100, 101, 105, 106, 107, 108 |
| ⑤ 運用管理 | [`05-operations.md`](05-operations.md) | DOC-TPL-OPS | 9 | 109, 110, 111, 112, 113, 114, 115, 116, 117 |
| ⑥ 変更・保守管理 | [`06-change-management.md`](06-change-management.md) | DOC-TPL-CHG | 9 | 118, 119, 120, 121, 122, 123, 124, 125, 126 |
| ⑦ 品質管理 | [`07-quality.md`](07-quality.md) | DOC-TPL-QUA | 3 | 128, 129, 130 |
| ⑧ 終結 | [`08-closure.md`](08-closure.md) | DOC-TPL-CLO | 5 | 146, 147, 148, 149, 150 |
| **合計** | — | — | **62** | **80 工程**（150 中） |

### 2.2 テンプレートの読み方

各テンプレートは以下の構造を持つ：

```
§A テンプレート ID と名称
  - 適用 IPA 工程
  - 目的
  - 記入者
  - 記入タイミング
  - 関連ドキュメント
  - NF タグ

§B テンプレート本体
  - ヘッダ部（ID, 日付, 起票者, 承認者）
  - 記入フィールド（テーブル / フォーム形式）
  - 監査証跡欄

§C 完了基準
  - 必須記入項目
  - 承認の根拠
  - 派生版保管先
```

---

## 3. IPA 工程 ↔ テンプレート対応マトリクス

> [DOC-ARCH-009](../architecture/08-workflow-overview.md) §5 の ⚪（未着手）工程 76 件のうち、62 件を本テンプレートのいずれかでカバーする。残 14 件は **不要**（`—` で完了する工程）または **設計で十分**（追加テンプレ不要）。

| IPA # | 工程名 | 状態 | テンプレート | ファイル |
|---|---|---|---|---|
| 20 | 要件レビュー | ⚪ | 要件レビューチェックリスト | 01-reviews.md §A.1 |
| 41 | 基本設計レビュー | ⚪ | 基本設計レビューチェックリスト | 01-reviews.md §A.2 |
| 52 | 詳細設計レビュー | ⚪ | 詳細設計レビューチェックリスト | 01-reviews.md §A.3 |
| 53 | 開発環境構築 | ⚪ | 開発環境構築手順書 | 04-runbooks.md §A.1 |
| 60 | 単体試験仕様書作成 | 🟡 | 単体試験仕様書テンプレート | 02-tests-execution.md §A.1 |
| 61 | 単体試験レビュー | ⚪ | 単体試験レビューチェックリスト | 01-reviews.md §A.4 |
| 62 | 単体試験実施 | ⚪ | 単体試験実施ログ | 02-tests-execution.md §A.2 |
| 63 | 不具合修正 | ⚪ | 不具合修正記録 | 02-tests-execution.md §A.3 |
| 64 | 再試験 | ⚪ | 再試験記録 | 02-tests-execution.md §A.4 |
| 65 | 単体試験完了承認 | ⚪ | UT 完了承認書 | 02-tests-execution.md §A.5 |
| 68 | 結合試験環境構築 | ⚪ | 結合試験環境構築手順 | 04-runbooks.md §A.2 |
| 69 | 内部結合試験 | ⚪ | 内部結合試験実施ログ | 02-tests-execution.md §A.6 |
| 70 | 外部結合試験 | ⚪ | 外部結合試験実施ログ | 02-tests-execution.md §A.7 |
| 71 | API 結合試験 | ⚪ | API 結合試験実施ログ | 02-tests-execution.md §A.8 |
| 72 | DB 結合試験 | ⚪ | DB 結合試験実施ログ | 02-tests-execution.md §A.9 |
| 73 | 外部システム連携試験 | ⚪ | 外部 IF 試験実施ログ | 02-tests-execution.md §A.10 |
| 74 | 障害・不具合対応 | ⚪ | 障害対応記録 | 02-tests-execution.md §A.11 |
| 75 | 回帰試験 | ⚪ | 回帰試験ログ | 02-tests-execution.md §A.12 |
| 78-88 | システム試験各種 | ⚪ | ST 実施ログ（機能/シナリオ/性能/負荷/ストレス/Sec/障害/復旧/BK/可用性/運用） | 02-tests-execution.md §A.13 |
| 89 | ST 完了承認 | ⚪ | ST 完了承認書 | 01-reviews.md §A.5 |
| 92 | ユーザー受入試験 | ⚪ | UAT 実施ログ | 02-tests-execution.md §A.14 |
| 93 | 業務シナリオ試験 | ⚪ | 業務シナリオ試験ログ | 02-tests-execution.md §A.15 |
| 94 | 受入判定 | ⚪ | 受入判定書 | 01-reviews.md §A.6 |
| 95 | 検収 | ⚪ | 検収書 | 02-tests-execution.md §A.16 |
| 97 | 移行手順作成 | ⚪ | 移行手順書 | 04-runbooks.md §A.3 |
| 98 | 移行リハーサル | ⚪ | 移行リハーサル記録 | 04-runbooks.md §A.4 |
| 99 | データ移行 | ⚪ | データ移行ログ | 04-runbooks.md §A.5 |
| 100 | システム移行 | ⚪ | システム移行ログ | 04-runbooks.md §A.6 |
| 101 | 移行結果確認 | ⚪ | 移行結果確認書 | 04-runbooks.md §A.7 |
| 103 | リリース判定 | ⚪ | リリース Go/No-Go 判定書 | 01-reviews.md §A.7 |
| 105 | 本番デプロイ | ⚪ | 本番デプロイ記録 | 04-runbooks.md §A.8 |
| 106 | 稼働確認 | ⚪ | Smoke Test 実施ログ | 04-runbooks.md §A.9 |
| 107 | サービス開始 | ⚪ | Go-Live 宣言書 | 04-runbooks.md §A.10 |
| 108 | 初期流動対応 | ⚪ | Hypercare 計画書 | 04-runbooks.md §A.11 |
| 109 | 運用引継ぎ | 🟡 | 運用引継ぎ書 | 05-operations.md §A.1 |
| 110 | システム監視 | 🟡 | 監視設定書 | 05-operations.md §A.2 |
| 111 | ジョブ管理 | 🟡 | ジョブ定義書 | 05-operations.md §A.3 |
| 112 | バックアップ | ⚪ | Backup スケジュール/ログ | 05-operations.md §A.4 |
| 113 | キャパシティ管理 | 🟡 | Capacity 管理表 | 05-operations.md §A.5 |
| 114 | インシデント管理 | ⚪ | Incident Response Runbook | 05-operations.md §A.6 |
| 115 | 障害管理 | ⚪ | Postmortem テンプレート | 05-operations.md §A.7 |
| 116 | 問題管理 | ⚪ | 問題管理台帳 | 05-operations.md §A.8 |
| 117 | 問い合わせ管理 | 🟡 | 問い合わせ対応記録 | 05-operations.md §A.9 |
| 118 | 変更要求 | 🟡 | 変更要求（CR）チケット | 06-change-management.md §A.1 |
| 119 | 影響分析 | 🟡 | 影響分析レポート | 06-change-management.md §A.2 |
| 120 | 変更管理 | 🟡 | 変更承認記録 | 06-change-management.md §A.3 |
| 121 | 構成管理 | 🟡 | 構成管理台帳 | 06-change-management.md §A.4 |
| 122 | パッチ適用 | 🟡 | Patch ログ | 06-change-management.md §A.5 |
| 123 | 脆弱性対応 | 🟡 | 脆弱性対応ログ | 06-change-management.md §A.6 |
| 124 | 改修 | 🟡 | 改修 PR テンプレ | 06-change-management.md §A.7 |
| 125 | 緊急改修 | 🟡 | Hotfix 手順書 | 06-change-management.md §A.8 |
| 126 | リグレッション | 🟡 | 回帰テストログ | 06-change-management.md §A.9 |
| 128 | 品質レビュー | 🚧 | 品質レビュー記録 | 07-quality.md §A.1 |
| 129 | 品質評価 | 🚧 | 品質評価レポート | 07-quality.md §A.2 |
| 130 | 品質監査 | 🟡 | 品質監査チェックリスト | 07-quality.md §A.3 |
| 132 | WBS 管理 | ⚪ | WBS テンプレート | 03-process-management.md §A.1 |
| 133 | 進捗管理 | 🚧 | 進捗レポート | 03-process-management.md §A.2 |
| 134 | 課題管理 | 🟡 | 課題管理台帳 | 03-process-management.md §A.3 |
| 135 | リスク管理 | 🟡 | リスク管理台帳 | 03-process-management.md §A.4 |
| 140 | 会議・報告 | ⚪ | 会議アジェンダ/議事録 | 03-process-management.md §A.5 |
| 141 | 工数管理 | ⚪ | 工数管理表 | 03-process-management.md §A.6 |
| 142 | コスト管理 | ⚪ | コスト管理表 | 03-process-management.md §A.7 |
| 145 | PJ 完了判定 | ⚪ | PJ 完了判定書 | 01-reviews.md §A.8 |
| 146 | 成果物引渡し | ⚪ | 成果物引渡し書 | 08-closure.md §A.1 |
| 147 | 完了報告 | ⚪ | 完了報告書 | 08-closure.md §A.2 |
| 148 | 振り返り | ⚪ | Retrospective 議事録 | 08-closure.md §A.3 |
| 149 | ナレッジ移管 | ⚪ | ナレッジ移管資料 | 08-closure.md §A.4 |
| 150 | アーカイブ | ⚪ | アーカイブ手順書 | 08-closure.md §A.5 |

---

## 4. 命名・保管・版管理ルール

### 4.1 派生版の命名規約

| 種別 | 命名パターン | 例 |
|---|---|---|
| レビュー記録 | `<テンプレ DOC-ID>-REV-<YYYYMMDD>-<連番 2 桁>.md` | `DOC-TPL-REV-REV-20260820-01.md` |
| 試験ログ | `<テンプレ DOC-ID>-LOG-<YYYYMMDD>-<連番 2 桁>.md` | `DOC-TPL-TST-LOG-20260820-01.md` |
| Runbook 派生 | `<テンプレ DOC-ID>-RBK-<env>.md` | `DOC-TPL-RBK-RBK-prod.md` |
| 変更チケット | `<テンプレ DOC-ID>-TKT-<連番 4 桁>.md` | `DOC-TPL-CHG-TKT-0001.md` |
| 議事録 | `<テンプレ DOC-ID>-MIN-<YYYYMMDD>.md` | `DOC-TPL-PRC-MIN-20260820.md` |
| 完了報告 | `<テンプレ DOC-ID>-FINAL.md` | `DOC-TPL-CLO-FINAL.md` |

### 4.2 派生版の保管場所

| 種類 | 保管先 |
|---|---|
| 設計レビュー関連 | `docs/records/reviews/` |
| 試験ログ | `docs/records/tests/` |
| プロセス管理 | `docs/records/process/` |
| Runbook | `docs/runbooks/`（環境別） |
| 運用ログ | `docs/records/ops/` |
| 変更チケット | `docs/records/changes/` |
| 品質記録 | `docs/records/quality/` |
| 終結成果物 | `docs/records/closure/` |

> ※ `docs/records/` および `docs/runbooks/` ディレクトリは本テンプレ利用開始時に作成する（[DOC-ARCH-009 §7 G2 通過時](../architecture/08-workflow-overview.md)）。

### 4.3 版管理ルール

- 派生版は **Git で管理**（commit message に IPA 工程番号を含める：`[Phase-053] dev env setup`）
- 派生版同士の依存関係は **frontmatter の「関連派生版」フィールド** に明示
- 派生版が **3 バージョン以上** になったら派生版 DOC の [DOC-CHG-001](../CHANGELOG.md) エントリに転記
- 派生版に **個人情報・機密情報を含めない**（監査ログは別ファイルへ参照）

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| テンプレート | 繰り返し使う空のフォーム | IPA 共通フレーム |
| 派生版 | テンプレートから生成された実記録 | 本書 §4 |
| 証跡 | 監査・追跡のための物的・電子的証拠 | ISO 9001 |
| RACI | Responsible / Accountable / Consulted / Informed | PMBOK |
| チェックリスト | 確認項目を列挙したリスト | 本書 |
| Runbook | 運用作業の手順書 | ITIL |
| Smoke Test | 本番デプロイ直後の簡易動作確認 | DOC-ARCH-009 |
| Hypercare | Go-Live 直後の高密度サポート期間 | DOC-ARCH-009 |
| Postmortem | 障害後の根本原因分析と学びの記録 | Google SRE |
| インシデント | サービス中断/品質低下事象 | ITIL |
| WBS | Work Breakdown Structure | PMBOK |
| リスク管理台帳 | 識別されたリスクの一覧表 | PMBOK |
| 課題管理台帳 | オープン issue / blocker の一覧 | 本書 |

---

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018 年 3 月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018 年 4 月
3. JIS X 0160:2012「ソフトウェアライフサイクルプロセス」
4. PMBOK Guide 第 7 版、Project Management Institute、2021 年
5. ITIL 4、AXELOS、2019 年
6. Google SRE Book 第 2 版、Google、2020 年
7. Ada プロジェクトチーム「[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md)」、2026-08-20
8. Ada プロジェクトチーム「[DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md)」、2026-08-19
9. Ada プロジェクトチーム「[DOC-ARCH-003 横断リスク](../architecture/03-cross-cutting-risks.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
