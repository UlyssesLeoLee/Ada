# Observability Platform 設計書（可观测性平台设计）

> **本ディレクトリの目的**：[DOC-ARCH-002 技術スタック](../architecture/01-tech-stack.md) と [DOC-ARCH-009 ワークフロー俯瞰](../architecture/08-workflow-overview.md) で定義された Rust 16 crate + PostgreSQL + Redis + K3s 構成に対し、**最小侵入で最大効果** を持つ可観測性体系を設計する。  
> 目標：Observe → Detect → Correlate → Diagnose → Alert → Recover の **完全閉ループ** を実現。

> **ドキュメントID**：DOC-OBS-INDEX
> **文書分類**：アーキテクチャ設計書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：
> - [`docs/architecture/00-anatomy-model.md`](../architecture/00-anatomy-model.md)（DOC-ARCH-001，4 層アーキテクチャ）
> - [`docs/architecture/01-tech-stack.md`](../architecture/01-tech-stack.md)（DOC-ARCH-002，技術スタック）
> - [`docs/architecture/06-rust-tech-selection.md`](../architecture/06-rust-tech-selection.md)（DOC-ARCH-007，crate 選定）
> **下位文書**（13 ドキュメント）：

| # | DOC-ID | ファイル | 内容 |
|---|---|---|---|
| 01 | DOC-OBS-001 | [01-current-state-analysis.md](01-current-state-analysis.md) | 現状調査（Phase 0） |
| 02 | DOC-OBS-002 | [02-architecture.md](02-architecture.md) | 全体アーキテクチャ |
| 03 | DOC-OBS-003 | [03-metrics-design.md](03-metrics-design.md) | 指標体系（RED/USE） |
| 04 | DOC-OBS-004 | [04-logging-design.md](04-logging-design.md) | ログ設計（構造化 + 脱敏） |
| 05 | DOC-OBS-005 | [05-tracing-design.md](05-tracing-design.md) | 分散トレーシング |
| 06 | DOC-OBS-006 | [06-dashboard-catalog.md](06-dashboard-catalog.md) | Grafana Dashboard カタログ |
| 07 | DOC-OBS-007 | [07-alert-policy.md](07-alert-policy.md) | 告警ポリシー（多段評価） |
| 08 | DOC-OBS-008 | [08-slo-design.md](08-slo-design.md) | SLO/SLI 設計 |
| 09 | DOC-OBS-009 | [09-security-design.md](09-security-design.md) | セキュリティ設計 |
| 10 | DOC-OBS-010 | [10-deployment-design.md](10-deployment-design.md) | デプロイ設計 |
| 11 | DOC-OBS-011 | [11-phased-rollout.md](11-phased-rollout.md) | 分段階実施計画 |
| 12 | DOC-OBS-012 | [12-code-impact.md](12-code-impact.md) | コード影響分析 |
| 13 | DOC-OBS-013 | [13-self-audit.md](13-self-audit.md) | アーキ自審 + Revision 2 |

> **関連文書**：
> - 全 [DOC-MOD-NNN](../modules/)（16 モジュール設計）
> - 全 [DOC-API-NNN](../api/)（6 API 仕様）
> - [`docs/templates/05-operations.md`](../templates/05-operations.md)（運用テンプレート，9 種）
> - [`docs/requirements/05-nfr-non-functional-requirements.md`](../requirements/05-nfr-non-functional-requirements.md)（NFR 6 区分）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - IPA「非機能要求グレード2018」【[NF-AVA\|PER\|OPS\|SEC]】
> - JIS X 0160:2012
> - OpenTelemetry 仕様 v1.x
> - Prometheus exposition format 0.0.4
> - Grafana 11.x
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（Phase 0〜13、13 ドキュメント） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 现状サマリ
2. 全体アーキテクチャ（俯瞰）
3. 技術スタック選定
4. 4 シグナル統合の目標
5. トレーサビリティ
6. 使い方
7. 関連 ADR
8. 用語集
9. 参考文献

---

## 1. 現状サマリ

| 項目 | 値 |
|---|---|
| 対象サービス | 18 Rust crate（scaffold 段階 v0.1.0）+ PostgreSQL 16 + Redis 7 + K3s |
| 現状 observability 成熟度 | **L0（未着手）** |
| 目標成熟度 | **L3（フル OpenTelemetry + 自動相関）** |
| 実装方針 | **最小侵入**（既存コードに crate 追加のみ、書き換えなし） |
| 適用範囲 | Infrastructure / Database / Application / Container / K8s の 5 層 |

### 1.1 現状 observability 言及箇所（実在）

