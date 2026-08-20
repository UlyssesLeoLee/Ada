# スコープベースライン（Scope Statement）

> **本文件の目的**：[PJ Charter](../upstream/01-pj-charter.md) §3 で定義した In/Out-of-Scope を **正式なベースライン** として確立する。スコープ変更は CR 経由のみ。  
> 関連 IPA 工程: 143（スコープ管理）+ 144（ベースライン管理）。

> **ドキュメントID**：DOC-MGT-SCP-001
> **文書分類**：管理文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/upstream/01-pj-charter.md`](../upstream/01-pj-charter.md)
> **下位文書**：[`docs/templates/06-change-management.md` §A.1 CR](../templates/06-change-management.md#a1-変更要求チケットipa-工程-118)
> **関連文書**：[`docs/CHANGELOG.md`](../CHANGELOG.md) v1.0.0（baseline entry）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - PMBOK Guide 第 7 版

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 143 + 144 に対応） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. ベースライン宣言
2. In-Scope
3. Out-of-Scope
4. スコープ制約
5. スコープ変更管理
6. 完了基準
7. 用語集
8. 参考文献

---

## 1. ベースライン宣言

| 項目 | 内容 |
|---|---|
| ベースライン番号 | SCOPE-BL-v1.0.0 |
| 制定日 | 2026-08-20 |
| 承認者 | PO + PM + 経営層 |
| 関連 PJ Charter | [DOC-UP-001 §3](../upstream/01-pj-charter.md) |
| CHANGELOG エントリ | v1.0.0 |
| 次回見直し | G1（要件 Baseline）通過時 |

## 2. In-Scope（含む）

| 区分 | 範囲 | 関連 M-ID |
|---|---|---|
| 機能 | 16 モジュール（[DOC-MOD-001〜016](../modules/M-01-acquisition-adapter.md)） | 全 |
| 機能 | 管理画面（[DOC-ARCH-006](../architecture/05-admin-operations-ui.md)） | M-14, M-15, M-16 |
| 機能 | 公開 API + 管理 API（[DOC-API-001〜006](../api/rest-endpoints.md)） | M-13 |
| 機能 | プラグイン SDK（[DOC-MOD-006](../modules/M-06-node-runtime-plugin-sdk.md)） | M-06 |
| プラットフォーム | macOS 14+ | 全 |
| プラットフォーム | Linux (Ubuntu 22.04+) | 全 |
| プラットフォーム | Windows 11+ | 全 |
| デプロイ | 単機モード | 全 |
| デプロイ | SaaS マルチテナント | M-10, M-13, M-16 |
| デプロイ | ハイブリッド（オンプレ + クラウド） | 全 |
| データソース | REST API | M-01 |
| データソース | DB (PostgreSQL, MySQL, Oracle) | M-01 |
| データソース | File (CSV, JSON, Parquet) | M-01 |
| データソース | gRPC | M-01 |
| データソース | WebSocket | M-01 |
| データソース | CDC | M-01 |
| 規模 | 10,000 ノード / 画布 | M-03, M-12 |
| 規模 | 100 同時編集ユーザー | M-11, M-12 |
| 規模 | 10,000 テナント | M-10 |
| 規模 | 100,000 WebSocket 同時接続 | M-13 |
| 言語 | UI: 日本語、英語、中国語 | M-12 |
| 言語 | ログ: 英語 | 全 |
| コンプライアンス | GDPR | M-10 |
| コンプライアンス | PIPL | M-10 |
| コンプライアンス | APPI | M-10 |

## 3. Out-of-Scope（含まない）

| 区分 | 範囲 | 理由 |
|---|---|---|
| 機能 | モバイルアプリ（iOS, Android） | 第 2 フェーズ |
| 機能 | ネイティブプラグイン | 要件未定 |
| 機能 | AI 機能（自動 ETL 推薦、異常検知） | 将来検討 |
| 機能 | BaaS フル機能 | 部分的対応 |
| 機能 | ホスティングマネージド | 顧客責任 |
| 機能 | ワークフロー承認 | 第 2 フェーズ |
| 機能 | 帳票生成（PDF/Excel レイアウト） | [DOC-ARCH-009 §5.3 27 帳票設計](../architecture/08-workflow-overview.md) で ⊘ 対象外 |
| 機能 | データレイク構築 | 別 PJ |
| 機能 | BI ダッシュボード作成 | Tableau 既存利用 |
| 機能 | データカタログ | 第 2 フェーズ |
| 機能 | データ品質ダッシュボード | 第 2 フェーズ |
| 機能 | 機械学習モデル訓練 | 別 PJ |
| 機能 | ETL Marketplace | 将来検討 |
| 機能 | ライブストリーミング | 第 2 フェーズ |
| 機能 | VR/AR インターフェース | 検討対象外 |
| 機能 | 音声操作 | 検討対象外 |
| 規模 | 1M 同時接続 | ロードマップ |
| 規模 | 100M メッセージ/秒 | ロードマップ |
| コンプライアンス | HIPAA | 業界別 |
| コンプライアンス | PCI-DSS | 業界別 |
| コンプライアンス | FedRAMP | 米国政府 |

## 4. スコープ制約

| 制約 | 内容 | 影響 |
|---|---|---|
| 言語 | Rust 必須 | ライブラリ・人材 |
| DB | PostgreSQL 16+ | ベンダ固定 |
| デプロイ | atomic 反映必須 | 運用 |
| 3 OS | 必須 | ビルド |
| 既存システム並行 | 最大 30 日 | 移行 |

## 5. スコープ変更管理

### 5.1 変更プロセス

1. 起票: [DOC-TPL-CHG §A.1 CR](../templates/06-change-management.md#a1-変更要求チケットipa-工程-118)
2. 影響分析: [DOC-TPL-CHG §A.2 影響分析](../templates/06-change-management.md#a2-影響分析レポートipa-工程-119)
3. 承認: [DOC-TPL-CHG §A.3 変更承認](../templates/06-change-management.md#a3-変更承認記録ipa-工程-120)
4. 反映: [DOC-TPL-CHG §A.7 改修 PR](../templates/06-change-management.md#a7-改修-pr-テンプレipa-工程-124)

### 5.2 承認者

| 変更種別 | 承認者 |
|---|---|
| スコープ追加（小） | PM |
| スコープ追加（中） | PO + PM |
| スコープ追加（大） | PO + PM + 経営層 |
| スコープ削減 | PO のみ |

### 5.3 ベースライン保護

- 本ドキュメントの改訂には PO + PM + 経営層の全会一致
- 改訂時は新ベースライン番号（SCOPE-BL-v1.x.0）
- 旧ベースラインは [CHANGELOG.md](../CHANGELOG.md) に記録

## 6. 完了基準

- In/Out-of-Scope 明確
- ベースライン番号発番
- PO + PM + 経営層承認
- 変更管理プロセス確立

## 7. 用語集

| 用語 | 説明 |
|---|---|
| Scope | スコープ（作業範囲） |
| In-Scope | 含む範囲 |
| Out-of-Scope | 含まない範囲 |
| Baseline | ベースライン（基準点） |
| CR | Change Request（変更要求） |

## 8. 参考文献

1. PMBOK Guide 第 7 版、Project Management Institute、2021 年
2. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
3. Ada プロジェクトチーム「[DOC-UP-001 PJ Charter §3](../upstream/01-pj-charter.md)」、2026-08-20

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
