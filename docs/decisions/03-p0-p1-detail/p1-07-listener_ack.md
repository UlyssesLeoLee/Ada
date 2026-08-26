# P1-07 listener 連続稼働時の ack 仕様 (QA-P08)

> **決議ID**: P1-07
> **関連决议**: UN-P1-07 (`docs/architecture/07-qa-register.md` §5.2)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/README.md`](../README.md)

---

## §1 背景 / 問題

QA 登録簿 §5.2 UN-P1-07: **QA-P08 listener 連続稼働時の ack 仕様**、期限: **M-15 着手 -3 日**、Owner: **信頼性 + SRE**。

[`crates/ada-event-bus/`](../../../crates/) (M-15) の listener プロセスが長時間稼働時の ack タイムアウト、再接続、メッセージ順序保証の仕様が未定義。

## §2 决策

**採用案**: **at-least-once + heartbeat 30s + ack timeout 60s + 指数バックオフ再接続**

- **配信保証**: at-least-once（重複許容、consumer 側冪等性）
- **heartbeat**: 30 秒毎（接続維持検出）
- **ack timeout**: 60 秒（処理時間 + バッファ）
- **再接続**: 指数バックオフ（1s → 30s max）
- **Dead Letter Queue (DLQ)**: 3 回失敗で DLQ 移送

## §3 選択肢と評価

### Option A: at-least-once + heartbeat 30s + ack 60s ⭐ 推奨

- **优点**: 業界標準（Kafka, RabbitMQ 互換）、DLQ で poison message 対応
- **缺点**: 重複処理可能性 → consumer 冪等性必須
- **リスク**: メッセージ重複、順序乱れ（パーティションキー）
- **可逆性**: 中

### Option B: at-most-once

- **优点**: シンプル、重複なし
- **缺点**: メッセージ喪失可能性
- **リスク**: 業務影響大
- **可逆性**: 低

### Option C: exactly-once

- **优点**: 重複なし、喪失なし
- **缺点**: 実装複雑、性能劣化、分散トランザクション必要
- **リスク**: 性能ボトルネック
- **可逆性**: 中

### Option D: 最長ACK + 同期処理

- **优点**: 強い整合性
- **缺点**: レイテンシ大、スケール難
- **リスク**: 1000 接続で破綻
- **可逆性**: 中

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| 信頼性エンジニア | A | TBD 採用 or 外注 / M-15 着手 -3 日 |
| SRE | R | 外注 / 同上 |
| Dev (M-15) | C | Solo / 実装 + テスト |
| アーキ | I | TBD / 設計レビュー |

## §5 期限 / 触发条件

- **决策期限**: M-15 着手 -3 日
- **性能目標**:
  - 接続断検知: < 60 秒
  - 再接続時間: < 5 秒
  - メッセージ重複率: < 0.01%
- **反映先**:
  - `crates/ada-event-bus/src/listener.rs`
  - `docs/architecture/07-qa-register.md` §3.3 QA-P08
  - 信頼性テスト `crates/ada-event-bus/tests/reconnect.rs`
  - 運用 Runbook `docs/operations/07-listener-recovery.md`
- **再评估触发**:
  - 重複率高 → exactly-once 再評価
  - 接続断検知遅い → heartbeat 短縮
  - DLQ 滞留 → 監視強化

## §6 影响範囲 / リスク

- **影响模块**:
  - `crates/ada-event-bus/` (M-15)
  - 全 consumer（plugins, audit_log 書込, 通知, etc.）
  - 監視・DLQ
- **リスク评估**:
  - ネットワーク分断: heartbeat タイムアウトで再接続
  - Consumer 遅延: ack timeout 60s で強制 unack → 再配信
  - Poison message: DLQ で停止防止
- **緩和策**:
  - Consumer 冪等性ハッシュ（`event_id` で重複検出）
  - DLQ 監視 + 月次レビュー
  - 接続断テスト自動化（Chaos Engineering）

## §7 参考 / 関連 ADR

- **D-ADR-07** (`02-design-adrs.md` §8): Event Bus 配信保証 - at-least-once + idempotent consumer
- 関連文档:
  - `docs/architecture/07-qa-register.md` §3.3 QA-P08
  - Kafka 公式ドキュメント - Delivery Semantics
  - RabbitMQ 公式ドキュメント - Acknowledgements

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
