-- =============================================================================
-- V003__phase8_remediation.sql
--
-- observability Phase 8 Auto-remediation 治理表 + PL/pgSQL 存过.
--
-- 設計依据:
--   - docs/observability/11-phased-rollout.md §10 (Phase 8 scope)
--   - docs/observability/12-auto-remediation.md (architecture)
--   - db/Makefile (migrate 入口)
--
-- 含むオブジェクト:
--   - 2 テーブル:
--     * remediation_history   — 每次 action 执行的审计行 (one row per
--       attempt; the same action may produce multiple rows when
--       retried within a single webhook delivery).
--     * remediation_cooldowns — 同一 action_id 在 cooldown 窗口内不能
--       再被 evaluate + execute; this table is the durable source of
--       truth across process restarts and replicas.
--   - 2 存过:
--     * remediation_record_execution(action_id, alert_name, outcome,
--         retry_count, error_msg) — 幂等写 history.
--     * remediation_check_cooldown(action_id) → bool — 返回 true 当且
--       仅当 cooldown 仍生效. 由生产 wiring 调用; the in-memory
--       `MemoryStore` in `crates/ada-remediation/src/history.rs` is the
--       fast path.
--
-- Migration 命名:
--   - 任务描述写 "V006__phase8_remediation.sql", 但本仓 db/migrations/
--     实际只有 V001 + V002 (per `git log -- db/migrations`); V003 是
--     下一个合法槽位. 本文件 V003 命名, 与 db/Makefile 的 V* glob 匹配.
--
-- 冪等性:
--   - CREATE TABLE IF NOT EXISTS / CREATE OR REPLACE FUNCTION. V001/V002
--     不动 (per task spec "Phase 6 已完成的 PL/pgSQL 决议保护").
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- remediation_history
-- -----------------------------------------------------------------------------
-- 一行 = 一次执行 attempt. retries 会产生多行 (one per attempt).
-- 表设计依据:
--   - id BIGSERIAL: 单调递增, dashboard / history 端点按此分页.
--   - action_id / alert_name: 主索引 + 联合索引便于按维度查询.
--   - outcome CHECK: 'succeeded' | 'failed' | 'skipped'.
--   - retry_count: 0-based; the first attempt is 0, the second is 1, ...
--   - error_msg: NULL on success; 失败时记录 engine 返回的 step_results
--     摘要 (per action 第一条失败 step 的 message).
--   - executed_at: 写入时的 server time, default now() — 不接受 client
--     时钟输入, 避免时区漂移.
CREATE TABLE IF NOT EXISTS remediation_history (
    id            BIGSERIAL    PRIMARY KEY,
    action_id     VARCHAR(100) NOT NULL,
    alert_name    VARCHAR(200) NOT NULL,
    executed_at   TIMESTAMPTZ  NOT NULL DEFAULT now(),
    outcome       VARCHAR(20)  NOT NULL,
    retry_count   INT          NOT NULL DEFAULT 0,
    error_msg     TEXT,
    CONSTRAINT remediation_history_outcome_chk CHECK (outcome IN (
        'succeeded', 'failed', 'skipped'
    )),
    CONSTRAINT remediation_history_retry_nonneg_chk CHECK (retry_count >= 0)
);

-- 联合索引: (action_id, executed_at DESC) 用于
--   SELECT * FROM remediation_history
--   WHERE action_id = $1
--   ORDER BY executed_at DESC LIMIT 50;
-- 是 dashboard 90% 的查询形态.
CREATE INDEX IF NOT EXISTS remediation_history_action_time_idx
    ON remediation_history (action_id, executed_at DESC);

-- 联合索引: (alert_name, executed_at DESC) 用于 SLO burn rate 事后分析.
CREATE INDEX IF NOT EXISTS remediation_history_alert_time_idx
    ON remediation_history (alert_name, executed_at DESC);

-- -----------------------------------------------------------------------------
-- remediation_cooldowns
-- -----------------------------------------------------------------------------
-- 持久 cooldown 状态. last_executed_at + cooldown_seconds → expires_at.
-- 多个 replica 都读这张表, 所以它是 source of truth; in-memory
-- MemoryStore 是 fast path.
CREATE TABLE IF NOT EXISTS remediation_cooldowns (
    action_id          VARCHAR(100) PRIMARY KEY,
    last_executed_at   TIMESTAMPTZ  NOT NULL DEFAULT now(),
    cooldown_seconds   INT          NOT NULL,
    expires_at         TIMESTAMPTZ  NOT NULL,
    CONSTRAINT remediation_cooldowns_seconds_positive_chk CHECK (cooldown_seconds > 0)
);

-- expires_at 上的索引, 便于 housekeeping ("找所有已过期的 cooldowns").
CREATE INDEX IF NOT EXISTS remediation_cooldowns_expires_at_idx
    ON remediation_cooldowns (expires_at);

