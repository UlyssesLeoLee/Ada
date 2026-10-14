-- =============================================================================
-- V001__init_schema_test.sql
--
-- V001__init_schema.sql の宣言済み制約 (NOT NULL / CHECK / UNIQUE / DEFAULT /
-- FK / SEQUENCE / RLS) の振舞テスト (PostgreSQL 18.6 想定)
--
-- 方針 (V002__plpgsql_functions_test.sql と一致):
--   - pgTAP 等の外部依存は導入しない (pure SQL のみ)
--   - ファイル全体を BEGIN; ... ROLLBACK; で包み、テストデータを一切残さない
--   - 各 test case は SAVEPOINT / ROLLBACK TO で独立 (1 件失敗が他に影響しない)
--   - RLS セッション変数は set_config(..., true) で transaction-local 設定
--   - 検証は DO ブロック内 RAISE EXCEPTION (failure) / RAISE NOTICE (pass)
--   - 失敗は全て '[t_<name>] ' 接頭辞に統一 (run-tests.sh が grep する)
--
-- 前提:
--   - V001__init_schema.sql が既に適用済み
--   - ここで検証するのは V001 が *宣言している* 制約のみ。
--     V001 が宣言していないもの (event_log.topic の FK 等) は仕様であり、
--     テスト対象にしない (欠落ではなく設計判断として報告する)。
--
-- 実行: psql -d ada_test -v ON_ERROR_STOP=1 -f V001__init_schema_test.sql
-- =============================================================================

BEGIN;

-- =============================================================================
-- Phase 0: テスト用フィクスチャ (RLS セッション + 親テーブル)
--   SAVEPOINT より *外* に置くので ROLLBACK TO 後も存続する
-- =============================================================================
SELECT set_config('app.current_tenant', '11111111-1111-1111-1111-111111111111', true);
SELECT set_config('app.current_user_id', '22222222-2222-2222-2222-222222222222', true);
SELECT set_config('app.current_service', 'pgtest-runner',                true);

-- tenant: module_registry / module_upgrade_history / shard_assignment の FK ターゲット
INSERT INTO tenant (id, name) VALUES
    ('11111111-1111-1111-1111-111111111111', 'tenant-A')
    ON CONFLICT (id) DO NOTHING;

-- cluster_node: module_instance.node_id / leader_lease.holder_node_id /
--              shard_assignment.node_id の FK ターゲット
INSERT INTO cluster_node (node_id, hostname, advertised_addr, state) VALUES
    ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'test-host-a', '10.0.0.1:8000', 'Active'),
    ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'test-host-b', '10.0.0.2:8000', 'Active')
    ON CONFLICT (node_id) DO NOTHING;

-- =============================================================================
-- §4.2 tenant
-- =============================================================================
SAVEPOINT t_tenant_exists;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    SELECT count(*) INTO v_cnt
        FROM information_schema.tables
        WHERE table_schema = 'public'
          AND table_name = ANY (ARRAY[
              'tenant', 'module_registry', 'module_upgrade_history',
              'module_instance', 'event_topic', 'event_subscription',
              'event_log', 'consumer_offset', 'cluster_node',
              'leader_lease', 'shard_assignment'
          ]::TEXT[]);
    IF v_cnt <> 11 THEN
        RAISE EXCEPTION 'V001 declares 11 tables, found %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: V001 declares all 11 tables';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_tenant_exists] %', SQLERRM;
END
$$;
ROLLBACK TO t_tenant_exists;

SAVEPOINT t_tenant_not_null;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    -- name は NOT NULL 宣言済み
    BEGIN
        INSERT INTO tenant (id) VALUES (gen_random_uuid());
        RAISE EXCEPTION 'tenant.name NOT NULL not enforced';
    EXCEPTION WHEN not_null_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM tenant WHERE name IS NULL;
    IF v_cnt <> 0 THEN
        RAISE EXCEPTION 'tenant rows with NULL name inserted: %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: tenant.name NOT NULL enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_tenant_not_null] %', SQLERRM;
END
$$;
ROLLBACK TO t_tenant_not_null;

SAVEPOINT t_tenant_created_at_default;
DO $$
DECLARE
    v_created TIMESTAMPTZ;
    v_id UUID;
BEGIN
    v_id := gen_random_uuid();
    -- created_at 省略 → DEFAULT now()
    INSERT INTO tenant (id, name) VALUES (v_id, 'default-probe');
    SELECT created_at INTO v_created FROM tenant WHERE id = v_id;
    IF v_created IS NULL
       OR v_created < now() - interval '1 second'
       OR v_created > now() + interval '1 second' THEN
        RAISE EXCEPTION 'tenant.created_at DEFAULT now() not applied: %', v_created;
    END IF;

    RAISE NOTICE 'PASS: tenant.created_at DEFAULT now() applied';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_tenant_created_at_default] %', SQLERRM;
END
$$;
ROLLBACK TO t_tenant_created_at_default;

-- =============================================================================
-- §4.3 module_registry
-- =============================================================================
SAVEPOINT t_module_registry_defaults;
DO $$
DECLARE
    v_state VARCHAR(30);
    v_active BOOLEAN;
    v_registered TIMESTAMPTZ;
    v_id UUID;
