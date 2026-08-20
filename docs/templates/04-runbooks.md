# Runbook / 構築手順テンプレート集（Runbooks & Build Procedures）

> **本ファイルの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) §5.5（実装 53）、§5.10（移行 97-101）、§5.11（リリース 105-108）に対応する **11 種類の Runbook / 構築手順 / 移行記録テンプレート** を提供する。  
> 開発着手・本番デプロイ・データ移行のたびに本テンプレートの派生版を作成する。

> **ドキュメントID**：DOC-TPL-RBK
> **文書分類**：テンプレート集
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/templates/README.md`](README.md)（DOC-TPL-INDEX）、[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)（DOC-ARCH-009）
> **下位文書**：派生版（`docs/runbooks/<env>/<テンプレ DOC-ID>-RBK-<env>.md`）
> **関連文書**：
> - [`docs/architecture/02-deployment.md`](../architecture/02-deployment.md)
> - [`docs/architecture/04-atomic-deployment.md`](../architecture/04-atomic-deployment.md)
> - [`docs/architecture/06-rust-tech-selection.md`](../architecture/06-rust-tech-selection.md)
> - [`docs/modules/M-14-module-registry.md`](../modules/M-14-module-registry.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - IPA「非機能要求グレード2018」【[NF-MIG] 必須】
> - JIS X 0160:2012
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（開発環境/移行/Smoke/Deploy/Hypercare の 11 Runbook テンプレート） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 開発環境構築手順書（IPA 工程 53）
2. 結合試験環境構築手順（IPA 工程 68）
3. 移行手順書（IPA 工程 97）
4. 移行リハーサル記録（IPA 工程 98）
5. データ移行ログ（IPA 工程 99）
6. システム移行ログ（IPA 工程 100）
7. 移行結果確認書（IPA 工程 101）
8. 本番デプロイ記録（IPA 工程 105）
9. Smoke Test 実施ログ（IPA 工程 106）
10. Go-Live 宣言書（IPA 工程 107）
11. Hypercare 計画書（IPA 工程 108）
12. 用語集
13. 参考文献

---

## A.1 開発環境構築手順書（IPA 工程 53）

### A.1.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 53（開発環境構築） |
| 目的 | Dev / SRE / QA がローカル + CI で 16 crate をビルド・テスト可能な環境を最短で構築 |
| 記入者 | Dev + SRE |
| 記入タイミング | 環境構築前（手順作成）、構築時（実行ログ追記） |
| 関連ドキュメント | [DOC-ARCH-007 §18 Cargo Workspace](../architecture/06-rust-tech-selection.md) |
| NF タグ | [NF-ENV]【必須】（3 OS 対応） |

### A.1.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-dev
対象環境: ☐ macOS  ☐ Linux  ☐ Windows  ☐ Docker  ☐ CI
Rust バージョン: 1.74+
Cargo バージョン: 1.74+
作成日: ____-__-__
作成者: <Dev + SRE>
検証者: ____-__-__
```

### A.1.3 前提条件

| 項目 | 必要バージョン | 確認コマンド | 結果 |
|---|---|---|---|
| OS | macOS 14+ / Ubuntu 22.04+ / Windows 11 | `uname -a` / `ver` | |
| Rust toolchain | 1.74+ (stable) | `rustc --version` | |
| Cargo | 1.74+ | `cargo --version` | |
| rustup target (Linux musl) | x86_64-unknown-linux-musl | `rustup target list --installed` | |
| Git | 2.40+ | `git --version` | |
| Docker (任意) | 24+ | `docker --version` | |
| PostgreSQL クライアント | 16+ | `psql --version` | |
| sqlx-cli | 0.7+ | `cargo install sqlx-cli` | |
| Node (WASM 用) | 20+ | `node --version` | |
| wasm-pack | latest | `wasm-pack --version` | |

### A.1.4 構築手順