-- -----------------------------------------------------------------------------
-- remediation_record_execution
-- -----------------------------------------------------------------------------
-- 用法: SELECT remediation_record_execution(
--            'disk-space-low', 'DiskSpaceFillingFast', 'succeeded', 0, NULL);
--
-- 副作用:
--   1. INSERT INTO remediation_history 一行.
--   2. 如果 outcome = 'succeeded', 同步 upsert remediation_cooldowns 行
--      (cooldown_seconds 来自 action 表 / app 配置; 此存过接受
--      cooldown_seconds 入参由调用者提供). 失败 execution 不更新
--      cooldown (per 12-auto-remediation.md §4.2).
--
-- 幂等性:
--   - history 行不要求幂等 (BIGSERIAL 主键, 每次调用都新增).
--   - cooldowns 表 UPSERT 用 ON CONFLICT.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION remediation_record_execution(
    p_action_id        VARCHAR,
    p_alert_name       VARCHAR,
    p_outcome          VARCHAR,
    p_retry_count      INT,
    p_error_msg        TEXT,
    p_cooldown_seconds INT DEFAULT 300
) RETURNS TABLE(success BOOLEAN, history_id BIGINT, error_msg TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_new_id      BIGINT;
    v_expires_at  TIMESTAMPTZ;
BEGIN
    -- 0. 参数校验
    IF p_action_id IS NULL OR btrim(p_action_id) = '' THEN
        RETURN QUERY SELECT FALSE, NULL::BIGINT, 'p_action_id 必填'::TEXT;
        RETURN;
    END IF;
    IF p_outcome NOT IN ('succeeded', 'failed', 'skipped') THEN
        RETURN QUERY SELECT FALSE, NULL::BIGINT,
            format('p_outcome 非法: %s (expected succeeded|failed|skipped)', p_outcome)::TEXT;
        RETURN;
    END IF;
    IF p_retry_count < 0 THEN
        RETURN QUERY SELECT FALSE, NULL::BIGINT, 'p_retry_count 必须 >= 0'::TEXT;
        RETURN;
    END IF;
    IF p_cooldown_seconds IS NULL OR p_cooldown_seconds <= 0 THEN
        RETURN QUERY SELECT FALSE, NULL::BIGINT, 'p_cooldown_seconds 必须 > 0'::TEXT;
        RETURN;
    END IF;

    -- 1. 写 history
    INSERT INTO remediation_history (
        action_id, alert_name, outcome, retry_count, error_msg
    ) VALUES (
        p_action_id, p_alert_name, p_outcome, p_retry_count, p_error_msg
    )
    RETURNING id INTO v_new_id;

    -- 2. 仅在 succeeded 时 upsert cooldown (失败留 10s 短 cooldown 由
    --    application 层处理; 此存过尊重 application 传入的值, 因为
    --    只有 application 知道 action 实际的 cooldown 字段).
    IF p_outcome = 'succeeded' THEN
        v_expires_at := now() + make_interval(secs => p_cooldown_seconds);
        INSERT INTO remediation_cooldowns (
            action_id, last_executed_at, cooldown_seconds, expires_at
        ) VALUES (
            p_action_id, now(), p_cooldown_seconds, v_expires_at
        )
        ON CONFLICT (action_id) DO UPDATE
        SET last_executed_at = EXCLUDED.last_executed_at,
            cooldown_seconds = EXCLUDED.cooldown_seconds,
            expires_at       = EXCLUDED.expires_at;
    END IF;

    RETURN QUERY SELECT TRUE, v_new_id, NULL::TEXT;
END;
$$;

COMMENT ON FUNCTION remediation_record_execution(
    VARCHAR, VARCHAR, VARCHAR, INT, TEXT, INT
) IS
    'Phase 8 / 12-auto-remediation.md §4.2: 记录一次 action 执行 + 同步 cooldown (仅 succeeded). 幂等 history 写 + upsert cooldown. 默认 cooldown_seconds=300 (5min).';


-- -----------------------------------------------------------------------------
-- remediation_check_cooldown
-- -----------------------------------------------------------------------------
-- 用法: SELECT remediation_check_cooldown('disk-space-low');
--   → true  = 在 cooldown 窗口内, 不要再 evaluate.
--   → false = cooldown 已过期或从未执行过.
--
-- 重要: 此函数不删除已过期的行. Housekeeping (vacuum expired) 是
-- application 责任 (避免 PL/pgSQL 在 hot path 做 DELETE).
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION remediation_check_cooldown(
    p_action_id VARCHAR
) RETURNS BOOLEAN
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_expires_at TIMESTAMPTZ;
BEGIN
    IF p_action_id IS NULL OR btrim(p_action_id) = '' THEN
        RETURN FALSE;
    END IF;

    SELECT expires_at INTO v_expires_at
        FROM remediation_cooldowns
        WHERE action_id = p_action_id;
    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;
    RETURN v_expires_at > now();
END;
$$;

COMMENT ON FUNCTION remediation_check_cooldown(VARCHAR) IS
    'Phase 8 / 12-auto-remediation.md §4.1: 持久 cooldown 查询. 返回 true 表示仍在窗口内 (拒绝再 evaluate).';

COMMIT;
