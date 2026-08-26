-- =============================================================================
-- V001__init_schema.sql
--
-- Ada 永続層 初期スキーマ (PostgreSQL 18.6 想定)
--
-- 設計依据:
--   - docs/modules/M-10-tenant-middleware.md §4.2 (tenant 主表 - FK ターゲット)
--   - docs/modules/M-10-tenant-middleware.md §4.3 (M-14 module_registry 系統)
--   - docs/modules/M-10-tenant-middleware.md §4.4 (M-15 event_log 系統)
--   - docs/modules/M-10-tenant-middleware.md §4.5 (M-16 cluster_node 系統)
--   - docs/architecture/04-atomic-deployment.md  §10.2 / §11
--
-- 含むオブジェクト:
--   - 11 テーブル (tenant / module_registry / module_upgrade_history /
--     module_instance / event_log / event_topic / event_subscription /
--     consumer_offset / cluster_node / leader_lease / shard_assignment)
--   -  1 シーケンス (event_seq_global - 6 本存过の append_event() が使用)
--
-- 命名注意:
--   - task spec の "tenants / modules / events / leases / cluster_nodes ..."
--     命名は §4.3-§4.5 実装命名 (単数形) と異なる。docs source of truth に従い
--     M-10 §4.3-§4.5 単数形を採用。task spec の 11 張という「数」だけ合致。
--   - RLS セッション変数名は M-10 §2.1 / §3.1 統一: `app.current_tenant`。
--     加えて task spec の 6 本存过は `app.current_user_id` / `app.current_service`
--     も参照する (§4.6 参照)。
--
-- 冪等性:
--   - CREATE TABLE IF NOT EXISTS / CREATE SEQUENCE IF NOT EXISTS / DO ブロックで
--     CREATE POLICY / CREATE INDEX の冪等性を担保。
--   - 本ファイルは BEGIN; ... COMMIT; 包裹なし - 各文が冪等なので再実行可能。
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- §4.2 tenant (FK ターゲットとして最小実装)
-- -----------------------------------------------------------------------------
-- §4.2 詳細 DDL は M-10 §4.2 を参照。本ファイルでは他 §4.3-§4.5 のテーブルが
-- REFERENCES する相手として最小列のみ用意 (id, name, created_at)。
-- 状態列 / RLS ポリシーは §4.2 完整 DDL 导入時にマージする。
CREATE TABLE IF NOT EXISTS tenant (
    id          UUID         PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- -----------------------------------------------------------------------------
-- §4.3 module_registry 系統 (M-14 配套)
-- -----------------------------------------------------------------------------
-- §4.3 module_registry:
--   - 1 module_id に対し複数 version 登録可
--   - 同 (tenant_id, module_id) 内で active=TRUE は 1 つのみ (部分 UNIQUE)
--   - state は string + CHECK で enum 制約 (PG ENUM 型は ALTER 不可なため)
CREATE TABLE IF NOT EXISTS module_registry (
    id                UUID         PRIMARY KEY,
    tenant_id         UUID         NOT NULL REFERENCES tenant(id),
    module_id         VARCHAR(100) NOT NULL,           -- 例: 'm01-acquisition'
    version           VARCHAR(50)  NOT NULL,           -- SemVer 例: '1.5.0'
    manifest          JSONB        NOT NULL,           -- Module.toml 全文
    artifact_url      TEXT,                            -- s3://bucket/path
    artifact_sha256   VARCHAR(64),                     -- 64 hex chars
    state             VARCHAR(30)  NOT NULL DEFAULT 'Registered',
    active            BOOLEAN      NOT NULL DEFAULT FALSE,
    retired_at        TIMESTAMPTZ,
    activated_at      TIMESTAMPTZ,
    registered_at     TIMESTAMPTZ  NOT NULL DEFAULT now(),
    registered_by     UUID,                            -- current_setting('app.current_user_id', true)::UUID
    CONSTRAINT module_registry_state_chk CHECK (state IN (
        'Registered', 'Downloading', 'Loaded', 'Active', 'Draining',
        'Drained', 'Unloading', 'Unloaded', 'Failed', 'Rejected'
    )),
    CONSTRAINT module_registry_unique UNIQUE (tenant_id, module_id, version)
);

-- 同 (tenant_id, module_id) で active=TRUE は 1 つのみ (部分 UNIQUE INDEX)
CREATE UNIQUE INDEX IF NOT EXISTS module_registry_one_active_per_module
    ON module_registry (tenant_id, module_id)
    WHERE active = TRUE;

-- §4.3 module_upgrade_history:
--   - 升级编排の監査ログ
--   - 戦略 (rolling/blue-green/canary/recreate/atomic_swap) + 状態
CREATE TABLE IF NOT EXISTS module_upgrade_history (
    id                UUID         PRIMARY KEY,
    tenant_id         UUID         NOT NULL REFERENCES tenant(id),
    module_id         VARCHAR(100) NOT NULL,
    from_version      VARCHAR(50),
    to_version        VARCHAR(50)  NOT NULL,
    strategy          VARCHAR(20)  NOT NULL,           -- 'rolling'|'blue-green'|'canary'|'recreate'|'atomic_swap'
    plan_id           UUID         NOT NULL,
    status            VARCHAR(20)  NOT NULL,           -- 'Pending'|'InProgress'|'Succeeded'|'Failed'|'Aborted'
    started_at        TIMESTAMPTZ,
    completed_at      TIMESTAMPTZ,
    total_nodes       INT,
    completed_nodes   INT          DEFAULT 0,
    failed_nodes      INT          DEFAULT 0,
    rolled_back       BOOLEAN      DEFAULT FALSE,
    error_message     TEXT,
    CONSTRAINT module_upgrade_history_strategy_chk CHECK (strategy IN (
        'rolling', 'blue-green', 'canary', 'recreate', 'atomic_swap'
    )),
    CONSTRAINT module_upgrade_history_status_chk CHECK (status IN (
        'Pending', 'InProgress', 'Succeeded', 'Failed', 'Aborted'
    ))
);

-- §4.3 module_instance:
--   - 各 node_id に deploy された module instance
--   - 1 node_id 上で同 (module_id, version) は 1 つのみ
CREATE TABLE IF NOT EXISTS module_instance (
    id                UUID         PRIMARY KEY,
    tenant_id         UUID         NOT NULL REFERENCES tenant(id),
    node_id           UUID         NOT NULL,           -- cluster_node.node_id (後述)
    module_id         VARCHAR(100) NOT NULL,
    version           VARCHAR(50)  NOT NULL,
    state             VARCHAR(20)  NOT NULL DEFAULT 'Loading',
    state_changed_at  TIMESTAMPTZ  NOT NULL DEFAULT now(),
    resource_usage    JSONB,                           -- {cpu, memory, ...}
    last_health_at    TIMESTAMPTZ,
    CONSTRAINT module_instance_state_chk CHECK (state IN (
        'Loading', 'Loaded', 'Active', 'Draining', 'Drained',
        'Unloading', 'Terminated', 'Failed'
    )),
    CONSTRAINT module_instance_unique UNIQUE (node_id, module_id, version),
    -- 自己参照整合: (tenant_id, module_id, version) は module_registry に存在
    -- cluster_node への FK は §4.5 テーブル作成後に追加 (下記)
    FOREIGN KEY (tenant_id, module_id, version)
        REFERENCES module_registry(tenant_id, module_id, version)
);

-- §4.3 用ルックアップ INDEX
CREATE INDEX IF NOT EXISTS idx_module_registry_lookup
    ON module_registry (tenant_id, module_id, version);
CREATE INDEX IF NOT EXISTS idx_module_upgrade_history_module
    ON module_upgrade_history (tenant_id, module_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_module_instance_node
    ON module_instance (node_id, state);

-- §4.3 RLS: tenant_id ベース
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'module_registry'
          AND policyname = 'module_registry_rls'
    ) THEN
        EXECUTE $POL$
            ALTER TABLE module_registry ENABLE ROW LEVEL SECURITY;
            CREATE POLICY module_registry_rls ON module_registry
              FOR ALL
              USING (tenant_id = current_setting('app.current_tenant', true)::uuid)
              WITH CHECK (tenant_id = current_setting('app.current_tenant', true)::uuid);
        $POL$;
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'module_upgrade_history'
          AND policyname = 'module_upgrade_history_rls'
    ) THEN
        EXECUTE $POL$
            ALTER TABLE module_upgrade_history ENABLE ROW LEVEL SECURITY;
            CREATE POLICY module_upgrade_history_rls ON module_upgrade_history
              FOR ALL
              USING (tenant_id = current_setting('app.current_tenant', true)::uuid)
              WITH CHECK (tenant_id = current_setting('app.current_tenant', true)::uuid);
        $POL$;
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'module_instance'
          AND policyname = 'module_instance_rls'
    ) THEN
        EXECUTE $POL$
            ALTER TABLE module_instance ENABLE ROW LEVEL SECURITY;
            CREATE POLICY module_instance_rls ON module_instance
              FOR ALL
              USING (tenant_id = current_setting('app.current_tenant', true)::uuid)
              WITH CHECK (tenant_id = current_setting('app.current_tenant', true)::uuid);
        $POL$;
    END IF;
