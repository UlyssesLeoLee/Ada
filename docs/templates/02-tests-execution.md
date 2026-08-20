# 試験実施ログテンプレート集（Test Execution Logs Templates）

> **本ファイルの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) §5.6, §5.7, §5.8, §5.9 の ⚪ 試験工程（**60, 62-75, 78-88, 92-95**）に対応する **11 種類の試験実施ログ・仕様書テンプレート** を提供する。  
> 既存の [DOC-TST-001/002/003 試験設計書](../tests/README.md) と組み合わせて使用する。

> **ドキュメントID**：DOC-TPL-TST
> **文書分類**：テンプレート集
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/templates/README.md`](README.md)（DOC-TPL-INDEX）、[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)（DOC-ARCH-009）
> **下位文書**：派生版（`docs/records/tests/<テンプレ DOC-ID>-LOG-<YYYYMMDD>-<連番>.md`）
> **関連文書**：
> - [`docs/tests/README.md`](../tests/README.md)（DOC-TST-INDEX）
> - [`docs/tests/UT-design.md`](../tests/UT-design.md)（DOC-TST-001）
> - [`docs/tests/IT-design.md`](../tests/IT-design.md)（DOC-TST-002）
> - [`docs/tests/ST-design.md`](../tests/ST-design.md)（DOC-TST-003）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - JIS X 0160:2012
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（UT/IT/ST/UAT 全試験種別の 11 テンプレート） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 単体試験仕様書テンプレート（IPA 工程 60）
2. 単体試験実施ログ（IPA 工程 62）
3. 不具合修正記録（IPA 工程 63）
4. 再試験記録（IPA 工程 64）
5. UT 完了承認書（IPA 工程 65）
6. 内部結合試験実施ログ（IPA 工程 69）
7. 外部結合試験実施ログ（IPA 工程 70）
8. API 結合試験実施ログ（IPA 工程 71）
9. DB 結合試験実施ログ（IPA 工程 72）
10. 外部 IF 試験実施ログ（IPA 工程 73）
11. 障害対応記録（IPA 工程 74）
12. 回帰試験ログ（IPA 工程 75）
13. ST 実施ログ（IPA 工程 78-88 共通）
14. UAT 実施ログ（IPA 工程 92）
15. 業務シナリオ試験ログ（IPA 工程 93）
16. 検収書（IPA 工程 95）
17. 用語集
18. 参考文献

---

## A.1 単体試験仕様書テンプレート（IPA 工程 60）

### A.1.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 60（単体試験仕様書作成） |
| 対象 | [DOC-TST-001](../tests/UT-design.md) の **crate 別** 詳細仕様 |
| 記入者 | QA + Dev |
| 記入タイミング | crate 実装完了後、UT 実施前 |
| 関連ドキュメント | [DOC-TST-001](../tests/UT-design.md)、[DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md) |

### A.1.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-01
対象 crate: <crate 名（例: ada-canvas-editor, ada-m01-acquisition）>
対象 crate version: v0.x.x
作成日: ____-__-__
作成者: <Dev + QA>
参照設計書: [DOC-MOD-NNN] §3
参照 UT 設計: [DOC-TST-001] v1.x
```

### A.1.3 テストケース表

| ケース ID | 関数 / モジュール | 入力 | 期待出力 | 種別 | 優先度 | 自動化 |
|---|---|---|---|---|---|---|
| TC-UT-NN-01 | `pub fn xxx` | `input_value` | `expected_output` | 正常/異常/境界 | P0/P1/P2 | yes/no |
| TC-UT-NN-02 | ... | ... | ... | ... | ... | ... |

### A.1.4 完了基準

- 全 crate に対し ≥ 8 ケース
- 異常系 ≥ 2 ケース / crate
- 境界値網羅率 100%

---

## A.2 単体試験実施ログ（IPA 工程 62）

