# Ada DB Schema & PL/pgSQL Functions

PostgreSQL 用の DDL マイグレーション (3 本) + 13 テーブル + 8 本の PL/pgSQL 存储过程 + 単体テスト (3 本)。

## ディレクトリ構成

```
db/
├── README.md                                  # 本ファイル
├── Makefile                                   # make 経由のランナー
├── run-tests.sh                               # bash/zsh 用テストランナー
├── migrations/
│   ├── V001__init_schema.sql                  # 11 テーブル + event_seq_global SEQUENCE
│   ├── V002__plpgsql_functions.sql            # 6 本 PL/pgSQL 存过
│   └── V003__phase8_remediation.sql           # 2 テーブル + 2 本 PL/pgSQL 存过 (remediation)
└── tests/
    ├── V001__init_schema_test.sql             # 31 PASS notices
    ├── V002__plpgsql_functions_test.sql       # 6 本存过の単体テスト (15 PASS notices)
    └── V003__phase8_remediation_test.sql      # remediation の単体テスト (8 PASS notices)
```

`run-tests.sh` は `migrations/V*.sql` と `tests/V*.sql` を glob で拾うため、ファイルを
追加してもこっちの記載だけ直す必要はありません。個数だけを、ここと
`crates/ada-core/tests/db_docs.rs` が実際の SQL から導出して照合します。

<!-- gate:inventory -->
migrations: 3
tables: 13
functions: 8
tests: 3
<!-- end -->

このブロックが機械可読な正です。上の見出し (`### <n> テーブル (V001)`) と合わせて、
`crates/ada-core/tests/db_docs.rs` が SQL と突き合わせます。V004 を足したときにこの
ブロックと対応する見出しを忘了しても、CI は赤くなります。

## 設計依据

- [`docs/modules/M-10-tenant-middleware.md` §4.3-§4.5](../docs/modules/M-10-tenant-middleware.md)
  - 11 テーブル (`tenant` + `module_registry` 系統 + `event_log` 系統 + `cluster_node` 系統)
- [`docs/modules/M-10-tenant-middleware.md` §4.6](../docs/modules/M-10-tenant-middleware.md)
  - 6 本 PL/pgSQL 存过 (`register_module` / `atomic_module_swap` / `append_event` /
    `acquire_lease` / `release_lease` / `register_node_heartbeat`)
- [`docs/architecture/04-atomic-deployment.md` §10.2 / §11](../docs/architecture/04-atomic-deployment.md)
  - 4 大能力 (原子化部署 / 中心事件 / 集群协调 / 热插拔) と PL/pgSQL 存过方針
- `config/remediation/*.json` + `crates/ada-remediation/`
  - 2 テーブル (`remediation_history` / `remediation_cooldowns`) と
    2 本 PL/pgSQL 存过 (`remediation_record_execution` / `remediation_check_cooldown`)。
    実行履歴と cooldown を DB に置き、`crates/ada-core/tests/runbook_triggers.rs` が
    runbook 側の対象表と整合しているかを見る。

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

### 2 テーブル (V003)

| 名前 | 用途 | 出典 |
|---|---|---|
| `remediation_history` | remediation 実行履歴 (action / alert / 実行者) | `V003__phase8_remediation.sql` |
| `remediation_cooldowns` | 同一 action の連発抑止 (cooldown) | `V003__phase8_remediation.sql` |

合計は **13 テーブル**。`make -C db clean` / `drop` の対象表リストも 13 張を列挙します。

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

### 2 本 PL/pgSQL 存过 (V003)

| 存过 | 用途 |
|---|---|
| `remediation_record_execution` | remediation 実行結果の記録 (`remediation_history` への insert) |
| `remediation_check_cooldown` | 直近実行から cooldown 経過したかの判定 (`remediation_cooldowns` 参照) |

合計は **8 本**。

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
psql -v ON_ERROR_STOP=1 -f db/migrations/V003__phase8_remediation.sql
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

1. `db/migrations/` の `V*.sql` を順に適用 (现在是 3 本)
2. `db/tests/` の `V*.sql` を順に実行 (现在是 3 本)
3. テストファイルは `BEGIN; ... ROLLBACK;` で全テストデータを巻き戻し、
   スキーマは残ります。再実行可能です。

`run-tests.sh` は「`RAISE NOTICE 'PASS:` の総数が `EXPECTED_PASS` に一致すること」と
「`PASS` notice が `MIN_PASS_NOTES=54` 個以上あること」を必ず確認します。現在の内訳は
V001: 31 / V002: 15 / V003: 8 = 54。テスト本文を書き換えて PASS を 1 個削っただけで
CI が赤くなります。

### 手動検証 (psql)

```bash
psql -d ada_dev
```

```sql
-- 8 本存过の確認
\df register_module
\df atomic_module_swap
\df append_event
\df acquire_lease
\df release_lease
\df register_node_heartbeat
\df remediation_record_execution
\df remediation_check_cooldown

-- 13 テーブルの確認
\dt

-- event_seq_global SEQUENCE
\df event_seq_global  -- ※ SEQUENCE は \d で確認
SELECT * FROM pg_sequences WHERE sequencename = 'event_seq_global';
```

## 検証ステータス

| 項目 | 状態 |
|---|---|
| SQL テスト (`run-tests.sh`) | ✅ CI の `db migrations` ジョブが `postgres:16` 上で実行。適用後の `public` に 13 テーブル / 8 関数が存在することを同じジョブが anti-vacuity として確認している (_tables < 13 / _functions < 8 で失敗) |
| ワークスペースの Rust ビルド | ✅ CI。`cargo build --workspace` は `rust` ジョブ、`cargo clippy` は同ジョブと `security` ジョブで走る |
| 静的構文チェック (LSP / sqlfluff) | ⚠️ 未適用 (本タスク範囲外) |
| 手元 psql での実機確認 | ローカルではなく CI を正とする。`postgres:16` は Docker が要るので手元では回せない環境がある |

> 以前のこの表は「`psql` ホスト未導入のため未実機」「CI 統合は `.github/workflows/db-test.yml`
> 追加予定」と書かれていた。両方とも现已事实不符: 実機は CI で行われており、CI 側のジョブ名も
> `db-test.yml` ではなく `ci.yml` 内の `db-migrations` です。

## 改版履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v0.1.0 | 2026-08-27 | 初版 (V001 + V002 + テスト 6 本) — worker (Mavis 接手 agent per DEC-008) |
| v0.1.1 | 2026-10-07 | V003 (`remediation_history` / `remediation_cooldowns` + 存过 2 本) とテスト 1 本を反映。11→13 テーブル / 6→8 存过。検証ステータスの古い記述 (psql 未実機 / `db-test.yml` 追加予定) を現状に直した。個数は `crates/ada-core/tests/db_docs.rs` が SQL から導出して照合する |