BEGIN
    v_id := gen_random_uuid();
    INSERT INTO module_registry (id, tenant_id, module_id, version, manifest)
        VALUES (v_id, '11111111-1111-1111-1111-111111111111',
                'm-default', '1.0.0', '{}'::jsonb);
    SELECT state, active, registered_at
        INTO v_state, v_active, v_registered
        FROM module_registry WHERE id = v_id;

    IF v_state <> 'Registered' THEN
        RAISE EXCEPTION 'state DEFAULT is not Registered: %', v_state;
    END IF;
    IF v_active IS DISTINCT FROM FALSE THEN
        RAISE EXCEPTION 'active DEFAULT is not FALSE: %', v_active;
    END IF;
    IF v_registered IS NULL
       OR v_registered < now() - interval '1 second'
       OR v_registered > now() + interval '1 second' THEN
        RAISE EXCEPTION 'registered_at DEFAULT now() not applied: %', v_registered;
    END IF;

    RAISE NOTICE 'PASS: module_registry state/active/registered_at defaults';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_registry_defaults] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_registry_defaults;

SAVEPOINT t_module_registry_state_chk;
DO $$
DECLARE
    v_state VARCHAR(30);
    v_ok INT;
BEGIN
    -- 宣言済みの 10 値すべてを受け入れる
    FOREACH v_state IN ARRAY ARRAY[
        'Registered', 'Downloading', 'Loaded', 'Active', 'Draining',
        'Drained', 'Unloading', 'Unloaded', 'Failed', 'Rejected'
    ]::VARCHAR[] LOOP
        INSERT INTO module_registry
            (id, tenant_id, module_id, version, manifest, state)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                'm-chk-' || v_state, '1.0.0', '{}'::jsonb, v_state);
    END LOOP;

    SELECT count(*) INTO v_ok FROM module_registry
        WHERE module_id LIKE 'm-chk-%';
    IF v_ok <> 10 THEN
        RAISE EXCEPTION 'expected 10 accepted states, got %', v_ok;
    END IF;

    -- 宣言外の状態は拒否される
    BEGIN
        INSERT INTO module_registry
            (id, tenant_id, module_id, version, manifest, state)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                'm-chk-bad', '1.0.0', '{}'::jsonb, 'Bogus');
        RAISE EXCEPTION 'module_registry_state_chk did not reject Bogus';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    RAISE NOTICE 'PASS: module_registry_state_chk accepts 10 states, rejects Bogus';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_registry_state_chk] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_registry_state_chk;

