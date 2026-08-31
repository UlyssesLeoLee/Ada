# TDS 模板 — 测试设计书 (Test Design Specification)

> 每条要写的新功能 / 新 mock / 新接口, 必须填一份, 入 git.
> 模板字段不允许"省" — 留空请写 `N/A` 加理由.

## 0. 元数据

| 字段 | 值 |
|---|---|
| TDS 编号 | TDS-MOCK-YYYY-NNN |
| 关联 crate | ada-mock / ada-mNN / ... |
| 设计人 | (Mavis 默认代签 Ulysses per 8/27 21:59) |
| 审批人 | (Mavis 自审 + DDD Review 阶段补) |
| 创建日期 | YYYY-MM-DD |
| 状态 | 草案 / 评审中 / 锁定 / 废止 |

## 1. 目标 (Objective)

> 一句话说明这个测试**要证明什么**或**要保护什么回归**.

例: "证明 `InMemoryEventBus` 在 1000 个并发 publisher 下不丢消息, 顺序按 seq 单调."

## 2. 范围 (Scope)

- **in-scope**: 列出被覆盖的代码路径/分支
- **out-of-scope**: 明确不覆盖什么 (例: 不测性能上限 / 不测网络异常 / 不测加密)

## 3. 入口与依赖 (Entry & Deps)

- 触发方式: `cargo test -p <crate> --test <file>`
- 外部依赖: 是否有 mock server / DB / 文件系统? 端口冲突策略?
- 共享状态: 是否修改全局 (env var / 单例)? 退出时是否回滚?

## 4. 输入分类 (Input Partition)

| 类别 | 取值 | 覆盖意图 |
|---|---|---|
| 空 | `""` / `[]` / `None` | 边界 |
| 最小有效 | 1 个字符 / 1 条记录 | 基线 |
| 正常有效 | 业务真实场景 | 主路径 |
| 边界 | 容量上限 / 长度 N-1 / N | 临界 |
| 异常 | 错误码 / 超时 / 中断 | 错误路径 |
| 恶意 / 模糊 | 注入 NUL / 超大 payload | 健壮性 |

## 5. 测试用例矩阵

| ID | 类别 | 输入 | 期望 | 断言点 |
|---|---|---|---|---|
| TC-01 | 空 | `""` | `Err(InvariantViolated)` | `assert!(matches!(...))` |
| TC-02 | 正常 | 3 条事件 | 3 条都收到, seq 1,2,3 | `assert_eq!` |
| ... | ... | ... | ... | ... |

## 6. 覆盖率目标

- 行覆盖: ≥ 80% (本模块)
- 分支覆盖: ≥ 70%
- 必须命中的关键分支: (列出 `if/else`/模式匹配)

## 7. 已知缺口 / 限制

- (显式列出, 不写 = 视同无缺口, 走 DDD Review 必查)

## 8. 验收标准 (Acceptance)

- [ ] `cargo test --workspace` 全绿
- [ ] `cargo +nightly llvm-cov --html` 报告本模块 ≥ 80%
- [ ] 至少 1 条用例覆盖每个错误变体
- [ ] 无 `#[ignore]` 标记 (除已知平台抖动, 需注释解释)

## 9. 维护

- 谁负责: (SRE Lead / 模块 Owner)
- 何时复审: 季度 / 接口变更即触发
- 关联 PR: (留空待合并时回填)
