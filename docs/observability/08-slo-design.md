# 08 SLO / SLI 設計（SLO Design）

> **信頼性の数値化**。可用性・レイテンシ・スループット・キャパシティを SLI/SLO で定義し、  
> Error Budget と Burn Rate で **残許容時間を継続監視** する。

> **ドキュメントID**：DOC-OBS-008
> **上位文書**：[DOC-OBS-INDEX](README.md)
> **下位文書**：[DOC-OBS-003 Metrics](03-metrics-design.md) / [DOC-OBS-007 Alert](07-alert-policy.md) / [DOC-OBS-013 Self-Audit](13-self-audit.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版（18 コンポーネント × 4 次元 SLI 体系） |

---

## 目次

1. 設計原則
2. SLI 定義 4 次元
3. SLO 目標値マトリクス
4. Error Budget 計算
5. Burn Rate アラート
6. コンプライアンスレポート
7. テナント別 SLO
8. SLO 改訂プロセス
9. 用語集
10. 参考文献

---

## 1. 設計原則

| 原則 | 説明 |
|---|---|
| **ユーザー視点** | 内部実装ではなく、エンドユーザーから観測可能な指標で SLI を定義 |
| **保守的目標** | 過去実績の 99.5%ile を超える範囲で設定（達成可能かつ挑戦的） |
| **継続計測** | 月次で達成率を測定し、未達なら是正アクション |
| **Burn Rate 連動** | SLO 消費速度でアラート、即時性が低いと重大事故になる |
| **テナント公平** | 全テナント同一 SLO、ティア別上乗せは Tier-SLA ドキュメントで |

## 2. SLI 定義 4 次元

| 次元 | SLI 名 | 計算式 | 計測ソース |
|---|---|---|---|
| **Availability** | `sli.availability` | 成功リクエスト数 / 総リクエスト数 | HTTP 5xx, gRPC UNAVAILABLE |
| **Latency** | `sli.latency.p99` | 99 パーセンタイルレイテンシ | OTel Span duration |
| **Throughput** | `sli.throughput` | 秒間処理リクエスト数 | RED メトリクス |
| **Capacity** | `sli.capacity.headroom` | 1 - (現在使用率 / 限界値) | node_exporter / DB metrics |

### 2.1 Availability SLI の計測方法

```yaml
# SLI: 5xx を「失敗」とする
sli_availability = 
  sum(rate(ada_app_requests_total{status!~"5.."}[28d]))
  /
  sum(rate(ada_app_requests_total[28d]))
```

### 2.2 Latency SLI の計測方法

```yaml
# SLI: 過去 28 日間の p99 レイテンシ
sli_latency_p99 = 
  histogram_quantile(0.99, 
    sum by (le, route) (rate(ada_app_request_duration_seconds_bucket[28d]))
  )
```

### 2.3 計測ウィンドウ

| SLI | 計測ウィンドウ | 理由 |
|---|---|---|
| Availability | 28 日ローリング | 月次 Error Budget と一致 |
| Latency | 28 日ローリング | 短すぎると外れ値に振られる |
| Throughput | 5 分ローリング | リアルタイム容量把握 |
| Capacity | 1 分ローリング | 突発的飽和を即時検知 |

## 3. SLO 目標値マトリクス

### 3.1 コンポーネント別 Availability SLO

| コンポーネント | クラスタ層 | SLO 目標 | Error Budget (28d) | 優先度 |
|---|---|---|---|---|
| **M-13 API Gateway** | 公開境界 | **99.9%** | 40 分 19 秒 | P0 |
| **M-03 Data Flow Engine** | コア | 99.5% | 3 時間 21 分 | P0 |
| **M-12 Canvas Editor (CRDT)** | コア | 99.5% | 3 時間 21 分 | P0 |
| **M-10 Tenant Middleware** | データ | 99.9% | 40 分 19 秒 | P0 |
| **M-15 Central Event Bus** | 神経系 | 99.95% | 20 分 10 秒 | P0 |
| **M-11 RBAC / Collab** | 認証 | 99.9% | 40 分 19 秒 | P0 |
| **M-01 Acquisition** | 入力 | 99.0% | 6 時間 43 分 | P1 |
| **M-02 Normalizer** | 入力 | 99.0% | 6 時間 43 分 | P1 |
| **M-04 Orchestration** | コア | 99.5% | 3 時間 21 分 | P1 |
| **M-05 Control Flow** | コア | 99.5% | 3 時間 21 分 | P1 |
| **M-06 Plugin SDK** | 拡張 | 99.0% | 6 時間 43 分 | P2 |
| **M-07 Debug** | 拡張 | 99.0% | 6 時間 43 分 | P2 |
| **M-08 Trigger** | 拡張 | 99.0% | 6 時間 43 分 | P2 |
| **M-09 Exporter** | 出力 | 99.0% | 6 時間 43 分 | P2 |
| **M-14 Module Registry** | メタ | 99.5% | 3 時間 21 分 | P1 |
| **M-16 Cluster Coordinator** | 制御 | 99.95% | 20 分 10 秒 | P0 |
| **PostgreSQL 16** | データ | 99.95% | 20 分 10 秒 | P0 |
| **Redis 7** | キャッシュ | 99.9% | 40 分 19 秒 | P1 |
| **Grafana / Prometheus / Loki / Tempo** | 観測基盤 | 99.9% | 40 分 19 秒 | P0 |

### 3.2 レイテンシ SLO

| コンポーネント | エンドポイント / 操作 | 目標レイテンシ | 計測方法 |
|---|---|---|---|
| M-13 API Gateway | 公開 REST API | **p99 < 500ms** | OTel HTTP span |
| M-13 API Gateway | WebSocket frame | p99 < 100ms | OTel WS span |
| M-03 Data Flow Engine | flow execute | p99 < 1s | OTel pipeline span |
| M-10 DB | 単一クエリ | p99 < 100ms | pg_stat_statements |
| M-10 DB | トランザクション | p95 < 200ms | OTel DB span |
| M-15 EventBus | メッセージ配信 lag | p99 < 1s | Kafka consumer lag |
| M-12 CRDT 同期 | Yrs ドキュメント更新 | p95 < 50ms | OTel CRDT span |
| M-11 認証 | JWT 検証 | p99 < 50ms | OTel auth span |

### 3.3 集約 Availability SLO（User Journey）

| ユーザージャーニー | 関係コンポーネント | 集約 SLO |
|---|---|---|
| **キャンバス起動** | Gateway + Auth + Tenant + Canvas | 99.5% |
| **データ取得** | Gateway + Acquire + Normalize + DB | 99.0% |
| **CRDT 共同編集** | Gateway + CRDT + EventBus + DB | 99.0% |
| **プラグイン実行** | Gateway + Plugin SDK + Orchestrator + DB | 99.0% |
| **データエクスポート** | Gateway + Exporter + Storage | 99.0% |

> **集約 SLO の計算**：各コンポーネント SLO の積（直列モデル）。  
> 例：キャンバス起動 = 0.999 × 0.999 × 0.999 × 0.999 ≈ 99.6%

## 4. Error Budget 計算

### 4.1 計算式

```
Error Budget (時間) = 計測ウィンドウ × (1 - SLO 目標)
                    = 28日 × 24時間 × (1 - SLO)
                    = 672時間 × (1 - SLO)
```

| SLO | 月間 Error Budget |
|---|---|
| 99.0% | 6 時間 43 分 |
| 99.5% | 3 時間 21 分 |
| 99.9% | 40 分 19 秒 |
| 99.95% | 20 分 10 秒 |
| 99.99% | 4 分 2 秒 |

### 4.2 Error Budget 残量ダッシュボード

```promql
# 過去 28 日間の Error Budget 消費率
error_budget_consumed_ratio = 
  1 - (
    sum(rate(ada_app_requests_total{status!~"5.."}[28d]))
    /
    sum(rate(ada_app_requests_total[28d]))
  ) / (1 - 0.999)

# 残予算（時間）
error_budget_remaining_hours = 
  (1 / 0.001) * 60 * 24 * 28 * (1 - error_budget_consumed_ratio)
```

### 4.3 Budget 残量によるアクション

| 残予算 | 状態 | アクション |
|---|---|---|
| **> 50%** | 🟢 健全 | 通常運用、機能開発 OK |
| **20-50%** | 🟡 注意 | 新機能リリースの凍結検討 |
| **5-20%** | 🟠 警戒 | 新機能凍結、信頼性改善に全力 |
| **< 5%** | 🔴 危険 | 全リリース停止、信頼性タスクのみ |
| **< 0%** | ⚫ 超過 | 緊急 RCA、信頼性回復まで機能開発停止 |

## 5. Burn Rate アラート

### 5.1 Burn Rate の意味

```
Burn Rate = (現在エラー率) / (SLO 許容エラー率)
```

| Burn Rate | 意味 | 28 日バジェット消費予測 |
|---|---|---|
| 1x | SLO 線上 | ちょうど 28 日で消費 |
| 2x | 2 倍速 | 14 日で消費 |
| 10x | 10 倍速 | 2.8 日で消費 |
| 100x | 100 倍速 | 6.7 時間で消費 |

### 5.2 マルチウィンドウ Burn Rate ルール

Google SRE Workbook 準拠。**短窓（高感度） + 長窓（低ノイズ）** の AND 条件で発火。

| アラート ID | 短窓 | 長窓 | Burn Rate | 重大度 | 通知 |
|---|---|---|---|---|---|
| **SLO-FAST-BURN-1h** | 1h | 5h | 14.4x | **Sev2** | 即時 |
| **SLO-FAST-BURN-6h** | 6h | 30h | 6x | **Sev2** | 即時 |
| **SLOW-BURN-24h** | 24h | 120h | 3x | **Sev3** | 30 分以内 |
| **SLOW-BURN-72h** | 72h | 360h | 1x | **Sev4** | 翌営業日 |

```yaml
# 例: SLO-FAST-BURN-1h
- alert: SLO_Availability_BurnRate_Fast_1h
  expr: |
    (
      sum(rate(ada_app_requests_total{status=~"5..", component="m13-gateway"}[1h]))
      /
      sum(rate(ada_app_requests_total{component="m13-gateway"}[1h]))
    ) > (14.4 * 0.001)
    and
    (
      sum(rate(ada_app_requests_total{status=~"5..", component="m13-gateway"}[5h]))
      /
      sum(rate(ada_app_requests_total{component="m13-gateway"}[5h]))
    ) > (14.4 * 0.001)
  for: 2m
  labels:
    severity: sev2
    slo: availability-m13-gateway
  annotations:
    summary: "M-13 Gateway SLO 高速消費（1h 窓で 14.4x Burn）"
    runbook: "https://wiki/runbooks/slo-fast-burn"
```

### 5.3 Latency Burn Rate

```yaml
# p99 レイテンシが SLO 目標の 2 倍を超えた状態が X 分継続
- alert: SLO_Latency_p99_BurnRate
  expr: |
    histogram_quantile(0.99, 
      sum by (le) (rate(ada_app_request_duration_seconds_bucket{slo_target="0.5"}[5m]))
    ) > 1.0
  for: 10m
```

## 6. コンプライアンスレポート

### 6.1 月次 SLO レポート

| 項目 | 集計方法 | 出力先 |
|---|---|---|
| 月次 Availability 実績 | 過去 30 日 SLI 平均 | Grafana Dashboard 90 + 月次 PDF |
| Error Budget 残量 | 残 / 当初 × 100% | 同上 |
| 未達 SLO | 目標 < 実績 のもの | SRE レビュー |
| インシデント寄与度 | 各 Sev の合計ダウンタイム | 月次 RCA |

### 6.2 SLA 報告（テナント向け）

Tier 別 SLA ドキュメント（UN-P0-02 完了後に確定）に従い、Enterprise テナントには月次 SLA レポートを提出。

| 項目 | 報告頻度 | 受領者 |
|---|---|---|
| 月次稼働率レポート | 月次 | Enterprise 顧客テナント管理者 |
| 四半期 SLA レビュー | 四半期 | 経営層 + 営業 |
| インシデント報告書 | Sev1/Sev2 発生時 | 該当テナント |

## 7. テナント別 SLO

### 7.1 ティア設計

| ティア | 対象 | Availability SLO | サポート |
|---|---|---|---|
| **Free** | 無料ユーザー | ベストエフォート（SLO 保証なし） | コミュニティ |
| **Standard** | 有料 | 99.5% | 営業日 9-18 |
| **Premium** | 中規模 | 99.9% | 24×7 |
| **Enterprise** | 大口 | 99.95% + カスタム SLO | 24×7 + 専任 CSM |

### 7.2 テナント別計測

```promql
# テナント別 Availability
sli_availability_by_tenant = 
  sum by (tenant_id) (rate(ada_app_requests_total{status!~"5.."}[28d]))
  /
  sum by (tenant_id) (rate(ada_app_requests_total[28d]))
```

**テナント分離ラベル**：
- `tenant_id`（ハッシュ化、SHA-256 先頭 8 文字）
- `tenant_tier` (free/standard/premium/enterprise)

## 8. SLO 改訂プロセス

### 8.1 改訂トリガー

| トリガー | 対応 |
|---|---|
| 過去 3 ヶ月連続 100% 達成 | 目標引き上げ検討 |
| 過去 3 ヶ月連続未達 | 信頼性改善 or 目標見直し |
| サービス構成変更 | 全 SLO 再評価 |
| ユーザーフィードバック | 妥当性レビュー |

### 8.2 改訂手順

```
G1. 提案書作成
 ↓ PO/PM レビュー
G2. 過去データ分析（最低 3 ヶ月）
 ↓ SRE レビュー
G3. ステークホルダー協議（営業 / 顧客 / 法務）
 ↓
G4. 新 SLO 承認（PO 決裁）
 ↓
G5. アラート閾値再計算
 ↓
G6. 4 週間告知期間（テナント向け）
 ↓
G7. 新 SLO 適用
```

### 8.3 改訂履歴管理

| 項目 | 保管場所 |
|---|---|
| SLO 改訂提案書 | `docs/observability/slo-changes/` |
| 過去 SLO 実績 | Prometheus long-term storage (Thanos / Mimir) |
| ステークホルダー協議議事録 | Confluence |

## 9. 用語集

| 用語 | 説明 |
|---|---|
| **SLI (Service Level Indicator)** | サービス品質を計測する指標 |
| **SLO (Service Level Objective)** | SLI の目標値 |
| **SLA (Service Level Agreement)** | SLO 未達時の契約上の補償を含む合意 |
| **Error Budget** | SLO 未達として許容される失敗量 |
| **Burn Rate** | Error Budget の消費速度（倍率） |
| **Fast Burn** | 短時間で大量にバジェットを消費する状態 |
| **Slow Burn** | 長時間かけて徐々にバジェットを消費する状態 |
| **Multi-Window** | 短窓と長窓の AND で評価する Burn Rate 方式 |
| **User Journey SLO** | 複数コンポーネントにまたがるユーザ視点の SLO |

## 10. 参考文献

1. Google SRE Workbook - Service Level Objectives  
   <https://sre.google/workbook/service-level-objectives/>
2. Google SRE Book Chapter 4 - Service Level Objectives  
   <https://sre.google/sre-book/service-level-objectives/>
3. Prometheus 公式 - Alerting on SLOs  
   <https://prometheus.io/docs/prometheus/latest/alerting/#alerting-on-slos>
4. OpenSLO Specification v1.0  
   <https://openslo.com/>
5. IPA 共通フレーム2018 SLCP-JCF2018 - SLA 設計指針

---

> **IPA 末尾注記**  
> 本ドキュメントは IPA 共通フレーム2018 (SLCP-JCF2018) および IPA 非機能要求グレード2018 に準拠する。  
> 記載の SLO 目標は初期値であり、運用実績および事業要件に応じて [SLO 改訂プロセス] に従い四半期ごとに見直す。  
> 商用利用前に PO（プロダクトオーナー）承認を必須とする。