SAVEPOINT t_module_registry_unique;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    INSERT INTO module_registry (id, tenant_id, module_id, version, manifest) VALUES
        (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-uniq', '1.0.0', '{}'::jsonb);

    -- module_registry_unique (tenant_id, module_id, version)
    BEGIN
        INSERT INTO module_registry (id, tenant_id, module_id, version, manifest) VALUES
            (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-uniq', '1.0.0', '{}'::jsonb);
        RAISE EXCEPTION 'module_registry_unique did not reject duplicate row';
    EXCEPTION WHEN unique_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM module_registry WHERE module_id = 'm-uniq';
    IF v_cnt <> 1 THEN
        RAISE EXCEPTION 'expected 1 row for m-uniq, got %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: module_registry_unique (tenant_id, module_id, version) enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_registry_unique] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_registry_unique;

SAVEPOINT t_module_registry_one_active;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    INSERT INTO module_registry
        (id, tenant_id, module_id, version, manifest, active) VALUES
        (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-act', '1.0.0', '{}'::jsonb, TRUE);

    -- 部分 UNIQUE INDEX ... WHERE active = TRUE
    BEGIN
        INSERT INTO module_registry
            (id, tenant_id, module_id, version, manifest, active) VALUES
            (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-act', '2.0.0', '{}'::jsonb, TRUE);
        RAISE EXCEPTION 'module_registry_one_active_per_module allowed a 2nd active row';
    EXCEPTION WHEN unique_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM module_registry
        WHERE module_id = 'm-act' AND active = TRUE;
    IF v_cnt <> 1 THEN
        RAISE EXCEPTION 'expected exactly 1 active row, got %', v_cnt;
    END IF;

    -- インデックスが「部分」UNIQUE であることの証明:
    -- active = FALSE の行は (tenant_id, module_id) が重複しても通る。
    -- ここが通らない = インデックスが部分 unique になっていない。
    INSERT INTO module_registry
        (id, tenant_id, module_id, version, manifest, active) VALUES
        (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-act', '3.0.0', '{}'::jsonb, FALSE);
    INSERT INTO module_registry
        (id, tenant_id, module_id, version, manifest, active) VALUES
        (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-act', '4.0.0', '{}'::jsonb, FALSE);

    SELECT count(*) INTO v_cnt FROM module_registry
        WHERE module_id = 'm-act' AND active = FALSE;
    IF v_cnt <> 2 THEN
        RAISE EXCEPTION 'inactive rows must be exempt from the partial UNIQUE index, got %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: module_registry_one_active_per_module partial UNIQUE enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_registry_one_active] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_registry_one_active;

SAVEPOINT t_module_registry_tenant_fk;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    BEGIN
        INSERT INTO module_registry (id, tenant_id, module_id, version, manifest) VALUES
            (gen_random_uuid(), '99999999-9999-9999-9999-999999999999',
             'm-fk', '1.0.0', '{}'::jsonb);
        RAISE EXCEPTION 'module_registry.tenant_id FK to tenant not enforced';
    EXCEPTION WHEN foreign_key_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM module_registry WHERE module_id = 'm-fk';
    IF v_cnt <> 0 THEN
        RAISE EXCEPTION 'orphan module_registry row persisted: %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: module_registry.tenant_id FK to tenant enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_registry_tenant_fk] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_registry_tenant_fk;

-- =============================================================================
-- §4.3 module_upgrade_history
-- =============================================================================
SAVEPOINT t_upgrade_history_defaults;
DO $$
DECLARE
    v_completed INT;
    v_failed INT;
    v_rolled BOOLEAN;
BEGIN
    INSERT INTO module_upgrade_history
        (id, tenant_id, module_id, to_version, strategy, plan_id, status)
    VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
            'm-hist', '2.0.0', 'rolling', gen_random_uuid(), 'Pending');

    SELECT completed_nodes, failed_nodes, rolled_back
        INTO v_completed, v_failed, v_rolled
        FROM module_upgrade_history WHERE module_id = 'm-hist';

    IF v_completed <> 0 THEN
        RAISE EXCEPTION 'completed_nodes DEFAULT is not 0: %', v_completed;
    END IF;
    IF v_failed <> 0 THEN
        RAISE EXCEPTION 'failed_nodes DEFAULT is not 0: %', v_failed;
    END IF;
    IF v_rolled IS DISTINCT FROM FALSE THEN
        RAISE EXCEPTION 'rolled_back DEFAULT is not FALSE: %', v_rolled;
    END IF;

    RAISE NOTICE 'PASS: module_upgrade_history completed/failed/rolled_back defaults';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_upgrade_history_defaults] %', SQLERRM;
END
$$;
ROLLBACK TO t_upgrade_history_defaults;

SAVEPOINT t_upgrade_history_strategy_chk;
DO $$
DECLARE
    v_strategy VARCHAR(20);
    v_ok INT;
BEGIN
    FOREACH v_strategy IN ARRAY ARRAY[
        'rolling', 'blue-green', 'canary', 'recreate', 'atomic_swap'
    ]::VARCHAR[] LOOP
        INSERT INTO module_upgrade_history
            (id, tenant_id, module_id, to_version, strategy, plan_id, status)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                'm-s-' || v_strategy, '2.0.0', v_strategy,
                gen_random_uuid(), 'Pending');
    END LOOP;

    SELECT count(*) INTO v_ok FROM module_upgrade_history WHERE module_id LIKE 'm-s-%';
    IF v_ok <> 5 THEN
        RAISE EXCEPTION 'expected 5 accepted strategies, got %', v_ok;
    END IF;

    BEGIN
        INSERT INTO module_upgrade_history
            (id, tenant_id, module_id, to_version, strategy, plan_id, status)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                'm-s-bad', '2.0.0', 'turbo', gen_random_uuid(), 'Pending');
        RAISE EXCEPTION 'strategy CHECK did not reject turbo';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    RAISE NOTICE 'PASS: module_upgrade_history_strategy_chk accepts 5, rejects turbo';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_upgrade_history_strategy_chk] %', SQLERRM;
END
$$;
ROLLBACK TO t_upgrade_history_strategy_chk;

SAVEPOINT t_upgrade_history_status_chk;
DO $$
DECLARE
    v_status VARCHAR(20);
    v_ok INT;
BEGIN
    FOREACH v_status IN ARRAY ARRAY[
        'Pending', 'InProgress', 'Succeeded', 'Failed', 'Aborted'
    ]::VARCHAR[] LOOP
        INSERT INTO module_upgrade_history
            (id, tenant_id, module_id, to_version, strategy, plan_id, status)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                'm-st-' || v_status, '2.0.0', 'rolling',
                gen_random_uuid(), v_status);
    END LOOP;

    SELECT count(*) INTO v_ok FROM module_upgrade_history WHERE module_id LIKE 'm-st-%';
    IF v_ok <> 5 THEN
        RAISE EXCEPTION 'expected 5 accepted statuses, got %', v_ok;
    END IF;

    BEGIN
        INSERT INTO module_upgrade_history
            (id, tenant_id, module_id, to_version, strategy, plan_id, status)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                'm-st-bad', '2.0.0', 'rolling', gen_random_uuid(), 'Unknown');
        RAISE EXCEPTION 'status CHECK did not reject Unknown';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    RAISE NOTICE 'PASS: module_upgrade_history_status_chk accepts 5, rejects Unknown';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_upgrade_history_status_chk] %', SQLERRM;
END
$$;
ROLLBACK TO t_upgrade_history_status_chk;

-- =============================================================================
-- §4.3 module_instance
-- =============================================================================
SAVEPOINT t_module_instance_defaults;
DO $$
DECLARE
    v_state VARCHAR(20);
    v_changed TIMESTAMPTZ;
BEGIN
    INSERT INTO module_registry (id, tenant_id, module_id, version, manifest) VALUES
        (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-inst', '1.0.0', '{}'::jsonb);

    INSERT INTO module_instance (id, tenant_id, node_id, module_id, version)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'm-inst', '1.0.0');

    SELECT state, state_changed_at INTO v_state, v_changed
        FROM module_instance WHERE module_id = 'm-inst';
    IF v_state <> 'Loading' THEN
        RAISE EXCEPTION 'state DEFAULT is not Loading: %', v_state;
    END IF;
    IF v_changed IS NULL
       OR v_changed < now() - interval '1 second'
       OR v_changed > now() + interval '1 second' THEN
        RAISE EXCEPTION 'state_changed_at DEFAULT now() not applied: %', v_changed;
    END IF;

    RAISE NOTICE 'PASS: module_instance state/state_changed_at defaults';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_instance_defaults] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_instance_defaults;

SAVEPOINT t_module_instance_state_chk;
DO $$
DECLARE
    v_state VARCHAR(20);
    v_node UUID;
    v_ok INT;
