# UT 単体テスト設計書

> **ドキュメントID**：DOC-TST-001
> **文書分類**：単体テスト設計書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/detailed-design.md`（DOC-DTL-001）、`docs/legacy/requirements.md`（DOC-REQ-001）
> **下位文書**：`docs/tests/IT-design.md`（DOC-TST-002）
> **関連文書**：全モジュール別設計書（DOC-MOD-001～013）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（13 モジュール × 169 ケース） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 通用约定
2. M-01 采集适配器
3. M-02 标准化转换
4. M-03 数据流引擎
5. M-04 编排引擎
6. M-05 控制流执行器
7. M-06 节点运行时/插件 SDK
8. M-07 可视化调试
9. M-08 触发器
10. M-09 输出/导出
11. M-10 多租户中间件
12. M-11 权限与协作
13. M-12 前端画布编辑器
14. M-13 API Gateway
15. 覆盖率与质量门禁
16. 持续集成
17. 用語集
18. 参考文献

---

## 0. 通用约定

- **测试框架**：Rust 内置 `#[test]`、`#[tokio::test]`、`proptest`（属性测试）、`mockall`（mock）
- **覆盖率目标**：行覆盖 ≥ 80%、分支覆盖 ≥ 70%
- **Mock 策略**：
  - 外部 HTTP 调用：使用 `wiremock` 起本地桩服务
  - 数据库：`sqlx::test` + 临时 SQLite（轻量） 或 `testcontainers`（PostgreSQL）
  - 浏览器（Playwright）：使用 `playwright` 的 headless 模式访问内嵌静态 HTML
  - LLM：使用 mock HTTP server 返回固定响应
- **运行**：`cargo test --workspace --all-features`
- **报告**：`cargo-llvm-cov` 生成 HTML 报告；CI 集成 `cargo-tarpaulin`

---

## 1. M-01 采集适配器（Acquisition Adapter）

**模块文档**：[M-01-acquisition-adapter.md §3](../modules/M-01-acquisition-adapter.md)  
**测试文件**：`crates/m01-acquisition/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M01-001 | F-02-00 | 双模式选择：forced_mode=Api | config.forced_mode=Api | 返回 AdapterMode::Api，不调用 probe | P0 | ✓ |
| TC-UT-M01-002 | F-02-00 | 双模式选择：probe=ApiAvailable | config.forced_mode=None, probe→ApiAvailable | 返回 AdapterMode::Api | P0 | ✓ |
| TC-UT-M01-003 | F-02-00 | 双模式选择：probe=ApiUnavailable 且支持 Browser | probe→ApiUnavailable, supported_modes=[Browser] | 返回 AdapterMode::Browser，写 audit_log `adapter_fallback_to_browser` | P0 | ✓ |
| TC-UT-M01-004 | F-02-00 | 双模式选择：均不可用 | probe→ApiUnavailable, supported_modes=[Api] | 返回 AdapterError::NoAvailableMode | P0 | ✓ |
| TC-UT-M01-005 | F-02-00 | probe 缓存命中 | Redis 已有 (tenant_id, adapter_id) 缓存 | 不调用 probe，直接读缓存 | P1 | ✓ |
| TC-UT-M01-006 | F-02-01 | 选择器点选生成：data-testid 优先 | 点击 `[data-testid="submit"]` | 生成 `data-testid="submit"` | P0 | ✓ |
| TC-UT-M01-007 | F-02-01 | 选择器点选生成：id 退化 | 点击 `<input id="user-name">` | 生成 `#user-name` | P1 | ✓ |
| TC-UT-M01-008 | F-02-01 | 选择器点选生成：class 组合 | 点击 `<div class="card-title">` | 生成语义化 class 组合选择器（非纯位置 XPath） | P1 | ✓ |
| TC-UT-M01-009 | F-02-02 | 登录态保持：OAuth2 刷新 | access_token 过期, refresh_token 有效 | 自动调用 refresh，返回新 token | P0 | ✓ |
| TC-UT-M01-010 | F-02-02 | 登录态保持：refresh 失败 | refresh_token 也过期 | 返回 AdapterError::AuthExpired | P0 | ✓ |
| TC-UT-M01-011 | F-02-04 | 限流：令牌桶耗尽 | tokens=0, refill_rate=1/s | acquire() 返回 Duration ≈ 1s + jitter | P1 | ✓ |
| TC-UT-M01-012 | F-02-04 | 限流：jitter 随机范围 | 连续 100 次 acquire | 返回的 jitter 落在配置范围内且分布均匀 | P2 | ✓ |
| TC-UT-M01-013 | F-02-05 | IM 发送：未实现 send_message | adapter 未 override send_message | 返回 AdapterError::Unsupported | P1 | ✓ |
| TC-UT-M01-014 | §3.4 BrowserPool | BrowserPool：acquire 未超配额 | current=0, limit=3 | 返回新 BrowserContext | P0 | ✓ |
| TC-UT-M01-015 | §3.4 BrowserPool | BrowserPool：acquire 超配额 + 等待 | current=3, limit=3, timeout=30s | 30s 内有实例释放则返回；否则 PoolError::AcquireTimeout | P0 | ✓ |
| TC-UT-M01-016 | §3.4 BrowserPool | BrowserPool：release 后实例复用 | acquire → release → acquire | 第二次 acquire 拿到同一实例 | P1 | ✓ |
| TC-UT-M01-017 | §3.4 BrowserPool | BrowserPool：使用次数超阈值销毁 | use_count=100 | 释放时销毁实例，下次 acquire 新建 | P2 | ✓ |
| TC-UT-M01-018 | F-15-03 | 增量同步：首次同步 | cursor=NULL | 全量拉取，结果写入 next_cursor | P0 | ✓ |
| TC-UT-M01-019 | F-15-03 | 增量同步：续传 | cursor=已存在 | 仅拉取 cursor 之后的数据 | P0 | ✓ |
| TC-UT-M01-020 | F-15-01 | OAuth2 配置解析 | AuthMethod::OAuth2 with scopes | 解析后 authorize_url 含 scopes | P1 | ✓ |
| TC-UT-M01-021 | §3.8 异常 | 浏览器进程崩溃 | 浏览器子进程 SIGKILL | BrowserPool 检测到，移除实例，触发节点重试 | P0 | ✓ |
| TC-UT-M01-022 | §3.8 异常 | 页面选择器失效 | 目标页面无对应元素 | 返回 AdapterError::SelectorNotFound，触发 F-12 自愈提示 | P0 | ✓ |
| TC-UT-M01-023 | F-16 | 通用 CRM 模板：endpoint path 插值 | path_template=`/orders/{{customer_id}}`, var=customer_id=123 | 实际请求路径 `/orders/123` | P1 | ✓ |
| TC-UT-M01-024 | F-16 | 通用 CRM 模板：pagination 翻页 | pagination=cursor-based, next_cursor=存在 | 翻页至 next_cursor 直至 has_more=false | P1 | ✓ |
| TC-UT-M01-025 | F-02-06 | 合规提示文案存在性 | — | AdapterConfig 序列化结果中含 `compliance_warning` 字段且非空 | P0 | ✓ |

