# P0-06 KMS 選定 (AWS KMS + Vault OSS)

> **決議ID**: P0-06 (per DOC-DEC-001 矩阵 §1)
> **関連决议**: UN-P0-06 (`docs/architecture/07-qa-register.md` §5.1)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/01-p0-decision-matrix.md`](../01-p0-decision-matrix.md) §7

---

## §1 背景 / 問題

[`docs/requirements/05-nfr-non-functional-requirements.md` NF-SEC-15 KMS 集中管理](../../../requirements/05-nfr-non-functional-requirements.md) が必要だが、**どの KMS を使うか** 未定。

QA 登録簿 §5.1 UN-P0-06 期限: **M-01 着手 -7 日**、Owner: **セキュリティ + SRE**。

クラウド本番環境とオンプレ開発環境で異なる KMS を選定する必要がある。

## §2 决策

**採用案**: **Option A + B 併用（本番 = AWS KMS、開発 = Vault OSS）**

- 本番: AWS KMS（クラウドネイティブ、FIPS 140-2 Level 2、低運用負荷）
- 開発 / オンプレ: HashiCorp Vault OSS（無料で同じ API 体験）

## §3 選択肢と評価

### Option A: AWS KMS ⭐ 推奨（本番クラウド時）

- **优点**: FIPS 140-2 Level 2、IAM 統合、API 従量課金、AWS 監査ログ自動連携
- **缺点**: AWS ロックイン、API コール $1/10K 件、依存ネット
- **リスク**: AWS 障害で KMS 利用不可（リージョン冗長で緩和）
- **可逆性**: 中（移行先選定時、コモディティ API 抽象化必要）

### Option B: HashiCorp Vault OSS ⭐ 推奨（開発 / オンプレ時）

- **优点**: OSS 無料、PKI/Secrets/Transit 統合、ベンダ中立
- **缺点**: 自前運用（HA、backup、upgrade）、Enterprise でないと FIPS 認定なし
- **リスク**: 運用ミスで全鍵喪失（auto-unseal + 定期 backup 必須）
- **可逆性**: 高（OSS ソース内）

### Option C: Azure Key Vault

- **优点**: Azure 環境統合、FIPS 140-2 Level 2
- **缺点**: Azure ロックイン、Ada は現状 Azure 採用予定なし
- **リスク**: Azure 移行時に KMS 再選定
- **可逆性**: 中

### Option D: GCP Cloud KMS

- **优点**: GCP 環境統合、FIPS 140-2 Level 3
- **缺点**: GCP ロックイン、Ada は現状 GCP 採用予定なし
- **リスク**: GCP 移行時に KMS 再選定
- **可逆性**: 中

### Option E: 自前 (KMS なし) ❌ 不可

- **优点**: 無料、依存最小
- **缺点**: NF-SEC 違反、鍵管理属人化、監査不可
- **リスク**: セキュリティ事故、コンプラ違反
- **可逆性**: — (NF 違反)

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| セキュリティ責任者 (SecO) | A | 外注 / 2026-08-29 |
| SRE | R | 外注 / M-01 着手 -7 日 |
| アーキ | C | TBD / Day 2 |
| PO | I | Ulysses / Day 2 |

## §5 期限 / 触发条件

- **决策期限**: 2026-08-29（Day 2）
- **反映先**:
  - `docs/requirements/05-nfr-non-functional-requirements.md` NF-SEC-15
  - `docs/architecture/07-qa-register.md` §3.2 QA-S01
  - `crates/ada-gateway/src/kms.rs` (抽象化レイヤ)
  - `crates/ada-tenant-middleware/src/kms.rs` (抽象化レイヤ)
- **鍵ローテーション頻度**: 90 日 (credential 鍵)
- **再评估触发**:
  - AWS 障害 3 ヶ月 2 回以上 → GCP KMS 追加評価
  - 開発環境で Vault 運用コスト高 → AWS KMS dev 環境利用
  - コンプラ監査指摘 → FIPS 認定 KMS 必須

## §6 影响範囲 / リスク

- **影响模块**:
  - 全 crate の credential アクセス (`ada-gateway`, `ada-tenant-middleware`, `ada-event-bus`)
  - JWT 鍵管理 ([`p0-07-`](./p0-07-JWT.md) と連動)
  - audit_log 暗号化
  - GDPR 削除フローの鍵管理
- **リスク评估**:
  - AWS KMS API 障害: ローカルキャッシュ + リトライ
  - Vault 自己解除 (auto-unseal) 失敗: AWS KMS 連携 (Transit) で代替
  - 鍵喪失: KMS 履歴 + S3 backup、復元手順整備
- **緩和策**:
  - KMS 抽象化レイヤ (`kms::Provider` trait) で実装差を隠蔽
  - 全鍵を 90 日毎にローテーション + 旧鍵 7 日 grace
  - KMS 障害時 fallback（環境変数直接注入、開発時のみ）

## §7 参考 / 関連 ADR

- **D-ADR-12** (`02-design-adrs.md` §13): PL/pgSQL 開発者 - DBA 兼任
- 関連文档:
  - `docs/decisions/01-p0-decision-matrix.md` §7 UN-P0-06
  - `docs/requirements/05-nfr-non-functional-requirements.md` NF-SEC-15
  - `docs/architecture/07-qa-register.md` §3.2 QA-S01
  - AWS KMS 公式ドキュメント
  - HashiCorp Vault 公式ドキュメント

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
