# IT 結合テスト設計書

> **ドキュメントID**：DOC-TST-002
> **文書分類**：結合テスト設計書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）
> **下位文書**：`docs/tests/ST-design.md`（DOC-TST-003）
> **関連文書**：全モジュール別設計書（DOC-MOD-001～013）、[DOC-API-001～003](../api/)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（跨モジュール集成 47 ケース） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 通用约定
2. 数据采集流水线
3. 编排与控制流
4. 插件与节点运行时
5. 触发与执行
6. 多租户隔离
7. 权限与协作
8. API Gateway 与前端
9. 数据库集成
10. 外部系统集成
11. 端到端关键路径
12. 模块部署/事件/集群集成
13. 性能/资源 IT 维度
14. 质量门禁
15. 持续集成
16. 用語集
17. 参考文献

---

## 0. 通用约定

- **测试框架**：`cargo test` + `testcontainers`（PostgreSQL/Redis 容器化） + `wiremock`（HTTP mock） + `sqlx::test`
- **Mock 策略**：
  - 真实依赖优先：DB、Redis 启容器，HTTP 用 wiremock
  - 浏览器自动化：内嵌静态 HTML + Playwright headless
  - LLM：mock HTTP server 返回固定 JSON
- **入口条件**：对应模块 UT 通过 + 依赖模块的真实实例或 mock 就绪
- **出口条件**：100% 用例执行 + P0/P1 全通过 + 接口契约 100% 覆盖
- **运行**：`cargo test --workspace --features=integration`

---

## 1. 数据采集流水线（M-01 → M-02 → M-03）

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-ACQ-001 | M-01→M-02 | API 模式采集 → 标准化 | 启 wiremock 返回 lark API JSON | M-01 输出 NJson(raw) → M-02 标准化 → payload 含 source.platform=lark | P0 | ✓ |
| TC-IT-ACQ-002 | M-01→M-02 | 浏览器模式采集 → 标准化 | Playwright 访问内嵌 HTML 页面 | 抓取 DOM → 标准化 → payload.type 与 NJSON schema 一致 | P0 | ✓ |
| TC-IT-ACQ-003 | M-01→M-02 | IM 消息发送（双向） | M-09 调用 M-01 send_message | M-01 复用适配器发送，回复消息标准化回 NJson | P0 | ✓ |
| TC-IT-ACQ-004 | M-01→M-02 | 增量同步：续传 100 条 | cursor 存在，仅 5 条新数据 | M-01 只拉 5 条，cursor 更新 | P0 | ✓ |
| TC-IT-ACQ-005 | M-02→M-03 | 标准化 → 数据流引擎入队 | 标准化后 100 条 NJson | M-03 队列收到 100 条，下游消费者顺序收到 | P0 | ✓ |

## 2. 编排与控制流（M-04 ↔ M-05 ↔ M-03）

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-ORC-001 | M-04→M-05 | 编排引擎提交 runnable 给控制流执行器 | 5 个 runnable 节点 | M-05.dispatch 返回 5 个 NodeExecutionResult | P0 | ✓ |
| TC-IT-ORC-002 | M-05→M-04 | 控制流执行器返回结果给编排引擎做状态迁移 | 5 个结果：3 success, 1 failed, 1 skipped | 编排引擎正确迁移状态，failed 触发重试/异常分支 | P0 | ✓ |
| TC-IT-ORC-003 | M-04→M-03 | 编排引擎通过数据流引擎读取上游结果 | 节点 A 完成 → 节点 B 启动 | B 从 M-03 队列读到 A 的输出 | P0 | ✓ |
| TC-IT-ORC-004 | M-05→M-03 | 暂停时数据流引擎队列保留数据 | 节点执行中 Pause | 队列中数据不丢，恢复后继续处理 | P0 | ✓ |
| TC-IT-ORC-005 | M-05→M-04 | Abort 终止后状态机退出 | Abort 信号 | run() 退出，最终 status=aborted | P0 | ✓ |
| TC-IT-ORC-006 | M-04→M-04 | 状态 checkpoint → 重启 → load_latest | kill -9 模拟 | 加载到 checkpoint，恢复到正确状态 | P0 | ✓ |

