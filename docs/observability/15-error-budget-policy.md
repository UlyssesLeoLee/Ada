# 15 Error Budget Policy (v0.7.0 新規)

> **Error Budget 残量 × Burn Rate しきい値 × 行動マトリクス** を 1 ページに集約。  
> 実装者・SRE・PM が同じ表を見て意思決定するための **Single Source of Truth**。

> **ドキュメントID**：DOC-OBS-015
> **上位文書**：[DOC-OBS-INDEX](README.md) / [08-slo-design.md §11](08-slo-design.md) / [07-alert-policy.md](07-alert-policy.md) / [11-phased-rollout.md §11](11-phased-rollout.md)
> **下位文書**：[14-auto-remediation.md](14-auto-remediation.md) / `config/alertmanager/*.yaml`

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v0.7.0 | 2026-08-27 | 初版 (Phase 8.5 SRE ハードニング タスク 6: Burn Rate ルール + 行動マトリクス + 4 アラート config) |

---

## 1. 目的

SLO 達成率 (= Error Budget 残量) に応じて **「今何ができるか / 何を止めるべきか」** を機械的に決定する。  
曖昧な判断 (PM 会議にかけないと…) を排し、Burn Rate シグナルがそのまま行動を駆動する。

## 2. Error Budget 残量 × 行動マトリクス

| 残予算 | 状態 | リリース | 機能開発 | 信頼性タスク | RCA | エスカレーション |
|---|---|---|---|---|---|---|
| **> 75%** | 🟢 健全 | ✅ 通常 | ✅ 通常 | 通常比率 | 任意 | 不要 |
| **50-75%** | 🟡 注意 | ✅ 通常 | ✅ 通常 | **+20% シフト** | 必要 | 月次レビュー |
| **20-50%** | 🟠 警戒 | ⚠️ PO 承認 | ⚠️ 凍結検討 | **信頼性 80%** | 必須 | 週次 Sync |
| **5-20%** | 🔴 危険 | ❌ 停止 | ❌ 停止 | **信頼性 100%** | 必須 | 日次 Standup |
| **< 5%** | ⚫ 超過 | ❌ 停止 | ❌ 停止 | **信頼性 100%** | 必須 + 顧客報告 | 即時 Sev1 |

> **「信頼性シフト」の定義**: 該当 SLO 担当の SRE Lead が翌週の工数を信頼性タスクに振り向ける。  
> 「リリース停止」は PM が起票する新 MR のマージをブロック (CI ラベル `release-freeze` を付与)。

## 3. Burn Rate しきい値 (Auto-remediation 専用)

[08-slo-design.md §11.3](08-slo-design.md) の表に対応する **完全 PromQL 式**。

### 3.1 SLO-REM-FAST-BURN-1h (SLO-004 / SLO-005 共通)

```yaml
- alert: SLO_Remediation_Availability_FastBurn_1h
  expr: |
    (
      sum(rate(ada_remediation_actions_total{outcome="failure"}[1h]))
      /
      sum(rate(ada_remediation_actions_total[1h]))
    ) > (14.4 * 0.005)
    and
    (
      sum(rate(ada_remediation_actions_total{outcome="failure"}[5h]))
      /
      sum(rate(ada_remediation_actions_total[5h]))
    ) > (14.4 * 0.005)
  for: 2m
  labels:
    severity: sev2
    slo: slo-004-remediation-webhook
  annotations:
    summary: "Auto-remediation SLO-004 高速消費 (1h 窓で 14.4x Burn)"
    runbook: "https://wiki/runbooks/slo-rem-fast-burn-1h"
    action: "PagerDuty 即時通知 / [Error Budget Policy §2] 残予算確認"
```

**しきい値の意味** (SLO-004 = 99.5% の場合):  
`(14.4 * 0.005) = 0.072` = **7.2% のエラー率** が 1h 窓で継続したら発火。  
これが継続すると 28 日バジェットを **2 日で使い切る** ペース。

### 3.2 SLO-REM-FAST-BURN-6h