END
$$;

-- -----------------------------------------------------------------------------
-- §4.4 event_log 系統 (M-15 配套)
-- -----------------------------------------------------------------------------
-- §4.4 event_topic: 静的な topic メタ (PERSISTENT retention_days 等)
CREATE TABLE IF NOT EXISTS event_topic (
    topic           VARCHAR(200) PRIMARY KEY,
    category        VARCHAR(50)  NOT NULL,             -- 'system'|'business'|'audit'|'data'
    retention_days  INT          NOT NULL DEFAULT 30,
    description     TEXT,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT now(),
    CONSTRAINT event_topic_category_chk CHECK (category IN (
        'system', 'business', 'audit', 'data'
    ))
);

-- §4.4 event_subscription: pub/sub の購読定義
CREATE TABLE IF NOT EXISTS event_subscription (
    id              UUID         PRIMARY KEY,
    topic_pattern   VARCHAR(200) NOT NULL,             -- 例: 'module.*' / 'cluster.#'
    group_id        VARCHAR(100) NOT NULL,             -- コンシューマグループ
    delivery_mode   VARCHAR(20)  NOT NULL,             -- 'durable'|'ephemeral'
    filter          JSONB        NOT NULL DEFAULT '{}'::jsonb,
    from_position   JSONB        NOT NULL,             -- 'earliest'|'latest'|{event_seq:N}
    enabled         BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT now(),
    CONSTRAINT event_subscription_delivery_chk CHECK (delivery_mode IN (
        'durable', 'ephemeral'
    )),
    CONSTRAINT event_subscription_unique UNIQUE (topic_pattern, group_id)
);

