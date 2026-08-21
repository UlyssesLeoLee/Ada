# 11 段階的導入計画（Phased Rollout）

> **一度に全部入れない**。Phase 0 現状把握 → Phase 1 インフラ → Phase 2 アプリ Metrics → ...  
> 各 Phase で **価値提供 + 振り返り** を繰り返し、リスクを最小化しながら進める。

> **ドキュメントID**：DOC-OBS-011
> **上位文書**：[DOC-OBS-INDEX](README.md)
> **下位文書**：[DOC-OBS-010 Deployment](10-deployment-design.md) / [DOC-OBS-012 Code Impact](12-code-impact.md) / [DOC-ARCH-008 Workflow](D:/Ada/docs/architecture/08-workflow-overview.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版（Phase 0-8、9 ヶ月計画） |

---

## 目次

1. 全体ロードマップ
2. Phase 0: 現状調査
3. Phase 1: インフラ監視
4. Phase 2: アプリ Metrics
5. Phase 3: ログ集中化
6. Phase 4: Distributed Trace
7. Phase 5: Dashboard 完成
8. Phase 6: Alert
9. Phase 7: SLO
10. Phase 8: 自動化運用
11. フェーズ移行判定（GATE）
12. ロールバック計画
13. 用語集
14. 参考文献

---

## 1. 全体ロードマップ

```
Month 1     Month 2     Month 3     Month 4     Month 5     Month 6     Month 7     Month 8     Month 9
  │           │           │           │           │           │           │           │           │
  ▼           ▼           ▼           ▼           ▼           ▼           ▼           ▼           ▼
┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐
│P0   │──▶│P1   │──▶│P2   │──▶│P3   │──▶│P4   │──▶│P5   │──▶│P6   │──▶│P7   │──▶│P8   │
│調査 │   │Infra│   │App  │   │Log  │   │Trace│   │Dash │   │Alert│   │SLO  │   │Auto │
└─────┘   └─────┘   └─────┘   └─────┘   └─────┘   └─────┘   └─────┘   └─────┘   └─────┘
```

| Phase | 主題 | 期間 | 主要マイルストーン | 成果物 |
|---|---|---|---|---|
| **P0** | 現状調査 | W1-W2 | 現状分析レポート | DOC-OBS-001 |
| **P1** | インフラ監視 | W3-W5 | Node/K8s メトリクス稼働 | Dashboard 10/20 |
| **P2** | アプリ Metrics | W6-W9 | RED メトリクス全 18 crate | Dashboard 30 |
| **P3** | ログ集中化 | W10-W12 | Loki 集中ログ稼働 | Dashboard 30 + Log Explorer |
| **P4** | Distributed Trace | W13-W16 | OTel Trace 全 crate | Dashboard 30 + Trace UI |
| **P5** | Dashboard 完成 | W17-W18 | 10 Dashboard 完成 | Dashboard 00-90 |
| **P6** | Alert | W19-W21 | 30+ アラート稼働 | Alert Policy 適用 |
| **P7** | SLO | W22-W24 | Error Budget 計測 | SLO レポート |
| **P8** | 自動化運用 | W25-W36 | Runbook 自動化、Auto-remediation | ChatOps + Auto-scaling |

## 2. Phase 0: 現状調査

### 2.1 目標

業務サービスを変更せず、**観測の現状を正確に把握** する。

### 2.2 作業

| タスク | 担当 | 期間 | 成果物 |
|---|---|---|---|
| サービス一覧化 | SRE | 1 日 | `service-inventory.md` |
| 既存監視棚卸し | SRE | 1 日 | 既存 exporter / dashboard 一覧 |
| 通信フロー図作成 | SRE + Dev | 2 日 | システムトポロジ図 |
| ログ / トレース現状 | SRE | 1 日 | `logs-traces-current.md` |
| アラート現状 | SRE | 1 日 | `alerts-current.md` |
| 非機能要求の確認 | SRE + PM | 1 日 | DOC-REQ-005 NFR 参照 |
| 容量 / コスト試算 | SRE | 1 日 | `cost-estimate.md` |
| ドキュメント | SRE | 1 日 | **DOC-OBS-001**（本ドキュメント 01 章） |

### 2.3 完了基準（G0 ゲート）

- [x] 既存サービス / Pod / Node 数が把握されている
- [x] 既存 exporter がリスト化されている
- [x] システムトポロジ図が完成
- [x] 既存ログ / トレースの有無が確認されている
- [x] 既存アラート（ある場合）が把握されている
- [x] NFR（DOC-REQ-005）と現状の差分が明確
- [x] **DOC-OBS-001（01-current-state-analysis.md）公開**

### 2.4 リスク

| リスク | 対策 |
|---|---|
| サービス一覧が不完全 | `kubectl get` + namespace 全走査で確認 |
| 既存 exporter が Prometheus 形式でない | 移行計画を Phase 1 後半に組み込む |

## 3. Phase 1: インフラ監視

### 3.1 目標

**ノード / K8s / コンテナ**の健全性を可視化する。  
業務サービスには**ゼロインパクト**で導入する。

### 3.2 作業

| タスク | 担当 | 期間 | 成果物 |
|---|---|---|---|
| `observability` namespace 作成 | SRE | 0.5 日 | Namespace YAML |
| Prometheus Operator デプロイ | SRE | 1 日 | Prometheus Pod 稼働 |
| node-exporter DaemonSet | SRE | 0.5 日 | 全ノードで exporter 稼働 |
| kube-state-metrics | SRE | 0.5 日 | K8s リソースメトリクス |
| cAdvisor (K8s 標準) 確認 | SRE | 0.5 日 | コンテナメトリクス |
| Grafana デプロイ + 初期接続 | SRE | 1 日 | Grafana UI アクセス可能 |
| Dashboard 10 (Infra) 作成 | SRE | 2 日 | Node / Disk / Network 表示 |
| Dashboard 20 (K8s) 作成 | SRE | 2 日 | Pod / Deployment 表示 |
| アラート 5 件投入（基本） | SRE | 1 日 | Node Down, Disk Full 等 |

### 3.3 完了基準（G1 ゲート）

- [x] 全 K8s ノードで node-exporter 稼働
- [x] Grafana でノード CPU / Memory / Disk / Network が見れる
- [x] Grafana で Pod / Deployment 状態が見れる
- [x] 業務サービスに**ゼロインパクト**で導入完了
- [x] 基本アラート 5 件稼働

### 3.4 検証

```bash
# 1. ノードメトリクス確認
curl -s http://prometheus.observability/api/v1/query?query=up | jq

# 2. K8s メトリクス確認
curl -s 'http://prometheus.observability/api/v1/query?query=kube_pod_info' | jq

# 3. 業務サービス影響確認
# 業務 namespace の Pod に変化がないことを確認
kubectl -n ada get pods
```

### 3.5 リスク

| リスク | 対策 |
|---|---|
| node-exporter が業務ノードのリソースを消費 | requests: 100m / 256Mi, limits: 200m / 512Mi |
| 既存 exporter との衝突 | port 9100 が空いているか事前確認 |

## 4. Phase 2: アプリ Metrics

### 4.1 目標

**全 18 crate で RED メトリクスを出力**。OTel SDK を Rust コードに組み込む。

### 4.2 作業

| タスク | 担当 | 期間 | 成果物 |
|---|---|---|---|
| `ada-telemetry` crate 完成 | SRE + Dev | 1 週 | OTel SDK ラッパー |
| M-13 API Gateway に組み込み | Dev (M-13) | 2 日 | RED メトリクス出力 |
| M-03 Data Flow Engine に組み込み | Dev (M-03) | 3 日 | RED + 内部スパンメトリクス |
| M-10 Tenant Middleware に組み込み | Dev (M-10) | 2 日 | DB クエリレイテンシ含む |
| M-15 EventBus に組み込み | Dev (M-15) | 2 日 | Producer/Consumer メトリクス |
| その他 12 crate に組み込み | 各担当 | 1 週 | 順次 |
| 共通ラベル規約の適用 | SRE | 0.5 日 | service, version, env, instance |
| Dashboard 30 (App) 作成 | SRE | 3 日 | RED メトリクス可視化 |

### 4.3 完了基準（G2 ゲート）

- [x] 全 18 crate から OTel 経由でメトリクス送信
- [x] `ada_app_requests_total` メトリクスが全 HTTP サービスから出力
- [x] `ada_app_request_duration_seconds` ヒストグラムが全サービスから出力
- [x] 共通ラベル規約が 100% 適用
- [x] Dashboard 30 で全アプリの Request / Error / Latency が見れる
- [x] **業務サービスレイテンシ p99 増加が 5% 以内**

### 4.4 検証

```bash
# 1. サービス数確認
curl -s 'http://prometheus.observability/api/v1/query?query=count(count by (service)(ada_app_requests_total))' | jq

# 2. レイテンシ影響測定
# Phase 1 と Phase 2 後で M-13 Gateway の p99 比較
# 5% 以内の増加を許容

# 3. CPU/Memory 影響
kubectl -n ada top pods
```

### 4.5 リスク

| リスク | 対策 |
|---|---|
| OTel SDK 導入でレイテンシ増 | async batch export、sampling 調整 |
| メトリクス Cardinality 爆発 | 静的ラベル厳格化、ハイカーディナリ除外 |
| 既存 logging との衝突 | 段階的移行、feature flag |

## 5. Phase 3: ログ集中化

### 5.1 目標

**全サービスの構造化ログを Loki 集中化**。PII 自動 redaction 付き。

### 5.2 作業

| タスク | 担当 | 期間 | 成果物 |
|---|---|---|---|
| Loki デプロイ | SRE | 2 日 | Loki StatefulSet 稼働 |
| Promtail / Fluent Bit DaemonSet | SRE | 1 日 | ログ収集 |
| OTel → Loki exporter 経路 | SRE | 1 日 | アプリログの OTel 経由 |
| PII 自動 redaction 実装 | Dev + SRE | 3 日 | `ada-telemetry` redact 機能 |
| ログレベル統一 | Dev (各 crate) | 1 週 | info/warn/error/debug |
| LogQL クエリ整備 | SRE | 2 日 | テンプレ 10 種 |
| Grafana Loki データソース | SRE | 0.5 日 | ログパネル |
| アラート 5 件追加 | SRE | 1 日 | Error rate log, PII detection |

### 5.3 完了基準（G3 ゲート）

- [x] 全 18 crate のログが Loki に到達
- [x] PII 自動 redaction が CI で検証（password, email 等検出ゼロ）
- [x] LogQL で 5 分以内にエラー原因ログを検索可能
- [x] **ログ量が想定範囲内**（日次 < 100 GB / 全サービス）
- [x] 業務サービスディスク使用量変化 < 10%

### 5.4 検証

```bash
# 1. ログ受信確認
curl -s 'http://loki.observability/loki/api/v1/query?query={service="m13-gateway"}' | jq '.data.result | length'

# 2. PII 検出 CI
# CI: regex scanner on logs
./scripts/pii-detect.sh logs/  # 0 件であるべき

# 3. 業務サービス影響
df -h /var/lib/containerd
```

### 5.5 リスク

| リスク | 対策 |
|---|---|
| ログ爆発（日 1TB+） | rate limit + sampling + log level 制御 |
| PII 漏洩 | CI 検証 + 自動 redaction + 監査 |
| 既存ログの移行 | 旧 syslog は 30 日で廃止、新形式に統一 |

## 6. Phase 4: Distributed Trace

### 6.1 目標

**リクエストの End-to-End 追跡**。  
Gateway → Service → DB まで全 Span を Tempo に集約。

### 6.2 作業

| タスク | 担当 | 期間 | 成果物 |
|---|---|---|---|
| Tempo デプロイ | SRE | 2 日 | Tempo 稼働 |
| OTel Collector → Tempo 経路 | SRE | 1 日 | OTLP exporter 設定 |
| M-13 Gateway に Span 出力 | Dev (M-13) | 2 日 | trace_id 生成 + 伝播 |
| W3C Trace Context 適用 | Dev (各 crate) | 1 週 | 全 crate トレースヘッダー伝播 |
| 内部関数 Span 追加 | Dev (各 crate) | 1 週 | 重要パスのみ（自動計装） |
| DB Span 計装 | Dev | 3 日 | sqlx / tokio-postgres 計装 |
| Sampling 戦略適用 | SRE | 1 日 | Head 10% + tail 100% errors |
| Grafana Tempo データソース | SRE | 0.5 日 | Trace UI |
| アラート 3 件追加 | SRE | 1 日 | 長時間トレース検出 |

### 6.3 完了基準（G4 ゲート）

- [x] M-13 → M-03 → M-10 → DB の Trace が Grafana で可視化
- [x] p99 レイテンシ時の Span が確認可能
- [x] **業務サービスレイテンシ p99 増加 3% 以内**
- [x] トレースデータ量 < 500 GB / 日
- [x] Sampling 戦略が稼働（DB ヒット率調整済み）

### 6.4 検証

```bash
# 1. Trace 数確認
curl -s 'http://tempo.observability/api/search?tags=service.name%3Dm13-gateway&limit=10' | jq

# 2. 業務レイテンシ比較
# Phase 3 完了時 vs Phase 4 完了時の M-13 p99

# 3. トレースデータ量
curl -s 'http://loki.observability/loki/api/v1/query?query=sum(rate(tempo_spans_received_total[1h]))' | jq
```

### 6.5 リスク

| リスク | 対策 |
|---|---|
| Trace 爆発（日 10M+） | サンプリング厳格化、重要パスのみ tail sampling |
| DB クエリトレースで PII 露出 | sqlx 計装でクエリパラメータ redact |
| 既存アプリへの計装コスト | OTel auto-instrumentation 優先 |

## 7. Phase 5: Dashboard 完成

### 7.1 目標

**10 個の Dashboard を完成**し、障害切り分けに必要な全情報を提供。

### 7.2 作業

| タスク | 担当 | 期間 | 成果物 |
|---|---|---|---|
| Dashboard 00 Overview 完成 | SRE | 1 日 | 全 SLO 状態一覧 |
| Dashboard 10 Infra 拡張 | SRE | 1 日 | ノード詳細、容量予測 |
| Dashboard 20 K8s 拡張 | SRE | 1 日 | Deployment / HPA 状況 |
| Dashboard 30 App 完成 | SRE | 2 日 | RED + 内部状態 |
| Dashboard 40 DB 完成 | SRE | 2 日 | PostgreSQL 詳細 |
| Dashboard 50 Middleware | SRE | 1 日 | Redis / Kafka 状況 |
| Dashboard 60 Network | SRE | 1 日 | Ingress / 内部通信 |
| Dashboard 70 Performance | SRE | 1 日 | p99 / キャッシュヒット率 |
| Dashboard 80 Security | SRE | 1 日 | 認証失敗 / PII 検出 |
| Dashboard 90 SLO | SRE | 1 日 | Error Budget 残量 |
| 障害切り分けテスト | SRE + Dev | 2 日 | シナリオ 5 件で検証 |

### 7.3 完了基準（G5 ゲート）

- [x] 10 個の Dashboard 全て稼働
- [x] 各 Dashboard が「正常 / 異常 / 影響範囲 / 次の手」を回答
- [x] 障害切り分けテスト 5 シナリオ全てで原因特定 15 分以内

### 7.4 検証

| シナリオ | 想定 | 検証 |
|---|---|---|
| ノード 1 台停止 | 業務影響 | Dashboard 10/20 で 5 分以内に特定 |
| DB スロークエリ | API 遅延 | Dashboard 30/40 で原因クエリ特定 |
| 1 サービス高エラー率 | SLO 違反 | Dashboard 30/90 で影響範囲特定 |
| メモリリーク | 緩慢な性能劣化 | Dashboard 30 で検出 |
| ネットワーク分断 | 接続失敗 | Dashboard 60 で分断点特定 |

## 8. Phase 6: Alert

### 8.1 目標

**30+ のアクション可能アラート**を稼働。誤報ゼロ、5 分以内初動。

### 8.2 作業

| タスク | 担当 | 期間 | 成果物 |
|---|---|---|---|
| アラートルール作成（30+ 件） | SRE | 1 週 | `alerts/*.yaml` |
| Burn Rate アラート | SRE | 2 日 | DOC-OBS-008 参照 |
| AlertManager 通知設定 | SRE | 1 日 | Slack / PagerDuty |
| Inhibition ルール | SRE | 1 日 | 親アラートで子抑制 |
| Runbook 整備 | SRE + Dev | 1 週 | 各アラートに runbook URL |
| エスカレーション試験 | SRE | 2 日 | Sev1/Sev2 通知テスト |
| 誤報ゼロ検証 | SRE | 3 日 | 1 週間運用で誤報 < 1% |

### 8.3 完了基準（G6 ゲート）

- [x] 30+ アクション可能アラート稼働
- [x] 4 段階 Sev 分類適用
- [x] 通知テスト 100% 成功
- [x] 1 週間運用で誤報 < 1%
- [x] 全アラートに Runbook リンク

## 9. Phase 7: SLO

### 9.1 目標

**SLO/SLI で信頼性を数値化**。Error Budget 残量を月次レポート化。

### 9.2 作業

| タスク | 担当 | 期間 | 成果物 |
|---|---|---|---|
| SLI 計測設定 | SRE | 3 日 | 4 次元 SLI 実装 |
| SLO 目標値設定 | SRE + PM | 2 日 | DOC-OBS-008 目標値承認 |
| Burn Rate アラート | SRE | 1 日 | Fast / Slow Burn |
| Error Budget 残量 | SRE | 1 日 | 月次レポートテンプレ |
| ティア別 SLO | SRE + Sales | 2 日 | Tier 別目標値 |
| 月次 SLO レポート運用 | SRE + PM | 継続 | 月次 PDF 自動生成 |

### 9.3 完了基準（G7 ゲート）

- [x] 全 18 コンポーネントに SLI 計測稼働
- [x] Error Budget が Dashboard 90 で可視化
- [x] 月次 SLO レポート自動生成
- [x] 4 種類の Burn Rate アラート稼働

## 10. Phase 8: 自動化運用

### 10.1 目標

**運用自動化 + Auto-remediation**。手動運用を極小化。

### 10.2 作業

| タスク | 担当 | 期間 | 成果物 |
|---|---|---|---|
| ChatOps 統合 | SRE | 1 週 | Slack からクエリ / 対応 |
| Auto-scaling チューニング | SRE | 1 週 | HPA 設定最適化 |
| Runbook 自動化 | SRE | 2 週 | 5 シナリオ自動対応 |
| Auto-remediation | SRE | 2 週 | セルフ healing |
| 定期レポート自動配信 | SRE | 1 週 | 日次 / 週次 / 月次 |
| 容量予測 | SRE + Data | 2 週 | ML ベース予測 |
| コスト最適化 | SRE | 1 週 | 未使用 exporter 削減等 |
| 訓練（DR 訓練） | SRE + 全員 | 1 週 | 四半期訓練実施 |

### 10.3 完了基準（G8 ゲート）

- [x] アラート対応の 70% が自動化
- [x] 手動運用タスク < 5 / 日
- [x] 容量予測モデル稼働
- [x] DR 訓練完了

## 11. フェーズ移行判定（GATE）

各フェーズ移行時に以下を判定：

| 項目 | 判定基準 |
|---|---|
| **完了基準** | 該当 Phase のチェックリスト全て達成 |
| **性能影響** | 業務サービス p99 レイテンシ増加 < 5% |
| **安定性** | 1 週間運用で Sev2 以上 0 件 |
| **誤報率** | 該当 Phase 追加アラートの誤報 < 1% |
| **ドキュメント** | 関連ドキュメント更新済み |
| **次 Phase 準備** | 次フェーズで必要な前提が満たされている |

判定 NG の場合：
- 該当 Phase の改善 → 再判定
- 2 回連続 NG → Phase 計画自体を見直し

## 12. ロールバック計画

各 Phase で問題発生時のロールバック：

| Phase | ロールバック方法 |
|---|---|
| **P1** | exporter DaemonSet 削除、業務影響なし |
| **P2** | OTel SDK feature flag OFF、5 分以内に旧コードに復帰 |
| **P3** | Promtail DaemonSet 削除 + Loki 停止、5 分以内 |
| **P4** | OTel SDK tracing feature OFF、5 分以内 |
| **P5** | Dashboard JSON ロールバック（Git 復元） |
| **P6** | Alert rule 無効化、AlertManager 再起動 |
| **P7** | SLO Alert ルール無効化 |
| **P8** | Auto-remediation CronJob 停止 |

ロールバック時間目標：**RTO 15 分**（業務影響なし）

## 13. 用語集

| 用語 | 説明 |
|---|---|
| **Phase** | 段階的導入の各ステップ |
| **GATE** | フェーズ移行判定 |
| **W (Week)** | 週単位期間 |
| **誤報率** | アラート発火のうち誤りだった割合 |
| **Auto-remediation** | 自動修復 |
| **ChatOps** | Slack 等から運用操作する方式 |
| **DR 訓練** | Disaster Recovery 訓練 |
| **RTO** | Recovery Time Objective |
| **Sampling** | トレース採取率調整 |
| **Burn Rate** | Error Budget 消費速度 |

## 14. 参考文献

1. Google SRE Book - Implementing SLOs  
   <https://sre.google/workbook/implementing-slos/>
2. OpenTelemetry Migration Guide  
   <https://opentelemetry.io/docs/migration/>
3. Grafana Adoption Journey  
   <https://grafana.com/docs/grafana/latest/adoption/>
4. Prometheus Migration Guide  
   <https://prometheus.io/docs/prometheus/latest/storage/>
5. IPA 共通フレーム2018 6.3 導入プロセス

---

> **IPA 末尾注記**  
> 本ドキュメントは IPA 共通フレーム2018 (SLCP-JCF2018) 第 7 章「システム化計画の立案」に準拠する。  
> 9 ヶ月の段階的導入により、リスクを最小化しながら観測基盤を完成させる。  
> 各 Phase の完了基準達成が次 Phase 移行の必要条件である。  
> PO（プロダクトオーナー）の承認と SRE Lead の技術承認を必須とする。