---

## 2. M-02 标准化转换（Normalizer）

**模块文档**：[M-02-normalizer.md §3](../modules/M-02-normalizer.md)  
**测试文件**：`crates/m02-normalizer/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M02-001 | F-03-01 | NJSON 必填字段生成 | 原始数据：platform=lark, captured_at=now | 输出 NJson 含 schema_version/source/captured_at/payload | P0 | ✓ |
| TC-UT-M02-002 | F-03-02 | 字段映射：Identity | source=fields.author.name → target=fields.sender | 输出 payload.fields.sender=原 author.name | P0 | ✓ |
| TC-UT-M02-003 | F-03-03 | 字段映射：ToUpperCase | input="hello" | output="HELLO" | P0 | ✓ |
| TC-UT-M02-004 | F-03-03 | 字段映射：DateFormat | input="2026/08/18", from="%Y/%m/%d", to="%Y-%m-%d" | output="2026-08-18" | P1 | ✓ |
| TC-UT-M02-005 | F-03-03 | 字段映射：Regex | input="abc-123", pattern="\\d+", replacement="XXX" | output="abc-XXX" | P1 | ✓ |
| TC-UT-M02-006 | F-03-03 | 表达式求值：正常 | expr=`upper(payload.author) + "_" + str(payload.timestamp)` | 求值成功，输出预期字符串 | P0 | ✓ |
| TC-UT-M02-007 | F-03-03 | 表达式求值：超时 | expr=死循环 `while(true){}`, timeout=50ms | 字段置 null，记录 EvalError::Timeout warning | P0 | ✓ |
| TC-UT-M02-008 | F-03-03 | 表达式沙箱：禁止网络访问 | expr=`http_get("https://evil.com")` | 沙箱拒绝，EvalError::Unsupported | P0 | ✓ |
| TC-UT-M02-009 | F-03-03 | 表达式沙箱：禁止文件系统 | expr=`file_read("/etc/passwd")` | 沙箱拒绝，EvalError::Unsupported | P0 | ✓ |
| TC-UT-M02-010 | F-03 | 脱敏：FullMask | input="13812345678" | output="************" | P0 | ✓ |
| TC-UT-M02-011 | F-03 | 脱敏：PartialMask keep_prefix=3 keep_suffix=4 | input="13812345678" | output="138****5678" | P0 | ✓ |
| TC-UT-M02-012 | F-03 | 脱敏：Hash SHA-256 | input="secret" | output=64 字符十六进制 | P1 | ✓ |
| TC-UT-M02-013 | F-03-01 | Schema 校验：input_schema 不匹配 | input 不符 input_schema | 路由至异常分支，记录 ValidationError | P0 | ✓ |
| TC-UT-M02-014 | F-03-01 | Schema 校验：output_schema 不匹配 | 映射后输出不符 output_schema | 路由至异常分支 | P0 | ✓ |
| TC-UT-M02-015 | F-03 | 转换流水线：并行执行字段映射 | 10 个独立字段同时映射 | 验证执行时间 ≈ 单字段时间（非 10 倍） | P2 | ✓ |

---

## 3. M-03 数据流引擎（Data Flow Engine）

**模块文档**：[M-03-data-flow-engine.md §3](../modules/M-03-data-flow-engine.md)  
**测试文件**：`crates/m03-dataflow/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M03-001 | F-04-01 | 基本数据传递 | upstream send 1 个 NJson | downstream 收到 1 个 NJson | P0 | ✓ |
| TC-UT-M03-002 | F-04-01 | 顺序保持 | upstream 顺序 send A, B, C | downstream 顺序收到 A, B, C | P0 | ✓ |
| TC-UT-M03-003 | F-04-02 | 背压：Block 策略，队列满 | capacity=10, 已发 10 | 第 11 个 send().await 挂起不返回 | P0 | ✓ |
| TC-UT-M03-004 | F-04-02 | 背压：Block 策略，下游消费 | 上游 send 挂起 → 下游 receive 1 个 | 上游 send 自动恢复 | P0 | ✓ |
| TC-UT-M03-005 | F-04-02 | 背压：DropOldest 策略 | 队列满 10 个，try_send 第 11 个 | 队首被弹出丢弃，新元素入队，metrics.dropped +=1 | P0 | ✓ |
| TC-UT-M03-006 | F-04-02 | 背压：DropNewest 策略 | 队列满 10 个，try_send 第 11 个 | 新元素被丢弃，metrics.dropped +=1 | P0 | ✓ |
| TC-UT-M03-007 | F-04-03 | 持久化溢出：超过 80% 触发 | 队列使用率从 79% → 81% | PersistentOverflowBuffer 接管写入 Redis/文件 | P0 | ✓ |
| TC-UT-M03-008 | F-04-03 | 持久化溢出：Runtime 重启后恢复 | 溢出到 Redis，重启 Runtime | 重启后从 Redis 读回未消费数据 | P0 | ✓ |
| TC-UT-M03-009 | F-04-04 | ChannelMetrics：throughput 计数 | 累计 1000 个数据包 | metrics.throughput_counter == 1000 | P0 | ✓ |
| TC-UT-M03-010 | F-04-04 | ChannelMetrics：queue_depth 实时 | 队列当前长度变化 | current_queue_depth 同步更新 | P1 | ✓ |
| TC-UT-M03-011 | F-04-04 | Prometheus 指标格式 | scrape `/metrics` | 包含 `ada_dataflow_throughput_total{...}` 等 3 类指标 | P1 | ✓ |
| TC-UT-M03-012 | F-04-03 | Replay：replay_from_snapshot | 已知 execution_id+node_id | 从存储读快照重新注入队列 | P0 | ✓ |

