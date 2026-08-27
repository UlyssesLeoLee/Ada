-- =============================================================================
-- V003__phase8_remediation_test.sql
--
-- Phase 8 Auto-remediation 治理表 + 2 PL/pgSQL 存过の単体テスト
--
-- 方針 (V002__plpgsql_functions_test.sql と一致):
--   - 外部依存ゼロ (pure SQL)
--   - 全体を BEGIN; ... ROLLBACK; で包み、テストデータ残さない
--   - 各 test case は SAVEPOINT / ROLLBACK TO で独立
--   - 検証は DO ブロック内 RAISE EXCEPTION (failure) / RAISE NOTICE (pass)
--
-- 前提: V001 + V002 + V003 migration が適用済み
-- =============================================================================

BEGIN;

-- =============================================================================
-- Phase 0: remediation_history / remediation_cooldowns 存在 + CHECK 制約
-- =============================================================================
SAVEPOINT t_tables_exist;
DO $$
DECLARE
    v_history_count INT;
    v_cooldown_count INT;
BEGIN
    SELECT count(*) INTO v_history_count
        FROM information_schema.tables
        WHERE table_name = 'remediation_history'
          AND table_schema = 'public';
    IF v_history_count <> 1 THEN
        RAISE EXCEPTION 'remediation_history 缺失 (count=%)', v_history_count;
    END IF;

    SELECT count(*) INTO v_cooldown_count
        FROM information_schema.tables
        WHERE table_name = 'remediation_cooldowns'
          AND table_schema = 'public';
    IF v_cooldown_count <> 1 THEN
        RAISE EXCEPTION 'remediation_cooldowns 缺失 (count=%)', v_cooldown_count;
    END IF;

    RAISE NOTICE 'PASS: t_tables_exist';
END;
$$;
ROLLBACK TO SAVEPOINT t_tables_exist;

-- =============================================================================
-- Phase 1: remediation_record_execution — 正常路径 (succeeded)
-- =============================================================================
SAVEPOINT t_record_success;
DO $$
DECLARE
    v_succ BOOLEAN; v_id BIGINT; v_err TEXT;
    v_count INT;
    v_expires_at TIMESTAMPTZ;
BEGIN
    SELECT success, history_id, error_msg
        INTO v_succ, v_id, v_err
        FROM remediation_record_execution(
            'disk-space-low', 'DiskSpaceFillingFast', 'succeeded', 0, NULL, 300
        );
    IF NOT v_succ THEN
        RAISE EXCEPTION 'record_execution(succeeded): success=false, err=%', v_err;
    END IF;
    IF v_id IS NULL OR v_id < 1 THEN
        RAISE EXCEPTION 'record_execution(succeeded): history_id=% 異常', v_id;
    END IF;

    -- history 行写入验证
    SELECT count(*) INTO v_count FROM remediation_history
        WHERE id = v_id AND action_id = 'disk-space-low'
          AND outcome = 'succeeded' AND retry_count = 0;
    IF v_count <> 1 THEN
        RAISE EXCEPTION 'remediation_history 行数=% (期待 1)', v_count;
    END IF;

    -- cooldown 行 upsert 验证 (succeeded → expires_at > now())
    SELECT expires_at INTO v_expires_at FROM remediation_cooldowns
        WHERE action_id = 'disk-space-low';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'remediation_cooldowns(disk-space-low) 未写入';
    END IF;
    IF v_expires_at <= now() THEN
        RAISE EXCEPTION 'cooldown expires_at=% 已经过期', v_expires_at;
    END IF;

    RAISE NOTICE 'PASS: t_record_success (id=%)', v_id;
END;
$$;
ROLLBACK TO SAVEPOINT t_record_success;

-- =============================================================================
-- Phase 2: remediation_record_execution — 失败路径 (failed → 不更新 cooldown)
-- =============================================================================
SAVEPOINT t_record_failure;
DO $$
DECLARE
    v_succ BOOLEAN; v_id BIGINT; v_err TEXT;
    v_cooldown_count INT;
