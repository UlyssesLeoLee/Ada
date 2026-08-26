# P1-10 WASM bundle サイズ目標 (QA-F03)

> **決議ID**: P1-10
> **関連决议**: UN-P1-10 (`docs/architecture/07-qa-register.md` §5.2)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/README.md`](../README.md)

---

## §1 背景 / 問題

QA 登録簿 §5.2 UN-P1-10: **QA-F03 WASM bundle サイズ目標設定**、期限: **M-12 着手 -3 日**、Owner: **フロントエンド**。

[`crates/ada-canvas-ui/`](../../../crates/) (M-12) の Bevy → WASM bundle サイズの目標値と計測・削減戦略が未確定。

## §2 决策

**採用案**: **gzip 後 3 MB / wasm-opt -O3 + LTO + dead code 除去 + CI 計測**

- **目標**: < 8 MB 生, < 3 MB gzip
- **最適化**: `wasm-opt -O3` + LTO + `opt-level = "z"` + dead code elimination
- **計測**: CI でビルド毎にサイズ取得 + 履歴グラフ
- **超過時**: アラート + 自動 PR コメント

## §3 選択肢と評価

### Option A: 8 MB / 3 MB gzip + wasm-opt -O3 ⭐ 推奨

- **优点**: 現実的目標、LTE/3G でも 30 秒以内ロード
- **缺点**: 機能追加で超過リスク
- **リスク**: 機能追加毎の監視必要
- **可逆性**: 中

### Option B: 1 MB 厳格目標

- **优点**: 最速ロード
- **缺点**: 機能制約大、Bevy 単体で > 1MB
- **リスク**: 機能不足
- **可逆性**: 高

### Option C: 16 MB 緩い目標

- **优点**: 機能制約なし
- **缺点**: ロード時間大、SEO/UX 影響
- **リスク**: モバイル離脱
- **可逆性**: 高

### Option D: 動的ロード（コード分割）

- **优点**: 初回ロード最小
- **缺点**: 実装複雑、wasm-split エコシステム未成熟
- **リスク**: 動的ロード失敗時の UX
- **可逆性**: 中

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| フロントエンド Lead | A, R | TBD 採用 or 外注 / M-12 着手 -3 日 |
| Dev (M-12) | R | Solo / ビルド設定 + 計測 |
| アーキ | C | TBD / 設計レビュー |
| SRE | I | 外注 / CDN キャッシュ設定 |

## §5 期限 / 触发条件

- **决策期限**: M-12 着手 -3 日
- **計測**:
  - ビルド毎に `twiggy top` でサイズ内訳
  - CI で 3 MB gzip 超過時 PR コメント
  - 月次でサイズ推移グラフ
- **反映先**:
  - `crates/ada-canvas-ui/Cargo.toml` (release profile)
  - `.cargo/config.toml` (`wasm-opt` 設定)
  - `scripts/measure-wasm-size.sh`
  - `docs/architecture/07-qa-register.md` §3.3 QA-F03
  - `docs/architecture/wasm-bundle-budget.md` (運用ガイド)
- **再评估触发**:
  - 連続 3 ビルドで 3 MB 超過 → 機能削減レビュー
  - ロード時間 p95 > 5s → CDN 強化

## §6 影响範囲 / リスク

- **影响模块**:
  - `crates/ada-canvas-ui/` (M-12)
  - Bevy プラグイン全部
  - CDN / キャッシュ戦略
- **リスク评估**:
  - 機能追加で膨張: 厳格な PR レビュー
  - 圧縮効率: gzip → brotli 評価
  - キャッシュヒット率: 長期キャッシュ + content hash
- **緩和策**:
  - CI 自動計測 + Slack 通知
  - 1 機能追加 = サイズ影響 PR コメント義務
  - 機能別クレート分割（[`p1-01`](./p1-01-模块边界.md) と連動）

## §7 参考 / 関連 ADR

- **D-ADR-05** (`02-design-adrs.md` §6): WASM Bundle Size < 8 MB (gzip 後 < 3 MB)
- **D-ADR-04** (`02-design-adrs.md` §5): Bevy バージョン 0.14
- 関連文档:
  - `docs/decisions/02-design-adrs.md` §6 D-05
  - `docs/architecture/07-qa-register.md` §3.3 QA-F03
  - wasm-opt 公式ドキュメント
  - twiggy (Rust WASM size profiler)

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