---

## 4. M-04 编排引擎（Orchestration Engine）

**模块文档**：[M-04-orchestration-engine.md §3](../modules/M-04-orchestration-engine.md)  
**测试文件**：`crates/m04-orchestration/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M04-001 | F-05-01 | 状态机核心循环：正常完成 | 线性 DAG: A→B→C | A、B、C 依次执行，最终 all_terminal=true | P0 | ✓ |
| TC-UT-M04-002 | F-05-01 | 条件分支：true 路径 | 条件节点 expr=true | 走 true 分支节点 | P0 | ✓ |
| TC-UT-M04-003 | F-05-01 | 条件分支：false 路径 | 条件节点 expr=false | 走 false 分支节点 | P0 | ✓ |
| TC-UT-M04-004 | F-05-01 | 循环节点：3 次迭代 | collection=[a,b,c] | 循环体执行 3 次，索引 0/1/2 | P0 | ✓ |
| TC-UT-M04-005 | F-05-01 | 汇聚节点：所有上游完成 | 上游 3 个节点都 Success | 汇聚节点进入 runnable 集合 | P0 | ✓ |
| TC-UT-M04-006 | F-05-01 | 汇聚节点：有上游 Failed | 上游 1 个 Failed | 汇聚节点不执行（或按配置处理） | P0 | ✓ |
| TC-UT-M04-007 | F-05-02 | 异常捕获：路由至异常分支 | 节点失败 + 存在 on_error 出边 | 状态 Failed，执行异常分支节点 | P0 | ✓ |
| TC-UT-M04-008 | F-05-02 | 异常未捕获：整体失败 | 节点失败 + 无 on_error 出边 | ExecutionState.status=failure | P0 | ✓ |
| TC-UT-M04-009 | F-05-03 | 重试：未达 max_attempts | max_attempts=3, 第一次失败 | 依据 BackoffStrategy 调度重试 | P0 | ✓ |
| TC-UT-M04-010 | F-05-03 | 重试：已达 max_attempts | max_attempts=2, 第二次失败 | 不再重试，状态 Failed | P0 | ✓ |
| TC-UT-M04-011 | F-05-03 | 指数退避计算 | attempt=3, base=100ms, multiplier=2, max=10s | delay=400ms + jitter | P1 | ✓ |
| TC-UT-M04-012 | F-05-04 | LLM 决策：合法返回 | LLM 返回 output_branches 中的值 | 走对应分支 | P1 | ✓ |
| TC-UT-M04-013 | F-05-04 | LLM 决策：非法返回 | LLM 返回不在候选中的值 | 走 fallback_branch | P1 | ✓ |
| TC-UT-M04-014 | F-05-04 | LLM 决策：调用失败 | LLM endpoint 不可达 | 走 fallback_branch | P1 | ✓ |
| TC-UT-M04-015 | §3.5 | 状态不可变：每次迁移生成新版本 | 状态迁移 1 次 | version += 1，旧状态仍可访问 | P0 | ✓ |
| TC-UT-M04-016 | §3.5 | 断点续传：checkpoint 后 kill -9 模拟 | 状态保存 → 进程被杀 → 重启 load_latest | 恢复到 checkpoint 时状态 | P0 | ✓ |
| TC-UT-M04-017 | §3.1 | 循环依赖检测 | DAG 含 A→B→A | 返回 OrchestrationError::CyclicDependency | P0 | ✓ |
| TC-UT-M04-018 | §3.1 | wait_for_external_event | 全部 runnable 耗尽但非 terminal | 挂起等待外部事件，不无限循环 | P0 | ✓ |
| TC-UT-M04-019 | §3.1 | HumanReview 节点 | 到达 HumanReview 节点 | 状态置为 waiting，挂起等待用户事件 | P0 | ✓ |

---

## 5. M-05 控制流执行器（Control Flow Executor）

**模块文档**：[M-05-control-flow-executor.md §3](../modules/M-05-control-flow-executor.md)  
**测试文件**：`crates/m05-controlflow/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M05-001 | F-06-01 | 节点并发执行 | 5 个 runnable 节点，tenant_semaphore=10 | 5 个节点并发执行 | P0 | ✓ |
| TC-UT-M05-002 | F-06-01 | 租户级并发限流 | A 租户 semaphore=2，A 提交 5 节点 | 同时仅 2 个节点执行，3 个等待 permit | P0 | ✓ |
| TC-UT-M05-003 | F-06-01 | 节点级并发限流 | 某节点 semaphore=1 | 该节点同时仅 1 个执行 | P1 | ✓ |
| TC-UT-M05-004 | F-06-02 | 暂停：节点执行中收到 Pause | 节点执行 → 发送 Pause | 当前节点完成后挂起 | P0 | ✓ |
| TC-UT-M05-005 | F-06-02 | 恢复：Pause → Resume | 挂起状态发送 Resume | 继续执行剩余节点 | P0 | ✓ |
| TC-UT-M05-006 | F-06-02 | 终止：Abort | 任意时机发送 Abort | 节点状态置 Skipped{reason:"aborted_by_user"} | P0 | ✓ |
| TC-UT-M05-007 | F-06-03 | 单节点手动触发 | trigger_single_node(node_id) | 不经过状态机直接执行 | P1 | ✓ |
| TC-UT-M05-008 | F-06-03 | 单节点触发：使用历史快照 | mock_input=None | 使用最近一次成功执行的 input 快照 | P1 | ✓ |
| TC-UT-M05-009 | F-06 | 跨租户并发隔离 | A 租户占满后 B 租户执行 | B 租户执行不受 A 阻塞 | P0 | ✓ |