BEGIN
    SELECT success, history_id, error_msg
        INTO v_succ, v_id, v_err
        FROM remediation_record_execution(
            'service-down', 'ServiceDown', 'failed', 2,
            'step 0: command not found', 300
        );
    IF NOT v_succ THEN
        RAISE EXCEPTION 'record_execution(failed): success=false, err=%', v_err;
    END IF;

    -- failed → 不应写 cooldown
    SELECT count(*) INTO v_cooldown_count FROM remediation_cooldowns
        WHERE action_id = 'service-down';
    IF v_cooldown_count <> 0 THEN
        RAISE EXCEPTION 'failed 应不更新 cooldown, 但 count=%', v_cooldown_count;
    END IF;

    -- history 行写入验证 (retry_count=2, error_msg 非空)
    PERFORM 1 FROM remediation_history
        WHERE id = v_id AND retry_count = 2
          AND outcome = 'failed'
          AND error_msg LIKE 'step 0:%';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'remediation_history (failed) 行字段异常';
    END IF;

    RAISE NOTICE 'PASS: t_record_failure';
END;
$$;
ROLLBACK TO SAVEPOINT t_record_failure;

-- =============================================================================
-- Phase 3: remediation_record_execution — 幂等 cooldown upsert
-- =============================================================================
SAVEPOINT t_record_cooldown_idempotent;
DO $$
DECLARE
    v_first_expires TIMESTAMPTZ;
    v_second_expires TIMESTAMPTZ;
    v_count INT;
BEGIN
    -- 第一次 succeeded → 写 cooldown
    PERFORM remediation_record_execution(
        'slo-burn-fast', 'SLIBurnRateFast', 'succeeded', 0, NULL, 60
    );
    SELECT expires_at INTO v_first_expires FROM remediation_cooldowns
        WHERE action_id = 'slo-burn-fast';

    -- 等 1s 让 expires_at 不同 (TIMESTAMPTZ 秒精度)
    PERFORM pg_sleep(1);

    -- 第二次 succeeded → upsert, expires_at 应该被更新
    PERFORM remediation_record_execution(
        'slo-burn-fast', 'SLIBurnRateFast', 'succeeded', 0, NULL, 600
    );
    SELECT expires_at INTO v_second_expires FROM remediation_cooldowns
        WHERE action_id = 'slo-burn-fast';

    -- cooldown 行应该只有 1 条 (PK 唯一)
    SELECT count(*) INTO v_count FROM remediation_cooldowns
        WHERE action_id = 'slo-burn-fast';
    IF v_count <> 1 THEN
        RAISE EXCEPTION 'cooldown upsert 后应只有 1 条, count=%', v_count;
    END IF;

    IF v_second_expires <= v_first_expires THEN
        RAISE EXCEPTION 'cooldown upsert 未更新: first=%, second=%',
            v_first_expires, v_second_expires;
    END IF;

    RAISE NOTICE 'PASS: t_record_cooldown_idempotent';
END;
$$;
ROLLBACK TO SAVEPOINT t_record_cooldown_idempotent;

-- =============================================================================
-- Phase 4: remediation_record_execution — 参数校验失败
-- =============================================================================
SAVEPOINT t_record_invalid;
DO $$
DECLARE
    v_succ BOOLEAN; v_id BIGINT; v_err TEXT;
BEGIN
    -- 空 action_id
    SELECT success, history_id, error_msg
        INTO v_succ, v_id, v_err
        FROM remediation_record_execution(
            '', 'X', 'succeeded', 0, NULL, 300
        );
    IF v_succ THEN
        RAISE EXCEPTION '空 action_id 应失败';
    END IF;
    IF v_err IS NULL OR v_err NOT LIKE '%action_id 必填%' THEN
        RAISE EXCEPTION '空 action_id 错误信息异常: %', v_err;
    END IF;

    -- 非法 outcome
    SELECT success, history_id, error_msg
        INTO v_succ, v_id, v_err
        FROM remediation_record_execution(
            'a', 'X', 'bogus', 0, NULL, 300
        );
    IF v_succ THEN
        RAISE EXCEPTION '非法 outcome 应失败';
    END IF;
    IF v_err NOT LIKE '%bogus%' THEN
        RAISE EXCEPTION '非法 outcome 错误信息异常: %', v_err;
    END IF;

    -- 负 retry_count
    SELECT success, history_id, error_msg
        INTO v_succ, v_id, v_err
        FROM remediation_record_execution(
            'a', 'X', 'succeeded', -1, NULL, 300
        );
    IF v_succ THEN
        RAISE EXCEPTION '负 retry_count 应失败';
    END IF;

    -- 0 cooldown_seconds
    SELECT success, history_id, error_msg
        INTO v_succ, v_id, v_err
        FROM remediation_record_execution(
            'a', 'X', 'succeeded', 0, NULL, 0
        );
    IF v_succ THEN
        RAISE EXCEPTION '0 cooldown_seconds 应失败';
    END IF;

    RAISE NOTICE 'PASS: t_record_invalid';