## 3. 插件与节点运行时（M-06 ↔ M-01/M-02/M-09）

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-PLG-001 | M-06→M-01 | 加载 M-01 适配器作为插件 | 动态加载 lark adapter .so | plugin.execute(input) 调用实际 lark API | P0 | ✓ |
| TC-IT-PLG-002 | M-06→M-02 | 加载 M-02 标准化器作为插件 | 动态加载 normalizer .wasm | 输入原始 JSON，输出标准化 NJson | P0 | ✓ |
| TC-IT-PLG-003 | M-06→M-09 | 加载 M-09 导出器作为插件 | 动态加载 file exporter | 插件执行后文件正确生成 | P0 | ✓ |
| TC-IT-PLG-004 | M-06 | 节点配置 → 连线校验 → 编排 | 前端发请求 validate-edge | 校验后端真正调用 M-06 的 validate_edge_compatibility | P0 | ✓ |

## 4. 触发与执行（M-08 → M-04）

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-TRG-001 | M-08→M-04 | Webhook 触发 → 编排引擎执行 | POST /webhooks/{id}，签名正确 | M-08 dispatch → M-04.run()，返回 execution_id | P0 | ✓ |
| TC-IT-TRG-002 | M-08→M-04 | Cron 触发到点执行 | 时间推进到 cron 时刻 | M-08 自动触发，M-04 开始执行 | P0 | ✓ |
| TC-IT-TRG-003 | M-08→M-04 | 手动触发：mock_input 注入 | trigger 时携带 mock_input | 编排引擎第一个节点收到 mock_input | P0 | ✓ |
| TC-IT-TRG-004 | M-08 | 触发 → M-10 配额检查 → 拒绝 | 配额满 | 触发返回 QuotaError，不进入 M-04 | P0 | ✓ |

## 5. 多租户隔离（贯穿所有模块）

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-MT-001 | 全部 | RLS 隔离：A 租户用户查 B 租户画布 | with_tenant_scope(A) 查询 B 画布 ID | 返回 0 行（无权限） | P0 | ✓ |
| TC-IT-MT-002 | 全部 | 跨事务不残留：A 事务设置 → B 事务查询 | 顺序执行两个事务 | B 看不到 A 的 `app.current_tenant`（详见 [M-10 §3.1](../modules/M-10-tenant-middleware.md)） | P0 | ✓ |
| TC-IT-MT-003 | M-10 | 浏览器实例池：A 满载不影响 B | A 租户 3 实例全占 | B 租户仍可 acquire 新实例 | P0 | ✓ |
| TC-IT-MT-004 | M-10 | 配额：A 满不影响 B | A 配额耗尽 | B 请求仍可成功 | P0 | ✓ |
| TC-IT-MT-005 | M-10 | 删除租户：级联清理 | hard_delete_tenant_data(A) | A 全部数据删除，B 数据不受影响 | P0 | ✓ |
| TC-IT-MT-006 | M-11 | RBAC：A 租户 Owner 访问 B 租户 API | A token + 路径含 B tenant_id | 403 TENANT_MISMATCH | P0 | ✓ |

## 6. 权限与协作（贯穿所有模块）

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-RBAC-001 | M-11×M-13 | 鉴权链：JWT + tenant + RBAC | Editor 角色访问 /admin/tenants | 403 Forbidden | P0 | ✓ |
| TC-IT-RBAC-002 | M-11×M-04 | RBAC：Viewer 触发执行 | role=Viewer, POST /execute | 403 Forbidden | P0 | ✓ |
| TC-IT-RBAC-003 | M-11 | 协作：2 客户端同时编辑 → CRDT 合并 | 2 个 Yjs 客户端并发 | 服务端 Y.Doc 合并后无丢失 | P0 | ✓ |
| TC-IT-RBAC-004 | M-11 | 审计：凭证访问 → audit_log 记录 | 读取 encrypted_payload | audit_log 含 action_type='credential.access' + before/after | P0 | ✓ |

