# レビュー記録テンプレート集（Review Records Templates）

> **本ファイルの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) §5.2, §5.3, §5.4, §5.6, §5.8, §5.9, §5.11, §6.16 の ⚪ レビュー工程（**20, 41, 52, 61, 89, 94, 103, 145**）に対応する **8 種類のレビューチェックリスト** を提供する。  
> 各レビューは IPA ゲート（G1〜G11）と対応し、**通過基準の根拠** として機能する。

> **ドキュメントID**：DOC-TPL-REV
> **文書分類**：テンプレート集
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/templates/README.md`](README.md)（DOC-TPL-INDEX）、[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)（DOC-ARCH-009）
> **下位文書**：派生版（`docs/records/reviews/<テンプレ DOC-ID>-REV-<YYYYMMDD>-<連番>.md`）
> **関連文書**：
> - 全モジュール文書（DOC-MOD-001〜016）
> - 全 API 文書（DOC-API-001〜006）
> - 全テスト文書（DOC-TST-001〜003）
> - [`docs/architecture/07-qa-register.md`](../architecture/07-qa-register.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（RD/BD/DD/UT/ST 完了/UAT/Go-Live/PJ 完了 の 8 レビュー記録テンプレート） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 要件レビューチェックリスト（IPA 工程 20 / G1）
2. 基本設計レビューチェックリスト（IPA 工程 41 / G2）
3. 詳細設計レビューチェックリスト（IPA 工程 52 / G3）
4. 単体試験レビューチェックリスト（IPA 工程 61）
5. システム試験完了承認書（IPA 工程 89 / G7）
6. 受入判定書（IPA 工程 94 / G8）
7. リリース Go/No-Go 判定書（IPA 工程 103 / G10）
8. プロジェクト完了判定書（IPA 工程 145 / G11）
9. 用語集
10. 参考文献

---

## A.1 要件レビューチェックリスト（IPA 工程 20 / G1）

### A.1.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 20（要件レビュー） |
| 関連 IPA ゲート | G1（要件ベースライン化） |
| 目的 | [DOC-REQ-001](../legacy/requirements.md) v1.2.1 のレビューを通し、要件ベースライン化 GO/NO-GO を判定する |
| 記入者 | PO（起票）、アーキ + PM + SecO（レビュー） |
| 記入タイミング | 要件定義書 v1.x 作成後、ベー スライン化前 |
| 関連ドキュメント | [DOC-REQ-001](../legacy/requirements.md)、[DOC-ARCH-008 §5 未決事項](../architecture/07-qa-register.md) |
| NF タグ | [NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV]【必須/推奨】 |

### A.1.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-REV-REV-<YYYYMMDD>-01
起票日: ____-__-__
起票者: <PO 氏名>
対象要件書: [DOC-REQ-001] v1.x.x
レビュー日: ____-__-__
参加者: PO、アーキ、PM、SecO、SRE、Biz 代表、外部有識者
判定: ☐ GO   ☐ NO-GO   ☐ 条件付き GO（条件: ____）
```

### A.1.3 レビューチェックリスト

| カテゴリ | チェック項目 | 結果 | 備考 |
|---|---|---|---|
| 完全性 | 全 F-ID（F-01〜F-17）に対応する要件定義があるか | ☐ Pass / ☐ Fail | |
| 一貫性 | 要件間の矛盾・重複がないか | ☐ Pass / ☐ Fail | |
| テスト可能性 | 各要件に検証方法（受入条件）が定義されているか | ☐ Pass / ☐ Fail | |
| トレーサビリティ | F-ID → M-ID → API までの対応が明確か | ☐ Pass / ☐ Fail | |
| NF 網羅 | 6 区分 × 必須/推奨 のタグが付与されているか | ☐ Pass / ☐ Fail | |
| セキュリティ | セキュリティ要件が定義されているか | ☐ Pass / ☐ Fail | |
| 運用 | 運用要件（SLA, 監視, BK）が定義されているか | ☐ Pass / ☐ Fail | |
| 移行 | 移行要件（既存システムからの切替）が定義されているか | ☐ Pass / ☐ Fail | |
| 用語 | 用語集が統一されているか | ☐ Pass / ☐ Fail | |
| 未決事項 | [DOC-ARCH-008 §5](../architecture/07-qa-register.md) の P0 が全て解消されているか | ☐ Pass / ☐ Fail | |
| 法令 | GDPR / PIPL / 業界規制への考慮が記載されているか | ☐ Pass / ☐ Fail | |
| 合意 | Biz + PO + アーキ + PM + SecO の合意があるか | ☐ Pass / ☐ Fail | |