END;
$$;
ROLLBACK TO SAVEPOINT t_record_invalid;

-- =============================================================================
-- Phase 5: remediation_check_cooldown — true 当未过期
-- =============================================================================
SAVEPOINT t_check_cooldown_active;
DO $$
DECLARE
    v_in_cooldown BOOLEAN;
BEGIN
    PERFORM remediation_record_execution(
        'disk-low', 'DiskSpaceFillingFast', 'succeeded', 0, NULL, 60
    );
    v_in_cooldown := remediation_check_cooldown('disk-low');
    IF NOT v_in_cooldown THEN
        RAISE EXCEPTION 'cooldown 写入后 check 应返回 true';
    END IF;

    RAISE NOTICE 'PASS: t_check_cooldown_active';
END;
$$;
ROLLBACK TO SAVEPOINT t_check_cooldown_active;

-- =============================================================================
-- Phase 6: remediation_check_cooldown — false 当未记录 / 已过期
-- =============================================================================
SAVEPOINT t_check_cooldown_inactive;
DO $$
DECLARE
    v_in_cooldown BOOLEAN;
BEGIN
    -- 未记录的 action
    v_in_cooldown := remediation_check_cooldown('never-executed-action');
    IF v_in_cooldown THEN
        RAISE EXCEPTION '未记录 action 应返回 false';
    END IF;

    -- 空 / NULL
    v_in_cooldown := remediation_check_cooldown('');
    IF v_in_cooldown THEN
        RAISE EXCEPTION '空 action_id 应返回 false';
    END IF;
    v_in_cooldown := remediation_check_cooldown(NULL::VARCHAR);
    IF v_in_cooldown THEN
        RAISE EXCEPTION 'NULL action_id 应返回 false';
    END IF;

    RAISE NOTICE 'PASS: t_check_cooldown_inactive';
END;
$$;
ROLLBACK TO SAVEPOINT t_check_cooldown_inactive;

-- =============================================================================
-- Phase 7: CHECK 约束 — outcome 拒绝非法值
-- =============================================================================
SAVEPOINT t_outcome_chk;
DO $$
DECLARE
    v_count INT;
BEGIN
    -- 先写一行 (绕过存过, 直接 INSERT)
    INSERT INTO remediation_history (
        action_id, alert_name, outcome, retry_count
    ) VALUES ('test', 'Test', 'succeeded', 0);

    -- 然后尝试非法 outcome
    BEGIN
        INSERT INTO remediation_history (
            action_id, alert_name, outcome, retry_count
        ) VALUES ('test', 'Test', 'BOGUS_OUTCOME', 0);
        RAISE EXCEPTION 'CHECK 约束未拒绝 BOGUS_OUTCOME';
    EXCEPTION WHEN check_violation THEN
        -- expected
        NULL;
    END;

    SELECT count(*) INTO v_count FROM remediation_history
        WHERE outcome = 'BOGUS_OUTCOME';
    IF v_count <> 0 THEN
        RAISE EXCEPTION 'CHECK 约束失败的行居然写入了: %', v_count;
    END IF;

    RAISE NOTICE 'PASS: t_outcome_chk';
END;
$$;
ROLLBACK TO SAVEPOINT t_outcome_chk;

ROLLBACK;
