-- =============================================================================
-- V002__plpgsql_functions_test.sql
--
-- Ada 6 本 PL/pgSQL 存过の単体テスト (PostgreSQL 18.6 想定)
--
-- 方針:
--   - pgTAP 等の外部依存は導入しない (pure SQL のみ)
--   - ファイル全体を BEGIN; ... ROLLBACK; で包み、テストデータを一切残さない
--   - 各 test case は SAVEPOINT / ROLLBACK TO で独立 (1 件失敗が他に影響しない)
--   - RLS セッション変数は set_config(..., true) で transaction-local 設定
--     (M-10 §3.1 with_tenant_scope パターンと一致)
--   - 検証は DO ブロック内 RAISE EXCEPTION (failure) / RAISE NOTICE (pass) で行う
--
-- 前提:
--   - V001__init_schema.sql と V002__plpgsql_functions.sql が既に適用済み
--   - 接続ユーザーは SUPERUSER 相当 (SECURITY DEFINER + RLS bypass が必要なため)
--   - pg_notify は verify しない (LISTEN 接続が必要、別テスト領域)
--
-- 実行: psql -d ada_test -v ON_ERROR_STOP=1 -f V002__plpgsql_functions_test.sql
-- =============================================================================

BEGIN;

-- =============================================================================
-- Phase 0: テスト用フィクスチャ (RLS セッション + 親テーブル)
-- =============================================================================
SELECT set_config('app.current_tenant', '11111111-1111-1111-1111-111111111111', true);
SELECT set_config('app.current_user_id', '22222222-2222-2222-2222-222222222222', true);
SELECT set_config('app.current_service', 'pgtest-runner',                true);

-- 親テーブル: tenant / cluster_node
--   (これらは module_registry / heartbeat / leader_lease の FK ターゲット)
INSERT INTO tenant (id, name) VALUES
    ('11111111-1111-1111-1111-111111111111', 'tenant-A')
    ON CONFLICT (id) DO NOTHING;

-- lease / heartbeat テストで使われる fixture node
--   (各 test の SAVEPOINT 外で insert するので、ROLLBACK TO 後も存続)
INSERT INTO cluster_node (node_id, hostname, advertised_addr, state) VALUES
    ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'test-host-a', '10.0.0.1:8000', 'Active'),
    ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'test-host-b', '10.0.0.2:8000', 'Active')
    ON CONFLICT (node_id) DO NOTHING;

-- =============================================================================
-- Phase 1: register_module
-- =============================================================================
SAVEPOINT t_register_idempotent;
DO $$
DECLARE
    v_succ1 BOOLEAN; v_id1 UUID; v_err1 TEXT;
    v_succ2 BOOLEAN; v_id2 UUID; v_err2 TEXT;
    v_count INT;
BEGIN
    -- 1回目: 新規作成
    SELECT success, module_instance_id, error_msg
        INTO v_succ1, v_id1, v_err1
        FROM register_module(
            'm01-test',
            '1.0.0',
            jsonb_build_object('meta', jsonb_build_object(
                'module_id', 'm01-test', 'version', '1.0.0'
            )),
            's3://test/m01-1.0.0',
            'a' || repeat('b', 63)
        );
    IF NOT v_succ1 OR v_id1 IS NULL THEN
        RAISE EXCEPTION '1st call: success=%, id=%, err=%', v_succ1, v_id1, v_err1;
    END IF;

    -- 2回目: 同一 (module_id, version) → 同一 UUID 返却 (幂等)
    SELECT success, module_instance_id, error_msg
        INTO v_succ2, v_id2, v_err2
        FROM register_module(
            'm01-test',
            '1.0.0',
            jsonb_build_object('meta', jsonb_build_object(
                'module_id', 'm01-test', 'version', '1.0.0'
            )),
            's3://test/m01-1.0.0',
            'a' || repeat('b', 63)
        );
    IF NOT v_succ2 OR v_id2 != v_id1 THEN
        RAISE EXCEPTION 'idempotency broken: id1=%, id2=%, err2=%', v_id1, v_id2, v_err2;
    END IF;

    -- module_registry 行は 1 件のみ (UNIQUE 制約)
    SELECT COUNT(*) INTO v_count FROM module_registry
        WHERE module_id = 'm01-test' AND version = '1.0.0';
    IF v_count != 1 THEN
        RAISE EXCEPTION 'expected 1 row, got %', v_count;
    END IF;

    RAISE NOTICE 'PASS: register_module idempotent (uuid=%)', v_id1;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_register_idempotent] %', SQLERRM;
