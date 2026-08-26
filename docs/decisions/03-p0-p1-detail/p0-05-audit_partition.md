# P0-05 audit_log 月次 RANGE パーティション

> **決議ID**: P0-05 (per DOC-DEC-001 矩阵 §1)
> **関連决议**: UN-P0-05 (`docs/architecture/07-qa-register.md` §5.1)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/01-p0-decision-matrix.md`](../01-p0-decision-matrix.md) §6

---

## §1 背景 / 問題

[`docs/modules/M-10-tenant-middleware.md` §4.4](../../../modules/M-10-tenant-middleware.md) で audit_log を 1 年保存と決めたが、PostgreSQL の **テーブル肥大化** 対策（パーティション）が未実装。

QA 登録簿 §5.1 UN-P0-05 期限: **M-10 着手 -7 日**、Owner: **DBA**。

監査ログは追記のみ・日付キーで範囲検索が多い → **RANGE パーティション** が最適。

## §2 决策

**採用案**: **月次 RANGE パーティション + pg_partman 自動化 + ハッシュチェーン**

```sql
-- パーティション親テーブル
CREATE TABLE audit_log (
  id BIGSERIAL,
  tenant_id UUID NOT NULL,
  user_id UUID,
  action VARCHAR(64) NOT NULL,
  resource_type VARCHAR(64) NOT NULL,
  resource_id UUID,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  ip INET,
  user_agent TEXT,
  prev_hash BYTEA,        -- ハッシュチェーン用
  curr_hash BYTEA NOT NULL,
  occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);

-- 月次パーティション
CREATE TABLE audit_log_2026_08 PARTITION OF audit_log
  FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');

-- 1 年以上前は別テーブルへアーカイブ
-- audit_log_archive_y2025
```

## §3 選択肢と評価

### Option A: 月次 RANGE + pg_partman ⭐ 推奨

- **优点**: 性能維持（古いデータ DELETE 高速）、pg_partman で自動作成、アーカイブ容易
- **缺点**: パーティション数増で pg_catalog 肥大、PK にパーティションキー必須
- **リスク**: パーティション作成失敗時の対応（監視必須）
- **可逆性**: 中（親テーブル → 通常テーブル変換可能だがコスト高）

### Option B: 週次 RANGE

- **优点**: パーティション粒度細かい、検索局所化
- **缺点**: パーティション数 52/年 × N 年で肥大、pg_partman 設定複雑
- **リスク**: pg_catalog パフォーマンス劣化
- **可逆性**: 中

### Option C: パーティションなし（単一テーブル + 月次 DELETE）

- **优点**: 実装最簡
- **缺点**: 大量 DELETE でテーブル肥大（VACUUM 効かない）、性能劣化
- **リスク**: 1 年後に性能破綻
- **可逆性**: 高

### Option D: TimescaleDB ハイパーテーブル

- **优点**: 自動圧縮、長期保存に最適
- **缺点**: 拡張依存、PostgreSQL ネイティブではない、運用知見必要
- **リスク**: ベンダ依存、移行コスト
- **可逆性**: 低

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| DBA | A, R | Ulysses (DBA 兼任) / 2026-08-29 |
| SRE | C | 外注 / M-10 着手 -7 日 |
| バックエンド Dev | I | Solo / M-10 着手時 |
| PO | I | Ulysses / Day 2 |

## §5 期限 / 触发条件

- **决策期限**: 2026-08-29（Day 2）
- **反映先**:
  - `docs/modules/M-10-tenant-middleware.md` §4.4
  - `migrations/0008_audit_log_partition.sql`
  - `crates/ada-tenant-middleware/migrations/` に組み込み
- **pg_partman cron 設定**: 月次実行（毎日 00:00 確認）
- **再评估触发**:
  - パーティション数 > 50 で pg_catalog 遅延 → 年次 RANGE 検討
  - 監査ログ検索性能 > 1s → インデックス追加

## §6 影响範囲 / リスク

- **影响模块**:
  - `audit_log` テーブル（全モジュールが書き込み）
  - GDPR 削除フロー（[`p0-08-`](./p0-08-GDPR.md) と連動）
  - バックアップ戦略（[`p0-10-`](./p0-10-Backup.md) と連動）
- **リスク评估**:
  - PK 変更: `(id, occurred_at)` 複合 PK への書き換え必要
  - 既存 INSERT コード: パーティションキー自動付与、影響なし
  - pg_partman cron 失敗: パーティション未作成 → INSERT 失敗
- **緩和策**:
  - cron 監視 + 失敗時アラート（P1 緊急対応）
  - パーティション作成 cron を 2 重化（master + standby）
  - 3 ヶ月前パーティション事前作成バッファ

## §7 参考 / 関連 ADR

- 関連文档:
  - `docs/decisions/01-p0-decision-matrix.md` §6 UN-P0-05
  - `docs/modules/M-10-tenant-middleware.md` §4.4
  - `docs/architecture/07-qa-register.md` §3.1 QA-D08
  - PostgreSQL 18.6 Documentation - PARTITION BY RANGE
  - pg_partman 5.x Documentation

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
