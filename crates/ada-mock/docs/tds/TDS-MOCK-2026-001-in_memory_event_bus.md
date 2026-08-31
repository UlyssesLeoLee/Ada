# TDS-MOCK-2026-001 — InMemoryEventBus 测试设计

> 元数据: 创建 2026-08-31, 设计 = Mavis 接手 (per DEC-008), 审批 = 架构师自审
> 状态: 锁定
> 关联 crate: ada-mock
> 关联源码: `crates/ada-mock/src/mocks/event_bus.rs`

## 1. 目标
证明 `InMemoryEventBus` 的精确-topic 扇出、seq 单调、unsubscribe 幂等、空 topic 拒绝 — 这四类不变量是后续写 sample 测试时**唯一**会触碰的契约面.

## 2. 范围
- in-scope: `subscribe/publish/try_recv/unsubscribe` 主路径, empty topic 拒绝, seq 单调, capacity
- out-of-scope: 性能压测, glob 通配 (mock 不实现), 持久化, 跨进程

## 3. 入口
```bash
cargo test -p ada-mock --lib mocks::event_bus
```

无外部依赖, 无端口, 无文件.

## 4. 输入分类

| 类别 | 取值 | 覆盖意图 |
|---|---|---|
| 空 topic | `""` | 拒绝路径 |
| 正常精确 | `"a.b"` | 主路径 |
| 不匹配 | publish `"b"`, sub `"a"` | 扇出隔离 |
| 多订阅 | 2 个 sub 不同 topic | 独立队列 |
| 取消 | unsubscribe 已存在 / 不存在 | 幂等 |

## 5. 用例矩阵

| ID | 类别 | 输入 | 期望 | 已实现 |
|---|---|---|---|---|
| TC-01 | 正常 | sub "a.b" + pub "a.b" | 1 条事件, seq=1 | `subscribe_publish_recv_round_trip` |
| TC-02 | 不匹配 | sub "a" + sub "b" + pub "a" | queue_depths = [1, 0] | `topic_mismatch_does_not_fan_out` |
| TC-03 | seq 单调 | sub "x" + pub 2 次 | 两条事件 seq 严格递增 | `seq_is_monotonic_per_bus` |
| TC-04 | unsubscribe 幂等 | sub + unsub + 再 unsub | 第 2 次返回 false | `unsubscribe_drops_subscriber` |
| TC-05 | 空 topic 拒绝 | sub "" / pub "" | `Err(InvariantViolated)` | `empty_topic_rejected` |

## 6. 覆盖率
- 行: 已通过 5 个测试覆盖 `subscribe/publish/try_recv/unsubscribe/subscribe_rejects_empty/publish_rejects_empty`
- 缺口: `Default` 派生 (`new()` 走 default) — 行覆盖 100%

## 7. 已知缺口
- `Recorder::drain` 等 server 模块辅助未在本 TDS 覆盖, 见 TDS-MOCK-2026-003
- 扇出策略是 fan-out (订阅前事件丢弃), 不做持久化 replay — 这是 mock 设计选择, 不是 bug

## 8. 验收
- [x] `cargo test -p ada-mock --lib mocks::event_bus` 全绿
- [x] 5 个用例覆盖 4 类输入
- [x] 无 #[ignore]

## 9. 维护
- 模块 Owner: Mavis (per DEC-008)
- 复审触发: 任何对 `InMemoryEventBus` 公共 API 的破坏性变更
