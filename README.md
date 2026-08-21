# Ada 无限画布跨平台数据集成系统

> **无コード数据集成平台** — 让业务人员用画布连接数据源、转换、输出，无需写代码。

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust 1.74+](https://img.shields.io/badge/Rust-1.74%2B-orange.svg)](rust-toolchain.toml)
[![IPA SLCP-JCF2018](https://img.shields.io/badge/IPA-SLCP--JCF2018-blue.svg)](docs/architecture/08-workflow-overview.md)
[![Cargo Workspace](https://img.shields.io/badge/Cargo-Workspace-blue.svg)](Cargo.toml)

> 🚧 **Status**: v0.1.0 scaffold (G0/G1 通过，G4 实施着手判定待 UN-P0-01〜11 解决)

---

## ✨ 核心特性

- 🎨 **无代码画布**：Bevy 0.14 WASM 前端，拖拽式 ETL 构造
- ⚡ **实时数据流**：事件驱动 + 分布式 Pub/Sub，毫秒级延迟
- 🔌 **插件化扩展**：Rust → WASM 沙箱，1 周内可新增数据源/节点
- 🏢 **企业级多租户**：PostgreSQL 16 + RLS + 审计日志，GDPR/PIPL 合规
- 🦀 **Rust 全栈**：16 crate + 2 共享 crate，类型安全 + 高性能
- 🌐 **跨平台**：macOS / Linux / Windows 3 OS，3 种部署模式

## 📦 仓库结构

```
ada/
├── Cargo.toml                 # Cargo Workspace 根配置（18 crate）
├── rust-toolchain.toml        # Rust 1.74+ 固定
├── .gitignore
├── LICENSE                    # MIT License
├── README.md                  # 本文件
├── scripts/
│   └── dev-setup.ps1          # 開発環境セットアップ（Windows）
├── crates/                    # 18 Rust crate
│   ├── ada-core/              # 共有型・エラー処理
│   ├── ada-telemetry/         # 構造化ログ・トレース・メトリクス
│   ├── ada-m01-acquisition/   # M-01 データ取得
│   ├── ada-m02-normalizer/    # M-02 標準化
│   ├── ada-m03-data-flow-engine/
│   ├── ada-m04-orchestration/
│   ├── ada-m05-control-flow/
│   ├── ada-m06-plugin-sdk/     # プラグイン SDK（WASM）
│   ├── ada-m07-debug/         # デバッグサービス
│   ├── ada-m08-trigger/       # トリガー
│   ├── ada-m09-exporter/      # エクスポート
│   ├── ada-m10-tenant-middleware/  # 11 テーブル + 6 PL/pgSQL
│   ├── ada-m11-rbac-collab/   # RBAC + Yrs CRDT
│   ├── ada-m12-canvas-editor/ # Bevy 0.14 WASM
│   ├── ada-m13-api-gateway/   # axum + utoipa
│   ├── ada-m14-module-registry/    # atomic swap
│   ├── ada-m15-central-event-bus/  # Pub/Sub
│   └── ada-m16-cluster-coordinator/  # リーダー選出
└── docs/                      # IPA 準拠ドキュメント
    ├── README.md              # ドキュメント索引
    ├── CHANGELOG.md
    ├── template.md            # IPA 標準テンプレート
    ├── architecture/          # 9 横切文書
    ├── modules/               # 16 モジュール設計
    ├── api/                   # 6 API 仕様
    ├── tests/                 # 試験設計
    ├── templates/             # 62 雛形
    ├── upstream/              # 9 上流工程
    ├── requirements/          # 10 要件細分
    ├── management/            # 5 プロジェクト管理
    ├── business/              # 1 業務シナリオ集
    ├── decisions/             # 11 P0 + 15 D-ADR
    └── legacy/                # 旧版归档
```

## 🚀 快速开始

### 前置要求

- Rust **1.74+** （[rustup](https://rustup.rs/) でインストール）
- Git 2.40+
- （オプション）PostgreSQL 16+ / Docker / Node.js 20+（WASM ビルド用）

### 1. リポジトリ取得

```bash
git clone https://github.com/UlyssesLeoLee/ada.git
cd ada
```

### 2. 開発環境セットアップ（Windows）

```powershell
# 完全セットアップ
.\scripts\dev-setup.ps1

# 環境チェックのみ
.\scripts\dev-setup.ps1 -Check

# Docker 不要
.\scripts\dev-setup.ps1 -SkipDocker
```

**macOS / Linux** は[docs/templates/04-runbooks.md §A.1](../docs/templates/04-runbooks.md) を参照。

### 3. ビルド & テスト

```bash
# 全 crate タイプチェック
cargo check --workspace

# 全 crate ビルド
cargo build --workspace

# 全 crate テスト
cargo test --workspace

# 特定 crate
cargo test -p ada-core
cargo build -p ada-m13-api-gateway

# Lint
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

### 4. Hello World 起動（G4 通過後）

```bash
# API Gateway
cargo run -p ada-m13-api-gateway

# 別ターミナルで health check
curl http://localhost:8080/health
```

## 🏗️ アーキテクチャ

仿生モデル（4 層）の詳細: [docs/architecture/00-anatomy-model.md](docs/architecture/00-anatomy-model.md)

```
┌─────────────────────────────────────────────┐
│ 筋肉: 動的拡張                                │
│  M-06 プラグイン / M-12 Canvas / M-14 Registry│
├─────────────────────────────────────────────┤
│ 神経: イベント・制御                           │
│  M-15 EventBus / M-04 Orchestration / M-08 Trg│
├─────────────────────────────────────────────┤
│ 血: データの流れ                                │
│  M-01 取得 / M-02 標準化 / M-03 実行 / M-09 出力│
├─────────────────────────────────────────────┤
│ 骨: 静的構造                                   │
│  M-10 ストレージ / M-16 クラスタ / M-13 GW      │
└─────────────────────────────────────────────┘
```

## 📊 16 モジュール

| 層 | モジュール | 機能 | 設計書 |
|---|---|---|---|
| 共有 | ada-core | 共有型・エラー | [M-10 §2](../docs/modules/M-10-tenant-middleware.md) |
| 共有 | ada-telemetry | ログ・トレース | [DOC-ARCH-007 §10](../docs/architecture/06-rust-tech-selection.md) |
| 骨 | ada-m10-tenant-middleware | 11 テーブル + RLS + 6 PL/pgSQL | [M-10](../docs/modules/M-10-tenant-middleware.md) |
| 骨 | ada-m11-rbac-collab | RBAC + Yrs CRDT | [M-11](../docs/modules/M-11-rbac-collab.md) |
| 骨 | ada-m13-api-gateway | axum + utoipa REST/WS | [M-13](../docs/modules/M-13-api-gateway.md) |
| 骨 | ada-m16-cluster-coordinator | リーダー選出 + shard | [M-16](../docs/modules/M-16-cluster-coordinator.md) |
| 血 | ada-m01-acquisition | 5 種データソース | [M-01](../docs/modules/M-01-acquisition-adapter.md) |
| 血 | ada-m02-normalizer | NJSON 標準化 | [M-02](../docs/modules/M-02-normalizer.md) |
| 血 | ada-m03-data-flow-engine | キャンバス実行 | [M-03](../docs/modules/M-03-data-flow-engine.md) |
| 血 | ada-m09-exporter | 外部出力 | [M-09](../docs/modules/M-09-exporter.md) |
| 神経 | ada-m04-orchestration | パイプライン制御 | [M-04](../docs/modules/M-04-orchestration-engine.md) |
| 神経 | ada-m05-control-flow | 条件分岐・ループ | [M-05](../docs/modules/M-05-control-flow-executor.md) |
| 神経 | ada-m08-trigger | スケジュール/Webhook | [M-08](../docs/modules/M-08-trigger-service.md) |
| 神経 | ada-m15-central-event-bus | Pub/Sub | [M-15](../docs/modules/M-15-central-event-bus.md) |
| 筋肉 | ada-m06-plugin-sdk | プラグイン SDK (WASM) | [M-06](../docs/modules/M-06-node-runtime-plugin-sdk.md) |
| 筋肉 | ada-m07-debug | デバッグ | [M-07](../docs/modules/M-07-debug-service.md) |
| 筋肉 | ada-m12-canvas-editor | Bevy 0.14 WASM | [M-12](../docs/modules/M-12-canvas-editor-frontend.md) |
| 筋肉 | ada-m14-module-registry | atomic swap | [M-14](../docs/modules/M-14-module-registry.md) |

## 📚 ドキュメント

- 📖 **[ドキュメント索引](docs/README.md)** — 全ドキュメントの入口
- 🏛️ **[アーキテクチャ](docs/architecture/)** — 9 横切文書
- 🔌 **[API 仕様](docs/api/)** — 6 API 文書
- 🧪 **[テスト設計](docs/tests/)** — UT/IT/ST/UAT
- 🎯 **[意思決定](docs/decisions/)** — 11 P0 + 15 D-ADR
- 📋 **[IPA ワークフロー俯瞰](docs/architecture/08-workflow-overview.md)** — 150 工程

## 🎯 現在のステータス

| 項目 | 状態 |
|---|---|
| ドキュメント | ✅ 73 ファイル / 905 KB / 83 DOC-ID |
| 設計書 | ✅ 全モジュール（16）+ 横切（9）完成 |
| Cargo Workspace | ✅ 18 crate scaffold（v0.1.0） |
| ビルド | ⏳ 環境制約で本機未検証 |
| G4 着手判定 | ⏳ UN-P0-01〜11 解決待ち |
| 実装 | ⏳ G4 通過後開始 |

## 📈 ロードマップ

| フェーズ | 状態 | 期日 |
|---|---|---|
| G0 PJ 立上げ | ✅ | 2026-08-19 |
| G1 要件 Baseline | ✅ | 2026-08-19 |
| G2 BD Review | 🟡 計画済 | TBD |
| G3 DD Review | 🟡 計画済 | TBD |
| G4 実装着手 | ⚪ 待機 | UN-P0 解消後 |
| G5-G7 試験 | ⚪ | TBD |
| G8 受入 | ⚪ | TBD |
| G9-G10 移行・リリース | ⚪ | TBD |
| G11 完了 | ⚪ | TBD |

## 📜 ライセンス

本プロジェクトは [MIT License](LICENSE) の下で公開されています。

商用版（dual license）については別途交渉：[lidian727@gmail.com](mailto:lidian727@gmail.com)

## 🙏 関連規格

本プロジェクトは IPA（情報処理推進機構）の以下規格に準拠：

- [共通フレーム2018 (SLCP-JCF2018)](docs/architecture/08-workflow-overview.md) — 150 工程プロセス
- [非機能要求グレード2018](docs/requirements/05-nfr-non-functional-requirements.md) — NF 6 区分
- JIS X 0160:2012 — ソフトウェアライフサイクルプロセス

---

**最終更新**: 2026-08-20 | **バージョン**: v0.1.0 scaffold | **作者**: Ada Project Team
