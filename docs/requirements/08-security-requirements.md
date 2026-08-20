# セキュリティ要件定義書（Security Requirements）

> **本文件の目的**：認証・認可・暗号化・監査・脆弱性管理・コンプライアンスのセキュリティ要件を定義する。  
> 関連 IPA 工程: 17（セキュリティ要件定義）。

> **ドキュメントID**：DOC-REQ-SEC-001
> **文書分類**：要件定義書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[SR](03-sr-system-requirements.md)
> **下位文書**：[DOC-MOD-011](../modules/M-11-rbac-collab.md)、[DOC-MOD-010 §4](../modules/M-10-tenant-middleware.md)
> **関連文書**：[`docs/architecture/03-cross-cutting-risks.md §3`](../architecture/03-cross-cutting-risks.md)、[`docs/architecture/07-qa-register.md` UN-P0-06,07,08](../architecture/07-qa-register.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - GDPR / PIPL

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 17 に対応） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 認証
2. 認可
3. 暗号化
4. 監査
5. 脆弱性管理
6. コンプライアンス
7. インシデント対応
8. 用語集
9. 参考文献

---

## 1. 認証

| SEC-ID | 要件 | 詳細 |
|---|---|---|
| SEC-01 | 認証方式 | JWT (15 分) + Refresh Token (7 日) |
| SEC-02 | パスワード | bcrypt cost 12 + 8 文字以上 + 複雑性 |
| SEC-03 | MFA | TOTP 必須（管理者）、推奨（一般） |
| SEC-04 | SSO | SAML 2.0 / OIDC 対応 |
| SEC-05 | セッション | アイドル 30 分でタイムアウト |
| SEC-06 | 鍵管理 | KMS（[UN-P0-06 選定待ち](../architecture/07-qa-register.md)） |
| SEC-07 | 鍵ローテーション | 90 日毎 |

## 2. 認可

| SEC-ID | 要件 | 詳細 |
|---|---|---|
| SEC-08 | RBAC | ロールベース |
| SEC-09 | ABAC | 属性ベース（テナント、部署） |
| SEC-10 | RLS | PostgreSQL 行レベルセキュリティ 100% |
| SEC-11 | リソース単位 | テナント/キャンバス/ノード単位 |
| SEC-12 | 権限継承 | 階層対応 |

## 3. 暗号化

| SEC-ID | 要件 | 詳細 |
|---|---|---|
| SEC-13 | 通信暗号化 | TLS 1.3（後方互換 TLS 1.2） |
| SEC-14 | 保存時暗号化 | AES-256-GCM |
| SEC-15 | 鍵管理 | KMS / HSM |
| SEC-16 | 証明書管理 | Let's Encrypt / 内部 CA |
| SEC-17 | データベース暗号化 | 透過的データ暗号化 (TDE) |

## 4. 監査

| SEC-ID | 要件 | 詳細 |
|---|---|---|
| SEC-18 | 監査ログ | 全操作記録 |
| SEC-19 | 監査ログ保存 | 1 年（改ざん不可） |
| SEC-20 | 監査ログ項目 | ユーザー、アクション、対象、時刻、IP、UA |
| SEC-21 | 改ざん検知 | ハッシュチェーン |
| SEC-22 | 監査ログ検索 | 100ms 以内 |
| SEC-23 | GDPR 忘れられる権利対応 | [UN-P0-08 フロー待ち](../architecture/07-qa-register.md) |
| SEC-24 | PIPL 越境防止 | 中国国内データ国内保管 |

## 5. 脆弱性管理

| SEC-ID | 要件 | 目標 |
|---|---|---|
| SEC-25 | SAST | CI 100% 実行（cargo-deny / cargo-audit） |
| SEC-26 | 脆弱性対応 SLA | Critical 24h / High 72h / Medium 1 週 / Low 次回 |
| SEC-27 | 依存関係スキャン | 週次 |
| SEC-28 | コンテナスキャン | Trivy / Snyk |
| SEC-29 | ペネトレーションテスト | 年 1 回 |
| SEC-30 | バグバウンティ | 検討 |

## 6. コンプライアンス

| SEC-ID | 要件 | 詳細 |
|---|---|---|
| SEC-31 | GDPR | EU 居住者データ対応 |
| SEC-32 | PIPL | 中国居住者データ対応 |
| SEC-33 | APPI | 日本個人情報保護法 |
| SEC-34 | SOC 2 Type II | 推奨 |
| SEC-35 | ISO 27001 | 推奨 |
| SEC-36 | 業界規制 | 金融、医療等の追加要件 |

## 7. インシデント対応

| SEC-ID | 要件 | 目標 |
|---|---|---|
| SEC-37 | インシデント検知 | < 1 分 |
| SEC-38 | インシデント初動 | < 30 分 |
| SEC-39 | インシデント通知 | 影響範囲判明後 < 1 時間 |
| SEC-40 | Postmortem | 5 営業日以内 |
| SEC-41 | 報告義務 | 72h 以内（GDPR Art.33） |

## 8. 用語集

| 用語 | 説明 |
|---|---|
| RBAC | Role-Based Access Control |
| ABAC | Attribute-Based Access Control |
| RLS | Row-Level Security |
| MFA | Multi-Factor Authentication |
| KMS | Key Management Service |
| GDPR | EU 一般データ保護規則 |
| PIPL | 中国個人情報保護法 |
| APPI | 日本個人情報保護法 |
| SAST | Static Application Security Testing |

## 9. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
2. GDPR (EU 2016/679)
3. PIPL (中華人民共和国 2021)
4. Ada プロジェクトチーム「[DOC-ARCH-003 横断リスク §3](../architecture/03-cross-cutting-risks.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