END
$$;
ROLLBACK TO t_register_idempotent;

SAVEPOINT t_register_validate_manifest;
DO $$
DECLARE
    v_succ BOOLEAN; v_id UUID; v_err TEXT;
BEGIN
    -- manifest 必須フィールド欠如
    SELECT success, module_instance_id, error_msg
        INTO v_succ, v_id, v_err
        FROM register_module(
            'm99-bad', '0.1.0',
            jsonb_build_object('meta', jsonb_build_object('module_id', 'm99-bad')),
            -- 故意に version 欠如
            's3://x', repeat('c', 64)
        );
    IF v_succ OR v_id IS NOT NULL OR v_err IS NULL THEN
        RAISE EXCEPTION 'expected fail, got success=%, id=%, err=%', v_succ, v_id, v_err;
    END IF;
    IF v_err NOT LIKE '%version%' THEN
        RAISE EXCEPTION 'expected error msg to mention version, got: %', v_err;
    END IF;

    RAISE NOTICE 'PASS: register_module manifest validation (err=%)', v_err;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_register_validate_manifest] %', SQLERRM;
END
$$;
ROLLBACK TO t_register_validate_manifest;

SAVEPOINT t_register_emit_event;
DO $$
DECLARE
    v_succ BOOLEAN; v_id UUID; v_err TEXT;
    v_event_count INT;
BEGIN
    -- 登録成功
    SELECT success, module_instance_id, error_msg
        INTO v_succ, v_id, v_err
        FROM register_module(
            'm02-evented', '2.0.0',
            jsonb_build_object('meta', jsonb_build_object(
                'module_id', 'm02-evented', 'version', '2.0.0'
            )),
            's3://test/m02-2.0.0',
            repeat('d', 64)
        );
    IF NOT v_succ THEN
        RAISE EXCEPTION 'register failed: %', v_err;
    END IF;

    -- module.registered イベントが event_log に記録されている
    SELECT COUNT(*) INTO v_event_count
        FROM event_log
        WHERE topic = 'module.registered'
          AND payload->>'module_id' = 'm02-evented'
          AND payload->>'version' = '2.0.0'
          AND payload->>'instance_id' = v_id::text;
    IF v_event_count != 1 THEN
        RAISE EXCEPTION 'expected 1 module.registered event, got %', v_event_count;
    END IF;

    RAISE NOTICE 'PASS: register_module emits module.registered event (count=%)', v_event_count;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_register_emit_event] %', SQLERRM;
END
$$;
ROLLBACK TO t_register_emit_event;

-- =============================================================================
-- Phase 2: atomic_module_swap
-- =============================================================================
SAVEPOINT t_swap_basic;
DO $$
DECLARE
    v_succ_reg BOOLEAN; v_id_from UUID; v_id_to UUID; v_err TEXT;
    v_succ_swap BOOLEAN; v_swap_err TEXT;
    v_active_count INT;
    v_history_count INT;
    v_event_count INT;
    v_from_active BOOLEAN;
    v_to_active BOOLEAN;
