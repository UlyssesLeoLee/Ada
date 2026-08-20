# Admin API - 模块管理（Module Management）

> **ドキュメントID**：DOC-API-004
> **文書分類**：横断文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/04-atomic-deployment.md`（DOC-ARCH-005）、`docs/modules/M-14`（DOC-MOD-014）
> **下位文書**：無
> **関連文書**：`docs/api/admin-events.md`（DOC-API-005）、`docs/api/admin-cluster.md`（DOC-API-006）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018)
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（モジュール管理 API） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 概要
2. 认证与授权
3. 端点清单
4. 关键端点详解
5. 错误码
6. 用語集
7. 参考文献

---

## 1. 概要

本文定义 `/api/v1/admin/modules/*` 命名空间下的 API 端点，承载 [DOC-MOD-014](../modules/M-14-module-registry.md) 模块注册与生命周期管理能力。NF セキュリティ：[NF-SEC]【必須】、可用性：[NF-AVA]【必須】。

## 2. 认证与授权

- **认证**：Bearer JWT（与 [DOC-API-001](rest-endpoints.md) 一致）
- **授权**：必须满足以下任一角色
  - `Owner`（租户内）
  - `PlatformAdmin`（集群级，跨租户）
- **审计**：所有写操作经 [DOC-MOD-011 §3.3](../modules/M-11-rbac-collab.md) 审计

## 3. 端点清单

```
# 模块清单与查询
GET    /api/v1/admin/modules                              # 列出所有模块
GET    /api/v1/admin/modules/:module_id                   # 模块详情
GET    /api/v1/admin/modules/:module_id/versions          # 版本历史

# 注册与升级
POST   /api/v1/admin/modules                              # 注册新模块（带 manifest）
POST   /api/v1/admin/modules/:module_id/upgrade           # 触发升级
GET    /api/v1/admin/modules/upgrades/:plan_id            # 升级进度
POST   /api/v1/admin/modules/upgrades/:plan_id/abort      # 中止升级
POST   /api/v1/admin/modules/:module_id/rollback           # 回滚到上一版本

# 实例与状态
GET    /api/v1/admin/modules/:module_id/instances          # 列出实例
GET    /api/v1/admin/modules/:module_id/instances/:node_id # 单实例详情
POST   /api/v1/admin/modules/instances/:node_id/:module_id/drain  # 排空
POST   /api/v1/admin/modules/instances/:node_id/:module_id/unload # 卸载
```

## 4. 关键端点详解

### 4.1 POST /api/v1/admin/modules - 注册新模块

**Request Body**：

```json
{
  "manifest": {
    "meta": {
      "module_id": "m01-acquisition",
      "version": "1.5.0",
      "display_name": "采集适配器",
      "description": "..."
    },
    "deps": { ... },
    "entry": { "type": "wasm", "artifact_url": "s3://...", "sha256": "..." },
    "api": { "routes": [...] },
    "resources": { ... },
    "state": { "kind": "stateless" }
  }
}
```

**Response 201**：

```json
{
  "module_id": "m01-acquisition",
  "version": "1.5.0",
  "instance_id": "uuid",
  "state": "Registered",
  "registered_at": "2026-08-19T10:00:00Z"
}
```

**Response 409**（同 module_id+version 已存在）：

```json
{
  "error_code": "MODULE_ALREADY_EXISTS",
  "module_id": "m01-acquisition",
  "version": "1.5.0"
}
```

**处理**：调用 PL/pgSQL `register_module()` 存过，写 `module_registry` 表 + 触发 `module.registered` 事件。

### 4.2 POST /api/v1/admin/modules/:module_id/upgrade - 触发升级

**Request Body**：

```json
{
  "to_version": "1.5.0",
  "strategy": "rolling",
  "batch_size": 1,
  "health_check_window_seconds": 60,
  "rollback_on_failure": true,
  "canary_stages": [5, 25, 50, 100]   // 仅 strategy=canary
}
```

**Response 202**：

```json
{
  "plan_id": "uuid",
  "module_id": "m01-acquisition",
  "from_version": "1.4.2",
  "to_version": "1.5.0",
  "strategy": "rolling",
  "status": "InProgress",
  "started_at": "2026-08-19T10:00:00Z",
  "estimated_duration_seconds": 180
}
```

### 4.3 GET /api/v1/admin/modules/upgrades/:plan_id - 升级进度

**Response 200**：

```json
{
  "plan_id": "uuid",
  "status": "InProgress",
  "current_step": "draining node-3 (5/12)",
  "progress": {
    "total_nodes": 12,
    "completed_nodes": 5,
    "failed_nodes": 0,
    "current_node": "node-3"
  },
  "started_at": "2026-08-19T10:00:00Z",
  "elapsed_seconds": 145,
  "events": [
    { "at": "...", "node": "node-1", "step": "drained", "duration_s": 12 },
    { "at": "...", "node": "node-2", "step": "drained", "duration_s": 15 }
  ]
}
```

### 4.4 POST /api/v1/admin/modules/:module_id/rollback - 回滚

**Response 202**：

```json
{
  "plan_id": "uuid",
  "module_id": "m01-acquisition",
  "from_version": "1.5.0",
  "to_version": "1.4.2",
  "status": "InProgress"
}
```

调用 PL/pgSQL `atomic_module_swap()` 存过 + 触发 rolling 升级流程。

## 5. 错误码

| Error Code | HTTP Status | 説明 | NF タグ |
|---|---|---|---|
| `MODULE_ALREADY_EXISTS` | 409 | 同 module_id+version 已注册 | [NF-OPS]【必須】 |
| `MODULE_NOT_FOUND` | 404 | 模块不存在 | [NF-OPS]【必須】 |
| `INVALID_MANIFEST` | 400 | manifest 校验失败 | [NF-SEC]【必須】 |
| `VERSION_CONFLICT` | 409 | 目标版本已被激活 | [NF-OPS]【必須】 |
| `DEPENDENCY_UNSATISFIED` | 412 | 依赖不满足 | [NF-OPS]【必須】 |
| `UPGRADE_IN_PROGRESS` | 409 | 模块正在升级 | [NF-AVA]【必須】 |
| `ARTIFACT_HASH_MISMATCH` | 422 | artifact sha256 不匹配 | [NF-SEC]【必須】 |
| `INSUFFICIENT_PERMISSIONS` | 403 | 无 Owner/PlatformAdmin 角色 | [NF-SEC]【必須】 |
| `INVALID_STATE_TRANSITION` | 409 | 状态机非法转移 | [NF-SEC]【必須】 |
| `UPGRADE_TIMEOUT` | 504 | 升级超时（默认 60s） | [NF-AVA]【必須】 |
| `HEALTH_CHECK_FAILED` | 503 | 健康检查失败已回滚 | [NF-AVA]【必須】 |

## 6. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 模块管理 API | /api/v1/admin/modules/* 端点群 | §1 |
| Manifest | 模块元数据声明 | §4.1 |
| 升级策略 | rolling/blue-green/canary/recreate | §4.2 |
| 升级计划 (plan) | 一次升级操作的元数据 | §4.2 |
| 排空 (Drain) | 停止接收新流量 | §4.5 |
| 回滚 | Rollback、回到上一版本 | §4.4 |
| 注册 (Register) | 元数据入库 | §4.1 |
| 健康检查窗口 | health check 的等待时长 | §4.2 |
| 灰度阶段 | Canary 各阶段比例 | §4.2 |
| 路由 | API 路由 | §4.1 |
| 审计 | 写操作记录 | §2 |

## 7. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Ada プロジェクトチーム「モジュール登録とライフサイクル v1.0.0」、2026-08-19（[DOC-MOD-014](../modules/M-14-module-registry.md)）
4. Ada プロジェクトチーム「原子化部署アーキテクチャ v1.0.0」、2026-08-19（[DOC-ARCH-005](../architecture/04-atomic-deployment.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