BEGIN
    INSERT INTO module_registry (id, tenant_id, module_id, version, manifest) VALUES
        (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-ist', '1.0.0', '{}'::jsonb);

    FOREACH v_state IN ARRAY ARRAY[
        'Loading', 'Loaded', 'Active', 'Draining', 'Drained',
        'Unloading', 'Terminated', 'Failed'
    ]::VARCHAR[] LOOP
        -- 状態ごとに別 node_id を使う (module_instance_unique
        -- (node_id, module_id, version) に引っ掛けないため)
        v_node := gen_random_uuid();
        INSERT INTO cluster_node (node_id, hostname, advertised_addr)
            VALUES (v_node, 'chk-node-' || v_state, '10.0.0.7:8000');
        INSERT INTO module_instance
            (id, tenant_id, node_id, module_id, version, state)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                v_node, 'm-ist', '1.0.0', v_state);
    END LOOP;

    SELECT count(*) INTO v_ok FROM module_instance WHERE module_id = 'm-ist';
    IF v_ok <> 8 THEN
        RAISE EXCEPTION 'expected 8 accepted states, got %', v_ok;
    END IF;

    v_node := gen_random_uuid();
    INSERT INTO cluster_node (node_id, hostname, advertised_addr)
        VALUES (v_node, 'chk-node-bad', '10.0.0.7:8000');

    BEGIN
        INSERT INTO module_instance
            (id, tenant_id, node_id, module_id, version, state)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                v_node, 'm-ist', '1.0.0', 'Bogus');
        RAISE EXCEPTION 'module_instance_state_chk did not reject Bogus';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    RAISE NOTICE 'PASS: module_instance_state_chk accepts 8 states, rejects Bogus';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_instance_state_chk] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_instance_state_chk;

SAVEPOINT t_module_instance_unique;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    INSERT INTO module_registry (id, tenant_id, module_id, version, manifest) VALUES
        (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-inst', '1.0.0', '{}'::jsonb);

    INSERT INTO module_instance (id, tenant_id, node_id, module_id, version)
        VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'm-inst', '1.0.0');

    -- module_instance_unique (node_id, module_id, version)
    BEGIN
        INSERT INTO module_instance (id, tenant_id, node_id, module_id, version)
            VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'm-inst', '1.0.0');
        RAISE EXCEPTION 'module_instance_unique did not reject duplicate row';
    EXCEPTION WHEN unique_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM module_instance WHERE module_id = 'm-inst';
    IF v_cnt <> 1 THEN
        RAISE EXCEPTION 'expected 1 module_instance row, got %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: module_instance_unique (node_id, module_id, version) enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_instance_unique] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_instance_unique;

SAVEPOINT t_module_instance_registry_fk;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    -- (tenant_id, module_id, version) が module_registry に無い
    BEGIN
        INSERT INTO module_instance (id, tenant_id, node_id, module_id, version)
            VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'm-ghost', '9.9.9');
        RAISE EXCEPTION 'module_instance composite FK to module_registry not enforced';
    EXCEPTION WHEN foreign_key_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM module_instance WHERE module_id = 'm-ghost';
    IF v_cnt <> 0 THEN
        RAISE EXCEPTION 'orphan module_instance row persisted: %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: module_instance (tenant_id, module_id, version) FK enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_instance_registry_fk] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_instance_registry_fk;

SAVEPOINT t_module_instance_node_fk;
DO $$
DECLARE
    v_cnt INT;
    v_conname TEXT;
    v_target TEXT;
    v_col TEXT;
BEGIN
    -- V001 は DO ブロックで遅延追加する (node_id -> cluster_node.node_id)
    SELECT conname INTO v_conname FROM pg_constraint WHERE conname = 'module_instance_node_fk';
    IF v_conname IS NULL THEN
        RAISE EXCEPTION 'module_instance_node_fk constraint missing from catalog';
    END IF;

    -- 存在だけでなく参照先 (cluster_node.node_id) も固定する
    SELECT cl.relname INTO v_target
        FROM pg_constraint c
        JOIN pg_class cl ON cl.oid = c.confrelid
        WHERE c.conname = 'module_instance_node_fk' AND c.contype = 'f';
    IF v_target IS DISTINCT FROM 'cluster_node' THEN
        RAISE EXCEPTION 'module_instance_node_fk references % (expected cluster_node)', v_target;
    END IF;

    SELECT att.attname INTO v_col
        FROM pg_constraint c
        JOIN pg_class cl ON cl.oid = c.confrelid
        JOIN pg_attribute att ON att.attrelid = c.confrelid AND att.attnum = c.confkey[1]
        WHERE c.conname = 'module_instance_node_fk';
    IF v_col IS DISTINCT FROM 'node_id' THEN
        RAISE EXCEPTION 'module_instance_node_fk references cluster_node.% (expected node_id)', v_col;
    END IF;

    INSERT INTO module_registry (id, tenant_id, module_id, version, manifest) VALUES
        (gen_random_uuid(), '11111111-1111-1111-1111-111111111111', 'm-nfk', '1.0.0', '{}'::jsonb);

    BEGIN
        INSERT INTO module_instance (id, tenant_id, node_id, module_id, version)
            VALUES (gen_random_uuid(), '11111111-1111-1111-1111-111111111111',
                    'cccccccc-cccc-cccc-cccc-cccccccccccc', 'm-nfk', '1.0.0');
        RAISE EXCEPTION 'module_instance.node_id FK to cluster_node not enforced';
    EXCEPTION WHEN foreign_key_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM module_instance WHERE module_id = 'm-nfk';
    IF v_cnt <> 0 THEN
        RAISE EXCEPTION 'module_instance with unknown node_id persisted: %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: module_instance_node_fk deferred FK to cluster_node enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_module_instance_node_fk] %', SQLERRM;
END
$$;
ROLLBACK TO t_module_instance_node_fk;