---

## 6. M-06 节点运行时 / 插件 SDK

**模块文档**：[M-06-node-runtime-plugin-sdk.md §3](../modules/M-06-node-runtime-plugin-sdk.md)  
**测试文件**：`crates/m06-plugin/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M06-001 | F-07-01 | Rust 原生插件加载 | libloading 加载 .so/.dll | 加载成功，PluginRuntime::Native | P0 | ✓ |
| TC-UT-M06-002 | F-07-01 | WASM 插件加载 | wasmtime 加载 .wasm | 加载成功，PluginRuntime::Wasm | P0 | ✓ |
| TC-UT-M06-003 | F-07-01 | 插件 execute 正常 | 合法 input + config | 返回 NJson | P0 | ✓ |
| TC-UT-M06-004 | F-07-01 | 插件 execute 错误 | 触发插件内错误 | 返回 PluginError | P0 | ✓ |
| TC-UT-M06-005 | §3.2 | WASM 资源限制：CPU 超限 | WASM 死循环 | PluginError::ResourceLimitExceeded，fuel 耗尽 | P0 | ✓ |
| TC-UT-M06-006 | §3.2 | WASM 资源限制：内存超限 | WASM 申请超大内存 | PluginError::ResourceLimitExceeded | P0 | ✓ |
| TC-UT-M06-007 | §3.3 | 连线 Schema 兼容性：兼容 | target_schema 必填字段 ⊆ source_schema 字段 | validate_edge 返回 true | P0 | ✓ |
| TC-UT-M06-008 | §3.3 | 连线 Schema 兼容性：不兼容 | target_schema 必填字段 ⊄ source_schema | validate_edge 返回 false，对应错误码 EDGE_INCOMPATIBLE_SCHEMA | P0 | ✓ |
| TC-UT-M06-009 | §3.1 | 插件生命周期：on_load/execute/on_unload | 完整生命周期 | 三阶段按序触发，状态正确 | P0 | ✓ |
| TC-UT-M06-010 | §3.1 | 多租户资源限制 | tenant_id=A, limits={max_memory_mb:10} | 插件实际内存不超过限制 | P1 | ✓ |
| TC-UT-M06-011 | §3.1 | PluginError::OutputSchemaViolation | 插件返回 NJson 不符 output_schema | PluginError::OutputSchemaViolation | P1 | ✓ |

---

## 7. M-07 可视化调试（Debug Service）

**模块文档**：[M-07-debug-service.md §3](../modules/M-07-debug-service.md)  
**测试文件**：`crates/m07-debug/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M07-001 | F-08-01 | 快照保留：保留最近 20 次 | 同一节点执行 25 次 | execution_node_snapshot 仅保留最近 20 次 | P0 | ✓ |
| TC-UT-M07-002 | F-08-01 | 快照：定时清理 | 过期快照 | 由定时任务清理 | P1 | ✓ |
| TC-UT-M07-003 | F-08-01 | 大体积数据：仅存 ref | input 1.5MB | input_ref 指向对象存储，DB 中无原始数据 | P0 | ✓ |
| TC-UT-M07-004 | §3.2 | Replay 重新注入队列 | replay_from_snapshot 已知快照 | 对应 Edge 队列收到 NJson | P0 | ✓ |
| TC-UT-M07-005 | §3.4 | DebugError::NoSnapshotAvailable | replay 不存在的快照 | DebugError::NoSnapshotAvailable | P1 | ✓ |
| TC-UT-M07-006 | F-08-02 | 快照 JSON 序列化可逆 | 复杂 NJson | 序列化→反序列化后值完全一致（属性测试） | P2 | ✓ |

---

## 8. M-08 定时/事件触发器（Trigger Service）

