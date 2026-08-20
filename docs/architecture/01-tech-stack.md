# 技術スタック推奨

> **ドキュメントID**：DOC-ARCH-002
> **文書分類**：横断文書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/basic-design.md`（DOC-BSC-001）
> **下位文書**：`docs/modules/M-12`（DOC-MOD-012）
> **関連文書**：`docs/architecture/00-anatomy-model.md`（DOC-ARCH-001）、`docs/architecture/02-deployment.md`（DOC-ARCH-003）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018)
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-18 | 初版制定（基本設計書 §9 抽出） | Ada プロジェクトチーム | TBD | TBD |
| v1.1.0 | 2026-08-19 | IPA 準拠メタデータ追加、NF タグ付与 | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 概要
2. 选型表
3. 关键选型理由
4. 风险与回退路径
5. 用語集
6. 参考文献

---

## 1. 概要

本文汇总各层推荐技术选型及备选方案，每项选型附 IPA 非機能要求グレード标签，以便在性能、可移植性、可维护性、可用性维度上做权衡追溯。

## 2. 选型表

| 层级 | 推荐选技术 | 备选方案 | NF タグ |
|---|---|---|---|
| 画布渲染引擎 | Bevy（ECS）+ bevy_egui，编译至 `wasm32-unknown-unknown` | React + Konva.js/Pixi.js | [NF-PER]【必須】 |
| 配置表单/中文输入层 | HTML Overlay（`web-sys` + 原生 DOM，与 Bevy 画布坐标同步） | 无（IME 支持是刚性约束） | [NF-OPS]【必須】 |
| 实时协作 | `yrs`（Yjs 的 Rust 移植） | Yjs (JS) + y-websocket，Automerge | [NF-OPS]【必須】 |
| 后端 API | Actix-web (Rust) | Tokio + Tonic (gRPC), Axum | [NF-PER]【必須】 |
| 编排引擎 | Rust 自研状态机 | LangGraph (Python) 移植 | [NF-AVA]【必須】 |
| 浏览器自动化 | Playwright Rust 绑定 | Puppeteer (Node.js) | [NF-PER]【必須】 |
| 数据库 | PostgreSQL 12+ | MySQL 8.0+, MariaDB | [NF-SEC]【必須】 |
| 缓存 | Redis 6+ | Memcached, Apache Druid | [NF-PER]【推奨】 |
| 对象存储 | AWS S3 / MinIO | Azure Blob, Google Cloud Storage | [NF-ENV]【推奨】 |
| 消息队列 | Tokio Channel / crossbeam | RabbitMQ, Apache Kafka | [NF-PER]【推奨】 |
| 容器化 | Docker + Kubernetes | Docker Swarm, Nomad | [NF-MIG]【必須】 |
| 监控日志 | Prometheus + Grafana + ELK | DataDog, New Relic | [NF-OPS]【必須】 |

## 3. 关键选型理由

### 3.1 前端混合渲染（Bevy + HTML Overlay）

- 前后端统一 Rust 语言栈，`CanvasDefinition`/`NJson` 等核心数据结构可直接前后端共享。
- Bevy 的 ECS 架构与 GPU 渲染管线天然契合"节点=实体、连线=关系"的画布模型。
- **混合渲染的关键决策**——bevy_egui 对中文/日文等需要输入法组合键的语言支持历史上不够成熟，采用分层策略：
  - 画布本体（节点卡片、连线、缩放平移、框选）→ Bevy + bevy_egui
  - 工具栏、快捷操作菜单、简单数值/开关参数 → bevy_egui
  - 节点详细配置表单 → **HTML Overlay**
  - 调试面板 → HTML Overlay
- 仍以"浏览器打开即用"的网页形式交付，满足 F-09 免安装要求。 [NF-MIG]【必須】

### 3.2 后端

- Actix-web：高吞吐异步运行时，tokio 生态深度整合。 [NF-PER]【必須】
- Playwright Rust 绑定：直接驱动 Chrome DevTools Protocol。

### 3.3 数据层

- PostgreSQL 12+：行级安全（RLS）是多租户隔离的关键能力。 [NF-SEC]【必須】
- Redis：会话、临时数据、速率限制计数、溢出缓冲。 [NF-PER]【推奨】
- S3/MinIO：执行快照、采集原始 HTML/截图等大体积数据。 [NF-ENV]【推奨】

## 4. 风险与回退路径

- **Bevy 生态成熟度低于 React 生态**：保留 React + Konva.js 备选路径作为团队能力不足时的回退方案。
- **Bevy WASM 包体积偏大**：裁剪未使用的 Bevy 默认 feature（3D 渲染、音频等）；启用 `wasm-opt` 压缩。 [NF-ENV]【必須】
- **bevy_egui 中文输入法支持不成熟**：见 §3.1 混合渲染分层策略。 [NF-OPS]【必須】

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| Bevy | Rust 製のゲームエンジン / ECS フレームワーク | §2 |
| bevy_egui | Bevy 用 egui バインディング（即時モード GUI） | §2、§3.1 |
| HTML Overlay | Bevy Canvas 上に重ねる DOM 層、IME 互換性確保用 | §3.1 |
| yrs | Yjs の Rust 移植版 CRDT ライブラリ | §2 |
| Actix-web | Rust の高スループット Web フレームワーク | §2、§3.2 |
| LangGraph | LLM アプリケーション向け状態グラフライブラリ | §2 |
| Playwright | Chromium ブラウザ自動化ライブラリ | §2、§3.2 |
| RLS (PostgreSQL) | Row-Level Security、行レベルセキュリティ | §3.3 |
| クロスプラットフォーム | 複数 OS での動作 | §4 |
| WASM | WebAssembly | §4 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Bevy 公式ドキュメント「Bevy — A refreshingly simple data-driven game engine」
4. PostgreSQL Global Development Group「PostgreSQL Documentation」
5. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 基本設計書 v1.3.0」、2026-08-18（[DOC-BSC-001](../legacy/basic-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