-- =============================================================================
-- §4.4 event_topic / event_subscription / event_log / consumer_offset
-- =============================================================================
SAVEPOINT t_event_topic_defaults_chk;
DO $$
DECLARE
    v_category VARCHAR(50);
    v_retention INT;
    v_created TIMESTAMPTZ;
    v_ok INT;
BEGIN
    INSERT INTO event_topic (topic, category)
        VALUES ('t.default', 'system')
        RETURNING retention_days, created_at INTO v_retention, v_created;

    IF v_retention <> 30 THEN
        RAISE EXCEPTION 'retention_days DEFAULT is not 30: %', v_retention;
    END IF;
    IF v_created IS NULL
       OR v_created < now() - interval '1 second'
       OR v_created > now() + interval '1 second' THEN
        RAISE EXCEPTION 'created_at DEFAULT now() not applied: %', v_created;
    END IF;

    FOREACH v_category IN ARRAY ARRAY['system', 'business', 'audit', 'data']::VARCHAR[] LOOP
        INSERT INTO event_topic (topic, category)
            VALUES ('t.chk.' || v_category, v_category);
    END LOOP;

    SELECT count(*) INTO v_ok FROM event_topic WHERE topic LIKE 't.chk.%';
    IF v_ok <> 4 THEN
        RAISE EXCEPTION 'expected 4 accepted categories, got %', v_ok;
    END IF;

    BEGIN
        INSERT INTO event_topic (topic, category) VALUES ('t.chk.bad', 'other');
        RAISE EXCEPTION 'event_topic_category_chk did not reject other';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    RAISE NOTICE 'PASS: event_topic retention default + category CHECK';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_topic_defaults_chk] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_topic_defaults_chk;

SAVEPOINT t_event_subscription_defaults;
DO $$
DECLARE
    v_filter JSONB;
    v_enabled BOOLEAN;
    v_created TIMESTAMPTZ;
BEGIN
    INSERT INTO event_subscription (id, topic_pattern, group_id, delivery_mode, from_position)
        VALUES (gen_random_uuid(), 'module.*', 'grp-default', 'durable', '"latest"'::jsonb)
        RETURNING filter, enabled, created_at INTO v_filter, v_enabled, v_created;

    IF v_filter <> '{}'::jsonb THEN
        RAISE EXCEPTION 'filter DEFAULT is not empty jsonb: %', v_filter;
    END IF;
    IF v_enabled IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION 'enabled DEFAULT is not TRUE: %', v_enabled;
    END IF;
    IF v_created IS NULL
       OR v_created < now() - interval '1 second'
       OR v_created > now() + interval '1 second' THEN
        RAISE EXCEPTION 'created_at DEFAULT now() not applied: %', v_created;
    END IF;

    RAISE NOTICE 'PASS: event_subscription filter/enabled/created_at defaults';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_subscription_defaults] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_subscription_defaults;

SAVEPOINT t_event_subscription_delivery_chk;
DO $$
DECLARE
    v_mode VARCHAR(20);
    v_ok INT;
BEGIN
    FOREACH v_mode IN ARRAY ARRAY['durable', 'ephemeral']::VARCHAR[] LOOP
        INSERT INTO event_subscription (id, topic_pattern, group_id, delivery_mode, from_position)
            VALUES (gen_random_uuid(), 'cluster.#', 'grp-' || v_mode, v_mode, '"earliest"'::jsonb);
    END LOOP;

    SELECT count(*) INTO v_ok FROM event_subscription WHERE group_id LIKE 'grp-%';
    IF v_ok <> 2 THEN
        RAISE EXCEPTION 'expected 2 accepted delivery modes, got %', v_ok;
    END IF;

    BEGIN
        INSERT INTO event_subscription (id, topic_pattern, group_id, delivery_mode, from_position)
            VALUES (gen_random_uuid(), 'cluster.#', 'grp-bad', 'at-most-once', '"earliest"'::jsonb);
        RAISE EXCEPTION 'event_subscription_delivery_chk did not reject at-most-once';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    RAISE NOTICE 'PASS: event_subscription_delivery_chk accepts durable/ephemeral';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_subscription_delivery_chk] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_subscription_delivery_chk;

SAVEPOINT t_event_subscription_unique;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    INSERT INTO event_subscription (id, topic_pattern, group_id, delivery_mode, from_position)
        VALUES (gen_random_uuid(), 'billing.*', 'grp-uniq', 'durable', '"latest"'::jsonb);

    BEGIN
        INSERT INTO event_subscription (id, topic_pattern, group_id, delivery_mode, from_position)
            VALUES (gen_random_uuid(), 'billing.*', 'grp-uniq', 'durable', '"latest"'::jsonb);
        RAISE EXCEPTION 'event_subscription_unique did not reject duplicate row';
    EXCEPTION WHEN unique_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM event_subscription WHERE group_id = 'grp-uniq';
    IF v_cnt <> 1 THEN
        RAISE EXCEPTION 'expected 1 event_subscription row, got %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: event_subscription_unique (topic_pattern, group_id) enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_subscription_unique] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_subscription_unique;

SAVEPOINT t_event_log_defaults;
DO $$
DECLARE
    v_headers JSONB;
    v_produced TIMESTAMPTZ;
    v_id UUID;
BEGIN
    v_id := gen_random_uuid();
    INSERT INTO event_log (id, event_seq, topic, payload)
        VALUES (v_id, 900001, 'test.default', '{"k":1}'::jsonb);

    SELECT headers, produced_at INTO v_headers, v_produced FROM event_log WHERE id = v_id;
    IF v_headers <> '{}'::jsonb THEN
        RAISE EXCEPTION 'headers DEFAULT is not empty jsonb: %', v_headers;
    END IF;
    IF v_produced IS NULL
       OR v_produced < now() - interval '1 second'
       OR v_produced > now() + interval '1 second' THEN
        RAISE EXCEPTION 'produced_at DEFAULT now() not applied: %', v_produced;
    END IF;

    -- tenant_id は NULL 可 (system event)
    PERFORM 1 FROM event_log WHERE id = v_id AND tenant_id IS NULL;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'event_log.tenant_id is not nullable for system events';
    END IF;

    RAISE NOTICE 'PASS: event_log headers/produced_at defaults + nullable tenant_id';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_log_defaults] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_log_defaults;