| 文書 | 言及 | 関連 |
|---|---|---|
| [DOC-ARCH-006 管理画面](../architecture/05-admin-operations-ui.md) | 8 件 | 監視 UI（§5, §6） |
| [DOC-ARCH-007 Rust 選択](../architecture/06-rust-tech-selection.md) | 19 件 | §10 tracing, §17 CI, §15 SAST |
| [DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md) | 39 件 | 監査/監視 |
| [DOC-ARCH-009 ワークフロー](../architecture/08-workflow-overview.md) | 17 件 | §6.8 運用試験, §9 監査 |
| [M-03 データフロー](../modules/M-03-data-flow-engine.md) | 13 件 | 実行監視、性能計測 |
| [M-16 クラスタ](../modules/M-16-cluster-coordinator.md) | 3 件 | リーダー選出、Shard 監視 |

→ **分散、断片的。統合設計が本書の目的**。

## 2. 全体アーキテクチャ（俯瞰）

```
┌─────────────────────────────────────────────────────────┐
│                    Application Layer                       │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐         │
│  │ M-13    │ │ M-03    │ │ M-10    │ │ M-15    │  ...    │
│  │ Gateway │ │ Engine  │ │ Tenant  │ │ EventBus│         │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘         │
│       │ OpenTelemetry SDK (tracing + metrics + logs)       │
└───────┼──────────────────────────────────────────────────┘
        │ OTLP (gRPC / HTTP)
┌───────▼──────────────────────────────────────────────────┐
│              OpenTelemetry Collector                      │
│  Receivers: OTLP/gRPC, OTLP/HTTP, Prometheus, K8s         │
│  Processors: batch, memory_limiter, tail_sampling        │
│  Exporters: Prometheus, Loki, Tempo, Jaeger              │
└─────┬─────────────┬─────────────┬─────────────┬──────────┘
      │             │             │             │
      ▼             ▼             ▼             ▼
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐
│Prometheus│  │   Loki   │  │  Tempo   │  │  AlertManager│
│  (metrics)│  │  (logs)  │  │ (traces) │  │  (alerts)    │
└─────┬────┘  └─────┬────┘  └─────┬────┘  └──────┬───────┘
      │             │             │             │
      └─────────────┴─────────────┴─────────────┘
                            │
                    ┌───────▼────────┐
                    │    Grafana     │
                    │  (Dashboard)   │
                    └────────────────┘
```

## 3. 技術スタック選定

| 層 | 採用 | 理由 | 代替案 |
|---|---|---|---|
| 計装 SDK | **OpenTelemetry Rust SDK 0.24+** | 業界標準、ベンダ中立、CRDT/WASM 対応 | honeycomb / datadog-trace |
| Metrics | **Prometheus 2.50+** | pull model、K8s 親和性、Operator 成熟 | VictoriaMetrics / Thanos |
| Logs | **Grafana Loki 2.9+** | Prometheus 同様ラベル、grafana 統合 | Elasticsearch / Splunk |
| Traces | **Grafana Tempo 2.4+** | S3 互換 storage、grafana 統合、OTLP native | Jaeger / Zipkin |
| Visualization | **Grafana 11.x** | OSS、Prometheus/Loki/Tempo native 統合 | Kibana / Datadog |
| Alert | **AlertManager + Grafana Unified Alerting** | 重複排除、抑制、SLO 連携 | PagerDuty / OpsGenie |
| Collector | **OpenTelemetry Collector Contrib** | 単一バイナリ、receiver/processor/exporter 拡張容易 | Vector / Fluentd |
| Service Mesh | **K3s + Linkerd（将来）** | mTLS、トレース自動挿入 | Istio（重厚） |

## 4. 4 シグナル統合の目標

| シグナル | 用途 | ツール | 関連 NFR |
|---|---|---|---|
| **Metrics** | システム/アプリ状態の数値 | Prometheus | [NF-AVA\|PER\|OPS] |
| **Logs** | イベント詳細、構造化 | Loki | [NF-OPS] |
| **Traces** | 分散リクエスト追跡 | Tempo | [NF-PER] |
| **Events** | ライフサイクル/状態変化 | Loki（as event） | [NF-OPS] |

**統合キー**：`trace_id` を共通キーとして Metric ↔ Trace ↔ Log を相関。

## 5. トレーサビリティ

```
OBS-REQ-001 (要件)
   ↓
Architecture (DOC-OBS-002)
   ↓
Component (Metrics/Logs/Traces 設計)
   ↓
Configuration (OpenTelemetry Collector, Grafana, Prometheus, etc.)
   ↓
Test Case (G7 ST §4 NFR, G8 UAT)
```