BEGIN
    -- 2 バージョン登録
    SELECT success, module_instance_id, error_msg
        INTO v_succ_reg, v_id_from, v_err
        FROM register_module(
            'm03-swap', '1.0.0',
            jsonb_build_object('meta', jsonb_build_object(
                'module_id', 'm03-swap', 'version', '1.0.0'
            )),
            's3://x', repeat('a', 64)
        );
    IF NOT v_succ_reg THEN RAISE EXCEPTION 'register from failed: %', v_err; END IF;

    SELECT success, module_instance_id, error_msg
        INTO v_succ_reg, v_id_to, v_err
        FROM register_module(
            'm03-swap', '2.0.0',
            jsonb_build_object('meta', jsonb_build_object(
                'module_id', 'm03-swap', 'version', '2.0.0'
            )),
            's3://y', repeat('b', 64)
        );
    IF NOT v_succ_reg THEN RAISE EXCEPTION 'register to failed: %', v_err; END IF;

    -- swap 実行
    SELECT success, error_msg
        INTO v_succ_swap, v_swap_err
        FROM atomic_module_swap('m03-swap', '1.0.0', '2.0.0');
    IF NOT v_succ_swap THEN
        RAISE EXCEPTION 'swap failed: %', v_swap_err;
    END IF;

    -- from=inactive, to=active 確認
    SELECT active INTO v_from_active FROM module_registry WHERE id = v_id_from;
    SELECT active INTO v_to_active   FROM module_registry WHERE id = v_id_to;
    IF v_from_active IS NOT FALSE THEN
        RAISE EXCEPTION 'from should be inactive, got %', v_from_active;
    END IF;
    IF v_to_active IS NOT TRUE THEN
        RAISE EXCEPTION 'to should be active, got %', v_to_active;
    END IF;

    -- history 行存在
    SELECT COUNT(*) INTO v_history_count
        FROM module_upgrade_history
        WHERE module_id = 'm03-swap'
          AND from_version = '1.0.0'
          AND to_version = '2.0.0'
          AND strategy = 'atomic_swap'
          AND status = 'Succeeded';
    IF v_history_count != 1 THEN
        RAISE EXCEPTION 'expected 1 history row, got %', v_history_count;
    END IF;

    -- event 存在
    SELECT COUNT(*) INTO v_event_count
        FROM event_log
        WHERE topic = 'module.swapped'
          AND payload->>'module_id' = 'm03-swap'
          AND payload->>'from' = '1.0.0'
          AND payload->>'to' = '2.0.0';
    IF v_event_count != 1 THEN
        RAISE EXCEPTION 'expected 1 module.swapped event, got %', v_event_count;
    END IF;

    RAISE NOTICE 'PASS: atomic_module_swap basic (from→inactive, to→active, history+event emitted)';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_swap_basic] %', SQLERRM;
END
$$;
ROLLBACK TO t_swap_basic;

SAVEPOINT t_swap_from_missing;
DO $$
DECLARE
    v_succ BOOLEAN; v_err TEXT;
BEGIN
    -- from_version が存在しないケース
    SELECT success, error_msg INTO v_succ, v_err
        FROM atomic_module_swap('m99-nonexistent', '0.0.1', '0.0.2');
    IF v_succ OR v_err NOT LIKE '%from_version not found%' THEN
        RAISE EXCEPTION 'expected from_version not found, got success=%, err=%', v_succ, v_err;
    END IF;

    RAISE NOTICE 'PASS: atomic_module_swap rejects missing from_version (err=%)', v_err;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_swap_from_missing] %', SQLERRM;
END
$$;
ROLLBACK TO t_swap_from_missing;

-- =============================================================================
-- Phase 3: append_event
-- =============================================================================
SAVEPOINT t_event_seq_monotonic;
DO $$
DECLARE
    v_id1 UUID; v_seq1 BIGINT;
    v_id2 UUID; v_seq2 BIGINT;
    v_id3 UUID; v_seq3 BIGINT;
BEGIN
    SELECT event_id, event_seq INTO v_id1, v_seq1
        FROM append_event('test.event.a', jsonb_build_object('k', 1));
    SELECT event_id, event_seq INTO v_id2, v_seq2
        FROM append_event('test.event.b', jsonb_build_object('k', 2));
    SELECT event_id, event_seq INTO v_id3, v_seq3
        FROM append_event('test.event.a', jsonb_build_object('k', 3));

    -- 単調増加
    IF NOT (v_seq1 < v_seq2 AND v_seq2 < v_seq3) THEN
        RAISE EXCEPTION 'event_seq not monotonic: %, %, %', v_seq1, v_seq2, v_seq3;
    END IF;

    -- 重複なし
    IF v_id1 = v_id2 OR v_id2 = v_id3 OR v_id1 = v_id3 THEN
        RAISE EXCEPTION 'event_id duplicated: %, %, %', v_id1, v_id2, v_id3;
    END IF;

    -- event_log に 3 行存在
    IF (SELECT COUNT(*) FROM event_log WHERE id IN (v_id1, v_id2, v_id3)) != 3 THEN
        RAISE EXCEPTION 'event_log row count mismatch';
    END IF;

    RAISE NOTICE 'PASS: append_event seq monotonic (seq1=%, seq2=%, seq3=%)', v_seq1, v_seq2, v_seq3;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_seq_monotonic] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_seq_monotonic;

SAVEPOINT t_event_tenant_injection;
DO $$
DECLARE
    v_id UUID; v_seq BIGINT;
    v_log_tenant UUID;
    v_log_producer TEXT;
