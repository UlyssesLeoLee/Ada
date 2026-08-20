# 終結テンプレート集（Project Closure Templates）

> **本ファイルの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) §5.16（終結プロセス、IPA 工程 146-150）に対応する **5 種類の終結テンプレート** を提供する。  
> 成果物引渡し、完了報告、Retrospective、ナレッジ移管、Archive の 5 活動を確実に実施する。

> **ドキュメントID**：DOC-TPL-CLO
> **文書分類**：テンプレート集
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/templates/README.md`](README.md)（DOC-TPL-INDEX）、[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)（DOC-ARCH-009）
> **下位文書**：派生版（`docs/records/closure/<テンプレ DOC-ID>-FINAL.md` 等）
> **関連文書**：
> - [`docs/CHANGELOG.md`](../CHANGELOG.md)（DOC-CHG-001）
> - [`docs/legacy/`](../legacy/)
> - [`docs/architecture/08-workflow-overview.md` §7 G11](../architecture/08-workflow-overview.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - PMBOK Guide 第 7 版
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（引渡し / 完了報告 / Retrospective / KT / Archive の 5 テンプレート） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 成果物引渡し書（IPA 工程 146）
2. 完了報告書（IPA 工程 147）
3. Retrospective 議事録（IPA 工程 148）
4. ナレッジ移管資料（IPA 工程 149）
5. アーカイブ手順書（IPA 工程 150）
6. 用語集
7. 参考文献

---

## A.1 成果物引渡し書（IPA 工程 146）

### A.1.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 146（成果物引渡し） |
| 目的 | 開発成果物の正式な引渡しを文書化 |
| 記入者 | PM + PO |
| 記入タイミング | PJ 完了判定後（[§A.8 of 01-reviews.md](01-reviews.md#a8-プロジェクト完了判定書ipa-工程-145--g11)） |
| 関連ドキュメント | 全 DOC-ID 一覧 |

### A.1.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CLO-handover
引渡し日: ____-__-__
引渡し元: <開発チーム / PM>
引渡し先: <PO / 顧客 / 運用チーム>
対象 PJ: Ada 无限画布跨平台数据集成系统 v1.x
参照: [§A.8 of 01-reviews.md PJ 完了判定書](01-reviews.md#a8-プロジェクト完了判定書ipa-工程-145--g11)
```

### A.1.3 成果物一覧

