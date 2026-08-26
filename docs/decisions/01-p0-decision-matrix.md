# P0 决策矩阵（P0 Decision Matrix）

> **本文件的目的**：[DOC-ARCH-008 §5](../architecture/07-qa-register.md) の **11 件の P0 重要未決事項**を、各々 **2-4 個の選択肢 + 評価 + 推奨案 + 決定者 + 期限** でまとめる。PO が 1 件 30 分で判断できる粒度。  
> 11 件全消化で G4 实施着手判定通過可能。

> **ドキュメントID**：DOC-DEC-001
> **文書分類**：意思決定文書
> **バージョン**：v1.1.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-27
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：PO
> **上位文書**：[`docs/decisions/README.md`](README.md)
> **下位文書**：[`03-p0-p1-detail/`](03-p0-p1-detail/)（DOC-DEC-003 細化決議 25 ファイル）、各決定後、関連モジュール文書 §X を更新
> **関連文書**：[`docs/architecture/07-qa-register.md`](../architecture/07-qa-register.md)、[`docs/upstream/08-initial-risk-assessment.md`](../upstream/08-initial-risk-assessment.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（11 P0 决策选项） | Ada プロジェクトチーム | TBD | PO |
| v1.1.0 | 2026-08-27 | §14 P0 細化決議へのリンク追加（DOC-DEC-003 連動） | Mavis (per DEC-008) | ⏳ 待 Ulysses | ⏳ 待 Ulysses |

---

## 目次

1. サマリ表
2. UN-P0-01: Rust 16 crate 担当人員
3. UN-P0-02: 起草/レビュー/承認組織
4. UN-P0-03: canvas 循環 FK + DEFERRABLE
5. UN-P0-04: Module Manifest JSON Schema
6. UN-P0-05: audit_log パーティション DDL
7. UN-P0-06: KMS 選定
8. UN-P0-07: JWT 鍵ローテーション
9. UN-P0-08: 忘れられる権利対応フロー
10. UN-P0-09: ログ基盤選定
11. UN-P0-10: Backup/Restore 戦略
12. UN-P0-11: ADR レビュー会
13. 决定完了チェックリスト
14. P0 細化決議へのリンク
15. 用語集
16. 参考文献

---

## 1. サマリ表

| ID | 主题 | 紧急度 | 推奨案 | 决定者 | 期限 |
|---|---|---|---|---|---|
| UN-P0-01 | 16 crate 人員 | 🔴 极高 | 段階採用 + 2 crate 外注 | PO + PM | Day 1 |
| UN-P0-02 | 起草/レビュー/承認組織 | 🔴 极高 | 最小 5 名 組織 | PO | Day 1 |
| UN-P0-03 | canvas 循環 FK | 🟡 高 | DEFERRABLE INITIALLY DEFERRED | DBA | Day 2 |
| UN-P0-04 | Module Manifest Schema | 🟡 高 | JSON Schema Draft 2020-12 | アーキ | Day 2 |
| UN-P0-05 | audit_log パーティション | 🟡 高 | 月次 RANGE + 1 年保存 | DBA | Day 2 |
| UN-P0-06 | KMS 選定 | 🟡 高 | AWS KMS (本番) + Vault (dev) | SecO | Day 2 |
| UN-P0-07 | JWT 鍵ローテーション | 🟡 高 | kid クレーム + 90 日ローテ | SecO | Day 2 |
| UN-P0-08 | 忘れられる権利フロー | 🟡 高 | 30 日以内削除 + 監査ログ | PO + SecO | Day 3 |
| UN-P0-09 | ログ基盤 | 🟡 高 | Loki (低コスト) | SRE | Day 3 |
| UN-P0-10 | Backup 戦略 | 🟡 高 | pg_dump 日次 + WAL + 別 AZ | DBA + SRE | Day 3 |
| UN-P0-11 | ADR レビュー会 | 🟡 高 | 週次 30 分 / テックリード主催 | テックリード | Day 1 |

---

## 2. UN-P0-01: Rust 16 crate 担当人員

### 2.1 課題

[DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md) で 16 crate + 2 共有 crate = 18 個の Rust crate 構造を定義したが、**実装者が 0 名**。Rust 専門人材の確保が G4 实施着手判定の最大ブロッカー。

### 2.2 選択肢

| 选项 | 規模 | 期間 | コスト | リスク | 適合性 |
|---|---|---|---|---|---|
| **A. 16 名内部採用** | 16 名 | 4-6 月採用 | 高（年俸 ×16） | 採用難 | 大企業向け |
| **B. 4-6 名コア + 外注** | 6 内部 + 10 外注 | 2 月 | 中 | 外部依存 | ⭐ **推奨** |
| **C. Solo + AI 補助（当プロジェクト）** | 1 名 + AI | 即時 | 低 | AI 品質 | ⭐ 個人 / 検証用 |
| **D. 外部 1 社一括委託** | 5-8 名（外部） | 1 月 | 中-高 | ベンダロックイン | 中堅企業 |

### 2.3 推奨：**C（Solo + AI）** + **B（段階採用）** 併用

- 当面（1-3 月）：Solo + AI 補助で核心 2-3 crate（[M-13 API Gateway](../modules/M-13-api-gateway.md)、[M-10 テナント](../modules/M-10-tenant-middleware.md)、[M-15 イベントバス](../modules/M-15-central-event-bus.md)）を実装
- 段階採用（3-6 月）：並行して 2-4 名を採用・教育、残り crate を委譲

### 2.4 决定記入欄

```
[决定] C を採用、当面 Solo+AI 補助。3 月後に 1-2 名追加採用を計画。
[根拠] 1 人でも G4 通過可能、AI で開発速度 3-5x 向上可能
[决定者] Ulysses (PO+PM 兼任) | [决定日] 2026-08-20
```

---

## 3. UN-P0-02: 起草/レビュー/承認組織

### 3.1 課題

[DOC-UP-001 §4 RACI](../upstream/01-pj-charter.md) で 12 ロールを定義したが、**現実の 1-2 名しかいない**。G2/G3/G7 等のレビューで「誰が決裁するか」未定。

### 3.2 選択肢

| 选项 | 構成 | 适合性 |
|---|---|---|
| **A. フル組織 12 名** | 12 ロール × 1-2 名 | 大企業のみ |
| **B. 最小組織 5 名** | PO+PM、アーキ、テック、QA、SecO（兼務） | ⭐ **推奨** |
| **C. 1 名独裁** | PO 単独 | 検証 / プロトタイプ |

### 3.3 推奨：**B（最小組織）** + 兼務

| ロール | 主担当 | 兼務 |
|---|---|---|
| PO | Ulysses | — |
| PM | Ulysses（PO 兼任） | — |
| アーキ | TBD 採用 or 外注 | PO が決定 |
| テックリード | アーキ兼任 | — |
| Dev | Solo + AI | — |
| QA | PO 兼任 | UAT は Biz ユーザー |
| SecO | 外注（コンプラ） | アーキ兼任可 |
| DBA | 外注 | — |
| SRE | 外注 | — |

### 3.4 决定記入欄

```
[决定] B 採用。PO/PM/アーキ兼任、外注は SecO/DBA/SRE。
[根拠] Solo+最小組織で G4 通過可能
[决定者] Ulysses | [决定日] 2026-08-20
```

---

## 4. UN-P0-03: canvas 循環 FK + DEFERRABLE

### 4.1 課題

[DOC-MOD-010 §4](../modules/M-10-tenant-middleware.md) で `canvas.current_version_id` → `canvas_version.id` のような自己参照 FK が含まれる場合、循環参照が発生し INSERT 時に FK 違反。

### 4.2 解決策

```sql
-- 推奨 SQL
ALTER TABLE canvas
  ADD CONSTRAINT fk_canvas_current_version
  FOREIGN KEY (current_version_id) 
  REFERENCES canvas_version(id)
  DEFERRABLE INITIALLY DEFERRED;
```

| 选项 | 説明 | 適合性 |
|---|---|---|
| **A. DEFERRABLE INITIALLY DEFERRED** | トランザクション終了まで FK チェック延期 | ⭐ **推奨**（PostgreSQL 標準） |
| **B. NULL 許容 + トリガ** | 親なし → トリガで更新 | 複雑 |
| **C. 別テーブル化** | canvas と current_version_id を別テーブルに | 性能影響大 |

### 4.3 决定記入欄

```
[决定] A 採用。DEFERRABLE INITIALLY DEFERRED で全循環 FK を設定。
[根拠] PostgreSQL 標準サポート、INSERT/UPDATE 同トランザクション内 OK
[决定者] Ulysses (DBA 兼任) | [决定日] 2026-08-20
[DDL 反映] DOC-MOD-010 §4 に追記
```

---

## 5. UN-P0-04: Module Manifest JSON Schema

### 5.1 課題

[DOC-MOD-014 §2.4](../modules/M-14-module-registry.md) で「atomic swap」のため **Module Manifest** が必要だが、JSON Schema が未定義。

### 5.2 推奨スキーマ（ドラフト）

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

### 5.3 决定記入欄

```
[决定] 上記スキーマ（Draft 2020-12）を採用。
[根拠] 業界標準、JSON Schema バリデータ多数、ツール豊富
[决定者] Ulysses (アーキ兼任) | [决定日] 2026-08-20
[反映] DOC-MOD-014 §2.4 に追記 + crates/ada-module-registry/src/schema.rs に組み込み
```

---

## 6. UN-P0-05: audit_log パーティション DDL

### 6.1 課題

[DOC-MOD-010 §4.4](../modules/M-10-tenant-middleware.md) で audit_log を 1 年保存と決めたが、PostgreSQL の **テーブル肥大化** 対策（パーティション）が未実装。

### 6.2 推奨 DDL

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

-- 月次パーティション（過去 12 月 + 今後 12 月）
-- pg_partman で自動化推奨
CREATE TABLE audit_log_2026_08 PARTITION OF audit_log
  FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');

-- 1 年以上前は別テーブルへアーカイブ
-- audit_log_archive_y2025
```

### 6.3 决定記入欄

```
[决定] 上記 DDL を採用。pg_partman で月次自動作成。
[根拠] 月次 RANGE パーティションで性能維持、ハッシュチェーン改ざん検知併用
[决定者] Ulysses (DBA 兼任) | [决定日] 2026-08-20
[反映] DOC-MOD-010 §4.4 に追記 + migrations/0008_audit_log_partition.sql
```

---

## 7. UN-P0-06: KMS 選定

### 7.1 課題

[NF-SEC-15 KMS 集中管理](../requirements/05-nfr-non-functional-requirements.md) が必要だが、**どの KMS を使うか** 未定。

### 7.2 選択肢

| 选项 | 用途 | コスト | FIPS 140-2 | 適合性 |
|---|---|---|---|---|
| **A. AWS KMS** | 本番クラウド | 中（API 課金） | ✅ Level 2 | ⭐ **推奨（クラウド時）** |
| **B. HashiCorp Vault OSS** | オンプレ / 開発 | 無料（OSS） | △（要 Enterprise） | ⭐ **推奨（オンプレ時）** |
| **C. Azure Key Vault** | Azure 環境 | 中 | ✅ | Azure 採用時 |
| **D. GCP Cloud KMS** | GCP 環境 | 中 | ✅ | GCP 採用時 |
| **E. 自前 (KMS なし)** | 最小構成 | 無料 | ❌ | ❌ 不可（[NF-SEC] 違反） |

### 7.3 推奨：**A + B 併用**

- 本番：AWS KMS（クラウドネイティブ、FIPS 140-2 Level 2、低運用負荷）
- 開発 / オンプレ：HashiCorp Vault OSS（無料で同じ API 体験）

### 7.4 决定記入欄

```
[决定] 本番 = AWS KMS、開発 = Vault OSS の 2 段構成。
[根拠] クラウドは native、オンプレは OSS で TCO 削減
[决定者] Ulysses (SecO 兼任) | [决定日] 2026-08-20
[反映] DOC-REQ-SEC-001 §3, DOC-ARCH-007 §N
```

---

## 8. UN-P0-07: JWT 鍵ローテーション

### 8.1 課題

[NF-SEC-03 認証](../requirements/05-nfr-non-functional-requirements.md) で JWT を使うが、**鍵ローテ方式** 未定。

### 8.2 推奨：kid クレーム方式

```json
// JWT ヘッダー
{
  "alg": "RS256",
  "typ": "JWT",
  "kid": "key-2026-08-20"  // 鍵 ID、日付で識別
}

// 公開鍵エンドポイント
GET /.well-known/jwks.json
{
  "keys": [
    {
      "kid": "key-2026-08-20",  // 現在
      "kty": "RSA",
      "alg": "RS256",
      "use": "sig",
      "n": "...",
      "e": "AQAB"
    },
    {
      "kid": "key-2026-05-20",  // 旧（検証のみ）
      "...": "..."
    }
  ]
}
```

| 选项 | 説明 | 適合性 |
|---|---|---|
| **A. kid + JWKS** | 複数鍵を並行稼働、graceful rotation | ⭐ **推奨** |
| **B. 単一鍵、定期交換** | ダウンタイム必要 | ❌ |
| **C. 鍵交換プロトコル** | 複雑 | 不要 |

### 8.3 ローテスケジュール

- **頻度**：90 日毎
- **運用**：旧鍵を 7 日間残す（猶予期間）
- **手順**：(1) 新鍵生成 (2) JWKS に追加 (3) 7 日後に旧鍵削除

### 8.4 决定記入欄

```
[决定] kid + JWKS 方式、90 日ローテ + 7 日 grace period。
[根拠] 業界標準（OAuth 2.0）、ダウンタイムなし
[决定者] Ulysses (SecO 兼任) | [决定日] 2026-08-20
[反映] DOC-REQ-SEC-001 §1, crates/ada-gateway/src/auth.rs
```

---

## 9. UN-P0-08: 忘れられる権利対応フロー

### 9.1 課題

[GDPR Art.17 / PIPL §47](../requirements/08-security-requirements.md) で本人削除要求対応が必要だが、**運用フロー** 未定義。

### 9.2 推奨フロー

```
1. ユーザー要求受付（support@ada.kanvas.dev）
   ↓
2. 本人確認（メール + ID 検証）
   ↓
3. 削除対象データ特定（email, name, audit_log, content）
   ↓
4. 削除実行（30 日以内）
   - 業務データ: DELETE FROM ... WHERE user_id = ?
   - 監査ログ: ハッシュ化（トレース用）+ 個人情報削除
   - バックアップ: 次回 Backup サイクルで自動消滅
   ↓
5. 削除完了通知（本人 + 監査ログに記録）
   ↓
6. SLA: 30 日以内（GDPR Art.12）
```

### 9.3 推奨 SQL

```sql
-- ユーザー削除プロシージャ
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

### 9.4 决定記入欄

```
[决定] 上記フロー + PL/pgSQL 存過を採用。30 日 SLA。
[根拠] GDPR Art.12/17、PIPL §47 準拠
[决定者] Ulysses (PO+SecO) | [决定日] 2026-08-20
[反映] DOC-REQ-SEC-001 §4, migrations/0009_gdpr_forget.sql
```

---

## 10. UN-P0-09: ログ基盤選定

### 10.1 課題

[NFR-OPS-02 ログ構造化](../requirements/05-nfr-non-functional-requirements.md) で JSON 100% だが、**集約基盤** 未定。

### 10.2 選択肢

| 选项 | コスト | 性能 | 適合性 |
|---|---|---|---|
| **A. Grafana Loki** | 低（OSS） | 中 | ⭐ **推奨**（中小規模） |
| **B. ELK Stack** | 高（RAM 大） | 高 | 大規模（> 10TB/日） |
| **C. CloudWatch Logs** | 中（従量） | 高 | AWS 環境 |
| **D. Datadog** | 高 | 高 | 商用 SaaS、容易 |

### 10.3 推奨：**A（Loki）** 段階的

- Phase 1: Loki OSS + Promtail（コンテナログ自動収集）
- Phase 2: 必要時 B（ELK）または C（CloudWatch）に移行

### 10.4 决定記入欄

```
[决定] Loki + Promtail を採用。Grafana でダッシュボード。
[根拠] OSS、低コスト、Prometheus との統合容易
[决定者] Ulysses (SRE 兼任) | [决定日] 2026-08-20
[反映] DOC-ARCH-005 §5, crates/ada-telemetry/src/loki.rs
```

---

## 11. UN-P0-10: Backup/Restore 戦略

### 11.1 課題

[NFR-AVA-07 RTO 1h / NFR-AVA-08 RPO 5min](../requirements/05-nfr-non-functional-requirements.md) だが、**具体的 Backup 戦略** 未定。

### 11.2 推奨戦略

| Backup | 頻度 | 保持 | 保管 | 暗号化 |
|---|---|---|---|---|
| フル（pg_dump） | 日次 02:00 | 30 日 | S3 別 AZ | AES-256 |
| 増分（WAL） | 連続 | 7 日 | S3 別 AZ | AES-256 |
| スナップショット | 週次 日曜 03:00 | 4 週 | 別リージョン | KMS |
| 設定 (Terraform) | 変更毎 | ∞ | Git | — |
| シークレット | 変更毎 | ∞ | KMS 内部 | — |

### 11.3 RTO / RPO 検証

| シナリオ | RPO 目標 | RTO 目標 | 検証頻度 |
|---|---|---|---|
| DB クラッシュ | < 5 min | < 30 min | 週次 Backup リストア |
| データセンター消失 | < 1 h | < 1 h | 月次 DR 訓練 |
| Backup 失敗 | — | — | 日次自動アラート |

### 11.4 决定記入欄

```
[决定] 上記 4 段 Backup + 週次リストア検証を採用。
[根拠] RTO/RPO 目標達成、Backup 多層化
[决定者] Ulysses (DBA+SRE 兼任) | [决定日] 2026-08-20
[反映] DOC-ARCH-004 §3.4, DOC-TPL-OPS §A.4
```

---

## 12. UN-P0-11: ADR レビュー会

### 12.1 課題

[DOC-ARCH-007 §10](../architecture/06-rust-tech-selection.md) で 10 ADR を提示したが、**正式レビュー会** 未開催。

### 12.2 推奨プロセス

| 項目 | 内容 |
|---|---|
| 頻度 | 週次 30 分（[DOC-MGT-COM-001 §1 アーキ会議](../management/04-communication-plan.md) と同時） |
| 主催 | テックリード |
| 参加者 | アーキ、テック、Dev 代表、PO、SecO（必要時） |
| 議題 | 保留中 ADR の GO/NO-GO |
| 議事録 | [DOC-TPL-PRC §A.5](../templates/03-process-management.md) で記録 |
| 決定 | 3 名以上の合議、過半数 |

### 12.3 保留中 ADR

| ADR# | 主题 | 状态 |
|---|---|---|
| ADR-01 | Cargo Workspace 構成 | ✅ DOC-ARCH-007 §18 |
| ADR-02 | Rust 1.74+ Edition 2021 | ✅ 同 §3 |
| ADR-03 | axum vs actix-web | ✅ 同 §5 |
| ADR-04 | sqlx vs diesel | ✅ 同 §6 |
| ADR-05 | tracing vs log | ✅ 同 §10 |
| ADR-06 | tokio vs async-std | ✅ 同 §4 |
| ADR-07 | Bevy 0.14 vs 0.15 | 🟡 保留 |
| ADR-08 | CRDT 库選型 | 🟡 保留（D-01 連動） |
| ADR-09 | プラグイン沙箱 (WASM vs 进程) | 🟡 保留（D-02 連動） |
| ADR-10 | License 選定 | 🟡 保留 |

### 12.4 决定記入欄

```
[决定] 週次アーキ会議で ADR レビュー実施。保留中 4 件は次回会議で決定。
[根拠] 既存会議体と統合、追加コスト 0
[决定者] Ulysses (テックリード兼任) | [决定日] 2026-08-20
[反映] DOC-MGT-COM-001 §1
```

---

## 13. 决定完了チェックリスト

| ID | 决定 | 决定者 | 完了 |
|---|---|---|---|
| UN-P0-01 |  |  | ☐ |
| UN-P0-02 |  |  | ☐ |
| UN-P0-03 |  |  | ☐ |
| UN-P0-04 |  |  | ☐ |
| UN-P0-05 |  |  | ☐ |
| UN-P0-06 |  |  | ☐ |
| UN-P0-07 |  |  | ☐ |
| UN-P0-08 |  |  | ☐ |
| UN-P0-09 |  |  | ☐ |
| UN-P0-10 |  |  | ☐ |
| UN-P0-11 |  |  | ☐ |

**11/11 完了で G4 通過可能**。

---

## 14. P0 細化決議へのリンク

各 P0 議題の背景・選択肢・評価・推奨案・RACI・期限を [`03-p0-p1-detail/`](03-p0-p1-detail/) で詳細展開している（DOC-DEC-003 細化決議群）。実装着手前に必ず参照。

| P0 議題 | 細化決議ファイル | 主题 |
|---|---|---|
| UN-P0-01 | [p0-01-人員.md](03-p0-p1-detail/p0-01-人员.md) | Rust 16 crate 担当人員（段階採用 + AI 補助） |
| UN-P0-02 | [p0-02-組織.md](03-p0-p1-detail/p0-02-组织.md) | 起草/レビュー/承認組織（最小 5 名） |
| UN-P0-03 | [p0-03-FK.md](03-p0-p1-detail/p0-03-FK.md) | canvas 循環 FK（DEFERRABLE INITIALLY DEFERRED） |
| UN-P0-04 | [p0-04-Manifest.md](03-p0-p1-detail/p0-04-Manifest.md) | Module Manifest JSON Schema (Draft 2020-12) |
| UN-P0-05 | [p0-05-audit_partition.md](03-p0-p1-detail/p0-05-audit_partition.md) | audit_log 月次 RANGE パーティション |
| UN-P0-06 | [p0-06-KMS.md](03-p0-p1-detail/p0-06-KMS.md) | KMS 選定 (AWS KMS + Vault OSS) |
| UN-P0-07 | [p0-07-JWT.md](03-p0-p1-detail/p0-07-JWT.md) | JWT 鍵ローテーション (kid + JWKS) |
| UN-P0-08 | [p0-08-GDPR.md](03-p0-p1-detail/p0-08-GDPR.md) | 忘れられる権利対応フロー (GDPR Art.17) |
| UN-P0-09 | [p0-09-log.md](03-p0-p1-detail/p0-09-log.md) | ログ基盤選定 (Loki + Promtail) |
| UN-P0-10 | [p0-10-Backup.md](03-p0-p1-detail/p0-10-Backup.md) | Backup/Restore 戦略 (4 段 + 週次リストア) |
| UN-P0-11 | [p0-11-ADR判定.md](03-p0-p1-detail/p0-11-ADR判定.md) | ADR レビュー会 (週次アーキ会議) |

---

## 15. 用語集

| 用語 | 説明 |
|---|---|
| P0 | Priority 0（最優先） |
| ADR | Architecture Decision Record |
| KMS | Key Management Service |
| JWT | JSON Web Token |
| GDPR | EU 一般データ保護規則 |
| PIPL | 中国個人情報保護法 |
| FK | Foreign Key |
| DEFERRABLE | FK チェックを遅延可能 |
| RTO / RPO | Recovery Time / Point Objective |

## 15. 参考文献

1. PMBOK Guide 第 7 版、Project Management Institute、2021 年
2. GDPR (EU 2016/679)
3. PIPL (中華人民共和国 2021)
4. PostgreSQL 18.6 Documentation
5. OAuth 2.0 / OpenID Connect 仕様
6. JSON Schema Draft 2020-12
7. Ada プロジェクトチーム「[DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
