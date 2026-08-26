# P0-11 ADR レビュー会 (週次アーキ会議)

> **決議ID**: P0-11 (per DOC-DEC-001 矩阵 §1)
> **関連决议**: UN-P0-11 (`docs/architecture/07-qa-register.md` §5.1)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/01-p0-decision-matrix.md`](../01-p0-decision-matrix.md) §12

---

## §1 背景 / 問題

[`docs/architecture/06-rust-tech-selection.md` §10](../../../architecture/06-rust-tech-selection.md) で 10 ADR を提示したが、**正式レビュー会** 未開催。

QA 登録簿 §5.1 UN-P0-11 期限: **実装着手 -1 日**、Owner: **マネージャ + アーキ**。

G4 实施着手判定の最終チェックとして、保留中 ADR の GO/NO-GO 判定が必要。

## §2 决策

**採用案**: **週次 30 分 / テックリード主催 / 既存アーキ会議と統合**

| 項目 | 内容 |
|---|---|
| 頻度 | 週次 30 分（[`docs/management/04-communication-plan.md` §1 アーキ会議](../../../management/04-communication-plan.md) と同時） |
| 主催 | テックリード |
| 参加者 | アーキ、テック、Dev 代表、PO、SecO（必要時） |
| 議題 | 保留中 ADR の GO/NO-GO |
| 議事録 | [`docs/templates/03-process-management.md` §A.5](../../../templates/03-process-management.md) で記録 |
| 決定 | 3 名以上の合議、過半数 |

## §3 選択肢と評価

### Option A: 週次 30 分 / 既存会議と統合 ⭐ 推奨

- **优点**: 追加コスト 0、既存会議体活用、合議制で品質確保
- **缺点**: 30 分で深い議論は困難、議題過多時は別会議
- **リスク**: 週次リズムが崩れる（リマインダー設定で緩和）
- **可逆性**: 高（頻度変更容易）

### Option B: 隔週 60 分

- **优点**: 深い議論可能
- **缺点**: 保留 ADR 滞留、決定遅延
- **リスク**: G4 判定前に未決定 ADR 残る
- **可逆性**: 高

### Option C: ADR 毎に臨時会議

- **优点**: 集中議論
- **缺点**: 会議コスト高、調整困難
- **リスク**: PO/SecO スケジュール調整不能
- **可逆性**: 高

### Option D: PO 単独判断（合議なし）

- **优点**: 意思決定最速
- **缺点**: 単一視点、品質ばらつき
- **リスク**: 属人化、後で見直し多発
- **可逆性**: 高

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| マネージャ / PO | A | Ulysses (PO 兼任) / 2026-08-27 |
| アーキ / テックリード | R | TBD 採用 or 外注 / Day 1 |
| Dev 代表 | C | Solo / 週次参加 |
| SecO | C | 外注 / 必要時のみ |
| 全参加者 | I | — / 議事録配布 |

## §5 期限 / 触发条件

- **决策期限**: 2026-08-27（Day 1、即時）
- **初回開催**: 2026-09-01 目標
- **反映先**:
  - `docs/management/04-communication-plan.md` §1
  - ADR テンプレ `docs/templates/03-process-management.md` §A.5
  - 議事録保管: `docs/management/meeting-notes/adr-review/`
- **再评估触发**:
  - 30 分で消化できない議題頻発 → 60 分化
  - 隔週リズム要望 → 隔週 60 分
  - 緊急 ADR 発生 → 臨時開催

## §6 影響範囲 / リスク

- **影响模块**:
  - 保留中 ADR: ADR-07 (Bevy 0.14/0.15), ADR-08 (CRDT 库), ADR-09 (WASM vs 进程沙箱), ADR-10 (License)
  - 関連: [`02-design-adrs.md`](../02-design-adrs.md) §3 D-01 (CRDT) で Yrs 採用済、§3 D-02 (沙箱) で WASM 採用済
- **リスク评估**:
  - 参加者揃わない: 非同期 ADR レビュー導入（GitHub PR ベース）
  - 議事録未記録: テンプレ強制、議事録なし = 会議未開催扱い
  - 決定保留長期化: 60 日ルール（保留 60 日で PO 単独決定）
- **緩和策**:
  - カレンダー リマインダー（金曜 16:00 30 分）
  - 議事録テンプレ + 自動採番
  - ADR ステータス board（GitHub Project）

## §7 参考 / 関連 ADR

- 関連文档:
  - `docs/decisions/01-p0-decision-matrix.md` §12 UN-P0-11
  - `docs/decisions/02-design-adrs.md` (D-01〜D-15 既決定)
  - `docs/architecture/06-rust-tech-selection.md` §10 (10 ADR)
  - `docs/architecture/07-qa-register.md` §3.4 QA-G04
  - `docs/management/04-communication-plan.md` §1
  - `docs/templates/03-process-management.md` §A.5

**保留中 ADR（初会議で GO/NO-GO 判定）**:

| ADR# | 主题 | 状態 | 連動 |
|---|---|---|---|
| ADR-07 | Bevy 0.14 vs 0.15 | 🟡 保留 | — |
| ADR-08 | CRDT 库選型 | 🟡 保留 | D-01 (Yrs) で解決済の可能性 |
| ADR-09 | プラグイン沙箱 (WASM vs 进程) | 🟡 保留 | D-02 (WASM) で解決済の可能性 |
| ADR-10 | License 選定 | 🟡 保留 | D-13 (MIT) で解決済の可能性 |

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