| 区分 | 成果物 | バージョン | 数量 | 保管場所 | 受領確認 |
|---|---|---|---|---|---|
| 設計書 | [DOC-INDEX-001 README](../README.md) | v1.5.0 | 1 | docs/ | ☐ |
| 設計書 | [DOC-ARCH-001〜008](../architecture/00-anatomy-model.md) | various | 8 | docs/architecture/ | ☐ |
| 設計書 | [DOC-MOD-001〜016](../modules/M-01-acquisition-adapter.md) | various | 16 | docs/modules/ | ☐ |
| 設計書 | [DOC-API-001〜006](../api/rest-endpoints.md) | various | 6 | docs/api/ | ☐ |
| 設計書 | [DOC-TST-001〜003](../tests/UT-design.md) | various | 3 | docs/tests/ | ☐ |
| 設計書 | [DOC-TPL-INDEX + 8 カテゴリ](../templates/README.md) | v1.0.0 | 9 | docs/templates/ | ☐ |
| ソースコード | 16 crate + ada-core + ada-telemetry = 18 | v0.x.0 | — | GitHub repo | ☐ |
| バイナリ | 16 crate リリースビルド | v0.x.0 | — | GitHub Releases | ☐ |
| DB | 11 テーブル DDL + 6 PL/pgSQL + RLS ポリシー | v0.x.0 | — | migrations/ | ☐ |
| コンテナ | 18 crate Docker イメージ | v0.x.0 | — | Container Registry | ☐ |
| 運用 | 全 Runbook（[DOC-TPL-RBK-RBK-*](04-runbooks.md)） | v1.0.0 | 11 | docs/runbooks/ | ☐ |
| 運用 | 監視設定（[§A.2 of 05-operations.md](05-operations.md#a2-監視設定書ipa-工程-110)） | v1.0.0 | 1 | Prometheus | ☐ |
| 契約 | 検収書（[§A.16 of 02-tests-execution.md](02-tests-execution.md#a16-検収書ipa-工程-95)） | — | 1 | — | ☐ |
| 教育 | トレーニング資料 | — | — | — | ☐ |

### A.1.4 引渡し条件

| 項目 | 条件 | 実測 | 確認 |
|---|---|---|---|
| 検収完了 | G8 通過 | ☐ | ☐ |
| 残存課題 | 別 PJ / 保守として整理 | ☐ | ☐ |
| ドキュメント完備 | 全 DOC-ID 受領 | ☐ | ☐ |
| サポート体制 | 確立 | ☐ | ☐ |
| 契約上の義務 | 履行 | ☐ | ☐ |

### A.1.5 受領署名

| ロール | 氏名 | 署名 | 日付 |
|---|---|---|---|
| 引渡し元（PM） | | | |
| 引渡し先（PO） | | | |
| 顧客代表 | | | |

---

## A.2 完了報告書（IPA 工程 147）

### A.2.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 147（完了報告） |
| 目的 | PJ の結果を経営層 / 関係者に報告 |
| 記入者 | PM |
| 記入タイミング | PJ 完了後 2 週間以内 |
| 関連ドキュメント | [§A.8 of 01-reviews.md PJ 完了判定書](01-reviews.md#a8-プロジェクト完了判定書ipa-工程-145--g11)、[§A.3 Retrospective](#a3-retrospective-議事録ipa-工程-148) |

### A.2.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CLO-FINAL
報告日: ____-__-__
報告者: <PM>
報告先: <経営層 / PO / 関係部署>
対象 PJ: Ada 无限画布跨平台数据集成系统 v1.x
PJ 期間: YYYY-MM-DD 〜 YYYY-MM-DD
```

### A.2.3 PJ 概要

| 項目 | 内容 |
|---|---|
| 目的 | <PJ 目的> |
| スコープ | <主要スコープ> |
| 期間 | 計画: __ 実績: __ 差分: __ |
| 予算 | 計画: ¥__ 実績: ¥__ 差分: __ |
| 人員 | 計画: __人月 実績: __人月 差分: __ |
| 主要成果物 | <一覧> |

### A.2.4 達成評価

| 目標 | 計画 | 実績 | 達成 |
|---|---|---|---|
| 全 F-ID 実装 | 100% | __% | ☐ |
| 全 NF 区分達成 | 100% | __% | ☐ |
| SLA 99.9% | 99.9% | __% | ☐ |
| 重大脆弱性 | 0 件 | __件 | ☐ |
| ユーザー受入 | 100% Pass | __% Pass | ☐ |
| 予算内 | 100% | __% | ☐ |
| 期間内 | 100% | __% | ☐ |

### A.2.5 振り返り要約

| 観点 | 内容 |
|---|---|
| 成功要因 | <詳細> |
| 課題 | <詳細> |
| 教訓 | <詳細> |

（詳細は [§A.3 Retrospective](#a3-retrospective-議事録ipa-工程-148) 参照）

### A.2.6 残存課題 / 次 PJ 引き継ぎ事項

| 課題 ID | 内容 | 担当 | 期限 |
|---|---|---|---|
| ISS-NN | <内容> | <氏名> | YYYY-MM-DD |
| ... | ... | ... | ... |

### A.2.7 完了基準

- 全達成評価項目を測定
- 残存課題に担当 + 期限設定
- 経営層承認

---

## A.3 Retrospective 議事録（IPA 工程 148）

### A.3.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 148（振り返り） |
| 目的 | PJ の振り返りを通じて学びを獲得し、次 PJ へ反映 |
| 記入者 | PM + 全参加者 |
| 記入タイミング | PJ 完了後 1 ヶ月以内 |
| 関連ドキュメント | [§A.2 完了報告書](#a2-完了報告書ipa-工程-147) |

### A.3.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CLO-retro
実施日: ____-__-__
実施者: <PM>
参加者: <PJ メンバー全員>
形式: ☐ KPT  ☐ YWT  ☐ 4Ls  ☐ Start/Stop/Continue
```

### A.3.3 KPT 表

| 種別 | 内容 | 担当 | 次 PJ 反映 |
|---|---|---|---|
| **Keep（続ける）** | | | |
| | <成功した点> | <氏名> | ☐ 次 PJ で継続 |
| | ... | ... | ... |
| **Problem（問題）** | | | |
| | <発生した問題> | <氏名> | ☐ 改善策: ... |
| | ... | ... | ... |
| **Try（試す）** | | | |
| | <次 PJ で試すこと> | <氏名> | ☐ 次 PJ で実施 |
| | ... | ... | ... |

### A.3.4 学んだこと

- 技術面: ...
- プロセス面: ...
- チーム面: ...
- ステークホルダー面: ...

### A.3.5 改善アクション（次 PJ 計画への反映）

| 改善 ID | 内容 | 担当 | 次 PJ 計画への反映 |
|---|---|---|---|
| RETRO-NN | <内容> | <氏名> | ☐ <計画項目> |
| ... | ... | ... | ... |

### A.3.6 完了基準

- 全参加者参加
- 改善アクションを次 PJ 計画に明記

---

## A.4 ナレッジ移管資料（IPA 工程 149）

### A.4.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 149（ナレッジ移管 / KT） |
| 目的 | PJ で得られた知見を組織 / 後続 PJ へ移転 |
| 記入者 | テックリード + アーキ |
| 記入タイミング | PJ 完了判定後 |
| 関連ドキュメント | 全 DOC-ID、Retrospective、Postmortem 一覧 |

### A.4.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CLO-KT
移管日: ____-__-__
移管元: <開発チーム>
移管先: <運用チーム / 次 PJ チーム / 組織ナレッジベース>
対象範囲: ☐ アーキテクチャ  ☐ 実装  ☐ 運用  ☐ 障害対応  ☐ プロセス
```

### A.4.3 移管対象ナレッジ

| 区分 | ナレッジ | 形態 | 受領者 | 受領確認 |
|---|---|---|---|---|
| アーキテクチャ | 16 crate 構造と責務 | 図 + 解説 | <SRE> | ☐ |
| アーキテクチャ | 11 テーブル + RLS ポリシー | ER 図 + DDL | <DBA> | ☐ |
| 実装 | 6 PL/pgSQL 存過 | コード + 解説 | <DBA> | ☐ |
| 実装 | Cargo Workspace 16 crate | [DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md) | <次 PJ テックリード> | ☐ |
| 実装 | Bevy WASM 構成 | [DOC-MOD-012 §3](../modules/M-12-canvas-editor-frontend.md) | <FE> | ☐ |
| 運用 | 全 Runbook | [docs/templates/04-runbooks.md](04-runbooks.md) | <SRE> | ☐ |
| 運用 | 監視設定 | [§A.2 of 05-operations.md](05-operations.md#a2-監視設定書ipa-工程-110) | <SRE> | ☐ |
| 障害対応 | Incident Response | [§A.6 of 05-operations.md](05-operations.md#a6-incident-response-runbookipa-工程-114) | <SRE> | ☐ |
| 障害対応 | Postmortem 一覧 | <添付> | <SRE + テックリード> | ☐ |
| プロセス | WBS / 進捗 / 課題 / リスク | [docs/templates/03-process-management.md](03-process-management.md) | <次 PJ PM> | ☐ |
| プロセス | Retrospective 学んだこと | [§A.3](#a3-retrospective-議事録ipa-工程-148) | <次 PJ PM> | ☐ |
| 教訓 | ハマりポイント集 | <別途ドキュメント> | <全体> | ☐ |
| ベストプラクティス | 採用した crate / ツール | <別途ドキュメント> | <全体> | ☐ |

### A.4.4 KT セッション

| セッション | 対象 | 日時 | 参加者 | 資料 | 議事録 |
|---|---|---|---|---|---|
| KT-01: アーキテクチャ | <SRE> | ____-__-__ | | | |
| KT-02: 実装詳細 | <次 PJ チーム> | ____-__-__ | | | |
| KT-03: 運用 | <SRE> | ____-__-__ | | | |
| KT-04: 障害対応 | <SRE> | ____-__-__ | | | |
| KT-05: プロセス | <次 PJ PM> | ____-__-__ | | | |

### A.4.5 完了基準

- 全 KT セッション実施
- 受領者全員の確認署名

---

## A.5 アーカイブ手順書（IPA 工程 150）

### A.5.1 適用情報

| 項目 | 内容 |
|---|---|
| 適用 IPA 工程 | 150（アーカイブ） |
| 目的 | PJ の証跡を長期保管可能な形でアーカイブ |
| 記入者 | PM + テックリード |
| 記入タイミング | PJ 完了判定後 1 ヶ月以内 |
| 関連ドキュメント | [docs/legacy/](../legacy/) の慣行 |

### A.5.2 ヘッダ部

```yaml
DOC-ID: DOC-TPL-CLO-archive
アーカイブ日: ____-__-__
アーカイブ担当: <PM>
保管先: ☐ GitHub Releases (tag: archive-v<x>) ☐ 社内ファイルサーバー ☐ 長期保管ストレージ
保管期間: ☐ 5年 ☐ 7年 ☐ 永久（法令による）
暗号化: ☐ AES-256 ☐ KMS
```

### A.5.3 アーカイブ対象

| 区分 | 内容 | 容量 | 保管形式 |
|---|---|---|---|
| ソースコード | 16 crate + α | __MB | Git tag + tarball |
| バイナリ | 16 crate release build | __MB | GitHub Releases |
| DB ダンプ | 最終状態 | __GB | gzip + 暗号化 |
| ドキュメント | 全 DOC-ID | __MB | ZIP + 暗号化 |
| 設定ファイル | Terraform / K8s manifest | __MB | Git tag |
| 運用 Runbook | [docs/templates/04-runbooks.md](04-runbooks.md) 派生 | __MB | PDF + ZIP |
| 試験ログ | 全 UT/IT/ST 結果 | __GB | HTML / JSON |
| 障害対応 | 全 Postmortem | __MB | PDF |
| 議事録 | 全会議 | __MB | PDF |
| 契約関連 | 契約 / 検収書 | __MB | PDF + 暗号化 |
| 顧客データ | 該当する場合 | __GB | 暗号化 + アクセス制御 |

### A.5.4 アーカイブ手順

| ステップ | コマンド / 操作 | 結果 | 証跡 |
|---|---|---|---|
| 1. Git tag 作成 | `git tag -a archive-v1.x -m "..."` | ☐ | |
| 2. リポジトリ tarball | `git archive --format=tar.gz --output=archive-v1.x.tar.gz archive-v1.x` | ☐ | |
| 3. DB ダンプ | `pg_dump | gzip > db-final.sql.gz` | ☐ | |
| 4. ドキュメント ZIP | `zip -r docs-v1.x.zip docs/` | ☐ | |
| 5. 暗号化 | `gpg --symmetric --cipher-algo AES256 archive-v1.x.tar.gz` | ☐ | |
| 6. アップロード | `<保管先>` | ☐ | |
| 7. ハッシュ記録 | `sha256sum archive-v1.x.tar.gz > archive-v1.x.sha256` | ☐ | |
| 8. Archive ログ記録 | 本ファイル | ☐ | |
| 9. アクセス権設定 | `<権限>` | ☐ | |

### A.5.5 アクセス権

| ロール | アクセス権 |
|---|---|
| PM | 読み取り |
| アーキ | 読み取り |
| SRE | 読み取り |
| 監査担当 | 読み取り |
| 顧客 | なし（契約による） |

### A.5.6 復元テスト

| テスト日 | 復元対象 | 結果 | 整合 |
|---|---|---|---|
| YYYY-MM-DD | archive-v1.x | ☐ Pass / ☐ Fail | ☐ |
| YYYY-MM-DD（5年後） | archive-v1.x | ☐ Pass / ☐ Fail | ☐ |

### A.5.7 完了基準

- 全アーカイブ対象が保管
- 暗号化 + アクセス権設定
- 復元テスト合格

---

## 6. 用語集

| 用語 | 説明 | 出典 |
|---|---|---|
| 終結 | プロジェクトの完了と引き継ぎ | PMBOK |
| 成果物引渡し | 開発成果物の正式な移転 | PMBOK |
| 完了報告 | PJ 結果のステークホルダーへの報告 | PMBOK |
| Retrospective | 振り返り（KPT, YWT 等） | アジャイル / PMBOK |
| KPT | Keep / Problem / Try | アジャイル |
| ナレッジ移管 (KT) | 知識・経験の移転 | PMBOK |
| アーカイブ | 長期保管可能な状態での保存 | PMBOK |
| 暗号化 | データの機密保護 | [NF-SEC] |
| ハッシュ | データ整合性検証 | [NF-SEC] |
| 復元テスト | アーカイブからの復旧確認 | ITIL |

---

## 7. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018 年 3 月
2. PMBOK Guide 第 7 版、Project Management Institute、2021 年
3. ITIL 4、AXELOS、2019 年
4. Ada プロジェクトチーム「[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md)」、2026-08-20
5. Ada プロジェクトチーム「[DOC-INDEX-001 README](../README.md)」、2026-08-20
6. Ada プロジェクトチーム「[DOC-CHG-001 CHANGELOG](../CHANGELOG.md)」、2026-08-20
7. Ada プロジェクトチーム「[DOC-ARCH-007 Rust crate 選択](../architecture/06-rust-tech-selection.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
