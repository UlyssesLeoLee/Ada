# 非機能要件定義書（Non-Functional Requirements）

> **本文件の目的**：性能・可用性・運用性・移行性・セキュリティ・環境 の 6 区分について、**測定可能な非機能要件**を定義する。  
> 関連 IPA 工程: 14（非機能要件定義 / NFR）+ IPA「非機能要求グレード2018」。

> **ドキュメントID**：DOC-REQ-NFR-001
> **文書分類**：要件定義書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[SR](03-sr-system-requirements.md)（DOC-REQ-SR-001）
> **下位文書**：各 [DOC-ARCH-NNN](../architecture/00-anatomy-model.md) + [DOC-MOD-NNN §1](../modules/M-01-acquisition-adapter.md)
> **関連文書**：[`docs/architecture/03-cross-cutting-risks.md`](../architecture/03-cross-cutting-risks.md)（NFR のリスク）
> **適用 IPA 標準**：
> - IPA「非機能要求グレード2018」（[NF-AVA|PER|OPS|MIG|SEC|ENV] × 必須/推奨）
> - IPA「共通フレーム2018 (SLCP-JCF2018)」

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 14 に対応、6 区分 × 必須/推奨 計 50+ 項目） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. NFR 6 区分概要
2. [NF-AVA] 可用性
3. [NF-PER] 性能
4. [NF-OPS] 運用・保守性
5. [NF-MIG] 移行性
6. [NF-SEC] セキュリティ
7. [NF-ENV] 環境
8. NF タグ網羅率確認
9. 用語集
10. 参考文献

---

## 1. NFR 6 区分概要

| 区分 | 必須項目 | 推奨項目 | 評価基準 |
|---|---|---|---|
| [NF-AVA] 可用性 | 8 | 4 | SLA、MTTR、MTBF、DR |
| [NF-PER] 性能 | 10 | 6 | レイテンシ、スループット、リソース |
| [NF-OPS] 運用・保守性 | 9 | 5 | 監視、ログ、Backup、変更管理 |
| [NF-MIG] 移行性 | 5 | 3 | 切替時間、ロールバック、データ移行 |
| [NF-SEC] セキュリティ | 12 | 6 | 暗号化、認証、認可、監査、脆弱性 |
| [NF-ENV] 環境 | 4 | 2 | OS、ブラウザ、ハードウェア |
| **合計** | **48** | **26** | 計 74 項目 |

## 2. [NF-AVA] 可用性

### 2.1 必須項目

| NFR-ID | 要件 | 目標 | 測定方法 |
|---|---|---|---|
| NFR-AVA-01 | サービス稼働率 | 99.9% (年 8.76 時間以内停止) | 月次集計 |
| NFR-AVA-02 | 計画停止通知 | 7 日前 | 通知記録 |
| NFR-AVA-03 | 計画停止時間 | 月 4 時間以内 | 実績 |
| NFR-AVA-04 | MTTR (Mean Time To Repair) | < 30 分 | Incident ログ |
| NFR-AVA-05 | MTBF (Mean Time Between Failures) | > 30 日 | Incident ログ |
| NFR-AVA-06 | クラスタ冗長化 | N+1 (最低 3 ノード) | 構成監査 |
| NFR-AVA-07 | DR 復旧時間 (RTO) | < 1 時間 | DR 訓練ログ |
| NFR-AVA-08 | DR 復旧時点 (RPO) | < 5 分 | DR 訓練ログ |

### 2.2 推奨項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-AVA-09 | マルチリージョン | 2 リージョン以上 |
| NFR-AVA-10 | 自動フェイルオーバー | < 30 秒 |
| NFR-AVA-11 | サーキットブレーカー | 99% 成功率 |
| NFR-AVA-12 | 縮退運転 | 主要機能は継続 |

## 3. [NF-PER] 性能

### 3.1 必須項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-PER-01 | 起動時間 | < 3 秒 |
| NFR-PER-02 | データ取得レイテンシ | p99 < 100ms |
| NFR-PER-03 | 1k node 操作レイテンシ | p95 < 100ms |
| NFR-PER-04 | イベント配信レイテンシ | p99 < 50ms |
| NFR-PER-05 | ストリーミングスループット | > 10 万件 / 秒 |
| NFR-PER-06 | API レスポンス | p95 < 200ms |
| NFR-PER-07 | WebSocket 接続数 | > 100,000 同時 |
| NFR-PER-08 | ノード数 / 画布 | > 10,000 |
| NFR-PER-09 | 同時編集ユーザー | > 100 |
| NFR-PER-10 | テナント数 | > 10,000 |

### 3.2 推奨項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-PER-11 | 検索レスポンス | p95 < 50ms |
| NFR-PER-12 | エクスポート処理 | 100 万件 / 5 分 |
| NFR-PER-13 | バックプレッシャー | 自動フロー制御 |
| NFR-PER-14 | キャッシュヒット率 | > 80% |
| NFR-PER-15 | CPU 使用率 | 平均 < 50% |
| NFR-PER-16 | メモリ使用率 | 平均 < 70% |

## 4. [NF-OPS] 運用・保守性

