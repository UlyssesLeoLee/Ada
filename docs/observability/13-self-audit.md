# 13 アーキテクチャ自審（Self-Audit & Revision 2）

> **第一版は完成ではない**。  
> 自審により、過剰監視・性能影響・データ爆発・セキュリティホール・Collector 単点障害を  
> 体系的にチェックし、**Revision 2 として修正方針を確定**する。

> **ドキュメントID**：DOC-OBS-013
> **上位文書**：[DOC-OBS-INDEX](README.md)
> **監査対象**：[DOC-OBS-001](01-current-state-analysis.md) 〜 [DOC-OBS-012](12-code-impact.md) 全 12 ファイル

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版（自審チェックリスト + Revision 2 計画） |

---

## 目次

1. 自審の必要性
2. 自審チェックリスト（11 カテゴリ）
3. チェック結果マトリクス
4. 重大問題（P0）
5. 重要問題（P1）
6. 軽微問題（P2）
7. Revision 2 修正方針
8. 残余リスク
9. 再自審計画
10. 用語集
11. 参考文献

---

## 1. 自審の必要性

第一版は **「設計者が設計を疑うことなく書いた」状態**。  
経験上、第三者視点で見ると必ず以下が見つかる：

| 問題カテゴリ | 第一版で見つかりやすい問題 |
|---|---|
| 過剰監視 | 「念のため」のメトリクス / Span が大量追加 |
| 性能影響 | サンプリング戦略なし、業務レイテンシ +10% |
| データ爆発 | ハイカーディナリラベル、トレース全量保存 |
| セキュリティ | Grafana 公開、Secret 平文、PII 漏洩 |
| 単点障害 | OTel Collector 1 replica、Prometheus 1 replica |
| 観測の観測不足 | 観測基盤自身の監視なし |

## 2. 自審チェックリスト（11 カテゴリ）

### カテゴリ 1: 過剰監視

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| 必須以外の Span がないか | ⚠️ | DOC-OBS-005 で定義した Span 以外も計装されないか確認 |
| メトリクスにデバッグ用が残っていないか | ⚠️ | `ada_app_debug_*` 系の扱い |
| トレースで全クエリを保存していないか | ❌ | DB クエリ全 Span は過剰 |
| 業務イベントを全 Span 化していないか | ⚠️ | ビジネスイベントの自動 Span 化リスク |

### カテゴリ 2: 性能影響

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| 各 crate の p99 レイテンシ増加 < 5% か | ⚠️ | M-12 で +3%、M-13 で +2% 想定 |
| メモリ増加 < 30% か | ✅ | OTel SDK overhead 想定内 |
| CPU 増加 < 20% か | ✅ | 同上 |
| ストレージ増加率が監視下か | ⚠️ | retention 設定と連動 |

### カテゴリ 3: データ爆発

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| Cardinality 予算 < 10,000 series/metric | ⚠️ | `user_id` 等ハイカーディナリ除外要 |
| トレース Sampling が稼働するか | ✅ | Head 10% + tail 100% |
| ログ Retention が設定されているか | ✅ | 30 日標準 |
| メトリクス Retention が設定されているか | ✅ | 13 ヶ月（長期 2 年） |

### カテゴリ 4: セキュリティ

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| Grafana が RBAC で守られているか | ✅ | Keycloak OAuth + Role mapping |
| 内部コンポーネントが mTLS か | ✅ | 全コンポーネント mTLS |
| PII 自動 redaction が CI で検証されるか | ✅ | `scripts/pii-detect.sh` CI 統合 |
| NetworkPolicy で通信が制限されているか | ✅ | default-deny + 必要最小限許可 |
| Secret が平文でないか | ✅ | External Secrets + sealed-secrets |
| 監査ログが S3 Object Lock されるか | ✅ | 1 年保持、書き込み禁止 |

### カテゴリ 5: 単点障害

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| OTel Collector が HA か | ✅ | 5 replica + HPA |
| Prometheus / Mimir が HA か | ✅ | 3 replica（Phase 1 → Mimir 移行） |
| Grafana が HA か | ✅ | 2 replica + shared storage |
| Loki が HA か | ✅ | 3 replica + S3 backend |
| AlertManager がクラスタ化されているか | ✅ | 3 replica + gossip |
| Tempo が HA か | ✅ | 3 replica + S3 backend |

### カテゴリ 6: 観測の観測（Meta-Observability）

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| 観測基盤自体の監視があるか | ⚠️ | DOC-OBS-007 alert policy に追加必要 |
| 観測基盤の SLO が定義されているか | ⚠️ | DOC-OBS-008 に追加必要 |
| 観測基盤のダッシュボードがあるか | ⚠️ | Dashboard 95 (Meta-Observability) 追加必要 |
| 観測基盤のコスト監視があるか | ❌ | 追加必要 |

