# 運用管理テンプレート集（Operations Management Templates）

> **本ファイルの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) §5.12（運用プロセス、IPA 工程 109-117）に対応する **9 種類の運用管理テンプレート** を提供する。  
> 運用引継ぎ、監視、ジョブ、Backup、Capacity、Incident、Postmortem、Problem、問い合わせの 9 領域をカバー。

> **ドキュメントID**：DOC-TPL-OPS
> **文書分類**：テンプレート集
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/templates/README.md`](README.md)（DOC-TPL-INDEX）、[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)（DOC-ARCH-009）
> **下位文書**：派生版（`docs/records/ops/<テンプレ DOC-ID>-OPS-<YYYYMMDD>.md`）
> **関連文書**：
> - [`docs/architecture/04-atomic-deployment.md`](../architecture/04-atomic-deployment.md)
> - [`docs/architecture/05-admin-operations-ui.md`](../architecture/05-admin-operations-ui.md)
> - [`docs/modules/M-15-central-event-bus.md`](../modules/M-15-central-event-bus.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - ITIL 4
> - Google SRE Book 第 2 版
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（運用引継ぎ/監視/ジョブ/BK/Capa/Incident/Postmortem/Problem/Support の 9 テンプレート） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 運用引継ぎ書（IPA 工程 109）
2. 監視設定書（IPA 工程 110）
3. ジョブ定義書（IPA 工程 111）
4. Backup スケジュール / ログ（IPA 工程 112）
5. Capacity 管理表（IPA 工程 113）
6. Incident Response Runbook（IPA 工程 114）
7. Postmortem テンプレート（IPA 工程 115）
8. 問題管理台帳（IPA 工程 116）
9. 問い合わせ対応記録（IPA 工程 117）
10. 用語集
11. 参考文献

---

## A.1 運用引継ぎ書（IPA 工程 109）

### A.1.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 109（運用引継ぎ） |
| 目的 | Dev → SRE への運用業務の確実な引継ぎ |
| 記入者 | PM + Dev + SRE |
| 記入タイミング | Go-Live 4 週間前、引継ぎ完了時 |
| 関連ドキュメント | [DOC-ARCH-005 §6](../architecture/05-admin-operations-ui.md) |
| NF タグ | [NF-OPS]【必須】 |

### A.1.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-OPS-OPS-handover
引継ぎ日: ____-__-__
引継ぎ元: <Dev チーム>
引継ぎ先: <SRE チーム>
対象システム: Ada v1.x
参照 Hypercare 計画: [DOC-TPL-RBK-RBK-hypercare](04-runbooks.md#a11-hypercare-計画書ipa-工程-108)
```

### A.1.3 引継ぎチェックリスト