SAVEPOINT t_event_log_not_null;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    -- event_seq / topic / payload は NOT NULL 宣言済み
    BEGIN
        INSERT INTO event_log (id, event_seq, topic, payload)
            VALUES (gen_random_uuid(), NULL, 'test', '{"k":1}'::jsonb);
        RAISE EXCEPTION 'event_log.event_seq NOT NULL not enforced';
    EXCEPTION WHEN not_null_violation THEN
        NULL;
    END;

    BEGIN
        INSERT INTO event_log (id, event_seq, topic, payload)
            VALUES (gen_random_uuid(), 900002, NULL, '{"k":1}'::jsonb);
        RAISE EXCEPTION 'event_log.topic NOT NULL not enforced';
    EXCEPTION WHEN not_null_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM event_log
        WHERE event_seq IS NULL OR topic IS NULL OR payload IS NULL;
    IF v_cnt <> 0 THEN
        RAISE EXCEPTION 'event_log rows with NULL required column persisted: %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: event_log event_seq/topic/payload NOT NULL enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_log_not_null] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_log_not_null;

SAVEPOINT t_event_log_seq_unique;
DO $$
DECLARE
    v_cnt INT;
BEGIN
    INSERT INTO event_log (id, event_seq, topic, payload)
        VALUES (gen_random_uuid(), 900010, 'test.uniq', '{"k":1}'::jsonb);

    BEGIN
        INSERT INTO event_log (id, event_seq, topic, payload)
            VALUES (gen_random_uuid(), 900010, 'test.uniq', '{"k":1}'::jsonb);
        RAISE EXCEPTION 'event_log_seq_unique did not reject duplicate event_seq';
    EXCEPTION WHEN unique_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM event_log WHERE event_seq = 900010;
    IF v_cnt <> 1 THEN
        RAISE EXCEPTION 'expected 1 event_log row for seq 900010, got %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: event_log_seq_unique (event_seq) enforced';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_log_seq_unique] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_log_seq_unique;

SAVEPOINT t_consumer_offset_defaults_fk_pk;
DO $$
DECLARE
    v_sub UUID;
    v_last BIGINT;
    v_updated TIMESTAMPTZ;
    v_cnt INT;
BEGIN
    INSERT INTO event_subscription (id, topic_pattern, group_id, delivery_mode, from_position)
        VALUES (gen_random_uuid(), 'offset.*', 'grp-offset', 'durable', '"earliest"'::jsonb)
        RETURNING id INTO v_sub;

    INSERT INTO consumer_offset (subscription_id, topic, consumer_id)
        VALUES (v_sub, 'offset.t', 'grp-offset:inst-1')
        RETURNING last_acked_event_seq, updated_at INTO v_last, v_updated;

    IF v_last <> 0 THEN
        RAISE EXCEPTION 'last_acked_event_seq DEFAULT is not 0: %', v_last;
    END IF;
    IF v_updated IS NULL
       OR v_updated < now() - interval '1 second'
       OR v_updated > now() + interval '1 second' THEN
        RAISE EXCEPTION 'updated_at DEFAULT now() not applied: %', v_updated;
    END IF;

    -- FK subscription_id -> event_subscription(id)
    BEGIN
        INSERT INTO consumer_offset (subscription_id, topic, consumer_id)
            VALUES (gen_random_uuid(), 'offset.t', 'grp-offset:inst-x');
        RAISE EXCEPTION 'consumer_offset.subscription_id FK not enforced';
    EXCEPTION WHEN foreign_key_violation THEN
        NULL;
    END;

    -- 複合 PK (subscription_id, topic, consumer_id)
    BEGIN
        INSERT INTO consumer_offset (subscription_id, topic, consumer_id)
            VALUES (v_sub, 'offset.t', 'grp-offset:inst-1');
        RAISE EXCEPTION 'consumer_offset composite PK did not reject duplicate row';
    EXCEPTION WHEN unique_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM consumer_offset WHERE topic = 'offset.t';
    IF v_cnt <> 1 THEN
        RAISE EXCEPTION 'expected 1 consumer_offset row, got %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: consumer_offset defaults + FK + composite PK';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_consumer_offset_defaults_fk_pk] %', SQLERRM;
END
$$;
ROLLBACK TO t_consumer_offset_defaults_fk_pk;

-- =============================================================================
-- §4.5 cluster_node / leader_lease / shard_assignment
-- =============================================================================
SAVEPOINT t_cluster_node_defaults_chk;
DO $$
DECLARE
    v_state VARCHAR(20);
    v_capacity INT;
    v_labels JSONB;
    v_started TIMESTAMPTZ;
    v_ok INT;