BEGIN
    SELECT event_id, event_seq INTO v_id, v_seq
        FROM append_event('test.tenant.injected', jsonb_build_object('k', 'v'));
    -- app.current_tenant が自動注入される
    SELECT tenant_id, producer INTO v_log_tenant, v_log_producer
        FROM event_log WHERE id = v_id;
    IF v_log_tenant != '11111111-1111-1111-1111-111111111111'::UUID THEN
        RAISE EXCEPTION 'tenant_id not injected: got %', v_log_tenant;
    END IF;
    IF v_log_producer != 'pgtest-runner' THEN
        RAISE EXCEPTION 'producer not injected: got %', v_log_producer;
    END IF;

    RAISE NOTICE 'PASS: append_event injects tenant_id+producer from session (tenant=%, producer=%)',
        v_log_tenant, v_log_producer;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_event_tenant_injection] %', SQLERRM;
END
$$;
ROLLBACK TO t_event_tenant_injection;

-- =============================================================================
-- Phase 4: acquire_lease / release_lease
-- =============================================================================
SAVEPOINT t_lease_basic_acquire_release;
DO $$
DECLARE
    v_node1 CONSTANT UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    v_acq BOOLEAN; v_exp TIMESTAMPTZ; v_id UUID;
    v_rel BOOLEAN;
    v_count INT;
BEGIN
    -- 新規取得
    SELECT acquired, lease_id, expires_at
        INTO v_acq, v_id, v_exp
        FROM acquire_lease('m04-singleton', v_node1, 60);
    IF NOT v_acq OR v_exp IS NULL THEN
        RAISE EXCEPTION 'acquire failed: acq=%, exp=%', v_acq, v_exp;
    END IF;

    -- leader_lease 行存在
    SELECT COUNT(*) INTO v_count FROM leader_lease WHERE lease_key = 'm04-singleton';
    IF v_count != 1 THEN
        RAISE EXCEPTION 'expected 1 leader_lease row, got %', v_count;
    END IF;

    -- 解放
    SELECT released INTO v_rel FROM release_lease('m04-singleton', v_node1);
    IF NOT v_rel THEN RAISE EXCEPTION 'release failed'; END IF;

    -- 行消滅
    SELECT COUNT(*) INTO v_count FROM leader_lease WHERE lease_key = 'm04-singleton';
    IF v_count != 0 THEN
        RAISE EXCEPTION 'expected 0 leader_lease row after release, got %', v_count;
    END IF;

    RAISE NOTICE 'PASS: acquire+release_lease basic';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_lease_basic_acquire_release] %', SQLERRM;
END
$$;
ROLLBACK TO t_lease_basic_acquire_release;

SAVEPOINT t_lease_renew;
DO $$
DECLARE
    v_node1 CONSTANT UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    v_acq1 BOOLEAN; v_exp1 TIMESTAMPTZ; v_renew1 INT;
    v_acq2 BOOLEAN; v_exp2 TIMESTAMPTZ; v_renew2 INT;
BEGIN
    -- 初回
    SELECT acquired, expires_at INTO v_acq1, v_exp1
        FROM acquire_lease('m04-renew', v_node1, 60);
    IF NOT v_acq1 THEN RAISE EXCEPTION 'first acquire failed'; END IF;
    SELECT renew_count INTO v_renew1 FROM leader_lease WHERE lease_key = 'm04-renew';

    -- 同 holder 续約
    SELECT acquired, expires_at INTO v_acq2, v_exp2
        FROM acquire_lease('m04-renew', v_node1, 60);
    IF NOT v_acq2 THEN RAISE EXCEPTION 'renew acquire failed'; END IF;
    SELECT renew_count INTO v_renew2 FROM leader_lease WHERE lease_key = 'm04-renew';

    IF v_renew2 != v_renew1 + 1 THEN
        RAISE EXCEPTION 'renew_count not incremented: % → %', v_renew1, v_renew2;
    END IF;
    IF v_exp2 <= v_exp1 THEN
        RAISE EXCEPTION 'expires_at not extended: % → %', v_exp1, v_exp2;
    END IF;

    RAISE NOTICE 'PASS: acquire_lease renew (renew_count % → %, expires_at 延長)', v_renew1, v_renew2;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_lease_renew] %', SQLERRM;
END
$$;
ROLLBACK TO t_lease_renew;

SAVEPOINT t_lease_contention;
DO $$
DECLARE
    v_node1 CONSTANT UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    v_node2 CONSTANT UUID := 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    v_acq_a BOOLEAN; v_exp_a TIMESTAMPTZ;
    v_acq_b BOOLEAN; v_exp_b TIMESTAMPTZ;
