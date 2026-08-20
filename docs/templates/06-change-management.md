# 変更・保守管理テンプレート集（Change & Maintenance Management Templates）

> **本ファイルの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) §5.13（保守プロセス、IPA 工程 118-126）に対応する **9 種類の変更・保守管理テンプレート** を提供する。  
> 変更要求、影響分析、承認、構成管理、Patch、脆弱性対応、改修、Hotfix、回帰テストを 1 件 = 1 チケットで管理。

> **ドキュメントID**：DOC-TPL-CHG
> **文書分類**：テンプレート集
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/templates/README.md`](README.md)（DOC-TPL-INDEX）、[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)（DOC-ARCH-009）
> **下位文書**：派生版（`docs/records/changes/<テンプレ DOC-ID>-TKT-<連番>.md`）
> **関連文書**：
> - [`docs/architecture/04-atomic-deployment.md`](../architecture/04-atomic-deployment.md)
> - [`docs/architecture/03-cross-cutting-risks.md`](../architecture/03-cross-cutting-risks.md)
> - [`docs/modules/M-14-module-registry.md`](../modules/M-14-module-registry.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - ITIL 4（変更管理）
> - JIS X 0160:2012
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（CR / 影響分析 / 承認 / CM / Patch / Vuln / 改修 / Hotfix / 回帰 の 9 テンプレート） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 変更要求チケット（IPA 工程 118）
2. 影響分析レポート（IPA 工程 119）
3. 変更承認記録（IPA 工程 120）
4. 構成管理台帳（IPA 工程 121）
5. Patch ログ（IPA 工程 122）
6. 脆弱性対応ログ（IPA 工程 123）
7. 改修 PR テンプレ（IPA 工程 124）
8. Hotfix 手順書（IPA 工程 125）
9. 回帰テストログ（IPA 工程 126）
10. 用語集
11. 参考文献

---

## A.1 変更要求チケット（IPA 工程 118）

### A.1.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 118（変更要求 / CR） |
| 目的 | 機能追加・変更・廃止の要求をトリアージし、影響分析と承認に回す |
| 記入者 | PO / Biz / ユーザー |
| 記入タイミング | 要求発生時（即時） |
| 関連ドキュメント | [DOC-ARCH-004 §2 変更管理](../architecture/04-atomic-deployment.md) |

### A.1.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CHG-TKT-<連番 4 桁>
CR ID: CR-<YYYY>-<連番 4 桁>
起票日: ____-__-__
起票者: <氏名>
種別: ☐ 機能追加  ☐ 機能変更  ☐ 機能廃止  ☐ バグ修正  ☐ 性能改善  ☐ セキュリティ  ☐ ドキュメント  ☐ その他
優先度: ☐ Critical（即対応）  ☐ High（今sprint）  ☐ Medium（次sprint）  ☐ Low（バックログ）
影響範囲: ☐ Local  ☐ Module  ☐ 全体  ☐ 複数テナント  ☐ 全テナント
```

### A.1.3 チケット詳細

| 項目 | 内容 |
|---|---|
| タイトル | <件名> |
| 概要 | <背景 + 目的> |
| 関連 F-ID | F-NN |
| 関連 NF 区分 | [NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV] |
| 影響を受ける M-ID | M-NN |
| 影響を受ける API | DOC-API-NNN §X |
| 期待する動作 | <詳細> |
| 受入条件 | 1. ... 2. ... |
| 関連リンク | <URL> |
| 添付資料 | <ファイル> |
| ステークホルダー | <PO / Biz / アーキ / Dev / SRE / SecO> |
| 関連 Issue | <ISS-NNNN> |

### A.1.4 状態遷移

