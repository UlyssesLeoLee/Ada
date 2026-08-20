# 外部インターフェース要件定義書（External IF Requirements）

> **本文件の目的**：外部システムとの連携 IF 要件を定義する。  
> 関連 IPA 工程: 16（外部インターフェース要件定義 / IF）。

> **ドキュメントID**：DOC-REQ-IF-001
> **文書分類**：要件定義書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[SR](03-sr-system-requirements.md)
> **下位文書**：[DOC-MOD-001 §2](../modules/M-01-acquisition-adapter.md)、[DOC-MOD-014](../modules/M-14-module-registry.md)
> **関連文書**：[`docs/api/rest-endpoints.md`](../api/rest-endpoints.md)（DOC-API-001）、[`docs/architecture/07-qa-register.md` UN-P0-06](../architecture/07-qa-register.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 16 に対応） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 外部システム一覧
2. IF 種別
3. 認証・認可
4. 性能・可用性
5. データフォーマット
6. 障害対応
7. 用語集
8. 参考文献

---

## 1. 外部システム一覧

| IF-ID | 外部システム | 種別 | 関連 F-ID | 優先度 |
|---|---|---|---|---|
| IF-01 | 業務 ERP | REST API + Webhook | F-02, F-15 | 必須 |
| IF-02 | 業務 CRM | REST API | F-02 | 必須 |
| IF-03 | 業務 在庫 | DB (CDC, PostgreSQL 12→16) | F-02 | 必須 |
| IF-04 | 業務 会計 | CSV / SFTP | F-02 | 高 |
| IF-05 | 業務 人事 | REST API | F-02 | 中 |
| IF-06 | IdP (Okta) | SAML / OIDC | F-11, F-17 | 必須 |
| IF-07 | 監視 (Datadog / Prometheus) | OTLP | F-15 | 必須 |
| IF-08 | KMS (AWS / HashiCorp) | API | F-17 | 必須 |
| IF-09 | 通知 (Slack / Email) | Webhook + SMTP | F-15 | 必須 |
| IF-10 | DWH (Snowflake) | COPY INTO | F-14 | 中 |

## 2. IF 種別

| 種別 | プロトコル | 用途 |
|---|---|---|
| REST API | HTTPS / JSON | 業務システム連携 |
| Webhook | HTTPS / JSON | イベント通知 |
| DB (CDC) | TCP / SSL | データ同期 |
| ファイル | SFTP / S3 | データ交換 |
| gRPC | HTTP/2 | 内部サービス間（プラグイン） |
| WebSocket | WSS | リアルタイム UI |

## 3. 認証・認可

| IF | 認証 | 認可 |
|---|---|---|
| 外部 API | OAuth 2.0 Client Credentials | スコープ |
| 内部 API | JWT (15 分) + Refresh | RBAC + ABAC |
| DB | SCRAM-SHA-256 | RLS |
| ファイル | IAM キー | パス制御 |
| IdP | SAML 2.0 / OIDC | ロール |

## 4. 性能・可用性

| 項目 | 目標 |
|---|---|
| 外部 API 呼び出し | p95 < 500ms |
| Webhook 配信 | p99 < 5s |
| ファイル転送 | 100 MB / 1 分 |
| 接続プール | 100 / 外部システム |
| リトライ | 3 回、指数バックオフ |
| サーキットブレーカー | 失敗率 50% で遮断 |

## 5. データフォーマット

| 種別 | 形式 | 仕様 |
|---|---|---|
| 業務データ | NJSON (内部) | [DOC-MOD-002 §2](../modules/M-02-normalizer.md) |
| API レスポンス | JSON | OpenAPI 3.1 |
| エラー | JSON | [DOC-API-003](../api/error-codes.md) |
| WebSocket | JSON | [DOC-API-002](../api/websocket-events.md) |
| ファイル | CSV / Parquet | RFC 4180 / Apache Parquet |

## 6. 障害対応

| シナリオ | 対応 |
|---|---|
| 接続タイムアウト | リトライ → サーキットブレーカー → 通知 |
| 認証失敗 | リフレッシュ → 再認証 → 失敗時アラート |
| レート制限 | バックオフ → 別経路 |
| スキーマ不整合 | 検証 → エラー通知 |
| 大量データ | ページネーション / 分割 |
| データ欠損 | 一時保持 → 整合性確認 → 補完 |

## 7. 用語集

| 用語 | 説明 |
|---|---|
| CDC | Change Data Capture |
| Webhook | HTTP コールバック |
| サーキットブレーカー | 障害遮断パターン |
| RLS | Row-Level Security |

## 8. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
2. OpenAPI 3.1 仕様
3. Ada プロジェクトチーム「[DOC-ARCH-007 Rust crate 選択 §12 HTTP](../architecture/06-rust-tech-selection.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