```yaml
- alert: SLO_Remediation_Availability_FastBurn_6h
  expr: |
    (
      sum(rate(ada_remediation_actions_total{outcome="failure"}[6h]))
      /
      sum(rate(ada_remediation_actions_total[6h]))
    ) > (6 * 0.005)
    and
    (
      sum(rate(ada_remediation_actions_total{outcome="failure"}[30h]))
      /
      sum(rate(ada_remediation_actions_total[30h]))
    ) > (6 * 0.005)
  for: 5m
  labels:
    severity: sev2
    slo: slo-004-remediation-webhook
  annotations:
    summary: "Auto-remediation SLO-004 中速消費 (6h 窓で 6x Burn)"
    runbook: "https://wiki/runbooks/slo-rem-fast-burn-6h"
```

### 3.3 SLO-REM-SLOW-BURN-24h

```yaml
- alert: SLO_Remediation_Availability_SlowBurn_24h
  expr: |
    (
      sum(rate(ada_remediation_actions_total{outcome="failure"}[24h]))
      /
      sum(rate(ada_remediation_actions_total[24h]))
    ) > (3 * 0.005)
    and
    (
      sum(rate(ada_remediation_actions_total{outcome="failure"}[120h]))
      /
      sum(rate(ada_remediation_actions_total[120h]))
    ) > (3 * 0.005)
  for: 30m
  labels:
    severity: sev3
    slo: slo-004-remediation-webhook
  annotations:
    summary: "Auto-remediation SLO-004 緩速消費 (24h 窓で 3x Burn)"
    runbook: "https://wiki/runbooks/slo-rem-slow-burn-24h"
    action: "Slack #ada-ops 通知 / [Error Budget Policy §2] 残予算確認"
```

### 3.4 SLO-REM-SLOW-BURN-72h

```yaml
- alert: SLO_Remediation_Availability_SlowBurn_72h
  expr: |
    (
      sum(rate(ada_remediation_actions_total{outcome="failure"}[72h]))
      /
      sum(rate(ada_remediation_actions_total[72h]))
    ) > (1 * 0.005)
    and
    (
      sum(rate(ada_remediation_actions_total{outcome="failure"}[360h]))
      /
      sum(rate(ada_remediation_actions_total[360h]))
    ) > (1 * 0.005)
  for: 1h
  labels:
    severity: sev4
    slo: slo-004-remediation-webhook
  annotations:
    summary: "Auto-remediation SLO-004 ベースライン超過 (72h 窓で 1x Burn)"
    runbook: "https://wiki/runbooks/slo-rem-slow-burn-72h"
    action: "翌営業日 SRE レビューで SLO 妥当性再評価"
```

### 3.5 SLO-006 (Latency p95) 派生

```yaml
- alert: SLO_Remediation_Latency_p95
  expr: |
    histogram_quantile(0.95,
      sum by (le) (rate(ada_remediation_action_duration_seconds_bucket[10m]))
    ) > 30
  for: 10m
  labels:
    severity: sev3
    slo: slo-006-remediation-latency
  annotations:
    summary: "Auto-remediation SLO-006 違反 (p95 latency > 30s, 10m 窓)"
    runbook: "https://wiki/runbooks/slo-rem-latency-p95"
```

## 4. 行動プロトコル (Burn Rate アラート受信時)

### 4.1 Sev2 (FAST-BURN) — 即時対応 (15 分以内)

```
1. アラート受信 → PagerDuty がオナー (SRE Lead) をページ
2. オナーは 5 分以内に [runbook URL] を確認
3. 影響範囲確認:
   - どの alert_name が失敗しているか
   - 特定コンポーネントか / 全般か
   - 1 リージョンか / マルチリージョンか
4. 緊急度判定:
   - 1 リージョン単独 → 当該リージョンで一時停止 (cooldown 拡張)
   - マルチリージョン → Phase 8.5 §11.4 緊急停止プロトコル発動
5. 暫定対応 (15 分以内):
   - 全 runbook の `executor.mode` を `dry-run` に切替
   - Alertmanager webhook 経路を 503 で fail-closed
6. RCA 開始 (24 時間以内に暫定レポート)
```

### 4.2 Sev3 (SLOW-BURN-24h) — 24 時間以内

```
1. Slack #ada-ops に通知
2. 当日 SRE デイリー Sync で議題化
3. 過去 24h の `ada_remediation_actions_total{outcome="failure"}` を label 分解
4. 失敗 runbook 単位の RCA 開始
5. [Error Budget Policy §2] で残予算を再計算し、PM に共有
```

### 4.3 Sev4 (SLOW-BURN-72h) — 翌営業日レビュー