| ステップ | コマンド | 期待結果 | 実測 |
|---|---|---|---|
| 1. リポジトリ clone | `git clone <repo-url> && cd ada` | clone 成功 | |
| 2. サブモジュール初期化 | `git submodule update --init --recursive` | OK | |
| 3. Rust ツールチェーン | `rustup default 1.74 && rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl` | OK | |
| 4. pre-commit フック | `cargo install cargo-deny cargo-audit cargo-tarpaulin && pre-commit install` | OK | |
| 5. .env 設定 | `cp .env.example .env && vi .env` | 完了 | |
| 6. DB 起動 (Docker) | `docker compose up -d postgres` | postgres ready | |
| 7. マイグレーション | `sqlx migrate run` | 11 テーブル + RLS 適用 | |
| 8. ビルド | `cargo build --workspace --all-targets` | success | |
| 9. UT | `cargo test --workspace` | 214 ケース pass | |
| 10. Lint | `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check` | 0 warning | |
| 11. SAST | `cargo deny check && cargo audit` | 重大 = 0 | |
| 12. WASM ビルド | `wasm-pack build m12-canvas-editor --target web` | .pkg 出力 | |
| 13. dev サーバー起動 | `cargo run -p ada-gateway` | listen :8080 | |
| 14. 動作確認 | `curl http://localhost:8080/health` | 200 OK | |

### A.1.5 トラブルシューティング

| 症状 | 原因 | 対処 |
|---|---|---|
| `linker not found` | musl-tools 未導入 | `apt install musl-tools` |
| `failed to bind` | DB 未起動 | `docker compose up -d postgres` |
| `wasm-pack: command not found` | wasm-pack 未導入 | `cargo install wasm-pack` |
| ... | ... | ... |

### A.1.6 完了基準

- 全 14 ステップ Pass
- 3 OS それぞれで構築成功
- `cargo test --workspace` 100% pass

---

## A.2 結合試験環境構築手順（IPA 工程 68）

### A.2.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-it
対象環境: 結合試験（ITa/ITb/API/DB/外部）
ベース: [DOC-ARCH-002 §5](../architecture/02-deployment.md) 「Staging」
作成日: ____-__-__
作成者: <SRE + QA>
```

### A.2.2 環境構成

| コンポーネント | バージョン | 台数 | 設定 |
|---|---|---|---|
| Rust runtime | 1.74+ | 3 ノード | release build |
| PostgreSQL | 16.x | 1 primary + 1 replica | RLS 有効 |
| Redis (cache) | 7.x | 1 | |
| Nginx (reverse proxy) | 1.25+ | 1 | TLS 終端 |
| 監視 | Prometheus + Grafana | 1 | |

### A.2.3 構築手順

| ステップ | コマンド / 操作 | 結果 |
|---|---|---|
| 1. infra コード適用 | `terraform apply -var-file=it.tfvars` | |
| 2. DB マイグレーション | `sqlx migrate run` | |
| 3. シーダーデータロード | `psql -f seed/it-data.sql` | |
| 4. 16 crate デプロイ | `./scripts/deploy-it.sh` | |
| 5. ヘルスチェック | `curl https://it.example.com/health` | |
| 6. 動作確認 | E2E 1 ケース実施 | |

### A.2.4 完了基準

- 全コンポーネント起動
- ヘルスチェック 200
- 1 サンプル E2E 成功

---

## A.3 移行手順書（IPA 工程 97）

