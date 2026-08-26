# P1-09 SharedArrayBuffer フォールバック検証 (QA-F01)

> **決議ID**: P1-09
> **関連决议**: UN-P1-09 (`docs/architecture/07-qa-register.md` §5.2)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/README.md`](../README.md)

---

## §1 背景 / 問題

QA 登録簿 §5.2 UN-P1-09: **QA-F01 SharedArrayBuffer フォールバック検証**、期限: **M-12 着手 -3 日**、Owner: **フロントエンド**。

[`crates/ada-canvas-ui/`](../../../crates/) (M-12) で並列処理高速化のため SharedArrayBuffer を使うが、COOP/COEP 隔離環境外（古いブラウザ、HTTP）でのフォールバックが未検証。

## §2 决策

**採用案**: **COOP/COEP 自動付与 + フォールバック to postMessage + 機能検出**

- 1st 試行: SharedArrayBuffer（SAB）有効化
- COOP/COEP: HTTP ヘッダで `Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy: require-corp`
- フォールバック: SAB 利用不可時は `postMessage` ベース or Web Worker
- 機能検出: `crossOriginIsolated === true` 確認

## §3 選択肢と評価

### Option A: COOP/COEP 自動付与 + postMessage フォールバック ⭐ 推奨

- **优点**: SAB 性能活用 + 互換性確保
- **缺点**: SAB 利用不可時の性能劣
- **リスク**: COOP/COEP で iframe 埋め込み問題
- **可逆性**: 中

### Option B: SAB 必須（SAB 非対応は未対応）

- **优点**: 実装最簡、コード 1 経路
- **缺点**: 古いブラウザ除外、SEO 影響
- **リスク**: ユーザ喪失
- **可逆性**: 高

### Option C: 常に postMessage（互換性最優先）

- **优点**: 完全互換
- **缺点**: SAB 比で性能劣
- **リスク**: 1000 ノード 30fps 未達 ([`p1-04`](./p1-04-1000节点30fps.md))
- **可逆性**: 高

### Option D: Web Worker + Atomics

- **优点**: SAB 類似性能
- **缺点**: コード複雑
- **リスク**: ブラウザ間差
- **可逆性**: 中

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| フロントエンド Lead | A, R | TBD 採用 or 外注 / M-12 着手 -3 日 |
| Dev (M-12) | C | Solo / 実装 + 検証 |
| 性能担当 | I | TBD / 性能レビュー |
| SRE | I | 外注 / ヘッダ設定確認 |

## §5 期限 / 触发条件

- **决策期限**: M-12 着手 -3 日
- **性能目標**:
  - SAB 有効時: 通常性能
  - SAB 無効時: postMessage で 1000 ノード 15fps 確保
- **ブラウザ対応**:
  - Chrome 120+ (SAB 自動)
  - Firefox 120+ (SAB 自動)
  - Safari 17+ (SAB 自動)
  - 旧ブラウザ: postMessage フォールバック
- **反映先**:
  - `crates/ada-canvas-ui/src/parallel.rs`
  - Nginx config `deploy/nginx/conf.d/ada.conf` (COOP/COEP)
  - `docs/architecture/07-qa-register.md` §3.3 QA-F01
  - ブラウザ互換表 `docs/architecture/browser-support.md`
- **再评估触发**:
  - フォールバック率 > 20% → 性能チューニング
  - 特定ブラウザで SAB 不可 → ドキュメント更新

## §6 影响範囲 / リスク

- **影响模块**:
  - `crates/ada-canvas-ui/` (M-12)
  - Nginx / CDN ヘッダ設定
  - 埋め込み iframe（外部システム）
- **リスク评估**:
  - iframe 埋め込み: COEP で他オリジン iframe ブロック
  - 認証 Cookie: COOP で別ウィンドウの参照不可
  - SEO: SAB 不要、影響なし
- **緩和策**:
  - iframe 埋め込み用エンドポイントは COEP なし
  - 機能検出 UI（「高速モード」/「互換モード」表示）
  - ブラウザ統計でフォールバック率監視

## §7 参考 / 関連 ADR

- 関連文档:
  - `docs/architecture/07-qa-register.md` §3.3 QA-F01
  - MDN - SharedArrayBuffer
  - MDN - Cross-Origin-Embedder-Policy
  - MDN - crossOriginIsolated
  - W3C - COOP/COEP 仕様

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