### A.1.4 完了基準

- 全 12 カテゴリ Pass
- NO-GO 判定項目 0 件
- 条件付き GO の場合、条件解除期限と再判定会議が設定されている

---

## A.2 基本設計レビューチェックリスト（IPA 工程 41 / G2）

### A.2.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 41（基本設計レビュー / BD Review） |
| 関連 IPA ゲート | G2 |
| 目的 | 全 16 モジュール §2 + 8 横切文書（[DOC-ARCH-001〜008](../architecture/00-anatomy-model.md)）の妥当性を検証し、詳細設計着手 GO を判定 |
| 記入者 | アーキ（起票）、PM + 外部有識者（レビュー） |
| 記入タイミング | BD 完了後、DD 着手前 |
| 関連ドキュメント | [DOC-ARCH-001〜008](../architecture/00-anatomy-model.md)、[DOC-MOD-001〜016](../modules/M-01-acquisition-adapter.md) §2、[DOC-API-001〜006](../api/rest-endpoints.md)、[DOC-ARCH-008](../architecture/07-qa-register.md) |
| NF タグ | [NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV]【必須】網羅率 ≥ 90% |

### A.2.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-REV-REV-<YYYYMMDD>-02
起票日: ____-__-__
起票者: <アーキ 氏名>
対象範囲: DOC-ARCH-001〜008 + DOC-MOD-001〜016 §2 + DOC-API-001〜006
レビュー日: ____-__-__
参加者: アーキ、PM、PO、テックリード、外部有識者、SecO
判定: ☐ GO   ☐ NO-GO   ☐ 条件付き GO（条件: ____）
```

### A.2.3 レビューチェックリスト

| カテゴリ | チェック項目 | 結果 | 備考 |
|---|---|---|---|
| アーキテクチャ | 仿生モデル（[DOC-ARCH-001](../architecture/00-anatomy-model.md)）の 4 層責務が守られているか | ☐ Pass / ☐ Fail | |
| モジュール境界 | 16 モジュールの責務分離が明確か | ☐ Pass / ☐ Fail | |
| モジュール間 I/F | 16 モジュール間の依存方向が適切か（循環依存なし） | ☐ Pass / ☐ Fail | |
| データ | データモデルが要件と整合するか | ☐ Pass / ☐ Fail | |
| API | REST/WS/Error の設計が [DOC-API-001〜006](../api/rest-endpoints.md) で網羅されているか | ☐ Pass / ☐ Fail | |
| セキュリティ | RLS, RBAC, KMS, 監査ログ設計が [NF-SEC] を満たすか | ☐ Pass / ☐ Fail | |
| 性能 | [NF-PER] 目標（起動 < 3s, 1k node 操作 < 100ms）が達成可能か | ☐ Pass / ☐ Fail | |
| 可用性 | クラスタ構成（[DOC-MOD-016](../modules/M-16-cluster-coordinator.md)）が [NF-AVA] 99.9% を満たすか | ☐ Pass / ☐ Fail | |
| 運用 | Runbook パターン（[DOC-ARCH-005](../architecture/05-admin-operations-ui.md)）が [NF-OPS] を満たすか | ☐ Pass / ☐ Fail | |
| 移行 | atomic 反映（[DOC-ARCH-004](../architecture/04-atomic-deployment.md)）が [NF-MIG] を満たすか | ☐ Pass / ☐ Fail | |
| 環境 | 3 OS / マルチテナント / デプロイモードが [DOC-ARCH-002](../architecture/02-deployment.md) で定義されているか | ☐ Pass / ☐ Fail | |
| NF タグ網羅率 | [NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV]【必須】網羅率 ≥ 90% | ☐ Pass / ☐ Fail | |
| リスク | [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) の全リスクに対応方針があるか | ☐ Pass / ☐ Fail | |
| 外部有識者 | 外部有識者 ≥ 1 名の参加と承認があるか | ☐ Pass / ☐ Fail | |

### A.2.4 完了基準

- 全 14 カテゴリ Pass
- NF タグ網羅率 ≥ 90%
- 外部有識者の GO 判定

---

## A.3 詳細設計レビューチェックリスト（IPA 工程 52 / G3）

### A.3.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 52（詳細設計レビュー / DD Review） |
| 関連 IPA ゲート | G3 |
| 目的 | 16 モジュール §3 + 全 API 詳細 + 11 DDL + 6 PL/pgSQL 存過 + Cargo Workspace 16 crate の妥当性を検証し、実装着手 GO を判定 |
| 記入者 | テックリード（起票）、アーキ + PM（レビュー） |
| 記入タイミング | DD 完了後、実装着手前 |
| 関連ドキュメント | [DOC-MOD-001〜016](../modules/M-01-acquisition-adapter.md) §3、[DOC-API-001〜006](../api/rest-endpoints.md)、[DOC-MOD-010 §4](../modules/M-10-tenant-middleware.md)、[DOC-ARCH-007](../architecture/06-rust-tech-selection.md) |
| NF タグ | [NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV]【必須】網羅率 ≥ 95% |

### A.3.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-REV-REV-<YYYYMMDD>-03
起票日: ____-__-__
起票者: <テックリード 氏名>
対象範囲: DOC-MOD-001〜016 §3 + DOC-API-001〜006 + 11 DDL + 6 PL/pgSQL + DOC-ARCH-007 §18
レビュー日: ____-__-__
参加者: テックリード、アーキ、PM、Dev 代表、SecO、DBA
判定: ☐ GO   ☐ NO-GO   ☐ 条件付き GO（条件: ____）
```