BEGIN
    -- node1 取得 (60s TTL)
    SELECT acquired, expires_at INTO v_acq_a, v_exp_a
        FROM acquire_lease('m04-contend', v_node1, 60);
    IF NOT v_acq_a THEN RAISE EXCEPTION 'node1 acquire failed'; END IF;

    -- node2 取得試行 (TTL 内) → 失敗
    SELECT acquired, expires_at INTO v_acq_b, v_exp_b
        FROM acquire_lease('m04-contend', v_node2, 60);
    IF v_acq_b THEN
        RAISE EXCEPTION 'node2 should fail (TTL not expired)';
    END IF;
    IF v_exp_b != v_exp_a THEN
        RAISE EXCEPTION 'expected node2 sees node1 expiry %, got %', v_exp_a, v_exp_b;
    END IF;

    RAISE NOTICE 'PASS: acquire_lease contention (node2 blocked while node1 holds)';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_lease_contention] %', SQLERRM;
END
$$;
ROLLBACK TO t_lease_contention;

SAVEPOINT t_lease_takeover;
DO $$
DECLARE
    v_node1 CONSTANT UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    v_node2 CONSTANT UUID := 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    v_acq1 BOOLEAN; v_acq2 BOOLEAN; v_exp2 TIMESTAMPTZ;
BEGIN
    -- node1 取得 (TTL=60s だが、後で expires_at を過去に書き換えて抢占検証)
    SELECT acquired INTO v_acq1
        FROM acquire_lease('m04-takeover', v_node1, 60);
    IF NOT v_acq1 THEN RAISE EXCEPTION 'node1 acquire failed'; END IF;

    -- TTL 切れをシミュレート: expires_at を 1s 過去に強制
    UPDATE leader_lease SET expires_at = now() - interval '1 second'
        WHERE lease_key = 'm04-takeover';

    -- node2 抢占
    SELECT acquired, expires_at INTO v_acq2, v_exp2
        FROM acquire_lease('m04-takeover', v_node2, 60);
    IF NOT v_acq2 THEN RAISE EXCEPTION 'node2 takeover failed (acq=FALSE)'; END IF;

    -- holder 確認
    IF (SELECT holder_node_id FROM leader_lease WHERE lease_key = 'm04-takeover') != v_node2 THEN
        RAISE EXCEPTION 'holder not updated to node2';
    END IF;

    -- renew_count: 元 1 (node1 取得時) → node2 抢占で DO UPDATE により +1 = 2
    -- 文档の acquire_lease は renew_count を reset せず、holder 交代時も
    -- 既存行を更新して +1 する設計。
    IF (SELECT renew_count FROM leader_lease WHERE lease_key = 'm04-takeover') != 2 THEN
        RAISE EXCEPTION 'renew_count expected 2 (1 + takeover increment), got %',
            (SELECT renew_count FROM leader_lease WHERE lease_key = 'm04-takeover');
    END IF;

    RAISE NOTICE 'PASS: acquire_lease takeover after TTL expiry (node2=% held, renew_count reset)', v_node2;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_lease_takeover] %', SQLERRM;
END
$$;
ROLLBACK TO t_lease_takeover;

SAVEPOINT t_lease_release_by_non_holder;
DO $$
DECLARE
    v_node1 CONSTANT UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    v_node2 CONSTANT UUID := 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    v_acq BOOLEAN;
    v_rel BOOLEAN;
BEGIN
    -- node1 取得
    SELECT acquired INTO v_acq FROM acquire_lease('m04-no-release', v_node1, 60);
    IF NOT v_acq THEN RAISE EXCEPTION 'acquire failed'; END IF;

    -- node2 が解放試行 → 失敗
    SELECT released INTO v_rel FROM release_lease('m04-no-release', v_node2);
    IF v_rel THEN RAISE EXCEPTION 'non-holder should not release'; END IF;

    -- まだ node1 が保持
    IF (SELECT holder_node_id FROM leader_lease WHERE lease_key = 'm04-no-release') != v_node1 THEN
        RAISE EXCEPTION 'lease should still be held by node1';
    END IF;

    RAISE NOTICE 'PASS: release_lease rejected for non-holder';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_lease_release_by_non_holder] %', SQLERRM;
END
$$;
ROLLBACK TO t_lease_release_by_non_holder;

-- =============================================================================
-- Phase 5: register_node_heartbeat
-- =============================================================================
SAVEPOINT t_heartbeat_missing_node;
DO $$
DECLARE
    v_healthy BOOLEAN; v_load NUMERIC;
