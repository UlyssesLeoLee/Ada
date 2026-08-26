# P0-04 Module Manifest JSON Schema

> **決議ID**: P0-04 (per DOC-DEC-001 矩阵 §1)
> **関連决议**: UN-P0-04 (`docs/architecture/07-qa-register.md` §5.1)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/01-p0-decision-matrix.md`](../01-p0-decision-matrix.md) §5

---

## §1 背景 / 問題

[`docs/modules/M-14-module-registry.md` §2.4](../../../modules/M-14-module-registry.md) で「atomic swap」のため **Module Manifest** が必要だが、JSON Schema が未定義。

QA 登録簿 §5.1 UN-P0-04 期限: **M-14 着手 -7 日**、Owner: **テックリード**。

## §2 决策

**採用案**: **JSON Schema Draft 2020-12 + WASM 実行時検証**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://ada.kanvas.dev/schemas/module-manifest/v1.json",
  "title": "Module Manifest",
  "type": "object",
  "required": ["name", "version", "entrypoint", "permissions", "dependencies"],
  "properties": {
    "name": { "type": "string", "pattern": "^[a-z][a-z0-9-]{2,63}$" },
    "version": { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+$" },
    "entrypoint": { "type": "string", "description": "WASM file path" },
    "permissions": {
      "type": "array",
      "items": {
        "type": "string",
        "enum": ["read:canvases", "write:canvases", "read:tenant", "network:outbound", "fs:read", "fs:write"]
      }
    },
    "dependencies": {
      "type": "array",
      "items": { "type": "string", "pattern": "^[a-z][a-z0-9-]+@\\d+\\.\\d+\\.\\d+$" }
    },
    "size_bytes": { "type": "integer", "maximum": 10485760 },
    "sandbox": {
      "type": "object",
      "properties": {
        "memory_mb": { "type": "integer", "maximum": 512 },
        "cpu_ms_per_call": { "type": "integer", "maximum": 1000 }
      }
    }
  }
}
```

## §3 選択肢と評価

### Option A: JSON Schema Draft 2020-12 ⭐ 推奨

- **优点**: 業界標準、JSON Schema バリデータ多数、ツール豊富（IDE, CI）、OpenAPI との相互運用
- **缺点**: 2020-12 対応のバリデータ実装が一部古い（jsonschema crate 1.5+ で対応）
- **リスク**: スキーマ進化時の後方互換性管理
- **可逆性**: 高（次バージョンで `$id` を `v2.json` に変更可）

### Option B: Protocol Buffers / FlatBuffers

- **优点**: 高速パース、型安全
- **缺点**: 人間可読性低、編集ツールが JSON 比で弱い、JSON Schema エコシステム未利用
- **リスク**: デバッグ困難、ドキュメント生成も JSON ほど成熟していない
- **可逆性**: 中（移行コスト中）

### Option C: 自前 YAML/JSON + アドホック検証

- **优点**: 自由度高、依存最小
- **缺点**: バリデーション属人化、エラー検出遅延
- **リスク**: プラグイン作者の認知負荷高
- **可逆性**: 高

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| アーキテクト | A, R | Ulysses (アーキ兼任) / 2026-08-29 |
| テックリード | C | アーキ兼任 / Day 2 |
| バックエンド Dev | I | Solo / M-14 着手時 |
| PO | I | Ulysses / Day 2 |

## §5 期限 / 触发条件

- **决策期限**: 2026-08-29（Day 2）
- **反映先**:
  - `docs/modules/M-14-module-registry.md` §2.4 に追記
  - `crates/ada-module-registry/src/schema.rs` に組み込み
  - `schemas/module-manifest/v1.json` をリポジトリに配置
- **再评估触发**:
  - プラグイン作者から「schema 厳しすぎ」フィードバック → permissions enum 拡張
  - スキーマバリデータで脆弱性 → Draft 2020-12 へ追従

## §6 影响範囲 / リスク

- **影响模块**:
  - `crates/ada-module-registry/` (M-14)
  - プラグイン開発者 SDK
  - atomic swap ワークフロー
- **リスク评估**:
  - スキーマ進化: `$id` バージョン固定 + 旧 `$id` 同時配信
  - バリデーション性能: WASM ロード時の 1 回のみ評価、性能影響軽微
  - 互換性: Draft 2020-12 対応の `jsonschema` crate 1.5+ 必要
- **緩和策**:
  - スキーマ検証を CI で自動化（schemas/ 配下）
  - 例外時エラーメッセージを JSON Pointer で明確化
  - スキーマ変更時の Deprecation 期間 6 ヶ月

## §7 参考 / 関連 ADR

- **D-ADR-02** (`02-design-adrs.md` §3): プラグイン沙箱 - WASM (wasmtime)
- **D-ADR-05** (`02-design-adrs.md` §6): WASM Bundle Size - < 8 MB
- 関連文档:
  - `docs/decisions/01-p0-decision-matrix.md` §5 UN-P0-04
  - `docs/modules/M-14-module-registry.md` §2.4
  - `docs/architecture/07-qa-register.md` §3.1 QA-D04
  - JSON Schema Draft 2020-12 仕様

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