**模块文档**：[M-08-trigger-service.md §3](../modules/M-08-trigger-service.md)  
**测试文件**：`crates/m08-trigger/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M08-001 | F-13-01 | Cron：标准 5 段表达式 | `0 9 * * *` | 到点触发 | P0 | ✓ |
| TC-UT-M08-002 | F-13-01 | Cron：时区正确 | 时区=Asia/Shanghai，cron `0 9 * * *` | 在北京时间 9:00 触发（非 UTC） | P0 | ✓ |
| TC-UT-M08-003 | F-13-01 | Cron：解析非法表达式 | `invalid cron expr` | 返回解析错误，不注册调度器 | P0 | ✓ |
| TC-UT-M08-004 | F-13-02 | Webhook：合法签名 | HMAC 签名正确 | 触发执行 | P0 | ✓ |
| TC-UT-M08-005 | F-13-02 | Webhook：非法签名 | HMAC 签名错误 | 401 Unauthorized，不触发 | P0 | ✓ |
| TC-UT-M08-006 | F-13-02 | Webhook：时间戳过期 | timestamp 超出 ±5min | 401 Unauthorized，防重放 | P0 | ✓ |
| TC-UT-M08-007 | F-13 | 手动触发：返回 execution_id | Manual 触发 | 立即返回 execution_id，异步执行 | P0 | ✓ |
| TC-UT-M08-008 | F-13 | 触发去重 | 同一 trigger 100ms 内 2 个 Webhook | 仅触发 1 次执行 | P1 | ✓ |
| TC-UT-M08-009 | F-17 | 配额检查：超 concurrent_canvas_executions | 配额满 | 返回 QuotaError::Exceeded | P0 | ✓ |

---

## 9. M-09 输出/导出（Exporter）

**模块文档**：[M-09-exporter.md §3](../modules/M-09-exporter.md)  
**测试文件**：`crates/m09-exporter/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M09-001 | F-14-01 | 文件输出：JSON | ExporterConfig::File{format:JSON} | 写入合法 JSON 文件 | P0 | ✓ |
| TC-UT-M09-002 | F-14-01 | 文件输出：CSV | ExporterConfig::File{format:CSV} | 写入合法 CSV 文件（特殊字符转义） | P0 | ✓ |
| TC-UT-M09-003 | F-14-01 | 数据库输出：Insert | ExporterConfig::Database{mode:Insert} | 数据插入，重复执行产生重复行 | P0 | ✓ |
| TC-UT-M09-004 | F-14-01 | 数据库输出：Upsert | ExporterConfig::Database{mode:Upsert} | 主键冲突时更新 | P0 | ✓ |
| TC-UT-M09-005 | F-14-01 | Webhook 输出：成功 | HTTP 200 | 成功 | P0 | ✓ |
| TC-UT-M09-006 | F-14-01 | Webhook 输出：重试 | HTTP 500 | 按 RetryPolicy 重试 | P0 | ✓ |
| TC-UT-M09-007 | F-14-01 | 平台写回：复用 M-01 send_message | IM 消息发送 | 调用 [M-01 §3.2 send_message](../modules/M-01-acquisition-adapter.md) | P1 | ✓ |
| TC-UT-M09-008 | F-14-01 | 大体积流式写入 | 1GB 数据 | 不爆内存（RSS 增长 < 100MB） | P1 | ✓ |
| TC-UT-M09-009 | F-17 | 多租户 DB 写入带 tenant_id | 写入本系统 DB | 自动注入 tenant_id，符合 RLS | P0 | ✓ |

---

## 10. M-10 多租户中间件（Tenant Middleware）

**模块文档**：[M-10-tenant-middleware.md §3](../modules/M-10-tenant-middleware.md)  
**测试文件**：`crates/m10-tenant/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M10-001 | §3.1 | 中间件：JWT 解析 | 合法 JWT | 提取 tenant_id, user_id, roles 注入扩展 | P0 | ✓ |
| TC-UT-M10-002 | §3.1 | 中间件：路径 tenant_id 与 Token 不一致 | path_tenant_id=A, claims.tenant_id=B | 返回 403 TENANT_MISMATCH | P0 | ✓ |
| TC-UT-M10-003 | §3.1 | 中间件：JWT 过期 | 过期 token | 401 Unauthorized | P0 | ✓ |
| TC-UT-M10-004 | §3.1 | with_tenant_scope：事务内设置会话变量 | 进入函数 | `app.current_tenant` 在事务内可读 | P0 | ✓ |
| TC-UT-M10-005 | §3.1 | with_tenant_scope：事务提交后清除 | tx.commit() | 后续查询无 `app.current_tenant` 残留 | P0 | ✓ |
| TC-UT-M10-006 | §3.1 | 连接池防御：跨事务不残留 | tx1 设置 → 提交 → tx2 无设置 | tx2 看不到 tx1 的会话变量 | P0 | ✓ |
| TC-UT-M10-007 | §3.2 | 配额：concurrent_canvas_executions 超限 | 当前=limit, 尝试 reserve | QuotaError::Exceeded | P0 | ✓ |
| TC-UT-M10-008 | §3.2 | 配额：api_calls_per_hour 滑动窗口 | 1 小时内 1001 次 | 第 1001 次返回 Exceeded | P0 | ✓ |
| TC-UT-M10-009 | §3.3 | 租户状态机：active → suspended | 调用 suspend() | tenant.status = suspended | P0 | ✓ |
| TC-UT-M10-010 | §3.3 | 租户状态机：suspended → active | 调用 resume() | tenant.status = active | P0 | ✓ |
| TC-UT-M10-011 | §3.3 | 租户硬删除：返回 DeletionReport | hard_delete_tenant_data(A) | report 含每表行数与吊销密钥列表 | P0 | ✓ |
| TC-UT-M10-012 | §3.3 | 硬删除中断：PartialFailure | 第 3 步失败 | DeletionError::PartialFailure{completed_steps: [...]} | P1 | ✓ |
| TC-UT-M10-013 | §3.4 | RLS：tenant A 用户查不到 tenant B 数据 | RLS 启用 + with_tenant_scope(A) | 查询结果不包含 B 数据 | P0 | ✓ |
| TC-UT-M10-014 | §3.4 | RLS：直接连接无 with_tenant_scope | 裸连接查询 | RLS 拒绝返回任何行（current_setting 为 NULL） | P0 | ✓ |
| TC-UT-M10-015 | §3.4 | RLS 策略：USING 字段缺失 | tenant_id 为 NULL | 行不返回 | P1 | ✓ |
| TC-UT-M10-016 | F-17-03 | 配额查看：返回 TenantQuotaView | GET /quota | 返回各资源使用率 | P1 | ✓ |

---

## 11. M-11 权限与协作（RBAC & Collab）

**模块文档**：[M-11-rbac-collab.md §3](../modules/M-11-rbac-collab.md)  
**测试文件**：`crates/m11-rbac/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M11-001 | F-11-01 | RBAC：Owner 角色权限 | role=Owner | 所有 Action 都有权限 | P0 | ✓ |
| TC-UT-M11-002 | F-11-01 | RBAC：Viewer 角色权限 | role=Viewer | 仅 Read，拒绝 Write/Execute/Delete | P0 | ✓ |
| TC-UT-M11-003 | F-11-01 | RBAC：Executor 角色权限 | role=Executor | 允许 Read+Execute，拒绝 Write/Delete | P0 | ✓ |
| TC-UT-M11-004 | F-11-02 | CRDT：不同字段并发编辑 | 用户 A 改 position, 用户 B 改 config | Yjs 自动合并，状态一致 | P0 | ✓ |
| TC-UT-M11-005 | F-11-02 | CRDT：同一标量字段 LWW | 用户 A、B 改同一字段 | 后到者获胜（基于时钟） | P1 | ✓ |
| TC-UT-M11-006 | F-11-03 | 审计日志：画布编辑记录 | edit_canvas | audit_log 写入 before/after JSON Patch | P0 | ✓ |
| TC-UT-M11-007 | F-11-03 | 审计日志：凭证访问 | 读取 encrypted_payload | audit_log 含 action_type='credential.access' | P0 | ✓ |
| TC-UT-M11-008 | F-11-04 | 画布共享邀请：链接生成 | 邀请用户 | 生成含 token 的邀请链接，TTL 可配 | P1 | ✓ |
| TC-UT-M11-009 | F-11-04 | 画布共享邀请：token 失效 | TTL 过期 | 401 Unauthorized | P1 | ✓ |
| TC-UT-M11-010 | §3.4 | 乐观锁冲突 | 旧 version 提交 PUT | 409 CONCURRENT_EDIT_CONFLICT | P0 | ✓ |

