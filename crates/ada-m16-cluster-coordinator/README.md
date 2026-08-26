# `ada-m16-cluster-coordinator` (M-16 Cluster Coordinator)

> M-16 集群协调 crate — v0.1.0 骨架实现(B2)
> 設計書:`docs/modules/M-16-cluster-coordinator.md` (DOC-MOD-016)

## v0.1.0 提供的能力

B2 范围**不**做真实 Raft / RPC / quorum 算法,而是给出一个**可测试**的
本地状态机 + 集群视图协议,让上层业务可以"插桩"：

| 模块 | 内容 |
|---|---|
| `node` | `NodeId`(Uuid newtype)+ `NodeState`(Follower / Candidate / Leader) |
| `term` | `Term` 单调计数器,`next()` 饱和到 `u64::MAX` |
| `heartbeat` | `Heartbeat` + `HeartbeatConfig`,在 `tokio::time::Instant` 上记账 |
| `election` | `Election` 三态状态机,`tick(now, cluster)` 推进 |
| `shard` | `ShardAssignment` 确定性分片(按 node id 升序) |
| `error` | `CoordError` 5 变体(`NotLeader` / `NoQuorum` / `Timeout` / `Network` / `Internal`) |

## 选举状态机

```
                  become_candidate (timeout)
        Follower ────────────────────────────► Candidate
            ▲                                     │
            │ on_leader_heartbeat /                │ 2× election_timeout
            │ reconcile 看到更高 term              │ + 没有同 term 现任 leader
            │                                     ▼
            └─────────────────────────────── Leader
```

关键设计:
- `tick(now, &cluster)` 接收集群视图(`&[ElectionSnapshot]`),先 reconcile:
  - 任何 peer term > self → step down,采用更高 term
  - 任何比自己 `NodeId` 小的 peer 在同 term 已是 leader → step down(小 id 赢 tiebreak)
- reconcile 后才跑本地定时器逻辑
- B2 简化版**不**做网络 RPC,所以 `CoordError::NoQuorum` 变体保留在 enum
  里,留给生产 RPC handler 触发

## 测试

```bash
cargo test -p ada-m16-cluster-coordinator
```

32 个测试覆盖:term 递增 / 状态转移 / 单节点选举 / **3 节点集群收敛到 1 leader** /
shard 分配 / 心跳超时 / leader heartbeat 重置 timer / higher-term step down。

## 集成示例(简化)

```rust,ignore
use std::time::Duration;
use ada_m16_cluster_coordinator::{Election, NodeId, ElectionSnapshot};

let me = NodeId::new();
let peers = vec![NodeId::new(), NodeId::new()];
let mut election = Election::new(me, peers);

loop {
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 真实生产:从 RPC 拿其他节点的 snapshot
    let cluster: Vec<ElectionSnapshot> = gather_peer_snapshots().await;

    let now = tokio::time::Instant::now();
    election.tick(now, &cluster).await.ok();
}
```
