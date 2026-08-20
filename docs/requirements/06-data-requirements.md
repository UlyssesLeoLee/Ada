# データ要件定義書（Data Requirements）

> **本文件の目的**：データモデル・データ品質・データライフサイクル・データガバナンスの要件を定義する。  
> 関連 IPA 工程: 15（データ要件定義）。

> **ドキュメントID**：DOC-REQ-DATA-001
> **文書分類**：要件定義書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[FR](04-fr-functional-requirements.md)
> **下位文書**：[DOC-MOD-010 §3-4](../modules/M-10-tenant-middleware.md)
> **関連文書**：[`docs/architecture/00-anatomy-model.md`](../architecture/00-anatomy-model.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 15 に対応） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. データモデル要件
2. データ品質要件
3. データライフサイクル要件
4. データガバナンス要件
5. 用語集
6. 参考文献

---

## 1. データモデル要件

| DATA-ID | 要件 | 詳細 |
|---|---|---|
| DATA-01 | 11 テーブル構造 | [DOC-MOD-010 §4.1](../modules/M-10-tenant-middleware.md) 参照 |
| DATA-02 | NJSON 標準化 | 内部データ交換形式 |
| DATA-03 | マルチテナント | tenant_id 必須、全テーブル |
| DATA-04 | 時系列管理 | created_at / updated_at 必須 |
| DATA-05 | 論理削除 | deleted_at カラム、永久削除は別途 |
| DATA-06 | 主キー戦略 | UUID v7（時系列順序保証） |
| DATA-07 | 外部キー制約 | 整合性保証 |
| DATA-08 | インデックス戦略 | tenant_id 先頭、B-tree + 部分インデックス |

## 2. データ品質要件

| DATA-ID | 要件 | 目標 |
|---|---|---|
| DATA-09 | 整合性チェック | 100%（CHECK 制約、外部キー） |
| DATA-10 | 一意性保証 | UUID v7 + UNIQUE 制約 |
| DATA-11 | データ型バリデーション | JSON Schema + DB 制約 |
| DATA-12 | 欠損値処理 | NOT NULL 必須箇所を明文化 |
| DATA-13 | データ正規化 | 第 3 正規形を基本 |
| DATA-14 | PII マスキング | ログ・キャッシュ・監査ログ |

## 3. データライフサイクル要件

| DATA-ID | 要件 | 期間 |
|---|---|---|
| DATA-15 | 監査ログ | 1 年保存（改ざん不可） |
| DATA-16 | イベントログ | 90 日 |
| DATA-17 | アクセスログ | 30 日 |
| DATA-18 | 一時データ | 7 日 |
| DATA-19 | バックアップ | 30 日（フル） + 7 日（増分） |
| DATA-20 | アーカイブ | 5 年（法令による） |
| DATA-21 | GDPR 忘れられる権利 | 30 日以内削除 |

## 4. データガバナンス要件

| DATA-ID | 要件 |
|---|---|
| DATA-22 | データ所有者 | テーブル毎に Biz 担当を任命 |
| DATA-23 | データ分類 | Public / Internal / Confidential / Restricted |
| DATA-24 | アクセス制御 | RLS + ロール |
| DATA-25 | 暗号化 | 個人情報・機密情報は AES-256 |
| DATA-26 | データ越境 | テナントリージョン内に限定 |
| DATA-27 | データ Lineage | 取得→変換→出力のトレース |
| DATA-28 | メタデータ | 各テーブルにコメント + ドキュメント |

## 5. 用語集

| 用語 | 説明 |
|---|---|
| NJSON | 標準化 JSON |
| RLS | Row-Level Security |
| PII | Personally Identifiable Information |
| Lineage | データ系譜 |
| UUID v7 | 時系列順序保証付き UUID |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
2. Ada プロジェクトチーム「[DOC-MOD-010 §3-4](../modules/M-10-tenant-middleware.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
