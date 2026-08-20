# デプロイメント戦略

> **ドキュメントID**：DOC-ARCH-003
> **文書分類**：横断文書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/basic-design.md`（DOC-BSC-001）
> **下位文書**：`docs/modules/M-10`（DOC-MOD-010）
> **関連文書**：`docs/architecture/01-tech-stack.md`（DOC-ARCH-002）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 7 章「運用・保守プロセス」
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-18 | 初版制定（基本設計書 §8 抽出） | Ada プロジェクトチーム | TBD | TBD |
| v1.1.0 | 2026-08-19 | IPA 準拠メタデータ追加、NF タグ付与 | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 概要
2. 単机本地模式
3. 多租户 SaaS 模式
4. 混合部署
5. 部署相关非功能要件
6. 用語集
7. 参考文献

---

## 1. 概要

本文定义系统三种部署形态（单机本地 / 多租户 SaaS / 混合）及对应的非功能要求，每项均按 IPA「非機能要求グレード」标注等级。

## 2. 単机本地模式

**目标用户**：个人开发者、小团队

```
User PC
  ├─ Ada Runtime (单一可执行文件)
  │  ├─ Frontend Web Server (静态资源)
  │  ├─ API Server (本地 HTTP)
  │  ├─ Orchestration Engine
  │  └─ SQLite 数据库
  │
  └─ 浏览器 (访问 http://localhost:8000)
```

**特点**：

- 零安装，零依赖（除浏览器内核） [NF-ENV]【必須】
- 数据本地存储，隐私优先
- 支持数据导出为 JSON/CSV 备份
- 满足 F-09 免安装要求 [NF-MIG]【必須】

## 3. 多租户 SaaS 模式

**目标用户**：企业、SaaS 服务商

```
互联网
  └─ CDN (前端静态资源)
      └─ API Gateway (Nginx/HAProxy)
          └─ Kubernetes 集群
              ├─ Pod: API Server (副本集)
              ├─ Pod: Orchestration Engine (副本集)
              ├─ Pod: Node Runtime Pool (自动扩容)
              ├─ Pod: WebSocket Gateway
              │
              └─ 存储
                  ├─ PostgreSQL (RDS)
                  ├─ Redis (缓存)
                  └─ S3 (对象存储)
```

**特点**：

- 自动扩容缩容（基于 CPU/内存/队列长度） [NF-PER]【必須】
- 多租户隔离（命名空间、网络策略） [NF-SEC]【必須】
- 高可用（多副本、健康检查、自动故障转移） [NF-AVA]【必須】
- 多租户隔离的三层模型详见 [M-10 §4](../modules/M-10-tenant-middleware.md)

## 4. 混合部署

支持企业内网部署：

- 私有 Docker 镜像库
- 离线安装包（包含依赖）
- Air-gapped 环境部署指南

## 5. 部署相关非功能要件

来自 [requirements §7.4 移行性](../legacy/requirements.md)：

- 跨操作系统：Runtime 需支持 Windows、macOS、Linux 三大主流桌面操作系统。 [NF-MIG]【必須】
- 配置可移植：画布配置文件应可在不同机器间直接复制迁移并正常运行（不依赖机器绑定的硬编码路径）。 [NF-MIG]【必須】
- 版本兼容：标准化 JSON Schema 需具备版本号（`schema_version`），保证向后兼容，旧版本数据可被新版本 Runtime 正确解析。 [NF-MIG]【必須】

来自 [requirements §7.6 システム環境](../legacy/requirements.md)：

- 运行环境：本地个人电脑（4 核 CPU / 8GB 内存起）即可运行基本功能；浏览器自动化采集功能建议 16GB 内存以支持多实例并发。 [NF-ENV]【必須】
- 依赖最小化：除浏览器内核（按需下载）外，不应引入需要用户手动安装的外部依赖（如 JVM、.NET Runtime 等）。 [NF-ENV]【必須】
- 资源占用可控：Runtime 空闲状态下 CPU 占用应低于 5%，内存占用应低于 300MB（不含已加载的浏览器实例）。 [NF-ENV]【推奨】

## 6. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 単机本地模式 | 単一 PC 上で完結する配置形態 | §2 |
| 多租户 SaaS 模式 | 単一インスタンスで複数テナントにサービス提供 | §3 |
| 混合部署 | 企業内網 + オフライン環境向け配置 | §4 |
| 高可用 (HA) | High Availability、複数副本 + 自動フェイルオーバ | §3 |
| Namespace 隔離 | Kubernetes namespace による多租户隔離 | §3 |
| Air-gapped | 外部ネットワークから完全に隔離された環境 | §4 |
| RDS | Relational Database Service、AWS のマネージド PostgreSQL | §3 |
| CDN | Content Delivery Network | §3 |
| 移行性 | Portability、異なる環境への移動容易性 | §5 |

## 7. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Kubernetes 公式ドキュメント「Kubernetes — Production-Grade Container Orchestration」
4. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 基本設計書 v1.3.0」、2026-08-18（[DOC-BSC-001](../legacy/basic-design.md)）
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 要件定義書 v1.2.1」、2026-08-18（[DOC-REQ-001](../legacy/requirements.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