### A.2.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 62（単体試験実施） |
| 記入者 | Dev |
| 記入タイミング | `cargo test` 実行後 |
| 関連ドキュメント | [DOC-TPL-TST §A.1](#a1-単体試験仕様書テンプレートipa-工程-60) で作成した仕様書 |

### A.2.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-02
対象 crate: <crate 名>
実施日: ____-__-__
実施者: <Dev 氏名>
環境: rustc <ver> + cargo <ver>、OS = <macOS/Linux/Windows>
参照仕様書: DOC-TPL-TST-LOG-<YYYYMMDD>-01
CI 実行: ☐ GitHub Actions  #____
```

### A.2.3 実施ログ

| ケース ID | 実行時刻 | 結果 | 実行時間 | 証跡（cargo test --nocapture 出力抜粋） | 備考 |
|---|---|---|---|---|---|
| TC-UT-NN-01 | HH:MM:SS | ☐ Pass / ☐ Fail | __ms | `....` | |
| TC-UT-NN-02 | HH:MM:SS | ☐ Pass / ☐ Fail | __ms | `....` | |

### A.2.4 集計

| 区分 | 件数 |
|---|---|
| 総ケース数 | __ |
| 合格 | __ |
| 不合格 | __ |
| スキップ | __ |
| 合格率 | __% |
| コードカバレッジ（line） | __% |
| コードカバレッジ（branch） | __% |
| 実行時間合計 | __分 |

### A.2.5 完了基準

- 合格率 100%（不合格は [§A.3](#a3-不具合修正記録ipa-工程-63) で記録し [§A.4](#a4-再試験記録ipa-工程-64) で再試験）

---

## A.3 不具合修正記録（IPA 工程 63）

### A.3.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 63（不具合修正 / Bug Fix） |
| 記入者 | Dev |
| 記入タイミング | 不具合検出後 24 時間以内に起票 |

### A.3.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-03
Bug ID: BUG-<連番 4 桁>
起票日: ____-__-__
起票者: <Dev 氏名>
重大度: ☐ Critical  ☐ Major  ☐ Minor  ☐ Trivial
優先度: ☐ P0（即修正）  ☐ P1（次回 sprint）  ☐ P2（次期リリース）  ☐ P3（保留）
状態: ☐ Open  ☐ In Progress  ☐ Fixed  ☐ Closed
```

### A.3.3 不具合詳細

| 項目 | 内容 |
|---|---|
| 関連 crate / module | `ada-crate-name` / `module-path` |
| 関連 UT ケース | TC-UT-NN-NN |
| 関連 F-ID | F-NN |
| 関連 NF 区分 | [NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV] |
| 再現手順 | 1. ... 2. ... 3. ... |
| 期待動作 | `...` |
| 実際動作 | `...` |
| スクリーンショット / ログ | `添付: ...` |
| 根本原因 | `...` |
| 修正コミット | `commit <SHA>` |
| 修正概要 | `...` |
| 回帰影響 | `...` |
| 修正者 | <Dev 氏名> |
| 修正日 | ____-__-__ |
| 確認者 | <QA / テックリード> |
| 確認日 | ____-__-__ |

### A.3.4 完了基準

- Critical / Major は 24 時間以内に修正
- P0 は 1 sprint 以内
- 修正コミット → [§A.4](#a4-再試験記録ipa-工程-64) → [§A.5](#a5-ut-完了承認書ipa-工程-65) の流れで完了

---

## A.4 再試験記録（IPA 工程 64）

### A.4.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-04
対象 Bug ID: BUG-<連番 4 桁>
対象 crate: <crate 名>
再試験日: ____-__-__
再試験者: <Dev + QA>
参照修正記録: DOC-TPL-TST-LOG-<YYYYMMDD>-03
```

### A.4.2 再試験ログ

| ケース ID | 実行結果 | 証跡 |
|---|---|---|
| TC-UT-NN-01（元の失敗ケース） | ☐ Pass / ☐ Fail | `cargo test` 出力 |
| TC-UT-NN-02（隣接ケース） | ☐ Pass / ☐ Fail | `....` |
| TC-UT-NN-03（回帰影響範囲） | ☐ Pass / ☐ Fail | `....` |

### A.4.3 判定

| 項目 | 結果 |
|---|---|
| 不具合解消 | ☐ Pass / ☐ Fail |
| 回帰なし | ☐ Pass / ☐ Fail |
| 修正完了 | ☐ Pass / ☐ Fail |

---

## A.5 UT 完了承認書（IPA 工程 65）

### A.5.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-05
対象範囲: 全 16 crate（[DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md)）
完了日: ____-__-__
起票者: <QA>
承認者: QA + テックリード
判定: ☐ UT 完了（IT 着手可）  ☐ 継続
```

### A.5.2 完了確認

| 項目 | 目標 | 実測 | 達成 |
|---|---|---|---|
| 総ケース数 | ≥ 214 | __ | ☐ |
| 合格率 | 100% | __% | ☐ |
| コードカバレッジ（line） | ≥ 80% | __% | ☐ |
| コードカバレッジ（branch） | ≥ 70% | __% | ☐ |
| 重大不具合残存 | 0 件 | __件 | ☐ |
| 全 crate `cargo test` パス | 16/16 | __/16 | ☐ |
| CI green | 100% | __% | ☐ |

### A.5.3 完了基準

- 全 7 項目達成
- QA + テックリードの合議 GO

---

## A.6 内部結合試験実施ログ（IPA 工程 69 / ITa）

### A.6.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-06
試験名: 内部結合試験（ITa）
対象: モジュール間 I/F（[DOC-TST-002 §2](../tests/IT-design.md)）
実施日: ____-__-__
実施者: <QA + Dev>
環境: ローカル Docker compose（[DOC-ARCH-002 §5](../architecture/02-deployment.md)）
```

### A.6.2 ケース実行ログ

| ケース ID | 対象モジュール間 | 入力シナリオ | 期待結果 | 実測 | 結果 |
|---|---|---|---|---|---|
| TC-ITa-NN-01 | M-01 ↔ M-02 | CSV 1000 行投入 | 标准化完了 | ... | ☐ Pass / ☐ Fail |
| TC-ITa-NN-02 | M-02 ↔ M-03 | 标准化済 100 件 | 画布ノード生成 | ... | ☐ Pass / ☐ Fail |

### A.6.3 完了基準

- 全ケース Pass
- モジュール間 I/F 整合 100%

---

## A.7 外部結合試験実施ログ（IPA 工程 70 / ITb）

### A.7.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-07
試験名: 外部結合試験（ITb）
対象: FE ↔ BE 統合（[DOC-TST-002 §3](../tests/IT-design.md)）
実施日: ____-__-__
実施者: <QA + FE>
環境: ステージング（3 OS ブラウザ互換検証）
```

### A.7.2 ブラウザ × OS マトリクス結果

| ブラウザ | macOS | Linux (X11) | Windows | 備考 |
|---|---|---|---|---|
| Chrome 最新 | ☐ | ☐ | ☐ | |
| Safari 最新 | ☐ | N/A | N/A | |
| Firefox 最新 | ☐ | ☐ | ☐ | |
| Edge 最新 | N/A | N/A | ☐ | |

### A.7.3 ケース実行ログ（FE ↔ BE E2E 抜粋）

| ケース ID | シナリオ | 結果 | 証跡（スクリーンショット/ログ） |
|---|---|---|---|
| TC-ITb-NN-01 | ログイン → 画布ロード → ノード追加 → 保存 | ☐ Pass / ☐ Fail | |
| TC-ITb-NN-02 | マルチテナント切替 → データ分離確認 | ☐ Pass / ☐ Fail | |

### A.7.4 完了基準

- 全ブラウザ × 全 OS で Pass
- 互換性 100%

---

## A.8 API 結合試験実施ログ（IPA 工程 71）

### A.8.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-08
対象 API: [DOC-API-001 REST](../api/rest-endpoints.md), [DOC-API-002 WS](../api/websocket-events.md)
実施日: ____-__-__
実施者: <QA + Dev>
ツール: Postman / curl / REST Client + k6
OpenAPI バージョン: 3.1.x
```

### A.8.2 ケース実行ログ

| ケース ID | エンドポイント | メソッド | シナリオ | 期待ステータス | 期待 body | 実測 | 結果 |
|---|---|---|---|---|---|---|---|
| TC-API-NN-01 | `/api/v1/canvases` | POST | 正常作成 | 201 | `{id, ...}` | ... | ☐ Pass / ☐ Fail |
| TC-API-NN-02 | `/api/v1/canvases/{id}` | GET | 存在しない ID | 404 | `ERR_NOT_FOUND` | ... | ☐ Pass / ☐ Fail |
| TC-API-NN-03 | `/api/v1/...` | POST | バリデーションエラー | 400 | `ERR_VALIDATION` | ... | ☐ Pass / ☐ Fail |
| TC-API-NN-04 | `/api/v1/...` | POST | 認証なし | 401 | `ERR_UNAUTHENTICATED` | ... | ☐ Pass / ☐ Fail |
| TC-API-NN-05 | `/api/v1/...` | POST | テナント越境 | 403 | `ERR_TENANT_FORBIDDEN` | ... | ☐ Pass / ☐ Fail |

### A.8.3 OpenAPI スキーマ整合

- レスポンス body が OpenAPI 3.1 スキーマと 100% 整合 ☐ Pass / ☐ Fail
- エラーコード体系が [DOC-API-003](../api/error-codes.md) と一致 ☐ Pass / ☐ Fail

### A.8.4 完了基準

- 全 API エンドポイント網羅
- 異常系 4 種類（404, 400, 401, 403）全て確認

---

## A.9 DB 結合試験実施ログ（IPA 工程 72）

### A.9.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-09
対象: 11 テーブル（[DOC-MOD-010 §4](../modules/M-10-tenant-middleware.md)）+ RLS + PL/pgSQL
実施日: ____-__-__
実施者: <QA + DBA>
環境: PostgreSQL 16.x
```

### A.9.2 マイグレーション検証

| マイグレーション | 適用 | ロールバック | 整合 |
|---|---|---|---|
| M-001 初期スキーマ | ☐ | ☐ | ☐ |
| M-002 RLS ポリシー | ☐ | ☐ | ☐ |
| M-003 PL/pgSQL 存過 | ☐ | ☐ | ☐ |
| ... | | | |

### A.9.3 RLS ポリシー検証

| テスト ID | テナント | 期待結果 | 実測 | 結果 |
|---|---|---|---|---|
| TC-RLS-01 | tenant_a → tenant_a リソース | 200, 1 件 | ... | ☐ |
| TC-RLS-02 | tenant_a → tenant_b リソース | 0 件 or 403 | ... | ☐ |
| TC-RLS-03 | 未認証 | 401 | ... | ☐ |
| TC-RLS-04 | 管理者ロール | 全 tenant 可視 | ... | ☐ |

### A.9.4 PL/pgSQL 存過検証

| 存過名 | テストシナリオ | 結果 |
|---|---|---|
| `register_module` | 新規モジュール登録 | ☐ |
| `atomic_module_swap` | atomic 切替 + ロールバック | ☐ |
| `append_event` | イベント追記 + 重複検知 | ☐ |
| `acquire_lease` | リーダー選出 | ☐ |
| `release_lease` | リース解放 | ☐ |
| `register_node_heartbeat` | ハートビート + 失効検知 | ☐ |

### A.9.5 完了基準

- 全マイグレーション 適用 / ロールバック 正常
- RLS ポリシー越境 0 件
- 6 存過全て正常動作

---

## A.10 外部 IF 試験実施ログ（IPA 工程 73）

### A.10.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-10
対象: M-01 外部データソース連携（[DOC-MOD-001 §4.3](../modules/M-01-acquisition-adapter.md)）
実施日: ____-__-__
実施者: <QA + アーキ>
```

### A.10.2 外部システム別結果

| 外部システム | プロトコル | 接続テスト | データ取得 | 認証 | エラー処理 |
|---|---|---|---|---|---|
| REST API A | HTTPS + OAuth2 | ☐ | ☐ | ☐ | ☐ |
| DB B | TLS + SCRAM | ☐ | ☐ | ☐ | ☐ |
| WS C | WSS + JWT | ☐ | ☐ | ☐ | ☐ |
| ファイル D | S3 + IAM | ☐ | ☐ | ☐ | ☐ |

### A.10.3 障害系

- 接続タイムアウト ☐ 対応確認
- 認証失敗 ☐ リトライポリシー確認
- 大量データ ☐ ページネーション確認
- スキーマ不整合 ☐ エラー通知確認

---

## A.11 障害対応記録（IPA 工程 74）

### A.11.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-11
障害 ID: INC-<YYYYMMDD>-<連番>
発生日: ____-__-__
発生者: <QA / Dev / SRE>
重大度: ☐ Sev1（サービス全停止）  ☐ Sev2（主要機能停止）  ☐ Sev3（部分機能低下）  ☐ Sev4（軽微）
```

### A.11.2 障害詳細

| 項目 | 内容 |
|---|---|
| 関連 IPA 試験 | 66-89（IT/ST） |
| 関連モジュール | M-NN |
| 関連 F-ID | F-NN |
| 関連 NF 区分 | [NF-AVA\|PER\|SEC] |
| 再現手順 | 1. ... 2. ... |
| 期待動作 | ... |
| 実際動作 | ... |
| ログ | 添付: ... |
| 根本原因 | ... |
| 一時回避策 | ... |
| 恒久対策 | ... |
| 恒久対策リリース予定 | ____-__-__ |
| 教訓 / 再発防止 | ... |

---

## A.12 回帰試験ログ（IPA 工程 75 / 126）

### A.12.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-12
回帰対象: <変更されたモジュール + 関連モジュール>
実施日: ____-__-__
実施者: <QA>
変更 ID: <関連 [§A.13](#a13-st-実施ログipa-工程-78-88-共通) or [§A.3](#a3-不具合修正記録ipa-工程-63)>
```

### A.12.2 ケース実行

| ケース ID | 範囲 | 自動/手動 | 結果 |
|---|---|---|---|
| TC-REG-NN-01 | 影響範囲 × 既存テスト | auto | ☐ |
| TC-REG-NN-02 | 隣接モジュール × 既存テスト | auto | ☐ |
| TC-REG-NN-03 | 主要 E2E シナリオ | auto | ☐ |

### A.12.3 完了基準

- 既存テスト 100% パス
- 影響範囲のテストカバレッジ ≥ 既存比 100%

---

## A.13 ST 実施ログ（IPA 工程 78-88 共通）

### A.13.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-13
ST 種別: ☐ E2E 機能 (78)  ☐ シナリオ (79)  ☐ 性能 PT (80)  ☐ 負荷 (81)  ☐ ストレス (82)  ☐ Sec (83)  ☐ 障害 (84)  ☐ 復旧 (85)  ☐ B/R (86)  ☐ 可用性 (87)  ☐ 運用 (88)
対象: [DOC-TST-003 §<N>](../tests/ST-design.md)
実施日: ____-__-__
実施者: <QA + SRE + SecO>
環境: 本番相当（[DOC-ARCH-002 §3](../architecture/02-deployment.md)）
```

### A.13.2 テスト種別別ログ

#### 機能 / シナリオ試験（78, 79）

| ケース ID | シナリオ | 結果 | 証跡 |
|---|---|---|---|
| TC-ST-NN-01 | ユーザー操作 E2E | ☐ Pass / ☐ Fail | スクショ + ログ |
| ... | ... | ... | ... |

#### 性能試験（80）

| メトリクス | 目標 | 実測 | 達成 |
|---|---|---|---|
| 起動時間 | < 3s | __s | ☐ |
| 1k node 操作レイテンシ | < 100ms | __ms | ☐ |
| スループット | > __ req/s | __ req/s | ☐ |
| リソース使用率 | CPU < 70%, Mem < 80% | __% / __% | ☐ |

#### 負荷試験（81）

| 同時ユーザー | 目標 | 実測 | エラー率 |
|---|---|---|---|
| 100 | p95 < 200ms | __ms | __% |
| 500 | p95 < 500ms | __ms | __% |
| 1000 | p95 < 1s, エラー < 0.1% | __ms | __% |

#### ストレス試験（82）

- 限界点: __ 同時ユーザーで劣化
- 復旧挙動: ☐ 確認
- データ整合: ☐ 確認

#### セキュリティ試験（83）

| 項目 | 結果 |
|---|---|
| SAST（cargo-deny / cargo-audit） | ☐ 重大 = 0 |
| DAST（OWASP ZAP） | ☐ 重大 = 0 |
| ペネトレーションテスト | ☐ 重大 = 0 |
| 認証 / 認可 / 監査ログ | ☐ |
| GDPR / PIPL データ保護 | ☐ |
| 暗号化（in-transit / at-rest） | ☐ |

#### 障害 / 復旧 / B/R 試験（84, 85, 86）

| シナリオ | RTO | RPO | 結果 |
|---|---|---|---|
| ノード 1 台停止 | < 30s | 0 | ☐ |
| データセンター 1 つ停止 | < 5min | < 1min | ☐ |
| DB 全損 | < 1h | < 5min | ☐ |
| Backup → Restore | < 1h | 0 | ☐ |

#### 可用性試験（87）

- SLA 99.9% 達成可能性: ☐ 確認（7 日連続稼働）
- 計画外ダウンタイム: __分

#### 運用試験（88）

- Runbook シナリオ実行: ☐ 全シナリオ成功
- 監視アラート発火: ☐ 確認
- インシデント対応: ☐ 30min 以内

### A.13.3 完了基準

- 全テスト種別 Pass
- 全 NF 区分目標達成

---

## A.14 UAT 実施ログ（IPA 工程 92）

### A.14.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-14
対象: [DOC-ACC-001](../tests/ST-design.md) §7
実施日: ____-__-__
実施者: <Biz ユーザー>
環境: UAT 環境（本番相当）
```

### A.14.2 ケース実行

| ケース ID | 業務シナリオ | 期待 | 実測 | 結果 | ユーザー所感 |
|---|---|---|---|---|---|
| TC-UAT-01 | データ取り込み → 標準化 → 画布表示 | ... | ... | ☐ | |
| TC-UAT-02 | 複数ユーザー同時編集 → CRDT 解決 | ... | ... | ☐ | |
| ... | ... | ... | ... | ... | ... |

---

## A.15 業務シナリオ試験ログ（IPA 工程 93）

### A.15.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-15
シナリオ名: <業務名>
作成日: ____-__-__
作成者: <Biz + PO>
```

### A.15.2 シナリオ詳細

| 項目 | 内容 |
|---|---|
| 業務背景 | ... |
| アクター | 〇〇（業務ロール） |
| 前提条件 | ... |
| シナリオ手順 | 1. ... 2. ... 3. ... |
| 期待結果（業務） | ... |
| 実測結果 | ... |
| 業務適合性 | ☐ 適合 / ☐ 不適合（理由: ...） |

---

## A.16 検収書（IPA 工程 95）

### A.16.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-TST-LOG-<YYYYMMDD>-16
検収日: ____-__-__
検収者: <契約担当 + PO>
対象システム: Ada 无限画布跨平台数据集成系统 v1.x
参照: [§A.6 受入判定書](#a6-受入判定書ipa-工程-94--g8)
```

### A.16.2 検収条件

| 項目 | 条件 | 実測 | 検収 |
|---|---|---|---|
| 全 F-ID 実装 | 100% | __% | ☐ |
| 全 NF 区分達成 | 100% | __% | ☐ |
| UAT 全合格 | 100% | __% | ☐ |
| 残存重大不具合 | 0 件 | __件 | ☐ |
| ドキュメント完備 | 全 DOC-ID | __/__ | ☐ |
| 教育完了 | 管理者 + ユーザー | ☐ | ☐ |
| サポート体制 | 確立 | ☐ | ☐ |

### A.16.3 検収判定

☐ 合格（検収可）  ☐ 不合格（差し戻し、理由: ...）

---

## 17. 用語集

| 用語 | 説明 | 出典 |
|---|---|---|
| 単体試験 (UT) | モジュール単位の試験 | IPA 共通フレーム |
| 結合試験 (IT) | モジュール間 I/F 試験 | IPA 共通フレーム |
| システム試験 (ST) | システム全体での試験 | IPA 共通フレーム |
| 受入試験 (UAT) | ユーザー視点での最終試験 | IPA 共通フレーム |
| 回帰試験 | 変更が既存機能に影響しないか確認する試験 | IPA 共通フレーム |
| Smoke Test | 簡易な動作確認 | 本書 |
| RTO | Recovery Time Objective（復旧時間目標） | DR 標準 |
| RPO | Recovery Point Objective（復旧時点目標） | DR 標準 |
| RLS | Row-Level Security | PostgreSQL |
| SLA | Service Level Agreement | ITIL |
| Sev1〜4 | 障害重大度レベル | Google SRE |
| 根本原因 | 障害の真因 | ITIL |

---

## 18. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018 年 3 月
2. JIS X 0160:2012「ソフトウェアライフサイクルプロセス」
3. Google SRE Book 第 2 版、Google、2020 年
4. Ada プロジェクトチーム「[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md)」、2026-08-20
5. Ada プロジェクトチーム「[DOC-TST-INDEX テスト総覧](../tests/README.md)」、2026-08-19
6. Ada プロジェクトチーム「[DOC-TST-001〜003 試験設計](../tests/UT-design.md)」、2026-08-19
7. Ada プロジェクトチーム「[DOC-ARCH-007 Rust crate 選択](../architecture/06-rust-tech-selection.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
