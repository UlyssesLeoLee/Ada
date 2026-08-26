# P0-07 JWT 鍵ローテーション (kid + JWKS)

> **決議ID**: P0-07 (per DOC-DEC-001 矩阵 §1)
> **関連决议**: UN-P0-07 (`docs/architecture/07-qa-register.md` §5.1)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/01-p0-decision-matrix.md`](../01-p0-decision-matrix.md) §8

---

## §1 背景 / 問題

[`docs/requirements/05-nfr-non-functional-requirements.md` NF-SEC-03 認証](../../../requirements/05-nfr-non-functional-requirements.md) で JWT を使うが、**鍵ローテ方式** 未定。

QA 登録簿 §5.1 UN-P0-07 期限: **M-13 着手 -7 日**、Owner: **セキュリティ**。

OAuth 2.0 業界標準の `kid` クレーム + JWKS (JSON Web Key Set) エンドポイントで graceful rotation を実現する。

## §2 决策

**採用案**: **Option A (kid + JWKS、90 日ローテ + 7 日 grace period)**

```json
// JWT ヘッダー
{
  "alg": "RS256",
  "typ": "JWT",
  "kid": "key-2026-08-20"
}

// 公開鍵エンドポイント
GET /.well-known/jwks.json
{
  "keys": [
    {
      "kid": "key-2026-08-20",
      "kty": "RSA",
      "alg": "RS256",
      "use": "sig",
      "n": "...",
      "e": "AQAB"
    },
    {
      "kid": "key-2026-05-20",
      "...": "..."
    }
  ]
}
```

- **頻度**: 90 日毎
- **運用**: 旧鍵を 7 日間残す（猶予期間）
- **手順**: (1) 新鍵生成 (2) JWKS に追加 (3) 7 日後に旧鍵削除

## §3 選択肢と評価

### Option A: kid + JWKS ⭐ 推奨

- **优点**: 業界標準（OAuth 2.0 / OpenID Connect）、ダウンタイムなし、graceful rotation
- **缺点**: JWKS エンドポイント可用性必要、キャッシュ戦略必要
- **リスク**: JWKS 取得時のネット障害 → 検証失敗（ローカルキャッシュで緩和）
- **可逆性**: 高（grace period 延長可能）

### Option B: 単一鍵、定期交換

- **优点**: シンプル
- **缺点**: 交換時のダウンタイム必要、ロールバック不可
- **リスク**: 鍵交換時のサービス断
- **可逆性**: 低（ロールバック困難）

### Option C: 鍵交換プロトコル（KDE / IKE 風）

- **优点**: 自動鍵交渉
- **缺点**: JWT では未採用、複雑すぎる
- **リスク**: 仕様独自、保守困難
- **可逆性**: 高

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| セキュリティ (SecO) | A, R | 外注 / 2026-08-29 |
| アーキ | C | TBD / Day 2 |
| バックエンド Dev (M-13) | I | Solo / M-13 着手時 |
| PO | I | Ulysses / Day 2 |

## §5 期限 / 触发条件

- **决策期限**: 2026-08-29（Day 2）
- **反映先**:
  - `docs/requirements/05-nfr-non-functional-requirements.md` NF-SEC-03
  - `crates/ada-gateway/src/auth.rs` (M-13)
  - `.well-known/jwks.json` エンドポイント
- **鍵ローテーション cron**: 90 日毎
- **再评估触发**:
  - 鍵漏洩疑い → 即時ローテーション（grace 期間短縮）
  - 鍵サイズ < 2048bit 推奨時代遅れ → 3072bit へ
  - ポスト量子暗号対応 (PQC) → 評価

## §6 影响範囲 / リスク

- **影响模块**:
  - `crates/ada-gateway/` (M-13 API Gateway)
  - 全クライアント（Web フロント、モバイル、他サービス）
  - KMS ([`p0-06-`](./p0-06-KMS.md) と連動)
- **リスク评估**:
  - JWKS エンドポイント障害: クライアントが検証失敗 → ログイン不可
  - クライアント側の kid キャッシュ: 5 分 TTL で grace 期間中に旧鍵削除事故防止
  - 鍵サイズ: RS256 = 2048bit 標準、3072bit 推奨（性能 30% 劣化）
- **緩和策**:
  - JWKS エンドポイントを 3 ノード冗長化
  - クライアントは kid キャッシュ + 検証失敗時再取得
  - grace 期間は 7 日固定（旧鍵削除は手動承認）

## §7 参考 / 関連 ADR

- 関連文档:
  - `docs/decisions/01-p0-decision-matrix.md` §8 UN-P0-07
  - `docs/requirements/05-nfr-non-functional-requirements.md` NF-SEC-03
  - `docs/architecture/07-qa-register.md` §3.2 QA-S02
  - RFC 7517 - JSON Web Key (JWK)
  - RFC 7519 - JSON Web Token (JWT)
  - OpenID Connect Discovery 1.0

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
