# P0-08 忘れられる権利対応フロー (GDPR Art.17 / PIPL §47)

> **決議ID**: P0-08 (per DOC-DEC-001 矩阵 §1)
> **関連决议**: UN-P0-08 (`docs/architecture/07-qa-register.md` §5.1)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/01-p0-decision-matrix.md`](../01-p0-decision-matrix.md) §9

---

## §1 背景 / 問題

[GDPR Art.17 / PIPL §47](../../../requirements/08-security-requirements.md) で本人削除要求対応が必要だが、**運用フロー** 未定義。

QA 登録簿 §5.1 UN-P0-08 期限: **規制適用前に**、Owner: **法務 + コンプラ**。

SLA 30 日以内、業務データ削除 + 監査ログ保持（トレース用）を両立する必要がある。

## §2 决策

**採用案**: **30 日 SLA + 業務データ削除 + 監査ログ匿名化 + 削除ログ記録**

```sql
CREATE OR REPLACE PROCEDURE forget_user(target_user_id UUID)
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
  -- 1. 業務データ匿名化
  UPDATE canvas SET owner_id = NULL, owner_name = '[REDACTED]'
    WHERE owner_id = target_user_id;
  UPDATE canvas_node SET config = jsonb_set(config, '{created_by}', '"[REDACTED]"'::jsonb)
    WHERE config->>'created_by' = target_user_id::text;

  -- 2. 監査ログ匿名化（保持はする）
  UPDATE audit_log SET user_id = NULL, ip = NULL, user_agent = '[REDACTED]'
    WHERE user_id = target_user_id;

  -- 3. ユーザー本体削除
  DELETE FROM users WHERE id = target_user_id;

  -- 4. 削除ログ
  INSERT INTO gdpr_erasure_log (user_id, requested_at, completed_at)
    VALUES (target_user_id, NOW(), NOW());
END;
$$;
```

## §3 選択肢と評価

### Option A: 30 日 SLA + 業務削除 + 監査匿名化 ⭐ 推奨

- **优点**: GDPR Art.12/17 準拠、PIPL §47 準拠、トレーサビリティ確保
- **缺点**: 業務データと監査ログの二重管理、削除プロシージャのテスト必須
- **リスク**: 匿名化漏れの PII 残存、削除対象の見落とし
- **可逆性**: 低（削除後復元不可）

### Option B: 即時完全削除（全データ + 監査ログ）

- **优点**: シンプル、コンプラ完全準拠
- **缺点**: 監査トレース完全喪失、不正調査不能
- **リスク**: フォレンジック不能、内部監査要求違反
- **可逆性**: 低

### Option C: 30 日 SLA + ハード削除 + 監査は別テーブルで別途保持

- **优点**: データ完全削除、監査独立性
- **缺点**: 実装複雑、二重管理コスト
- **リスク**: 監査ログの PII 漏れ（別テーブル側で）
- **可逆性**: 低

### Option D: 削除要求を 90 日で対応（SLA 延長）

- **优点**: 運用余裕
- **缺点**: GDPR 違反（Art.12 で 30 日以内必須）
- **リスク**: 規制違反、制裁金
- **可逆性**: 高（ポリシー変更）

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| PO (Product Owner) | A | Ulysses / 2026-08-30 |
| セキュリティ (SecO) | R | 外注 / Day 3 |
| 法務 / コンプラ | C | 外注 / 規制適用前 |
| DBA | C | Ulysses (DBA 兼任) / Day 3 |
| サポート | I | 採用 or 外注 / サポート開始時 |
| ユーザ | I | — |

## §5 期限 / 触发条件

- **决策期限**: 2026-08-30（Day 3）
- **規制適用**: GDPR（EU ユーザー対応時即時）、PIPL（中国展開時即時）
- **反映先**:
  - `docs/requirements/08-security-requirements.md` §4
  - `migrations/0009_gdpr_forget.sql`
  - `crates/ada-tenant-middleware/src/gdpr.rs`
  - サポート手順書 `docs/operations/05-gdpr-handling.md` (新規)
- **再评估触发**:
  - 削除要求 > 月 10 件 → 自動化バッチ追加
  - 監査ログ匿名化漏れ検出 → プロシージャ見直し
  - 法規制変更（CCPA 等）→ 対応範囲拡大

## §6 影响範囲 / リスク

- **影响模块**:
  - `users`, `canvas`, `canvas_node`, `audit_log` テーブル
  - サポート業務フロー（メール → 確認 → 削除）
  - UI（削除リクエスト画面）
  - バックアップ戦略（[`p0-10-`](./p0-10-Backup.md) と連動）
- **リスク评估**:
  - 削除対象の見落とし: 業務横断検索ユーティリティ必要
  - 匿名化漏れ: 文字列フィールド全件スキャン
  - バックアップ内の PII: 次回 Backup サイクルで自動消滅（30 日以内）
- **緩和策**:
  - `gdpr_erasure_log` テーブルで全削除履歴記録
  - 月次で PII スキャン監査
  - 削除前 dry-run（`forget_user_dryrun()` 関数）提供
  - 本人確認はメール + 2FA + 30 日猶予

## §7 参考 / 関連 ADR

- **D-ADR-12** (`02-design-adrs.md` §13): PL/pgSQL 開発者 - DBA 兼任
- 関連文档:
  - `docs/decisions/01-p0-decision-matrix.md` §9 UN-P0-08
  - `docs/requirements/08-security-requirements.md` §4
  - `docs/architecture/07-qa-register.md` §3.2 QA-S08
  - GDPR Art.12 (1 month), Art.17 (Right to erasure)
  - PIPL §47 (中国個人情報保護法)

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