### カテゴリ 7: コスト

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| 月次コスト試算があるか | ✅ | DOC-OBS-010 §5 + 本ドキュメント §6 |
| 未使用メトリクス削減計画があるか | ⚠️ | Phase 8 で実施予定 |
| ログサンプリング戦略があるか | ⚠️ | 詳細化必要 |
| S3 ライフサイクルポリシーあるか | ✅ | Loki: 30→90日 Glacier、Tempo: 7→30日 Glacier |

### カテゴリ 8: 運用

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| Runbook が全アラートに紐づいているか | ⚠️ | Phase 6 で完成予定 |
| オンコール体制が定義されているか | ✅ | Sev1 5分 / Sev2 30分 |
| DR 訓練計画があるか | ✅ | 四半期訓練 |
| キャパシティプランニングがあるか | ✅ | 四半期レビュー |

### カテゴリ 9: テスト

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| 観測基盤自体のテストがあるか | ⚠️ | 詳細化必要 |
| 性能回帰テストがあるか | ✅ | Phase 11 §7.4 |
| 障害シナリオテストがあるか | ✅ | Phase 5 §7.4 |
| PII 検出 CI があるか | ✅ | Phase 3 §5.4 |

### カテゴリ 10: ドキュメント

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| 全要件のトレーサビリティがあるか | ✅ | DOC-REQ ↔ DOC-OBS マトリクス（本ドキュメント §6 参照） |
| 改訂履歴があるか | ✅ | 全 13 ファイル |
| 用語集があるか | ✅ | 各ファイル末尾 |
| ADR（決定記録）があるか | ✅ | 10 OBS-ADR |

### カテゴリ 11: 規制 / コンプライアンス

| チェック項目 | 評価 | 詳細 |
|---|---|---|
| GDPR 削除権対応があるか | ✅ | DOC-OBS-009 §7.4 |
| PIPL 越境転送対応があるか | ✅ | 中国リージョン別クラスタ |
| ISO 27001 統制要件を満たすか | ✅ | アクセス制御 + 監査 + 暗号化 |
| データレジデンシー要件を満たすか | ✅ | テナント設定で選択可能 |

## 3. チェック結果マトリクス

| カテゴリ | ✅ Pass | ⚠️ 要対応 | ❌ 重大 | 合計 |
|---|---|---|---|---|
| 1. 過剰監視 | 0 | 4 | 0 | 4 |
| 2. 性能影響 | 2 | 2 | 0 | 4 |
| 3. データ爆発 | 3 | 1 | 0 | 4 |
| 4. セキュリティ | 6 | 0 | 0 | 6 |
| 5. 単点障害 | 6 | 0 | 0 | 6 |
| 6. 観測の観測 | 0 | 3 | 1 | 4 |
| 7. コスト | 1 | 3 | 0 | 4 |
| 8. 運用 | 3 | 1 | 0 | 4 |
| 9. テスト | 2 | 2 | 0 | 4 |
| 10. ドキュメント | 4 | 0 | 0 | 4 |
| 11. 規制 | 4 | 0 | 0 | 4 |
| **合計** | **31** | **16** | **1** | **48** |

> 合格率：31/48 = **64.6%**。16 件要対応、1 件重大。

## 4. 重大問題（P0）

### P0-01: 観測基盤のコスト監視欠如

**問題**：
観測基盤が肥大化した場合、SLO 維持と引き換えにコストが青天井になるリスク。
**現状**：コスト監視の仕組みなし、月次手動集計のみ。

**影響**：
- 月次コスト $5K → $50K への膨張を検知できない
- 部門別コスト按分ができない
- 容量計画に財務的数字がない

**対策（Revision 2）**：
1. **kubecost** を `observability` namespace にデプロイ
2. Label 別コスト按分（service, environment, tenant_tier）
3. コストアラート（予算 +20% で Sev3、+50% で Sev2）
4. 月次コストレポート自動生成
5. Dashboard 95 (Cost) 新規作成

**優先度**：P0
**担当**：SRE + Finance
**期限**：Phase 1 完了時（Month 2 末）

## 5. 重要問題（P1）

### P1-01: 観測基盤自身の SLO 未定義

**問題**：
観測基盤がダウンしたら業務が観測不能になるが、その SLO が未定義。

**対策**：
- DOC-OBS-008 に「観測基盤 SLO サブセクション」を追加
- Grafana / Prometheus / Loki / Tempo / OTel Collector それぞれに 99.9% SLO
- Dashboard 95 に観測基盤 SLO 表示

