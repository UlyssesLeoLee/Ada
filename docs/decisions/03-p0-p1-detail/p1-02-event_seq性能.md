# P1-02 event_seq 性能検証 (QA-D02)

> **決議ID**: P1-02
> **関連决议**: UN-P1-02 (`docs/architecture/07-qa-register.md` §5.2)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/README.md`](../README.md)

---

## §1 背景 / 問題

QA 登録簿 §5.2 UN-P1-02: **QA-D02 event_seq 性能検証**、期限: **M-15 着手 -7 日**、Owner: **性能 + DB**。

[`crates/ada-event-bus/`](../../../crates/) (M-15) の event_seq (sequence number) 生成・付与のスループット・レイテンシが未検証。大規模イベント処理でホットスポット化する懸念。

## §2 决策

**採用案**: **PostgreSQL `SEQUENCE` + Snowflake 風ハイブリッド + 性能ベンチ公開**

- 単調増加性: PostgreSQL `SEQUENCE` トランザクション内取得
- スケール: 高頻度経路は Snowflake 風 (時間 + worker_id + seq) で DB 負荷回避
- ベンチマーク: 10K events/s 持続、1M events burst レイテンシ < 100ms を目標

## §3 選択肢と評価

### Option A: PostgreSQL SEQUENCE + Snowflake ハイブリッド ⭐ 推奨

- **优点**: 単調増加性保証 + スケール時 DB 負荷回避
- **缺点**: 実装複雑、2 経路の整合性テスト
- **リスク**: ハイブリッド切替時のギャップ
- **可逆性**: 中

### Option B: PostgreSQL SEQUENCE のみ

- **优点**: シンプル、強い単調増加性
- **缺点**: DB 接続が event 数のボトルネック
- **リスク**: 高頻度時 DB 過負荷
- **可逆性**: 高

### Option C: Snowflake ID のみ

- **优点**: DB 負荷最小、水平スケール容易
- **缺点**: 単調増加性は秒単位まで、厳密な順序保証なし
- **リスク**: 順序依存処理で問題
- **可逆性**: 中

### Option D: UUID v7

- **优点**: 標準化、衝突確率極小
- **缺点**: 順序性の弱さ、PostgreSQL 16+ ネイティブ対応のみ
- **リスク**: バージョン依存
- **可逆性**: 高

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| 性能担当 | A | TBD 採用 or 外注 / M-15 着手 -7 日 |
| DBA | R | Ulysses (DBA 兼任) / M-15 着手 -7 日 |
| アーキ | C | TBD / ベンチ結果レビュー |
| Dev (M-15) | I | Solo / M-15 着手時 |

## §5 期限 / 触发条件

- **决策期限**: M-15 着手 -7 日
- **性能目標**:
  - 持続スループット: 10K events/s
  - バースト: 1M events < 100ms p99
  - DB 接続数: < 50
- **反映先**:
  - `docs/architecture/07-qa-register.md` §3.1 QA-D02
  - `crates/ada-event-bus/src/seq.rs` 実装
  - `benches/event_seq.rs` (Criterion)
- **再评估触发**:
  - 10K/s 未達 → Snowflake 単独へ
  - 単調増加性要求 → Option B へ

## §6 影响範囲 / リスク

- **影响模块**:
  - `crates/ada-event-bus/` (M-15)
  - イベント購読側 (M-12 フロント同期)
  - audit_log 書き込み（[`p0-05-`](./p0-05-audit_partition.md) と連動）
- **リスク评估**:
  - DB 接続枯渇: 接続プール + 取得バッチ化
  - 順序乱れ: ハイブリッド切替時にテスト必須
  - ベンチ不正: 実環境サイズで検証
- **緩和策**:
  - ベンチ結果を `docs/architecture/bench-results/` に保存
  - 週次で再実行（リグレッション検出）
  - ハイブリッドモード切替は feature flag

## §7 参考 / 関連 ADR

- **D-ADR-07** (`02-design-adrs.md` §8): Event Bus 配信保証 - at-least-once
- 関連文档:
  - `docs/architecture/07-qa-register.md` §3.1 QA-D02
  - `docs/architecture/08-workflow-overview.md`
  - Snowflake ID 仕様 (Twitter)

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
