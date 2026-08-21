# 03 指標体系設計（Metrics Design）

> RED（Rate/Errors/Duration）+ USE（Utilization/Saturation/Errors）フレームワーク適用。**全 18 crate** で一貫したメトリクス命名と必須カーディナリティを強制。

> **ドキュメントID**：DOC-OBS-003
> **上位文書**：[DOC-OBS-INDEX](README.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版 |

---

## 目次

1. 設計フレームワーク
2. 命名規約
3. Infrastructure Metrics（USE）
4. Kubernetes Metrics
5. Application Metrics（RED）
6. Database Metrics
7. Middleware Metrics
8. 必須ラベル
9. カーディナリティ管理
10. 業務別 metrics 一覧
11. 用語集

---

## 1. 設計フレームワーク

### 1.1 USE（インフラ層）

| メトリック | 説明 | 対象 |
|---|---|---|
| **U**tilization | リソース使用率（%） | CPU, Memory, Disk, Network |
| **S**aturation | キュー深度、待機列 | Run Queue, Socket Buffer, DB Connection Pool |
| **E**rrors | エラー率 | I/O Error, Network Error, HW Error |

### 1.2 RED（アプリ層）

| メトリック | 説明 | 計算 |
|---|---|---|
| **R**ate | リクエスト数 / 秒 | `rate(requests_total[5m])` |
| **E**rrors | エラー率 | `rate(errors_total[5m]) / rate(requests_total[5m])` |
| **D**uration | レイテンシ | `histogram_quantile(0.99, ...)` |

## 2. 命名規約

```
ada.{layer}.{component}.{metric}_{unit}

例:
  ada.app.api_gateway.requests_total{endpoint, method, status}
  ada.app.data_flow_engine.executions_total{canvas_id, status}
  ada.app.event_bus.events_published_total{topic}
  ada.app.tenant_middleware.db_query_duration_seconds{query_type, table}
  ada.infra.node.cpu_utilization_ratio
  ada.k8s.pod.restart_total{pod, namespace}
```

### 2.1 必須プレフィックス

| プレフィックス | 意味 | 例 |
|---|---|---|
| `ada.app.*` | アプリケーションメトリクス | `ada.app.api_gateway.*` |
| `ada.infra.*` | Infrastructure メトリクス | `ada.infra.node.*` |
| `ada.k8s.*` | Kubernetes メトリクス | `ada.k8s.pod.*` |
| `ada.db.*` | データベース メトリクス | `ada.db.postgres.*` |
| `ada.cache.*` | キャッシュ メトリクス | `ada.cache.redis.*` |
| `ada.mq.*` | メッセージキュー メトリクス | `ada.mq.event_bus.*` |
| `ada.runtime.*` | 言語ランタイム メトリクス | `ada.runtime.tokio.*` |

### 2.2 サフィックス規約

| サフィックス | データ型 | 単位 |
|---|---|---|
| `_total` | Counter | 件数（単調増加） |
| `_count` | Counter | 件数（リセット可） |
| `_size` | Gauge | バイト |
| `_bytes` | Histogram | バイト |
| `_seconds` | Histogram | 秒 |
| `_ratio` | Gauge | 0.0〜1.0 |
| `_current` | Gauge | 現在値 |
| `_max` / `_min` | Gauge | 極値 |

## 3. Infrastructure Metrics（USE）

> ソース：[node_exporter](https://github.com/prometheus/node_exporter) + cAdvisor

### 3.1 CPU

| メトリック | タイプ | 用途 |
|---|---|---|
| `ada_infra_node_cpu_utilization_ratio` | Gauge | CPU 使用率（0-1） |
| `ada_infra_node_cpu_count` | Gauge | 論理 CPU 数 |
| `ada_infra_node_load_average_1m` | Gauge | 1 分平均 Load |
| `ada_infra_node_load_average_5m` | Gauge | 5 分平均 Load |
| `ada_infra_node_context_switches_total` | Counter | コンテキストスイッチ数 |
| `ada_infra_node_cpu_frequency_hertz` | Gauge | CPU 周波数 |

### 3.2 Memory

| メトリック | タイプ | 用途 |
|---|---|---|
| `ada_infra_node_memory_used_bytes` | Gauge | メモリ使用量 |
| `ada_infra_node_memory_total_bytes` | Gauge | メモリ総量 |
| `ada_infra_node_memory_utilization_ratio` | Gauge | 使用率（0-1） |
| `ada_infra_node_memory_swap_used_bytes` | Gauge | スワップ使用量 |
| `ada_infra_node_memory_cached_bytes` | Gauge | キャッシュ使用量 |
| `ada_infra_node_memory_available_bytes` | Gauge | 利用可能メモリ |

### 3.3 Disk

| メトリック | タイプ | 用途 |
|---|---|---|
| `ada_infra_node_disk_used_bytes` | Gauge | ディスク使用量 |
| `ada_infra_node_disk_total_bytes` | Gauge | ディスク総量 |
| `ada_infra_node_disk_utilization_ratio` | Gauge | 使用率 |
| `ada_infra_node_disk_read_bytes_total` | Counter | 読み取りバイト累計 |
| `ada_infra_node_disk_written_bytes_total` | Counter | 書き込みバイト累計 |
| `ada_infra_node_disk_inodes_free` | Gauge | 空き i-node |

### 3.4 Network

| メトリック | タイプ | 用途 |
|---|---|---|
| `ada_infra_node_network_receive_bytes_total` | Counter | 受信バイト累計 |
| `ada_infra_node_network_transmit_bytes_total` | Counter | 送信バイト累計 |
| `ada_infra_node_network_receive_errors_total` | Counter | 受信エラー累計 |
| `ada_infra_node_network_transmit_errors_total` | Counter | 送信エラー累計 |
| `ada_infra_node_network_connections` | Gauge | アクティブ接続数 |

### 3.5 Process

| メトリック | タイプ | 用途 |
|---|---|---|
| `ada_infra_process_cpu_seconds_total` | Counter | プロセス CPU 時間 |
| `ada_infra_process_resident_memory_bytes` | Gauge | RSS メモリ |
| `ada_infra_process_virtual_memory_bytes` | Gauge | VSZ メモリ |
| `ada_infra_process_open_fds` | Gauge | 開いている FD 数 |
| `ada_infra_process_max_fds` | Gauge | 最大 FD 数 |
| `ada_infra_process_threads` | Gauge | スレッド数 |
| `ada_infra_process_start_time_seconds` | Gauge | プロセス開始時刻 |

## 4. Kubernetes Metrics

> ソース：[kube-state-metrics](https://github.com/kubernetes/kube-state-metrics) + cAdvisor

| メトリック | タイプ | 用途 |
|---|---|---|
| `ada_k8s_pod_status_phase` | Gauge | Pod Phase (Running/Pending/Failed) |
| `ada_k8s_pod_restart_total` | Counter | 累積再起動回数 |
| `ada_k8s_pod_container_status_restarts_total` | Counter | コンテナ再起動 |
| `ada_k8s_pod_resource_cpu_usage_cores` | Gauge | CPU 使用量 |
| `ada_k8s_pod_resource_memory_usage_bytes` | Gauge | メモリ使用量 |
| `ada_k8s_deployment_replicas_desired` | Gauge | Desired Replicas |
| `ada_k8s_deployment_replicas_ready` | Gauge | Ready Replicas |
| `ada_k8s_deployment_replicas_available` | Gauge | Available Replicas |
| `ada_k8s_deployment_replicas_unavailable` | Gauge | Unavailable Replicas |
| `ada_k8s_node_status_condition` | Gauge | ノード状態（Ready/NotReady） |
| `ada_k8s_cronjob_status_last_successful_time` | Gauge | 最終成功時刻 |
| `ada_k8s_job_status_failed` | Gauge | 失敗回数 |
| `ada_k8s_pod_crashloopbackoff` | Counter | CrashLoopBackOff 発生回数 |
| `ada_k8s_namespace_resource_quota_cpu_cores` | Gauge | 名前空間 CPU クォータ |

## 5. Application Metrics（RED）

### 5.1 M-13 API Gateway

| メトリック | タイプ | ラベル |
|---|---|---|
| `ada_app_api_gateway_requests_total` | Counter | `endpoint`, `method`, `status` |
| `ada_app_api_gateway_request_duration_seconds` | Histogram | `endpoint`, `method` |
| `ada_app_api_gateway_active_connections` | Gauge | `protocol` (http/ws) |
| `ada_app_api_gateway_errors_total` | Counter | `endpoint`, `error_code` |
| `ada_app_api_gateway_websocket_messages_total` | Counter | `direction` (in/out) |
| `ada_app_api_gateway_jwt_validation_duration_seconds` | Histogram | `result` (success/fail) |
| `ada_app_api_gateway_rate_limited_total` | Counter | `tenant_id` |
| `ada_app_api_gateway_request_size_bytes` | Histogram | `endpoint` |
| `ada_app_api_gateway_response_size_bytes` | Histogram | `endpoint` |

### 5.2 M-03 Data Flow Engine

| メトリック | タイプ | ラベル |
|---|---|---|
| `ada_app_data_flow_engine_executions_total` | Counter | `canvas_id`, `status` |
| `ada_app_data_flow_engine_execution_duration_seconds` | Histogram | `canvas_id` |
| `ada_app_data_flow_engine_nodes_executed_total` | Counter | `node_type` |
| `ada_app_data_flow_engine_node_duration_seconds` | Histogram | `node_type` |
| `ada_app_data_flow_engine_active_executions` | Gauge | — |
| `ada_app_data_flow_engine_queue_depth` | Gauge | `queue_name` |
| `ada_app_data_flow_engine_errors_total` | Counter | `node_type`, `error_code` |

### 5.3 M-15 Central Event Bus

| メトリック | タイプ | ラベル |
|---|---|---|
| `ada_app_event_bus_events_published_total` | Counter | `topic`, `result` |
| `ada_app_event_bus_events_consumed_total` | Counter | `topic`, `consumer_group` |
| `ada_app_event_bus_event_publish_duration_seconds` | Histogram | `topic` |
| `ada_app_event_bus_event_consume_lag_seconds` | Histogram | `topic` |
| `ada_app_event_bus_active_subscribers` | Gauge | `topic` |
| `ada_app_event_bus_outbox_pending` | Gauge | — |
| `ada_app_event_bus_duplicates_detected_total` | Counter | `topic` (idempotency 確認) |

### 5.4 M-10 Tenant Middleware

| メトリック | タイプ | ラベル |
|---|---|---|
| `ada_app_tenant_middleware_db_query_duration_seconds` | Histogram | `query_type`, `table` |
| `ada_app_tenant_middleware_db_connection_pool_active` | Gauge | — |
| `ada_app_tenant_middleware_db_connection_pool_idle` | Gauge | — |
| `ada_app_tenant_middleware_db_connection_pool_max` | Gauge | — |
| `ada_app_tenant_middleware_rls_check_total` | Counter | `table`, `result` (allow/deny) |
| `ada_app_tenant_middleware_tenant_resolution_duration_seconds` | Histogram | `result` |
| `ada_app_tenant_middleware_audit_log_writes_total` | Counter | `result` |
| `ada_app_tenant_middleware_plpgsql_call_duration_seconds` | Histogram | `procedure_name` |

### 5.5 M-11 RBAC + Collab（CRDT）

| メトリック | タイプ | ラベル |
|---|---|---|
| `ada_app_rbac_authentication_total` | Counter | `result` (success/fail) |
| `ada_app_rbac_authorization_check_total` | Counter | `action`, `resource`, `result` |
| `ada_app_rbac_jwt_token_active` | Gauge | `tenant_id` (PII 配慮) |
| `ada_app_rbac_crdt_operations_total` | Counter | `operation` (insert/update/delete), `document_id` |
| `ada_app_rbac_crdt_conflicts_total` | Counter | `document_id` |
| `ada_app_rbac_crdt_sync_duration_seconds` | Histogram | `peer` |

### 5.6 M-14 Module Registry

| メトリック | タイプ | ラベル |
|---|---|---|
| `ada_app_module_registry_load_total` | Counter | `result` |
| `ada_app_module_registry_atomic_swap_duration_seconds` | Histogram | `module_id` |
| `ada_app_module_registry_active_modules` | Gauge | — |
| `ada_app_module_registry_wasm_executions_total` | Counter | `module_id`, `result` |
| `ada_app_module_registry_wasm_fuel_consumed_total` | Counter | `module_id` |
| `ada_app_module_registry_wasm_memory_bytes` | Gauge | `module_id` |

### 5.7 M-16 Cluster Coordinator

| メトリック | タイプ | ラベル |
|---|---|---|
| `ada_app_cluster_leader_elected_total` | Counter | `result` |
| `ada_app_cluster_leader_duration_seconds` | Gauge | `node_id` |
| `ada_app_cluster_heartbeat_total` | Counter | `node_id`, `result` |
| `ada_app_cluster_shard_assignments` | Gauge | `shard_id`, `node_id` |
| `ada_app_cluster_split_brain_detected_total` | Counter | — |

### 5.8 全 18 crate 共通

| メトリック | タイプ | ラベル |
|---|---|---|
| `ada_app_{crate}_uptime_seconds` | Gauge | — |
| `ada_app_{crate}_build_info` | Gauge (1) | `version`, `git_sha` |
| `ada_app_{crate}_panics_total` | Counter | — |
| `ada_app_{crate}_tokio_tasks_active` | Gauge | — |
| `ada_app_{crate}_tokio_tasks_total` | Counter | — |

## 6. Database Metrics

> ソース：[postgres_exporter](https://github.com/prometheus-community/postgres_exporter)

| メトリック | タイプ | 用途 |
|---|---|---|
| `ada_db_postgres_up` | Gauge | DB 接続可能か |
| `ada_db_postgres_connections` | Gauge | 現在接続数 |
| `ada_db_postgres_connections_max` | Gauge | 最大接続数 |
| `ada_db_postgres_connections_idle` | Gauge | アイドル接続数 |
| `ada_db_postgres_query_duration_seconds` | Histogram | クエリ実行時間 |
| `ada_db_postgres_slow_queries_total` | Counter | 1 秒超のクエリ数 |
| `ada_db_postgres_transactions_total` | Counter | トランザクション数 |
| `ada_db_postgres_commits_total` | Counter | コミット数 |
| `ada_db_postgres_rollbacks_total` | Counter | ロールバック数 |
| `ada_db_postgres_locks_total` | Gauge | 現在ロック数 |
| `ada_db_postgres_deadlocks_total` | Counter | デッドロック数 |
| `ada_db_postgres_cache_hit_ratio` | Gauge | キャッシュヒット率 |
| `ada_db_postgres_index_scans_total` | Counter | インデックススキャン数 |
| `ada_db_postgres_seq_scans_total` | Counter | シーケンシャルスキャン数 |
| `ada_db_postgres_replication_lag_seconds` | Gauge | レプリケーション遅延 |
| `ada_db_postgres_tuples_returned_total` | Counter | 返却行数 |
| `ada_db_postgres_tuples_fetched_total` | Counter | 取得行数 |
| `ada_db_postgres_tuples_inserted_total` | Counter | 挿入行数 |
| `ada_db_postgres_tuples_updated_total` | Counter | 更新行数 |
| `ada_db_postgres_tuples_deleted_total` | Counter | 削除行数 |
| `ada_db_postgres_wal_receipt_lag_seconds` | Gauge | WAL 受信遅延 |
| `ada_db_postgres_wal_replay_lag_seconds` | Gauge | WAL 適用遅延 |
| `ada_db_postgres_temp_files_total` | Counter | 一時ファイル数 |
| `ada_db_postgres_temp_bytes_total` | Counter | 一時バイト数 |
| `ada_db_postgres_deadlocks_per_second` | Gauge | デッドロック率 |
| `ada_db_postgres_active_connections` | Gauge | アクティブ接続数 |
| `ada_db_postgres_idle_in_transaction_connections` | Gauge | idle-in-transaction 接続数 |

## 7. Middleware Metrics

### 7.1 Redis（計画）

| メトリック | タイプ | 用途 |
|---|---|---|
| `ada_cache_redis_up` | Gauge | 接続可 |
| `ada_cache_redis_commands_total` | Counter | `command` |
| `ada_cache_redis_command_duration_seconds` | Histogram | `command` |
| `ada_cache_redis_memory_used_bytes` | Gauge | 使用メモリ |
| `ada_cache_redis_memory_max_bytes` | Gauge | 最大メモリ |
| `ada_cache_redis_connected_clients` | Gauge | 接続クライアント数 |
| `ada_cache_redis_evicted_keys_total` | Counter | 退避キー数 |
| `ada_cache_redis_expired_keys_total` | Counter | 失効キー数 |
| `ada_cache_redis_keyspace_hits_total` | Counter | ヒット数 |
| `ada_cache_redis_keyspace_misses_total` | Counter | ミス数 |
| `ada_cache_redis_cache_hit_ratio` | Gauge | ヒット率 |

## 8. 必須ラベル

すべての metrics に以下を付与：

| ラベル | 説明 | 例 |
|---|---|---|
| `service` | crate 名 | `ada-m13-api-gateway` |
| `service.namespace` | K8s namespace | `ada` |
| `service.pod` | Pod 名 | `api-gateway-7d8f-abc` |
| `service.instance` | インスタンス ID | `ada-m13-api-gateway-7d8f-abc` |
| `service.version` | バージョン | `0.1.0` |
| `deployment.environment` | 環境 | `production` / `staging` / `dev` |
| `deployment.region` | リージョン | `us-east-1` |
| `k8s.namespace` | 名前空間 | `ada` |
| `k8s.pod.name` | Pod 名 | `api-gateway-7d8f-abc` |
| `k8s.container.name` | コンテナ名 | `api-gateway` |
| `tenant_id` | テナント ID（PII 配慮で hash） | `t_a1b2c3` |

## 9. カーディナリティ管理

### 9.1 高 Cardinality 禁止

❌ 以下を **label** に使用禁止：

| 禁止 | 理由 |
|---|---|
| `user_id` | 100 万+ ユーザーで 1M 系列 |
| `request_id` (raw) | 毎リクエストで 1 系列 |
| `email` | PII + 高 cardinality |
| `full_url` | URL パスパラメータで無限 |
| `error_message` (raw) | 文字列で無限 |
| `stack_trace` (raw) | 文字列で無限 |

✅ 許可パターン：

| パターン | 例 |
|---|---|
| hash 化 | `tenant_id_hash` |
| 固定値のバケット | `latency_bucket` (0.1, 0.5, 1, 5s) |
| enum 値 | `error_code` (AUTH_FAILED, NOT_FOUND 等) |
| 短い ID | `canvas_id` (UUID v7) |

### 9.2 Cardinality 監視

```promql
# ラベル組み合わせ数の監視
count(
  count by (service, endpoint, method, status) (
    ada_app_api_gateway_requests_total
  )
)
```

警告閾値：> 10,000 シリーズ/メトリクス

## 10. 業務別 metrics 一覧

| 業務 | 主要 metrics | 関連 NF |
|---|---|---|
| 認証/認可 | `ada_app_rbac_authentication_total` | [NF-SEC-03](../requirements/05-nfr-non-functional-requirements.md) |
| データ取得 | `ada_app_data_flow_engine_executions_total` | [NF-PER-02](../requirements/05-nfr-non-functional-requirements.md) |
| データ標準化 | `ada_app_m02_normalizer_*`（将来） | [NF-PER] |
| キャンバス描画 | `ada_app_canvas_editor_fps` | [NF-PER-01](../requirements/05-nfr-non-functional-requirements.md) |
| 共同編集 | `ada_app_rbac_crdt_*` | [NF-PER-09](../requirements/05-nfr-non-functional-requirements.md) |
| プラグイン実行 | `ada_app_module_registry_wasm_*` | [NF-OPS] |
| イベント配信 | `ada_app_event_bus_*` | [NF-PER-04](../requirements/05-nfr-non-functional-requirements.md) |
| Backup/Restore | `ada_infra_node_disk_*` | [NF-AVA-07](../requirements/05-nfr-non-functional-requirements.md) |
| DB アクセス | `ada_app_tenant_middleware_db_*` | [NF-PER] |
| マルチリージョン | `ada_db_postgres_replication_lag_seconds` | [NF-AVA] |

## 11. 用語集

| 用語 | 説明 |
|---|---|
| Cardinality | ラベルの組み合わせ数 |
| Counter | 単調増加する値（リクエスト数、エラー数） |
| Gauge | 増減する値（CPU 使用率、接続数） |
| Histogram | 分布（レイテンシ、サイズ） |
| Quantile | 分位数（p50, p95, p99） |
| USE | Utilization / Saturation / Errors |
| RED | Rate / Errors / Duration |
| Exemplar | Metric に Trace を紐付ける |
| Resource | 計装データの属性セット |
| Label | メトリクスの属性 |

## 12. 参考文献

1. Prometheus Best Practices on Naming: https://prometheus.io/docs/practices/naming/
2. USE Method: Brendan Gregg, http://www.brendangregg.com/usemethod.html
3. RED Method: Tom Wilkie, https://www.weave.works/blog/the-red-method-key-metrics-for-microservices-architecture/
4. postgres_exporter ドキュメント
5. OpenTelemetry Metrics Semantic Conventions
6. Ada プロジェクトチーム「[DOC-REQ-NFR-001 NFR](../requirements/05-nfr-non-functional-requirements.md)」

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