| # | カテゴリ | 項目 | 引継ぎ日 | 確認者 | 完了 |
|---|---|---|---|---|---|
| 1 | アーキテクチャ | 全体構成図の説明 | ____-__-__ | <SRE> | ☐ |
| 2 | アーキテクチャ | 16 crate の責務 | ____-__-__ | <SRE> | ☐ |
| 3 | インフラ | 本番環境構成（[DOC-ARCH-002](../architecture/02-deployment.md)） | ____-__-__ | <SRE> | ☐ |
| 4 | 監視 | 監視設定（[§A.2](#a2-監視設定書ipa-工程-110)） | ____-__-__ | <SRE> | ☐ |
| 5 | ジョブ | ジョブ一覧（[§A.3](#a3-ジョブ定義書ipa-工程-111)） | ____-__-__ | <SRE> | ☐ |
| 6 | BK | Backup 設定（[§A.4](#a4-backup-スケジュール--ログipa-工程-112)） | ____-__-__ | <SRE> | ☐ |
| 7 | Runbook | 全 Runbook（[§A.6](#a6-incident-response-runbookipa-工程-114)） | ____-__-__ | <SRE> | ☐ |
| 8 | インシデント | Incident Response（[§A.6](#a6-incident-response-runbookipa-工程-114)） | ____-__-__ | <SRE> | ☐ |
| 9 | 問い合わせ | サポート窓口（[§A.9](#a9-問い合わせ対応記録ipa-工程-117)） | ____-__-__ | <サポート> | ☐ |
| 10 | ツール | 監視/ログ/連絡ツールの権限付与 | ____-__-__ | <SRE> | ☐ |
| 11 | ドキュメント | 管理者 UI（[DOC-ARCH-005](../architecture/05-admin-operations-ui.md)）の説明 | ____-__-__ | <SRE> | ☐ |
| 12 | 教育 | SRE トレーニング完了 | ____-__-__ | <SRE> | ☐ |

### A.1.4 完了基準

- 全 12 項目 完了
- 引継ぎ会議実施 + 議事録共有

---

## A.2 監視設定書（IPA 工程 110）

### A.2.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 110（システム監視） |
| 目的 | サービスの稼働状況・リソース・パフォーマンスを継続的に監視 |
| 記入者 | SRE |
| 記入タイミング | 環境構築時、変更時 |
| NF タグ | [NF-AVA]【必須】（SLA 99.9% 達成） |

### A.2.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-OPS-OPS-monitoring
対象環境: ☐ Production  ☐ Staging  ☐ Dev
監視ツール: ☐ Prometheus + Grafana  ☐ Datadog  ☐ New Relic  ☐ CloudWatch
ログ基盤: ☐ Loki  ☐ ELK  ☐ CloudWatch  ☐ Datadog
APM: ☐ Jaeger  ☐ Datadog APM  ☐ New Relic
```

### A.2.3 監視対象

| カテゴリ | メトリクス | 閾値（warn / crit） | アラート先 | 通知方法 |
|---|---|---|---|---|
| サービス稼働 | HTTP 200/4xx/5xx rate | 4xx > 1% / 5xx > 0.1% | SRE | Slack / PagerDuty |
| レイテンシ | p50/p95/p99 | p95 > 500ms / 1s | SRE | Slack |
| リソース | CPU 使用率 | > 70% / > 85% | SRE | Slack |
| リソース | メモリ使用率 | > 75% / > 90% | SRE | Slack |
| ディスク | 使用率 | > 80% / > 90% | SRE | Slack / PagerDuty |
| DB | 接続数 | > 80% / > 95% | SRE | Slack |
| DB | レプリケーション遅延 | > 5s / > 30s | SRE | Slack |
| DB | Slow query | > 1s / > 5s | SRE | Slack |
| Cluster | ノード稼働数 | < N-1 / < N-2 | SRE | PagerDuty |
| Cluster | リーダー選出 | 切替発生 / 連続失敗 | SRE | Slack |
| Backup | 成功 / 失敗 | fail / 2 連続 fail | SRE | PagerDuty |
| 監査ログ | 連続性 | gap > 5min | SecO | PagerDuty |

### A.2.4 アラート通知ルール

| 重大度 | 通知先 | 通知方法 | エスカレーション |
|---|---|---|---|
| Critical | on-call SRE | PagerDuty（即時電話） | 5min 応答なしで PM + アーキ |
| Warning | #ops-alerts チャンネル | Slack | 30min 対応なしで on-call |
| Info | ダッシュボードのみ | なし | なし |

### A.2.5 完了基準

- 全カテゴリに閾値設定
- アラート通知テスト成功

---

## A.3 ジョブ定義書（IPA 工程 111）

### A.3.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-OPS-OPS-jobs
対象: 定期実行ジョブ一覧
スケジューラ: ☐ cron  ☐ Airflow  ☐ Argo Workflows  ☐ 商用 SaaS
参照: [DOC-MOD-015 §2.5 Outbox](../modules/M-15-central-event-bus.md)
```

### A.3.2 ジョブ一覧

| ジョブ ID | 名前 | スケジュール | 実行内容 | タイムアウト | 失敗時動作 | 関連 IPA 工程 |
|---|---|---|---|---|---|---|
| JOB-001 | 監査ログローテーション | 0 0 * * * | パーティション切替 + 古いログアーカイブ | 30min | アラート + 手動 | 110 |
| JOB-002 | Backup 実行 | 0 2 * * * | pg_dump + S3 アップロード | 1h | アラート + 再試行 | 112 |
| JOB-003 | Outbox イベント配信 | * * * * * | 5 秒毎の Polling + 配信 | 1min | アラート | 114 |
| JOB-004 | キャパシティレポート | 0 9 * * 1 | 週次使用率レポート生成 | 10min | アラート | 113 |
| JOB-005 | 古いイベントログ削除 | 0 3 * * 0 | 90 日以上前のイベント削除 | 30min | アラート | 110 |
| JOB-006 | モジュール健全性チェック | */5 * * * * | 16 crate に対する heartbeat | 1min | アラート | 110 |
| JOB-007 | セキュリティスキャン | 0 4 * * * | cargo-deny + cargo-audit | 30min | 即時 SecO 通知 | 123 |
| ... | ... | ... | ... | ... | ... | ... |

### A.3.3 完了基準

- 全ジョブにタイムアウト設定
- 失敗時アラート + エスカレーション設定

---

## A.4 Backup スケジュール / ログ（IPA 工程 112）

### A.4.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 112（バックアップ） |
| 目的 | データ損失防止 + ディザスタリカバリ |
| 記入者 | DBA + SRE |
| 記入タイミング | 設計時、Backup 取得時 |
| NF タグ | [NF-AVA]【必須】 |
| 関連 | [DOC-ARCH-008 UN-P0-10](../architecture/07-qa-register.md)（Backup 戦略未決） |

### A.4.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-OPS-OPS-backup
Backup 方式: ☐ pg_dump フル  ☐ 増分 WAL  ☐ PITR  ☐ スナップショット
保管先: ☐ S3  ☐ GCS  ☐ Azure Blob  ☐ オンプレ NAS
暗号化: ☐ AES-256  ☐ KMS
保持期間: ☐ 30 日  ☐ 90 日  ☐ 1 年
RPO 目標: __分
RTO 目標: __h
```

### A.4.3 Backup スケジュール

| Backup ID | 種別 | スケジュール | 容量 | 保持 | 暗号化 | 検証頻度 |
|---|---|---|---|---|---|---|
| BK-001 | フル | 0 2 * * *（毎日 02:00） | __GB | 30 日 | AES-256 | 週次 |
| BK-002 | 増分 | 0 */6 * * *（6 時間毎） | __GB | 7 日 | AES-256 | 日次 |
| BK-003 | WAL | 連続アーカイブ | __GB | 30 日 | AES-256 | 日次 |
| BK-004 | スナップショット | 0 3 * * 0（毎週日曜 03:00） | __GB | 4 週 | KMS | 週次 |

### A.4.4 Backup ログ

| 日付 | Backup ID | 開始 | 終了 | 容量 | 結果 | 検証 | 担当 |
|---|---|---|---|---|---|---|---|
| YYYY-MM-DD | BK-001 | HH:MM | HH:MM | __GB | ☐ Pass / ☐ Fail | ☐ | <DBA> |
| ... | ... | ... | ... | ... | ... | ... | ... |

### A.4.5 リストアテスト（四半期毎）

| テスト日 | 対象 Backup | RTO 目標 | RTO 実測 | RPO 目標 | RPO 実測 | 結果 |
|---|---|---|---|---|---|---|
| YYYY-MM-DD | BK-001 + WAL | 1h | __h | 5min | __min | ☐ Pass / ☐ Fail |

### A.4.6 完了基準

- Backup 成功率 ≥ 99.9%
- 四半期毎リストアテスト合格
- RTO / RPO 目標達成

---

## A.5 Capacity 管理表（IPA 工程 113）

### A.5.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-OPS-OPS-capacity
対象環境: ☐ Production  ☐ Staging
作成日: ____-__-__
作成者: <SRE>
更新頻度: 週次
```

### A.5.2 キャパシティ実績

| リソース | 現状 | 1 ヶ月予測 | 3 ヶ月予測 | 閾値 | 警告 |
|---|---|---|---|---|---|
| CPU（平均） | __% | __% | __% | 70% | ☐ |
| メモリ（平均） | __% | __% | __% | 75% | ☐ |
| ディスク | __% | __% | __% | 80% | ☐ |
| DB 接続数 | __/__ | __/__ | __/__ | 80% | ☐ |
| 帯域 | __Mbps | __Mbps | __Mbps | 70% | ☐ |
| ストレージ | __GB / __GB | | | 80% | ☐ |

### A.5.3 スケール判断

| 判断 | 閾値 | アクション |
|---|---|---|
| 通常 | < 50% | なし |
| 計画的スケール | > 60% 予測 | ノード追加計画 |
| 即時スケール | > 80% | 緊急スケールアウト |
| 縮退 | < 20% × 3 ヶ月 | スケールイン検討 |

### A.5.4 完了基準

- 週次更新
- 3 ヶ月予測で全リソース閾値以内

---

## A.6 Incident Response Runbook（IPA 工程 114）

### A.6.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 114（インシデント管理） |
| 目的 | サービス障害発生時の即時対応フロー |
| 記入者 | SRE |
| 記入タイミング | 設計時、障害発生時の更新 |
| NF タグ | [NF-SEC]【必須】 |

### A.6.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-OPS-OPS-incident
オンコール: PagerDuty rotation
連絡: Slack #incident-<id>
エスカレーション: <SRE Lead> → <PM> → <PO>
```

### A.6.3 重大度定義

| 重大度 | 定義 | 例 | 初期対応 |
|---|---|---|---|
| Sev1 | 全サービス停止、データ損失 | 全ノードダウン、Backup 失敗 | 即時対応、5min 以内 |
| Sev2 | 主要機能停止 | 1 機能完全不可、特定テナント全停止 | 15min 以内 |
| Sev3 | 部分機能低下 | 性能劣化、一部エラー | 1h 以内 |
| Sev4 | 軽微 | UI の乱れ、警告 | 次営業日 |

### A.6.4 対応フロー（タイムライン）

| 時間 | アクション | 担当 | チェック |
|---|---|---|---|
| 0min | アラート受信、認識 | on-call SRE | ☐ |
| 5min | 重大度判定、Sev1/2 は即時エスカレーション | on-call SRE | ☐ |
| 5min | Incident Channel 開設、IC（Incident Commander）任命 | on-call SRE | ☐ |
| 10min | 状況把握、影響範囲特定、ログ確認 | SRE + Dev | ☐ |
| 15min | 暫定対応（ロールバック、機能停止、トラフィック制限） | SRE | ☐ |
| 30min | ステータスページ更新 | SRE | ☐ |
| 30min | 1 時間毎の進捗報告 | IC | ☐ |
| 解決時 | 全機能復旧確認、モニタリング強化 | SRE | ☐ |
| 解決後 24h | Postmortem 作成（[§A.7](#a7-postmortem-テンプレートipa-工程-115)） | IC + 関連者 | ☐ |

### A.6.5 連絡先

| ロール | 氏名 | 電話 | Slack |
|---|---|---|---|
| SRE Lead | | | |
| SRE on-call | PagerDuty | | |
| テックリード | | | |
| DBA | | | |
| SecO | | | |
| PM | | | |
| PO | | | |

### A.6.6 共通インシデント対応プレイブック

#### ノードダウン

1. アラート確認 → 該当ノード特定
2. クラスタから自動フェイルオーバー（[DOC-MOD-016](../modules/M-16-cluster-coordinator.md)）
3. ノード再起動 or 入れ替え
4. 復旧確認

#### DB 接続不可

1. 接続文字列確認
2. DB 稼働確認（`pg_isready`）
3. 接続プール状況確認
4. 再起動 or フェイルオーバー

#### メモリリーク

1. 該当プロセスのメモリ使用率確認
2. ヒープダンプ取得
3. プロセス再起動（暫定）
4. 原因分析（[Postmortem](#a7-postmortem-テンプレートipa-工程-115)）

#### ...（各障害パターン追記）

### A.6.7 完了基準

- 全 Sev に対応フロー定義
- 連絡先一覧最新化
- 四半期毎の訓練実施

---

## A.7 Postmortem テンプレート（IPA 工程 115）

### A.7.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-OPS-OPS-postmortem
Incident ID: INC-<YYYYMMDD>-<連番>
作成日: ____-__-__
作成者: <IC>
レビュアー: <SRE Lead + テックリード + PO>
公開レベル: ☐ 社内  ☐ 公開
参照: [§A.6 Incident Response](#a6-incident-response-runbookipa-工程-114)
```

### A.7.2 インシデントサマリ

| 項目 | 内容 |
|---|---|
| 発生日時 | ____-__-__ __:__ |
| 解決日時 | ____-__-__ __:__ |
| 復旧時間 | __h __min |
| 重大度 | ☐ Sev1 ☐ Sev2 ☐ Sev3 |
| 影響範囲 | <ユーザー数、機能> |
| 影響度 | <売上損失、信用低下> |

### A.7.3 タイムライン

| 時刻 | イベント |
|---|---|
| HH:MM | アラート発火 |
| HH:MM | 認識 |
| HH:MM | IC 任命 |
| HH:MM | 暫定対応 |
| HH:MM | 根本対応 |
| HH:MM | 復旧 |

### A.7.4 根本原因分析（5 Whys）

| Why | 回答 |
|---|---|
| Why 1 | 何が起きた？ |
| Why 2 | なぜ起きた？ |
| Why 3 | なぜ検知できなかった？ |
| Why 4 | なぜ防げなかった？ |
| Why 5 | なぜ再発防止策がなかった？ |

### A.7.5 影響と対策

| 種別 | 内容 |
|---|---|
| ユーザー影響 | |
| 直接原因 | |
| 根本原因 | |
| 暫定対応 | |
| 恒久対応 | |

### A.7.6 再発防止策（Action Items）

| AI ID | 対策 | 担当 | 期限 | 状態 |
|---|---|---|---|---|
| AI-NN | <対策> | <氏名> | YYYY-MM-DD | ☐ Open / ☐ Done |
| ... | ... | ... | ... | ... |

### A.7.7 学んだこと

- 良かった点: ...
- 改善点: ...
- 教訓: ...

### A.7.8 完了基準

- 5 Whys 完了
- 再発防止策に担当 + 期限設定
- 1 週間以内に PO 承認

---

## A.8 問題管理台帳（IPA 工程 116）

### A.8.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-OPS-OPS-problems
起票日: ____-__-__
起票者: <SRE>
更新頻度: 月次
```

### A.8.2 問題管理台帳

| Problem ID | タイトル | 関連 Incident | 重大度 | 根本原因 | 暫定対応 | 恒久対策 | 担当 | 期限 | 状態 |
|---|---|---|---|---|---|---|---|---|---|
| PRB-NNNN | <件名> | INC-XXX | Sev? | <原因> | <暫定> | <恒久> | <氏名> | YYYY-MM-DD | ☐ Open / ☐ In Progress / ☐ Resolved |

### A.8.3 完了基準

- 全 Incident に対応する Problem 起票
- 重大度「高」問題は 30 日以内に恒久対策

---

## A.9 問い合わせ対応記録（IPA 工程 117）

### A.9.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-OPS-OPS-support-<YYYYMMDD>
対象日: ____-__-__
対応者: <サポート>
SLA: 初回応答 ≤ 4h、解決 ≤ 24h（重大 ≤ 1h）
参照: [DOC-ARCH-005 §7](../architecture/05-admin-operations-ui.md)
```

### A.9.2 問い合わせ一覧

| Ticket ID | 受付時刻 | テナント | 重大度 | 件名 | カテゴリ | 初回応答時刻 | 解決時刻 | 対応者 | 状態 | 関連 |
|---|---|---|---|---|---|---|---|---|---|---|
| SUP-NNNN | HH:MM | <テナント> | P0/P1/P2 | <件名> | 機能/性能/エラー/その他 | HH:MM | HH:MM | <氏名> | ☐ Open / ☐ Resolved | |

### A.9.3 月次サマリ

| 項目 | 件数 |
|---|---|
| 総受付 | __ |
| SLA 内応答 | __% |
| SLA 内解決 | __% |
| P0 件数 | __ |
| FAQ 化候補 | __ |

### A.9.4 完了基準

- SLA 遵守率 ≥ 95%
- 月次で FAQ 反映

---

## 10. 用語集

| 用語 | 説明 | 出典 |
|---|---|---|
| 監視 | システム稼働状況の継続的観測 | ITIL |
| アラート | 閾値超過時の通知 | ITIL |
| ジョブ | 定期実行タスク | 本書 |
| Backup | データ複製による保護 | ITIL |
| RTO / RPO | 復旧時間 / 復旧時点目標 | DR |
| Capacity | 処理能力 | ITIL |
| Incident | サービス中断/品質低下事象 | ITIL |
| Problem | 根本原因が未特定の問題 | ITIL |
| Sev1〜4 | 重大度レベル | Google SRE |
| IC | Incident Commander | SRE |
| Postmortem | 障害後の根本原因分析 | Google SRE |
| 5 Whys | 5 段階のなぜなぜ分析 | Toyota |
| サポート | ユーザー問い合わせ対応 | ITIL |

---

## 11. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018 年 3 月
2. ITIL 4、AXELOS、2019 年
3. Google SRE Book 第 2 版、Google、2020 年
4. Ada プロジェクトチーム「[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md)」、2026-08-20
5. Ada プロジェクトチーム「[DOC-ARCH-004 原子化デプロイ](../architecture/04-atomic-deployment.md)」、2026-08-19
6. Ada プロジェクトチーム「[DOC-ARCH-005 管理画面](../architecture/05-admin-operations-ui.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