BEGIN
    -- 未登録 node で heartbeat → EXCEPTION
    BEGIN
        PERFORM register_node_heartbeat(
            'cccccccc-cccc-cccc-cccc-cccccccccccc'::UUID,
            jsonb_build_object('health', true)
        );
        RAISE EXCEPTION 'expected exception for unknown node, got success';
    EXCEPTION WHEN OTHERS THEN
        IF SQLERRM NOT LIKE '%未登録%' AND SQLERRM NOT LIKE '%not registered%' THEN
            RAISE EXCEPTION 'unexpected error: %', SQLERRM;
        END IF;
    END;
    RAISE NOTICE 'PASS: register_node_heartbeat rejects unknown node_id';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_heartbeat_missing_node] %', SQLERRM;
END
$$;
ROLLBACK TO t_heartbeat_missing_node;

SAVEPOINT t_heartbeat_upsert;
DO $$
DECLARE
    v_node CONSTANT UUID := '99999999-9999-9999-9999-999999999999';
    v_healthy BOOLEAN; v_load NUMERIC;
    v_heartbeat1 TIMESTAMPTZ;
    v_heartbeat2 TIMESTAMPTZ;
BEGIN
    -- 事前 register_node (heartbeat の前提)
    INSERT INTO cluster_node (node_id, hostname, advertised_addr, state)
    VALUES (v_node, 'test-host', '10.0.0.1:8000', 'Active');

    -- 1 回目 heartbeat
    SELECT healthy, current_load INTO v_healthy, v_load
        FROM register_node_heartbeat(
            v_node,
            jsonb_build_object('health', true, 'cpu', 0.3, 'mem', 0.4)
        );
    IF NOT v_healthy THEN RAISE EXCEPTION '1st heartbeat should be healthy'; END IF;
    SELECT last_heartbeat_at INTO v_heartbeat1 FROM cluster_node WHERE node_id = v_node;

    -- last_heartbeat_at を過去に書き換え (時刻差を確実に作る)
    UPDATE cluster_node SET last_heartbeat_at = now() - interval '5 seconds'
        WHERE node_id = v_node;
    v_heartbeat1 := now() - interval '5 seconds';

    -- 2 回目 heartbeat
    SELECT healthy, current_load INTO v_healthy, v_load
        FROM register_node_heartbeat(
            v_node,
            jsonb_build_object('health', false, 'reason', 'cpu-high')
        );
    IF v_healthy THEN RAISE EXCEPTION '2nd heartbeat should be unhealthy'; END IF;
    SELECT last_heartbeat_at INTO v_heartbeat2 FROM cluster_node WHERE node_id = v_node;

    -- last_heartbeat_at が更新
    IF v_heartbeat2 <= v_heartbeat1 THEN
        RAISE EXCEPTION 'last_heartbeat_at not advanced: % → %', v_heartbeat1, v_heartbeat2;
    END IF;

    -- state が Unhealthy に切替
    IF (SELECT state FROM cluster_node WHERE node_id = v_node) != 'Unhealthy' THEN
        RAISE EXCEPTION 'state not flipped to Unhealthy';
    END IF;

    RAISE NOTICE 'PASS: register_node_heartbeat upsert + state flip on unhealthy';
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_heartbeat_upsert] %', SQLERRM;
END
$$;
ROLLBACK TO t_heartbeat_upsert;

-- =============================================================================
-- Phase 6: pg_notify (event_appended) 簡易検証
-- =============================================================================
-- 厳密な LISTEN 検証は別接続が必要。代わりに unlisten の副作用がないことで
-- 「NOTIFY チャンネルが存在する呼び出しでクラッシュしない」ことを確認。
SAVEPOINT t_notify_call;
DO $$
DECLARE
    v_id UUID; v_seq BIGINT;
BEGIN
    SELECT event_id, event_seq INTO v_id, v_seq
        FROM append_event('test.notify.smoke', jsonb_build_object('k', 'v'));
    IF v_id IS NULL OR v_seq IS NULL THEN
        RAISE EXCEPTION 'append_event returned null id/seq';
    END IF;
    RAISE NOTICE 'PASS: append_event does not crash on pg_notify (seq=%)', v_seq;
EXCEPTION WHEN OTHERS THEN
    RAISE EXCEPTION '[t_notify_call] %', SQLERRM;
END
$$;
ROLLBACK TO t_notify_call;

-- =============================================================================
-- 最終 ROLLBACK: テストデータ全消去 (V001/V002 のスキーマは別ファイルで commit 済)
-- =============================================================================
ROLLBACK;