## 7. API Gateway 与前端（M-13 ↔ M-12）

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-FE-001 | M-12→M-13 | 画布加载：REST + WebSocket | 前端打开画布 | REST 拉取定义，WS 订阅 node.status_changed | P0 | ✓ |
| TC-IT-FE-002 | M-12→M-13 | 节点执行状态推送 | 节点完成 | WS 推送 canvas.node.status_changed，前端更新 ECS 状态 | P0 | ✓ |
| TC-IT-FE-003 | M-12→M-13 | 数据流指标推送 | 画布运行 | WS 每 1s 推送 metrics，前端更新流光动效 | P1 | ✓ |
| TC-IT-FE-004 | M-12 | 协作者感知：Yjs 双向同步 | 2 用户同时编辑 | 双方均看到对方光标与改动 | P1 | ✓ |

## 8. 数据库集成

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-DB-001 | M-10 | 全部表 DDL 顺序执行 | 全新 DB 跑 migrations | 所有表创建成功，RLS 策略生效 | P0 | ✓ |
| TC-IT-DB-002 | M-10 | canvas → canvas_version 循环外键 | 应用层先插 version 再更新 canvas.current_version_id | 一致性由应用层保证（详见 [M-10 §4.2](../modules/M-10-tenant-middleware.md)） | P0 | ✓ |
| TC-IT-DB-003 | M-10 | 行级 RLS 跨表关联查询 | 多表 JOIN | 所有 JOIN 都受 tenant_id 过滤 | P0 | ✓ |

## 9. 外部系统集成

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-EXT-001 | M-01 | 飞书/企业微信等真实 API（沙箱环境） | 沙箱环境凭证 | API 调用成功，NJson 正确生成 | P1 | △（依赖沙箱环境） |
| TC-IT-EXT-002 | M-01 | Playwright 真实浏览器访问 | 启 headless Chromium | DOM 抓取、选择器匹配成功 | P0 | ✓ |
| TC-IT-EXT-003 | M-09 | Webhook 输出到 mock HTTP server | 启 wiremock | 收到正确格式的 POST 请求 | P0 | ✓ |
| TC-IT-EXT-004 | M-09 | 数据库输出到真实 PostgreSQL | testcontainers 启 PG | 数据正确写入，可查询 | P0 | ✓ |

## 10. 端到端关键路径（集成层面验证业务完整性）

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-E2E-001 | M-01→M-02→M-03→M-04→M-05→M-09 | 完整数据通路：采集→转换→入队→编排→执行→导出 | 单节点线性画布 | 数据从源平台流向目标导出，全程无丢失 | P0 | ✓ |
| TC-IT-E2E-002 | M-04→M-07 | 执行 → 调试快照 | 画布执行完成 | execution_node_snapshot 表含输入输出引用 | P0 | ✓ |
| TC-IT-E2E-003 | M-08→M-04→M-07→M-11 | 触发 → 执行 → 审计 | 手动触发 + 凭证访问 | audit_log 完整记录触发者、操作、结果 | P0 | ✓ |
| TC-IT-E2E-004 | 全部 | 协作编辑 → 保存 → 加载 | 2 用户编辑后保存 | 新加载的画布包含 2 用户的所有改动（CRDT 合并） | P0 | ✓ |