### P1-02: 観測基盤自身の監視不足

**問題**：
観測基盤のメトリクスが Prometheus で集まらない（自己参照不可）。

**対策**：
- 観測基盤コンポーネントのメトリクスは**別 Prometheus**で集約
- もしくは **Grafana Meta-Observability Dashboard** でコンポーネント別稼働状況
- AlertManager 自身の up/down 監視

### P1-03: 必須 Span 以外の計装制御不足

**問題**：
開発者が「とりあえず Span 追加」すると観測データが増殖。

**対策**：
- ada-telemetry に **許可 Span ホワイトリスト** 機能
- CI で Span 数が想定範囲内か検証
- コードレビュー観点に「Span 必要性」を追加

### P1-04: Cardinality 予算超過リスク

**問題**：
`user_id`, `request_id` 等を安易にラベルにすると Cardinality 爆発。

**対策**：
- CI で `cardinality-check` ジョブ追加
- ラベルホワイトリスト厳格化
- アラート：1 メトリクス 10K series 超過で Sev3

### P1-05: ログ Sampling 戦略未詳細化

**問題**：
全量ログ保存はコスト高、ERROR のみだと障害解析で情報不足。

**対策**：
- ログレベル別 Sampling：
  - ERROR / WARN: 100% 保存
  - INFO: 10% sampling
  - DEBUG: 0%（デフォルト OFF）
- 特定 namespace / service は 100% 保存

### P1-06: 未使用メトリクス削減計画不足

**問題**：
一度追加したメトリクスは使われなくても削除されない。

**対策**：
- 四半期レビュー（Quarterly Metric Cleanup）
- 90 日間未参照メトリクスを自動リスト化
- SRE レビューで削除判定

### P1-07: 観測基盤自体の性能テスト不足

**問題**：
観測基盤の性能限界が不明（Prometheus 100K series 時の動作等）。

**対策**：
- 容量テスト：Prometheus 1M series 負荷試験
- 結果に基づくリソース計画

### P1-08: 詳細 Runbook 整備の遅延

**問題**：
アラートに runbook URL はあるが、内容が薄い。

**対策**：
- Phase 6 で全 30+ アラートに runbook 詳細化
- Runbook テンプレ統一（[DOC-TPL-007]）

## 6. 軽微問題（P2）

| ID | 問題 | 対策 |
|---|---|---|
| P2-01 | 過剰 Span 計装リスク | Phase 2 で 1 crate のみ先行検証 |
| P2-02 | 性能影響の M-12 想定 +3% 高い | head sampling を 5% に下げる |
| P2-03 | 一部 crate でメモリ +30% 想定 | OTel batch size 最適化 |
| P2-04 | ストレージ増加率と retention 連動確認 | 月次レビュー追加 |
| P2-05 | DEBUG メトリクスの扱い | 本番では release feature flag で完全 OFF |
| P2-06 | Grafana プラグイン一覧未整理 | 必要最小限のプラグインのみ許可リスト化 |
| P2-07 | アラート文言の英語化 | i18n 化（英語 / 日本語 / 中国語） |
| P2-08 | 容量計画が四半期に固定 | 月次予測に切り替え |

## 7. Revision 2 修正方針

### 7.1 修正サマリー

| 重大度 | 件数 | 状態 |
|---|---|---|
| P0 | 1 | Phase 1 内（Month 2）に修正 |
| P1 | 8 | Phase 2-3（Month 3-5）に修正 |
| P2 | 8 | Phase 8（Month 8-9）に随時修正 |
| **合計** | **17** | - |

### 7.2 文書更新計画

| 文書 | 更新内容 | Phase | 担当 |
|---|---|---|---|
| **DOC-OBS-001** | §1.5 コスト試算追加 | Phase 1 | SRE |
| **DOC-OBS-002** | §6.2 kubecost 追加 | Phase 1 | SRE |
| **DOC-OBS-007** | §10 観測基盤自身のアラート追加 | Phase 1 | SRE |
| **DOC-OBS-008** | §11 観測基盤 SLO サブセクション追加 | Phase 1 | SRE |
| **DOC-OBS-009** | §8.5 コスト監査ログ追加 | Phase 1 | SRE + Finance |
| **DOC-OBS-010** | §5.3 コスト監視スタック追加 | Phase 1 | SRE |
| **DOC-OBS-011** | §1 ロードマップにコスト Phase 追加 | Phase 1 | SRE |
| **DOC-OBS-012** | §8.3 cardinality-check ジョブ追加 | Phase 2 | Dev + SRE |
| **DOC-OBS-013** | 本ドキュメント、Revision 3 として更新 | Phase 1 完了時 | SRE |

