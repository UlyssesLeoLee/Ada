# 運用要件定義書（Operation Requirements）

> **本文件の目的**：監視・運用・保守・サポートの運用要件を定義する。  
> 関連 IPA 工程: 18（運用要件定義）。

> **ドキュメントID**：DOC-REQ-OPS-001
> **文書分類**：要件定義書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[SR](03-sr-system-requirements.md)
> **下位文書**：[DOC-ARCH-004](../architecture/04-atomic-deployment.md)、[DOC-ARCH-005](../architecture/05-admin-operations-ui.md)
> **関連文書**：[`docs/architecture/04-atomic-deployment.md`](../architecture/04-atomic-deployment.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - ITIL 4
> - Google SRE

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 18 に対応） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 監視要件
2. バックアップ要件
3. 障害対応要件
4. 保守要件
5. サポート要件
6. 用語集
7. 参考文献

---

## 1. 監視要件

| OPS-ID | 要件 | 目標 |
|---|---|---|
| OPS-01 | 稼働監視 | 100%（HTTP/DB/Cache） |
| OPS-02 | リソース監視 | CPU/メモリ/ディスク/帯域 |
| OPS-03 | アプリ監視 | レイテンシ、エラー率、スループット |
| OPS-04 | ビジネス監視 | パイプライン成功率、ユーザー数 |
| OPS-05 | ログ集約 | 1 年保存、構造化 |
| OPS-06 | トレース | OpenTelemetry 100% 計装 |
| OPS-07 | アラート | 即時通知、深刻度別エスカレーション |
| OPS-08 | ダッシュボード | リアルタイム表示 |

## 2. バックアップ要件

| OPS-ID | 要件 | 詳細 |
|---|---|---|
| OPS-09 | バックアップ頻度 | 日次フル + 6 時間毎増分 |
| OPS-10 | バックアップ保持 | 30 日 |
| OPS-11 | バックアップ暗号化 | AES-256 |
| OPS-12 | バックアップ検証 | 週次リストアテスト |
| OPS-13 | バックアップ保管 | 別リージョン / 別 AZ |
| OPS-14 | 設定バックアップ | Terraform / K8s manifest |
| OPS-15 | シークレットバックアップ | KMS 管理（[UN-P0-06 選定待ち](../architecture/07-qa-register.md)） |

## 3. 障害対応要件

| OPS-ID | 要件 | 目標 |
|---|---|---|
| OPS-16 | 検知時間 | < 1 分 |
| OPS-17 | 初動時間 | < 30 分 |
| OPS-18 | 復旧時間 (MTTR) | < 30 分 |
| OPS-19 | ロールバック時間 | < 5 分 |
| OPS-20 | Postmortem | 5 営業日以内 |
| OPS-21 | オンコール | 24/7 |
| OPS-22 | エスカレーション | PM → PO → 経営層 |
| OPS-23 | インシデント分類 | Sev1〜4 |

## 4. 保守要件

| OPS-ID | 要件 | 詳細 |
|---|---|---|
| OPS-24 | 変更管理 | CR 経由 100% |
| OPS-25 | 影響分析 | 必須 |
| OPS-26 | リリース判定 | Go/No-Go 会議 |
| OPS-27 | atomic 反映 | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) 準拠 |
| OPS-28 | 構成管理 | Git + Cargo.lock |
| OPS-29 | バージョン管理 | SemVer |
| OPS-30 | ロールバック容易性 | 旧版保持 7 日 |

## 5. サポート要件

| OPS-ID | 要件 | 目標 |
|---|---|---|
| OPS-31 | 初回応答 | 4 時間以内 |
| OPS-32 | 解決時間 | 24 時間以内（重大 1 時間） |
| OPS-33 | サポートチャネル | メール / Slack / 電話 |
| OPS-34 | ナレッジベース | 80% 問題が自己解決可能 |
| OPS-35 | トレーニング資料 | 動画 + マニュアル |
| OPS-36 | 教育 | 月 1 回 |

## 6. 用語集

| 用語 | 説明 |
|---|---|
| SLA | Service Level Agreement |
| SLO | Service Level Objective |
| MTTR | Mean Time To Repair |
| RTO | Recovery Time Objective |
| RPO | Recovery Point Objective |
| On-call | 24/7 待機 |
| CR | Change Request |
| atomic 反映 | 旧版保持での切替 |

## 7. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
2. ITIL 4、AXELOS、2019 年
3. Google SRE Book 第 2 版、Google、2020 年
4. Ada プロジェクトチーム「[DOC-ARCH-004 原子化デプロイ](../architecture/04-atomic-deployment.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