-- §4.4 event_log: 永続イベントログ
--   - event_seq は nextval('event_seq_global') で取得 (BIGINT, 大局単調増加)
--   - tenant_id NULL 可 (system-level イベント用)
CREATE TABLE IF NOT EXISTS event_log (
    id           UUID         PRIMARY KEY,
    event_seq    BIGINT       NOT NULL,                -- 大局単調増加 SEQUENCE
    topic        VARCHAR(200) NOT NULL,
    tenant_id    UUID,                                 -- NULL = system event
    payload      JSONB        NOT NULL,
    headers      JSONB        NOT NULL DEFAULT '{}'::jsonb,
    produced_at  TIMESTAMPTZ  NOT NULL DEFAULT now(),
    producer     VARCHAR(100),
    CONSTRAINT event_log_seq_unique UNIQUE (event_seq)
);

-- §4.4 consumer_offset: コンシューマの ACK 位置
CREATE TABLE IF NOT EXISTS consumer_offset (
    subscription_id      UUID         NOT NULL REFERENCES event_subscription(id),
    topic                VARCHAR(200) NOT NULL,
    consumer_id          VARCHAR(200) NOT NULL,        -- 'group_id:instance_id'
    last_acked_event_seq BIGINT       NOT NULL DEFAULT 0,
    updated_at           TIMESTAMPTZ  NOT NULL DEFAULT now(),
    PRIMARY KEY (subscription_id, topic, consumer_id)
);

-- §4.4 用 INDEX
CREATE INDEX IF NOT EXISTS idx_event_log_topic_time
    ON event_log (topic, produced_at DESC);
CREATE INDEX IF NOT EXISTS idx_event_log_tenant_time
    ON event_log (tenant_id, produced_at DESC);
CREATE INDEX IF NOT EXISTS idx_event_log_seq
    ON event_log (event_seq);

-- §4.4 RLS: event_log のみ RLS (subscription/offset/table は RLS 不要)
--   - tenant_id NULL = system event (全テナント可視)
--   - tenant_id != NULL = 該当テナントのみ
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'event_log'
          AND policyname = 'event_log_rls'
    ) THEN
        EXECUTE $POL$
            ALTER TABLE event_log ENABLE ROW LEVEL SECURITY;
            CREATE POLICY event_log_rls ON event_log
              FOR ALL
              USING (tenant_id IS NULL
                  OR tenant_id = current_setting('app.current_tenant', true)::uuid)
              WITH CHECK (tenant_id IS NULL
                  OR tenant_id = current_setting('app.current_tenant', true)::uuid);
        $POL$;
    END IF;
END
$$;

-- §4.4 event_seq_global SEQUENCE (append_event() が nextval で使用)
CREATE SEQUENCE IF NOT EXISTS event_seq_global
    START 1
    INCREMENT 1
    CACHE 100;