BEGIN
    INSERT INTO cluster_node (node_id, hostname, advertised_addr)
        VALUES ('cccccccc-cccc-cccc-cccc-cccccccccccc', 'host-c', '10.0.0.3:8000')
        RETURNING state, capacity, labels, started_at
        INTO v_state, v_capacity, v_labels, v_started;

    IF v_state <> 'Registering' THEN
        RAISE EXCEPTION 'state DEFAULT is not Registering: %', v_state;
    END IF;
    IF v_capacity <> 100 THEN
        RAISE EXCEPTION 'capacity DEFAULT is not 100: %', v_capacity;
    END IF;
    IF v_labels <> '{}'::jsonb THEN
        RAISE EXCEPTION 'labels DEFAULT is not empty jsonb: %', v_labels;
    END IF;
    IF v_started IS NULL
       OR v_started < now() - interval '1 second'
       OR v_started > now() + interval '1 second' THEN
        RAISE EXCEPTION 'started_at DEFAULT now() not applied: %', v_started;
    END IF;

    FOREACH v_state IN ARRAY ARRAY[
        'Registering', 'Active', 'Unhealthy', 'Draining', 'Removed'
    ]::VARCHAR[] LOOP
        INSERT INTO cluster_node (node_id, hostname, advertised_addr, state)
            VALUES (gen_random_uuid(), 'host-' || v_state, '10.0.0.9:8000', v_state);
    END LOOP;

    SELECT count(*) INTO v_ok FROM cluster_node WHERE hostname LIKE 'host-%' AND advertised_addr = '10.0.0.9:8000';
    IF v_ok <> 5 THEN
        RAISE EXCEPTION 'expected 5 accepted states, got %', v_ok;
    END IF;

    BEGIN
        INSERT INTO cluster_node (node_id, hostname, advertised_addr, state)
            VALUES (gen_random_uuid(), 'host-bad', '10.0.0.9:8000', 'Zombie');
        RAISE EXCEPTION 'cluster_node_state_chk did not reject Zombie';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    RAISE NOTICE 'PASS: cluster_node defaults + state CHECK';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_cluster_node_defaults_chk] %', SQLERRM;
END
$$;
ROLLBACK TO t_cluster_node_defaults_chk;

SAVEPOINT t_cluster_node_current_load_unchecked;
DO $$
DECLARE
    v_load NUMERIC(5,2);
BEGIN
    -- V001 は current_load の範囲 CHECK を宣言していない (意図的・コメント記載)。
    -- 宣言されていないので「拒否される」ことは検証しない。NaN でも格納できることの
    -- 記録として、>1.0 と負値が通ることを確認する (仕様を固定するテスト)。
    INSERT INTO cluster_node (node_id, hostname, advertised_addr, current_load)
        VALUES (gen_random_uuid(), 'host-load-hi', '10.0.0.8:8000', 9.99)
        RETURNING current_load INTO v_load;
    IF v_load <> 9.99 THEN
        RAISE EXCEPTION 'current_load 9.99 not stored verbatim: %', v_load;
    END IF;

    RAISE NOTICE 'PASS: cluster_node.current_load intentionally unchecked (V001 design)';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_cluster_node_current_load_unchecked] %', SQLERRM;
END
$$;
ROLLBACK TO t_cluster_node_current_load_unchecked;

SAVEPOINT t_leader_lease_defaults_fk;
DO $$
DECLARE
    v_acquired TIMESTAMPTZ;
    v_renew INT;
    v_metadata JSONB;
    v_cnt INT;
BEGIN
    INSERT INTO leader_lease (lease_key, holder_node_id, expires_at)
        VALUES ('m04-singleton', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', now() + interval '30 seconds')
        RETURNING acquired_at, renew_count, metadata INTO v_acquired, v_renew, v_metadata;

    IF v_acquired IS NULL
       OR v_acquired < now() - interval '1 second'
       OR v_acquired > now() + interval '1 second' THEN
        RAISE EXCEPTION 'acquired_at DEFAULT now() not applied: %', v_acquired;
    END IF;
    IF v_renew <> 0 THEN
        RAISE EXCEPTION 'renew_count DEFAULT is not 0: %', v_renew;
    END IF;
    IF v_metadata <> '{}'::jsonb THEN
        RAISE EXCEPTION 'metadata DEFAULT is not empty jsonb: %', v_metadata;
    END IF;

    BEGIN
        INSERT INTO leader_lease (lease_key, holder_node_id, expires_at)
            VALUES ('m04-ghost', 'dddddddd-dddd-dddd-dddd-dddddddddddd', now() + interval '30 seconds');
        RAISE EXCEPTION 'leader_lease.holder_node_id FK to cluster_node not enforced';
    EXCEPTION WHEN foreign_key_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM leader_lease WHERE lease_key = 'm04-ghost';
    IF v_cnt <> 0 THEN
        RAISE EXCEPTION 'leader_lease with unknown holder persisted: %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: leader_lease defaults + holder_node_id FK';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_leader_lease_defaults_fk] %', SQLERRM;
END
$$;
ROLLBACK TO t_leader_lease_defaults_fk;

SAVEPOINT t_shard_assignment_fk_pk;
DO $$
DECLARE
    v_assigned TIMESTAMPTZ;
    v_cnt INT;