```
1. アラートは Slack #ada-sre-reports に投稿のみ (ページなし)
2. 翌営業日 SRE 週次レビューで:
   - 過去 72h の傾向を 1 枚スライドにまとめる
   - SLO 目標値の妥当性を再評価
   - 必要なら [08-slo-design.md §8.2 改訂プロセス] に則り SLO 改訂提案
```

## 5. クロスリージョン挙動

Auto-remediation は **リージョン独立** で動作する (各リージョンが自分の Alertmanager を見る)。

| シナリオ | 挙動 | アクション |
|---|---|---|
| **片方のリージョンのみ Burn** | もう片方は正常 | 当該リージョンのみ緊急停止 (Phase 8.5 §11.4 手順 4-ii) |
| **両リージョン同時に Burn** | 共通原因 (DB 全体障害 / 認証基盤障害) を疑う | Phase 8.5 §11.4 手順 4-iii 全体停止 |
| **片方のリージョンでテスト中** | テスト起因の Burn は SLO 計算から除外 | 一時的に `replica=test` ラベル付与、SLO 計算式から除外 |

### 5.1 クロスリージョン Burn Rate 合算

```promql
# グローバル Burn Rate (両リージョン合算)
sum by (slo) (
  rate(ada_remediation_actions_total{outcome="failure"}[1h])
) / sum by (slo) (
  rate(ada_remediation_actions_total[1h])
)
```

リージョン別の Burn が発生したら、**片方だけの対処では SLO は回復しない**。  
必ず両リージョンのメトリクスを並べて確認する。

## 6. 関連 Alert Rules ファイル一覧

| ファイル | 内容 | 配置先 |
|---|---|---|
| `config/alertmanager/slo-rem-fast-burn-1h.yaml` | SLO-REM-FAST-BURN-1h アラート | k8s ConfigMap |
| `config/alertmanager/slo-rem-fast-burn-6h.yaml` | SLO-REM-FAST-BURN-6h アラート | k8s ConfigMap |
| `config/alertmanager/slo-rem-slow-burn-24h.yaml` | SLO-REM-SLOW-BURN-24h アラート | k8s ConfigMap |
| `config/alertmanager/slo-rem-slow-burn-72h.yaml` | SLO-REM-SLOW-BURN-72h アラート | k8s ConfigMap |
| `config/alertmanager/slo-rem-latency-p95.yaml` | SLO-006 派生レイテンシアラート | k8s ConfigMap |

> 各 yaml ファイルは PrometheusRule (CRD) 形式。Helm chart で `kube-prometheus-stack` に同梱してデプロイ。

## 7. 用語集

| 用語 | 説明 |
|---|---|
| **Burn Rate (BR)** | Error Budget の消費速度 (倍率) |
| **Fast Burn** | 短時間 (1-6h) で 14.4x-6x 速度の消費 = Sev2 |
| **Slow Burn** | 長時間 (24-72h) で 1x-3x 速度の消費 = Sev3-4 |
| **Multi-Window** | 短窓 × 長窓の AND 条件で発火する Burn Rate ルール |
| **Fail-Closed** | 認証/外部依存が失敗したときに拒否する設計 (Auto-remediation のデフォルト) |
| **Cross-Region** | マルチリージョン環境で、リージョン横断で挙動を一致させる運用 |

## 8. 改訂履歴

| 項目 | 内容 |
|---|---|
| 追加 v0.7.0 | Phase 8.5 SRE ハードニング タスク 6 として初版。SLO-004~006 に対応する Burn Rate ルール、4 段階の重大度別行動プロトコル、クロスリージョン挙動を 1 ページに集約。 |
| 関連 | [08-slo-design.md §11](08-slo-design.md), [11-phased-rollout.md §11](11-phased-rollout.md), [14-auto-remediation.md](14-auto-remediation.md) |

---

> **末尾注記**  
> 本ドキュメントは [08-slo-design.md §11](08-slo-design.md) の SLO 体系と [07-alert-policy.md](07-alert-policy.md) のアラート分類に対応する **行動契約** である。  
> Burn Rate しきい値・残予算しきい値・行動プロトコルは **PM / SRE Lead / PO** の三者の合意を前提とし、四半期ごとに見直す (改訂プロセスは [08-slo-design.md §8](08-slo-design.md) 参照)。
