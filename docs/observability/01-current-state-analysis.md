# 01 現状分析（Current State Analysis）

> **Phase 0 成果物**。既存システムの実態を可視化してから設計する。**禁止：直接インストール／直接変更**。

> **ドキュメントID**：DOC-OBS-001
> **文書分類**：アーキテクチャ設計書
> **バージョン**：v1.0.0
> **最終更新日**：2026-08-20
> **上位文書**：[DOC-OBS-INDEX](README.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版 |

---

## 目次

1. プロジェクト位置付け
2. サービス構成（計画）
3. コードベース（scaffold 状態）
4. 通信方式
5. ストレージ層
6. デプロイ計画
7. 既存 observability 言及
8. ギャップ分析
9. 結論

---

## 1. プロジェクト位置付け

| 項目 | 値 |
|---|---|
| プロジェクト | Ada 无限画布跨平台数据集成系统 |
| コードバージョン | **v0.1.0 scaffold**（実装前） |
| ドキュメント | v2.1.0（86 DOC-ID 完了） |
| 言語 | **Rust 1.74+** 必須 |
| ターゲット OS | macOS 14+ / Linux (Ubuntu 22.04+) / Windows 11+ |
| デプロイモード | 単機 / SaaS マルチテナント / ハイブリッド |
| アーキテクチャ | 仿生モデル 4 層（骨/血/神経/筋肉） |

## 2. サービス構成（計画）

### 2.1 18 Rust crate（[DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md) より）

| 層 | crate 名 | 役割 | 通信 |
|---|---|---|---|
| **骨 (skeleton)** | ada-m10-tenant-middleware | PostgreSQL 接続、11 テーブル + 6 PL/pgSQL + RLS | TCP/TLS |
| | ada-m11-rbac-collab | RBAC + Yrs CRDT 共同編集 | WebSocket |
| | ada-m13-api-gateway | axum REST + WebSocket | HTTP/WS |
| | ada-m16-cluster-coordinator | リーダー選出、Shard 割当 | TCP internal |
| **血 (blood)** | ada-m01-acquisition | REST / DB / gRPC / WS / File 取得 | 5 種 |
| | ada-m02-normalizer | NJSON 標準化 | in-proc |
| | ada-m03-data-flow-engine | キャンバス実行 | in-proc + IPC |
| | ada-m09-exporter | 外部出力（5 種） | HTTP/gRPC/File |
| **神経 (nerve)** | ada-m04-orchestration | DAG ベース依存解決 | IPC |
| | ada-m05-control-flow | 条件分岐 / ループ | in-proc |
| | ada-m08-trigger | cron / Webhook / イベント | HTTP + Cron |
| | ada-m15-central-event-bus | Pub/Sub（at-least-once） | in-proc + TCP |
| **筋肉 (muscle)** | ada-m06-plugin-sdk | プラグイン SDK → WASM | FFI / WASM |
| | ada-m07-debug | デバッグサービス | WebSocket |
| | ada-m12-canvas-editor | Bevy 0.14 WASM フロントエンド | WS + REST |
| | ada-m14-module-registry | モジュール動的ロード + atomic swap | HTTP internal |
| **共有** | ada-core | 共有型、エラー、Result 型 | in-proc |
| | ada-telemetry | **本 Observability Platform の計装先** | OTLP |

### 2.2 サービス数集計

| 区分 | 数 |
|---|---|
| ネットワーク公開サービス | 1（M-13 API Gateway） |
| 内部サービス間通信 | 16（残り 17 crate のうち 1 は in-proc） |
| データベース | 1（PostgreSQL）+ 計画 1（Redis） |
| メッセージング | 1（M-15 EventBus、Pub/Sub） |
| キャッシュ | 0（将来 Redis） |
| ファイルストレージ | 0（将来 S3） |
| フロントエンド | 1（M-12 Bevy WASM） |

## 3. コードベース（scaffold 状態）

各 crate には以下が実装予定（[DOC-ARCH-007 §5-§10](../architecture/06-rust-tech-selection.md)）：

| 依存 | crate | 用途 |
|---|---|---|
| HTTP | axum 0.7+ | サーバー/クライアント |
| 非同期 | tokio 1.40+ | ランタイム |
| トレース | **tracing 0.1+ + tracing-subscriber 0.3+** | 構造化ログ + span |
| OpenTelemetry | **opentelemetry 0.24+ + opentelemetry-otlp 0.17+** | OTLP 出力 |
| メトリクス | **opentelemetry-prometheus** または **metrics 0.23+** | Prometheus 出力 |
| ログ | **tracing-subscriber + tracing-bunyan-formatter** | 構造化ログ |
| DB | sqlx 0.8+ (PostgreSQL) | DB アクセス |
| エラー | thiserror 2.0+ / anyhow 1.0+ | エラー処理 |
| シリアライズ | serde 1.0+ | JSON |
| ID | uuid 1.10+ (v4, v7) | 識別子 |
| 時間 | chrono 0.4+ | タイムスタンプ |

## 4. 通信方式

| 通信 | プロトコル | 用途 | 計装ポイント |
|---|---|---|---|
| **Client → Gateway** | HTTPS REST + WebSocket | エンドユーザーアクセス | HTTP middleware（axum-tracing, tower-http TraceLayer） |
| **Gateway → Service** | in-proc / Unix socket / TCP | 内部 RPC | tonic gRPC interceptor（将来）または in-proc span 伝播 |
| **Service → DB** | TCP/TLS（PostgreSQL wire protocol） | データ永続化 | sqlx::query の instrumentation |
| **Service → EventBus** | in-proc (M-15 直接呼出) | 非同期イベント | カスタム span + metric |
| **Service → Plugin** | WASM 実行 | ユーザー拡張 | wasmtime の計装フック |
| **Service → External API** | HTTPS / WSS / gRPC | 外部連携 | reqwest / tonic middleware |
| **WASM → Backend** | WebSocket | キャンバス状態同期 | WS middleware |
| **K8s Control Plane** | HTTPS (kube-apiserver) | クラスタ管理 | kube-state-metrics exporter |

## 5. ストレージ層

| DB / Storage | バージョン | 役割 | 監視対象 |
|---|---|---|---|
| **PostgreSQL** | 16+ | 主 DB、11 テーブル、6 PL/pgSQL、RLS | pg_stat_statements, slow query log, lock wait, replication lag |
| **Redis**（計画） | 7+ | キャッシュ、Pub/Sub 補助 | redis_exporter, commandstats, memory |
| **S3 互換**（将来） | — | Backup, アーカイブ | メトリクス exporter 経由 |

## 6. デプロイ計画

| 項目 | 値 |
|---|---|
| オーケストレーション | **K3s**（[DOC-ARCH-002 §1](../architecture/01-tech-stack.md)） |
| コンテナ | Docker 24+ |
| Service Mesh | 未選定（将来 Linkerd 候補） |
| CI/CD | GitHub Actions |
| 環境 | dev / staging / production（3 環境） |
| GitOps | 将来 ArgoCD |
| ノード数 | 3+（[NFR-AVA-06](../requirements/05-nfr-non-functional-requirements.md)） |
| ネットワーク | マルチ AZ |

## 7. 既存 observability 言及

| ソース | 該当 | 内容 |
|---|---|---|
| [DOC-ARCH-006 管理画面](../architecture/05-admin-operations-ui.md) | §5, §6 | 監視 UI 8 件 |
| [DOC-ARCH-007 Rust 選択](../architecture/06-rust-tech-selection.md) | §10 | **tracing + tracing-subscriber** 採用決定 |
| [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) | §15 | SAST（cargo-deny, cargo-audit） |
| [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) | §17 | CI（GitHub Actions） |
| [DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md) | 39 件 | 監査・監視項目 |
| [DOC-ARCH-009 ワークフロー](../architecture/08-workflow-overview.md) | §6.8 | 運用試験 |
| [DOC-ARCH-009](../architecture/08-workflow-overview.md) | §9 | 監査チェックポイント（15 項目） |
| [DOC-REQ-NFR-001 NFR](../requirements/05-nfr-non-functional-requirements.md) | §2-§7 | [NF-AVA\|PER\|OPS] 計 41 項目 |
| [M-03 データフロー](../modules/M-03-data-flow-engine.md) | 13 件 | 実行監視、性能計測 |
| [M-10 テナント](../modules/M-10-tenant-middleware.md) | §4.4 | audit_log（1 年保存） |
| [M-15 中央イベントバス](../modules/M-15-central-event-bus.md) | §2 | イベント配信保証 |
| [M-16 クラスタ](../modules/M-16-cluster-coordinator.md) | §3 | リーダー選出、Shard 監視 |

## 8. ギャップ分析

| 領域 | 現状 | 目標 | ギャップ |
|---|---|---|---|
| **Metrics** | ❌ なし | RED + USE 全 18 crate 計装 | **高** |
| **Logs** | ❌ 構造化ログ未設計 | 全 crate で JSON 構造化ログ + 脱敏 | **高** |
| **Traces** | ❌ なし | distributed trace 100% 計装 | **高** |
| **Backend** | ❌ なし | Prometheus + Loki + Tempo | **高** |
| **Visualization** | ❌ なし | Grafana + 10 Dashboard | **中** |
| **Alert** | ❌ なし | 4 段階（Sev1-4）+ SLO 連携 | **中** |
| **SLO/SLI** | 部分（NFR あり） | SLO 定義 + Error Budget 管理 | **中** |
| **Security** | ❌ なし | RBAC + テナント隔離 + 認証 | **高** |
| **Deployment** | ❌ なし | observability namespace + Helm | **中** |
| **Documentation** | 散在（DOC-ARCH-007 §10 のみ） | **本 Observability 設計書で統合** | — |

### 8.1 設計上の重要制約

| 制約 | 影響 | 対応 |
|---|---|---|
| **マルチテナント** | テナント間データ隔離必須 | ラベル `tenant_id` 必須付与 + アクセス制御 |
| **GDPR / PIPL** | PII を log/trace に含めない | 脱敏フィールド明示 + 自動検証 |
| **3 OS 対応** | 同一設定でビルド | OpenTelemetry SDK が OS 抽象化 |
| **WASM フロント** | ブラウザ内計装 | OTel JS / Web トレーシング |
| **プラグイン沙箱** | WASM 実行の観測 | wasmtime 計装フック |
| **CRDT 共同編集** | 競合解決の観測 | カスタム span 記録 |
| **NFR-PER-04 起動 < 3s** | 計装は低オーバーヘッド | sampling、batch 送信 |

## 9. 結論

### 9.1 設計方針

1. **OpenTelemetry を唯一の計装 SDK** に統一（業務コードからの直接依存を禁止）
2. **Grafana スタック**（Prometheus + Loki + Tempo）採用
3. **otel-collector** 単一バイナリで受信・加工・転送
4. **マルチテナント**をラベルで識別し、アクセス制御で隔離
5. **GDPR 準拠**のため、PII フィールドは自動脱敏
6. **段階導入**（Phase 0-8）でリスク最小化

### 9.2 優先度

| 優先度 | 領域 | 理由 |
|---|---|---|
| **P0** | Infrastructure Metrics（K3s/Node） | 即時可視化、容量計画 |
| **P0** | Application Metrics（[M-13 Gateway](../modules/M-13-api-gateway.md)） | エンドユーザー観測 |
| **P1** | Logs（Loki、構造化） | 障害詳細 |
| **P1** | Traces（[M-15 EventBus](../modules/M-15-central-event-bus.md) + Gateway） | 性能ボトルネック |
| **P2** | Database Metrics（PostgreSQL 18.6） | 性能分析 |
| **P2** | Alert + SLO | 自動運用 |
| **P3** | CRDT / Plugin 詳細観測 | 高度分析 |

## 10. 用語集

| 用語 | 説明 |
|---|---|
| scaffold | 骨組みのみ実装された状態 |
| in-proc | 同一プロセス内通信 |
| OTLP | OpenTelemetry Protocol |
| K3s | 軽量 Kubernetes ディストリビューション |
| Tenant | マルチテナント環境での顧客単位 |

## 11. 参考文献

1. Ada プロジェクトチーム「[DOC-ARCH-001 仿生モデル](../architecture/00-anatomy-model.md)」, 2026-08-19
2. Ada プロジェクトチーム「[DOC-ARCH-002 技術スタック](../architecture/01-tech-stack.md)」, 2026-08-19
3. Ada プロジェクトチーム「[DOC-ARCH-007 Rust 選択](../architecture/06-rust-tech-selection.md)」, 2026-08-19
4. Ada プロジェクトチーム「[DOC-REQ-NFR-001 NFR](../requirements/05-nfr-non-functional-requirements.md)」, 2026-08-20
5. OpenTelemetry Documentation
6. Prometheus Best Practices

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
