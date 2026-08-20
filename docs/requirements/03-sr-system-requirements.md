# システム要件定義書（System Requirements）

> **本文件の目的**：[BR](02-br-business-requirements.md) をシステム化の観点から整理し、**システムとして備えるべき機能・制約**を定義する。  
> 関連 IPA 工程: 12（システム要件定義 / SR）。

> **ドキュメントID**：DOC-REQ-SR-001
> **文書分類**：要件定義書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[BR](02-br-business-requirements.md)（DOC-REQ-BR-001）
> **下位文書**：[FR](04-fr-functional-requirements.md)（DOC-REQ-FR-001）
> **関連文書**：[`docs/upstream/07-to-be-system.md`](../upstream/07-to-be-system.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 12 に対応、15 SR 項目） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. システム化範囲
2. システム要件一覧
3. システム間インタフェース
4. システム制約
5. 用語集
6. 参考文献

---

## 1. システム化範囲

### 1.1 システム化対象

- 16 モジュール（[DOC-UP-007 To-Be システム §3](../upstream/07-to-be-system.md)）
- 11 テーブル + 6 PL/pgSQL（[DOC-MOD-010 §4](../modules/M-10-tenant-middleware.md)）
- REST API + WebSocket + 管理 API（[DOC-API-001〜006](../api/rest-endpoints.md)）
- 3 OS 対応、3 デプロイモード

### 1.2 システム化対象外

- 業務システム側の改修
- データソース提供側の改修
- ネットワーク・ハードウェア

## 2. システム要件一覧

| SR-ID | システム要件 | 関連 BR | 関連 F-ID | 優先度 |
|---|---|---|---|---|
| SR-001 | データ取得モジュール | BR-001 | F-02 | 必須 |
| SR-002 | データ標準化モジュール | BR-002 | F-03, F-10 | 必須 |
| SR-003 | データフロー実行エンジン | BR-003 | F-04 | 必須 |
| SR-004 | パイプライン制御 | BR-003 | F-05 | 必須 |
| SR-005 | 条件分岐・繰り返し | BR-003 | F-06 | 必須 |
| SR-006 | プラグイン機構 | BR-006 | F-07 | 高 |
| SR-007 | デバッグ機能 | BR-004 | F-08 | 高 |
| SR-008 | 認証・認可・テナント分離 | BR-009, BR-011 | F-11, F-17 | 必須 |
| SR-009 | トリガーエンジン | BR-007 | F-13 | 必須 |
| SR-010 | 視覚エディタ | BR-005 | F-01 | 必須 |
| SR-011 | 単一バイナリ | BR-008 | F-09 | 必須 |
| SR-012 | 外部出力 | BR-001 | F-14 | 必須 |
| SR-013 | イベント駆動 | BR-010 | F-15 | 必須 |
| SR-014 | ストリーミング処理 | BR-010 | F-16 | 必須 |
| SR-015 | マルチテナントストレージ | BR-011 | F-17 | 必須 |

## 3. システム間インタフェース

| システム | 連携方法 | 認証 |
|---|---|---|
| 業務 ERP | REST API / Webhook | OAuth 2.0 |
| 業務 CRM | REST API | OAuth 2.0 |
| 業務 在庫 | DB (CDC) | SCRAM |
| 業務 会計 | CSV / SFTP | IAM キー |
| BI ツール | Snowflake → Tableau | 既存 |
| IdP | SAML / OIDC | 既存 |

## 4. システム制約

| 制約 | 影響 | 対応 |
|---|---|---|
| Rust 必須 | ライブラリ・人材 | [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) |
| PostgreSQL 16+ | DB ベンダ | [DOC-MOD-010](../modules/M-10-tenant-middleware.md) |
| 3 OS 対応 | ビルド・配布 | [DOC-ARCH-002](../architecture/01-tech-stack.md) |
| ブラウザ最新 | UI 互換性 | [DOC-MOD-012](../modules/M-12-canvas-editor-frontend.md) |
| オフライン動作 | 機能制約 | 部分的制限 |

## 5. 用語集

| 用語 | 説明 |
|---|---|
| SR | System Requirements（システム要件） |
| システム化 | 手作業・他システムを本システムに統合する |
| CDC | Change Data Capture |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
2. Ada プロジェクトチーム「[DOC-REQ-BR-001](02-br-business-requirements.md)」、2026-08-20
3. Ada プロジェクトチーム「[DOC-UP-007 To-Be システム](../upstream/07-to-be-system.md)」、2026-08-20

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
