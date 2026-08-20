# 实施前 QA 登録簿（Pre-Implementation Q&A Register）

> **本文件の目的**：本ドキュメント一式（[DOC-ARCH-001](../architecture/00-anatomy-model.md) ～ [DOC-ARCH-007](06-rust-tech-selection.md) 及び [DOC-MOD-001](../modules/M-01-acquisition-adapter.md) ～ [DOC-MOD-016](../modules/M-16-cluster-coordinator.md)）を実装に着手する前に、すべての**懸念・疑問・未決・仮定・リスク**を一覧化する。  
> 実装フェーズで再評価が必要な項目を一覧化することで、**手戻り・見落とし・認識齟齬**を防ぐ。  
> 開発開始の判断材料として定期的に更新する。

> **ドキュメントID**：DOC-ARCH-008
> **文書分類**：横断文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-19
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/00-anatomy-model.md`（DOC-ARCH-001）
> **下位文書**：全モジュール文書、全 API 文書
> **関連文書**：`docs/architecture/03-cross-cutting-risks.md`（DOC-ARCH-004，本書は「リスク」の上位概念）、`docs/CHANGELOG.md`（DOC-CHG-001）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章「システム開発プロセス」
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-19 | 初版制定（設計完了時点での懸念・未決事項を 1 冊に集約） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 概要と使い方
2. 質問一覧（実装前 Q&A マトリクス）
3. カテゴリ別深掘り
4. 既決事項の根拠再確認
5. 未決事項（実装着手前に対応必須）
6. 仮定一覧（コード記述時に正しいと見なすもの）
7. リスク登録（横断リスクの補完）
8. 実装着手判定チェックリスト
9. 验收要点
10. 用語集
11. 参考文献

---

## 1. 概要と使い方

### 1.1 位置付け

| 比較軸 | 本書（QA 登録簿） | [DOC-ARCH-004 横断リスク](03-cross-cutting-risks.md) | [DOC-CHG-001 CHANGELOG](CHANGELOG.md) |
|---|---|---|---|
| 焦点 | **未知・疑問・未決・仮定** | 既知の **リスク** | **過去の変更履歴** |
| 性質 | 設計の「穴」を可視化 | 想定外への対処方針 | 何がいつ変わったか |
| 実装前 | 必ずレビュー | 実装中も追跡 | 実装後記録 |
| 收束条件 | 「実装着手判定」で 0 件（P0/P1） | リスクは常在 | 累積 |

### 1.2 使い方

1. **設計レビュー時**：[§2 質問一覧](../architecture/07-qa-register.md) を上から順に消化
2. **実装着手前**：[§8 実装着手判定チェックリスト](../architecture/07-qa-register.md) を実行
3. **実装中**：[§5 未決事項](../architecture/07-qa-register.md) が解消されたかを逐一更新
4. **設計変更時**：[§6 仮定一覧](../architecture/07-qa-register.md) が崩れていないか確認

### 1.3 重大度凡例

| 重大度 | 意味 | 対応期限 |
|---|---|---|
| **P0** | 実装着手前に必ず解消 | コード記述開始前 |
| **P1** | 1st sprint 着手前までに解消 | M-01〜M-04 実装前 |
| **P2** | 該当モジュール着手前までに解消 | 個別モジュール実装前 |
| **P3** | 解決を後回し可（次イテレーション） | リリース前 |

---

## 2. 質問一覧（実装前 Q&A マトリクス）

### 2.1 アーキテクチャ / モジュール境界

| QA ID | カテゴリ | 質問 / 懸念 | 重大度 | 状態 | Owner |
|---|---|---|---|---|---|
| QA-A01 | モジュール境界 | M-06 (プラグイン) と M-14 (モジュール) の責務境界が将来曖昧になる懸念。M-06 が「単一プラグイン」、M-14 が「複数プラグイン束ねたモジュール」を扱うが、M-06 §3.4 で導入した `PluginRuntime::Module` との役割分担は M-14 の定義に収束すべき | P1 | 要整理 | アーキ |
| QA-A02 | モジュール境界 | M-03 (画布内データフロー) と M-15 (システムイベント) の境界が現状「画布内 vs 画布外」で線引きされているが、`M-04 编排引擎` が M-15 を購読して `module.*` イベントを受けた場合の動作責務が M-04 と M-15 のどちらにあるか不明確 | P1 | 要明確化 | アーキ |
| QA-A03 | アーキ | Cargo Workspace の 16 crate は [DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md) で確定したが、共通 crate (`ada-core` / `ada-telemetry`) の責務境界と公開 API 凍結ポリシーが未定義。新 crate 追加時のレビュー手順もない | P1 | 要定義 | アーキ + テックリード |
| QA-A04 | アーキ | Cargo Workspace 全体で [NF-MIG]【必須】（3 OS対応）のバイナリを 16 crate 同時にクロスコンパイルする CI 時間は現実的か。`x86_64-unknown-linux-musl` × 3 OS × 16 crate = 48 ビルド × nightly | P2 | 検証必要 | DevOps |
| QA-A05 | アーキ | 16 crate 全てを別 git tag でリリースする想定だが、現状 mono-repo 単一 `version` フィールド。複数 crate 独立バージョニング戦略がない | P2 | 戦略未策定 | テックリード |
| QA-A06 | アーキ | フロントエンド `m12-canvas-editor` は Bevy WASM だが、M-12 以外のモジュール（M-13 認証ヘッダ注入等）に依存する必要がある。WASM bundle にどう同梱するか | P1 | 要決定 | フロントエンド |

### 2.2 データ / Schema

| QA ID | カテゴリ | 質問 / 懸念 | 重大度 | 状態 | Owner |
|---|---|---|---|---|---|
| QA-D01 | Schema | `canvas.current_version_id` ↔ `canvas_version.canvas_id` の循環 FK を DB レベルで張らない設計 ([DOC-MOD-010 §4.2](../modules/M-10-tenant-middleware.md)) だが、ORM や migration ツールが循環参照を警告する。DBA レビューを未実施 | P0 | レビュー未実施 | DBA |
| QA-D02 | Schema | `event_seq_global` を全イベント単一 SEQUENCE にすると、1000 events/s × 24h = 86M/日 で contention 増。シーケンスプール拡大 (`CACHE 100`) で緩和しているが、それでも将来 10K events/s で検証必要 | P1 | 検証必要 | DB + 性能 |
| QA-D03 | Schema | `event_log.payload JSONB` のサイズ上限が問題。PostgreSQL JSONB は 1GB 上限だが実用 1MB で性能劣化。`raw_ref` に切り出す閾値が [DOC-MOD-007 §3.1](../modules/M-07-debug-service.md) で 1MB だが、**NJSON 単体のサイズポリシー**がない | P1 | ポリシー未策定 | アーキ + 性能 |
| QA-D04 | Schema | `module_registry` の `manifest JSONB` カラムに Module.toml 全体を格納するが、manifest の **schema validation** が DB 側にもアプリ側にもない。不正 manifest が混入するリスク | P0 | 検証機構未実装 | テックリード |
| QA-D05 | Schema | `consumer_offset` の `last_acked_event_seq BIGINT` だが、1000 events/s × 1年 = 31B を超える可能性。`BIGINT` は 9.2 × 10^18 まで対応、実用上は問題ないが overflow 監視ルールを明示すべき | P2 | 監視ルール未策定 | SRE |
| QA-D06 | Schema | `cluster_node.tenant_id UUID NULLABLE`（システム級ノード対応）の NULL レコードが RLS ポリシーでどう扱われるか。`tenant_id IS NULL OR tenant_id = current_setting(...)` の OR 条件は OK だが、`append_event` などの PL/pgSQL 関数で `tenant_id=NULL` を許容するか不明確 | P1 | 確認必要 | DBA |
| QA-D07 | Schema | `module_upgrade_history` が `error_message TEXT` で持つのに、構造化エラー（code + context）が記録できない。`JSONB` 型にすべきか | P2 | 改善余地 | テックリード |
| QA-D08 | Schema | `audit_log` が 1 年保存要件（[DOC-ARCH-005 §5 AD-09](../architecture/04-atomic-deployment.md)）だが、log rotation / partition 戦略なし。`PARTITION BY RANGE (action_at)` の月次パーティションが [DOC-MOD-010 §4.2](../modules/M-10-tenant-middleware.md) 実行ログ表にのみ言及、audit_log は未着手 | P0 | **未実装** | DBA |

### 2.3 性能 / 容量

| QA ID | カテゴリ | 質問 / 懸念 | 重大度 | 状態 | Owner |
|---|---|---|---|---|---|
| QA-P01 | 性能 | 1000 节点 30fps は [requirements §7.2](../legacy/requirements.md) の [NF-PER]【必須】だが、Bevy ECS の `sync_node_screen_position` を毎フレーム全 1000 ノードに実行するのは [DOC-MOD-012 §3.5](../modules/M-12-canvas-editor-frontend.md) 設計。`Changed<T>` フィルタは使うが、画面に表示中の 10 ノードでも毎フレーム全件 iterate になる懸念 | P1 | 検証未実施 | フロントエンド |
| QA-P02 | 性能 | 1000 节点 5000 边 全て可視領域内 だと視錐裁剪の効果がゼロ。逆に 100 万 边でも 10 のみ可視だと描画は軽い。**可視化テストは 2 极端**のみ、中间规模が未検証 | P2 | 検証範囲不足 | 性能 + フロントエンド |
| QA-P03 | 性能 | M-15 イベントトピック 1000 events/s P95 ≤ 100ms [AD-02](../architecture/04-atomic-deployment.md)。`pg_notify` は 非同期 listener あり、listener 落ちるとイベントロスト。**at-least-once 保証の真偽**が不明確 | P0 | 検証未実施 | 性能 + 信頼性 |
| QA-P04 | 性能 | M-16 100 ノード線形拡張 [AD-03](../architecture/04-atomic-deployment.md) の検証環境がない。100 ノード = 100 プロセス × 各 1 ブラウザ = 200-300GB メモリ。テストベッド調達が課題 | P1 | 環境未調達 | DevOps + SRE |
| QA-P05 | 性能 | 100 並列ユーザ + 1000 ノード + 高頻度イベントで 30fps 維持できるか。フロントエンドの WASM bundle が大きいと初回ロード時間に影響 | P1 | E2E 検証未実施 | フロントエンド |
| QA-P06 | 容量 | event_log 30 日 + audit_log 365 日 + execution_log 90 日 の保管容量試算がない。1000 events/s × 30 日 × 平均 5KB = 12.4 TB/年（rough estimate） | P1 | 容量計画未実施 | SRE |
| QA-P07 | 容量 | オブジェクトストレージ（`raw_ref` 参照先）の容量計画がない。Playwright 1 セッションあたり 50-200 MB スクリーンショット | P1 | 容量計画未実施 | SRE |
| QA-P08 | 性能 | `pg_notify` 通知の dispatcher プロセスが落ちた場合、event_log に書き込みはあるが listener がいない状態。再起動時にどこから追いつくか、**consumer_offset に記録された最終 ACK 位置**で replay する設計が [DOC-MOD-015 §3.4](../modules/M-15-central-event-bus.md) にあるが、listener 連続稼働時にどこまで ack 済みか未定義 | P1 | 障害復旧手順未策定 | 信頼性 + SRE |

### 2.4 セキュリティ / コンプライアンス

| QA ID | カテゴリ | 質問 / 懸念 | 重大度 | 状態 | Owner |
|---|---|---|---|---|---|
| QA-S01 | セキュリティ | `credential.encrypted_payload` の AES-256 鍵は [DOC-MOD-010 §4.2](../modules/M-10-tenant-middleware.md) で「KMS 管理」とだけ記載。鍵ローテーション戦略なし。AWS KMS / HashiCorp Vault / 自前の選択肢未決 | P0 | 戦略未策定 | セキュリティ + SRE |
| QA-S02 | セキュリティ | JWT 秘密鍵のローテーションが未設計。現在 1 鍵で全期間。鍵漏洩時の影響範囲が全期間 | P0 | 戦略未策定 | セキュリティ |
| QA-S03 | セキュリティ | PL/pgSQL `SECURITY DEFINER` 関数のオーナー指定が未設計。誤って `postgres` ユーザーに所有させると権限昇格リスク | P1 | 確認必要 | DBA + セキュリティ |
| QA-S04 | セキュリティ | `audit_log` に対するテナント管理者 (Owner) のアクセス制御が「自テナントのみ」としか書かれていない。PlatformAdmin が他テナント audit_log を閲覧する権限範囲と監査 | P1 | ポリシー未策定 | セキュリティ + コンプラ |
| QA-S05 | セキュリティ | 管理者 UI の「操作リプレイ」（[DOC-ARCH-006 §7.2](../architecture/05-admin-operations-ui.md)）がデータ変更を再実行する機能。誤って 2 回実行すると 2 重書き込み。**冪等性保証**が UI 仕様で必要 | P1 | 仕様未策定 | セキュリティ + フロントエンド |
| QA-S06 | セキュリティ | WebSocket 接続 (`/api/v1/admin/events/stream`) に対する CSRF 対策、Origin 検証、token 失効時の接続終了仕様が未設計 | P1 | 仕様未策定 | セキュリティ |
| QA-S07 | コンプラ | [requirements §7.5 セキュリティ「合规声明」](../legacy/requirements.md) は画面表示のみで、ユーザーが「同意」操作の証跡 (audit_log に `consent_accepted` 記録) が未設計 | P1 | 仕様未策定 | 法務 + コンプラ |
| QA-S08 | コンプラ | GDPR / 個人情報保護法 の「忘れられる権利」 (right to erasure) 対応が [DOC-MOD-010 §3.3 hard_delete_tenant_data](../modules/M-10-tenant-middleware.md) のみ。**個別ユーザ単位**での削除リクエスト手順が未設計 | P0 | 手順未策定 | 法務 + コンプラ |
| QA-S09 | コンプラ | データ越境移転：中国 PIPL (个人信息保护法) と GDPR 競合時の方針。event_log payload に個人情報が混入しないか、ペイロード暗号化要否が未決定 | P1 | 方針未策定 | 法務 + コンプラ |
| QA-S10 | コンプラ | [requirements §10.1 制約事項](../legacy/requirements.md)「本システム不承诺突破任何平台的登录验证码」と法的免責事項 (F-02-06 提示文案) が UI 表示のみ。ユーザーが「それでも突破する」と選択した場合の挙動（例: CAPTCHA 検出時の自動停止）がない | P1 | 仕様未策定 | 法務 + アーキ |

### 2.5 運用 / 可観測性

| QA ID | カテゴリ | 質問 / 懸念 | 重大度 | 状態 | Owner |
|---|---|---|---|---|---|
| QA-O01 | 運用 | structured log の保管先が未決定。Loki? ELK? CloudWatch? Datadog? [DOC-ARCH-007 §16](../architecture/06-rust-tech-selection.md) で「Prometheus + Grafana + ELK」と書かれているが具体的選定なし | P0 | 戦略未策定 | SRE |
| QA-O02 | 運用 | Prometheus exporter を Runtime プロセスに同梱するか、別プロセスとして scrape するかが未設計。サイドカー vs 組み込み | P2 | 決定未実施 | SRE |
| QA-O03 | 運用 | backup / restore 戦略が未設計。`pg_dump` 間隔 / 保管期間 / リストア RTO/RPO が未定義 | P0 | 戦略未策定 | SRE |
| QA-O04 | 運用 | DR (Disaster Recovery) 戦略が未設計。マルチリージョン？RPO/RTO 目標？DR drill 頻度？ | P1 | 戦略未策定 | SRE |
| QA-O05 | 運用 | on-call rotation / 障害対応プレイブックが未作成。`/admin` UI が障害を表示するが、**人が見て判断**する手順がない | P1 | プロセス未策定 | SRE + マネージャ |
| QA-O06 | 運用 | log / metric / trace の相関 ID 設計はあるが ([M-15 §3.3 event schema](../modules/M-15-central-event-bus.md))、**APM ツール選定**なし (Jaeger? Tempo? Honeycomb?) | P2 | 選定未実施 | SRE |
| QA-O07 | 運用 | マルチテナント環境での observability 分離。A テナントの metric を B テナント運用者が参照できない仕組み ([M-10 RLS](../modules/M-10-tenant-middleware.md)) はあるが、PlatformAdmin は全テナント参照OK。意図通りか再確認 | P2 | 確認必要 | セキュリティ + SRE |
| QA-O08 | 運用 | 7×24h 安定稼働 [NF-AVA]【必須】 の検証テスト方法。1 週間 soak test 環境で何を見るか、どこで停止するかの判断基準 | P1 | 検証手順未策定 | SRE + 性能 |
| QA-O09 | 運用 | Capacity planning：ストレージ、ネットワーク、計算資源の予測式がない。SLO 達成に必要なリソース規模 | P1 | 計画未策定 | SRE |

### 2.6 フロントエンド / WASM

| QA ID | カテゴリ | 質問 / 懸念 | 重大度 | 状態 | Owner |
|---|---|---|---|---|---|
| QA-F01 | FE | Bevy + HTML Overlay の座標同期で 60fps を出せるか。`SharedArrayBuffer` 対応ブラウザは限定的、フォールバックで `postMessage` だと遅延が累積 | P1 | 検証未実施 | フロントエンド |
| QA-F02 | FE | 100 万 行の NJSON payload を HTML Overlay で tree 展開した時の描画性能。CodeMirror 6 の性能特性未検証 | P2 | 検証未実施 | フロントエンド |
| QA-F03 | FE | `m12-canvas-editor` WASM bundle サイズ。bevy + bevy_egui + wasm-bindgen + yrs で [DOC-MOD-012 §3.5](../modules/M-12-canvas-editor-frontend.md) は 3D/音频 feature 削減で対処としているが、目標サイズ（例: < 5MB gzipped）未設定 | P1 | 目標値未設定 | フロントエンド |
| QA-F04 | FE | ブラウザ互換性 [NF-MIG]【必須】 4 ブラウザ × 各 OS のマトリクステスト計画がない。Safari は SharedArrayBuffer に CORS 必須など、固有制約 | P1 | 計画未策定 | フロントエンド + QA |
| QA-F05 | FE | M-12 以外の crate (`m13-gateway` 等) の WASM 版は存在するか？ 現状「前端 = m12 のみ」の前提が崩れた場合の影響 | P2 | 確認必要 | アーキ |
| QA-F06 | FE | 中文 IME 互換性が [DOC-ARCH-002 §3.1](../architecture/02-deployment.md) と [DOC-ARCH-007 §3.1](../architecture/06-rust-tech-selection.md) で言及されているが、Safari/Firefox での実機検証レポートがない | P1 | 検証未実施 | フロントエンド + QA |

### 2.7 移行 / アップグレード

| QA ID | カテゴリ | 質問 / 懸念 | 重大度 | 状態 | Owner |
|---|---|---|---|---|---|
| QA-M01 | 移行 | M-01〜M-16 いずれの interface も **frozen contract** 宣言がない。初期実装で頻繁に変更すると下流（PL/pgSQL 関数 / 他 crate）に影響 | P1 | 凍結基準未策定 | テックリード |
| QA-M02 | 移行 | NJSON `schema_version` フィールド ([DOC-MOD-001 §3.1](../modules/M-01-acquisition-adapter.md)) のバージョニングポリシー。`1.1` → `1.2` への後方互換性ルール（破壊的変更は major 2.0？） | P1 | ポリシー未策定 | アーキ + テックリード |
| QA-M03 | 移行 | モジュールを 1 個廃止する場合の dependency 解消手順。`module.deprecated` フラグ後のステップ（猶予期間、強制 uninstall） | P2 | 手順未策定 | テックリード + プロダクト |
| QA-M04 | 移行 | Cargo.lock 戦略：commit するか regenerate か。monorepo なら commit 推奨だが CI で再現性確認 | P2 | 戦略未策定 | DevOps |
| QA-M05 | 移行 | DB マイグレーション戦略：expand-contract pattern 採用か blue-green 採用か。`ALTER TABLE` 中のダウンタイム許容値 | P1 | 戦略未策定 | DBA + DevOps |
| QA-M06 | 移行 | 新モジュール追加時、既存 16 crate 全部 rebuild vs 新モジュールのみ incremental。incremental 採用時の整合性検証 | P2 | 検証未実施 | DevOps |

### 2.8 テスト / 演習

| QA ID | カテゴリ | 質問 / 懸念 | 重大度 | 状態 | Owner |
|---|---|---|---|---|---|
| QA-T01 | テスト | [UT-design.md](../tests/UT-design.md) §14 で「行 ≥ 80%, 分岐 ≥ 70%」だが、これはビジネスモジュールの話。M-14/15/16 の PL/pgSQL 関数カバレッジ測定ツールなし | P1 | ツール未選定 | QA |
| QA-T02 | テスト | [ST-design.md §6 DR](../tests/ST-design.md) は「フェイルオーバ 30s」をチェックしているが、**データロス件数**を許容するかは未定義 | P1 | 許容値未策定 | プロダクト + SRE |
| QA-T03 | テスト | 100 ノードクラスタでの性能テスト環境がない。100 ノード調達 + 100 ブラウザ起動のコスト | P1 | 環境未調達 | DevOps + QA |
| QA-T04 | テスト | chaos engineering（ランダム kill -9、ネットワーク分断、ディスク満杯）の定期演習計画がない | P2 | 計画未策定 | SRE + QA |
| QA-T05 | テスト | [requirements §12 受入条件](../legacy/requirements.md) の 6 項目は ST 単体で合格するが、**実際の業務ユーザーが触って使い物になるか**の UX 検証が未計画 | P1 | UX 検証計画未策定 | プロダクト + ユーザ |
| QA-T06 | テスト | セキュリティ浸透テスト ([DOC-ARCH-004 R12](../architecture/03-cross-cutting-risks.md)) の定期実施計画がない。四半期に 1 回等のルール未策定 | P1 | 計画未策定 | セキュリティ |

### 2.9 過程 / チーム

| QA ID | カテゴリ | 質問 / 懸念 | 重大度 | 状態 | Owner |
|---|---|---|---|---|---|
| QA-G01 | チーム | 16 crate に対応する専門チーム人員計画がない。Rust 経験者が [DOC-ARCH-007 §3.1](../architecture/06-rust-tech-selection.md) で想定されるが、確保できているか | P0 | 計画未策定 | マネージャ |
| QA-G02 | 過程 | [DOC-ARCH-001 §5 開発体験最適化](../architecture/00-anatomy-model.md) で「80% ゼロコード」と謳っているが、**ユーザーテスト**(エンドユーザが本当にコードなしで完結するか)計画がない | P1 | 検証未実施 | プロダクト |
| QA-G03 | 過程 | 全部署に「TBD」状態 ([全 doc §0 起草/レビュー/承認](../architecture/00-anatomy-model.md)) のメタデータが残っている。実際のレビュー / 承認組織が未確定 | P0 | 組織未確定 | マネージャ |
| QA-G04 | 過程 | 実装前の「アーキテクチャ決定記録 (ADR)」レビュー会が未開催。§19 ADR はドラフト状態。最終 GO/NO-GO 判定のプロセス未定義 | P0 | プロセス未策定 | マネージャ + アーキ |
| QA-G05 | チーム | 日本語の用語集（[§22 集 etc.](../architecture/06-rust-tech-selection.md)）が各文書に大量に含まれる。中国語話者が多いチームでは「読みづらい」可能性。翻訳ポリシー未策定 | P2 | ポリシー未策定 | マネージャ |
| QA-G06 | 過程 | 本書（QA 登録簿）自体の更新フロー。実装中も更新されるか、誰が更新するか。CHANGELOG との関係 | P1 | プロセス未策定 | テックリード |

---

## 3. カテゴリ別深掘り（重要事項）

### 3.1 データ / Schema

#### QA-D01 canvas.current_version_id 循環 FK

**問題**：[DOC-MOD-010 §4.2 canvas 表](../modules/M-10-tenant-middleware.md) で `current_version_id` ↔ `canvas_version.canvas_id` の循環 FK を DB レベルで作らず、application 層で保証している。

**懸念**：
- ORM (Diesel / SeaORM) や migration ツール (sqlx-cli) が循環を警告し、誤って物理 FK を張るリスク
- `DEFERRABLE INITIALLY DEFERRED` 採用案を [DOC-MOD-010 §4.2](../modules/M-10-tenant-middleware.md) で言及しているが、postgres での動作未検証

**アクション**：
1. P0：DBA レビューを今週中に実施
2. 検証スクリプト：両テーブルを 1 トランザクション内で相互参照 insert → 成功確認
3. 採用案：`DEFERRABLE INITIALLY DEFERRED` で物理 FK 化、検証 OK ならスキーマ更新

#### QA-D04 Module Manifest validation

**問題**：`module_registry.manifest JSONB` カラムに格納される Module.toml 全体に対して、**JSON Schema 検証機構がない**。

**懸念**：
- 誤った manifest（`module_id` 欠落、`version` 不正）が登録され、後段の atomic_module_swap が NULL 参照でクラッシュ
- バリデーションは `register_module` PL/pgSQL 関数内で「必須フィールド存在チェック」レベル（[DOC-MOD-010 §4.6.1](../modules/M-10-tenant-middleware.md)）。semver 形式、依存関係整合性、API ルート形式などは未検証

**アクション**：
1. P0：`ModuleManifest` の JSON Schema 定義
2. P0：PL/pgSQL `register_module` 内で JSON Schema 検証（`jsonb_matches_schema` 拡張使用）
3. P0：アプリ側でも `jsonschema` crate で事前検証、二重防御

#### QA-D08 audit_log パーティション未実装

**問題**：[DOC-MOD-010 §4.2 execution_log](../modules/M-10-tenant-middleware.md) で月次パーティションが言及されているが、1 年保存要件の `audit_log` は **パーティション未実装**。

**影響**：1000 events/s × 1 年 = 31B 行に到達、index 肥大で SELECT 性能劣化。

**アクション**：
1. P0：DDL を expand-contract pattern で `audit_log` を月次 RANGE パーティション化
2. 過去パーティションの読み取り専用化 + 古い partition の drop

### 3.2 セキュリティ

#### QA-S08 忘れられる権利（GDPR）個別ユーザ対応

**問題**：[DOC-MOD-010 §3.3 hard_delete_tenant_data](../modules/M-10-tenant-middleware.md) は**テナント単位**の完全削除。GDPR / 個人情報保護法では**個別ユーザ単位**の削除要求が来た場合、別フローが必要。

**具体例**：
- A テナントの一般ユーザ alice が「自分のデータ削除」を要求
- 該当 alice の audit_log、credential アクセス記録、NJSON payload 内の `captured_by` フィールド、を匿名化（完全削除ではなく）または完全削除

**アクション**：
1. P0：法務レビューで「忘れられる権利」の解釈確認
2. P0：PL/pgSQL `anonymize_user_data(user_id)` 関数の設計：全テーブル × user_id カラムを NULL 化
3. P0：UI/API に「データ削除リクエスト」エンドポイント追加

#### QA-S01 / S02 鍵管理と JWT ローテーション

**問題**：
- AES-256 鍵 (credential.encrypted_payload 用) の保管 / ローテーション戦略なし ([DOC-MOD-010 §4.2](../modules/M-10-tenant-middleware.md) で「KMS」とだけ)
- JWT 署名鍵 (HS256 or RS256) のローテーション戦略なし

**アクション**：
1. P0：KMS 選定（AWS KMS / HashiCorp Vault / GCP KMS）
2. P0：JWT 鍵ローテーション戦略（kid クレームによる複数鍵並行運用 → 旧鍵の grace period 後削除）

### 3.3 性能

#### QA-P03 pg_notify 信頼性

**問題**：[DOC-MOD-015 §3.4](../modules/M-15-central-event-bus.md) で `pg_notify` を使用しているが、**pg_notify はクラッシュ時に通知がロスト**する。

**具体シナリオ**：
1. `append_event` トランザクション commit 成功 → event_log に書き込み
2. 同時に `pg_notify` 発火 → listener プロセスへ通知
3. ここで listener プロセスがクラッシュ → 通知ロス
4. listener 再起動 → consumer_offset を見て replay 開始、しかし **ロストした通知分のイベント**は replay されない

**アクション**：
1. P0：at-least-once 保証の真偽を検証
2. 代替策：listener プロセスが `event_log` を定期 polling（5s）して新規イベントを検出（pg_notify ではなく）
3. 推奨：**polling ベース + ポーリング間隔 ≤ 1s** で pg_notify を補助的に

### 3.4 過程

#### QA-G01 Rust 人員確保

**問題**：[DOC-ARCH-007 §3.1](../architecture/06-rust-tech-selection.md) で Rust 1.74+ を採用するが、**16 crate を Rust で書く人員が確保できているか不明**。

**アクション**：
1. P0：人員計画（Rust senior 2-3 名、Rust mid 3-4 名 程度？）
2. P0：採用計画 or 外部委託プラン

#### QA-G03 起草/レビュー/承認欄 TBD

**問題**：[全 doc §0 起草/レビュー/承認欄](../architecture/00-anatomy-model.md) がすべて `TBD` 状態。

**アクション**：
1. P0：実際の起草 / レビュー / 承認組織を確定
2. P0：レビューサイクル（例：1 週間以内にレビュー完了）を運用化

---

## 4. 既決事項の根拠再確認

実装着手前に**再確認すべき既決事項**：

| 既決 ID | 項目 | 根拠 | 再確認必要？ |
|---|---|---|---|
| DEC-01 | 主要言語 = Rust | [DOC-ARCH-002 §1.2](../architecture/01-tech-stack.md) で「前後端統一」 | **是**：Rust 経験値確認 |
| DEC-02 | フロントエンド = Bevy + HTML Overlay | [DOC-ARCH-002 §3.1](../architecture/02-deployment.md) | **是**：Safari/Firefox 検証 |
| DEC-03 | DB = PostgreSQL | [DOC-ARCH-002 §3.3](../architecture/02-deployment.md) RLS 要件 | **否**（要件上確定） |
| DEC-04 | Web framework = Actix-web | [DOC-ARCH-007 ADR-002](../architecture/06-rust-tech-selection.md) | **否**（生態系十分） |
| DEC-05 | DB driver = sqlx | [DOC-ARCH-007 ADR-003](../architecture/06-rust-tech-selection.md) | **否**（生態系十分） |
| DEC-06 | PL/pgSQL 存过で原子性保証 | [DOC-ARCH-005 §11](../architecture/04-atomic-deployment.md) | **是**：性能と保守性の再評価 |
| DEC-07 | Module = 1 個 の deployable 単位 | [DOC-ARCH-004 §4](../architecture/04-atomic-deployment.md) | **否** |
| DEC-08 | CRDT ライブラリ = yrs | [DOC-ARCH-007 ADR-010](../architecture/06-rust-tech-selection.md) | **否** |
| DEC-09 | 16 crate 分割 | [DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md) | **是**：人員との整合 |
| DEC-10 | Admin UI = 別ページ `/admin` | [DOC-ARCH-006 §1](../architecture/05-admin-operations-ui.md) | **否**（要件適合） |

---

## 5. 未決事項（実装着手前に対応必須）

### 5.1 P0：実装着手前に必ず解消（コード記述開始前）

| ID | 項目 | 期限 | Owner |
|---|---|---|---|
| UN-P0-01 | **人員計画**：Rust 16 crate 開発人員の確保 | 実装着手 -7 日 | マネージャ |
| UN-P0-02 | **起草/レビュー/承認組織確定**：全 doc §0 の TBD 解消 | 実装着手 -3 日 | マネージャ |
| UN-P0-03 | **QA-D01 canvas 循環 FK**：DBA レビュー + DEFERRABLE 検証 | 実装着手 -3 日 | DBA |
| UN-P0-04 | **QA-D04 Module Manifest 検証**：JSON Schema 定義 + PL/pgSQL 統合 | M-14 着手 -7 日 | テックリード |
| UN-P0-05 | **QA-D08 audit_log パーティション**：DDL 設計 + マイグレーション手順 | M-10 着手 -7 日 | DBA |
| UN-P0-06 | **QA-S01 鍵管理戦略**：KMS 選定 + credential 鍵ローテーション手順 | M-01 着手 -7 日 | セキュリティ + SRE |
| UN-P0-07 | **QA-S02 JWT 鍵ローテーション**：kid クレーム + 複数鍵並行運用 | M-13 着手 -7 日 | セキュリティ |
| UN-P0-08 | **QA-S08 忘れられる権利**：PL/pgSQL `anonymize_user_data` 関数 + UI 削除リクエスト | 規制適用前に | 法務 + コンプラ |
| UN-P0-09 | **QA-O01 ログ基盤選定**：Loki / ELK / CloudWatch / Datadog | 実装着手 -7 日 | SRE |
| UN-P0-10 | **QA-O03 backup/restore 戦略**：RTO/RPO + リストア手順書 | プレプロダクション環境構築前 | SRE |
| UN-P0-11 | **QA-G04 ADR レビュー会**：GO/NO-GO 判定 | 実装着手 -1 日 | マネージャ + アーキ |

### 5.2 P1：1st sprint 着手前までに解消（M-01〜M-04 実装前）

| ID | 項目 | 期限 | Owner |
|---|---|---|---|
| UN-P1-01 | QA-A01 / A02 モジュール境界整理 | M-01 着手 -3 日 | アーキ |
| UN-P1-02 | QA-D02 event_seq 性能検証 | M-15 着手 -7 日 | 性能 + DB |
| UN-P1-03 | QA-D03 NJSON サイズポリシー | M-01 着手 -3 日 | アーキ |
| UN-P1-04 | QA-P01 1000 ノード 30fps 検証 | M-12 着手 -3 日 | フロントエンド |
| UN-P1-05 | QA-P04 100 ノードクラスタ検証環境 | M-16 着手 -7 日 | DevOps + SRE |
| UN-P1-06 | QA-P06/P07 容量計画 | プレプロダクション環境構築前 | SRE |
| UN-P1-07 | QA-P08 listener 連続稼働時の ack 仕様 | M-15 着手 -3 日 | 信頼性 + SRE |
| UN-P1-08 | QA-S03 SECURITY DEFINER オーナー | M-10 着手 -3 日 | DBA + セキュリティ |
| UN-P1-09 | QA-F01 SharedArrayBuffer フォールバック検証 | M-12 着手 -3 日 | フロントエンド |
| UN-P1-10 | QA-F03 WASM bundle サイズ目標設定 | M-12 着手 -3 日 | フロントエンド |
| UN-P1-11 | QA-M01 / M02 凍結基準 + NJSON バージョンポリシー | M-01 着手 -3 日 | テックリード + アーキ |
| UN-P1-12 | QA-M05 DB マイグレーション戦略 | M-10 着手 -3 日 | DBA + DevOps |
| UN-P1-13 | QA-T05 UX 検証計画 | M-12 着手 -3 日 | プロダクト + ユーザ |
| UN-P1-14 | QA-T06 セキュリティ浸透テスト計画 | M-13 着手 -3 日 | セキュリティ |

### 5.3 P2：個別モジュール着手前

（A-04 / A-05 / D-05 / D-07 / F-02 / F-05 / F-06 / M-03 / M-04 / M-06 / O-02 / O-06 / O-07 / P-02 / T-04 / G-05 / G-06）

### 5.4 P3：次イテレーション

（O-04 / O-08 / O-09 / Q-A06 など）

---

## 6. 仮定一覧（実装時に正しいと見なす前提）

実装者は下記を**仮定として進める**が、いずれかが崩れたら本書 §5 に戻り対応。

### 6.1 技術仮定

| ID | 仮定 | 検証方法 |
|---|---|---|
| ASM-T01 | Rust 1.74 stable で全 crate ビルド可能 | CI 実行 |
| ASM-T02 | PostgreSQL 15+ の RLS / `set_config` / advisory lock / `pg_notify` が安定動作 | DB 単体テスト |
| ASM-T03 | Tokio 1.40 の multi-thread runtime で 16 crate 並列実行可能 | 性能テスト |
| ASM-T04 | Bevy WASM bundle が `wasm-opt -O3` 後 5MB gzipped 以下 | ビルド検証 |
| ASM-T05 | yrs CRDT が 100 並列エディタで 100ms 以内収束 | 性能テスト |
| ASM-T06 | wasmtime 23.0 で WASM プラグイン sandbox が完全隔離 | セキュリティテスト |

### 6.2 業務仮定

| ID | 仮定 | 検証方法 |
|---|---|---|
| ASM-B01 | 8 割の業務ユーザがゼロコードで完結 ([DOC-ARCH-001 §5](../architecture/00-anatomy-model.md)) | UX テスト |
| ASM-B02 | 1000 ノードはキャンバスの上限ではなく 1 つの目安。実際平均 < 100 ノード | 利用統計 |
| ASM-B03 | 多租户 SaaS モードのテナント平均サイズ = 10 ユーザ、100 画布 | 事業計画 |
| ASM-B04 | IM メッセージ双方向 (F-02-05) は初期リリースでは read のみで十分 | プロダクト判断 |
| ASM-B05 | LLM 意味判断 (F-05-04) は Premium プラン限定 | 事業計画 |

### 6.3 運用仮定

| ID | 仮定 | 検証方法 |
|---|---|---|
| ASM-O01 | 開発人員 6-8 名（Rust senior 2-3 + mid 3-4 + DevOps 1） | 人員計画 |
| ASM-O02 | プレプロダクション環境 = 10 ノードクラスタ | 環境構築 |
| ASM-O03 | 本番初期 = 30 ノードクラスタ、100 テナント | 容量計画 |
| ASM-O04 | 監視ツール = Prometheus + Grafana + Loki（[DOC-ARCH-007 §16](../architecture/06-rust-tech-selection.md) 推奨） | SRE 判断 |
| ASM-O05 | バックアップ = pg_dump 日次 + WAL ストリーミング | SRE 判断 |

### 6.4 業務 SLA 仮定

| ID | 仮定 | 検証方法 |
|---|---|---|
| ASM-S01 | [requirements §7.2 1000 ノード 30fps](../legacy/requirements.md) は P95 ではなく平均 | 性能テスト |
| ASM-S02 | [NF-AVA 7×24h 連続稼働](../legacy/requirements.md) は再起動 1 回/月以内 | 監視 |
| ASM-S03 | [NF-MIG 跨 OS 対応](../legacy/requirements.md) は macOS 12+ / Win 10+ / Ubuntu 22.04+ | QA 計画 |

---

## 7. リスク登録（横断リスクの補完）

[DOC-ARCH-004 横断リスクと対応](../architecture/03-cross-cutting-risks.md) は**モジュール横断の既知リスク**。本書は **実装着手前に想定する未知リスク** を補完。

| リスク | 影響 | 検知方法 | 緩和策 |
|---|---|---|---|
| **設計 vs 実装の人員ミスマッチ**：Rust 経験不足で 16 crate 開発が長期化 | P0 | sprint velocity | Rust ペアプロ、外部メンター |
| **イベントログ容量爆発**：1000 events/s × 1 年 = 31B 行 | P1 | 監視メトリック | 月次パーティション、retention 30 日 |
| **pg_notify 通知ロス**：listener クラッシュ時のイベント欠落 | P1 | missing event 検知 | polling ベース併用 |
| **JWT 鍵漏洩**：全テナント全期間影響 | P0 | 鍵使用ログ監視 | kid ベース鍵ローテーション |
| **Module Manifest 不正**：JSON Schema 検証なしで不正 manifest 投入 | P0 | 起動時バリデーション | JSON Schema + PL/pgSQL 二重検証 |
| **16 crate 間循環依存**：crate 分割を誤ると cargo ビルド時に検出 | P1 | cargo tree 監視 | ビルド CI 失敗時即時 |
| **WASM bundle ブラウザ互換性**：Safari 15 以前で SharedArrayBuffer CORS 制約 | P1 | ブラウザマトリクステスト | postMessage フォールバック |
| **Adopt Rust 採用後の保守人員流動**：Rust 経験者の市場流動性が高い | P1 | 採用維持 | ドキュメント充実 + 採用強化 |
| **PL/pgSQL 採用の将来性**：PostgreSQL バージョンアップ時の互換性 | P2 | メジャー Upgrade サイクル | テストカバレッジ + マイグレーション手順 |
| **マルチリージョン展開の要求発生**：現状 1 リージョン前提、要件追加時に大改造 | P2 | 顧客要求 | イベントログの region tag 設計余地 |

---

## 8. 実装着手判定チェックリスト

実装に着手する前に**以下を全て満たす**こと。一つでも `NO` があれば着手延期。

### 8.1 組織 / 過程

- [ ] Rust 16 crate 開発人員が確保されている（ASM-O01 充足）
- [ ] 起草/レビュー/承認組織が確定し、各 doc の TBD が解消
- [ ] ADR レビュー会で GO 判定が出ている（QA-G04）
- [ ] 実装スプリント計画がレビュー済み

### 8.2 アーキテクチャ

- [ ] QA-A01 / A02 モジュール境界が M-01〜M-16 全てで明確
- [ ] QA-A04 16 crate クロスコンパイル時間 ≤ 30 分
- [ ] QA-A06 FE ↔ Backend 依存関係が cargo workspace で解決可能
- [ ] 16 ADR（[DOC-ARCH-007 §19](../architecture/06-rust-tech-selection.md)）がレビュー済み

### 8.3 データ / Schema

- [ ] QA-D01 canvas 循環 FK が DBA レビュー済み
- [ ] QA-D04 Module Manifest JSON Schema 定義完了
- [ ] QA-D08 audit_log パーティション DDL 設計完了
- [ ] 11 张 DDL 全部が DBA レビュー済み
- [ ] 6 PL/pgSQL 存过がレビュー済み

### 8.4 セキュリティ

- [ ] QA-S01 KMS 選定、credential 鍵ローテーション手順
- [ ] QA-S02 JWT 鍵ローテーション戦略（kid クレーム）
- [ ] QA-S03 PL/pgSQL SECURITY DEFINER オーナー指定
- [ ] QA-S08 忘れられる権利対応フロー

### 8.5 性能 / 容量

- [ ] QA-P01 1000 ノード 30fps 検証環境
- [ ] QA-P03 pg_notify at-least-once 保証確認
- [ ] QA-P04 100 ノードクラスタ検証環境
- [ ] QA-P06/P07 容量計画（DB / S3）

### 8.6 運用

- [ ] QA-O01 ログ基盤選定
- [ ] QA-O03 backup / restore 戦略
- [ ] QA-O08 7×24h 安定稼働検証手順

### 8.7 テスト

- [ ] UT / IT / ST 設計書が全モジュール分完備
- [ ] QA-T05 UX 検証計画
- [ ] QA-T06 セキュリティ浸透テスト計画

### 8.8 過程

- [ ] §5 P0 未決事項 11 件全て解消

---

## 9. 验收要点

1. **本書の P0/P1 未決事項が全て解消**されてから実装着手すること。 [NF-OPS]【必須】
2. **§8 実装着手判定チェックリスト**を 100% 満たす。 [NF-OPS]【必須】
3. **§6 仮定一覧**が崩れた時点で §5 に戻る。 [NF-AVA]【必須】
4. **§7 リスク登録**の各リスクに対する緩和策を実装フェーズで実施。 [NF-PER]【必須】
5. **本書を四半期ごとにレビュー**し、解決済み項目を CHANGELOG に記録。 [NF-OPS]【必須】

---

## 10. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| QA 表 | Question & Assumption Register、未知・疑問・未決・仮定の一覧 | §1.1 |
| ADR | Architecture Decision Record、§3 §4 | [DOC-ARCH-007 §19](../architecture/06-rust-tech-selection.md) |
| 仮定 (Assumption) | 実装時に正しいと見なす前提 | §6 |
| リスク (Risk) | 既知の想定外への対処方針 | §7、[DOC-ARCH-004](../architecture/03-cross-cutting-risks.md) |
| TBD | To Be Determined、未確定 | 全 doc §0 |
| GO/NO-GO | 実装着手判定 | §8 |
| expand-contract pattern | DB マイグレーション手法（追加 → 旧削除の 2 段） | §5.2 UN-P1-12 |
| 忘れられる権利 | GDPR / PIPL の個別ユーザ削除要求 | §3.2 QA-S08 |
| KMS | Key Management Service | §3.2 QA-S01 |
| kid | JWT Key ID、ローテーション識別子 | §3.2 QA-S02 |
| pg_notify | PostgreSQL 非同期通知機構 | §3.3 QA-P03 |
| chaos engineering | 意図的障害注入による回復力テスト | §2.8 QA-T04 |
| 浸透テスト | ペネトレーションテスト、攻撃者視点のセキュリティ検証 | §2.8 QA-T06 |
| DEFERRABLE INITIALLY DEFERRED | PostgreSQL の遅延 FK 制約 | §3.1 QA-D01 |
| SOC2 / FedRamp | クラウドセキュリティ認証 | §2.4 (将来) |
| GDPR | EU 一般データ保護規則 | §2.4 / §3.2 |
| PIPL | 中国個人情報保護法 | §2.4 QA-S09 |
| NF-OPS | IPA 非機能要求グレード：運用・保守性 | 全 doc |
| NF-PER | IPA 非機能要求グレード：性能・拡張性 | 全 doc |
| NF-AVA | IPA 非機能要求グレード：可用性 | 全 doc |
| NF-SEC | IPA 非機能要求グレード：セキュリティ | 全 doc |
| 3 OS | Windows / macOS / Linux | §6.4 ASM-S03 |

---

## 11. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. IPA「ソフトウェア開発データ白書」、独立行政法人情報処理推進機構、各年度版
4. JIS X 0160:2012「ソフトウェアライフサイクルプロセス」、日本工業標準調査会、2012年
5. **Risk Register ベストプラクティス** — Project Management Institute「Practice Standard for Project Risk Management」
6. **Pre-Mortem 手法** — Gary Klein「Performing a Project Premortem」、Harvard Business Review
7. GDPR — EU 一般データ保護規則 (Regulation (EU) 2016/679)
8. PIPL — 中華人民共和国個人情報保護法 (2021年8月施行)
9. Ada プロジェクトチーム各設計書 — [DOC-ARCH-001〜007](../architecture/) / [DOC-MOD-001〜016](../modules/) / [DOC-API-001〜006](../api/) / [DOC-REQ-001](../legacy/requirements.md) / [DOC-BSC-001](../legacy/basic-design.md) / [DOC-DTL-001](../legacy/detailed-design.md)
10. Ada プロジェクトチーム「全体自审報告」（本ドキュメント [CHANGELOG.md §2026-08-19 全体自审](CHANGELOG.md) 参照）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
