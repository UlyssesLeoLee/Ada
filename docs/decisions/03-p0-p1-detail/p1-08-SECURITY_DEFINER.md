# P1-08 SECURITY DEFINER オーナー (QA-S03)

> **決議ID**: P1-08
> **関連决议**: UN-P1-08 (`docs/architecture/07-qa-register.md` §5.2)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/README.md`](../README.md)

---

## §1 背景 / 問題

QA 登録簿 §5.2 UN-P1-08: **QA-S03 SECURITY DEFINER オーナー**、期限: **M-10 着手 -3 日**、Owner: **DBA + セキュリティ**。

[`crates/ada-tenant-middleware/`](../../../crates/) (M-10) の PL/pgSQL 関数で `SECURITY DEFINER` 使用時のオーナー設定が未確定。`search_path` 固定と最小権限の原則適用が必要。

## §2 决策

**採用案**: **専用ロール `ada_func` 作成 + `SET search_path = pg_catalog, pg_temp` 強制 + REVOKE デフォルト**

```sql
-- 専用ロール作成
CREATE ROLE ada_func NOLOGIN;

-- 関数定義（推奨パターン）
CREATE OR REPLACE FUNCTION public.forget_user(target_user_id UUID)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp  -- 必須
AS $$
DECLARE
  v_caller uuid := current_setting('ada.current_user_id')::uuid;
BEGIN
  -- 権限チェック
  IF NOT EXISTS (
    SELECT 1 FROM admin_user
    WHERE user_id = v_caller AND role IN ('super_admin', 'compliance')
  ) THEN
    RAISE EXCEPTION 'Insufficient privilege';
  END IF;
  -- 処理
END;
$$;

ALTER FUNCTION public.forget_user(UUID) OWNER TO ada_func;
REVOKE ALL ON FUNCTION public.forget_user(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.forget_user(UUID) TO ada_app_role;
```

## §3 選択肢と評価

### Option A: 専用ロール + search_path 固定 + REVOKE ⭐ 推奨

- **优点**: PostgreSQL 標準、PostgreSQL 14+ で `SET search_path` 強制可能、最小権限
- **缺点**: 関数ごとに GRANT/REVOKE 管理
- **リスク**: 漏れ（PUBLIC デフォルト）
- **可逆性**: 中

### Option B: SECURITY INVOKER (呼び出し元権限)

- **优点**: 権限管理シンプル、SQL インジェクション影響局所化
- **缺点**: RLS ポリシー必須、関数で elevated 操作不可
- **リスク**: 用途制限
- **可逆性**: 中

### Option C: superuser 実行

- **优点**: 権限管理不要
- **缺点**: 最小権限原則違反、監査不可
- **リスク**: セキュリティ事故時の被害甚大
- **可逆性**: 低

### Option D: 個別 GRANT 細分化

- **优点**: 細粒度制御
- **缺点**: 管理複雑、運用ミス
- **リスク**: 設定漏れ
- **可逆性**: 低

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| DBA | A | Ulysses (DBA 兼任) / M-10 着手 -3 日 |
| セキュリティ | R | 外注 / 同上 |
| バックエンド Dev | I | Solo / 関数呼び出し実装 |
| アーキ | C | TBD / 設計レビュー |

## §5 期限 / 触发条件

- **决策期限**: M-10 着手 -3 日
- **反映先**:
  - `docs/architecture/07-qa-register.md` §3.2 QA-S03
  - `migrations/0011_security_definer_roles.sql`
  - `docs/operations/08-secure-functions.md` (運用ガイド)
- **チェックリスト**:
  - 全 SECURITY DEFINER 関数に `SET search_path`
  - 全関数で `PUBLIC` デフォルト REVOKE
  - `ada_func` ロールに最小権限のみ
- **再评估触发**:
  - 監査で SECURITY DEFINER 関数に `search_path` 未設定発見 → 緊急修正
  - 新しい elevated 関数追加 → 本ポリシーに従う

## §6 影响範囲 / リスク

- **影响模块**:
  - `migrations/0009_gdpr_forget.sql` ([`p0-08`](./p0-08-GDPR.md) と連動)
  - `migrations/0008_audit_log_partition.sql` ([`p0-05`](./p0-05-audit_partition.md) と連動)
  - 全 PL/pgSQL 関数
- **リスク评估**:
  - search_path 設定漏れ: SQL インジェクション
  - 過剰権限: 最小権限違反
  - 監査ログなし: 関数呼び出し記録
- **緩和策**:
  - CI で `pg_dump` + 静的解析
  - 月次で SECURITY DEFINER 関数棚卸し
  - 監査ログに `current_user`, `function_name` 記録

## §7 参考 / 関連 ADR

- **D-ADR-12** (`02-design-adrs.md` §13): PL/pgSQL 開発者 - DBA 兼任
- 関連文档:
  - `docs/architecture/07-qa-register.md` §3.2 QA-S03
  - PostgreSQL 18.6 Documentation - SECURITY DEFINER, search_path
  - OWASP - SQL Injection Prevention Cheat Sheet

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
