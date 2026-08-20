# 要件定義ドキュメント（Requirements Documents）

> **本ディレクトリの目的**：[DOC-ARCH-009 ワークフロー全体俯瞰](../architecture/08-workflow-overview.md) §5.2（要件定義プロセス、IPA 工程 10-19）に対応する **9 種類の要件定義ドキュメント** を提供する。  
> 各要件種別（UR/BR/SR/FR/NFR/Data/IF/Sec/Ops/Mig）を独立したドキュメントとして管理し、トレーサビリティを担保する。

> **ドキュメントID**：DOC-REQ-INDEX
> **文書分類**：要件定義書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：
> - [`docs/upstream/06-to-be-business.md`](../upstream/06-to-be-business.md)（DOC-UP-006）
> - [`docs/upstream/07-to-be-system.md`](../upstream/07-to-be-system.md)（DOC-UP-007）
> - [`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)
> **下位文書**：
> - [`docs/requirements/01-ur-user-requirements.md`](01-ur-user-requirements.md)（DOC-REQ-UR-001）— 工程 10
> - [`docs/requirements/02-br-business-requirements.md`](02-br-business-requirements.md)（DOC-REQ-BR-001）— 工程 11
> - [`docs/requirements/03-sr-system-requirements.md`](03-sr-system-requirements.md)（DOC-REQ-SR-001）— 工程 12
> - [`docs/requirements/04-fr-functional-requirements.md`](04-fr-functional-requirements.md)（DOC-REQ-FR-001）— 工程 13
> - [`docs/requirements/05-nfr-non-functional-requirements.md`](05-nfr-non-functional-requirements.md)（DOC-REQ-NFR-001）— 工程 14
> - [`docs/requirements/06-data-requirements.md`](06-data-requirements.md)（DOC-REQ-DATA-001）— 工程 15
> - [`docs/requirements/07-external-if-requirements.md`](07-external-if-requirements.md)（DOC-REQ-IF-001）— 工程 16
> - [`docs/requirements/08-security-requirements.md`](08-security-requirements.md)（DOC-REQ-SEC-001）— 工程 17
> - [`docs/requirements/09-operation-requirements.md`](09-operation-requirements.md)（DOC-REQ-OPS-001）— 工程 18
> - [`docs/requirements/10-migration-requirements.md`](10-migration-requirements.md)（DOC-REQ-MIG-001）— 工程 19
> **関連文書**：
> - [`docs/legacy/requirements.md`](../legacy/requirements.md)（DOC-REQ-001、原本 v1.2.1）
> - [`docs/architecture/07-qa-register.md`](../architecture/07-qa-register.md)
> - 全モジュール文書（DOC-MOD-001〜016 §1 需求来源）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（要件 9 種 × 9 ファイル + 索引） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 使い方
2. ドキュメント一覧と IPA 工程マッピング
3. 要件階層のトレーサビリティ
4. 命名・ID 規約
5. 用語集
6. 参考文献

---

## 1. 使い方

### 1.1 読み順

1. **UR**（[01-ur](01-ur-user-requirements.md)）→ ユーザーが「やりたいこと」
2. **BR**（[02-br](02-br-business-requirements.md)）→ 業務上の要件
3. **SR**（[03-sr](03-sr-system-requirements.md)）→ システム化の要件
4. **FR**（[04-fr](04-fr-functional-requirements.md)）→ 機能要件（F-01〜F-17）
5. **NFR**（[05-nfr](05-nfr-non-functional-requirements.md)）→ 非機能要件
6. **Data/IF/Sec/Ops/Mig**（[06-10](06-data-requirements.md)）→ 専門分野別要件

### 1.2 トレーサビリティ

```
UR-NN → BR-NN → SR-NN → FR-NN (F-ID) → M-NN §3 / API-NN
              ↓
         NFR-NN → 横断文書（[DOC-ARCH-NNN](../architecture/)）
```

各 F-ID は全モジュール文書 §1「需求来源」で参照される。

---

## 2. ドキュメント一覧と IPA 工程マッピング

| DOC-ID | ファイル | タイトル | IPA 工程 | NF 区分 |
|---|---|---|---|---|
| DOC-REQ-UR-001 | [01-ur-user-requirements.md](01-ur-user-requirements.md) | ユーザー要求定義書 | 10 | — |
| DOC-REQ-BR-001 | [02-br-business-requirements.md](02-br-business-requirements.md) | 業務要件定義書 | 11 | — |
| DOC-REQ-SR-001 | [03-sr-system-requirements.md](03-sr-system-requirements.md) | システム要件定義書 | 12 | — |
| DOC-REQ-FR-001 | [04-fr-functional-requirements.md](04-fr-functional-requirements.md) | 機能要件定義書 | 13 | — |
| DOC-REQ-NFR-001 | [05-nfr-non-functional-requirements.md](05-nfr-non-functional-requirements.md) | 非機能要件定義書 | 14 | [NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV] 必須 |
| DOC-REQ-DATA-001 | [06-data-requirements.md](06-data-requirements.md) | データ要件定義書 | 15 | — |
| DOC-REQ-IF-001 | [07-external-if-requirements.md](07-external-if-requirements.md) | 外部 IF 要件定義書 | 16 | [NF-SEC] |
| DOC-REQ-SEC-001 | [08-security-requirements.md](08-security-requirements.md) | セキュリティ要件定義書 | 17 | [NF-SEC] 必須 |
| DOC-REQ-OPS-001 | [09-operation-requirements.md](09-operation-requirements.md) | 運用要件定義書 | 18 | [NF-OPS] |
| DOC-REQ-MIG-001 | [10-migration-requirements.md](10-migration-requirements.md) | 移行要件定義書 | 19 | [NF-MIG] |

---

## 3. 要件階層のトレーサビリティ

| 階層 | ドキュメント | ID 形式 | 件数目標 |
|---|---|---|---|
| 1. UR | [01-ur](01-ur-user-requirements.md) | UR-001〜 | 30 件 |
| 2. BR | [02-br](02-br-business-requirements.md) | BR-001〜 | 50 件 |
| 3. SR | [03-sr](03-sr-system-requirements.md) | SR-001〜 | 80 件 |
| 4. FR | [04-fr](04-fr-functional-requirements.md) | F-01〜F-17 | 17 大機能 |
| 5. NFR | [05-nfr](05-nfr-non-functional-requirements.md) | NFR-{区分}-{連番} | 6 区分 × 5-10 件 |

### 3.1 F-ID 対応表（[DOC-LEGACY-001](../legacy/requirements.md) §9 より）

| F-ID | 機能名 | 関連 BR/SR | 関連 NF | 関連 M-ID |
|---|---|---|---|---|
| F-01 | 无限画布エディタ | BR-005, SR-010 | NFR-PER-01 | M-12 |
| F-02 | データ取得アダプタ | BR-001, SR-001 | NFR-PER-02 | M-01 |
| F-03 | データ標準化 | BR-002, SR-002 | — | M-02 |
| F-04 | データフローエンジン | BR-003, SR-003 | NFR-PER-03 | M-03 |
| F-05 | オーケストレーション | BR-003, SR-004 | — | M-04 |
| F-06 | 制御フロー | BR-003, SR-005 | — | M-05 |
| F-07 | プラグイン SDK | BR-006, SR-006 | — | M-06 |
| F-08 | デバッグ | BR-004, SR-007 | — | M-07 |
| F-09 | 免インストール / 単一バイナリ | BR-008, SR-011 | NFR-ENV-01 | M-12, M-13 |
| F-10 | 標準化 JSON | BR-002, SR-002 | — | M-02 |
| F-11 | RBAC + 协作 | BR-009, SR-008 | NFR-SEC-01 | M-11 |
| F-12 | (将来) | — | — | — |
| F-13 | トリガー | BR-007, SR-009 | — | M-08 |
| F-14 | エクスポータ | BR-001, SR-012 | — | M-09 |
| F-15 | リアルタイム | BR-010, SR-013 | NFR-PER-04 | M-15 |
| F-16 | ストリーミング | BR-010, SR-014 | NFR-PER-05 | M-15 |
| F-17 | マルチテナント | BR-011, SR-015 | NFR-SEC-02 | M-10 |

## 4. 命名・ID 規約

- **UR-NNN**: ユーザー要求（自然言語）
- **BR-NNN**: 業務要件（業務上の機能・制約）
- **SR-NNN**: システム要件（システム化のための機能・制約）
- **F-NN**: 機能要件（実装機能）
- **NFR-{区分}-{NNN}**: 非機能要件
- **DATA-NN**: データ要件
- **IF-NN**: 外部 IF 要件
- **SEC-NN**: セキュリティ要件
- **OPS-NN**: 運用要件
- **MIG-NN**: 移行要件

## 5. 用語集

| 用語 | 説明 |
|---|---|
| UR | User Requirements（ユーザー要求） |
| BR | Business Requirements（業務要件） |
| SR | System Requirements（システム要件） |
| FR | Functional Requirements（機能要件） |
| NFR | Non-Functional Requirements（非機能要件） |
| トレーサビリティ | 要求の追跡可能性 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
2. IPA「非機能要求グレード2018」、2018 年 4 月
3. IEEE 830-1998「Recommended Practice for Software Requirements Specifications」
4. Ada プロジェクトチーム「[DOC-UP-006 To-Be 業務](../upstream/06-to-be-business.md)」、2026-08-20
5. Ada プロジェクトチーム「[DOC-UP-007 To-Be システム](../upstream/07-to-be-system.md)」、2026-08-20
6. Ada プロジェクトチーム「[DOC-LEGACY-001 要件定義書原本](../legacy/requirements.md)」、2026-08-18

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
