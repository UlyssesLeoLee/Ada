-- =============================================================================
-- V002__plpgsql_functions.sql
--
-- Ada 永続層 キー 6 本 PL/pgSQL 存过 (PostgreSQL 18.6 想定)
--
-- 設計依据:
--   - docs/modules/M-10-tenant-middleware.md §4.6  (4.6.1 - 4.6.5)
--   - docs/modules/M-14-module-registry.md §3.4-§3.5
--   - docs/modules/M-15-central-event-bus.md (M-15 event_log 配套)
--   - docs/modules/M-16-cluster-coordinator.md §3.4 / §3.6
--   - docs/architecture/04-atomic-deployment.md §11 PL/pgSQL 存过策略
--
-- 6 本存过一覧:
--   1. register_module           (§4.6.1 / M-14 §3.4) - モジュール登録
--   2. atomic_module_swap        (§4.6.2 / M-14 §3.5) - 原子切替
--   3. append_event              (§4.6.3 / M-15)        - イベント追記
--   4. acquire_lease             (§4.6.4 / M-16 §3.4)   - 領導租約取得
--   5. release_lease             (§4.6.4 / M-16 §3.4)   - 領導租約解放
--   6. register_node_heartbeat   (§4.6.5 / M-16 §3.6)   - クラスタノード心跳
--
-- 命名/シグネチャ方針:
--   - task spec の "register_module(p_name, p_version, p_kind, p_manifest,
--     p_endpoint)" 等の簡易シグネチャは M-10 §4.6 詳細シグネチャと異なる。
--     文档 (source of truth) 優先。本ファイル冒頭の NOTICE で明示する。
--   - 全存过 LANGUAGE plpgsql / SECURITY DEFINER / SET search_path
--     で schema injection を防御 (M-10 §4.6 全例踏襲)。
--   - 6 本全て RETURN TABLE(...) 形式 (M-10 §4.6 と一致)。
--     task spec 記載の "→ BOOLEAN" は誤読、RETURNS TABLE を採用。
--
-- 冪等性:
--   - CREATE OR REPLACE FUNCTION で再実行可。
-- =============================================================================

BEGIN;