### A.3.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 97（移行手順作成） |
| 目的 | 本番環境への切替手順を時系列で明記 |
| 記入者 | SRE + アーキ |
| 記入タイミング | 移行判定 G9 通過前 |
| 関連ドキュメント | [DOC-ARCH-004 §2 原子化デプロイ](../architecture/04-atomic-deployment.md)、[§A.4 移行リハーサル](#a4-移行リハーサル記録ipa-工程-98) |
| NF タグ | [NF-MIG]【必須】 |

### A.3.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-migration
移行対象: v1.x.x → v1.x.y
移行方式: ☐ Big-bang  ☐ 段階移行  ☐ Blue-Green  ☐ Canary
計画ダウンタイム: __分
実施予定: ____-__-__ __:__
作成日: ____-__-__
作成者: <SRE>
レビュー: <アーキ + DBA>
承認: <PM + PO>
```

### A.3.3 前提条件

- [§A.4 移行リハーサル](#a4-移行リハーサル記録ipa-工程-98) 2 回成功
- ロールバック手順（[DOC-ARCH-004 §2.5](../architecture/04-atomic-deployment.md)）即時実行可能
- Hypercare 体制確立（[§A.11](#a11-hypercare-計画書ipa-工程-108)）
- 関係者全員に通知済

### A.3.4 移行手順（時系列）

| T | ステップ | コマンド / 操作 | 期待結果 | 確認者 | 結果 |
|---|---|---|---|---|---|
| T-24h | 通知 | Slack / Email 配信 | 関係者全員確認 | PM | |
| T-2h | 事前確認 | 16 crate ビルド、最新マイグレーション、Backup 取得 | すべて成功 | SRE | |
| T-30min | サービス停止予告 | メンテナンスページ表示 | 200 | SRE | |
| T-15min | 読み取り専用モード | `kubectl scale --replicas=0 <write-pod>` | write API 503 | SRE | |
| T-10min | DB マイグレーション | `sqlx migrate run` | 適用成功 | DBA | |
| T-5min | 最終 Backup | `pg_dump > pre-upgrade.dump` | 成功 | DBA | |
| T-0 | 切替実行 | `./scripts/atomic-swap.sh v1.x.y` | 成功 | SRE | |
| T+1min | ヘルスチェック | `curl /health` | 200 | SRE | |
| T+5min | 読み書き再開 | `kubectl scale --replicas=N <write-pod>` | write API 200 | SRE | |
| T+10min | Smoke Test | [§A.9](#a9-smoke-test-実施ログipa-工程-106) | 全 8 ケース Pass | QA | |
| T+30min | 監視強化 | Prometheus アラート一時閾値厳格化 | 設定反映 | SRE | |
| T+2h | 移行結果確認 | [§A.7](#a7-移行結果確認書ipa-工程-101) | データ整合 100% | SRE + DBA | |
| T+24h | インシデント監視 | SRE 24h 待機 | 重大障害 0 | SRE | |

### A.3.5 ロールバック手順

| ステップ | コマンド | 期待結果 | 確認者 |
|---|---|---|---|
| 1. サービス停止 | `kubectl scale --replicas=0` | 停止 | SRE |
| 2. 旧版復元 | `./scripts/atomic-swap.sh v1.x.x` | 成功 | SRE |
| 3. DB ロールバック | `sqlx migrate revert` | 適用前状態 | DBA |
| 4. 動作確認 | Smoke Test | 全 Pass | QA |
| 5. 通知 | 関係者 + ユーザー | 配信 | PM |

### A.3.6 完了基準

- 全 T ステップ Pass
- ロールバック検証済み

---

## A.4 移行リハーサル記録（IPA 工程 98）

### A.4.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-rehearsal-<連番>
リハーサル番号: 第 <N> 回
実施日: ____-__-__
実施者: <SRE + DBA + QA>
参照手順書: DOC-TPL-RBK-RBK-migration
環境: Staging（本番相当）
```

### A.4.2 リハーサル結果

| 評価項目 | 結果 | 備考 |
|---|---|---|
| 切替時間 | __分（目標 ≤ 5分） | |
| ロールバック時間 | __分 | |
| データ整合 | ☐ 100% | |
| Smoke Test | ☐ 全 Pass | |
| 監視アラート | ☐ 正常 | |
| 関係者コミュニケーション | ☐ 良好 | |

### A.4.3 改善点

| # | 改善点 | 担当 | 期限 |
|---|---|---|---|
| 1 | <内容> | <氏名> | YYYY-MM-DD |
| 2 | ... | ... | ... |

### A.4.4 完了基準

- 2 回連続成功（G9 通過要件）
- 全改善点が反映

---

## A.5 データ移行ログ（IPA 工程 99）

### A.5.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-data-migration
対象: <既存システム or 別テナント>
データ規模: <件数 / 容量>
実施日: ____-__-__
実施者: <DBA + SRE>
参照手順: [DOC-MOD-010 §4.7](../modules/M-10-tenant-middleware.md)
```

### A.5.2 移行データ

| テーブル | 移行前件数 | 移行後件数 | 整合 | 備考 |
|---|---|---|---|---|
| tenants | __ | __ | ☐ | |
| users | __ | __ | ☐ | |
| canvases | __ | __ | ☐ | |
| canvas_nodes | __ | __ | ☐ | |
| ... | | | | |

### A.5.3 整合性検証

| チェック | 結果 |
|---|---|
| 主キー整合 | ☐ |
| 外部キー整合 | ☐ |
| RLS ポリシー整合 | ☐ |
| データ形式整合 | ☐ |
| 件数一致 | ☐ |
| checksum 一致 | ☐ |

### A.5.4 完了基準

- 全テーブル整合 ☐
- 整合性検証 6/6 Pass

---

## A.6 システム移行ログ（IPA 工程 100）

### A.6.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-system-migration
切替日時: ____-__-__ __:__
旧システム: v1.x.x
新システム: v1.x.y
実施者: <SRE>
参照手順: [§A.3 移行手順書](#a3-移行手順書ipa-工程-97)
```

### A.6.2 切替実行ログ

| ステップ | 実行時刻 | 結果 | 証跡 |
|---|---|---|---|
| 1. サービス停止 | HH:MM | ☐ Pass / ☐ Fail | |
| 2. DB マイグレーション | HH:MM | ☐ Pass / ☐ Fail | |
| 3. アプリ切替 | HH:MM | ☐ Pass / ☐ Fail | |
| 4. サービス再開 | HH:MM | ☐ Pass / ☐ Fail | |
| 5. Smoke | HH:MM | ☐ Pass / ☐ Fail | |

---

## A.7 移行結果確認書（IPA 工程 101）

### A.7.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-migration-result
確認日時: ____-__-__
確認者: <SRE + DBA + PM>
参照ログ: [§A.5](#a5-データ移行ログipa-工程-99), [§A.6](#a6-システム移行ログipa-工程-100)
判定: ☐ 移行成功  ☐ ロールバック要
```

### A.7.2 確認チェックリスト

| 項目 | 結果 |
|---|---|
| データ整合性（[§A.5](#a5-データ移行ログipa-工程-99) 6 チェック） | ☐ Pass |
| Smoke Test 全合格（[§A.9](#a9-smoke-test-実施ログipa-工程-106)） | ☐ Pass |
| 全 API 正常応答 | ☐ Pass |
| 監査ログ連続性 | ☐ Pass |
| 監視・アラート正常 | ☐ Pass |
| ユーザー影響なし | ☐ Pass |

### A.7.3 完了基準

- 全 6 項目 Pass
- ロールバック要の場合、即時実行

---

## A.8 本番デプロイ記録（IPA 工程 105）

### A.8.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-prod-deploy
リリース: v1.x.y
実施日: ____-__-__
実施者: <SRE>
参照: [DOC-MOD-014 §2.4 atomic swap](../modules/M-14-module-registry.md)
参照: [DOC-ARCH-004 §2 原子化デプロイ](../architecture/04-atomic-deployment.md)
参照判定: [§A.7 of 01-reviews.md G10 リリース判定](01-reviews.md#a7-リリース-gono-go-判定書ipa-工程-103--g10) = GO
```

### A.8.2 デプロイログ

| ステップ | コマンド | 実行時刻 | 結果 | 証跡 |
|---|---|---|---|---|
| 1. ビルド | `cargo build --release --workspace` | HH:MM | ☐ | |
| 2. イメージ push | `docker push registry/v1.x.y` | HH:MM | ☐ | |
| 3. Backup 取得 | `pg_dump > pre-deploy.dump` | HH:MM | ☐ | |
| 4. atomic swap | `./scripts/atomic-swap.sh v1.x.y` | HH:MM | ☐ | 旧版保持 |
| 5. ヘルスチェック | `curl /health` | HH:MM | ☐ 200 | |
| 6. Smoke 起動 | [§A.9](#a9-smoke-test-実施ログipa-工程-106) | HH:MM | ☐ | |

### A.8.3 ロールバック可否

| 状況 | ロールバック判断 | 実行 |
|---|---|---|
| Smoke 全 Pass | ロールバック不要 | — |
| Smoke 1 件 Fail | 原因解析、修正版デプロイ or ロールバック | ☐ 実施 |
| 重大障害 | 即時ロールバック | ☐ 実施 |

---

## A.9 Smoke Test 実施ログ（IPA 工程 106）

### A.9.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 106（稼働確認 / Smoke Test） |
| 関連文書 | [DOC-TST-003 §8 SMK 8 ケース](../tests/ST-design.md) |
| NF タグ | なし |

### A.9.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-smoke
実施日時: ____-__-__
実施者: <SRE + QA>
対象環境: Production
参照: [DOC-TST-003 §8 SMK](../tests/ST-design.md)
判定: ☐ Pass  ☐ Fail
```

### A.9.3 Smoke ケース実行

| SMK ID | シナリオ | 期待 | 実測 | 結果 |
|---|---|---|---|---|
| SMK-01 | ヘルスチェック `/health` | 200 | __ | ☐ |
| SMK-02 | ログイン | 200 + JWT | __ | ☐ |
| SMK-03 | 画布ロード | 200 + canvas data | __ | ☐ |
| SMK-04 | ノード追加 | 201 | __ | ☐ |
| SMK-05 | WebSocket 接続 | upgrade success | __ | ☐ |
| SMK-06 | モジュールロード | 200 + module info | __ | ☐ |
| SMK-07 | イベント購読 | 1 件以上配信 | __ | ☐ |
| SMK-08 | ログアウト | 200 | __ | ☐ |

### A.9.4 完了基準

- 8/8 Pass
- 1 件でも Fail ならロールバック検討

---

## A.10 Go-Live 宣言書（IPA 工程 107）

### A.10.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-go-live
宣言日時: ____-__-__
宣言者: <PO + PM>
対象サービス: Ada 无限画布跨平台数据集成系统
バージョン: v1.x
参照: [§A.7 of 01-reviews.md G10](01-reviews.md#a7-リリース-gono-go-判定書ipa-工程-103--g10), [§A.8 本番デプロイ](#a8-本番デプロイ記録ipa-工程-105), [§A.9 Smoke](#a9-smoke-test-実施ログipa-工程-106)
```

### A.10.2 宣言内容

| 項目 | 内容 |
|---|---|
| サービス開始日時 | ____-__-__ __:__ |
| エンドユーザー告知 | ☐ 完了 |
| 監視体制 | ☐ 確立 |
| サポート体制 | ☐ 確立 |
| Hypercare | [§A.11](#a11-hypercare-計画書ipa-工程-108) 参照 |
| 旧システム停止 | ____-__-__ __:__ |

### A.10.3 関係者通知

- エンドユーザー: ☐ 通知済
- 社内: ☐ 通知済
- 経営層: ☐ 通知済

---

## A.11 Hypercare 計画書（IPA 工程 108）

### A.11.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 108（初期流動対応 / Hypercare） |
| 期間 | Go-Live 後 2 週間（最低限） |
| 目的 | リリース直後の高密度サポート、重大障害ゼロ達成 |
| 記入者 | PM + SRE + サポート |
| NF タグ | [NF-AVA]【必須】 |

### A.11.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-RBK-RBK-hypercare
対象リリース: v1.x
期間: YYYY-MM-DD 〜 YYYY-MM-DD（2 週間）
体制長: <PM>
参加者: <Dev × 3, SRE × 2, サポート × 1>
連絡体制: PagerDuty + Slack #hypercare チャンネル
```

### A.11.3 日次運用

| 時刻 | 活動 | 担当 |
|---|---|---|
| 08:00 | 日次 standup | PM |
| 09:00 / 13:00 / 17:00 | 監視確認 | SRE |
| 障害発生時 | 即時対応（on-call） | SRE + Dev |
| 18:00 | 日次レポート | SRE |
| 不定期 | ユーザー問い合わせ対応 | サポート |

### A.11.4 Hypercare 終了基準

- 2 週間無重大障害（Sev1/Sev2 = 0）
- SLA 99.9% 達成
- 主要 KPI 安定

---

## 12. 用語集

| 用語 | 説明 | 出典 |
|---|---|---|
| Runbook | 運用作業手順書 | ITIL |
| Big-bang | 一括切替方式 | IPA 共通フレーム |
| Blue-Green | 旧新環境並列で切替 | DR 標準 |
| Canary | 段階的リリース | SRE |
| atomic swap | 旧版を保持したまま新版に切替 | DOC-ARCH-004 |
| RTO | Recovery Time Objective | DR |
| RPO | Recovery Point Objective | DR |
| Hypercare | リリース直後の高密度サポート | 本書 |
| Smoke Test | 簡易動作確認 | 本書 |
| PagerDuty | オンコール通知ツール | 商用 |

---

## 13. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018 年 3 月
2. Google SRE Book 第 2 版、Google、2020 年
3. Ada プロジェクトチーム「[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md)」、2026-08-20
4. Ada プロジェクトチーム「[DOC-ARCH-002 デプロイ](../architecture/02-deployment.md)」、2026-08-19
5. Ada プロジェクトチーム「[DOC-ARCH-004 原子化デプロイ](../architecture/04-atomic-deployment.md)」、2026-08-19
6. Ada プロジェクトチーム「[DOC-ARCH-007 Rust crate 選択](../architecture/06-rust-tech-selection.md)」、2026-08-19
7. Ada プロジェクトチーム「[DOC-MOD-014 モジュール登録](../modules/M-14-module-registry.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
