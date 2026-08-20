# M-14 模块注册与生命周期（Module Registry & Lifecycle）

> **ドキュメントID**：DOC-MOD-014
> **文書分類**：モジュール別設計書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/04-atomic-deployment.md`（DOC-ARCH-005）
> **下位文書**：`docs/api/admin-modules.md`（DOC-API-004）
> **関連文書**：`docs/modules/M-06`（DOC-MOD-006）、`docs/modules/M-13`（DOC-MOD-013）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 7 章
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（モジュール登録 + ホットスワップ） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 需求来源
2. 基本设计
3. 詳細设计
4. 验收要点
5. 用語集
6. 参考文献

---

## 1. 需求来源（要件定義書）

### 1.1 涉及来源

- 上一级需求：[DOC-ARCH-005 §4 原子化部署、§7 热插拔协议](../architecture/04-atomic-deployment.md)
- 隐含需求：F-17 多租户隔离（部署操作必须带 tenant_id 上下文）

### 1.2 NF タグ付き要求

- 原子化部署：每个模块独立升级，业务无中断 [NF-AVA]【必須】
- 模块注册：单实例写入原子性 [NF-SEC]【必須】
- 版本回滚：失败 30s 内自动回滚 [NF-AVA]【必須】
- 热插拔：全过程 ≤ 60s [NF-OPS]【必須】

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/04 §3 总体架构](../architecture/04-atomic-deployment.md) 中的"管理平面"层，是 M-13 API Gateway 管理路由（`/api/v1/admin/modules/*`）的后端实现。

### 2.2 核心职责

- 模块清单（Manifest）解析、校验、持久化
- 模块状态机推进（Discovered → Registered → ... → Unloaded）
- 版本管理与多版本并存
- 升级策略执行（rolling / blue-green / canary / recreate）
- 与 M-13 路由表联动（注册/注销路由）
- 与 M-15 事件总线联动（发 `module.*` 事件）

### 2.3 涉及表

| 表 | 用途 | 詳細位置 |
|---|---|---|
| `module_registry` | 模块清单 | [M-10 §10 扩展](../modules/M-10-tenant-middleware.md) |
| `module_version` | 模块多版本 | 同上 |
| `module_instance` | 各节点上的实例 | 同上 |
| `module_upgrade_plan` | 升级计划（灰度参数等） | 同上 |

## 3. 詳細设计（詳細設計書）

### 3.1 核心 Trait

```rust
#[async_trait]
pub trait ModuleRegistry: Send + Sync {
    /// 注册新模块（PL/pgSQL 存过 register_module）
    async fn register(&self, manifest: ModuleManifest) -> Result<ModuleId, RegistryError>;

    /// 列出已注册模块
    async fn list(&self, filter: &ModuleFilter) -> Result<Vec<ModuleSummary>, RegistryError>;

    /// 触发升级（按策略）
    async fn upgrade(
        &self,
        module_id: ModuleId,
        to_version: SemVer,
        strategy: UpgradeStrategy,
    ) -> Result<UpgradePlanId, RegistryError>;

    /// 升级进度查询
    async fn upgrade_progress(&self, plan_id: UpgradePlanId) -> Result<UpgradeProgress, RegistryError>;

    /// 中止升级
    async fn abort_upgrade(&self, plan_id: UpgradePlanId) -> Result<(), RegistryError>;

    /// 回滚到上一版本
    async fn rollback(&self, module_id: ModuleId) -> Result<UpgradePlanId, RegistryError>;
}
```

### 3.2 状态机（每个版本实例独立状态）

```
Discovered → Registered → Downloading → Verifying → Loaded → Activating
   ↓          ↓             ↓            ↓          ↓
 Failed    Rejected     DownloadFailed  BadHash   InitFailed
   ↑                                                    ↓
   └────────────── 重试/卸载 ←─── Active ←──── Healthy/Unhealthy
                                  ↓
                                Draining
                                  ↓
                                Unloading
                                  ↓
                                Unloaded
```

每个状态转移均经 PL/pgSQL 存过 `register_module_state_transition()` 写审计 + 触发 `module.state_changed` 事件。

### 3.3 升级编排

```rust
pub async fn execute_rolling_upgrade(plan: UpgradePlan) -> Result<(), UpgradeError> {
    let nodes = cluster.healthy_nodes_with_module(plan.module_id).await?;
    
    for batch in nodes.chunks(plan.batch_size) {  // 默认 1 个节点/批
        // 1. 在每节点上下载新版本
        for node in batch {
            artifact_pull(node, plan.target_version).await?;
        }
        
        // 2. 每节点依次执行：load → activate → drain old → wait inflight=0
        for node in batch {
            // 新版本激活
            module_registry.activate(node, plan.target_version).await?;
            // 旧版本 drain
            module_registry.drain(node, plan.current_version).await?;
            // 等待 inflight=0（默认超时 30s）
            cluster.wait_inflight_zero(node, plan.module_id, plan.current_version, Duration::from_secs(30)).await?;
            // 旧版本卸载
            module_registry.unload(node, plan.current_version).await?;
        }
        
        // 3. 批次间健康检查
        if !health_check_window_pass(plan, Duration::from_secs(60)).await? {
            return Err(UpgradeError::HealthCheckFailed);
        }
    }
    Ok(())
}
```

### 3.4 PL/pgSQL 存过：register_module

```sql
CREATE OR REPLACE FUNCTION register_module(
    p_module_id    TEXT,
    p_version      TEXT,
    p_manifest     JSONB,
    p_artifact_url TEXT,
    p_artifact_sha256 TEXT
) RETURNS TABLE(success BOOLEAN, module_instance_id UUID, error_msg TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_manifest_json JSONB;
    v_semver TEXT;
    v_existing_id UUID;
    v_new_id UUID;
BEGIN
    -- 1. 校验 manifest 必填字段
    IF p_manifest->'meta'->>'module_id' IS NULL THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, 'manifest.meta.module_id 必填';
    END IF;
    IF p_manifest->'meta'->>'version' IS NULL THEN
        RETURN QUERY SELECT FALSE, NULL::UUID, 'manifest.meta.version 必填';
    END IF;
    
    v_semver := p_manifest->'meta'->>'version';
    
    -- 2. 幂等性检查：同 module_id+version 不重复
    SELECT id INTO v_existing_id
        FROM module_registry
        WHERE module_id = p_module_id AND version = v_semver;
    IF FOUND THEN
        RETURN QUERY SELECT TRUE, v_existing_id, NULL::TEXT;
        RETURN;
    END IF;
    
    -- 3. 插入新版本
    INSERT INTO module_registry (
        id, module_id, version, manifest, artifact_url, artifact_sha256,
        state, registered_at, registered_by
    ) VALUES (
        gen_random_uuid(), p_module_id, v_semver, p_manifest,
        p_artifact_url, p_artifact_sha256,
        'Registered', now(), current_setting('app.current_user_id', true)::UUID
    )
    RETURNING id INTO v_new_id;
    
    -- 4. 触发 module.registered 事件（M-15 事件总线订阅）
    PERFORM append_event('module.registered', jsonb_build_object(
        'module_id', p_module_id,
        'version', v_semver,
        'instance_id', v_new_id
    ), current_setting('app.current_tenant', true)::UUID);
    
    RETURN QUERY SELECT TRUE, v_new_id, NULL::TEXT;
END;
$$;
```

### 3.5 PL/pgSQL 存过：atomic_module_swap

```sql
-- 在事务内原子地完成"双写元数据 + 切 active 标记 + 写历史"
CREATE OR REPLACE FUNCTION atomic_module_swap(
    p_module_id    TEXT,
    p_from_version TEXT,
    p_to_version   TEXT
) RETURNS TABLE(success BOOLEAN, error_msg TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_from_id UUID;
    v_to_id   UUID;
BEGIN
    -- 锁：同一 module_id 串行化
    PERFORM pg_advisory_xact_lock(hashtext('module_swap:' || p_module_id));
    
    -- 1. 校验两边都存在
    SELECT id INTO v_from_id FROM module_registry
        WHERE module_id = p_module_id AND version = p_from_version;
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'from_version not found';
        RETURN;
    END IF;
    SELECT id INTO v_to_id FROM module_registry
        WHERE module_id = p_module_id AND version = p_to_version;
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'to_version not found';
        RETURN;
    END IF;
    
    -- 2. 双写：同时标记 from=inactive 与 to=active（在同一事务中）
    UPDATE module_registry SET active = FALSE, retired_at = now()
        WHERE id = v_from_id;
    UPDATE module_registry SET active = TRUE, activated_at = now()
        WHERE id = v_to_id;
    
    -- 3. 写一条 module_upgrade_history
    INSERT INTO module_upgrade_history (
        module_id, from_version, to_version, swapped_at, swapped_by
    ) VALUES (
        p_module_id, p_from_version, p_to_version, now(),
        current_setting('app.current_user_id', true)::UUID
    );
    
    -- 4. 事件
    PERFORM append_event('module.swapped', jsonb_build_object(
        'module_id', p_module_id, 'from', p_from_version, 'to', p_to_version
    ), current_setting('app.current_tenant', true)::UUID);
    
    RETURN QUERY SELECT TRUE, NULL::TEXT;
END;
$$;
```

### 3.6 兼容性检查

- 同一 module_id 多版本：仅一个 active，其余 inactive（active 列唯一约束）
- 依赖关系：被依赖模块未达最低版本时拒绝激活
- 状态机非法转移：抛出 `InvalidStateTransition` 错误码

### 3.7 与 M-13 路由联动

`module_registry.activate()` 完成后调用 M-13 的 `register_routes(module_id, version, routes)`，将 Manifest 中声明的路由加入负载均衡池。

## 4. 验收要点

1. **单模块升级零中断**：通过 rolling 策略升级 m01-acquisition 到 1.5.0，业务 P95 延迟波动 < 5%。 [NF-AVA]【必須】
2. **回滚速度**：升级失败后 30s 内自动回滚到上一版本。 [NF-AVA]【必須】
3. **状态机完整性**：非法状态转移（如 Loaded → Active 跳过 Activating）被拒绝。 [NF-SEC]【必須】
4. **PL/pgSQL 原子性**：并发 `atomic_module_swap` 调用在同 module_id 下串行化执行，无中间可见态。 [NF-SEC]【必須】
5. **依赖检查**：激活需要 m03 ≥ 1.0 的模块，DB 中 m03=0.9 时激活被拒绝。 [NF-OPS]【必須】

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 模块注册 | 将模块元数据持久化 | §1 |
| 状态机 | 模块生命周期状态转移 | §3.2 |
| 升级编排 | 多副本分批替换 | §3.3 |
| 双写 | 同一事务内 from/to 元数据并存 | §3.5 |
| Manifest | 模块清单 | §3.1 |
| 灰度策略 | Canary、按比例 | §3.1 |
| Rolling Update | 滚动升级 | §3.3 |
| 兼容性检查 | 依赖版本与契约匹配 | §3.6 |
| register_module | PL/pgSQL 存过 | §3.4 |
| atomic_module_swap | PL/pgSQL 存过 | §3.5 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. PostgreSQL Global Development Group「PostgreSQL Documentation — PL/pgSQL」
4. SemVer 公式「Semantic Versioning 2.0.0」
5. Ada プロジェクトチーム「原子化部署アーキテクチャ v1.0.0」、2026-08-19（[DOC-ARCH-005](../architecture/04-atomic-deployment.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