| 状態 | 日付 | 担当 | 備考 |
|---|---|---|---|
| ☐ Open | | | 起票 |
| ☐ Triage | | | 優先度・種別確認 |
| ☐ Impact Analysis | | | [§A.2](#a2-影響分析レポートipa-工程-119) |
| ☐ Approved | | | [§A.3](#a3-変更承認記録ipa-工程-120) |
| ☐ In Progress | | | 実装 |
| ☐ Testing | | | [§A.9](#a9-回帰テストログipa-工程-126) |
| ☐ Released | | | [DOC-TPL-RBK-RBK-prod-deploy](04-runbooks.md#a8-本番デプロイ記録ipa-工程-105) |
| ☐ Closed | | | 完了 |

---

## A.2 影響分析レポート（IPA 工程 119）

### A.2.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CHG-TKT-<連番>-IA
対象 CR: CR-<YYYY>-<連番>
作成日: ____-__-__
作成者: <アーキ>
レビュー: <テックリード + DBA + SRE + SecO>
```

### A.2.2 影響範囲分析

| 観点 | 影響範囲 | 詳細 | 対応 |
|---|---|---|---|
| 機能影響 | ☐ あり / ☐ なし | <内容> | <対応> |
| データ影響 | ☐ あり / ☐ なし | スキーマ変更、マイグレーション要否 | <対応> |
| API 影響 | ☐ あり / ☐ なし | breaking change / 後方互換 | <対応> |
| UI 影響 | ☐ あり / ☐ なし | <内容> | <対応> |
| 性能影響 | ☐ あり / ☐ なし | <内容> | <対応> |
| セキュリティ影響 | ☐ あり / ☐ なし | <内容> | <対応> |
| 運用影響 | ☐ あり / ☐ なし | 監視・アラート・Runbook 更新要否 | <対応> |
| 移行影響 | ☐ あり / ☐ なし | データ移行要否 | <対応> |
| 環境影響 | ☐ あり / ☐ なし | 3 OS 全部への影響 | <対応> |
| ドキュメント影響 | ☐ あり / ☐ なし | 関連 DOC 更新 | <対応> |
| テナント影響 | ☐ あり / ☐ なし | 既存テナントへの破壊的変更 | <対応> |
| 依存関係影響 | ☐ あり / ☐ なし | 依存 crate / API への影響 | <対応> |

### A.2.3 リスク評価

| リスク | 影響度 | 発生確率 | スコア | 対応 |
|---|---|---|---|---|
| <リスク> | 1-5 | 1-5 | __ | 回避 / 軽減 / 受容 |

### A.2.4 実装計画

| 項目 | 内容 |
|---|---|
| 想定工数 | __人日 |
| 想定リリース日 | ____-__-__ |
| ロールバック計画 | [DOC-ARCH-004 §2.5](../architecture/04-atomic-deployment.md) |
| テスト計画 | [§A.9](#a9-回帰テストログipa-工程-126) |
| 関連 PR | <URL> |

### A.2.5 完了基準

- 全観点の分析完了
- リスク対応計画明記
- アーキ + テックリード + DBA + SRE + SecO の合議

---

## A.3 変更承認記録（IPA 工程 120）

### A.3.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CHG-TKT-<連番>-APP
対象 CR: CR-<YYYY>-<連番>
参照影響分析: DOC-TPL-CHG-TKT-<連番>-IA
承認日: ____-__-__
承認者: <CAB（変更諮問委員会）>
```

### A.3.2 承認構成

| ロール | 氏名 | 承認 | 条件 / 反対理由 |
|---|---|---|---|
| PO | | ☐ | |
| PM | | ☐ | |
| アーキ | | ☐ | |
| テックリード | | ☐ | |
| SRE | | ☐ | |
| SecO | | ☐ | |

### A.3.3 判定

| 項目 | 内容 |
|---|---|
| 判定 | ☐ 承認（実装可） ☐ 条件付き承認 ☐ 保留 ☐ 却下 |
| 条件 | <条件> |
| 却下理由 | <理由> |
| 実装期限 | ____-__-__ |
| リリース予定 | ____-__-__ |

### A.3.4 完了基準

- 全承認者の合意
- 1 名でも反対なら保留または却下

---

## A.4 構成管理台帳（IPA 工程 121）

### A.4.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 121（構成管理） |
| 目的 | 16 crate + DB スキーマ + ドキュメントのバージョン管理 |
| 記入者 | テックリード + PM |
| 記入タイミング | リリース毎（Git tag と連動） |
| 関連ドキュメント | [DOC-ARCH-007 §16.4](../architecture/06-rust-tech-selection.md) |

### A.4.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CHG-TKT-CM
管理対象: 16 crate + 11 テーブル + ドキュメントセット
更新: Git tag と連動（手動更新不要）
```

### A.4.3 構成管理台帳

| カテゴリ | 名称 | バージョン | Git tag | リリース日 | 担当 | 状態 |
|---|---|---|---|---|---|---|
| crate | ada-core | v0.1.0 | v0.1.0-ada-core | YYYY-MM-DD | <氏名> | ☐ Active / ☐ Deprecated |
| crate | ada-telemetry | v0.1.0 | v0.1.0-ada-telemetry | YYYY-MM-DD | <氏名> | ☐ |
| crate | ada-m01-acquisition | v0.1.0 | ... | | | ☐ |
| ... | ... | ... | ... | ... | ... | ... |
| DB | schema (全体) | v0.1.0 | tag: db-v0.1.0 | | <DBA> | ☐ |
| ドキュメント | docs 全体 | v1.0.0 | tag: docs-v1.0.0 | | <PM> | ☐ |

### A.4.4 変更履歴

| 変更日 | 変更種別 | 変更内容 | 影響バージョン | 関連 CR |
|---|---|---|---|---|
| YYYY-MM-DD | 機能追加 | <内容> | v0.x.x → v0.y.y | CR-YYYY-NNNN |

### A.4.5 完了基準

- 全 crate / DB / ドキュメントにバージョン付与
- Git tag と一致

---

## A.5 Patch ログ（IPA 工程 122）

### A.5.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CHG-TKT-PATCH-<YYYYMMDD>
適用日: ____-__-__
適用者: <SRE>
参照: [DOC-MOD-014 §2.4 atomic swap](../modules/M-14-module-registry.md)
参照: [DOC-ARCH-004 §2.5 ロールバック](../architecture/04-atomic-deployment.md)
```

### A.5.2 Patch 詳細

| 項目 | 内容 |
|---|---|
| Patch ID | PATCH-<YYYYMMDD>-<連番> |
| 対象 crate | <crate 名> |
| 変更概要 | <バグ修正 / 性能改善 / etc> |
| 変更前バージョン | v0.x.x |
| 変更後バージョン | v0.x.y |
| 関連 CR | CR-YYYY-NNNN |
| 関連 PR | <URL> |
| コミット SHA | `<SHA>` |

### A.5.3 適用ログ

| ステップ | コマンド | 結果 | 証跡 |
|---|---|---|---|
| 1. 事前 Backup | `pg_dump > pre-patch.dump` | ☐ | |
| 2. atomic swap | `./scripts/atomic-swap.sh` | ☐ | |
| 3. ヘルスチェック | `curl /health` | ☐ | |
| 4. Smoke | [DOC-TPL-RBK-RBK-smoke](04-runbooks.md#a9-smoke-test-実施ログipa-工程-106) | ☐ | |
| 5. 監視 | 30 分異常なし | ☐ | |

### A.5.4 ロールバック判断

| 状況 | 対応 |
|---|---|
| Smoke 全 Pass | 完了 |
| Smoke 1 件 Fail | ロールバック |
| 重大障害 | 即時ロールバック |

---

## A.6 脆弱性対応ログ（IPA 工程 123）

### A.6.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 123（脆弱性対応） |
| 目的 | 検出された脆弱性への迅速な対応履歴 |
| 記入者 | SecO + Dev |
| 記入タイミング | 脆弱性発見時（即時）、対応完了時 |
| 関連ドキュメント | [DOC-ARCH-003 §3.2](../architecture/03-cross-cutting-risks.md)、[DOC-ARCH-007 §15](../architecture/06-rust-tech-selection.md) |
| NF タグ | [NF-SEC]【必須】 |

### A.6.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CHG-TKT-VULN-<YYYYMMDD>
脆弱性 ID: VULN-<CVE / Internal ID>
検出日: ____-__-__
検出ツール: ☐ cargo-audit  ☐ cargo-deny  ☐ Snyk  ☐ Trivy  ☐ 手動
重大度: ☐ Critical  ☐ High  ☐ Medium  ☐ Low
CVSS スコア: __
```

### A.6.3 脆弱性詳細

| 項目 | 内容 |
|---|---|
| 対象 crate / 依存 | <crate 名 @ version> |
| 対象コンポーネント | <コード / DDL / API / 設定> |
| CVE 番号 | CVE-YYYY-NNNN |
| 説明 | <内容> |
| 攻撃シナリオ | <再現方法> |
| 影響範囲 | <どのデータが / どの操作で> |
| 既存対応 | ☐ なし ☐ WAF ☐ 緩和策あり |
| 恒久対策 | <crate upgrade / patch / 設定変更 / 設計変更> |
| 暫定対応 | ☐ WAF rule 追加 ☐ 機能停止 ☐ アクセス制限 |

### A.6.4 対応計画

| ステップ | 内容 | 担当 | 期限 |
|---|---|---|---|
| 1 | 影響範囲特定 | <SecO> | YYYY-MM-DD |
| 2 | 暫定対応 | <SRE> | YYYY-MM-DD |
| 3 | 恒久対策実装 | <Dev> | YYYY-MM-DD |
| 4 | テスト | <QA> | YYYY-MM-DD |
| 5 | リリース | <SRE> | YYYY-MM-DD |
| 6 | クローズ確認 | <SecO> | YYYY-MM-DD |

### A.6.5 SLA（IPA 非機能要件 [NF-SEC]【必須】）

| 重大度 | 暫定対応 | 恒久対策 |
|---|---|---|
| Critical | 24h 以内 | 7 日以内 |
| High | 72h 以内 | 30 日以内 |
| Medium | 1 週間以内 | 90 日以内 |
| Low | 次回リリースまで | - |

---

## A.7 改修 PR テンプレ（IPA 工程 124）

### A.7.1 ヘッダ部

```yaml
PR タイトル: [<crate>] <変更概要>
PR ID: PR-#<番号>
関連 CR: CR-<YYYY>-<連番>
ブランチ: feature/<cr-id>-<short-desc>
ベース: main
```

### A.7.2 PR 説明テンプレート

```markdown
## 概要
<変更の背景と目的を 1-2 文で>

## 変更内容
- <変更点 1>
- <変更点 2>

## 関連
- CR: CR-YYYY-NNNN
- 影響分析: DOC-TPL-CHG-TKT-NNNN-IA
- 関連 Issue: #NNN
- 関連 F-ID: F-NN
- 関連 NF: [NF-XXX]

## テスト
- [ ] 単体テスト追加（[DOC-TST-001] §X 準拠）
- [ ] 統合テスト追加（[DOC-TST-002] §X 準拠）
- [ ] 回帰テスト実施（[§A.9](#a9-回帰テストログipa-工程-126)）
- [ ] SAST: cargo deny / audit pass
- [ ] カバレッジ: 既存比 ≥ 100%

## チェックリスト
- [ ] コードレビュー（[DOC-ARCH-007 §15.4](../architecture/06-rust-tech-selection.md)）
- [ ] ドキュメント更新（API/モジュール §X）
- [ ] マイグレーション要否（[DOC-MOD-010 §4](../modules/M-10-tenant-middleware.md)）
- [ ] 設定変更要否（環境変数、シークレット）

## デプロイ計画
- リリース: v0.x.y
- リリース日: ____-__-__
- ロールバック: [DOC-ARCH-004 §2.5](../architecture/04-atomic-deployment.md)

## スクリーンショット / ログ
<必要に応じて>
```

---

## A.8 Hotfix 手順書（IPA 工程 125）

### A.8.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 125（緊急改修） |
| 目的 | Sev1/Sev2 インシデント発生時の緊急リリース手順 |
| 記入者 | SRE + Dev + テックリード |
| 記入タイミング | インシデント発生時（即時） |
| NF タグ | [NF-OPS]【必須】 |

### A.8.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CHG-TKT-HOTFIX-<YYYYMMDD>
Hotfix ID: HF-<YYYYMMDD>-<連番>
対象 Incident: INC-<YYYYMMDD>-<連番>
重大度: ☐ Sev1 ☐ Sev2
発動時刻: ____-__-__ __:__
発動者: <SRE Lead>
承認者: <PM + テックリード>（口頭承認 OK、事後記録）
```

### A.8.3 通常リリースとの差分

| 項目 | 通常 | Hotfix |
|---|---|---|
| 承認プロセス | [§A.3](#a3-変更承認記録ipa-工程-120) CAB | 口頭承認（事後記録） |
| テスト | UT + IT + ST + 回帰 | UT + 重要回帰 + Smoke |
| レビュー | 2 名 | 1 名（テックリード） |
| デプロイ時間 | 計画 | 即時 |
| ドキュメント | 完全 | 事後更新 |
| 影響評価 | 必須 | 簡易評価 |

### A.8.4 Hotfix 手順

| ステップ | コマンド / 操作 | 担当 | 結果 |
|---|---|---|---|
| 1. ブランチ作成 | `git checkout -b hotfix/hf-YYYYMMDD-NN main` | Dev | |
| 2. 最小修正 | 該当箇所のみ修正 | Dev | |
| 3. UT 実行 | `cargo test -p <crate>` | Dev | |
| 4. 緊急レビュー | テックリード 1 名 | Dev + テックリード | |
| 5. ビルド | `cargo build --release` | Dev | |
| 6. Smoke | [DOC-TPL-RBK-RBK-smoke](04-runbooks.md#a9-smoke-test-実施ログipa-工程-106) | QA | |
| 7. atomic 反映 | [DOC-TPL-RBK-RBK-prod-deploy](04-runbooks.md#a8-本番デプロイ記録ipa-工程-105) | SRE | |
| 8. 監視強化 | 1 時間厳格 | SRE | |
| 9. 事後ドキュメント | 本手順書 + PR 作成 | Dev | |
| 10. Postmortem 連動 | [§A.7 of 05-operations.md](05-operations.md#a7-postmortem-テンプレートipa-工程-115) | IC | |

### A.8.5 完了基準

- Incident 解消
- 5 時間以内に Hotfix 完了
- 事後 24 時間以内にドキュメント完備

---

## A.9 回帰テストログ（IPA 工程 126）

### A.9.1 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CHG-TKT-REG-<YYYYMMDD>
対象 CR: CR-<YYYY>-<連番>
対象リリース: v0.x.y
実施日: ____-__-__
実施者: <QA>
参照: [§A.12 of 02-tests-execution.md](02-tests-execution.md#a12-回帰試験ログipa-工程-75--126)
```

### A.9.2 回帰テスト範囲

| 区分 | ケース | 範囲 | 自動/手動 |
|---|---|---|---|
| 影響範囲 | <変更モジュール + 隣接> | M-NN | auto |
| 全 UT 再実行 | [DOC-TST-001](../tests/UT-design.md) 全 214 ケース | 全体 | auto |
| 主要 IT 再実行 | [DOC-TST-002](../tests/IT-design.md) 47 ケースから 10 ケース | 主要 API | auto |
| 主要 ST 再実行 | [DOC-TST-003](../tests/ST-design.md) 100 ケースから 20 ケース | 主要 E2E | auto + manual |
| 性能回帰 | [NF-PER] 計測 | 起動 / 1k node / p95 | auto |
| セキュリティ回帰 | SAST 再実行 | 全体 | auto |

### A.9.3 結果

| 区分 | 結果 | 備考 |
|---|---|---|
| UT | __/214 Pass | |
| IT | __/10 Pass | |
| ST | __/20 Pass | |
| 性能 | ☐ 目標達成 | |
| セキュリティ | ☐ 脆弱性 0 | |

### A.9.4 判定

☐ Pass（リリース可）  ☐ Fail（要追加対応）

---

## 10. 用語集

| 用語 | 説明 | 出典 |
|---|---|---|
| CR | Change Request（変更要求） | ITIL |
| CAB | Change Advisory Board（変更諮問委員会） | ITIL |
| 影響分析 | 変更が他に与える影響の評価 | ITIL |
| 構成管理 (CM) | 成果物の変更履歴と整合性管理 | ITIL |
| Patch | 修正リリース | 本書 |
| 脆弱性 | セキュリティ上の欠陥 | IPA [NF-SEC] |
| CVE | Common Vulnerabilities and Exposures | MITRE |
| CVSS | Common Vulnerability Scoring System | FIRST |
| Hotfix | 緊急修正リリース | 本書 |
| 回帰 | 既存機能が変更で壊れること | IPA 共通フレーム |
| atomic swap | 旧版保持での切替 | DOC-ARCH-004 |
| Sev1/2 | 重大度 | Google SRE |
| ロールバック | 旧版への切り戻し | 本書 |

---

## 11. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018 年 3 月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018 年 4 月
3. ITIL 4、AXELOS、2019 年
4. JIS X 0160:2012
5. Ada プロジェクトチーム「[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md)」、2026-08-20
6. Ada プロジェクトチーム「[DOC-ARCH-004 原子化デプロイ](../architecture/04-atomic-deployment.md)」、2026-08-19
7. Ada プロジェクトチーム「[DOC-MOD-014 モジュール登録](../modules/M-14-module-registry.md)」、2026-08-19
8. Ada プロジェクトチーム「[DOC-ARCH-007 Rust crate 選択](../architecture/06-rust-tech-selection.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