### A.3.3 レビューチェックリスト

| カテゴリ | チェック項目 | 結果 | 備考 |
|---|---|---|---|
| モジュール詳細 | 全 16 モジュールの §3 が 4 段構造（型/関数/IF/Err）で記述されているか | ☐ Pass / ☐ Fail | |
| API 詳細 | OpenAPI 3.1 / WebSocket イベント型 / エラーコード体系が完備されているか | ☐ Pass / ☐ Fail | |
| DB 詳細 | 11 テーブルの DDL, RLS ポリシー, インデックス, パーティション戦略が [DOC-MOD-010 §4](../modules/M-10-tenant-middleware.md) で定義されているか | ☐ Pass / ☐ Fail | |
| PL/pgSQL | 6 存過（register_module, atomic_module_swap, append_event, acquire_lease, release_lease, register_node_heartbeat）の権限/ロック/ロールバックが明記されているか | ☐ Pass / ☐ Fail | |
| Cargo Workspace | [DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md) の 16 crate 構造と公開 API 凍結ポリシーが定義されているか | ☐ Pass / ☐ Fail | |
| セキュリティ実装 | KMS 鍵取得、JWT 検証、CSRF 対策、SQL インジェクション対策が具体的か | ☐ Pass / ☐ Fail | |
| エラー処理 | [DOC-API-003](../api/error-codes.md) のエラー体系と Retry-After ヘッダが反映されているか | ☐ Pass / ☐ Fail | |
| ログ設計 | PII マスキング、相関 ID、ログレベルが [NF-OPS] を満たすか | ☐ Pass / ☐ Fail | |
| 性能実装 | DB クエリプラン、Hot Path 関数、WASM bundle サイズが [NF-PER] を満たすか | ☐ Pass / ☐ Fail | |
| 監視フック | メトリクス公開（Prometheus 形式）、OTel 計装が組み込まれているか | ☐ Pass / ☐ Fail | |
| テスト容易性 | DI / Mock / Test Harness が組み込まれているか | ☐ Pass / ☐ Fail | |
| NF タグ網羅率 | 網羅率 ≥ 95% | ☐ Pass / ☐ Fail | |
| 未決事項 | [DOC-ARCH-008 §5](../architecture/07-qa-register.md) の P0/P1 が解消されているか | ☐ Pass / ☐ Fail | |