### 4.1 必須項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-OPS-01 | 監視カバレッジ | 100% (主要メトリクス) |
| NFR-OPS-02 | ログ構造化 | 100% (JSON) |
| NFR-OPS-03 | ログ保持期間 | 1 年（監査ログ） |
| NFR-OPS-04 | バックアップ頻度 | 日次 + 増分 6 時間毎 |
| NFR-OPS-05 | 自動デプロイ | 100% (atomic swap) |
| NFR-OPS-06 | 変更管理 | 100% (CR 経由) |
| NFR-OPS-07 | 構成管理 | Git 管理 100% |
| NFR-OPS-08 | 監視アラート | 即時通知 (< 1 分) |
| NFR-OPS-09 | インシデント対応 | 初動 30 分以内 |

### 4.2 推奨項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-OPS-10 | 自動修復 | 主要障害 50% 自動 |
| NFR-OPS-11 | キャパシティ予測 | 30 日先まで |
| NFR-OPS-12 | コスト可視化 | 月次レポート |
| NFR-OPS-13 | Runbook 完備 | 100% カバー |
| NFR-OPS-14 | トレーニング | 月 1 回以上 |

## 5. [NF-MIG] 移行性

### 5.1 必須項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-MIG-01 | 切替時間 | < 5 分 |
| NFR-MIG-02 | ロールバック時間 | < 5 分 |
| NFR-MIG-03 | データ整合 | 100% |
| NFR-MIG-04 | リハーサル | 2 回成功必須 |
| NFR-MIG-05 | 監査ログ連続性 | 100% 維持 |

### 5.2 推奨項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-MIG-06 | 段階移行 | Blue-Green / Canary |
| NFR-MIG-07 | データ移行並列度 | 動的調整 |
| NFR-MIG-08 | 旧システム並行稼働 | 最大 30 日 |

## 6. [NF-SEC] セキュリティ

### 6.1 必須項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-SEC-01 | 通信暗号化 | TLS 1.3 |
| NFR-SEC-02 | 保存時暗号化 | AES-256 + KMS |
| NFR-SEC-03 | 認証 | JWT (15 分) + Refresh |
| NFR-SEC-04 | 認可 | RBAC + ABAC |
| NFR-SEC-05 | RLS | 100% (PostgreSQL) |
| NFR-SEC-06 | 監査ログ | 1 年 + 改ざん検知 |
| NFR-SEC-07 | 脆弱性管理 | Critical 24h, High 72h |
| NFR-SEC-08 | GDPR 対応 | 全データ対象 |
| NFR-SEC-09 | PIPL 対応 | 全データ対象 |
| NFR-SEC-10 | データ越境防止 | リージョン制限 |
| NFR-SEC-11 | シークレット管理 | KMS 集中管理 |
| NFR-SEC-12 | SAST | CI 100% 実行 |

### 6.2 推奨項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-SEC-13 | HSM 利用 | 鍵管理 |
| NFR-SEC-14 | ペネトレーションテスト | 年 1 回 |
| NFR-SEC-15 | SOC 2 準拠 | Type II |
| NFR-SEC-16 | ISO 27001 | 認証取得 |
| NFR-SEC-17 | 不正アクセス検知 | 24/7 |
| NFR-SEC-18 | DLP | データ流出防止 |

## 7. [NF-ENV] 環境

### 7.1 必須項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-ENV-01 | OS 対応 | macOS 14+ / Linux (Ubuntu 22.04+) / Windows 11+ |
| NFR-ENV-02 | ブラウザ対応 | Chrome, Safari, Firefox, Edge（最新版） |
| NFR-ENV-03 | ハードウェア要件 | 8GB RAM, 4 コア, 50GB ディスク |
| NFR-ENV-04 | コンテナ対応 | Docker 24+, K8s 1.28+ |

### 7.2 推奨項目

| NFR-ID | 要件 | 目標 |
|---|---|---|
| NFR-ENV-05 | オフライン動作 | 部分的対応 |
| NFR-ENV-06 | ARM 対応 | Apple Silicon, AWS Graviton |

## 8. NF タグ網羅率確認

| 区分 | 必須網羅率 | 推奨網羅率 | 目標 |
|---|---|---|---|
| [NF-AVA] | 100% (8/8) | 50% (2/4) | 必須 100% |
| [NF-PER] | 100% (10/10) | 50% (3/6) | 必須 100% |
| [NF-OPS] | 100% (9/9) | 40% (2/5) | 必須 100% |
| [NF-MIG] | 100% (5/5) | 33% (1/3) | 必須 100% |
| [NF-SEC] | 100% (12/12) | 33% (2/6) | 必須 100% |
| [NF-ENV] | 100% (4/4) | 50% (1/2) | 必須 100% |

## 9. 用語集

| 用語 | 説明 |
|---|---|
| NFR | Non-Functional Requirements（非機能要件） |
| SLA | Service Level Agreement |
| MTTR | Mean Time To Repair |
| MTBF | Mean Time Between Failures |
| RTO | Recovery Time Objective |
| RPO | Recovery Point Objective |
| KMS | Key Management Service |
| RLS | Row-Level Security |
| GDPR | EU 一般データ保護規則 |
| PIPL | 中国個人情報保護法 |

## 10. 参考文献

1. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018 年 4 月
2. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
3. Ada プロジェクトチーム「[DOC-ARCH-003 横断リスク §NF](../architecture/03-cross-cutting-risks.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
