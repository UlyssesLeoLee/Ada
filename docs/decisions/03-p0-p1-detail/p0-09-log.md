# P0-09 ログ基盤選定 (Loki + Promtail)

> **決議ID**: P0-09 (per DOC-DEC-001 矩阵 §1)
> **関連决议**: UN-P0-09 (`docs/architecture/07-qa-register.md` §5.1)
> **作成日**: 2026-08-27
> **起草**: Mavis (per DEC-008)
> **レビュー**: ⏳ 待 Ulysses
> **承認**: ⏳ 待 Ulysses
> **ステータス**: 草案 v0.1
> **上位文書**: [`docs/decisions/01-p0-decision-matrix.md`](../01-p0-decision-matrix.md) §10

---

## §1 背景 / 問題

[`docs/requirements/05-nfr-non-functional-requirements.md` NFR-OPS-02 ログ構造化](../../../requirements/05-nfr-non-functional-requirements.md) で JSON 100% だが、**集約基盤** 未定。

QA 登録簿 §5.1 UN-P0-09 期限: **実装着手 -7 日**、Owner: **SRE**。

中小規模（～ 1TB/日）での低コスト運用が目標。

## §2 决策

**採用案**: **Option A (Grafana Loki + Promtail) 段階的**

- Phase 1: Loki OSS + Promtail（コンテナログ自動収集）+ Grafana ダッシュボード
- Phase 2: 必要時 B（ELK）または C（CloudWatch）に移行

## §3 選択肢と評価

### Option A: Grafana Loki ⭐ 推奨（中小規模）

- **优点**: OSS、低コスト、Prometheus との統合容易、ラベルベースでクエリ高速
- **缺点**: 全文検索弱い（LogQL は構造化クエリ）、運用知見が ELK 比で少ない
- **リスク**: Loki 単一障害点（マルチテナント化で緩和）
- **可逆性**: 中（ログフォーマット標準化で移行可能）

### Option B: ELK Stack

- **优点**: 全文検索最強、運用実績大、Kibana ダッシュボード高機能
- **缺点**: コスト高（RAM 大）、運用負荷高
- **リスク**: Elasticsearch クラスタ運用難度、ライセンス変更（SSPL/Elastic License）
- **可逆性**: 中

### Option C: CloudWatch Logs

- **优点**: AWS マネージド、運用負荷低、AWS 監査ログ自動連携
- **缺点**: AWS ロックイン、従量課金で大規模時コスト高
- **リスク**: AWS 障害
- **可逆性**: 中

### Option D: Datadog

- **优点**: SaaS で容易、APM 統合、メトリクス・ログ・トレース統合
- **缺点**: コスト高（ホスト数 × プラン）、ベンダ依存
- **リスク**: コスト予測難、契約条件変更
- **可逆性**: 中

## §4 RACI

| 角色 | R / A / C / I | 担当者 / 期限 |
|---|---|---|
| SRE | A, R | 外注 / 2026-08-30 |
| アーキ | C | TBD / Day 3 |
| バックエンド Dev | I | Solo / 実装着手時 |
| PO | I | Ulysses / Day 3 |

## §5 期限 / 触发条件

- **决策期限**: 2026-08-30（Day 3）
- **反映先**:
  - `docs/architecture/05-admin-operations-ui.md` §5 (Ops ダッシュボード)
  - `crates/ada-telemetry/src/loki.rs` (ログ送信クライアント)
  - Helm chart `deploy/helm/loki/`
  - Grafana ダッシュボード JSON
- **再评估触发**:
  - ログ量 > 1TB/日 → ELK 検討
  - Loki 障害 3 ヶ月 2 回以上 → CloudWatch Logs 評価
  - 全文検索需要増 → ELK 併用

## §6 影响範囲 / リスク

- **影响模块**:
  - 全 16 crate のログ出力
  - `crates/ada-telemetry/` (テレメトリ抽象化)
  - 監視・アラート基盤
  - バックアップ戦略（[`p0-10-`](./p0-10-Backup.md) と連動）
- **リスク评估**:
  - Loki ストレージ: S3 互換ストレージ使用、成本管理
  - ラベル cardinality: tenant_id をラベルにすると高基数 → 避ける
  - 構造化ログ強制: 全 crate で `tracing` JSON 化必須
- **緩和策**:
  - Loki は S3 互換オブジェクトストレージで永続化
  - ラベル設計は `level`, `service`, `env` のみ、user 固有情報は metadata
  - ログローテーション + 古いログ S3 Glacier 移行

## §7 参考 / 関連 ADR

- 関連文档:
  - `docs/decisions/01-p0-decision-matrix.md` §10 UN-P0-09
  - `docs/requirements/05-nfr-non-functional-requirements.md` NFR-OPS-02
  - `docs/architecture/07-qa-register.md` §3.3 QA-O01
  - Grafana Loki 公式ドキュメント
  - Promtail 公式ドキュメント

---

## 修订历史

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| v0.1 | 2026-08-27 | Mavis (per DEC-008) | 初版起草 |
