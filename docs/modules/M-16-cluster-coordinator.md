# M-16 集群协调（Cluster Coordinator）

> **ドキュメントID**：DOC-MOD-016
> **文書分類**：モジュール別設計書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/04-atomic-deployment.md`（DOC-ARCH-005）
> **下位文書**：`docs/api/admin-cluster.md`（DOC-API-006）
> **関連文書**：`docs/modules/M-13`（DOC-MOD-013）、`docs/modules/M-14`（DOC-MOD-014）、`docs/modules/M-15`（DOC-MOD-015）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 7 章
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（サービス検出 + リーダー選出 + ハートビート） | Ada プロジェクトチーム | TBD | TBD |

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

- 上一级需求：[DOC-ARCH-005 §6 App 集群协调](../architecture/04-atomic-deployment.md)
- 既有能力：M-13 API Gateway 假设单机部署，需扩展支持多副本

### 1.2 NF 标签要求

- 心跳失联检测 ≤ 30s 自动摘除 [NF-AVA]【必須】
- 集群规模 100 节点线性扩展 [NF-PER]【必須】
- 节点时间同步偏差 ≤ 100ms [NF-OPS]【必須】
- 状态分片按 tenant_id hash 均匀分布 [NF-PER]【必須】

## 2. 基本设计（基本設計書）

### 2.1 架构位置

属于 [architecture/04 §3 总体架构](../architecture/04-atomic-deployment.md) 中的"管理平面"，是 M-13 API Gateway 实现多副本负载均衡与 leader 选举的基础。

### 2.2 核心职责

- 节点身份与注册（启动时）
- 心跳机制（周期性 5s）
- 健康检查（health check）
- 服务发现（其他节点查询）
- 领导选举（基于租约）
- 状态分片（按 tenant_id hash）
- 节点摘除（不健康自动）

### 2.3 涉及表

| 表 | 用途 | 詳細位置 |
|---|---|---|
| `cluster_node` | 集群节点清单 | [M-10 §10 扩展](../modules/M-10-tenant-middleware.md) |
| `leader_lease` | 领导租约 | 同上 |
| `node_health` | 健康检查历史（按时间窗口） | 同上 |
| `shard_assignment` | 状态分片映射 | 同上 |

## 3. 詳細设计（詳細設計書）

### 3.1 节点身份

```rust
pub struct NodeIdentity {
    pub node_id: Uuid,        // UUID v5(hostname + boot_time + nonce)
    pub hostname: String,
    pub advertised_addr: SocketAddr,
    pub labels: HashMap<String, String>,  // 如 {"zone": "us-east-1a", "role": "worker"}
    pub started_at: DateTime<Utc>,
    pub runtime_version: SemVer,
}
```

启动时：
1. 生成 `node_id`（确定性 + 随机性，避免重启冲突）
2. 解析 hostname / IP / 端口
3. 读取环境变量（`ADA_LABELS`、`ADA_RUNTIME_VERSION`）
4. 写 `cluster_node` 表（PL/pgSQL `register_node()` 存过）
5. 启动心跳任务

### 3.2 心跳机制

```rust
pub async fn heartbeat_loop(coord: Arc<ClusterCoordinator>) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        if let Err(e) = coord.send_heartbeat().await {
            warn!("heartbeat failed: {e}");
        }
    }
}