## 10.5 模块部署 / 事件 / 集群集成（M-14 ↔ M-15 ↔ M-16 ↔ M-13）

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-DEP-001 | M-14→M-15 | 模块注册 → 事件发布 | POST /admin/modules | module.registered 事件 1s 内投递到 M-15 订阅者 | P0 | ✓ |
| TC-IT-DEP-002 | M-14→M-13 | 原子升级 → 路由更新 | atomic_module_swap 完成 | M-13 路由表 5s 内刷新，新版本可见 | P0 | ✓ |
| TC-IT-DEP-003 | M-14→M-06 | 模块级热插拔 | activate m01-acquisition@1.5.0 | 双版本共存，旧版 drain 完成，路由切换 | P0 | ✓ |
| TC-IT-DEP-004 | M-16→M-13 | 节点摘除 → 路由摘除 | node_removed 事件 | M-13 在 30s 内从负载均衡池移除 | P0 | ✓ |
| TC-IT-DEP-005 | M-16→M-15 | Leader 选举 → 事件触发 | leader 变化 | cluster.leader_elected 事件发布 | P0 | ✓ |
| TC-IT-EVT-001 | M-15→M-11 | 审计事件 → audit_log | permission.changed 事件 | audit_log 同步产生记录 | P0 | ✓ |
| TC-IT-EVT-002 | M-15→M-04 | 编排事件订阅 | 编排引擎订阅 module.* | atomic_module_swap 触发时自动更新执行计划 | P1 | ✓ |
| TC-IT-EVT-003 | M-15 | 持久队列溢出 | Redis 满 | 事件入文件后备，告警 | P1 | △ |
| TC-IT-CLU-001 | M-16→M-15 | 心跳失联 → 事件 | 节点失联 30s | cluster.node_unhealthy 事件 | P0 | ✓ |
| TC-IT-CLU-002 | M-16→M-10 | 状态分片 → tenant 隔离 | 10 节点集群 | A 租户请求仅路由到负责其 shard 的节点 | P0 | ✓ |
| TC-IT-CLU-003 | M-16 | Re-balance：节点摘除 | 摘除 1 节点 | 其 shard 自动迁移到剩余节点 | P1 | ✓ |
| TC-IT-DB-004 | M-10 | 全部 DDL（含 M-14/15/16 新表）顺序执行 | 全新 DB 跑 migrations | 所有表（含 module_registry/event_log/cluster_node/leader_lease）创建成功 | P0 | ✓ |
| TC-IT-DB-005 | M-10 | PL/pgSQL 存过在事务内的原子性 | 嵌套调用 register_module + atomic_module_swap | 全部回滚或全部提交，无中间态 | P0 | ✓ |

## 11. 性能 / 资源 IT 维度

| TC ID | 关联模块 | 描述 | 输入/前置 | 预期 | 优先级 | 自动化 |
|---|---|---|---|---|---|---|
| TC-IT-PERF-001 | M-03 | 队列吞吐 | 持续 send 10000 条 | 10000 条全部成功传递，无丢失 | P0 | ✓ |
| TC-IT-PERF-002 | M-10 | 100 并发租户的 RLS 查询 | 启 100 并发 | P95 查询延迟 < 50ms | P1 | ✓ |
| TC-IT-PERF-003 | M-12 | 1000 节点画布加载 + 编辑 | 1000 节点 5000 边 | 加载 < 2s，编辑帧率 ≥ 30fps | P0 | ✓ |

## 12. 质量门禁

- **接口契约覆盖率**：100%（所有模块导出 API 都有对应 IT 用例）
- **P0 用例通过率**：100%
- **P1 用例通过率**：100%
- **IT 套件总时长**：≤ 15 分钟（CI 卡点）
- **数据库容器复用**：testcontainers 跨用例共享容器实例

## 13. 持续集成

- IT 套件在每次 PR 合并前必须通过
- 每日定时任务执行完整 IT + 生成报告
- 不通过 PR 阻塞：任意 P0 失败 / 接口契约覆盖 < 100% / 套件超时
- 报告产物：`target/test-report/integration.html`

---

## 15. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 結合テスト | Integration Test、モジュール間連携 | §0 |
| インターフェース契約 | モジュール間の入出力規約 | §0 |
| testcontainers | テスト用コンテナ起動 | §0 |
| 実例依存 | 真实 DB/Redis 利用 | §0 |
| 跨事务边界 | with_tenant_scope の SET LOCAL スコープ | §6 |
| E2E 关键路径 | データソースから出力までの完全パス | §11 |
| 1000 ノード 5000 エッジ | 性能テストの基準規模 | §12 |
| 100 并发租户 | マルチテナント性能基準 | §12 |
| 套件时长 | IT 全実行時間 ≤ 15 分 | §13 |
| 容器复用 | testcontainers の共有インスタンス | §13 |

## 16. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. testcontainers-rs 公式ドキュメント
4. wiremock 公式ドキュメント
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 基本設計書 v1.3.0」、2026-08-18（[DOC-BSC-001](../legacy/basic-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
