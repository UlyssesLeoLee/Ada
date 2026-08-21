# 意思決定ドキュメント（Decision Documents）

> **本ディレクトリの目的**：[DOC-ARCH-008 QA 登録簿 §5 P0](../architecture/07-qa-register.md) の **11 件の重要未決事項** と、設計段階の **15 件の設計詳細未確定** を体系的に整理し、**意思決定者（PO / テックリード / SecO / DBA / SRE）が短時間で判断できる**ようにする。  
> 各決定事項について (a) 選択肢、(b) 評価、(c) 推奨案、(d) 決定者、(e) 期限 を明示する。

> **ドキュメントID**：DOC-DEC-INDEX
> **文書分類**：意思決定文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：PO
> **上位文書**：
> - [`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)（DOC-ARCH-009）
> - [`docs/architecture/07-qa-register.md`](../architecture/07-qa-register.md)（DOC-ARCH-008）
> - [`docs/architecture/06-rust-tech-selection.md`](../architecture/06-rust-tech-selection.md)（DOC-ARCH-007）
> **下位文書**：
> - [`docs/decisions/01-p0-decision-matrix.md`](01-p0-decision-matrix.md)（DOC-DEC-001）— 11 P0 决策矩阵
> - [`docs/decisions/02-design-adrs.md`](02-design-adrs.md)（DOC-DEC-002）— D-01〜15 设计 ADR
> **関連文書**：
> - [`docs/upstream/08-initial-risk-assessment.md`](../upstream/08-initial-risk-assessment.md)
> - 全 [DOC-MOD-NNN §3](../modules/M-01-acquisition-adapter.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - PMBOK Guide 第 7 版
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（11 P0 + 15 D-ADR） | Ada プロジェクトチーム | TBD | PO |

---

## 目次

1. 使い方
2. 意思決定フロー
3. ドキュメント一覧
4. 推奨決定タイムライン
5. 用語集
6. 参考文献

---

## 1. 使い方

### 1.1 いつ参照するか

| シーン | 参照先 |
|---|---|
| **実装着手判定 G4 前** | [§A P0 决策矩阵](01-p0-decision-matrix.md) を 11 件全消化 |
| **設計レビュー中** | [§B D-ADR](02-design-adrs.md) で既決定事項を確認 |
| **新規開発者** | §2 意思決定フローと §4 推奨タイムラインで全体像把握 |

### 1.2 决定记录方法

各 P0 / D-ADR について：
1. 選択肢を読む（各項目 5 分以内）
2. 評価を見る（10 分）
3. 推奨案を参考に決定（5 分）
4. 决定の根拠を「决定」列に記述（5 分）
5. PO 承認を得る

合計：1 P0 あたり 30 分、11 P0 で **5.5 時間**（1 営業日以内）。

---

## 2. 意思決定フロー

```
[DOC-ARCH-008 §5 P0] 
       ↓
[意思決定者割当] ← 推奨案付き選択肢提示
       ↓
[PO レビュー] ← 30 分 / P0
       ↓
[決定 + 根拠記録] ← DOC-DEC-001 / DOC-DEC-002
       ↓
[関連ドキュメント更新] ← DOC-ARCH-008 §5 状態更新
       ↓
[G4 实施着手判定 通過]
       ↓
[実装開始]
```

## 3. ドキュメント一覧

| DOC-ID | ファイル | 内容 | 対象 |
|---|---|---|---|
| DOC-DEC-INDEX | [README.md](README.md) | 本索引 | — |
| DOC-DEC-001 | [01-p0-decision-matrix.md](01-p0-decision-matrix.md) | 11 P0 决策矩阵 + 推奨案 | PO + 全意思決定者 |
| DOC-DEC-002 | [02-design-adrs.md](02-design-adrs.md) | D-01〜15 設計 ADR + 解決 | テックリード + アーキ |

## 4. 推奨決定タイムライン

| 日 | 决定 | 担当 |
|---|---|---|
| Day 1 AM | UN-P0-01（人員） | PO + PM |
| Day 1 PM | UN-P0-02（組織） | PO |
| Day 1 PM | UN-P0-11（ADR レビュー） | テックリード |
| Day 2 AM | UN-P0-03（FK） | DBA |
| Day 2 AM | UN-P0-04（Manifest Schema） | アーキ |
| Day 2 AM | UN-P0-05（audit_log パーティション） | DBA |
| Day 2 PM | UN-P0-06（KMS） | SecO |
| Day 2 PM | UN-P0-07（JWT 鍵ローテーション） | SecO |
| Day 3 AM | UN-P0-08（GDPR フロー） | PO + SecO |
| Day 3 AM | UN-P0-09（ログ基盤） | SRE |
| Day 3 PM | UN-P0-10（Backup） | DBA + SRE |
| Day 4 | G4 判定会議 | 全員 |
| Day 5+ | 実装開始 | Dev × 16 |

**合計**：11 P0 消化 = 3 営業日、G4 通過 = 4 営業日。

## 5. 用語集

| 用語 | 説明 |
|---|---|
| P0 | Priority 0（最優先、必須） |
| ADR | Architecture Decision Record |
| 推奨案 | 最も一般的なベストプラクティス |
| 決定者 | 最終決定権を持つロール |

## 6. 参考文献

1. PMBOK Guide 第 7 版、Project Management Institute、2021 年
2. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
3. Ada プロジェクトチーム「[DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