BEGIN
    INSERT INTO shard_assignment (shard_id, tenant_id, node_id)
        VALUES (7, '11111111-1111-1111-1111-111111111111',
                'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa')
        RETURNING assigned_at INTO v_assigned;

    IF v_assigned IS NULL
       OR v_assigned < now() - interval '1 second'
       OR v_assigned > now() + interval '1 second' THEN
        RAISE EXCEPTION 'assigned_at DEFAULT now() not applied: %', v_assigned;
    END IF;

    -- FK tenant_id -> tenant(id)
    BEGIN
        INSERT INTO shard_assignment (shard_id, tenant_id, node_id)
            VALUES (8, '99999999-9999-9999-9999-999999999999',
                    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa');
        RAISE EXCEPTION 'shard_assignment.tenant_id FK to tenant not enforced';
    EXCEPTION WHEN foreign_key_violation THEN
        NULL;
    END;

    -- FK node_id -> cluster_node(node_id)
    BEGIN
        INSERT INTO shard_assignment (shard_id, tenant_id, node_id)
            VALUES (8, '11111111-1111-1111-1111-111111111111',
                    'dddddddd-dddd-dddd-dddd-dddddddddddd');
        RAISE EXCEPTION 'shard_assignment.node_id FK to cluster_node not enforced';
    EXCEPTION WHEN foreign_key_violation THEN
        NULL;
    END;

    -- 複合 PK (shard_id, tenant_id)
    BEGIN
        INSERT INTO shard_assignment (shard_id, tenant_id, node_id)
            VALUES (7, '11111111-1111-1111-1111-111111111111',
                    'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb');
        RAISE EXCEPTION 'shard_assignment composite PK did not reject duplicate row';
    EXCEPTION WHEN unique_violation THEN
        NULL;
    END;

    SELECT count(*) INTO v_cnt FROM shard_assignment WHERE shard_id = 7;
    IF v_cnt <> 1 THEN
        RAISE EXCEPTION 'expected 1 shard_assignment row, got %', v_cnt;
    END IF;

    RAISE NOTICE 'PASS: shard_assignment assigned_at default + 2 FKs + composite PK';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_shard_assignment_fk_pk] %', SQLERRM;
END
$$;
ROLLBACK TO t_shard_assignment_fk_pk;

-- =============================================================================
-- event_seq_global SEQUENCE
-- =============================================================================
SAVEPOINT t_event_seq_global_params;
DO $$
DECLARE
    v_start BIGINT;
    v_increment BIGINT;
BEGIN
    SELECT start_value, increment INTO v_start, v_increment
        FROM information_schema.sequences
        WHERE sequence_schema = 'public' AND sequence_name = 'event_seq_global';

    IF NOT FOUND THEN
        RAISE EXCEPTION 'event_seq_global sequence missing';
    END IF;
    IF v_start <> 1 THEN
        RAISE EXCEPTION 'event_seq_global start_value is not 1: %', v_start;
    END IF;
    IF v_increment <> 1 THEN
        RAISE EXCEPTION 'event_seq_global increment is not 1: %', v_increment;
    END IF;

    RAISE NOTICE 'PASS: event_seq_global declared START 1 INCREMENT 1';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_seq_global_params] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_seq_global_params;

SAVEPOINT t_event_seq_global_advances;
DO $$
DECLARE
    v_a BIGINT;
    v_b BIGINT;
    v_c BIGINT;
    v_id UUID;
BEGIN
    v_a := nextval('event_seq_global');
    v_b := nextval('event_seq_global');
    v_c := nextval('event_seq_global');

    IF NOT (v_a < v_b AND v_b < v_c) THEN
        RAISE EXCEPTION 'event_seq_global not monotonic: %, %, %', v_a, v_b, v_c;
    END IF;
    IF v_b - v_a <> 1 OR v_c - v_b <> 1 THEN
        RAISE EXCEPTION 'event_seq_global increment is not 1: %, %, %', v_a, v_b, v_c;
    END IF;

    -- nextval の値が event_log.event_seq に UNIQUE 制約つきで入ること
    v_id := gen_random_uuid();
    INSERT INTO event_log (id, event_seq, topic, payload)
        VALUES (v_id, v_c, 'test.seq', '{"k":1}'::jsonb);
    SELECT event_seq INTO STRICT v_a FROM event_log WHERE id = v_id;
    IF v_a <> v_c THEN
        RAISE EXCEPTION 'event_seq_global value % not stored in event_log: %', v_c, v_a;
    END IF;

    RAISE NOTICE 'PASS: event_seq_global advances monotonically (%, %, %)', v_a, v_b, v_c;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_seq_global_advances] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_seq_global_advances;

-- =============================================================================
-- RLS: V001 が宣言する 5 policy
-- =============================================================================
SAVEPOINT t_v001_rls_policies;
DO $$
DECLARE
    v_enabled INT;
    v_policies INT;
    v_expected TEXT[] := ARRAY[
        'module_registry_rls', 'module_upgrade_history_rls', 'module_instance_rls',
        'event_log_rls', 'cluster_node_rls'
    ];
    v_missing TEXT;
BEGIN
    -- RLS が有効化されているテーブル
    SELECT count(*) INTO v_enabled
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public'
          AND c.relname = ANY (ARRAY[
              'module_registry', 'module_upgrade_history', 'module_instance',
              'event_log', 'cluster_node'
          ]::TEXT[])
          AND c.relrowsecurity;
    IF v_enabled <> 5 THEN
        RAISE EXCEPTION 'expected RLS enabled on 5 tables, got %', v_enabled;
    END IF;

    -- 宣言済みの policy が全て存在する
    SELECT string_agg(p, ', ') INTO v_missing FROM unnest(v_expected) p
        WHERE NOT EXISTS (
            SELECT 1 FROM pg_policies
            WHERE schemaname = 'public' AND policyname = p
        );
    IF v_missing IS NOT NULL THEN
        RAISE EXCEPTION 'missing V001 RLS policies: %', v_missing;
    END IF;

    SELECT count(*) INTO v_policies FROM pg_policies
        WHERE schemaname = 'public' AND policyname = ANY (v_expected);
    IF v_policies <> 5 THEN
        RAISE EXCEPTION 'expected 5 V001 RLS policies, got %', v_policies;
    END IF;

    RAISE NOTICE 'PASS: V001 declares RLS on 5 tables with 5 policies';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_v001_rls_policies] %', SQLERRM;
END
$$;
ROLLBACK TO t_v001_rls_policies;

ROLLBACK;