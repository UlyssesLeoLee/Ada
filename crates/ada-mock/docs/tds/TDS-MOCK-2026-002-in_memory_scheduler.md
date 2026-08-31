# TDS-MOCK-2026-002 — InMemoryScheduler 测试设计

> 元数据: 创建 2026-08-31, 设计 = Mavis 接手 (per DEC-008), 审批 = 架构师自审
> 状态: 锁定
> 关联 crate: ada-mock
> 关联源码: `crates/ada-mock/src/mocks/scheduler.rs`

## 1. 目标
证明 `InMemoryScheduler` 的状态机正确性 (合法/非法转移), capacity 强制, 终态释放 in_flight 槽位, FIFO snapshot 顺序.

## 2. 范围
- in-scope: 6 状态机 + capacity + terminal + snapshot
- out-of-scope: worker poll 循环 (mock 同步), 持久化

## 3. 入口
```bash
cargo test -p ada-mock --lib mocks::scheduler
```

## 4. 输入分类

| 类别 | 取值 |
|---|---|
| 状态机合法 | Pending→Queued→Running→Succeeded |
| 状态机非法 | Pending→Running (越级) |
| 容量边界 | capacity=N 时入队 N+1 |
| 终态释放 | 1 个槽位, 一个终态后入队新 job |
| FIFO 顺序 | 入队 a, b → snapshot 是 [a, b] |

## 5. 用例矩阵

| ID | 类别 | 期望 | 已实现 |
|---|---|---|---|
| TC-01 | 合法路径 | 5 步走完, in_flight=0 | `happy_path_pending_to_succeeded` |
| TC-02 | 非法转移 | `Err(IllegalTransition)` | `illegal_transition_rejected` |
| TC-03 | capacity=2 满 | 第 3 个入队 `Err(QueueFull(2))` | `capacity_enforced` |
| TC-04 | terminal 释放 | capacity=1, 终态后入队新 OK | `terminal_release_slot` |
| TC-05 | FIFO | snapshot 顺序与入队一致 | `snapshot_is_fifo` |

## 6. 覆盖率
- 行: 5 测试覆盖 `enqueue/transition/state_of/in_flight/capacity/snapshot`
- 状态机所有合法边全部走过

## 7. 已知缺口
- `enqueue` 重复 ID 走更新分支**未单测** (sample 内会覆盖)
- 错误显示字符串 `JobNotFound` 的 display 仅在错误路径触发, 无独立断言

## 8. 验收
- [x] 全绿, 5 passed
- [x] 所有合法边 + 至少 1 条非法边
- [x] 无 #[ignore]

## 9. 维护
- 模块 Owner: Mavis
- 复审触发: JobState 增/减/重命名, capacity 语义变更