---

## 12. M-12 前端画布编辑器（Canvas Editor Frontend）

**模块文档**：[M-12-canvas-editor-frontend.md §3](../modules/M-12-canvas-editor-frontend.md)  
**测试文件**：`frontend/canvas/src/tests/`（Rust + WASM，使用 `wasm-bindgen-test`）

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M12-001 | F-01-01 | 缩放范围 10%~1000% | zoom=0.05, 0.1, 1.0, 10.0, 15.0 | 0.05/15.0 被 clamp 到 0.1/10.0 | P0 | ✓ |
| TC-UT-M12-002 | F-01-02 | 框选 | 鼠标框选 3 个节点 | 3 节点全被选中 | P0 | ✓ |
| TC-UT-M12-003 | F-01-02 | 多选 + 批量移动 | Shift 点击 3 节点 + 拖拽 | 3 节点同步移动 | P0 | ✓ |
| TC-UT-M12-004 | F-01-03 | 连线创建 | 拖拽 from.output → to.input | Edge 创建，存入 CanvasDefinition | P0 | ✓ |
| TC-UT-M12-005 | F-01-03 | 连线删除 | 选中连线 + Delete | Edge 从 CanvasDefinition 移除 | P0 | ✓ |
| TC-UT-M12-006 | F-01-04 | 连线视觉区分：DataFlow | 拖拽数据流连线 | 实线 + 流光动效 | P0 | ✓ |
| TC-UT-M12-007 | F-01-04 | 连线视觉区分：ControlFlow | 拖拽控制流连线 | 虚线/箭头样式 | P0 | ✓ |
| TC-UT-M12-008 | F-01-05 | 分组框 | 创建 Frame 包裹 3 节点 | Frame 与节点位置联动 | P1 | ✓ |
| TC-UT-M12-009 | F-01-06 | Undo/Redo | 连续 60 步操作 | Undo 可回退最近 50 步 | P0 | ✓ |
| TC-UT-M12-010 | §3.3 | 视锥裁剪：1000 节点 | viewport 仅可见 10 节点 | 仅 10 节点进入渲染队列 | P0 | ✓ |
| TC-UT-M12-011 | §3.3 | R-tree 空间索引更新 | 单节点移动 | 仅局部索引节点更新，不整体重建 | P1 | ✓ |
| TC-UT-M12-012 | §3.4 | HTML Overlay 坐标同步 | 画布 pan/zoom | Overlay left/top 同步 | P0 | ✓ |
| TC-UT-M12-013 | §3.6 | 协作者光标渲染 | 收到 awareness_update | 渲染对方光标实体 | P1 | ✓ |
| TC-UT-M12-014 | §3.6 | 客户端乐观更新 | 拖拽节点 | 本地立即响应，异步同步服务端 | P0 | ✓ |
| TC-UT-M12-015 | §3.6 | 服务端校正 | 同步失败/冲突 | 静默纠正本地渲染，不打断用户 | P0 | ✓ |

---

## 13. M-13 API Gateway