### A.3.4 完了基準

- 全 13 カテゴリ Pass
- NF タグ網羅率 ≥ 95%
- P0 解消 100%、P1 解消 ≥ 80%

---

## A.4 単体試験レビューチェックリスト（IPA 工程 61）

### A.4.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 61（単体試験レビュー / UT Review） |
| 関連 IPA ゲート | G5（UT 完了）の前段 |
| 目的 | [DOC-TST-001 UT 設計](../tests/UT-design.md)（214 ケース）のレビューを通し、UT 実施の妥当性を検証 |
| 記入者 | QA（起票）、テックリード（レビュー） |
| 記入タイミング | UT 仕様書作成後、UT 実施前 |
| 関連ドキュメント | [DOC-TST-001](../tests/UT-design.md)、[DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §15 |
| NF タグ | なし |

### A.4.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-REV-REV-<YYYYMMDD>-04
起票日: ____-__-__
起票者: <QA 氏名>
対象 UT 仕様: [DOC-TST-001] v1.x.x
レビュー日: ____-__-__
参加者: QA、テックリード、Dev 代表
判定: ☐ GO   ☐ NO-GO   ☐ 条件付き GO
```

### A.4.3 レビューチェックリスト

| カテゴリ | チェック項目 | 結果 | 備考 |
|---|---|---|---|
| ケース網羅 | 全 crate に対し ≥ 8 ケース / crate | ☐ Pass / ☐ Fail | |
| 境界値 | 各関数に境界値（min, max, 0, 負数）テストがあるか | ☐ Pass / ☐ Fail | |
| 異常系 | 各 crate に異常系 ≥ 2 ケース | ☐ Pass / ☐ Fail | |
| モック | 外部依存（DB, HTTP, WASM）がモック化されているか | ☐ Pass / ☐ Fail | |
| 命名 | テスト名 `test_<crate>_<func>_<scenario>` を遵守 | ☐ Pass / ☐ Fail | |
| 独立性 | テスト間依存なし、並列実行可能 | ☐ Pass / ☐ Fail | |
| カバレッジ目標 | コードカバレッジ ≥ 80%（line）+ ≥ 70%（branch） | ☐ Pass / ☐ Fail | |
| CI 統合 | `cargo test` が CI で自動実行される | ☐ Pass / ☐ Fail | |
| flaky 対策 | timing 依存テストには `proptest` / `tokio::time::pause` 利用 | ☐ Pass / ☐ Fail | |
| ドキュメント | 各 crate に `#[doc]` コメント + Example | ☐ Pass / ☐ Fail | |

### A.4.4 完了基準

- 全 10 カテゴリ Pass
- 想定コードカバレッジ ≥ 80%

---

## A.5 システム試験完了承認書（IPA 工程 89 / G7）

### A.5.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 89（ST 完了承認） |
| 関連 IPA ゲート | G7 |
| 目的 | [DOC-TST-003 ST 設計](../tests/ST-design.md)（約 100 ケース：E2E + NFR + ACC + SMK + DDI + DR + AD）の全合格を確認し、UAT 着手を承認 |
| 記入者 | QA + PM（共同起票） |
| 記入タイミング | ST 全 100 ケース合格後 |
| 関連ドキュメント | [DOC-TST-003](../tests/ST-design.md)、[DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) |
| NF タグ | [NF-AVA\|PER\|SEC]【必須】 |

### A.5.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-REV-REV-<YYYYMMDD>-05
起票日: ____-__-__
起票者: <QA + PM>
対象 ST 結果: [DOC-TST-003] 実行ログ
判定日: ____-__-__
承認者: QA + PM + PO
判定: ☐ GO（UAT 着手可）  ☐ NO-GO
```

### A.5.3 合格確認チェックリスト

| 区分 | テスト種別 | ケース数 | 合格数 | 不合格 | 備考 |
|---|---|---|---|---|---|
| E2E | 機能試験 | 31 | __ | __ | |
| E2E | シナリオ試験 | — | __ | __ | |
| NFR | 性能試験 | 8 | __ | __ | |
| NFR | 負荷試験 | 4 | __ | __ | |
| NFR | ストレス試験 | 3 | __ | __ | |
| NFR | 可用性試験 | 4 | __ | __ | |
| NFR | 運用試験 | 5 | __ | __ | |
| SEC | セキュリティ試験 | 5 | __ | __ | |
| DR | 障害/復旧/BK 試験 | 6 | __ | __ | |
| ACC | 受入関連 | 8 | __ | __ | |
| SMK | Smoke | 8 | __ | __ | |
| DDI | データ駆動 | 6 | __ | __ | |
| AD | 監査 | — | __ | __ | |
| **合計** | | **~100** | __ | __ | |

### A.5.4 NF 目標達成確認

| NF 区分 | 目標 | 実測 | 達成 |
|---|---|---|---|
| [NF-AVA] | SLA 99.9% | __% | ☐ |
| [NF-PER] | 起動 < 3s, 1k node < 100ms | __s / __ms | ☐ |
| [NF-SEC] | 脆弱性 重大 = 0 | __件 | ☐ |
| [NF-OPS] | MTTR < 30min | __min | ☐ |
| [NF-MIG] | 切替時間 < 5min | __min | ☐ |
| [NF-ENV] | 3 OS 全動作 | __/3 | ☐ |

### A.5.5 完了基準

- 全テスト種別 100% 合格（不合格 0 件）
- 重大脆弱性 0 件
- NF 全区分目標達成
- PM + PO の合議 GO

---

## A.6 受入判定書（IPA 工程 94 / G8）

### A.6.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 94（受入判定） |
| 関連 IPA ゲート | G8 |
| 目的 | UAT 8 ケース + 業務シナリオ全合格を確認し、検収（IPA 工程 95）と移行着手を承認 |
| 記入者 | PO（起票） |
| 記入タイミング | UAT 完了後 |
| 関連ドキュメント | [DOC-ACC-001](../tests/ST-design.md) §7 |
| NF タグ | なし |

### A.6.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-REV-REV-<YYYYMMDD>-06
起票日: ____-__-__
起票者: <PO 氏名>
対象 UAT 結果: [DOC-ACC-001] 実行ログ
判定日: ____-__-__
承認者: PO + Biz 代表 + 契約担当
判定: ☐ 受入（検収可）  ☐ 不受入（差し戻し）
```

### A.6.3 判定チェックリスト

| カテゴリ | チェック項目 | 結果 |
|---|---|---|
| 機能要件 | 全 F-ID（F-01〜F-17）がユーザー操作で検証された | ☐ Pass / ☐ Fail |
| 業務要件 | [DOC-REQ-001](../legacy/requirements.md) §7 業務要件の業務シナリオが全て成功 | ☐ Pass / ☐ Fail |
| 非機能要件 | 性能・セキュリティ・可用性が業務要件を満たす | ☐ Pass / ☐ Fail |
| ユーザー教育 | 管理者・運用者・エンドユーザー向け教育完了 | ☐ Pass / ☐ Fail |
| ドキュメント | ユーザーマニュアル、運用マニュアル、FAQ が整備されている | ☐ Pass / ☐ Fail |
| サポート | サポート窓口・SLA 体制が確立 | ☐ Pass / ☐ Fail |
| 残存課題 | リリース後の残存課題が把握され、対応スケジュールが合意 | ☐ Pass / ☐ Fail |
| 契約条件 | 検収条件が満たされている | ☐ Pass / ☐ Fail |

### A.6.4 完了基準

- 全 8 カテゴリ Pass
- PO + Biz 代表 + 契約担当の合議 GO

---

## A.7 リリース Go/No-Go 判定書（IPA 工程 103 / G10）

### A.7.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 103（リリース判定） |
| 関連 IPA ゲート | G10 |
| 目的 | 本番デプロイ（IPA 工程 105）の実施可否を最終判定する |
| 記入者 | PM（起票） |
| 記入タイミング | 本番デプロイ前 24 時間以内 |
| 関連ドキュメント | [DOC-ARCH-002](../architecture/02-deployment.md)、[DOC-ARCH-004](../architecture/04-atomic-deployment.md) |
| NF タグ | [NF-MIG\|SEC\|ENV]【必須】 |

### A.7.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-REV-REV-<YYYYMMDD>-07
判定日時: ____-__-__ __:__
起票者: <PM 氏名>
対象リリース: v1.x.x → 本番
判定者: PM + PO + SRE
判定: ☐ GO（リリース実施可）  ☐ NO-GO（延期）
```

### A.7.3 Go/No-Go チェックリスト

| カテゴリ | チェック項目 | 結果 |
|---|---|---|
| ST 完了 | G7 通過証跡（[§A.5](#a5-システム試験完了承認書ipa-工程-89--g7)） | ☐ Pass / ☐ Fail |
| UAT 完了 | G8 通過証跡（[§A.6](#a6-受入判定書ipa-工程-94--g8)） | ☐ Pass / ☐ Fail |
| 移行判定 | G9 通過証跡（[`04-runbooks.md` §A.7](04-runbooks.md#a7-移行結果確認書ipa-工程-101)） | ☐ Pass / ☐ Fail |
| 本番環境 | 104 完了（[`04-runbooks.md` §A.8](04-runbooks.md#a8-本番デプロイ記録ipa-工程-105)） | ☐ Pass / ☐ Fail |
| Smoke Test 準備 | Smoke シナリオ（[`04-runbooks.md` §A.9](04-runbooks.md#a9-smoke-test-実施ログipa-工程-106)）即時実行可能 | ☐ Pass / ☐ Fail |
| ロールバック | atomic ロールバック手順（[DOC-ARCH-004 §2.5](../architecture/04-atomic-deployment.md)）即時実行可能 | ☐ Pass / ☐ Fail |
| Hypercare 体制 | 2 週間の Hypercare 体制（[`04-runbooks.md` §A.11](04-runbooks.md#a11-hypercare-計画書ipa-工程-108)）確立 | ☐ Pass / ☐ Fail |
| 監視・サポート | 本番監視開始、サポート窓口開設 | ☐ Pass / ☐ Fail |
| コミュニケーション | リリース通知・マニュアル・FAQ の発出準備完了 | ☐ Pass / ☐ Fail |
| ダウンタイム | 計画ダウンタイム内（≤ 5min）か | ☐ Pass / ☐ Fail |
| 規制・コンプラ | 監査ログ連続性、データ保護要件遵守 | ☐ Pass / ☐ Fail |

### A.7.4 完了基準

- 全 11 カテゴリ Pass
- PM + PO + SRE 三者の全会一致 GO
- 1 名でも NO-GO 判定があればリリース延期

---

## A.8 プロジェクト完了判定書（IPA 工程 145 / G11）

### A.8.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 145（PJ 完了判定） |
| 関連 IPA ゲート | G11 |
| 目的 | 全フェーズ完了 + 残存課題なし + 引き継ぎ完了 + KT 完了を確認し、PJ を正式に完了 |
| 記入者 | PM（起票） |
| 記入タイミング | 全ての主要開発/移行/リリース工程完了後 |
| 関連ドキュメント | [`08-closure.md`](08-closure.md)、[DOC-ARCH-009 §7 G11](../architecture/08-workflow-overview.md) |
| NF タグ | なし |

### A.8.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-REV-REV-<YYYYMMDD>-08
判定日: ____-__-__
起票者: <PM 氏名>
対象 PJ: Ada 无限画布跨平台数据集成系统 v1.x
承認者: PO + PM + 経営層
判定: ☐ PJ 完了  ☐ 継続
```

### A.8.3 完了判定チェックリスト

| カテゴリ | チェック項目 | 結果 |
|---|---|---|
| 全 Gate 通過 | G0〜G10 全て通過 | ☐ Pass / ☐ Fail |
| リリース後安定 | Hypercare 期間（≥ 2 週）重大障害 0 件 | ☐ Pass / ☐ Fail |
| SLA 達成 | 運用 SLA 99.9% 達成 | ☐ Pass / ☐ Fail |
| 残存課題 | 全残存課題が別 PJ / 保守対応として整理済み | ☐ Pass / ☐ Fail |
| 成果物引渡し | 全成果物の引渡し書（[`08-closure.md` §A.1](08-closure.md#a1-成果物引渡し書ipa-工程-146)）完了 | ☐ Pass / ☐ Fail |
| ナレッジ移管 | KT 資料（[`08-closure.md` §A.4](08-closure.md#a4-ナレッジ移管資料ipa-工程-149)）完成 + 受領確認 | ☐ Pass / ☐ Fail |
| 契約完了 | 検収完了 + 契約上の義務全履行 | ☐ Pass / ☐ Fail |
| 振り返り | Retrospective（[`08-closure.md` §A.3](08-closure.md#a3-retrospective-議事録ipa-工程-148)）実施 + 改善策が次 PJ 計画に反映 | ☐ Pass / ☐ Fail |
| Archive | Archive 手順（[`08-closure.md` §A.5](08-closure.md#a5-アーカイブ手順書ipa-工程-150)）完了 | ☐ Pass / ☐ Fail |
| 完了報告 | 完了報告書（[`08-closure.md` §A.2](08-closure.md#a2-完了報告書ipa-工程-147)）提出 | ☐ Pass / ☐ Fail |

### A.8.4 完了基準

- 全 10 カテゴリ Pass
- PO + PM + 経営層の全会一致

---

## 9. 用語集

| 用語 | 説明 | 出典 |
|---|---|---|
| レビュー | 文書・成果物の妥当性を多人数で検証する活動 | IPA 共通フレーム |
| ゲート | フェーズ通過判定の節目 | IPA 共通フレーム |
| GO / NO-GO | リリース可否の最終判定 | PMBOK |
| 承認者 | 文書の最終責任を持つロール | IPA 共通フレーム |
| チェックリスト | 確認項目の列挙 | 本書 |

---

## 10. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018 年 3 月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018 年 4 月
3. PMBOK Guide 第 7 版、Project Management Institute、2021 年
4. Ada プロジェクトチーム「[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md)」、2026-08-20
5. Ada プロジェクトチーム「[DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md)」、2026-08-19
6. Ada プロジェクトチーム「[DOC-REQ-001 要件定義書](../legacy/requirements.md)」、2026-08-18
7. Ada プロジェクトチーム「[DOC-ARCH-004 原子化デプロイ](../architecture/04-atomic-deployment.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
