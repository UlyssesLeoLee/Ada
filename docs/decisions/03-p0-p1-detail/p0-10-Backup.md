# P0-10 Backup/Restore 戦略 (4 段 + 週次リストア)

> **決議ID**: P0-10 (per DOC-DEC-001 矩阵 §1)
> **関連决议**: UN-P0-10 (`docs/architecture/07-qa-register.md` §5.1)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/01-p0-decision-matrix.md`](../01-p0-decision-matrix.md) §11

---

## §1 背景 / 問題

[`docs/requirements/05-nfr-non-functional-requirements.md` NFR-AVA-07 RTO 1h / NFR-AVA-08 RPO 5min](../../../requirements/05-nfr-non-functional-requirements.md) だが、**具体的 Backup 戦略** 未定。

QA 登録簿 §5.1 UN-P0-10 期限: **プレプロダクション環境構築前**、Owner: **SRE**。

GDPR 削除フロー（[`p0-08-`](./p0-08-GDPR.md)）と連動し、バックアップ内の PII 自動消滅も考慮。

## §2 决策

**採用案**: **4 段 Backup + 週次リストア検証**

| Backup | 頻度 | 保持 | 保管 | 暗号化 |
|---|---|---|---|---|
| フル（pg_dump） | 日次 02:00 | 30 日 | S3 別 AZ | AES-256 |
| 増分（WAL） | 連続 | 7 日 | S3 別 AZ | AES-256 |
| スナップショット | 週次 日曜 03:00 | 4 週 | 別リージョン | KMS |
| 設定 (Terraform) | 変更毎 | ∞ | Git | — |
| シークレット | 変更毎 | ∞ | KMS 内部 | — |

**RTO / RPO 検証:**

| シナリオ | RPO 目標 | RTO 目標 | 検証頻度 |
|---|---|---|---|
| DB クラッシュ | < 5 min | < 30 min | 週次 Backup リストア |
| データセンター消失 | < 1 h | < 1 h | 月次 DR 訓練 |
| Backup 失敗 | — | — | 日次自動アラート |

## §3 選択肢と評価

### Option A: 4 段 Backup + 週次リストア ⭐ 推奨

- **优点**: RTO/RPO 目標達成、Backup 多層化、DR 訓練容易
- **缺点**: コスト中（ストレージ）、運用負荷（週次リストア確認）
- **リスク**: Backup 失敗検出遅延（アラート自動化で緩和）
- **可逆性**: 高

### Option B: フル Backup のみ（pg_dump 日次）

- **优点**: シンプル、低コスト
- **缺点**: RPO 24h（リストア時点までデータ損失）
- **リスク**: NFR-AVA-08 (RPO 5min) 違反
- **可逆性**: 高

### Option C: スナップショットのみ（EBS / Disk）

- **优点**: 高速リストア、アプリ整合性不要
- **缺点**: クラッシュ整合性なし、コスト高
- **リスク**: スナップショット I/O フリーズ
- **可逆性**: 中

### Option D: ベンダ DRaaS (Disaster Recovery as a Service)

- **优点**: マネージド、訓練不要
- **缺点**: ベンダロックイン、コスト高
- **リスク**: ベンダ障害 = 自社障害
- **可逆性**: 低

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| DBA | A | Ulysses (DBA 兼任) / 2026-08-30 |
| SRE | R | 外注 / プレプロダクション環境構築前 |
| アーキ | C | TBD / Day 3 |
| PO | I | Ulysses / Day 3 |
| コンプラ | I | 外注 / 規制適用前 |

## §5 期限 / 触发条件

- **决策期限**: 2026-08-30（Day 3）
- **プレプロダクション環境構築前**: 2026-09-15 目標
- **反映先**:
  - `docs/architecture/04-atomic-deployment.md` §3.4
  - `docs/templates/03-process-management.md` §A.4 (Backup 手順)
  - Terraform モジュール `terraform/backup/`
  - リストア手順書 `docs/operations/03-backup-restore.md` (新規)
- **再评估触发**:
  - Backup サイズ > 1TB/週 → 増分 backup 検討
  - リストア時間 > 30 min → スナップショット併用
  - コスト > 月 $1,000 → S3 Glacier 移行

## §6 影响範囲 / リスク

- **影响模块**:
  - PostgreSQL DB 全体
  - audit_log ([`p0-05-`](./p0-05-audit_partition.md) と連動)
  - GDPR 削除フロー（[`p0-08-`](./p0-08-GDPR.md) と連動）
  - ログ基盤（[`p0-09-`](./p0-09-log.md) と連動）
- **リスク评估**:
  - Backup 漏れの検出: 自動監視 + 失敗時 P1 アラート
  - リストア未検証: 週次自動化で staging に復元
  - PII 含有 Backup: 暗号化 + 30 日保持後自動削除
- **緩和策**:
  - Backup ジョブ監視（Prometheus exporter）
  - 週次リストアを staging 環境で自動実行（crontab + 検証スクリプト）
  - 月次 DR 訓練（az 障害想定、本番影響なし）

## §7 参考 / 関連 ADR

- 関連文档:
  - `docs/decisions/01-p0-decision-matrix.md` §11 UN-P0-10
  - `docs/requirements/05-nfr-non-functional-requirements.md` NFR-AVA-07/08
  - `docs/architecture/07-qa-register.md` §3.3 QA-O03
  - PostgreSQL 18.6 Documentation - pg_dump, WAL archiving
  - AWS S3 + Glacier ドキュメント

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
