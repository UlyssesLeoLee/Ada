# 設計詳細 ADR（Design-level ADRs）

> **本文件の目的**：[DOC-ARCH-009 §5.16 実装フロー](../architecture/08-workflow-overview.md) 開始前に**未確定**だった **D-01〜D-15 設計詳細** を ADR 形式で解決する。  
> テックリード + アーキ の合議で決定、PO 承認。

> **ドキュメントID**：DOC-DEC-002
> **文書分類**：意思決定文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：PO
> **上位文書**：[`docs/decisions/README.md`](README.md)
> **下位文書**：各 ADR に基づき関連モジュール文書 §X を更新
> **関連文書**：[`docs/architecture/06-rust-tech-selection.md`](../architecture/06-rust-tech-selection.md)（10 ADR 既存）、[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（D-01〜D-15 全 15 件 ADR） | Ada プロジェクトチーム | TBD | PO |

---

## 目次

1. ADR 一覧
2. D-01: CRDT 协作库选型
3. D-02: 插件沙箱执行机制
4. D-03: Plugin SDK 语言边界
5. D-04: Bevy 版本
6. D-05: WASM Bundle Size 目标
7. D-06: RLS 性能影响
8. D-07: Event Bus 配信保証
9. D-08: マルチリージョン同期
10. D-09: Cargo Workspace バージョン戦略
11. D-10: CI 矩阵并行
12. D-11: OpenAPI 生成源
13. D-12: PL/pgSQL 開発者
14. D-13: License 選定
15. D-14: テストデータ戦略
16. D-15: CI fail 修复 SLA
17. 用語集
18. 参考文献

---

## 1. ADR 一覧

| ADR | 主题 | 推奨案 | 决定 |
|---|---|---|---|
| D-01 | CRDT 协作库 | **Yrs (Rust binding of Yjs)** | ✅ |
| D-02 | 插件沙箱 | **WASM (wasmtime)** | ✅ |
| D-03 | Plugin SDK 语言 | **Rust のみ（v1）→ Python via PyO3 (v2)** | ✅ |
| D-04 | Bevy 版本 | **0.14（stable）** | ✅ |
| D-05 | WASM Bundle Size | **< 8 MB（gzip 後 < 3 MB）** | ✅ |
| D-06 | RLS 性能 | **計測後ベンチマーク公開** | ✅ |
| D-07 | Event Bus 配信 | **at-least-once + idempotent consumer** | ✅ |
| D-08 | マルチリージョン | **Phase 1: 単一リージョン** | ✅ |
| D-09 | Cargo Workspace バージョン | **単一 workspace version（mono-repo）** | ✅ |
| D-10 | CI 并行 | **cargo キャッシュ + マトリクス shard** | ✅ |
| D-11 | OpenAPI 生成 | **utoipa（axum router から自動）** | ✅ |
| D-12 | PL/pgSQL 開発者 | **DBA 兼任、外部レビュー 1 名** | ✅ |
| D-13 | License | **MIT（本体） + 各 crate dual license 検討** | ✅ |
| D-14 | テストデータ | **合成 data（fixtures/）+ 実 data mask** | ✅ |
| D-15 | CI fail SLA | **build break 4h, test fail 24h** | ✅ |

---

## 2. D-01: CRDT 协作库选型

### 2.1 背景

[M-11 §3.2](../modules/M-11-rbac-collab.md) で CRDT を採用と決めたが、ライブラリが未定。

### 2.2 选项

| 库 | 言語 | 成熟度 | 性能 | 適合性 |
|---|---|---|---|---|
| **Yrs** | Rust | 高（Yjs binding） | 高 | ⭐ **推奨** |
| Automerge | Rust | 中 | 中 | △（性能劣） |
| Loro | Rust | 中（新） | 高 | △（実績少） |
| 自研 | Rust | — | — | ❌ 非推奨 |

### 2.3 决定

**Yrs 採用**。理由：
- Yjs の Rust binding、ブラウザ側と互換性 100%
- アクティブ開発、コミュニティ大
- WASM ターゲット対応
- `yrs = "0.18"` を [DOC-ARCH-007 §8](../architecture/06-rust-tech-selection.md) に追加

**影响**：[DOC-MOD-011 §3.2](../modules/M-11-rbac-collab.md) を更新

---

## 3. D-02: 插件沙箱执行机制

### 3.1 背景

[M-06 §3](../modules/M-06-node-runtime-plugin-sdk.md) でプラグインを安全実行する仕組みが必要。

### 3.2 选项

| 机制 | 性能 | 安全性 | 适合性 |
|---|---|---|---|
| **WASM (wasmtime)** | 中 | 高（メモリ/CPU 隔離） | ⭐ **推奨** |
| 独立进程 | 低 | 高 | 复杂 |
| Lua スクリプト | 高 | 中 | 功能限 |
| in-process | 高 | 低 | ❌ 不安全 |

### 3.3 决定

**WASM (wasmtime)** 採用。理由：
- メモリ隔離、CPU 制限、Capability 制御
- 業界標準（WASI 準拠）
- Rust エコシステム成熟（`wasmtime = "20"`）
- [D-05 WASM Bundle](#5-d-05-wasm-bundle-size-目标) と整合

**架构**：
```
ada-plugin-loader:
  1. 受信: module.wasm (manifest + binary)
  2. 検証: module_manifest.json (D-04 JSON Schema)
  3. 隔离: wasmtime::Engine.new(config) with fuel/memory limits
  4. 実行: store.call_async(func, args)
  5. 审计: ada-telemetry に記録
```

**影响**：[DOC-MOD-006 §3](../modules/M-06-node-runtime-plugin-sdk.md) + [DOC-MOD-014 §2.4](../modules/M-14-module-registry.md) を更新

---

## 4. D-03: Plugin SDK 语言边界

### 4.1 背景

[UR-007 Python 資産流用](../requirements/01-ur-user-requirements.md) 要望あり。

### 4.2 选项

| 选项 | 适合性 |
|---|---|
| **A. Rust のみ（v1）** | ⭐ **推奨（v1 短期）** |
| B. Rust + Python（PyO3） | ⭐ 推奨（v2 中期） |
| C. Rust + Python + JS | 大規模 |

### 4.3 决定

**v1: Rust のみ**、**v2: PyO3 バインディング追加**。

| 版本 | サポート言語 | ロード |
|---|---|---|
| v1.x | Rust → WASM | 公式 SDK |
| v2.x | + Python (PyO3 → WASM) | 公式 SDK |
| v3.x | + Node.js (napi-rs) | 検討 |

**影响**：[DOC-MOD-006 §1](../modules/M-06-node-runtime-plugin-sdk.md) + [DOC-REQ-FR-001 F-07](../requirements/04-fr-functional-requirements.md) を更新

---

## 5. D-04: Bevy 版本

### 5.1 背景

[M-12 §3](../modules/M-12-canvas-editor-frontend.md) で Bevy 採用と決めたが、0.14 vs 0.15 保留。

### 5.2 选択

| 版本 | 状態 | 適合性 |
|---|---|---|
| **0.14** | stable (LTS) | ⭐ **推奨（v1.x）** |
| 0.15 | beta (新機能) | △（v2.x で採用） |

### 5.3 决定

**Bevy 0.14** 採用。理由：
- 安定版、API 確定
- bevy_egui 0.14 対応（[DOC-ARCH-002 §2 UI](../architecture/01-tech-stack.md) と整合）
- 本番運用には stable 必須

**バージョン固定**：`Cargo.toml` に `bevy = "=0.14.0"` で固定。

**影响**：[DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md) + [DOC-MOD-012 §3](../modules/M-12-canvas-editor-frontend.md) を更新

---

## 6. D-05: WASM Bundle Size 目标

### 6.1 背景

[NF-PER-01 起動 < 3s](../requirements/05-nfr-non-functional-requirements.md) のため WASM bundle size に制約。

### 6.2 目标

| メトリクス | 目標 |
|---|---|
| **WASM bundle (uncompressed)** | < 8 MB |
| **WASM bundle (gzip)** | < 3 MB |
| **初回ロード時間（3G, 1Mbps）** | < 3s |
| **Bevy ECS runtime** | < 5 MB |
| **App code (ada-canvas-editor)** | < 3 MB |

### 6.3 最適化戦略

- `wasm-opt -Oz` 適用
- tree shaking（`wasm-bindgen`）
- 動的ロード（必要時）
- code splitting

### 6.4 决定

**8 MB / gzip 3 MB 目標**。CI で `wasm-snip` による自動チェック。

**影响**：[DOC-REQ-NFR-001 §3 NFR-PER-01](../requirements/05-nfr-non-functional-requirements.md) + [DOC-MOD-012 §4](../modules/M-12-canvas-editor-frontend.md)

---

## 7. D-06: RLS 性能影响

### 7.1 背景

[NF-PER-10 テナント 10,000](../requirements/05-nfr-non-functional-requirements.md) で RLS overhead が未知。

### 7.2 計測計画

| 场景 | 目標 |
|---|---|
| 1 テナント 1 クエリ | p95 < 5ms（RLS なしと差 < 1ms） |
| 1 万テナント 1 クエリ | p95 < 20ms |
| 100 同時接続 × 10K テナント | p95 < 50ms |
| 監査ログ書き込み（RLS + パーティション） | < 10ms |

### 7.3 决定

**実装後にベンチマーク公開**（[DOC-TPL-QUA §A.2 品質評価](../templates/07-quality.md) に含める）。

- ベンチマークツール：`pgbench` + `sqlx` カスタム
- CI 統合：毎日自動実行、結果は Grafana で可視化
- 性能劣化時：[NF-PER] 目標未達として再設計

**影响**：[DOC-REQ-NFR-001 §3 NFR-PER](../requirements/05-nfr-non-functional-requirements.md) + [DOC-MOD-010 §2.2](../modules/M-10-tenant-middleware.md)

---

## 8. D-07: Event Bus 配信保証

### 8.1 背景

[M-15 中央イベントバス](../modules/M-15-central-event-bus.md) の配信保証レベル。

### 8.2 选项

| 配信保証 | 説明 | 性能影響 | 適合性 |
|---|---|---|---|
| at-most-once | 重複なし、欠落あり | 高 | ロス許容 |
| **at-least-once** | 重複あり、欠落なし | 中 | ⭐ **推奨** |
| exactly-once | 重複なし、欠落なし | 低 | 複雑、KV 必要 |

### 8.3 决定

**at-least-once + idempotent consumer** 採用。

理由：
- at-least-once が業務イベントで十分
- consumer 側で idempotency key (event_id) を保持
- 重複処理は業務ロジックで吸収（NJSON に event_id 埋め込み）

**影响**：[DOC-MOD-015 §2](../modules/M-15-central-event-bus.md) + [DOC-ARCH-007 §10 observability](../architecture/06-rust-tech-selection.md)

---

## 9. D-08: マルチリージョン同期

### 9.1 背景

[NFR-AVA-09 マルチリージョン推奨](../requirements/05-nfr-non-functional-requirements.md) だが、初版で実装するか未定。

### 9.2 フェーズ戦略

| フェーズ | リージョン | Backup | 同期 |
|---|---|---|---|
| v1.0 | 単一 AZ (us-east-1) | 別 AZ | 不要 |
| v1.5 | 単一リージョン 2 AZ | 別リージョン | WAL ストリーミング |
| v2.0 | マルチリージョン (us-east-1 + eu-west-1) | 各リージョン | CDC + 競合解決 |

### 9.3 决定

**v1.0: 単一リージョン単一 AZ**。**v1.5: 2 AZ**。**v2.0: マルチリージョン**（将来検討）。

初版はシンプル、段階的に拡張。

**影响**：[DOC-ARCH-002 §1 インフラ](../architecture/01-tech-stack.md) + [DOC-ARCH-005 §3 監視](../architecture/05-admin-operations-ui.md)

---

## 10. D-09: Cargo Workspace バージョン戦略

### 10.1 背景

[QA-A05 16 crate 独立バージョン未決定](../architecture/07-qa-register.md)。

### 10.2 选项

| 选项 | 適合性 |
|---|---|
| **A. 単一 workspace version（mono-repo）** | ⭐ **推奨** |
| B. 独立 SemVer | 复杂，发布難 |

### 10.3 决定

**A 採用**。Cargo Workspace の `workspace.version` フィールドで全 crate 同一バージョン。

```
[workspace.package]
version = "0.1.0"
```

例外：外部公開予定の crate は個別 SemVer 可（v2 検討）。

**影响**：[DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md) ルート Cargo.toml

---

## 11. D-10: CI 并行

### 11.1 背景

[QA-A04 CI 矩阵 48 ビルド](../architecture/07-qa-register.md) 問題。

### 11.2 戦略

| 戦略 | 効果 |
|---|---|
| **cargo キャッシュ（sccache）** | 2x 高速化 |
| **shard matrix** | 16 crate を N 並列 |
| **skip on draft** | draft PR では一部 skip |
| **clippy + fmt + test 並列** | 3 job 同時に |

### 11.3 决定

**sccache + 4-shard matrix** 採用。CI 時間を 30 分 → 8 分に短縮。

**GitHub Actions**：
```yaml
strategy:
  matrix:
    shard: [1, 2, 3, 4]
steps:
  - uses: Swatinem/rust-cache@v2
  - run: cargo test --workspace -- --test-threads=1
```

**影响**：[DOC-ARCH-007 §17](../architecture/06-rust-tech-selection.md) + 新規 `.github/workflows/ci.yml`

---

## 12. D-11: OpenAPI 生成源

### 12.1 背景

[DOC-API-001 REST エンドポイント](../api/rest-endpoints.md) の生成方法。

### 12.2 选项

| 选项 | 適合性 |
|---|---|
| **A. utoipa（axum router から自動）** | ⭐ **推奨** |
| B. 手書き OpenAPI YAML | 同期難 |
| C. paperclip | △ |

### 12.3 决定

**A 採用**。`utoipa = "4"` + `utoipa-swagger-ui`。

```rust
#[utoipa::path(get, path = "/api/v1/canvases/{id}")]
async fn get_canvas(Path(id): Path<Uuid>) -> Json<Canvas> { ... }
```

**CI 自動生成**：`cargo run --bin gen-openapi > docs/api/openapi.json`

**影响**：[DOC-ARCH-007 §5 axum](../architecture/06-rust-tech-selection.md) + [DOC-API-001](../api/rest-endpoints.md)

---

## 13. D-12: PL/pgSQL 開発者

### 13.1 背景

[M-10 §4.6](../modules/M-10-tenant-middleware.md) の 6 PL/pgSQL 存過、誰が書くか。

### 13.2 选项

| 选项 | 適合性 |
|---|---|
| **A. DBA 兼任 + 外部レビュー 1 名** | ⭐ **推奨** |
| B. 専用 DBA 採用 | 過大 |
| C. Dev が書く | 性能落とし穴 |

### 13.3 决定

**A 採用**：
- 主担当：Ulysses（DBA 兼任）
- レビュー：外部 SQL 専門家 1 名（[DOC-MGT-REV-001 G3 DD Review](../management/02-review-schedule.md) で召集）
- Lint：`plpgsql_check` extension

**影响**：[DOC-MOD-010 §4.6](../modules/M-10-tenant-middleware.md) + [DOC-MGT-REV-001](../management/02-review-schedule.md)

---

## 14. D-13: License 選定

### 14.1 背景

[DOC-MOD-001〜016](../modules/) のライセンス未選定。

### 14.2 选项

| License | 商用利用 | 改変 | 配布 | 適合性 |
|---|---|---|---|---|
| **MIT** | ✅ | ✅ | ✅ | ⭐ **推奨（本体）** |
| Apache 2.0 | ✅ | ✅ | ✅ | 特許条項あり |
| GPLv3 | ⚠️ | ✅ | ❌ copyleft | 避ける |
| BSL (商用) | ❌ | ❌ | ❌ | 商用のみ |
| AGPL | ❌ | ✅ | ❌ | 避ける |

### 14.3 决定

**MIT（本体）** + 一部 crate で **dual license（MIT or Commercial）** 検討。

| 区分 | License | 理由 |
|---|---|---|
| ada-core, ada-telemetry | MIT | 基本ライブラリ |
| ada-canvas-editor (WASM UI) | MIT | ユーザー可視 |
| ada-*-enterprise-* | Commercial | 商用版（v2 検討） |
| 商用 crate | 各自の License 順守 | cargo-deny で監査 |

**CI 監査**：`cargo-deny` でライセンス違反検出。

**影响**：[DOC-ARCH-007 §15.3](../architecture/06-rust-tech-selection.md) + 新規 `LICENSE-MIT`

---

## 15. D-14: テストデータ戦略

### 15.1 背景

[M-10 テストデータ](../modules/M-10-tenant-middleware.md) 戦略未定。

### 15.2 选项

| 选项 | 適合性 |
|---|---|
| **A. 合成 data（fixtures/）** | ⭐ **推奨** |
| B. 実 data mask | 個人情報保護 |
| C. 匿名化 dump | 復元リスク |

### 15.3 决定

**A + B 併用**：

| 環境 | データ | 用途 |
|---|---|---|
| 単体試験 | 合成（fixtures/） | 各 crate の UT |
| 結合試験 | 合成（大容量、1 万行） | IT 性能試験 |
| ステージング | 合成 + mask 少量 | ST シナリオ |
| 本番 | 顧客実 data | 不可（mask のみ） |

**工具**：
- `sqlx fixtures/` で DDL + data 一括投入
- 自動生成：`faker` crate で 1 万行生成
- 個人情報 mask：email/phone/name を `***@***` 化

**影响**：[DOC-MOD-010 §5 テスト](../modules/M-10-tenant-middleware.md) + 新規 `tests/fixtures/`

---

## 16. D-15: CI fail 修复 SLA

### 16.1 背景

[QA-A09 CI fail 修复時間](../architecture/07-qa-register.md) 未定。

### 16.2 选项

| 选项 | 適合性 |
|---|---|
| **A. build break 4h, test fail 24h** | ⭐ **推奨** |
| B. 1h / 1h | 過小 |
| C. 1 週 | 過大 |

### 16.3 决定

**A 採用**：

| 失敗種別 | SLA | エスカレーション |
|---|---|---|
| **build break** | 4h | 超過 → on-call Dev |
| **test fail** | 24h | 超過 → 担当 Dev |
| **lint fail** | 24h | 超過 → 担当 Dev |
| **SAST 重大** | 24h | 超過 → SecO + PM |
| **coverage 低下** | 48h | 超過 → テック |

**影响**：[DOC-ARCH-007 §17 CI](../architecture/06-rust-tech-selection.md) + [DOC-MGT-COM-001 §4 エスカレーション](../management/04-communication-plan.md)

---

## 17. 用語集

| 用語 | 説明 |
|---|---|
| CRDT | Conflict-free Replicated Data Type |
| Yrs | Yjs の Rust binding |
| WASM | WebAssembly |
| RLS | Row-Level Security |
| at-least-once | 1 回以上配信（重複可能性あり） |
| sccache | Rust 用コンパイルキャッシュ |
| utoipa | Rust 向け OpenAPI 自動生成 crate |
| PL/pgSQL | PostgreSQL 手続き言語 |
| CI | Continuous Integration |
| SLA | Service Level Agreement |

## 18. 参考文献

1. PMBOK Guide 第 7 版、Project Management Institute、2021 年
2. PostgreSQL 18.6 Documentation
3. wasmtime Documentation
4. Yrs (Yjs for Rust) GitHub
5. utoipa Documentation
6. Bevy 0.14 Book
7. Ada プロジェクトチーム「[DOC-ARCH-007 Rust crate 選択](../architecture/06-rust-tech-selection.md)」、2026-08-19
8. Ada プロジェクトチーム「[DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
