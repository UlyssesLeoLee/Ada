# 05 分散トレーシング設計（Distributed Tracing Design）

> W3C Trace Context + OpenTelemetry Span。**エンドツーエンドの遅延分析と障害伝播追跡**。Rust の in-proc 通信も含めて全て span でカバー。

> **ドキュメントID**：DOC-OBS-005
> **上位文書**：[DOC-OBS-INDEX](README.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版 |

---

## 目次

1. 設計目標
2. トレースモデル
3. Span 設計
4. サンプリング
5. 業務別トレース要件
6. WASM フロントエンド
7. プラグイン沙箱トレース
8. CRDT 共同編集トレース
9. コード規約
10. 用語集

---

## 1. 設計目標

| 目標 | 説明 |
|---|---|
| **W3C 準拠** | ベンダ中立、相互運用 |
| **100% カバレッジ** | HTTP + WS + in-proc + DB + EventBus + WASM 全て |
| **低オーバーヘッド** | < 3% 性能影響 |
| **完全相関** | trace_id で metric/log を引ける |
| **障害伝播可視化** | エラーがどこで起きたか即特定 |

## 2. トレースモデル

### 2.1 全体フロー例

```
[Client (Browser)]
  │
  │ HTTPS POST /api/v1/canvases (trace_id=ABC, span_id=001)
  ▼
[M-13 API Gateway]
  │ Span: gateway.request
  │ ├─ Span: gateway.jwt_validate (12ms)
  │ ├─ Span: gateway.tenant_resolve (3ms)
  │ └─ Span: gateway.route (1ms)
  │
  │ IPC: forward to M-03 (trace_id=ABC, parent=001)
  ▼
[M-03 Data Flow Engine]
  │ Span: m03.execute (parent=001)
  │ ├─ Span: m03.load_canvas (5ms)
  │ ├─ Span: m03.parse_nodes (2ms)
  │ └─ Span: m03.run_nodes (parent=002)  ← 親を新たに
  │     ├─ Span: node.m01.acquire (parent=003)  100ms
  │     │  └─ Span: db.query (parent=004)  80ms
  │     ├─ Span: node.m02.normalize (parent=003)  20ms
  │     └─ Span: node.m09.export (parent=003)  50ms
  │
  │ Event publish to M-15 (trace_id=ABC, parent=001)
  ▼
[M-15 Event Bus]
  │ Span: m15.publish (parent=001)
  │ Span: m15.consume (parent=001, m03=consumer)
```

### 2.2 Span 階層

```
Root Span: api.request
  ├─ gateway.auth
  ├─ gateway.tenant_resolve
  ├─ m03.execute
  │   ├─ m03.load_canvas
  │   ├─ m03.run_nodes
  │   │   ├─ m01.acquire (REST adapter)
  │   │   │   └─ db.query
  │   │   └─ m02.normalize
  │   └─ m09.export
  └─ m15.publish
```

## 3. Span 設計

### 3.1 Span 名規約

```
{service}.{operation}

例:
  gateway.request
  m03.execute
  m01.acquire
  m02.normalize
  db.query
  m15.publish
```

### 3.2 Span 属性

すべての Span に以下を付与：

| 属性 | 型 | 必須 | 例 |
|---|---|---|---|
| `service.name` | string | ✅ | `ada-m13-api-gateway` |
| `service.version` | string | ✅ | `0.1.0` |
| `deployment.environment` | string | ✅ | `production` |
| `k8s.pod.name` | string | ✅ | `api-gateway-7d8f-abc` |
| `k8s.namespace` | string | ✅ | `ada` |
| `tenant.id` | string (hash) | 条件付き | `t_a1b2c3` |
| `user.id` | string (hash) | 条件付き | `u_x9y8z7` |
| `http.method` | string | 条件付き | `POST` |
| `http.path` | string | 条件付き | `/api/v1/canvases` |
| `http.status_code` | number | 条件付き | `201` |
| `db.system` | string | 条件付き | `postgresql` |
| `db.statement` | string | 条件付き | `INSERT INTO canvas ...`（パラメータ化） |
| `db.table` | string | 条件付き | `canvas` |
| `messaging.system` | string | 条件付き | `event_bus` |
| `messaging.destination` | string | 条件付き | `canvas.created` |
| `error.kind` | string | 条件付き | `DatabaseError` |
| `error.message` | string | 条件付き | `connection refused` |
| `error.stack_trace` | string | 条件付き | （[PII 配慮] パス・型のみ） |

### 3.3 業務 Span 定義

#### 3.3.1 M-13 API Gateway

| Span 名 | 属性 | 重要度 |
|---|---|---|
| `gateway.request` | http.* | 高 |
| `gateway.jwt_validate` | result | 高 |
| `gateway.tenant_resolve` | tenant.id | 高 |
| `gateway.rbac_check` | action, resource | 高 |
| `gateway.forward` | target_service | 中 |
| `gateway.ws_session` | session.id | 中 |

#### 3.3.2 M-03 Data Flow Engine

| Span 名 | 属性 | 重要度 |
|---|---|---|
| `m03.execute` | canvas.id | 高 |
| `m03.load_canvas` | canvas.id | 中 |
| `m03.parse_nodes` | node_count | 中 |
| `m03.run_node` | node.type, node.id | 高 |
| `m03.dependency_resolve` | cycle_check | 中 |

#### 3.3.3 M-01 Acquisition

| Span 名 | 属性 | 重要度 |
|---|---|---|
| `m01.acquire` | source_type | 高 |
| `m01.http_call` | http.url, http.status | 中 |
| `m01.db_query` | db.statement | 中 |
| `m01.parse_response` | record_count | 中 |

#### 3.3.4 M-10 Tenant Middleware

| Span 名 | 属性 | 重要度 |
|---|---|---|
| `m10.db_query` | db.statement (パラメータ化) | 高 |
| `m10.rls_check` | table, result | 高 |
| `m10.tenant_resolve` | result | 中 |
| `m10.audit_log_write` | action, resource_type | 中 |

#### 3.3.5 M-15 Event Bus

| Span 名 | 属性 | 重要度 |
|---|---|---|
| `m15.publish` | topic, event_id | 高 |
| `m15.consume` | topic, consumer_group, event_id | 高 |
| `m15.deduplicate` | event_id, result | 中 |

#### 3.3.6 M-11 RBAC + CRDT

| Span 名 | 属性 | 重要度 |
|---|---|---|
| `m11.auth` | user.id, result | 高 |
| `m11.crdt_op` | op, doc.id | 中 |
| `m11.crdt_merge` | peer, conflicts | 中 |
| `m11.crdt_conflict` | doc.id, conflict_type | 中 |

#### 3.3.7 M-14 Module Registry

| Span 名 | 属性 | 重要度 |
|---|---|---|
| `m14.atomic_swap` | module_id, from_version, to_version | 高 |
| `m14.wasm_validate` | module_id, result | 中 |
| `m14.wasm_load` | module_id, size_bytes | 中 |
| `m14.wasm_execute` | module_id, fuel_used | 高 |

#### 3.3.8 M-16 Cluster Coordinator

| Span 名 | 属性 | 重要度 |
|---|---|---|
| `m16.leader_elect` | result, candidates | 中 |
| `m16.heartbeat` | peer, result | 中 |
| `m16.shard_assign` | shard_id, node_id | 中 |

## 4. サンプリング

### 4.1 戦略

| 戦略 | サンプリング率 | 用途 |
|---|---|---|
| **Head Sampling** | 10% | 通常時（コスト削減） |
| **Tail Sampling** | 100% on error | 失敗時（完全追跡） |
| **Always Sample** | 100% | 重要 API（auth, payment） |
| **Never Sample** | 0% | ヘルスチェック `/health` |

### 4.2 実装（otel-collector）

```yaml
processors:
  tail_sampling:
    decision_wait: 10s
    num_traces: 100000
    expected_new_traces_per_sec: 1000
    policies:
      - name: errors
        type: status_code
        status_code:
          status_codes: [error]
      - name: slow
        type: latency
        latency:
          threshold_ms: 1000
      - name: important
        type: string_attribute
        string_attribute:
          key: http.path
          values: [/api/v1/auth, /api/v1/payments, /api/v1/admin/*]
      - name: default
        type: probabilistic
        probabilistic:
          sampling_percentage: 10
```

### 4.3 性能影響

| シナリオ | 影響 |
|---|---|
| 通常時（10% サンプリング） | < 1% CPU |
| エラー時（100% サンプリング） | < 5% CPU |
| 重要 API | < 3% CPU |

## 5. 業務別トレース要件

| 業務 | トレース要件 | NF 区分 |
|---|---|---|
| ユーザー認証 | 100% サンプリング、auth 全 span | [NF-SEC-03](../requirements/05-nfr-non-functional-requirements.md) |
| データ取得（M-01） | 10%、エラー時 100% | [NF-PER-02](../requirements/05-nfr-non-functional-requirements.md) |
| キャンバス実行（M-03） | 10%、1s 超は 100% | [NF-PER-03](../requirements/05-nfr-non-functional-requirements.md) |
| イベント配信（M-15） | 10%、失敗時 100% | [NF-PER-04](../requirements/05-nfr-non-functional-requirements.md) |
| プラグイン実行（M-14） | 100%（容量・コスト重要） | [NF-OPS] |
| クラスタ調整（M-16） | 100%（可用性影響大） | [NF-AVA-06](../requirements/05-nfr-non-functional-requirements.md) |
| 共同編集（M-11） | 1%（高頻度のため） | [NF-PER-09](../requirements/05-nfr-non-functional-requirements.md) |
| Atomic swap（M-14） | 100% | [NF-MIG-01](../requirements/05-nfr-non-functional-requirements.md) |

## 6. WASM フロントエンド

### 6.1 ブラウザ内計装

```typescript
// M-12 Bevy WASM Canvas Editor
import { trace } from '@opentelemetry/api';
const tracer = trace.getTracer('m12-canvas-editor');

const span = tracer.startSpan('canvas.drag_node', {
  attributes: {
    'canvas.id': canvasId,
    'node.id': nodeId,
    'user.id': userIdHash,
  },
});
// ... drag operation
span.end();
```

### 6.2 WS 双方向トレース

```
[Browser M-12]                  [M-13 Gateway]
  │                                  │
  │ traceparent: 00-{trace_id}-{span_id}-01
  │ ────── WS connect ──────────→  │
  │                                  │ trace_id を context から抽出
  │ ←───── WS messages ──────────  │ span_id = parent's span
```

W3C Trace Context は WebSocket ヘッダー (`Sec-WebSocket-Protocol: traceparent`) でも伝播可能。

## 7. プラグイン沙箱トレース

### 7.1 WASM 内計装

```rust
// ada-m06-plugin-sdk: プラグインコード内
use opentelemetry::trace::Tracer;

#[plugin_instrument]
async fn execute(input: Value) -> Result<Value, Error> {
    let tracer = opentelemetry::global::tracer("plugin");
    let span = tracer.start("plugin.execute");
    // ... プラグインロジック
    span.end();
}
```

### 7.2 wasmtime 計装フック

```rust
// ada-m14-module-registry
let mut store = Store::new(&engine, state);
store.tracer(|ctx| {
    // WASM 内関数呼び出しをトレース
    Some(TracingFlag::Recording)
});
```

## 8. CRDT 共同編集トレース

### 8.1 操作トレース

```rust
// M-11 RBAC + Collab: Yrs CRDT
#[instrument(skip(payload), fields(op, doc.id, peer))]
async fn apply_crdt_op(op: CrdtOp, doc_id: &str, peer: &str) -> Result<(), Error> {
    // CRDT 操作を span 化
    info!(op = %op.kind(), doc_id, peer, "CRDT op applied");
    Ok(())
}
```

### 8.2 競合トレース

```rust
// 競合検知時に詳細 trace
warn!(
    doc_id = %doc_id,
    local_clock = %local_clock,
    remote_clock = %remote_clock,
    conflict_kind = "vector_clock",
    "CRDT conflict detected"
);
```

## 9. コード規約

### 9.1 in-proc span 伝播

```rust
use tracing::Instrument;

#[instrument(name = "m03.execute", skip(canvas_id))]
pub async fn execute(canvas_id: &str) -> Result<Output, Error> {
    // in-proc でも子 span を作成
    let result = load_canvas(canvas_id)
        .instrument(trace_span!("m03.load_canvas", canvas.id = %canvas_id))
        .await?;
    
    // M-15 への publish もトレース
    publish_event("canvas.executed", &result)
        .instrument(trace_span!("m15.publish", topic = "canvas.executed"))
        .await?;
    
    Ok(result)
}
```

### 9.2 DB トレース

```rust
// sqlx tracing feature を使用
sqlx::query("INSERT INTO canvas ...")
    .execute(&pool)
    .instrument(trace_span!("db.query", 
        db.system = "postgresql",
        db.statement = "INSERT INTO canvas",
        db.table = "canvas"
    ))
    .await?;
```

### 9.3 HTTP トレース

```rust
use axum::http::Request;
use tower_http::trace::TraceLayer;

let app = Router::new()
    .route("/api/v1/canvases", get(list_canvases))
    .layer(TraceLayer::new_for_http());  // 自動計装
```

### 9.4 WASM トレース

```rust
use opentelemetry::trace::Tracer;

#[instrument(name = "m12.drag_node", fields(canvas.id, node.id))]
pub fn handle_drag(canvas_id: &str, node_id: &str) {
    let tracer = opentelemetry::global::tracer("m12-canvas-editor");
    tracer.in_span("drag_complete", |_| {});
}
```

## 10. 用語集

| 用語 | 説明 |
|---|---|
| Span | 単一処理単位 |
| Trace | 複数 Span の集合 |
| Parent Span | 子 Span の親 |
| Context | Span 間で渡される状態 |
| W3C Trace Context | 標準 traceparent ヘッダー |
| Sampler | Trace 採取戦略 |
| Tail Sampling | 完了時にサンプリング判断 |
| Head Sampling | 開始時にサンプリング判断 |
| In-proc | 同一プロセス内 |
| Exemplar | Metric に Trace を紐付け |

## 11. 参考文献

1. W3C Trace Context: https://www.w3.org/TR/trace-context/
2. OpenTelemetry Tracing Semantic Conventions
3. Grafana Tempo: https://grafana.com/docs/tempo/
4. sqlx tracing feature
5. tower-http TraceLayer
6. wasmtime profiling
7. Ada プロジェクトチーム「[DOC-ARCH-007 Rust 選択 §10](../architecture/06-rust-tech-selection.md)」

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
