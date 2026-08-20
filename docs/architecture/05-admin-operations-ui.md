# 管理员运维界面规范

> **ドキュメントID**：DOC-ARCH-006
> **文書分類**：横断文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/04-atomic-deployment.md`（DOC-ARCH-005）
> **下位文書**：`docs/api/admin-modules.md`（DOC-API-004）、`docs/api/admin-events.md`（DOC-API-005）、`docs/api/admin-cluster.md`（DOC-API-006）
> **関連文書**：`docs/modules/M-12`（DOC-MOD-012 业务前端）、`docs/modules/M-11`（DOC-MOD-011 RBAC）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 7 章「運用・保守プロセス」
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（Admin UI 規範） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 背景与定位
2. 角色与权限
3. 页面架构
4. 模块管理页
5. 事件中心页
6. 集群拓扑页
7. 审计与回放
8. 实时面板
9. 与业务前端的关系
10. 验收要点
11. 用語集
12. 参考文献

---

## 1. 背景与定位

业务前端 [M-12](../modules/M-12-canvas-editor-frontend.md) 面向**画布设计者**（运营、业务分析师），关注"如何编排数据流"。本界面面向 **SRE / 平台管理员**，关注"系统如何被部署、监控、应急"。

| 维度 | 业务前端 (M-12) | 运维界面 (本文档) |
|---|---|---|
| 角色 | Editor / Executor / Viewer | Owner / PlatformAdmin |
| 入口 | `https://<host>/` | `https://<host>/admin` |
| 关注 | 画布/节点/数据流 | 模块/事件/集群/审计 |
| 美学 | 类 Miro/MI 风格 | 类 Grafana/K8s Dashboard |

## 2. 角色与权限

| 角色 | 业务前端可见 | 运维界面可见 | 业务范围 |
|---|---|---|---|
| Viewer | ✓ | ✗ | 读 |
| Editor | ✓ | ✗ | 读/写画布 |
| Executor | ✓ | ✗ | 读/触发执行 |
| Owner | ✓ | △（仅租户内） | 全部业务 + 租户内管理 |
| **PlatformAdmin** | △（可选） | ✓ | 集群级管理（跨租户） |

`PlatformAdmin` 不属于 `tenant_user` 表，是系统级账号，存在 `platform_admin` 表（与 M-10 的多租户模型正交）。登录后切换到 `/admin` 路由加载本界面。

## 3. 页面架构

```
/admin
├── /dashboard              # 总览：集群健康 + 事件流
├── /modules
│   ├── /                   # 模块列表
│   ├── /:module_id         # 模块详情（版本/状态/依赖）
│   ├── /upload             # 上传新版本
│   └── /:module_id/upgrade # 升级向导（灰度/蓝绿/可插拔）
├── /events
│   ├── /                   # 事件流（实时+过滤）
│   ├── /topics             # 主题列表
│   ├── /replay             # 事件重放向导
│   └── /subscriptions      # 订阅管理
├── /cluster
│   ├── /                   # 节点拓扑图
│   ├── /:node_id           # 节点详情（CPU/内存/流量/任务）
│   └── /leaders            # Leader 选举状态
├── /audit
│   ├── /                   # 审计日志（按租户/操作/资源过滤）
│   └── /replay             # 操作回放
├── /system
│   ├── /health             # 系统健康总览
│   ├── /config             # 全局配置（读为主）
│   └── /about              # 版本/许可/合规信息
└── /profile                # 当前管理员信息
```

## 4. 模块管理页

### 4.1 列表视图

| 列 | 数据来源 | 实时性 |
|---|---|---|
| 模块名 | `module_registry` | 静态 |
| 当前版本 | `module_version` 状态=active | 5s 轮询 |
| 健康度 | `module_instance.heartbeat` | 5s 轮询 |
| 节点数 | `cluster_node` JOIN `module_instance` | 10s 轮询 |
| 流量（QPS） | `module_metrics` Prometheus | 5s |
| 上次更新 | `module_version.updated_at` | 静态 |
| 操作 | — | 按钮：升级 / 回滚 / 暂停 / 卸载 |

支持多租户过滤、全局搜索、批量操作。

### 4.2 详情页

- **基本信息**：Manifest 全文、依赖图、API 路由清单
- **版本历史**：所有已注册版本、发布时间、checksum、迁移说明
- **实例分布**：每节点上的实例数、运行时长、健康曲线
- **资源占用**：CPU/内存/网络/磁盘实时与历史
- **事件时间线**：仅显示该模块的事件

### 4.3 升级向导

```
1. 选择目标版本
2. 选择升级策略
   - 滚动 (rolling)    0 0/0 副本
   - 蓝绿 (blue-green) 0/0 新建
   - 灰度 (canary)     5% → 25% → 50% → 100%
   - 重建 (recreate)   全部停机后启动
3. 配置灰度参数
   - 健康检查窗口
   - 失败回滚触发条件
4. 预览影响范围
5. 确认执行
```

执行时展示实时进度（drain 状态、新副本就绪、流量切换比例），可一键中止回滚。

## 5. 事件中心页

### 5.1 实时流视图

类似 K8s Events 风格，左侧实时追加（WebSocket），右侧按 topic 过滤：

```
[10:32:15] cluster.leader_elected  m04-orchestrator → node-7
[10:32:14] module.registered         m01-acquisition@1.5.0
[10:32:13] module.draining           m01-acquisition@1.4.2 (0 inflight)
[10:32:10] execution.failed          canvas-123 node-5 (selector_not_found)
...
```

