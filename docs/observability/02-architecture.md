# 02 全体アーキテクチャ（Overall Architecture）

> OpenTelemetry + Grafana スタックによる 4 シグナル統合。**業務コードは OTel のみ依存**、バックエンドは隠蔽。

> **ドキュメントID**：DOC-OBS-002
> **上位文書**：[DOC-OBS-INDEX](README.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版 |

---

## 目次

1. アーキテクチャ全体像
2. レイヤー定義
3. 技術スタック
4. データフロー
5. 業務コード規約
6. 設計原則
7. 用語集

---

## 1. アーキテクチャ全体像

```
┌──────────────────────────────────────────────────────────────────────┐
│                        業務サービス層（18 Rust crate）                 │
│                                                                        │
│   M-13 API GW    M-03 Engine    M-10 Tenant    M-15 EventBus           │
│       │              │              │              │                  │
│       └──────────────┴──────────────┴──────────────┘                  │
│                              │                                       │
│                   ┌──────────▼──────────┐                            │
│                   │ OpenTelemetry SDK  │ ← 唯一の計装窓口             │
│                   │ (tracing + metrics │                            │
│                   │  + logs)           │                            │
│                   └──────────┬──────────┘                            │
│                              │ OTLP (gRPC :4317)                     │
│                              │                                       │
│                   ┌──────────▼──────────┐                            │
│                   │  ada-telemetry     │ ← crate 単位 collector       │
│                   │  (sender)          │   または直接 OTLP 送信       │
│                   └──────────┬──────────┘                            │
└──────────────────────────────┼──────────────────────────────────────┘
                               │ mTLS (Istio/Linkerd 将来)
┌──────────────────────────────▼──────────────────────────────────────┐
│              observability namespace (K3s)                           │
│                                                                        │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │              OpenTelemetry Collector (Deployment)              │ │
│  │  Receivers: OTLP/gRPC, OTLP/HTTP, prometheus, k8scluster       │ │
│  │  Processors: batch, memory_limiter, tail_sampling,             │ │
│  │              resource, attributes/tenant_id, transform         │ │
│  │  Exporters: prometheusremotewrite, loki, otlp/tempo            │ │
│  │  Extensions: health_check, pprof, zpages                        │ │
│  └─────────┬───────────────┬───────────────┬─────────────────────┘ │
│            │               │               │                       │
│            ▼               ▼               ▼                       │
│   ┌────────────┐   ┌────────────┐   ┌────────────┐                 │
│   │Prometheus  │   │   Loki     │   │   Tempo    │                 │
│   │(15d retain)│   │(30d retain)│   │(7d retain) │                 │
│   └─────┬──────┘   └──────┬─────┘   └──────┬─────┘                 │
│         │                │                │                         │
│         └────────────────┼────────────────┘                         │
│                          │                                          │
│                  ┌───────▼────────┐                                 │
│                  │    Grafana     │ ← Unified UI                    │
│                  │  (Dashboard +  │                                 │
│                  │   Alerting)    │                                 │
│                  └───────┬────────┘                                 │
│                          │                                          │
│                  ┌───────▼────────┐                                 │
│                  │ AlertManager   │                                 │
│                  │ + PagerDuty    │                                 │
│                  └────────────────┘                                 │
└──────────────────────────────────────────────────────────────────────┘
```

## 2. レイヤー定義

### 2.1 計装レイヤー（業務コード）

| 役割 | 技術 | ライブラリ |
|---|---|---|
| Span / Context | tracing 0.1+ | `tracing` + `tracing-subscriber` |
| Metrics | `opentelemetry-prometheus` または `metrics` 0.23+ | 直接 counter/gauge/histogram |
| Logs | `tracing-subscriber::fmt` (JSON layer) | bunyan 風 JSON |
| OTLP 送信 | `opentelemetry-otlp` 0.17+ | tonic gRPC client |
| Resource 検出 | `opentelemetry-resource-detectors` | k8s, env, process |

### 2.2 収集レイヤー（otel-collector）

| 役割 | コンポーネント |
|---|---|
| 受信 | `otlp`, `prometheus`, `k8scluster`, `k8sobjects`, `filelog`, `journald` |
| 加工 | `batch`, `memory_limiter`, `tail_sampling`, `resource`, `transform`, `filter`, `attributes`, `tenant_enrichment` |
| 転送 | `prometheusremotewrite`, `loki`, `otlp/tempo` |

### 2.3 バックエンドレイヤー

| バックエンド | 役割 | 構成 |
|---|---|---|
| Prometheus | Metrics ストレージ | StatefulSet 1 replica + 15d 保持 |
| Loki | Logs ストレージ | 単一 Pod + 30d 保持（gossip 無効、単一ノード想定） |
| Tempo | Traces ストレージ | 単一 Pod + 7d 保持（外部ストレージ: MinIO / S3） |
| Grafana | UI / Alerting | 2 replica + セッション永続化 |

### 2.4 アラート / 通知レイヤー

| ツール | 役割 |
|---|---|
| Grafana Unified Alerting | アラート定義、評価 |
| AlertManager | 重複排除、エスカレーション |
| PagerDuty / Slack / Email | 通知先 |

## 3. 技術スタック

| カテゴリ | 採用 | バージョン | 備考 |
|---|---|---|---|
| 計装 SDK | OpenTelemetry Rust | 0.24+ | CNCF Graduated |
| App 計装 | tracing + tracing-subscriber | 0.1+ / 0.3+ | [DOC-ARCH-007 §10](../architecture/06-rust-tech-selection.md) |
| Metrics | Prometheus | 2.50+ | Pull model、K8s native |
| Logs | Grafana Loki | 2.9+ | Promtail 経由 or OTLP 直送 |
| Traces | Grafana Tempo | 2.4+ | OTLP native |
| Visualize | Grafana | 11.x | Unified Alerting 内蔵 |
| Alert routing | AlertManager | 0.27+ | Grafana 経由でも可 |
| Collector | otel-collector-contrib | 0.104+ | 単一 Deployment |
| K8s 連携 | kube-state-metrics | 2.13+ | K8s メトリクス |
| K8s logs | promtail | 2.9+ | DaemonSet |
| ノード | node-exporter | 1.8+ | DaemonSet |
| DB | postgres_exporter | 0.15+ | postgres_exporter 0.15.0 |
| Redis | redis_exporter | 1.62+ | 計画 |

## 4. データフロー

### 4.1 計装データ → バックエンド

```
[Service: ada-m13-gateway]
    │ (1) span + metric + log 生成
    ▼
[otel SDK (in-process)]
    │ (2) batching (max 512 / 5s)
    │ (3) resource attributes 付与（service.name, version, env, tenant_id, k8s.*）
    │ (4) OTLP encode
    ▼
[ada-telemetry crate: OTLP gRPC client (tonic)]
    │ (5) mTLS (本番環境) / plaintext (dev)
    ▼
[otel-collector:4317 (otlp receiver)]
    │ (6) batch / memory_limiter
    │ (7) tail_sampling（trace 100% キープ、エラー時 100%）
    │ (8) attributes (PII 脱敏)
    ▼
[Exporter 分岐]
    ├──> [Prometheus remote_write] → Prometheus
    ├──> [Loki] (loki exporter)
    └──> [Tempo] (otlp/tempo)
```

### 4.2 4 シグナルの関連キー

| シグナル | 共通キー | 例 |
|---|---|---|
| Span | `trace_id` | `4bf92f3577b34da6a3ce929d0e0e4736` |
| Span 内 log | `trace_id` + `span_id` | span ID `00f067aa0ba902b7` |
| Metric | `trace_id`（任意、exemplar 経由） | histogram に exemplar 付与 |
| Log | `trace_id` + `span_id` + `request_id` | request_id `a1b2c3d4` |
| Event | `trace_id`（任意） | atomic swap イベント等 |

**故障相関フロー**：

```
[1] Latency metric 増加 (Prometheus alert)
       ↓ trace_id exemplar から該当 trace 取得
[2] Tempo で trace 確認 → 遅い span 特定
       ↓ span_id から log 検索
[3] Loki で該当 span の error log 確認
       ↓ tenant_id, request_id でフィルタ
[4] Root Cause 特定 (例: 特定 tenant の DB slow query)
```

## 5. 業務コード規約

### 5.1 禁止事項

❌ 業務コードから直接依存：

```rust
// ❌ 禁止: 直接 Prom exporter
use prometheus::*;
// ❌ 禁止: 直接 Loki API
use loki_client::*;
// ❌ 禁止: 独自 HTTP exporter
use reqwest::Client; // 計装目的では使用禁止
```

### 5.2 許可パターン

✅ OpenTelemetry SDK のみ使用：

```rust
// ✅ 許可: tracing span
use tracing::{info, instrument, Span};
#[instrument(skip_all, fields(tenant_id, request_id))]
async fn handle_request(req: Request) -> Result<Response, Error> {
    let _enter = Span::current().entered();
    info!("processing request");
    // ... 業務ロジック
    Ok(response)
}

// ✅ 許可: opentelemetry metrics
use opentelemetry::metrics::{Counter, Histogram};
static REQUEST_COUNTER: Lazy<Counter<u64>> = Lazy::new(|| {
    meter().u64_counter("ada.requests.total").build()
});
REQUEST_COUNTER.add(1, &[KeyValue::new("endpoint", "/api/v1/canvases")]);
```

### 5.3 業務側で決める必要がないこと

| 項目 | 誰が管理 |
|---|---|
| メトリクス保存期間 | SRE / OTel 設定 |
| ログ保存期間 | SRE / Loki 設定 |
| アラート閾値 | SRE / Alertmanager 設定 |
| ダッシュボード | SRE / Grafana 管理 |
| バックエンドエンドポイント | 環境変数 `OTEL_EXPORTER_OTLP_ENDPOINT` |

## 6. 設計原則

| 原則 | 説明 |
|---|---|
| **1 つの計装窓口** | OTel SDK 以外触らない |
| **ベンダ中立** | OTel → 任意のバックエンドへ接続可能（移行容易） |
| **業務影響最小** | 計装は < 5% 性能影響（[NFR-PER](../requirements/05-nfr-non-functional-requirements.md) 遵守） |
| **テナント隔離** | 全データに `tenant_id` ラベル |
| **PII ゼロトレランス** | 自動脱敏、CI チェック |
| **段階導入** | Phase 0 → 8 で段階展開 |

## 7. 関連 ADR

- [OBS-ADR-01](#) OpenTelemetry 採用
- [OBS-ADR-02](#) Prometheus + Loki + Tempo
- [OBS-ADR-08](#) 業務コードは OTel のみ依存

## 8. 用語集

| 用語 | 説明 |
|---|---|
| OTel / OpenTelemetry | CNCF 計装標準 |
| OTLP | OpenTelemetry Protocol |
| Signal | Metrics / Logs / Traces / Events |
| Exemplar | メトリクスに trace_id を紐付ける仕組み |
| Resource | サービス、Pod、テナントなどの属性 |
| Span | 単一処理単位のトレースデータ |
| Tenant | マルチテナント環境での顧客単位 |
| mTLS | 相互 TLS 認証 |
| Pull / Push model | Prometheus は pull、OTel は push |

## 9. 参考文献

1. OpenTelemetry Architecture, https://opentelemetry.io/docs/architecture/
2. CNCF Observability Whitepaper
3. Google SRE Book 第 2 版
4. Ada プロジェクトチーム「[DOC-ARCH-001 仿生モデル](../architecture/00-anatomy-model.md)」
5. Ada プロジェクトチーム「[DOC-ARCH-007 Rust 選択](../architecture/06-rust-tech-selection.md)」

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