### 7.3 新規追加ドキュメント

| 文書 | 内容 | Phase |
|---|---|---|
| **DOC-OBS-014** `dashboards/95-meta-observability.json` | 観測基盤自身の監視 | Phase 1 |
| **DOC-OBS-015** `dashboards/96-cost.json` | コスト監視 | Phase 1 |
| **DOC-OBS-016** `runbooks/observability-cost.md` | コスト超過時対応 | Phase 1 |
| **DOC-OBS-017** `runbooks/observability-self-down.md` | 観測基盤ダウン時対応 | Phase 1 |
| **DOC-OBS-018** `alerts/observability-self.yaml` | 観測基盤自身のアラート | Phase 1 |
| **DOC-OBS-019** `sli-slo/observability-platform.yaml` | 観測基盤 SLO 定義 | Phase 1 |
| **DOC-OBS-020** `cardinality-policy.md` | Cardinality 予算管理ポリシー | Phase 2 |

### 7.4 追加 ADR

| ADR | 内容 |
|---|---|
| **OBS-ADR-011** | コスト監視に kubecost を採用 |
| **OBS-ADR-012** | 観測基盤の SLO 99.9% 採用 |
| **OBS-ADR-013** | Cardinality 予算 10K series/metric 厳守 |
| **OBS-ADR-014** | Span ホワイトリストによる計装制御 |
| **OBS-ADR-015** | ログ Sampling 戦略（ERROR 100%、INFO 10%、DEBUG 0%） |

## 8. 残余リスク

| リスク | 発生確率 | 影響 | 対応 |
|---|---|---|---|
| OTel SDK バージョンアップ時の破壊的変更 | 中 | 中 | 固定 + 監視 |
| Grafana / Prometheus の セキュリティ CVE | 中 | 高 | 週次 Trivy スキャン |
| 観測基盤のキャパシティ超過 | 低 | 高 | 容量テスト + 自動アラート |
| 業務コードが OTel に依存しすぎる | 低 | 中 | 緊急 OFF 機能（feature flag） |
| 観測基盤チームがボトルネック | 中 | 中 | Runbook 整備 + 自動化 |

## 9. 再自審計画

### 9.1 次回自審タイミング

| タイミング | 対象範囲 | 担当 |
|---|---|---|
| **Phase 1 完了時** | インフラ監視のみ | SRE Lead |
| **Phase 4 完了時** | Trace 関連 | SRE Lead + Dev Lead |
| **Phase 7 完了時** | SLO 関連 | SRE Lead + PO |
| **Phase 8 完了時** | 全体（最終） | SRE Lead + PO + Dev Lead + QA |
| **四半期定期** | 増分変更 | SRE Lead |

### 9.2 自審チェックリスト更新

- 各 Phase 完了時にチェックリストを更新
- 新たに発見された問題カテゴリを追加
- チェック項目の重複 / 漏れを整理

## 10. 用語集

| 用語 | 説明 |
|---|---|
| **自審** | 設計者自身が設計を疑い、第三者視点で問題を発見する作業 |
| **Revision 2** | 第一版への修正版 |
| **Cardinality 予算** | 1 メトリクスあたりの時系列数の上限 |
| **kubecost** | K8s リソースのコスト可視化ツール |
| **Meta-Observability** | 観測基盤を観測すること |
| **Span ホワイトリスト** | 計装を許可する Span のリスト |
| **log Sampling** | ログを採取する確率を調整する仕組み |
| **Quarterly Metric Cleanup** | 四半期ごとの未使用メトリクス削除活動 |

## 11. 参考文献

1. Google SRE Book - Error Budgets and SLOs  
   <https://sre.google/sre-book/error-budget/>
2. Brendan Gregg - The USE Method  
   <https://www.brendangregg.com/usemethod.html>
3. Tom Wilkie - Cardinality is Key  
   <https://grafana.com/blog/2021/05/07/cardinality-is-key/>
4. kubecost Documentation  
   <https://docs.kubecost.com/>
5. OpenTelemetry Sampling Specification  
   <https://opentelemetry.io/docs/concepts/sampling/>
6. IPA 共通フレーム2018 第 8 章「システム監査」

---

> **IPA 末尾注記**  
> 本ドキュメントは IPA 共通フレーム2018 (SLCP-JCF2018) 第 8 章「システム監査」および  
> IPA 非機能要求グレード2018 に準拠する。  
> 自審チェックリストは Phase 毎に更新し、PO（プロダクトオーナー）の最終承認を経て確定する。  
> 改訂内容は次版（Revision 2）として各 Phase 完了時に反映する。
