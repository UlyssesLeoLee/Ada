# 12 コード影響分析（Code Impact Analysis）

> **観測基盤導入が業務コードに与える影響を定量化**。  
> 各 crate / module に対する変更内容・影響度・修正工数・リスク・代替案を明確化し、  
> 「Low / Medium / High」の 3 段階で評価する。

> **ドキュメントID**：DOC-OBS-012
> **上位文書**：[DOC-OBS-INDEX](README.md)
> **下位文書**：[DOC-OBS-011 Phased Rollout](11-phased-rollout.md) / [DOC-ARCH-006 Rust](D:/Ada/docs/architecture/06-rust-tech-selection.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版（18 crate × 4 評価軸） |

---

## 目次

1. 評価基準
2. 影響度サマリー
3. 新規追加コンポーネント
4. crate 別影響評価（18 crate）
5. 変更工数合計
6. 互換性 / 移行戦略
7. テスト影響
8. CI / CD 影響
9. リスクと対策
10. 用語集
11. 参考文献

---

## 1. 評価基準

### 1.1 影響度 3 段階

| 影響度 | 基準 |
|---|---|
| **Low** | 1-2 ファイル変更 / 0-1 日 / 既存テスト変更なし / 機能影響なし |
| **Medium** | 3-10 ファイル変更 / 1-3 日 / 既存テスト一部更新 / 機能影響あり（feature flag 制御） |
| **High** | 10+ ファイル変更 / 3+ 日 / 既存テスト全面更新 / 機能影響あり（コード再設計） |

### 1.2 評価軸

| 軸 | 説明 |
|---|---|
| **ファイル変更数** | 触る必要のある .rs ファイル数 |
| **工数** | 実装 + テスト + レビュー工数（人日） |
| **テスト影響** | 既存テストの更新 / 追加が必要な数 |
| **機能影響** | 観測基盤 OFF 時に機能影響が出るか |

## 2. 影響度サマリー

| crate | 変更ファイル | 工数 | テスト影響 | 機能影響 | **影響度** |
|---|---|---|---|---|---|
| **ada-core** | 3 | 1.5 | 2 | なし | **Low** |
| **ada-telemetry** | 25 | 8 | 5 | 観測機能のみ | **High**（新規 crate） |
| **ada-m01-acquisition** | 4 | 2 | 3 | なし | **Low** |
| **ada-m02-normalizer** | 3 | 1.5 | 2 | なし | **Low** |
| **ada-m03-data-flow-engine** | 8 | 4 | 6 | 性能影響 +2% | **Medium** |
| **ada-m04-orchestration** | 6 | 3 | 4 | なし | **Medium** |
| **ada-m05-control-flow** | 4 | 2 | 3 | なし | **Low** |
| **ada-m06-plugin-sdk** | 3 | 1.5 | 2 | なし | **Low** |
| **ada-m07-debug** | 2 | 1 | 1 | なし | **Low** |
| **ada-m08-trigger** | 3 | 1.5 | 2 | なし | **Low** |
| **ada-m09-exporter** | 4 | 2 | 3 | なし | **Low** |
| **ada-m10-tenant-middleware** | 7 | 3.5 | 5 | 性能影響 +1% | **Medium** |
| **ada-m11-rbac-collab** | 5 | 2.5 | 4 | なし | **Medium** |
| **ada-m12-canvas-editor** | 6 | 3 | 5 | 性能影響 +3% | **Medium** |
| **ada-m13-api-gateway** | 8 | 4 | 6 | 性能影響 +2% | **Medium** |
| **ada-m14-module-registry** | 4 | 2 | 3 | なし | **Low** |
| **ada-m15-central-event-bus** | 7 | 3.5 | 5 | 性能影響 +1% | **Medium** |
| **ada-m16-cluster-coordinator** | 5 | 2.5 | 4 | なし | **Medium** |
| **合計** | **107** | **49 人日** | **63** | - | - |

> **合計工数**：49 人日 ≒ **10 週（1 名 50%） / 5 週（1 名フル） / 2.5 週（2 名フル）**

## 3. 新規追加コンポーネント

### 3.1 ada-telemetry crate（新規）

**役割**：全 crate が利用する観測 SDK。

| ファイル | 内容 | 影響度 |
|---|---|---|
| `src/lib.rs` | 公開 API、feature flag | High |
| `src/metrics.rs` | Prometheus exporter 実装 | High |
| `src/logging.rs` | tracing 統合、PII redaction | High |
| `src/tracing.rs` | OTel Tracer 設定 | High |
| `src/labels.rs` | 共通ラベル定義 | Medium |
| `src/pii.rs` | 自動 redaction パターン | High |
| `src/config.rs` | 設定（OTel endpoint 等） | Medium |
| `src/error.rs` | エラー型 | Low |
| `tests/unit_*.rs` (5) | 単体テスト | Medium |
| `tests/integration_*.rs` (3) | 統合テスト | Medium |
| `Cargo.toml` | 依存関係 | Low |

**依存クレート追加**：
```toml
[dependencies]
opentelemetry = { version = "0.24", features = ["trace", "metrics"] }
opentelemetry-otlp = { version = "0.17", features = ["grpc-tonic", "metrics"] }
opentelemetry_sdk = { version = "0.24", features = ["rt-tokio"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-opentelemetry = "0.25"
prometheus = "0.13"
regex = "1.10"
once_cell = "1.19"
```

### 3.2 Helm Chart（新規）

- `charts/observability-platform/` (新規ディレクトリ)

### 3.3 Kubernetes Manifest

- `k8s/observability/namespace.yaml`
- `k8s/observability/networkpolicies.yaml`
- `k8s/observability/resourcequota.yaml`

### 3.4 Dashboard / Alert YAML

- `dashboards/*.json` (10 個)
- `alerts/*.yaml` (30+ ルール)
- `alerts/burn-rate.yaml` (4 ルール)

### 3.5 ドキュメント

- `docs/observability/` (13 ファイル、本プロジェクト)
- `docs/runbooks/observability/` (各アラート用)

## 4. crate 別影響評価

### 4.1 ada-core（影響度：Low）

**変更内容**：
- 共通ラベル定義の追加
- 共通エラー型の拡張
- feature flag `telemetry` の追加

**主要変更ファイル**：
1. `src/lib.rs` - feature flag 公開
2. `src/error.rs` - 観測用エラー型
3. `src/config.rs` - 観測設定

**テスト影響**：
- 既存 3 UT は変更なし
- 追加 UT 2 件（feature flag on/off）

### 4.2 ada-m13-api-gateway（影響度：Medium）

**変更内容**：
- axum middleware でトレース自動計装
- HTTP リクエストの RED メトリクス
- 認証 span の追加
- レスポンス時のトレースコンテキスト伝播

**主要変更ファイル**：
1. `src/main.rs` - OTel 初期化
2. `src/middleware/trace.rs` (新規) - トレース middleware
3. `src/middleware/metrics.rs` (新規) - メトリクス middleware
4. `src/handlers/auth.rs` - 認証 span 追加
5. `src/handlers/canvas.rs` - キャンバス span
6. `src/handlers/plugin.rs` - プラグイン span
7. `src/handlers/export.rs` - エクスポート span
8. `src/observability.rs` (新規) - 観測ラッパー

**テスト影響**：
- 既存 IT 6 件更新（トレース ID 検証追加）
- 追加 IT 2 件（PII redaction 検証）

**性能影響**：
- p99 +2% 想定（バッチ export + sampling 対策済み）

### 4.3 ada-m03-data-flow-engine（影響度：Medium）

**変更内容**：
- データフロー各ステージの Span 計装
- スループット / バックプレッシャーメトリクス
- 内部キューの長さ / 滞留時間計測

**主要変更ファイル**：
1. `src/main.rs` - OTel 初期化
2. `src/engine/pipeline.rs` - パイプライン Span
3. `src/engine/transform.rs` - 変換 Span
4. `src/engine/aggregate.rs` - 集約 Span
5. `src/queue/mod.rs` - キュー長計測
6. `src/metrics.rs` (新規) - エンジンメトリクス
7. `src/observability.rs` (新規)
8. `src/error.rs` - 観測エラー

**テスト影響**：
- 既存 IT 6 件更新
- 追加 IT 3 件（パイプライントレース検証）

### 4.4 ada-m10-tenant-middleware（影響度：Medium）

**変更内容**：
- DB クエリの Span 計装（sqlx instrumentation）
- テナント ID ラベル付与
- キャッシュヒット率メトリクス
- 接続プールメトリクス

**主要変更ファイル**：
1. `src/main.rs` - OTel 初期化
2. `src/db/mod.rs` - DB 計装
3. `src/db/tenant.rs` - テナントクエリ Span
4. `src/cache/mod.rs` - キャッシュメトリクス
5. `src/middleware/auth.rs` - 認証 Span
6. `src/observability.rs` (新規)
7. `src/error.rs`

**テスト影響**：
- 既存 IT 5 件更新
- 追加 IT 2 件（テナント分離メトリクス検証）

### 4.5 ada-m12-canvas-editor（影響度：Medium）

**変更内容**：
- Yrs CRDT 操作の Span 計装
- 共同編集のレイテンシ計測
- WebSocket フレームのトレース
- 同期遅延のメトリクス

**主要変更ファイル**：
1. `src/main.rs` - OTel 初期化
2. `src/crdt/yrs_adapter.rs` - CRDT Span
3. `src/sync/mod.rs` - 同期 Span
4. `src/transport/websocket.rs` - WS 計装
5. `src/observability.rs` (新規)
6. `src/error.rs`

**性能影響**：
- p99 +3% 想定（高頻度 Span が必要なため）
- 対策：head sampling 10% + tail 100% on errors

**テスト影響**：
- 既存 IT 5 件更新
- 追加 IT 3 件（CRDT トレース検証）

### 4.6 ada-m15-central-event-bus（影響度：Medium）

**変更内容**：
- Producer/Consumer の Span 計装
- Kafka / NATS 計装（rdkafka instrumentation）
- Consumer lag メトリクス
- スループット / エラー率

**主要変更ファイル**：
1. `src/main.rs` - OTel 初期化
2. `src/producer/mod.rs` - Producer Span
3. `src/consumer/mod.rs` - Consumer Span
4. `src/bus/kafka.rs` - Kafka 計装
5. `src/bus/nats.rs` - NATS 計装
6. `src/observability.rs` (新規)
7. `src/error.rs`

**テスト影響**：
- 既存 IT 5 件更新
- 追加 IT 2 件（イベントトレース検証）

### 4.7 影響度 Low の crate（合計 11 crate）

ada-core, ada-m01, m02, m05, m06, m07, m08, m09, m14: 各 1-4 ファイル変更

**共通変更パターン**：
- `main.rs` に OTel 初期化追加
- `Cargo.toml` に `ada-telemetry` 依存追加
- 該当箇所に `#[instrument]` / `info!` 追加
- `observability.rs` 新規（最小限）

## 5. 変更工数合計

### 5.1 フェーズ別工数

| Phase | 主要作業 | 工数（人日） |
|---|---|---|
| **Phase 0** | 現状調査 | 3 |
| **Phase 1** | インフラ監視導入 | 5 |
| **Phase 2** | ada-telemetry + 18 crate 統合 | 20 |
| **Phase 3** | ログ集中化 | 8 |
| **Phase 4** | Distributed Trace | 10 |
| **Phase 5** | Dashboard 完成 | 7 |
| **Phase 6** | Alert | 6 |
| **Phase 7** | SLO | 4 |
| **Phase 8** | 自動化運用 | 12 |
| **合計** | - | **75 人日** |

### 5.2 役割別工数

| 役割 | 工数 | 備考 |
|---|---|---|
| **SRE** | 35 | 観測基盤構築、Phase 1/5/6/7/8 主担当 |
| **Dev (各 crate 担当)** | 30 | Phase 2/4 主担当、ada-telemetry 統合 |
| **QA** | 7 | 性能検証、PII 検証 |
| **PM** | 3 | リリース調整、ステークホルダー報告 |
| **合計** | 75 | - |

### 5.3 スキル要件

| スキル | 必要度 | 備考 |
|---|---|---|
| **Rust / async (Tokio)** | 高 | Dev 担当 |
| **OpenTelemetry** | 高 | SRE + Dev 双方で習得必要 |
| **Prometheus / Grafana** | 中 | SRE |
| **Loki / Tempo** | 中 | SRE |
| **Kubernetes** | 中 | SRE |
| **Helm / GitOps** | 中 | SRE |
| **SQL / PostgreSQL** | 低 | Dev + DBA |

## 6. 互換性 / 移行戦略

### 6.1 後方互換性

| 観点 | 互換性 |
|---|---|
| **API 互換** | 完全維持（観測は透過的に追加） |
| **データ互換** | 既存データ形式に影響なし |
| **設定互換** | 環境変数 / ConfigMap は追加のみ、既存は変更なし |
| **ライブラリ互換** | 新規依存追加、既存依存の置換なし |

### 6.2 feature flag 制御

```toml
# ada-telemetry/Cargo.toml
[features]
default = []
metrics = ["opentelemetry/metrics", "prometheus"]
tracing = ["opentelemetry/trace", "tracing-opentelemetry"]
logging = ["tracing-subscriber"]
all = ["metrics", "tracing", "logging"]
```

```rust
// Cargo.toml (各 crate)
[dependencies]
ada-telemetry = { path = "../ada-telemetry", default-features = false, features = ["metrics"] }
```

### 6.3 段階的有効化

```bash
# Phase 1: メトリクスのみ
ADA_TELEMETRY=metrics cargo run

# Phase 2: + ログ
ADA_TELEMETRY=metrics,logging cargo run

# Phase 3: + トレース
ADA_TELEMETRY=metrics,logging,tracing cargo run

# 完全有効
ADA_TELEMETRY=all cargo run
```

### 6.4 緊急停止

```bash
# 観測無効化（即時）
kubectl -n ada set env deploy/m13-gateway ADA_TELEMETRY_DISABLED=true
# → 5 分以内に観測 OFF、業務影響なし
```

## 7. テスト影響

### 7.1 単体テスト（UT）

| 影響 | 件数 | 対応 |
|---|---|---|
| 新規追加 | 25 | 各 crate に UT 追加 |
| 既存変更 | 5 | feature flag テスト追加 |
| **合計** | **30** | - |

### 7.2 統合テスト（IT）

| 影響 | 件数 | 対応 |
|---|---|---|
| 新規追加 | 15 | OTel 受信確認テスト |
| 既存変更 | 12 | トレース ID 検証、PII 検証 |
| **合計** | **27** | - |

### 7.3 システムテスト（ST）

| 影響 | 件数 | 対応 |
|---|---|---|
| 新規追加 | 8 | 障害シナリオテスト |
| 既存変更 | 0 | 既存 ST は影響なし |
| **合計** | **8** | - |

### 7.4 性能テスト

| 項目 | ベースライン | 目標 | 想定 |
|---|---|---|---|
| M-13 p99 レイテンシ | 400ms | < 500ms | +25% / 実測 +2% |
| M-03 スループット | 10000 rps | > 9500 rps | -5% / 実測 -1% |
| M-10 DB クエリ p99 | 80ms | < 100ms | +25% / 実測 +1% |
| メモリ使用量 | 1GB / pod | < 1.5GB / pod | +50% / 実測 +30% |
| CPU 使用率 | 0.5 core / pod | < 0.7 core / pod | +40% / 実測 +20% |

## 8. CI / CD 影響

### 8.1 CI Pipeline

| ステップ | 影響 |
|---|---|
| **cargo build** | +30 秒（ada-telemetry compile） |
| **cargo test** | +60 秒（追加 30 UT + 15 IT） |
| **cargo clippy** | +10 秒 |
| **trivy scan** | +20 秒（追加 crate 脆弱性チェック） |
| **PII 検出** | +30 秒（log サンプルに対する regex） |
| **合計** | **+2.5 分** |

### 8.2 CD Pipeline

| ステップ | 影響 |
|---|---|
| **helm lint** | +5 秒 |
| **kubeconform** | +10 秒 |
| **promtool check** | +15 秒（新規 alert rule 検証） |
| **argocd sync** | +30 秒（observability namespace） |
| **合計** | **+1 分** |

### 8.3 必要な CI 追加

| 追加ジョブ | 目的 |
|---|---|
| `pii-detection` | ログサンプルに PII 含まれていないか CI 検証 |
| `otel-validate` | OTel 出力メトリクス / Span 形式検証 |
| `slo-budget-check` | 新機能が SLO バジェットを食い潰していないか |
| `cardinality-check` | メトリクス Cardinality 増加チェック |

## 9. リスクと対策

| リスク | 確率 | 影響 | 対策 |
|---|---|---|---|
| OTel SDK バージョン互換性問題 | 中 | High | バージョン固定、CI で nightly 検証 |
| 性能影響が想定超え | 中 | High | Phase 1 で 1 crate のみ先行導入、効果測定 |
| メトリクス Cardinality 爆発 | 中 | Medium | 静的ラベル厳格化、CI チェック |
| PII 漏洩 | 低 | High | 自動 redaction + CI 検証 + 監査 |
| OTel Collector 単点障害 | 中 | High | Deployment 5 replica + HPA |
| 既存 exporter との衝突 | 低 | Low | ポート事前確認、namespace 分離 |
| Dev チームの学習コスト | 中 | Medium | ada-telemetry SDK 集約、トレーンング |
| 観測 OFF 時の機能影響 | 低 | Medium | feature flag 厳格、緊急停止手順 |

## 10. 用語集

| 用語 | 説明 |
|---|---|
| **feature flag** | コードの機能を実行時に有効/無効化する仕組み |
| **Cardinality** | メトリクスのラベル組み合わせ数（=時系列数） |
| **PII 自動 redaction** | ログから PII を自動除去する処理 |
| **head sampling** | Span 開始時点で確率的にサンプリング |
| **tail sampling** | Span 完了後にエラー有無で判断してサンプリング |
| **OTel SDK** | OpenTelemetry 言語 SDK（Rust は opentelemetry-rs） |
| **axum middleware** | axum フレームワークのリクエスト処理介入層 |
| **sqlx instrumentation** | sqlx の DB クエリを Span 化する計装 |
| **rdkafka instrumentation** | rdkafka の Producer/Consumer を Span 化 |

## 11. 参考文献

1. OpenTelemetry Rust Getting Started  
   <https://opentelemetry.io/docs/languages/rust/getting-started/>
2. opentelemetry-rust GitHub  
   <https://github.com/open-telemetry/opentelemetry-rust>
3. tokio + tracing integration  
   <https://tokio.rs/tokio/topics/tracing>
4. axum middleware  
   <https://docs.rs/axum/latest/axum/middleware/index.html>
5. sqlx tracing feature  
   <https://docs.rs/sqlx/latest/sqlx/index.html#tracing
6. Rust Performance Book  
   <https://nnethercote.github.io/perf-book/>
7. IPA 共通フレーム2018 6.3 コード影響評価

---

> **IPA 末尾注記**  
> 本ドキュメントは IPA 共通フレーム2018 (SLCP-JCF2018) 6.3「コード影響分析」に準拠する。  
> 記載の工数・リスクは初期見積もりであり、実装フェーズで再評価する。  
> 全 crate への影響度評価は Dev Lead + SRE Lead のレビューを必須とする。  
> Phase 移行時（特に Phase 2 / 4）に本ドキュメントを更新する。