impl ClusterCoordinator {
    async fn send_heartbeat(&self) -> Result<(), CoordError> {
        // 收集本节点状态
        let status = NodeStatus {
            cpu_usage: self.metrics.cpu_usage(),
            memory_usage: self.metrics.memory_usage(),
            active_executions: self.metrics.active_count(),
            health: self.health_checker.overall(),
        };
        // PL/pgSQL register_node_heartbeat
        sqlx::query("SELECT register_node_heartbeat($1, $2)")
            .bind(self.node_id)
            .bind(sqlx::types::Json(&status))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

**失联判定**：`cluster_node.last_heartbeat_at` 超过 30s 未更新 → 标记 `unhealthy`，超过 60s → 自动摘除（state='removed'，从负载均衡池排除）。

### 3.3 服务发现

API Gateway 查询"运行中且健康 + 提供某路由"的节点列表：

```sql
SELECT cn.node_id, cn.advertised_addr, cn.labels
FROM cluster_node cn
JOIN module_instance mi ON mi.node_id = cn.node_id
WHERE mi.module_id = $1
  AND mi.state = 'Active'
  AND cn.state = 'Active'
  AND cn.last_heartbeat_at > now() - interval '30 seconds'
ORDER BY cn.load_factor ASC, cn.node_id ASC
LIMIT 10;
```

按 `load_factor` 升序 + `node_id` 升序返回（最空闲优先，公平 fallback）。

### 3.4 领导选举

基于 `leader_lease` 表 + PL/pgSQL `acquire_lease()` 存过。

```sql
CREATE OR REPLACE FUNCTION acquire_lease(
    p_lease_key    TEXT,
    p_node_id      UUID,
    p_ttl_seconds  INT
) RETURNS TABLE(acquired BOOLEAN, lease_id UUID, expires_at TIMESTAMPTZ)
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_existing_node UUID;
    v_existing_expires TIMESTAMPTZ;
    v_new_id UUID;
    v_new_expires TIMESTAMPTZ;
BEGIN
    v_new_expires := now() + (p_ttl_seconds || ' seconds')::INTERVAL;
    
    -- 1. 查询当前持有者
    SELECT holder_node_id, expires_at INTO v_existing_node, v_existing_expires
        FROM leader_lease
        WHERE lease_key = p_lease_key
        FOR UPDATE;  -- 行锁
    
    -- 2. 无人持有 或 已过期 或 同一节点续约
    IF v_existing_node IS NULL 
       OR v_existing_expires < now() 
       OR v_existing_node = p_node_id THEN
        -- 抢占/续约
        INSERT INTO leader_lease (lease_key, holder_node_id, acquired_at, expires_at)
        VALUES (p_lease_key, p_node_id, now(), v_new_expires)
        ON CONFLICT (lease_key) DO UPDATE 
            SET holder_node_id = p_node_id, acquired_at = now(), expires_at = v_new_expires
        RETURNING id INTO v_new_id;
        
        RETURN QUERY SELECT TRUE, v_new_id, v_new_expires;
    ELSE
        -- 失败：已被他人持有且未过期
        RETURN QUERY SELECT FALSE, NULL::UUID, v_existing_expires;
    END IF;
END;
$$;
```

**续约机制**：每个 leader 持有者每 10s 调用一次 `acquire_lease()`（同 lease_key + 同 node_id）续约 30s TTL。

**故障转移**：原 leader 失联 30s 后，TTL 自然过期，其他节点的 `acquire_lease` 尝试可成功抢占。

**事件通知**：每次 leader 变化触发 `cluster.leader_elected` 事件。

### 3.5 状态分片

带状态模块（M-04 编排引擎 checkpoint、M-15 event_log 索引等）的状态按 `tenant_id` 分布到不同节点：

```rust
pub fn shard_for(tenant_id: Uuid, total_shards: u32) -> u32 {
    // 简单 hash，可换成 consistent hashing
    let hash = crc32(tenant_id.as_bytes());
    (hash % total_shards) as u32
}
```

`shard_assignment` 表记录 `shard_id → node_id` 映射。

**Re-balance**：节点增减时触发：
- 移除节点 → 重新分配其负责的 shard 到剩余节点
- 新增节点 → 重新分配，使负载更均匀

**实现约束**：rebalance 期间使用双读（old + new shard 位置）保证可用性。

### 3.6 PL/pgSQL 存过：register_node_heartbeat

```sql
CREATE OR REPLACE FUNCTION register_node_heartbeat(
    p_node_id UUID,
    p_status  JSONB
) RETURNS TABLE(healthy BOOLEAN, current_load NUMERIC)
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_load NUMERIC;
    v_healthy BOOLEAN;
BEGIN
    -- 1. Upsert 心跳
    INSERT INTO cluster_node (node_id, last_heartbeat_at, status, state)
    VALUES (p_node_id, now(), p_status, 'Active')
    ON CONFLICT (node_id) DO UPDATE
        SET last_heartbeat_at = now(), status = p_status;
    
    -- 2. 计算当前 load（active module instance 数 / capacity）
    SELECT 
        (COUNT(*) FILTER (WHERE state = 'Active')::NUMERIC / GREATEST(capacity, 1))::NUMERIC
        INTO v_load
    FROM module_instance
    WHERE node_id = p_node_id;
    
    -- 3. 健康判定
    v_healthy := (p_status->>'health')::BOOLEAN;
    
    RETURN QUERY SELECT v_healthy, v_load;
END;
$$;
```

### 3.7 节点生命周期状态机

```
Registering → Active → Unhealthy → Removed
                ↑           ↓
                └──────── Drain
```

- **Registering**：启动中，尚未发送首个心跳
- **Active**：健康，可接收流量
- **Unhealthy**：超过 30s 未心跳但仍在 60s 内
- **Drain**：管理员手动触发，停止接收新流量
- **Removed**：超过 60s 未心跳或管理员强制摘除

### 3.8 与 M-15 事件联动

节点加入/离开/leader 变化等事件通过 M-15 中心事件总线发布：

| 事件 | 触发时机 | 订阅者 |
|---|---|---|
| `cluster.node_joined` | 新节点成功注册 | 运维 UI、所有节点 |
| `cluster.node_left` | 节点主动关闭 | 运维 UI |
| `cluster.node_unhealthy` | 心跳失联 | 运维 UI、其他节点 |
| `cluster.leader_elected` | Leader 变化 | 运维 UI、所有节点 |
| `cluster.shard_rebalanced` | 分片迁移完成 | 运维 UI、受影响模块 |

## 4. 验收要点

1. **失联检测**：节点 kill -9 后 ≤ 30s 内被标记 Unhealthy。 [NF-AVA]【必須】
2. **Leader 故障转移**：原 leader 失联 30s 后，新 leader 在 5s 内产生。 [NF-AVA]【必須】
3. **线性扩展**：集群从 10 节点扩到 100 节点，吞吐提升 ≥ 80 倍（理想线性）。 [NF-PER]【必須】
4. **PL/pgSQL 原子性**：并发 `acquire_lease` 在同 lease_key 下仅一节点成功。 [NF-SEC]【必須】
5. **分片均衡**：1000 个 tenant 分布到 10 节点，标准差 ≤ 5%。 [NF-PER]【必須】
6. **心跳开销**：每节点 5s 一次，每次 < 1KB。 [NF-OPS]【必須】

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 集群协调 | 多 Runtime 副本的协调机制 | §1 |
| 节点身份 | 启动时生成的 UUID | §3.1 |
| 心跳 | 周期性存活信号 | §3.2 |
| 服务发现 | 节点列表与健康查询 | §3.3 |
| 领导选举 | 集群中选出一个主 | §3.4 |
| 租约 | 持有者声明有效性的时间窗口 | §3.4 |
| 状态分片 | 按 key 分布状态到节点 | §3.5 |
| 一致性哈希 | consistent hashing，分片迁移代价小 | §3.5 |
| TTL | Time To Live | §3.4 |
| 抢占 | 抢占他人持有的租约 | §3.4 |
| 续约 | 续期租约 | §3.4 |
| 故障转移 | failover、leader 切换 | §3.4 |
| 失联 | heartbeat 超过阈值未到达 | §3.2 |
| Drain | 排空进行中请求 | §3.7 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. PostgreSQL Global Development Group「PostgreSQL Documentation — Advisory Locks」
4. etcd 公式「etcd — Distributed reliable key-value store」（参考其 leader election）
5. Kubernetes 公式「Kubernetes — Leader Election」（参考）
6. Ada プロジェクトチーム「原子化部署アーキテクチャ v1.0.0」、2026-08-19（[DOC-ARCH-005](../architecture/04-atomic-deployment.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
