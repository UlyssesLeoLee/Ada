# 06 Grafana ダッシュボードカタログ（Dashboard Catalog）

> すべての Dashboard は **障害特定** に使われる。**見て終わり**のグラフは作らない。
> 番号体系: 00 概要 → 10 Infra → 20 K8s → 30 App → 40 DB → 50 Middleware → 60 Network → 70 Performance → 80 Security → 90 SLO

> **ドキュメントID**：DOC-OBS-006
> **上位文書**：[DOC-OBS-INDEX](README.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版（10 ダッシュボード） |

---

## 目次

1. 設計原則
2. ダッシュボード番号体系
3. 00 System Overview
4. 10 Infrastructure
5. 20 Kubernetes
6. 30 Application
7. 40 Database
8. 50 Middleware
9. 60 Network
10. 70 Performance
11. 80 Security
12. 90 SLA/SLO
13. 業務別ビュー
14. 用語集

---

## 1. 設計原則

| 原則 | 説明 |
|---|---|
| **1 画面 = 1 つの質問に答える** | 「正常か？」「どこ異常？」「いつから？」 |
| **30 秒以内に問題特定** | SRE が on-call で 30 秒以内に状況把握 |
| **全パネルに PromQL/Loki/Trace リンク** | クリックで詳細へ |
| **アラートは Dashboard と連動** | アラート発生時に Dashboard へ deep link |
| **仮定検証用パネル常設** | ボトルネック仮説の検証 |

## 2. ダッシュボード番号体系

| プレフィックス | カテゴリ | 目的 |
|---|---|---|
| 00 | System Overview | 全体俯瞰 |
| 10 | Infrastructure | ノード・ホスト |
| 20 | Kubernetes | クラスタ・Pod |
| 30 | Application | 業務サービス |
| 40 | Database | PostgreSQL |
| 50 | Middleware | Redis / EventBus |
| 60 | Network | HTTP / WS / TCP |
| 70 | Performance | 性能プロファイリング |
| 80 | Security | 認証・認可・監査 |
| 90 | SLA / SLO | サービスレベル |

## 3. 00 System Overview

### 3.1 00-01 Status Board（常時表示）

| Panel | 問い | データソース | 閾値 |
|---|---|---|---|
| **全体 SLA（凡例）** | 正常か？ | PromQL: `1 - avg_over_time(...)` | 99.9% |
| **アクティブテナント数** | 利用者数 | PromQL: `count by (tenant_id) (...)` | — |
| **アクティブユーザー数** | 同時利用 | PromQL: `count by (instance) (...)` | — |
| **リクエスト数 / 分** | トラフィック | PromQL: `sum(rate(...))` | — |
| **エラー率（5m）** | 障害の有無 | PromQL: `sum(rate(errors)) / sum(rate(requests))` | < 0.1% |
| **p99 レイテンシ** | 性能 | PromQL: `histogram_quantile(0.99, ...)` | < 1s |
| **Event Bus 配信遅延** | 内部通信 | PromQL: `histogram_quantile(0.99, m15_consume_lag)` | < 1s |
| **クラスタ健全性** | 可用性 | PromQL: `up{job="cluster"}` | all = 1 |

**配置**: 全 SRE ダッシュボードのトップ、CEO ダッシュボードと同じ

## 4. 10 Infrastructure

### 4.1 10-01 Node Overview

| Panel | 問い | PromQL |
|---|---|---|
| ノード一覧（ステータス） | どのノードが問題か？ | `up` |
| CPU 使用率（ノード別） | 過負荷ノード | `1 - rate(node_cpu_seconds_total{mode="idle"}[5m])` |
| メモリ使用率 | メモリ不足 | `node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes` |
| ディスク使用率 | ディスクフル | `1 - node_filesystem_avail_bytes / node_filesystem_size_bytes` |
| ネットワーク I/O | 帯域使用 | `rate(node_network_receive_bytes_total[5m])` |
| Load Average | 過負荷予兆 | `node_load5` |

### 4.2 10-02 Disk I/O Deep Dive

| Panel | 問い | PromQL |
|---|---|---|
| 読み取りレイテンシ | I/O 遅延 | `rate(node_disk_read_time_seconds_total[5m])` |
| 書き込みレイテンシ | I/O 遅延 | `rate(node_disk_write_time_seconds_total[5m])` |
| IOPS（read/write） | 負荷 | `rate(node_disk_reads_completed_total[5m])` |
| キュー深度 | 飽和 | `node_disk_io_now` |
| 容量推移 | 枯渇予測 | `predict_linear(node_filesystem_avail_bytes[6h], 24*3600)` |

## 5. 20 Kubernetes

### 5.1 20-01 Cluster Overview

| Panel | 問い | PromQL |
|---|---|---|
| Node Status（Ready/NotReady） | クラスタ健全性 | `kube_node_status_condition` |
| Pod 数（namespace 別） | テナント分離 | `count by (namespace) (kube_pod_info)` |
| Deployment Replica Status | スケール正常性 | `kube_deployment_status_replicas_ready` |
| CrashLoopBackOff Pods | 起動失敗 | `kube_pod_container_status_waiting_reason{reason="CrashLoopBackOff"}` |
| OOMKilled Containers | メモリ不足 | `kube_pod_container_status_last_terminated_reason{reason="OOMKilled"}` |
| Pending Pods | スケジュール待ち | `kube_pod_status_phase{phase="Pending"}` |

### 5.2 20-02 Namespace: ada

| Panel | 問い | PromQL |
|---|---|---|
| Pod ステータス（service 別） | サービス健全性 | `kube_pod_status_phase{namespace="ada"}` |
| CPU 使用率（Pod 別） | 過負荷 Pod | `container_cpu_usage_seconds_total{namespace="ada"}` |
| メモリ使用率（Pod 別） | メモリ不足 | `container_memory_working_set_bytes{namespace="ada"}` |
| 再起動回数（累計） | 不安定 Pod | `kube_pod_container_status_restarts_total{namespace="ada"}` |
| HPA 状況 | 自動スケール | `kube_horizontalpodautoscaler_status_current_replicas` |

### 5.3 20-03 Pod Lifecycle

| Panel | 問い | PromQL |
|---|---|---|
| Pod 起動時間（p50/p99） | 起動性能 | `histogram_quantile(0.99, kube_pod_start_time)` |
| コンテナ再起動率 | 不安定度 | `rate(kube_pod_container_status_restarts_total[1h])` |
| Eviction イベント | ノード圧迫 | `kube_pod_evicted` |
| Job 失敗率 | バッチ失敗 | `kube_job_status_failed` |

## 6. 30 Application

### 6.1 30-01 Application Overview（18 crate）

| Panel | 問い | PromQL |
|---|---|---|
| Service health（18 crate） | 全サービス健全性 | `up{job=~"ada-.*"}` |
| Request rate（service 別） | 負荷分散 | `sum by (service) (rate(ada_app_requests_total[5m]))` |
| Error rate（service 別） | サービス品質 | `sum by (service) (rate(ada_app_errors_total[5m])) / sum by (service) (rate(ada_app_requests_total[5m]))` |
| p99 Latency（service 別） | 性能 | `histogram_quantile(0.99, ...)` |
| Active connections | 同時接続 | `ada_app_api_gateway_active_connections` |

### 6.2 30-10 M-13 API Gateway Deep Dive

| Panel | 問い | PromQL |
|---|---|---|
| Endpoint 別 RPS | ホットスポット | `sum by (endpoint) (rate(ada_app_api_gateway_requests_total[5m]))` |
| Endpoint 別エラー率 | 失敗エンドポイント | endpoint 別 errors / requests |
| p99 Latency（endpoint 別） | 遅いエンドポイント | endpoint 別 histogram |
| HTTP Status 分布 | ステータス | `sum by (status) (rate(ada_app_api_gateway_requests_total[5m]))` |
| WebSocket 同時接続 | WS 容量 | `ada_app_api_gateway_active_connections{protocol="ws"}` |
| Rate Limit 発動率 | 制限到達 | `rate(ada_app_api_gateway_rate_limited_total[5m])` |
| JWT 検証失敗率 | 認証問題 | `rate(ada_app_api_gateway_jwt_validation_duration_seconds_count{result="fail"}[5m])` |
| Request size 分布 | 大きなリクエスト | `histogram_quantile(0.95, ada_app_api_gateway_request_size_bytes)` |

### 6.3 30-11 M-03 Data Flow Engine

| Panel | 問い | PromQL |
|---|---|---|
| 実行中のキャンバス数 | 同時実行 | `ada_app_data_flow_engine_active_executions` |
| 実行レイテンシ（canvas 別） | 遅いキャンバス | `histogram_quantile(0.99, ada_app_data_flow_engine_execution_duration_seconds)` |
| ノード実行分布 | どのノードが遅い | `histogram_quantile(0.99, ada_app_data_flow_engine_node_duration_seconds)` |
| エラー（node type 別） | 失敗ノード | `sum by (node_type) (rate(ada_app_data_flow_engine_errors_total[5m]))` |
| Queue depth | バックログ | `ada_app_data_flow_engine_queue_depth` |

### 6.4 30-12 M-15 Event Bus

| Panel | 問い | PromQL |
|---|---|---|
| Publish rate（topic 別） | 流量 | `sum by (topic) (rate(ada_app_event_bus_events_published_total[5m]))` |
| Consume lag（topic 別） | 遅延 | `histogram_quantile(0.99, ada_app_event_bus_event_consume_lag_seconds)` |
| Outbox pending | 未配信 | `ada_app_event_bus_outbox_pending` |
| Duplicates detected | 重複処理 | `rate(ada_app_event_bus_duplicates_detected_total[5m])` |
| Active subscribers | 購読者 | `sum by (topic) (ada_app_event_bus_active_subscribers)` |

### 6.5 30-13 M-10 Tenant Middleware

| Panel | 問い | PromQL |
|---|---|---|
| DB Connection Pool 使用率 | プール枯渇 | `ada_app_tenant_middleware_db_connection_pool_active / ada_app_tenant_middleware_db_connection_pool_max` |
| Query レイテンシ（table 別） | 遅いテーブル | `histogram_quantile(0.99, ada_app_tenant_middleware_db_query_duration_seconds)` |
| PL/pgSQL 呼び出し時間 | ストアド性能 | `histogram_quantile(0.99, ada_app_tenant_middleware_plpgsql_call_duration_seconds)` |
| RLS check 結果 | 越境試行 | `sum by (result) (rate(ada_app_tenant_middleware_rls_check_total[5m]))` |
| Audit log 書き込み | 監査 | `rate(ada_app_tenant_middleware_audit_log_writes_total[5m])` |

### 6.6 30-14 M-11 RBAC + CRDT

| Panel | 問い | PromQL |
|---|---|---|
| Auth 成功率 | 認証品質 | `sum(rate(ada_app_rbac_authentication_total{result="success"}[5m])) / sum(rate(ada_app_rbac_authentication_total[5m]))` |
| Authz deny 率 | 越権試行 | `rate(ada_app_rbac_authorization_check_total{result="deny"}[5m])` |
| JWT active tokens | トークン数 | `ada_app_rbac_jwt_token_active` |
| CRDT 競合 | 共同編集競合 | `rate(ada_app_rbac_crdt_conflicts_total[5m])` |

### 6.7 30-15 M-14 Module Registry

| Panel | 問い | PromQL |
|---|---|---|
| Atomic swap 成功率 | デプロイ健全性 | `sum(rate(ada_app_module_registry_atomic_swap_duration_seconds_count{result="success"}[5m])) / sum(rate(ada_app_module_registry_atomic_swap_duration_seconds_count[5m]))` |
| Swap レイテンシ | デプロイ時間 | `histogram_quantile(0.99, ada_app_module_registry_atomic_swap_duration_seconds)` |
| Active modules | ロード済み | `ada_app_module_registry_active_modules` |
| WASM fuel 消費 | プラグインコスト | `rate(ada_app_module_registry_wasm_fuel_consumed_total[5m])` |
| WASM memory | メモリ使用 | `ada_app_module_registry_wasm_memory_bytes` |

### 6.8 30-16 M-16 Cluster Coordinator

| Panel | 問い | PromQL |
|---|---|---|
| Leader 選出成功率 | クラスタ安定性 | `sum(rate(ada_app_cluster_leader_elected_total{result="success"}[5m])) / sum(rate(ada_app_cluster_leader_elected_total[5m]))` |
| Leader 任期 | 安定性 | `ada_app_cluster_leader_duration_seconds` |
| Heartbeat 失敗率 | ノード健全性 | `sum(rate(ada_app_cluster_heartbeat_total{result="fail"}[5m]))` |
| Shard 分布 | 負荷分散 | `sum by (node_id) (ada_app_cluster_shard_assignments)` |
| Split-brain 検知 | 重大障害 | `rate(ada_app_cluster_split_brain_detected_total[5m])` |

## 7. 40 Database

### 7.1 40-01 PostgreSQL Overview

| Panel | 問い | PromQL |
|---|---|---|
| Up | 接続可 | `pg_up` |
| Connections（active / idle / max） | 接続枯渇 | `pg_stat_activity_count` |
| Transactions / sec | 負荷 | `rate(pg_stat_database_xact_commit[5m])` |
| Slow queries | 性能問題 | `rate(pg_slow_queries_total[5m])` |
| Cache hit ratio | メモリ効率 | `pg_stat_database_blks_hit / (pg_stat_database_blks_hit + pg_stat_database_blks_read)` |
| Locks | 競合 | `pg_locks_count` |
| Deadlocks | 重大 | `rate(pg_stat_database_deadlocks_total[5m])` |
| Replication lag | マルチ AZ 影響 | `pg_replication_lag_seconds` |

### 7.2 40-02 Slow Query Analysis

| Panel | 問い | PromQL |
|---|---|---|
| Top 10 slow queries | 何が遅い | `topk(10, ...)` |
| p99 query duration | 性能目標 | `histogram_quantile(0.99, pg_query_duration_seconds)` |
| Seq scan 多い | インデックス不足 | `rate(pg_stat_user_tables_seq_scan[5m])` |
| Tuple 更新多い | ホットテーブル | `rate(pg_stat_user_tables_n_tup_upd[5m])` |
| Temporary files | メモリ不足 | `rate(pg_stat_database_temp_files[5m])` |

### 7.3 40-03 Tenant Isolation（RLS）

| Panel | 問い | PromQL |
|---|---|---|
| RLS 越境試行（table 別） | セキュリティ | `sum by (table) (rate(ada_app_tenant_middleware_rls_check_total{result="deny"}[5m]))` |
| Tenant 別 query 数 | 不正検出 | `sum by (tenant_id_hash) (rate(ada_app_tenant_middleware_db_query_duration_seconds_count[5m]))` |
| 監査ログ書き込み | 監査 | `rate(ada_app_tenant_middleware_audit_log_writes_total[5m])` |

## 8. 50 Middleware

### 8.1 50-01 Event Bus（M-15）

| Panel | 問い | PromQL |
|---|---|---|
| Throughput（topic 別） | 流量 | `sum by (topic) (rate(ada_app_event_bus_events_published_total[5m]))` |
| Lag 分布 | 遅延 | `histogram_quantile(0.5/0.95/0.99, ada_app_event_bus_event_consume_lag_seconds)` |
| At-least-once 保証 | 重複 | `rate(ada_app_event_bus_duplicates_detected_total[5m])` |
| Outbox backlog | 未配信 | `ada_app_event_bus_outbox_pending` |
| 失敗率 | 配信失敗 | `sum(rate(ada_app_event_bus_events_published_total{result="fail"}[5m]))` |

### 8.2 50-02 Redis（計画）

| Panel | 問い | PromQL |
|---|---|---|
| Memory 使用 | 容量 | `redis_memory_used_bytes` |
| Cache hit ratio | 効率 | `redis_keyspace_hits_total / (redis_keyspace_hits_total + redis_keyspace_misses_total)` |
| Evictions | 容量不足 | `rate(redis_evicted_keys_total[5m])` |
| Connected clients | 接続数 | `redis_connected_clients` |
| Command latency | 性能 | `histogram_quantile(0.99, redis_command_duration_seconds)` |

## 9. 60 Network

### 9.1 60-01 HTTP / WebSocket

| Panel | 問い | PromQL |
|---|---|---|
| RPS（endpoint × method） | 流量 | `sum by (endpoint, method) (rate(ada_app_api_gateway_requests_total[5m]))` |
| Status code 分布 | エラー率 | `sum by (status) (rate(ada_app_api_gateway_requests_total[5m]))` |
| p50 / p95 / p99 レイテンシ | 性能 | `histogram_quantile(0.5/0.95/0.99, ada_app_api_gateway_request_duration_seconds)` |
| Request size / Response size | 帯域 | `histogram_quantile(0.95, ...)` |
| WebSocket 同時接続 | WS 容量 | `ada_app_api_gateway_active_connections{protocol="ws"}` |
| WS 送受信レート | 通信 | `rate(ada_app_api_gateway_websocket_messages_total[5m])` |

### 9.2 60-02 Network I/O

| Panel | 問い | PromQL |
|---|---|---|
| Node 別送受信 | 帯域 | `rate(node_network_*_bytes_total[5m])` |
| ネットワークエラー | 問題 | `rate(node_network_*_errs_total[5m])` |
| アクティブ接続 | 容量 | `node_netstat_Tcp_CurrEstab` |
| TCP retransmission | 品質 | `node_netstat_Tcp_RetransSegs` |

## 10. 70 Performance

### 10.1 70-01 p99 Latency Budget

| Panel | 問い | PromQL |
|---|---|---|
| p99 latency budget consumption | 予算内か | `histogram_quantile(0.99, ...) / budget * 100` |
| Latency by service | 遅いサービス | `histogram_quantile(0.99, ada_app_*_request_duration_seconds)` |
| DB query latency | 遅いクエリ | `histogram_quantile(0.99, ada_app_tenant_middleware_db_query_duration_seconds)` |
| WASM execution time | プラグイン性能 | `histogram_quantile(0.99, ada_app_module_registry_wasm_executions)` |

### 10.2 70-02 Throughput

| Panel | 問い | PromQL |
|---|---|---|
| RPS（総数） | 容量 | `sum(rate(ada_app_*_requests_total[5m]))` |
| Event publish rate | 内部通信 | `sum(rate(ada_app_event_bus_events_published_total[5m]))` |
| DB QPS | 永続化 | `sum(rate(pg_stat_database_xact_commit[5m]))` |
| Capacity headroom | 余裕 | `(current / max) * 100` |

### 10.3 70-03 Runtime（Tokio / Rust）

| Panel | 問い | PromQL |
|---|---|---|
| Tokio tasks active | 並行度 | `ada_app_*_tokio_tasks_active` |
| Tokio tasks spawned total | タスク生成 | `rate(ada_app_*_tokio_tasks_total[5m])` |
| Panics | 重大 | `rate(ada_app_*_panics_total[5m])` |
| ヒープ使用 | メモリ | `ada_infra_process_resident_memory_bytes` |

## 11. 80 Security

### 11.1 80-01 Authentication

| Panel | 問い | PromQL |
|---|---|---|
| Auth 成功/失敗 | 認証品質 | `sum by (result) (rate(ada_app_rbac_authentication_total[5m]))` |
| Auth 失敗多い IP | ブルートフォース | top IP by failed auth |
| JWT validation 失敗 | 改ざん | `rate(ada_app_api_gateway_jwt_validation_duration_seconds_count{result="fail"}[5m])` |

### 11.2 80-02 Authorization

| Panel | 問い | PromQL |
|---|---|---|
| Authz deny（action 別） | 越権試行 | `sum by (action) (rate(ada_app_rbac_authorization_check_total{result="deny"}[5m]))` |
| RLS 越境 | テナント越境 | `sum by (table) (rate(ada_app_tenant_middleware_rls_check_total{result="deny"}[5m]))` |
| 権限変更 | ガバナンス | `audit_log` from Loki |

### 11.3 80-03 Audit

| Panel | 問い | PromQL |
|---|---|---|
| 監査ログ書き込み | 監査 | `rate(ada_app_tenant_middleware_audit_log_writes_total[5m])` |
| GDPR 削除 | コンプラ | `audit_log` filter event=gdpr_user_forgotten |
| 設定変更 | ガバナンス | `audit_log` filter event=config_changed |

## 12. 90 SLA / SLO

### 12.1 90-01 Service SLO

各 crate の SLO：

| Service | SLI | SLO | Burn Rate |
|---|---|---|---|
| M-13 Gateway | Availability | 99.9% | < 1% / 1h |
| M-13 Gateway | Latency p99 | < 500ms | < 1% / 1h |
| M-03 Engine | Availability | 99.5% | < 1% / 1h |
| M-10 Tenant | DB Query p99 | < 100ms | < 1% / 1h |
| M-15 EventBus | Lag p99 | < 1s | < 1% / 1h |

### 12.2 90-02 Error Budget

| Service | 月間 Error Budget | 残量 |
|---|---|---|
| M-13 Gateway | 43.2 分 | （PromQL で計算） |
| M-03 Engine | 216 分 | |
| M-10 Tenant | 432 分 | |

### 12.3 90-03 Burn Rate

| Alert | 1h burn | 6h burn | 24h burn |
|---|---|---|---|
| Page | > 14.4× (1h 予算) | > 6× (6h 予算) | — |
| Ticket | — | > 3× (6h 予算) | > 1× (24h 予算) |

## 13. 業務別ビュー

| 業務 | ダッシュボード | 主要パネル |
|---|---|---|
| 営業（営業企画 田中） | 00 + 30-11 | Latency + Success rate |
| データエンジニア（鈴木） | 30-11 + 30-12 + 70-01 | Canvas execution |
| IT 管理者（佐藤） | 80-01 + 80-02 + 90 | Auth + SLO |
| SRE（山田） | 00 + 10-20-30-40 全部 | 全体 |

## 14. 用語集

| 用語 | 説明 |
|---|---|
| Dashboard | Grafana の表示画面 |
| Panel | ダッシュボード内の 1 グラフ |
| PromQL | Prometheus Query Language |
| LogQL | Loki Query Language |
| Burn Rate | 予算消費速度 |
| RED | Rate / Errors / Duration |
| USE | Utilization / Saturation / Errors |
| SLO | Service Level Objective |

## 15. 参考文献

1. Grafana Dashboard Best Practices
2. RED Method (Tom Wilkie)
3. USE Method (Brendan Gregg)
4. Google SRE Book 第 2 版
5. Ada プロジェクトチーム「[DOC-REQ-NFR-001 NFR](../requirements/05-nfr-non-functional-requirements.md)」

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