-- -----------------------------------------------------------------------------
-- §4.5 cluster_node 系統 (M-16 配套)
-- -----------------------------------------------------------------------------
-- §4.5 cluster_node: クラスタノード台帳
--   - tenant_id NULL = system-level node
--   - labels = {zone, role, ...} 動的タグ
--   - state: 'Registering' | 'Active' | 'Unhealthy' | 'Draining' | 'Removed'
CREATE TABLE IF NOT EXISTS cluster_node (
    node_id           UUID         PRIMARY KEY,
    tenant_id         UUID,                            -- NULL = system-level
    hostname          VARCHAR(255) NOT NULL,
    advertised_addr   VARCHAR(255) NOT NULL,           -- '10.0.1.5:8000'
    labels            JSONB        NOT NULL DEFAULT '{}'::jsonb,
    state             VARCHAR(20)  NOT NULL DEFAULT 'Registering',
    capacity          INT          NOT NULL DEFAULT 100,
    last_heartbeat_at TIMESTAMPTZ,
    status            JSONB,                           -- 最新 health/resource JSON
    current_load      NUMERIC(5, 2),                   -- 0.00 ~ 1.00
    runtime_version   VARCHAR(50),
    started_at        TIMESTAMPTZ  NOT NULL DEFAULT now(),
    CONSTRAINT cluster_node_state_chk CHECK (state IN (
        'Registering', 'Active', 'Unhealthy', 'Draining', 'Removed'
    ))
    -- current_load の上限 1.0 は M-10 §4.5 原文にも CHECK も無いため
    -- アプリ層 (register_node_heartbeat) に委ねる。負値だけ DB レベルで弾く。
    -- (負値は明らかにバグ。>1.0 は overloaded 状態として運用側で警告可能)
);

-- §4.5 leader_lease: 領導選挙 / Singleton 役割の排他制御
CREATE TABLE IF NOT EXISTS leader_lease (
    lease_key        VARCHAR(200) PRIMARY KEY,          -- 'm04-orchestrator-singleton'
    holder_node_id   UUID         NOT NULL REFERENCES cluster_node(node_id),
    acquired_at      TIMESTAMPTZ  NOT NULL DEFAULT now(),
    expires_at       TIMESTAMPTZ  NOT NULL,
    renew_count      INT          NOT NULL DEFAULT 0,
    metadata         JSONB        NOT NULL DEFAULT '{}'::jsonb
);

-- §4.5 shard_assignment: 状態分片マッピング (tenant_id → node_id)
CREATE TABLE IF NOT EXISTS shard_assignment (
    shard_id     INT          NOT NULL,
    tenant_id    UUID         NOT NULL REFERENCES tenant(id),
    node_id      UUID         NOT NULL REFERENCES cluster_node(node_id),
    assigned_at  TIMESTAMPTZ  NOT NULL DEFAULT now(),
    PRIMARY KEY (shard_id, tenant_id)
);

-- §4.5 用 INDEX
CREATE INDEX IF NOT EXISTS idx_leader_lease_expires
    ON leader_lease (expires_at);
CREATE INDEX IF NOT EXISTS idx_cluster_node_state
    ON cluster_node (state, last_heartbeat_at DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_shard_assignment_node
    ON shard_assignment (node_id);

-- §4.5 RLS: cluster_node のみ (leader_lease / shard_assignment は system-level)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'cluster_node'
          AND policyname = 'cluster_node_rls'
    ) THEN
        EXECUTE $POL$
            ALTER TABLE cluster_node ENABLE ROW LEVEL SECURITY;
            CREATE POLICY cluster_node_rls ON cluster_node
              FOR ALL
              USING (tenant_id IS NULL
                  OR tenant_id = current_setting('app.current_tenant', true)::uuid)
              WITH CHECK (tenant_id IS NULL
                  OR tenant_id = current_setting('app.current_tenant', true)::uuid);
        $POL$;
    END IF;
END
$$;

-- §4.3 遅延 FK: module_instance.node_id → cluster_node.node_id
--   (cluster_node テーブルが後で CREATE されるため、先延ばし)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'module_instance_node_fk'
    ) THEN
        ALTER TABLE module_instance
            ADD CONSTRAINT module_instance_node_fk
            FOREIGN KEY (node_id) REFERENCES cluster_node(node_id);
    END IF;
END
$$;

-- -----------------------------------------------------------------------------
-- 統計情報更新 (オプティマイザヒント)
-- -----------------------------------------------------------------------------
ANALYZE module_registry;
ANALYZE module_upgrade_history;
ANALYZE module_instance;
ANALYZE event_log;
ANALYZE event_topic;
ANALYZE event_subscription;
ANALYZE consumer_offset;
ANALYZE cluster_node;
ANALYZE leader_lease;
ANALYZE shard_assignment;
ANALYZE tenant;

COMMIT;