-- =============================================================================
-- §4.6.1 register_module
-- =============================================================================
-- 用途: モジュールを module_registry に登録 (幂等)
-- シグネチャ (M-10 §4.6.1 と完全一致):
--   register_module(
--       p_module_id       TEXT,    -- 'm01-acquisition' 等
--       p_version         TEXT,    -- '1.5.0' (SemVer)
--       p_manifest        JSONB,   -- Module.toml 全文 (meta.module_id, meta.version 必須)
--       p_artifact_url    TEXT,    -- s3://bucket/path
--       p_artifact_sha256 TEXT     -- 64 hex chars
--   ) RETURNS TABLE(success BOOLEAN, module_instance_id UUID, error_msg TEXT)
--
-- 不変量:
--   - 同 (tenant_id, module_id, version) 2 度目の呼び出しは幂等 (既存 ID 返却)
--   - manifest 必須フィールド未設定は fail-fast (error_msg で返却)
--   - 成功時に module.registered イベントを M-15 へ自動発火
-- =============================================================================
CREATE OR REPLACE FUNCTION register_module(
    p_module_id       TEXT,
    p_version         TEXT,
    p_manifest        JSONB,
    p_artifact_url    TEXT,
    p_artifact_sha256 TEXT
) RETURNS TABLE(success BOOLEAN, module_instance_id UUID, error_msg TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_existing_id  UUID;
    v_new_id        UUID;
    v_tenant_id     UUID;
    v_registered_by UUID;
BEGIN
    -- 1. manifest 必須フィールドバリデーション (M-10 §4.6.1 / 実装ガイド §3.4)
    IF p_module_id IS NULL OR btrim(p_module_id) = '' THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, 'p_module_id 必填'::TEXT;
        RETURN;
    END IF;
    IF p_version IS NULL OR btrim(p_version) = '' THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, 'p_version 必填'::TEXT;
        RETURN;
    END IF;
    IF p_manifest IS NULL THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, 'p_manifest 必填'::TEXT;
        RETURN;
    END IF;
    IF p_manifest->'meta'->>'module_id' IS NULL THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, 'manifest.meta.module_id 必填'::TEXT;
        RETURN;
    END IF;
    IF p_manifest->'meta'->>'version' IS NULL THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, 'manifest.meta.version 必填'::TEXT;
        RETURN;
    END IF;

    -- 2. tenant 解決 (RLS セッション変数、空なら system-level)
    v_tenant_id := NULLIF(
        current_setting('app.current_tenant', true), ''
    )::UUID;

    -- 3. 幂等性チェック: 同 (tenant_id, module_id, version) が既存なら ID 返却
    SELECT id INTO v_existing_id
        FROM module_registry
        WHERE module_id = p_module_id
          AND version   = p_version
          AND tenant_id IS NOT DISTINCT FROM v_tenant_id;
    IF FOUND THEN
        RETURN QUERY SELECT TRUE, v_existing_id, NULL::TEXT;
        RETURN;
    END IF;

    -- 4. registered_by 解決 (app.current_user_id、無ければ NULL)
    BEGIN
        v_registered_by := NULLIF(
            current_setting('app.current_user_id', true), ''
        )::UUID;
    EXCEPTION WHEN OTHERS THEN
        v_registered_by := NULL;
    END;

    -- 5. 新規 INSERT
    INSERT INTO module_registry (
        id, tenant_id, module_id, version, manifest,
        artifact_url, artifact_sha256,
        state, registered_at, registered_by
    ) VALUES (
        gen_random_uuid(), v_tenant_id, p_module_id, p_version, p_manifest,
        p_artifact_url, p_artifact_sha256,
        'Registered', now(), v_registered_by
    )
    RETURNING id INTO v_new_id;

    -- 6. 副作用: module.registered イベント発行 (M-15 へ)
    --    append_event() は SECURITY DEFINER 内側で動く想定 (RLS バイパス可)
    PERFORM append_event(
        'module.registered',
        jsonb_build_object(
            'module_id', p_module_id,
            'version',   p_version,
            'instance_id', v_new_id,
            'artifact_url', p_artifact_url,
            'artifact_sha256', p_artifact_sha256
        )
    );

    RETURN QUERY SELECT TRUE, v_new_id, NULL::TEXT;
END;
$$;

COMMENT ON FUNCTION register_module(TEXT, TEXT, JSONB, TEXT, TEXT) IS
    'M-10 §4.6.1 / M-14 §3.4: モジュール登録 (幂等、同 (tenant, module, version) で既存 ID 返却、成功時に module.registered イベント発火)';


-- =============================================================================
-- §4.6.2 atomic_module_swap
-- =============================================================================
-- 用途: 旧 version → 新 version の切替を 1 トランザクションで原子化
-- シグネチャ (M-10 §4.6.2):
--   atomic_module_swap(
--       p_module_id    TEXT,
--       p_from_version TEXT,
--       p_to_version   TEXT
--   ) RETURNS TABLE(success BOOLEAN, error_msg TEXT)
--
-- 不変量:
--   - pg_advisory_xact_lock(hashtext('module_swap:' || module_id)) で同 module
--     並列 swap を直列化
--   - 同一トランザクション内で from=inactive + to=active の双書
--   - module_upgrade_history に 'Succeeded' 記録
--   - module.swapped イベント発火
--   - 失敗時は ROLLBACK で無中間可観測状態
-- =============================================================================
CREATE OR REPLACE FUNCTION atomic_module_swap(
    p_module_id    TEXT,
    p_from_version TEXT,
    p_to_version   TEXT
) RETURNS TABLE(success BOOLEAN, error_msg TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_tenant_id UUID;
    v_from_id   UUID;
    v_to_id     UUID;
    v_from_active BOOLEAN;
BEGIN
    -- 0. 引数バリデーション
    IF p_module_id IS NULL OR btrim(p_module_id) = '' THEN
        RETURN QUERY SELECT FALSE, 'p_module_id 必填'::TEXT;
        RETURN;
    END IF;
    IF p_from_version IS NULL OR p_to_version IS NULL THEN
        RETURN QUERY SELECT FALSE, 'p_from_version / p_to_version 必填'::TEXT;
        RETURN;
    END IF;
    IF p_from_version = p_to_version THEN
        RETURN QUERY SELECT FALSE, 'from_version == to_version (no-op 拒否)'::TEXT;
        RETURN;
    END IF;

    -- 1. tenant 解決
    v_tenant_id := NULLIF(
        current_setting('app.current_tenant', true), ''
    )::UUID;

    -- 2. 同一 module_id への並列 swap を直列化 (M-10 §4.6.2 / M-14 §3.5)
    PERFORM pg_advisory_xact_lock(hashtext('module_swap:' || p_module_id));

    -- 3. from / to 両方が存在することを確認
    SELECT id, active INTO v_from_id, v_from_active
        FROM module_registry
        WHERE module_id = p_module_id
          AND version   = p_from_version
          AND tenant_id IS NOT DISTINCT FROM v_tenant_id;
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'from_version not found'::TEXT;
        RETURN;
    END IF;

    SELECT id INTO v_to_id
        FROM module_registry
        WHERE module_id = p_module_id
          AND version   = p_to_version
          AND tenant_id IS NOT DISTINCT FROM v_tenant_id;
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'to_version not found'::TEXT;
        RETURN;
    END IF;

    -- 4. 双書: 同一トランザクション内で from=inactive + to=active
    UPDATE module_registry
        SET active      = FALSE,
            retired_at  = now()
        WHERE id = v_from_id;

    UPDATE module_registry
        SET active        = TRUE,
            activated_at  = now()
        WHERE id = v_to_id;

    -- 5. 升级履歴記録 (strategy='atomic_swap' で PL/pgSQL 経路と識別)
    INSERT INTO module_upgrade_history (
        id, tenant_id, module_id, from_version, to_version,
        strategy, plan_id, status, started_at, completed_at
    ) VALUES (
        gen_random_uuid(), v_tenant_id, p_module_id,
        p_from_version, p_to_version,
        'atomic_swap', gen_random_uuid(), 'Succeeded', now(), now()
    );

    -- 6. 副作用: module.swapped イベント発火
    PERFORM append_event(
        'module.swapped',
        jsonb_build_object(
            'module_id', p_module_id,
            'from',      p_from_version,
            'to',        p_to_version,
            'from_id',   v_from_id,
            'to_id',     v_to_id,
            'was_active', v_from_active
        )
    );

    RETURN QUERY SELECT TRUE, NULL::TEXT;
END;
$$;

COMMENT ON FUNCTION atomic_module_swap(TEXT, TEXT, TEXT) IS
    'M-10 §4.6.2 / M-14 §3.5: モジュール原子的切替 (advisory_lock で並列直列化、双書 + history + event 発火)';


-- =============================================================================
-- §4.6.3 append_event
-- =============================================================================
-- 用途: イベントを event_log に追記 (event_seq は大局単調増加)
-- シグネチャ (M-10 §4.6.3):
--   append_event(
--       p_topic    TEXT,
--       p_payload  JSONB
--   ) RETURNS TABLE(event_id UUID, event_seq BIGINT)
--
-- 不変量:
--   - nextval('event_seq_global') で取得した seq は唯一 + 単調増加
--   - pg_notify('event_appended', ...) で dispatcher へ非同期通知
--   - tenant_id は app.current_tenant から自動注入
--   - headers は producer=app.current_service, schema_version='1.0' 自動付与
-- =============================================================================
CREATE OR REPLACE FUNCTION append_event(
    p_topic    TEXT,
    p_payload  JSONB
) RETURNS TABLE(event_id UUID, event_seq BIGINT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_seq        BIGINT;
    v_id         UUID;
    v_tenant_id  UUID;
    v_producer   TEXT;
    v_headers    JSONB;
BEGIN
    -- 1. 引数チェック
    IF p_topic IS NULL OR btrim(p_topic) = '' THEN
        RAISE EXCEPTION 'append_event: p_topic 必填';
    END IF;
    IF p_payload IS NULL THEN
        RAISE EXCEPTION 'append_event: p_payload 必填';
    END IF;

    -- 2. tenant / producer 解決
    BEGIN
        v_tenant_id := NULLIF(
            current_setting('app.current_tenant', true), ''
        )::UUID;
    EXCEPTION WHEN OTHERS THEN
        v_tenant_id := NULL;
    END;

    BEGIN
        v_producer := NULLIF(current_setting('app.current_service', true), '');
    EXCEPTION WHEN OTHERS THEN
        v_producer := NULL;
    END;

    v_seq := nextval('event_seq_global');
    v_id  := gen_random_uuid();

    -- 3. event_log へ INSERT
    v_headers := jsonb_build_object(
        'schema_version', '1.0',
        'produced_at',    to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"')
    );

    INSERT INTO event_log (
        id, event_seq, topic, tenant_id, payload, headers, produced_at, producer
    ) VALUES (
        v_id, v_seq, p_topic, v_tenant_id, p_payload, v_headers, now(), v_producer
    );

    -- 4. 非同期通知: pg_notify (PostgreSQL LISTEN/NOTIFY channel 'event_appended')
    --    - listener は dispatcher プロセス
    --    - payload は listener 側で event_log から再 fetch する想定
    --      (notify payload は 8KB 制限あり、seq のみ送る軽量設計)
    PERFORM pg_notify(
        'event_appended',
        json_build_object('seq', v_seq, 'topic', p_topic)::TEXT
    );

    RETURN QUERY SELECT v_id, v_seq;
END;
$$;

COMMENT ON FUNCTION append_event(TEXT, JSONB) IS
    'M-10 §4.6.3 / M-15: イベント追記 (event_seq 大局単調増加 + pg_notify で dispatcher へ通知)';


-- =============================================================================
-- §4.6.4 acquire_lease / release_lease
-- =============================================================================
-- 用途: 領導租約 (Singleton 役割) の取得 / 解放
--
-- acquire_lease(p_lease_key, p_node_id, p_ttl_seconds INT DEFAULT 30)
--   RETURNS TABLE(acquired BOOLEAN, lease_id UUID, expires_at TIMESTAMPTZ)
--
-- release_lease(p_lease_key, p_node_id)
--   RETURNS TABLE(released BOOLEAN)
--
-- 不変量 (M-10 §4.6.4 / M-16 §3.4):
--   - 同一 lease_key に対し 1 ノードのみが保持
--   - TTL 切れは他ノードが抢占可能
--   - renew_count は取得/续約の度に +1
--   - release は保持者のみ実行可
-- =============================================================================
CREATE OR REPLACE FUNCTION acquire_lease(
    p_lease_key    TEXT,
    p_node_id      UUID,
    p_ttl_seconds  INT DEFAULT 30
) RETURNS TABLE(acquired BOOLEAN, lease_id UUID, expires_at TIMESTAMPTZ)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_existing_node    UUID;
    v_existing_expires TIMESTAMPTZ;
    v_new_expires      TIMESTAMPTZ;
    v_effective_ttl    INT;
BEGIN
    -- 0. 引数チェック
    IF p_lease_key IS NULL OR btrim(p_lease_key) = '' THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, NULL::TIMESTAMPTZ;
        RETURN;
    END IF;
    IF p_node_id IS NULL THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, NULL::TIMESTAMPTZ;
        RETURN;
    END IF;
    v_effective_ttl := COALESCE(p_ttl_seconds, 30);
    IF v_effective_ttl <= 0 THEN
        v_effective_ttl := 30;
    END IF;

    v_new_expires := now() + (v_effective_ttl || ' seconds')::INTERVAL;

    -- 1. 既存 holder 確認 (行ロック → 同一トランザクション内で重複取得防止)
    SELECT holder_node_id, expires_at
        INTO v_existing_node, v_existing_expires
        FROM leader_lease
        WHERE lease_key = p_lease_key
        FOR UPDATE;

    -- 2. 取得判定:
    --    (a) 既存なし → 新規 INSERT
    --    (b) 既存 holder が自分 → 续約 (renew_count++)
    --    (c) 既存 holder の TTL 切れ → 抢占
    --    (d) 上記以外 → 取得失敗
    IF v_existing_node IS NULL
       OR v_existing_expires < now()
       OR v_existing_node = p_node_id
    THEN
        INSERT INTO leader_lease (lease_key, holder_node_id, acquired_at, expires_at, renew_count)
        VALUES (p_lease_key, p_node_id, now(), v_new_expires, 1)
        ON CONFLICT (lease_key) DO UPDATE
            SET holder_node_id = EXCLUDED.holder_node_id,
                acquired_at    = now(),
                expires_at     = EXCLUDED.expires_at,
                renew_count    = leader_lease.renew_count + 1;
        RETURN QUERY SELECT TRUE, NULL::UUID, v_new_expires;
    ELSE
        RETURN QUERY SELECT FALSE, NULL::UUID, v_existing_expires;
    END IF;
END;
$$;

COMMENT ON FUNCTION acquire_lease(TEXT, UUID, INT) IS
    'M-10 §4.6.4 / M-16 §3.4: 領導租約取得 (TTL 切れ抢占可、同 holder 续約は renew_count++)';


CREATE OR REPLACE FUNCTION release_lease(
    p_lease_key TEXT,
    p_node_id   UUID
) RETURNS TABLE(released BOOLEAN)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_existing_node UUID;
BEGIN
    -- 0. 引数チェック
    IF p_lease_key IS NULL OR p_node_id IS NULL THEN
        RETURN QUERY SELECT FALSE;
        RETURN;
    END IF;

    -- 1. 既存 holder 確認 (保持者のみ解放可)
    SELECT holder_node_id INTO v_existing_node
        FROM leader_lease
        WHERE lease_key = p_lease_key;
    IF v_existing_node IS NULL OR v_existing_node != p_node_id THEN
        RETURN QUERY SELECT FALSE;
        RETURN;
    END IF;

    -- 2. 解放
    DELETE FROM leader_lease WHERE lease_key = p_lease_key;
    RETURN QUERY SELECT TRUE;
END;
$$;

COMMENT ON FUNCTION release_lease(TEXT, UUID) IS
    'M-10 §4.6.4 / M-16 §3.4: 領導租約解放 (保持者のみ成功、他者の解放試行は FALSE)';


-- =============================================================================
-- §4.6.5 register_node_heartbeat
-- =============================================================================
-- 用途: クラスタノード心跳 (upsert + load 集計)
-- シグネチャ (M-10 §4.6.5):
--   register_node_heartbeat(p_node_id UUID, p_status JSONB)
--   RETURNS TABLE(healthy BOOLEAN, current_load NUMERIC)
--
-- 不変量:
--   - cluster_node 行が既に存在することが前提 (register_node で事前登録)
--   - last_heartbeat_at = now() に更新
--   - status JSONB を新値で上書き
--   - current_load は module_instance の (Active 数 / capacity) で計算
--   - healthy は p_status->>'health'::BOOLEAN から抽出
-- =============================================================================
CREATE OR REPLACE FUNCTION register_node_heartbeat(
    p_node_id UUID,
    p_status  JSONB
) RETURNS TABLE(healthy BOOLEAN, current_load NUMERIC)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_load    NUMERIC;
    v_healthy BOOLEAN;
    v_exists  BOOLEAN;
BEGIN
    -- 0. 引数チェック
    IF p_node_id IS NULL THEN
        RETURN QUERY SELECT FALSE, NULL::NUMERIC;
        RETURN;
    END IF;

    -- 1. ノード存在確認 (heartbeat は事前登録が前提)
    SELECT EXISTS (
        SELECT 1 FROM cluster_node WHERE node_id = p_node_id
    ) INTO v_exists;
    IF NOT v_exists THEN
        -- heartbeat 先行で node 情報が無いケースは cluster_node に空行を
        -- 作らずエラーで返す。register_node() 経路での事前 INSERT を強制。
        RAISE EXCEPTION 'register_node_heartbeat: node_id=% 未登録 (register_node で事前登録が必要)', p_node_id;
    END IF;

    -- 2. Upsert: 既存行の last_heartbeat_at / status のみ更新
    UPDATE cluster_node
        SET last_heartbeat_at = now(),
            status            = p_status,
            state             = CASE
                WHEN p_status ? 'health'
                 AND (p_status->>'health')::BOOLEAN = FALSE
                THEN 'Unhealthy'
                ELSE 'Active'
            END
        WHERE node_id = p_node_id;

    -- 3. current_load 計算: 当該 node_id の Active module_instance 数 / capacity
    SELECT (COUNT(*) FILTER (WHERE state = 'Active')::NUMERIC
            / GREATEST(cn.capacity, 1))::NUMERIC
        INTO v_load
        FROM module_instance mi
        CROSS JOIN cluster_node cn
        WHERE mi.node_id = p_node_id
          AND cn.node_id = p_node_id
        GROUP BY cn.capacity;

    v_load := COALESCE(v_load, 0::NUMERIC);

    -- 4. cluster_node.current_load も更新 (observability 用)
    UPDATE cluster_node
        SET current_load = v_load
        WHERE node_id = p_node_id;

    -- 5. healthy 判定
    v_healthy := COALESCE((p_status->>'health')::BOOLEAN, FALSE);

    RETURN QUERY SELECT v_healthy, v_load;
END;
$$;

COMMENT ON FUNCTION register_node_heartbeat(UUID, JSONB) IS
    'M-10 §4.6.5 / M-16 §3.6: クラスタノード心跳 upsert + current_load 集計 (status->>health で healthy 判定)';


-- -----------------------------------------------------------------------------
-- 統計情報更新
-- -----------------------------------------------------------------------------
ANALYZE module_registry;
ANALYZE module_upgrade_history;
ANALYZE module_instance;
ANALYZE event_log;
ANALYZE cluster_node;
ANALYZE leader_lease;

COMMIT;