每行可点击展开完整 payload，支持：
- 按 topic 过滤（含通配符 `module.*`）
- 按时间窗口过滤
- 按租户/严重级别过滤
- 暂停/恢复实时流
- 导出为 JSONL

### 5.2 重放向导

- 选择 topic
- 选择时间范围 / 起始 `event_seq`
- 选择目标订阅者（测试 consumer / 重建衍生数据）
- 速率限制（防止打爆下游）
- 干跑模式（仅记录不实际投递）

## 6. 集群拓扑页

### 6.1 拓扑可视化

```
            ┌───────────┐
            │  Admin UI │
            └─────┬─────┘
                  │ HTTPS
       ┌──────────┴──────────┐
       ▼                     ▼
┌─────────────┐       ┌─────────────┐
│  node-1     │       │  node-2     │
│  m01, m02   │       │  m01, m03   │
│  leader:m04 │       │  standby    │
│  cpu: 45%   │       │  cpu: 30%   │
└──────┬──────┘       └──────┬──────┘
       └──────────┬──────────┘
                  ▼
         ┌────────────────┐
         │  PostgreSQL    │
         │  + Redis       │
         └────────────────┘
```

- 节点用方块表示，按角色着色（leader 描边、standby 半透明、crashed 红色）
- 模块实例用方块内的小圆点表示
- 连线表示网络通信/数据流
- 鼠标悬停显示 hover 详情，点击进入节点/模块详情页

### 6.2 节点详情

| 区域 | 内容 |
|---|---|
| 概览 | 节点 ID / hostname / IP / 启动时间 / 运行时长 / 标签 |
| 资源 | CPU/内存/网络/磁盘实时 + 24h 趋势图（Prometheus 拉取） |
| 模块实例 | 该节点运行的所有模块实例 + 各自状态 |
| 流量 | 入/出流量 QPS、错误率、延迟 P95/P99 |
| 日志 | 节点级日志（WebSocket 实时） |
| 操作 | 重启 Runtime / 摘除节点 / 重新加入 |

## 7. 审计与回放

### 7.1 审计日志

延续 [M-11 §3.3](../modules/M-11-rbac-collab.md) 的 `audit_log` 表。UI 增强：

- 按时间线浏览（默认最近 7 天）
- 按操作者 / 资源 / 操作类型过滤
- 全文搜索（payload 内的关键字）
- 单条详情：操作前/后状态 JSON diff

### 7.2 操作回放

为"灾难恢复"场景设计：

- 选定时间窗口 → 提取该窗口所有写操作
- 展示为可执行脚本（dry-run 默认开启）
- 在沙箱环境执行验证后，生产环境回放

> ⚠️ 操作回放有副作用风险，仅 PlatformAdmin 可触发，需二次确认 + 全程审计。

## 8. 实时面板

Dashboard 顶部固定区域，always-visible：

- **集群在线节点数** / 总节点数（颜色编码：绿/黄/红）
- **当前 Leader 分布**（按模块）
- **最近 5 分钟事件速率**（按 topic 堆叠图）
- **未处理告警**（P0/P1 计数）
- **当前进行中的部署**（数量 + 进度）

## 9. 与业务前端的关系

- **代码层面**：复用 M-12 的 Bevy + HTML Overlay 混合渲染，但 Admin 页面以 HTML 为主（管理面少有高 DPI 画布需求）
- **状态隔离**：业务前端使用 `TenantState` Resource；Admin 前端使用 `AdminState` Resource
- **入口切换**：顶部导航条根据当前用户角色显示 `/canvas` 或 `/admin` 或两者
- **同源后端**：Admin API 与业务 API 共享 M-13 API Gateway，仅路径前缀与权限不同

## 10. 验收要点

1. **可观测性**：集群任一节点 / 模块 / 事件的健康状态均可在 3 次点击内查到。
2. **零停机升级**：通过灰度策略升级 m01-acquisition 到 1.5.0，整个过程业务无中断。 [NF-AVA]【必須】
3. **事件可重放**：选定 topic + 时间窗口后能在 5 分钟内完成重放配置。 [NF-AVA]【必須】
4. **角色隔离**：Viewer 角色访问 `/admin` 返回 403。 [NF-SEC]【必須】
5. **审计完整性**：所有写操作（包括 UI 上的部署按钮）都产生 `audit_log` 记录。 [NF-SEC]【必須】
6. **多租户隔离**：Admin 界面在切换租户后只看到该租户的模块/事件。 [NF-SEC]【必須】

## 11. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 运维界面 | Admin Ops UI、SRE/平台管理员使用 | §1 |
| PlatformAdmin | 系统级管理员账号 | §2 |
| 灰度发布 | Canary、按比例切换流量 | §4.3 |
| 蓝绿部署 | Blue-Green、双版本并存 | §4.3 |
| 事件重放 | Event Replay、历史事件重新投递 | §5.2 |
| 拓扑图 | 节点与模块的图状展示 | §6.1 |
| 操作回放 | Action Replay、审计操作的二次执行 | §7.2 |
| 告警 | 异常通知（event severity = alert） | §8 |
| 心跳 | 节点存活信号 | §6.1 |
| 角色 | RBAC 角色（Viewer/Editor/Executor/Owner/PlatformAdmin） | §2 |
| 业务前端 | 画布设计者使用的前端 | §9 |
| 升级向导 | 部署引导式 UI | §4.3 |

## 12. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Kubernetes Dashboard 公式「Kubernetes Dashboard」
4. Grafana 公式「Grafana Dashboards」
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）
6. Ada プロジェクトチーム「原子化部署アーキテクチャ v1.0.0」、2026-08-19（[DOC-ARCH-005](04-atomic-deployment.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