**模块文档**：[M-13-api-gateway.md §3](../modules/M-13-api-gateway.md)  
**测试文件**：`crates/m13-gateway/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M13-001 | I-01 | JWT 鉴权：合法 token | Bearer token 合法 | 鉴权通过，进入 handler | P0 | ✓ |
| TC-UT-M13-002 | I-01 | JWT 鉴权：伪造 token | 篡改签名 | 401 Unauthorized | P0 | ✓ |
| TC-UT-M13-003 | I-01 | 中间件链顺序 | 请求通过 | CORS→日志→JWT→tenant→RBAC→handler 按序执行 | P0 | ✓ |
| TC-UT-M13-004 | F-17 | 多租户上下文强制：缺失 token | 无 Authorization 头 | 401 Unauthorized | P0 | ✓ |
| TC-UT-M13-005 | F-17 | 限流：per-tenant 超限 | A 租户 1h 内 1001 次 | 429 QUOTA_EXCEEDED | P0 | ✓ |
| TC-UT-M13-006 | I-06 | 多租户管理 API：Owner 可访问 | role=Owner | 200 OK | P0 | ✓ |
| TC-UT-M13-007 | I-06 | 多租户管理 API：Viewer 拒绝 | role=Viewer | 403 Forbidden | P0 | ✓ |
| TC-UT-M13-008 | §3.4 | 错误码转换：AdapterError::AuthExpired | — | HTTP 401, error_code=ADAPTER_AUTH_EXPIRED | P0 | ✓ |
| TC-UT-M13-009 | §3.4 | 错误码转换：QuotaError::Exceeded | — | HTTP 429, error_code=QUOTA_EXCEEDED | P0 | ✓ |
| TC-UT-M13-010 | §3.4 | 错误码转换：AdapterError::SelectorNotFound | — | HTTP 200, 节点级失败（不阻断 HTTP） | P0 | ✓ |
| TC-UT-M13-011 | I-01 | HTTPS 强制 | HTTP 请求 → HTTPS-only 端点 | 301/308 重定向到 HTTPS | P0 | ✓ |
| TC-UT-M13-012 | I-01 | CORS：合法 origin | Origin 头白名单内 | 携带 CORS 头 | P1 | ✓ |
| TC-UT-M13-013 | I-01 | CORS：非法 origin | Origin 不在白名单 | 拒绝 + 不携带 CORS 头 | P1 | ✓ |

---

## 13.5 M-14 模块注册与生命周期（Module Registry）

**模块文档**：[M-14-module-registry.md §3](../modules/M-14-module-registry.md)  
**测试文件**：`crates/m14-module-registry/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M14-001 | §3.4 | PL/pgSQL `register_module`：合法 manifest | manifest 必填字段齐全 | 返回 (TRUE, instance_id, NULL) | P0 | ✓ |
| TC-UT-M14-002 | §3.4 | PL/pgSQL `register_module`：缺 module_id | manifest.meta.module_id 为 NULL | 返回 (FALSE, NULL, 'module_id 必填') | P0 | ✓ |
| TC-UT-M14-003 | §3.4 | PL/pgSQL `register_module`：幂等性 | 同 module_id+version 重复注册 | 第二次返回已存在 instance_id，不创建新行 | P0 | ✓ |
| TC-UT-M14-004 | §3.4 | PL/pgSQL `register_module`：事件触发 | 注册成功 | `module.registered` 事件已发布 | P0 | ✓ |
| TC-UT-M14-005 | §3.5 | PL/pgSQL `atomic_module_swap`：正常升级 | from_version 存在 + to_version 存在 | from=inactive, to=active，写 module_upgrade_history | P0 | ✓ |
| TC-UT-M14-006 | §3.5 | PL/pgSQL `atomic_module_swap`：from 不存在 | from_version 未注册 | 返回 (FALSE, 'from_version not found') | P0 | ✓ |
| TC-UT-M14-007 | §3.5 | PL/pgSQL `atomic_module_swap`：并发安全 | 两个事务同时 swap 同一 module | advisory lock 串行化，无中间态可见 | P0 | ✓ |
| TC-UT-M14-008 | §3.2 | 状态机：Discovered→Registered | — | 状态转移合法，event 触发 | P0 | ✓ |
| TC-UT-M14-009 | §3.2 | 状态机：非法转移 | Registered→Active 跳过 Loaded | 返回 InvalidStateTransition 错误 | P0 | ✓ |
| TC-UT-M14-010 | §3.6 | 唯一约束：同 module_id 仅 1 active | 两次 active 同一 module | UNIQUE(tenant_id, module_id) WHERE active=TRUE 阻止 | P0 | ✓ |
| TC-UT-M14-011 | §3.3 | Rolling 升级：1 节点 | batch_size=1, 1 个节点 | 节点依次 execute_single → drain → unload | P0 | ✓ |
| TC-UT-M14-012 | §3.3 | Rolling 升级：健康检查失败回滚 | health_check 不通过 | 30s 内回滚到 from_version | P0 | ✓ |
| TC-UT-M14-013 | §3.3 | Blue-Green 升级 | strategy=blue-green | 双倍资源，瞬时切换 | P1 | ✓ |
| TC-UT-M14-014 | §3.3 | Canary 升级：阶段比例 | canary_stages=[5,25,50,100] | 依次按比例切换 | P1 | ✓ |

## 13.6 M-15 中心事件总线（Central Event Bus）

