# Ada DB Schema & PL/pgSQL Functions

PostgreSQL 18.6 用の DDL マイグレーション + 6 本の PL/pgSQL 存储过程 + 単体テスト。

## ディレクトリ構成

```
db/
├── README.md                                  # 本ファイル
├── Makefile                                   # make 経由のランナー
├── run-tests.sh                               # bash/zsh 用テストランナー
├── migrations/
│   ├── V001__init_schema.sql                  # 11 テーブル + event_seq_global SEQUENCE
│   └── V002__plpgsql_functions.sql            # 6 本 PL/pgSQL 存过
└── tests/
    └── V002__plpgsql_functions_test.sql       # 6 本存过の単体テスト
```

## 設計依据

- [`docs/modules/M-10-tenant-middleware.md` §4.3-§4.5](../docs/modules/M-10-tenant-middleware.md)
  - 11 テーブル (`tenant` + `module_registry` 系統 + `event_log` 系統 + `cluster_node` 系統)
- [`docs/modules/M-10-tenant-middleware.md` §4.6](../docs/modules/M-10-tenant-middleware.md)
  - 6 本 PL/pgSQL 存过 (`register_module` / `atomic_module_swap` / `append_event` /
    `acquire_lease` / `release_lease` / `register_node_heartbeat`)
- [`docs/architecture/04-atomic-deployment.md` §10.2 / §11](../docs/architecture/04-atomic-deployment.md)
  - 4 大能力 (原子化部署 / 中心事件 / 集群协调 / 热插拔) と PL/pgSQL 存过方針

## 含まれるオブジェクト

### 11 テーブル (V001)

| 名前 | 用途 | 出典 |
|---|---|---|
| `tenant` | マルチテナント主表 (FK ターゲット) | M-10 §4.2 (最小) |
| `module_registry` | モジュール登録 (version 含む) | M-10 §4.3 |
| `module_upgrade_history` | 升级履歴 | M-10 §4.3 |
| `module_instance` | node × module 配置 | M-10 §4.3 |
| `event_log` | イベント永続ログ | M-10 §4.4 |
| `event_topic` | topic メタ (retention_days 等) | M-10 §4.4 |
| `event_subscription` | pub/sub 購読定義 | M-10 §4.4 |
| `consumer_offset` | コンシューマ ACK 位置 | M-10 §4.4 |
| `cluster_node` | クラスタノード台帳 | M-10 §4.5 |
| `leader_lease` | 領導租約 | M-10 §4.5 |
| `shard_assignment` | 状態分片マッピング | M-10 §4.5 |

### 1 シーケンス

| 名前 | 用途 |
|---|---|
| `event_seq_global` | `append_event()` が `nextval` で取得する大局単調増加 seq |

### 6 本 PL/pgSQL 存过 (V002)

| 存过 | シグネチャ | 用途 |
|---|---|---|
| `register_module` | `(p_module_id, p_version, p_manifest, p_artifact_url, p_artifact_sha256) → TABLE(success, module_instance_id, error_msg)` | モジュール登録 (幂等) |
| `atomic_module_swap` | `(p_module_id, p_from_version, p_to_version) → TABLE(success, error_msg)` | 原子的切替 (advisory_lock) |
| `append_event` | `(p_topic, p_payload) → TABLE(event_id, event_seq)` | イベント追記 (pg_notify) |
| `acquire_lease` | `(p_lease_key, p_node_id, p_ttl_seconds DEFAULT 30) → TABLE(acquired, lease_id, expires_at)` | 領導租約取得 |
| `release_lease` | `(p_lease_key, p_node_id) → TABLE(released)` | 領導租約解放 (保持者のみ) |
| `register_node_heartbeat` | `(p_node_id, p_status JSONB) → TABLE(healthy, current_load)` | ノード心跳 upsert |

> **命名注意 (task spec との差分)**: タスク仕様では `tenants` / `modules` / `events` /
> `leases` / `cluster_nodes` のように複数形 + 別名が記載されていましたが、本実装は
> 設計文档 (source of truth) の命名 (`tenant` / `module_registry` / `event_log` /
> `leader_lease` / `cluster_node`) に従っています。「11 張」という**数**は一致。

> **シグネチャ注意**: タスク仕様では `register_module(... p_kind, p_endpoint)` /
> `acquire_lease(... p_owner)` / `release_lease(... p_owner)` /
> `register_node_heartbeat(... p_endpoint, p_load)` と記載されていましたが、
> M-10 §4.6 詳細シグネチャ (`p_artifact_url` / `p_node_id` / `p_status JSONB` 等)
> に従っています。

## RLS セッション変数

| 変数 | 用途 | 設定者 |
|---|---|---|
| `app.current_tenant` | RLS フィルタ (UUID) | アプリ層トランザクション開始時 `set_config(..., true)` |
| `app.current_user_id` | `module_registry.registered_by` 等 | アプリ層 (任意) |
| `app.current_service` | `event_log.producer` | アプリ層 (任意) |

詳細: [`docs/modules/M-10-tenant-middleware.md` §3.1](../docs/modules/M-10-tenant-middleware.md)
「`with_tenant_scope` 関数」

## 使い方

### 前提

- PostgreSQL 18.6 以降 (`gen_random_uuid()` / `JSONB` / `pg_notify` 標準装備)
- `psql` クライアントが PATH に存在
- 接続ユーザーが以下の権限を持つこと:
  - `CREATE` / `DROP` (テーブル / ポリシー / シーケンス)
  - `USAGE` on `pg_catalog`

### マイグレーション適用

```bash
# 環境変数で接続先指定
export PGHOST=localhost
export PGPORT=5432
export PGUSER=ada
export PGPASSWORD=ada
export PGDATABASE=ada_dev

# マイグレーション実行
psql -v ON_ERROR_STOP=1 -f db/migrations/V001__init_schema.sql
psql -v ON_ERROR_STOP=1 -f db/migrations/V002__plpgsql_functions.sql
```

### テスト実行

```bash
# Makefile 経由
make -C db test
# または
make -C db test DB=ada_test

# 直接 bash / zsh
DB=ada_test ./db/run-tests.sh
```

`run-tests.sh` / `make test` の挙動:

1. `db/migrations/V001__init_schema.sql` を適用
2. `db/migrations/V002__plpgsql_functions.sql` を適用
3. `db/tests/V002__plpgsql_functions_test.sql` を実行
4. テストファイルは `BEGIN; ... ROLLBACK;` で全テストデータを巻き戻し、
   スキーマは残ります。再実行可能です。

### 手動検証 (psql)

```bash
psql -d ada_dev
```

```sql
-- 6 本存过の確認
\df register_module
\df atomic_module_swap
\df append_event
\df acquire_lease
\df release_lease
\df register_node_heartbeat

-- 11 テーブルの確認
\dt

-- event_seq_global SEQUENCE
\df event_seq_global  -- ※ SEQUENCE は \d で確認
SELECT * FROM pg_sequences WHERE sequencename = 'event_seq_global';
```

## 検証ステータス

| 項目 | 状態 |
|---|---|
| `cargo check --workspace` (5 門) | ✅ 既存テスト不変 (本タスクは SQL のみで Rust コード無改変) |
| `psql` 実機実行 | ⚠️ ホストに psql 未導入のため未実機 |
| 静的構文チェック (LSP / sqlfluff) | ⚠️ 未適用 (本タスク範囲外) |
| CI 統合 | TODO: 別タスクで `.github/workflows/db-test.yml` 追加予定 |

## 改版履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v0.1.0 | 2026-08-27 | 初版 (V001 + V002 + テスト 6 本) — worker (Mavis 接手 agent per DEC-008) |
