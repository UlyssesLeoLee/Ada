# 07 アラートポリシー（Alert Policy）

> 単純な閾値超過ではなく、**持続時間 + 多指標 + SLO Burn Rate** で評価。  
> 4 段階（Sev1〜4）で分類し、**誤報ゼロ** を目指す。

> **ドキュメントID**：DOC-OBS-007
> **上位文書**：[DOC-OBS-INDEX](README.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版 |

---

## 目次

1. 設計原則
2. 4 段階重大度
3. アラート評価ロジック
4. アラート一覧
5. 通知先
6. エスカレーション
7. 抑制（Inhibition）
8. アラートライフサイクル
9. 用語集

---

## 1. 設計原則

| 原則 | 説明 |
|---|---|
| **誤報ゼロ** | 単純な閾値超過ではなく、持続時間 + 関連指標で評価 |
| **行動可能** | アラート受信 → 30 分以内に初動可能 |
| **多指標相関** | 1 指標で発火せず、関連指標で裏取り |
| **Burn Rate ベース** | SLO Error Budget 消費率で評価 |
| **抑制** | 親アラートで子アラート抑制、誤報連鎖防止 |

## 2. 4 段階重大度

| 重大度 | 定義 | 応答時間 | 通知先 |
|---|---|---|---|
| **Sev1** | サービス全停止、データ損失リスク | **5 分** | 電話（PagerDuty）+ 経営層 |
| **Sev2** | 主要機能停止、SLO 違反 | **30 分** | PagerDuty + Slack #incident |
| **Sev3** | 部分機能低下、パフォーマンス劣化 | **1 時間** | Slack #alerts |
| **Sev4** | 注意、警告レベル | **翌営業日** | Slack #alerts-info |

## 3. アラート評価ロジック

### 3.1 単一指標アラート（基本）

```yaml
# 例: API エラー率 > 5% が 5 分継続
- alert: HighErrorRate
  expr: |
    sum(rate(ada_app_api_gateway_requests_total{status=~"5.."}[5m]))
    /
    sum(rate(ada_app_api_gateway_requests_total[5m]))
    > 0.05
  for: 5m
  labels:
    severity: sev2
  annotations:
    summary: "API エラー率 5% 超過"
    description: "{{ $labels.service }} のエラー率が {{ $value | humanizePercentage }} で 5 分以上継続"
```

### 3.2 多指標相関アラート

```yaml
# 例: Latency 増加 + Error 増加 の組み合わせ
- alert: ServiceDegradation
  expr: |
    (histogram_quantile(0.99, sum by (le) (rate(ada_app_api_gateway_request_duration_seconds_bucket[5m]))) > 1.0)
    and
    (sum(rate(ada_app_api_gateway_requests_total{status=~"5.."}[5m])) / sum(rate(ada_app_api_gateway_requests_total[5m])) > 0.01)
  for: 10m
  labels:
    severity: sev2
  annotations:
    summary: "サービス劣化（Latency + Error 両方）"
    description: "Latency p99 > 1s かつ Error > 1% が 10 分継続"
```

### 3.3 SLO Burn Rate アラート

```yaml
# 例: M-13 Gateway availability SLO (99.9%) burn rate
# 1h で 14.4x burn = 月予算の 2% を 1h で消費
- alert: SLIBurnRateHigh
  expr: |
    (
      sum(rate(ada_app_api_gateway_requests_total{status=~"5.."}[1h]))
      /
      sum(rate(ada_app_api_gateway_requests_total[1h]))
      > (1 - 0.999) * 14.4
    )
    and
    (
      sum(rate(ada_app_api_gateway_requests_total{status=~"5.."}[6h]))
      /
      sum(rate(ada_app_api_gateway_requests_total[6h]))
      > (1 - 0.999) * 6
    )
  for: 5m
  labels:
    severity: sev1
  annotations:
    summary: "M-13 API Gateway SLO 予算 1h で 14.4x 消費"
```

## 4. アラート一覧

### 4.1 Sev1（即時対応）

| ID | アラート | 条件 | 応答 |
|---|---|---|---|
| ALT-001 | ServiceDown | `up == 0 for 1m` | on-call 即時 |
| ALT-002 | HighErrorRateSev1 | error rate > 10% for 2m | on-call 即時 |
| ALT-003 | DataLossRisk | `audit_log_writes_fail` > 0 for 5m | on-call 即時 |
| ALT-004 | ClusterSplitBrain | `split_brain_detected_total` > 0 for 0s | on-call 即時 |
| ALT-005 | SLIBurnRateFast | burn rate 14.4× for 5m | on-call 即時 |
| ALT-006 | DatabaseDown | `pg_up == 0 for 30s` | on-call 即時 |
| ALT-007 | GDPRDataExposure | `auth_failed` 多数 + `gdpr_*` ログ異常 | on-call 即時 |

### 4.2 Sev2（30 分以内）

| ID | アラート | 条件 | 応答 |
|---|---|---|---|
| ALT-101 | HighErrorRate | error rate > 1% for 10m | on-call 30min |
| ALT-102 | HighLatency | p99 > 1s for 10m | on-call 30min |
| ALT-103 | SLIBurnRateMedium | burn rate 6× for 30m | on-call 30min |
| ALT-104 | PodRestartLoop | restart > 5 in 1h | on-call 30min |
| ALT-105 | OOMKilled | OOMKilled > 0 for 5m | on-call 30min |
| ALT-106 | DiskSpaceLow | disk > 80% for 30m | on-call 30min |
| ALT-107 | DBConnectionPoolExhausted | active / max > 90% for 5m | on-call 30min |
| ALT-108 | EventBusLag | p99 lag > 5s for 10m | on-call 30min |
| ALT-109 | AtomicSwapFail | atomic swap fail rate > 0 for 5m | on-call 30min |
| ALT-110 | ReplicationLag | `pg_replication_lag` > 30s for 5m | on-call 30min |
| ALT-111 | AuthFailureSpike | auth fail rate > 10× baseline | on-call 30min |
| ALT-112 | RLSViolationSpike | RLS deny > 100 in 5m | on-call 30min |

### 4.3 Sev3（1 時間以内）

| ID | アラート | 条件 | 応答 |
|---|---|---|---|
| ALT-201 | ModerateErrorRate | error rate > 0.1% for 30m | 1h |
| ALT-202 | ModerateLatency | p99 > 500ms for 30m | 1h |
| ALT-203 | CPUHigh | CPU > 80% for 30m | 1h |
| ALT-204 | MemoryHigh | memory > 80% for 30m | 1h |
| ALT-205 | DiskIOHigh | disk I/O > 80% for 30m | 1h |
| ALT-206 | BackupFailed | backup fail for 1h | 1h |
| ALT-207 | LeaderFlapping | leader change > 3 in 1h | 1h |
| ALT-208 | CRDTConflicts | conflict > 100 in 5m | 1h |
| ALT-209 | SlowQueries | slow query > 10/s for 30m | 1h |
| ALT-210 | ConnectionPoolHigh | active / max > 70% for 30m | 1h |

### 4.4 Sev4（翌営業日）

| ID | アラート | 条件 | 応答 |
|---|---|---|---|
| ALT-301 | CertificateExpiringSoon | cert expiry < 30 days | 翌営業日 |
| ALT-302 | DiskSpaceWarning | disk > 60% for 1d | 翌営業日 |
| ALT-303 | VulnerabilityDetected | vuln detected, not critical | 翌営業日 |
| ALT-304 | DependencyUpdate | 依存 crate 新バージョン | 翌営業日 |
| ALT-305 | AnomalousTraffic | 平常時より ±50% 乖離 | 翌営業日 |

## 5. 通知先

| 重大度 | 電話 | Slack | Email | PagerDuty |
|---|---|---|---|---|
| Sev1 | ✅ | #incident | ✅ | ✅ Critical |
| Sev2 | — | #incident | ✅ | ✅ High |
| Sev3 | — | #alerts | ✅ | ✅ Low |
| Sev4 | — | #alerts-info | — | — |

### 5.1 連絡網

```
Sev1 → PagerDuty → On-call SRE → (5min 応答なし) → PM → (15min) → 経営層
Sev2 → PagerDuty → On-call SRE → (30min 応答なし) → PM
Sev3 → Slack → 当番 SRE → (1h 応答なし) → チーム全体
Sev4 → Slack → 次回 standup で確認
```

## 6. エスカレーション

| 経過時間 | アクション |
|---|---|
| 0 分 | アラート発火、on-call 通知 |
| 5 分 | on-call 応答なし → PM 通知 |
| 15 分 | PM 応答なし → 経営層通知（Sev1 のみ） |
| 30 分 | Sev2 → Sev1 エスカレーション検討 |
| 1 時間 | Postmortem 開始判断 |
| 24 時間 | Postmortem 完了（[Postmortem テンプレート](../templates/05-operations.md#a7-postmortem-テンプレートipa-工程-115) 使用） |
| 5 営業日 | 恒久対策 PR |

## 7. 抑制（Inhibition）

### 7.1 Alertmanager 抑制ルール

```yaml
# alertmanager.yaml
inhibit_rules:
  # サービスダウン中は個別アラートを抑制
  - source_matchers: [alertname="ServiceDown"]
    target_matchers: [severity=~"sev[23]"]
    equal: [service]

  # クラスタ全体で障害中は個別 Pod アラートを抑制
  - source_matchers: [alertname="ClusterDegraded"]
    target_matchers: [alertname=~"Pod.*"]
    equal: [namespace]

  # DB ダウン中は DB 関連アラートを抑制
  - source_matchers: [alertname="DatabaseDown"]
    target_matchers: [alertname=~"DB.*"]
    equal: [database]

  # 計画メンテナンス中は全アラートを抑制
  - source_matchers: [alertname="PlannedMaintenance"]
    target_matchers: [severity=~"sev[1-4]"]
    equal: [cluster]
```

## 8. アラートライフサイクル

| 状態 | 説明 |
|---|---|
| **inactive** | 発火条件未充足 |
| **pending** | 発火条件充足、`for` 期間内 |
| **firing** | `for` 期間経過、通知済み |
| **resolved** | 発火条件未充足に戻る |
| **suppressed** | 抑制ルールで通知停止 |

### 8.1 アラート定義フォーマット

```yaml
# prometheus-alerts/ada-{service}.yaml
groups:
  - name: ada.{service}
    interval: 30s
    rules:
      - alert: {AlertName}
        expr: {PromQL_expression}
        for: {duration}
        labels:
          severity: {sev1|sev2|sev3|sev4}
          service: {crate_name}
          team: ada-platform
        annotations:
          summary: "{short_description}"
          description: "{detailed_description_with_labels}"
          runbook_url: "https://wiki.ada.kanvas.dev/runbooks/{alert_name}"
          dashboard_url: "https://grafana.ada.kanvas.dev/d/{dashboard_id}"
          trace_query: "{trace_query_for_explore}"
          log_query: "{log_query_for_explore}"
```

### 8.2 アラートに必要な情報（必須）

各アラートは以下を含む：

| 項目 | 説明 |
|---|---|
| トリガー条件 | 発火 PromQL |
| 影響 | ユーザー影響範囲 |
| 確認方法 | Grafana ダッシュボード URL |
| トレース深掘り | Tempo 検索クエリ |
| ログ深掘り | Loki 検索クエリ |
| 排查手順 | runbook URL |
| 復旧手順 | 自動 or 手動 |
| エスカレーション | 連絡先 |

## 9. 推奨実装

### 9.1 アラート階層

```
┌─────────────────────────────────────────┐
│ Alertmanager (Grafana)                  │
│   - 重複排除                            │
│   - グループ化                           │
│   - 抑制 (Inhibition)                   │
│   - 沈黙 (Silences)                      │
└─────────────────┬───────────────────────┘
                  │
        ┌─────────┼─────────┐
        ▼         ▼         ▼
   PagerDuty   Slack    Email
        │
        ▼
   on-call SRE
```

### 9.2 推奨構成

```yaml
# grafana-alerting/alertmanager.yaml
route:
  receiver: 'default'
  group_by: ['alertname', 'service', 'severity']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  routes:
    - matchers: [severity="sev1"]
      receiver: 'pagerduty-critical'
      group_wait: 10s
      repeat_interval: 1h
    - matchers: [severity="sev2"]
      receiver: 'pagerduty-high'
      repeat_interval: 2h
    - matchers: [severity="sev3"]
      receiver: 'slack-alerts'
    - matchers: [severity="sev4"]
      receiver: 'slack-info'

receivers:
  - name: 'default'
    slack_configs:
      - channel: '#alerts-info'
        title: '{{ .GroupLabels.alertname }}'
        text: '{{ .CommonAnnotations.description }}'
  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: '<PAGERDUTY_KEY>'
        severity: 'critical'
  - name: 'pagerduty-high'
    pagerduty_configs:
      - service_key: '<PAGERDUTY_KEY>'
        severity: 'error'
  - name: 'slack-alerts'
    slack_configs:
      - channel: '#alerts'
  - name: 'slack-info'
    slack_configs:
      - channel: '#alerts-info'
```

## 10. 用語集

| 用語 | 説明 |
|---|---|
| Alert | 通知発火条件 |
| Burn Rate | SLO 予算消費速度 |
| Inhibition | アラート抑制 |
| Receiver | 通知送信先 |
| Route | 通知ルーティング |
| Silences | 計画的アラート停止 |
| Runbook | 対応手順書 |
| PagerDuty | 通知 SaaS |
| Severity | 重大度 |

## 11. 参考文献

1. Google SRE Book 第 2 版: Chapter 5-6
2. Prometheus Alerting Best Practices
3. Grafana Alerting Documentation
4. PagerDuty Incident Response
5. Ada プロジェクトチーム「[DOC-ARCH-008 QA 登録簿 §2.4](../architecture/07-qa-register.md)」

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