**模块文档**：[M-15-central-event-bus.md §3](../modules/M-15-central-event-bus.md)  
**测试文件**：`crates/m15-event-bus/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M15-001 | §3.5 | PL/pgSQL `append_event`：基本追加 | 合法 topic + payload | event_seq 单调递增，event_log 写入 | P0 | ✓ |
| TC-UT-M15-002 | §3.5 | PL/pgSQL `append_event`：并发追加 | 100 并发 append | event_seq 唯一不重复 | P0 | ✓ |
| TC-UT-M15-003 | §3.5 | PL/pgSQL `append_event`：tenant 隔离 | tenant_id=A | RLS 限制 A 租户只能查 A 事件 | P0 | ✓ |
| TC-UT-M15-004 | §3.3 | 发布：自动注册 topic | topic 不存在 | event_topic 表插入新行 | P0 | ✓ |
| TC-UT-M15-005 | §3.7 | 持久订阅：ACK 更新 offset | 消费者处理后调用 ACK | consumer_offset.last_acked_event_seq 更新 | P0 | ✓ |
| TC-UT-M15-006 | §3.7 | 持久订阅：重启续传 | 消费者崩溃重启 | 从 consumer_offset 继续 | P0 | ✓ |
| TC-UT-M15-007 | §3.7 | At-least-once：重复消费 | 同一事件被 2 个 consumer 收到 | 都收到，至少 1 次 | P0 | ✓ |
| TC-UT-M15-008 | §3.8 | Replay：from_seq | replay(topic, from=1000) | 投递 event_seq >= 1000 的事件 | P0 | ✓ |
| TC-UT-M15-009 | §3.8 | Replay：干跑模式 | dry_run=true | 仅记录不实际投递 | P1 | ✓ |
| TC-UT-M15-010 | §3.8 | Replay：不影响生产 offset | replay 后 | 生产 consumer_offset 不变 | P0 | ✓ |
| TC-UT-M15-011 | §3.7 | 临时订阅：实时推送 | ephemeral group | 事件通过 WebSocket 推送 | P0 | ✓ |
| TC-UT-M15-012 | §3.1 | Topic 通配符订阅 | `module.*` | 匹配 module.registered、module.swapped 等 | P0 | ✓ |
| TC-UT-M15-013 | §3.6 | Retention 清理 | retention_days=1 | 后台任务删除 produced_at < now-1d 的事件 | P1 | ✓ |

## 13.7 M-16 集群协调（Cluster Coordinator）

**模块文档**：[M-16-cluster-coordinator.md §3](../modules/M-16-cluster-coordinator.md)  
**测试文件**：`crates/m16-cluster/src/tests/`

| TC ID | 关联要件 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-UT-M16-001 | §3.1 | 节点注册：合法 | hostname + boot_time + nonce | node_id 生成，cluster_node 写入 | P0 | ✓ |
| TC-UT-M16-002 | §3.1 | 节点注册：重复 ID | 同 nonce 二次启动 | 同 node_id 复用 | P0 | ✓ |
| TC-UT-M16-003 | §3.2 | 心跳：5s 周期 | 启动 heartbeat_loop | 5s 一次 last_heartbeat_at 更新 | P0 | ✓ |
| TC-UT-M16-004 | §3.2 | 失联检测：30s 标记 Unhealthy | 心跳停 30s | state='Unhealthy' | P0 | ✓ |
| TC-UT-M16-005 | §3.2 | 失联检测：60s 自动摘除 | 心跳停 60s | state='Removed' | P0 | ✓ |
| TC-UT-M16-006 | §3.4 | PL/pgSQL `acquire_lease`：抢占空 | lease_key 无持有者 | 返回 acquired=TRUE | P0 | ✓ |
| TC-UT-M16-007 | §3.4 | PL/pgSQL `acquire_lease`：抢占失败 | 已被 A 持有，TTL 未过 | 返回 acquired=FALSE，expires_at=持有者时间 | P0 | ✓ |
| TC-UT-M16-008 | §3.4 | PL/pgSQL `acquire_lease`：过期抢占 | 已被 A 持有，TTL 已过 | B 抢占成功 | P0 | ✓ |
| TC-UT-M16-009 | §3.4 | PL/pgSQL `acquire_lease`：续约 | 同 node_id + 同 lease_key | renew_count += 1，expires_at 续期 | P0 | ✓ |
| TC-UT-M16-010 | §3.4 | PL/pgSQL `release_lease`：合法 | 持有者调用 | 返回 released=TRUE，DELETE 行 | P0 | ✓ |
| TC-UT-M16-011 | §3.4 | PL/pgSQL `release_lease`：非持有者 | 非持有节点调用 | 返回 released=FALSE | P0 | ✓ |
| TC-UT-M16-012 | §3.4 | PL/pgSQL `acquire_lease`：并发安全 | 100 节点同时抢 | 仅 1 节点成功（advisory lock） | P0 | ✓ |
| TC-UT-M16-013 | §3.6 | PL/pgSQL `register_node_heartbeat`：upsert | 新节点 | INSERT + last_heartbeat_at=now | P0 | ✓ |
| TC-UT-M16-014 | §3.6 | PL/pgSQL `register_node_heartbeat`：load 计算 | 3 active module / capacity 5 | current_load=0.60 | P1 | ✓ |
| TC-UT-M16-015 | §3.3 | 服务发现：仅返回健康节点 | 1 healthy + 1 unhealthy | 仅 healthy 被返回 | P0 | ✓ |
| TC-UT-M16-016 | §3.3 | 服务发现：load_factor 升序 | 多 healthy 节点 | load_factor 升序返回 | P1 | ✓ |
| TC-UT-M16-017 | §3.5 | 状态分片：1000 tenant × 10 节点 | — | 标准差 ≤ 5% | P1 | ✓ |
| TC-UT-M16-018 | §3.7 | 状态机：Active → Unhealthy | 30s 失联 | 状态转移 + event 触发 | P0 | ✓ |

---

## 14. 覆盖率与质量门禁

- **行覆盖**：≥ 80%（CI 卡点）
- **分支覆盖**：≥ 70%（CI 卡点）
- **P0 用例通过率**：100%（CI 卡点，任意 P0 失败阻塞合并）
- **P1 用例通过率**：100%（合并后可放行，但下个迭代必须修复）
- **M-10（多租户）专项**：P0 用例执行时长 < 5 分钟（含 DB 容器启动）

## 15. 持续集成

- 每次 PR 触发 `cargo test --workspace`
- 每日定时任务执行完整 UT 套件 + 覆盖率报告
- 不通过 PR 阻塞：行覆盖 < 80% / 任意 P0 失败
- 报告产物：`coverage/html/index.html` + `coverage/summary.txt`

---

## 17. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 単体テスト | 関数・構造体・trait 単位のテスト | §0 |
| 行覆盖 | Line Coverage、実行行の網羅率 | §14 |
| 分支覆盖 | Branch Coverage、分岐網羅率 | §14 |
| 属性测试 | Property-Based Testing、proptest 等 | §0 |
| mockall | Rust のモック生成クレート | §0 |
| wiremock | HTTP スタブサーバ | §0 |
| testcontainers | コンテナ化された依存サービス | §0 |
| sqlx::test | テスト時 DB セットアップ | §0 |
| cargo-llvm-cov | カバレッジ計測ツール | §0 |
| P0 阻塞 | 最高優先度、リリースブロック | §0 |

## 18. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. JIS X 25010:2013「システム及びソフトウェア製品の品質モデル」
4. Rust 公式ドキュメント「The Rust Programming Language — Testing」
5. proptest 公式ドキュメント「proptest — Hypothesis-like property testing for Rust」
6. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
