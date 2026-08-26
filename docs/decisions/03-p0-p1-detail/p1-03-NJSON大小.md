# P1-03 NJSON サイズポリシー (QA-D03)

> **決議ID**: P1-03
> **関連决议**: UN-P1-03 (`docs/architecture/07-qa-register.md` §5.2)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/README.md`](../README.md)

---

## §1 背景 / 問題

QA 登録簿 §5.2 UN-P1-03: **QA-D03 NJSON サイズポリシー**、期限: **M-01 着手 -3 日**、Owner: **アーキ**。

[`docs/architecture/04-atomic-deployment.md`](../../../architecture/04-atomic-deployment.md) の `canvas_node.config` 等で JSONB (NJSON) を使用しているが、最大サイズ・圧縮・分割方針が未定義。

## §2 决策

**採用案**: **JSONB 上限 64 KB + gzip 圧縮 + 超過時は外部ストレージ参照**

- `canvas_node.config`: 64 KB 上限（DB 性能保護）
- 超過時: 別カラム `config_blob BYTEA` (gzip 圧縮) または `external_ref UUID` (S3 参照)
- バリデーション: PL/pgSQL トリガで size チェック

## §3 選択肢と評価

### Option A: 64 KB + gzip + 外部参照 ⭐ 推奨

- **优点**: 性能保護 + 柔軟性、TOAST 自動活用
- **缺点**: アプリ層で分岐必要、外部参照管理
- **リスク**: 64 KB 超過時の運用判断
- **可逆性**: 中

### Option B: 上限なし（PostgreSQL JSONB 任せ）

- **优点**: シンプル
- **缺点**: 1MB 超で性能劣化、TOAST 頻発
- **リスク**: 性能ボトルネック
- **可逆性**: 高

### Option C: 16 KB 厳格上限

- **优点**: DB 性能保護強い
- **缺点**: ユースケース制約大、設定ファイル記述で即超過
- **リスク**: ユーザビリティ低下
- **可逆性**: 中

### Option D: 外部ストレージのみ (S3 + 参照 ID)

- **优点**: DB 軽量化、無限サイズ対応
- **缺点**: JOIN 不能、トランザクション整合性複雑
- **リスク**: S3 障害時の可用性
- **可逆性**: 低

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| アーキテクト | A, R | Ulysses (アーキ兼任) / M-01 着手 -3 日 |
| DBA | C | Ulysses (DBA 兼任) / 設計レビュー |
| バックエンド Dev | I | Solo / M-01 着手時 |

## §5 期限 / 触发条件

- **决策期限**: M-01 着手 -3 日
- **反映先**:
  - `docs/architecture/04-atomic-deployment.md` §X (NJSON ポリシー)
  - `migrations/0010_njson_size_limit.sql` (CHECK 制約 + トリガ)
  - `crates/ada-tenant-middleware/src/jsonb.rs` ヘルパー
- **再评估触发**:
  - 64 KB でユースケース 30% 超過 → 256 KB へ
  - 外部参照 S3 コスト高 → gzip 単独強化

## §6 影响範囲 / リスク

- **影响模块**:
  - `canvas_node.config` 等の全 JSONB カラム
  - アダプタ層（M-01）での JSON ハンドリング
  - バックアップサイズ（[`p0-10-`](./p0-10-Backup.md) と連動）
- **リスク评估**:
  - 超過時のデータ移行: 自動マイグレーションスクリプト必要
  - アプリ層判定ミス: バリデーション層で統一
  - TOAST 増大: モニタリング
- **緩和策**:
  - CHECK 制約で 64 KB 強制（DB 層）
  - アプリ層でも事前バリデーション
  - 月次で JSONB サイズ統計レポート

## §7 参考 / 関連 ADR

- 関連文档:
  - `docs/decisions/01-p0-decision-matrix.md` §5 (P0-04 Manifest)
  - `docs/architecture/07-qa-register.md` §3.1 QA-D03
  - PostgreSQL 18.6 Documentation - JSONB, TOAST

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