| フェーズ | 担当ドキュメント |
|---|---|
| Phase 0 調査 | [01-current-state-analysis.md](01-current-state-analysis.md) |
| Phase 1 設計 | [02-architecture.md](02-architecture.md) 〜 [09-security-design.md](09-security-design.md) |
| Phase 2 実装 | [10-deployment-design.md](10-deployment-design.md) + [12-code-impact.md](12-code-impact.md) |
| Phase 3 展開 | [11-phased-rollout.md](11-phased-rollout.md) |
| Phase 4 監査 | [13-self-audit.md](13-self-audit.md) |

## 6. 使い方

| シーン | 参照ドキュメント |
|---|---|
| **新サービス追加時** | [12-code-impact.md](12-code-impact.md) → [03-metrics-design.md](03-metrics-design.md) |
| **障害発生時** | [07-alert-policy.md](07-alert-policy.md) + [06-dashboard-catalog.md](06-dashboard-catalog.md) |
| **SLO 設計** | [08-slo-design.md](08-slo-design.md) |
| **容量計画** | [06-dashboard-catalog.md §70 Performance](06-dashboard-catalog.md) + [08-slo-design.md](08-slo-design.md) |
| **新ダッシュボード** | [06-dashboard-catalog.md](06-dashboard-catalog.md) |

## 7. 関連 ADR

| ADR | 主题 | 状態 | 関連文書 |
|---|---|---|---|
| OBS-ADR-01 | OpenTelemetry 採用（vs Honeycomb/Datadog） | ✅ 推奨 | [02-architecture.md §3](02-architecture.md) |
| OBS-ADR-02 | Prometheus + Loki + Tempo（vs Elastic + Jaeger） | ✅ 推奨 | [02-architecture.md §3](02-architecture.md) |
| OBS-ADR-03 | Grafana 11（vs Kibana） | ✅ 推奨 | [02-architecture.md §3](02-architecture.md) |
| OBS-ADR-04 | OpenTelemetry Collector（vs Vector/Fluentd） | ✅ 推奨 | [10-deployment-design.md](10-deployment-design.md) |
| OBS-ADR-05 | ログを OTLP で Loki 直送（vs ファイル → Fluentd → Loki） | ✅ 推奨 | [04-logging-design.md §5](04-logging-design.md) |
| OBS-ADR-06 | Trace sampling: head 10% + tail 100%（エラー時） | ✅ 推奨 | [05-tracing-design.md §4](05-tracing-design.md) |
| OBS-ADR-07 | テナント別 metrics 隔離（同一 Prometheus に label 隔離） | ✅ 推奨 | [09-security-design.md §3](09-security-design.md) |
| OBS-ADR-08 | 業務コードは OpenTelemetry のみ依存（直接 Loki/Prometheus 禁止） | ✅ 推奨 | [02-architecture.md §4](02-architecture.md) |
| OBS-ADR-09 | K3s + Linkerd（mTLS + 自動 trace）（将来） | 🟡 保留 | [10-deployment-design.md §6](10-deployment-design.md) |
| OBS-ADR-10 | Backup: observability データも 4 段 Backup 対象 | ✅ 推奨 | [09-security-design.md §6](09-security-design.md) |

## 8. 用語集

| 用語 | 説明 |
|---|---|
| Observability | システムの内部状態を外部から観測する能力 |
| Signal | Metrics / Logs / Traces / Events の 4 種 |
| OpenTelemetry (OTel) | CNCF の計装標準、ベンダ中立 |
| OTLP | OpenTelemetry Protocol（gRPC / HTTP） |
| RED | Rate / Errors / Duration のアプリ指標フレーム |
| USE | Utilization / Saturation / Errors のリソース指標フレーム |
| SLI / SLO | Service Level Indicator / Objective |
| Cardinality | 指標のラベルの組み合わせ数（高 cardinality = 性能問題） |
| Trace Context | W3C 標準の分散トレーシング用 HTTP ヘッダ |
| Span | 単一処理単位のトレースデータ |
| Tenant | マルチテナント環境での顧客単位 |
| PII | Personally Identifiable Information（GDPR/PIPL 保護対象） |

## 9. 参考文献

1. OpenTelemetry Documentation, https://opentelemetry.io/docs/
2. Prometheus Best Practices, https://prometheus.io/docs/practices/
3. Grafana Loki Documentation, https://grafana.com/docs/loki/
4. Grafana Tempo Documentation, https://grafana.com/docs/tempo/
5. Google SRE Book 第 2 版, Google, 2020
6. IPA「共通フレーム2018 (SLCP-JCF2018)」, 2018
7. IPA「非機能要求グレード2018」, 2018
8. W3C Trace Context, https://www.w3.org/TR/trace-context/
9. Ada プロジェクトチーム「[DOC-ARCH-001 仿生モデル](../architecture/00-anatomy-model.md)」, 2026-08-19
10. Ada プロジェクトチーム「[DOC-ARCH-002 技術スタック](../architecture/01-tech-stack.md)」, 2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
