# 新システム構成（To-Be System Architecture）

> **本文件の目的**：[DOC-UP-006 To-Be 業務](06-to-be-business.md) を支える**新システムのアーキテクチャ**を定義する。  
> 関連 IPA 工程: 09（新業務設計 / To-Be）+ 22-24（基本設計のインプット）。

> **ドキュメントID**：DOC-UP-007
> **文書分類**：上流工程文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/upstream/README.md`](README.md)
> **下位文書**：[`docs/architecture/00-anatomy-model.md`](../architecture/00-anatomy-model.md)、[`docs/architecture/01-tech-stack.md`](../architecture/01-tech-stack.md)
> **関連文書**：
> - [To-Be 業務](06-to-be-business.md)
> - [課題一覧](05-issue-list.md)
> - [初期リスク評価](08-initial-risk-assessment.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - IPA「非機能要求グレード2018」

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 09 + 22-24 に対応） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. アーキテクチャビジョン
2. システム全体像
3. 16 モジュール構成
4. 技術スタック
5. データフロー
6. 性能・可用性目標
7. セキュリティ設計
8. 運用設計
9. 用語集
10. 参考文献

---

## 1. アーキテクチャビジョン

詳細: [DOC-ARCH-001 仿生モデル](../architecture/00-anatomy-model.md)

> **「骨・血・神経・筋肉」の 4 層アーキテクチャ** で生体のように自己修復・自己拡張するシステムを構築する。

| 層 | 役割 | モジュール |
|---|---|---|
| 骨（Skeleton） | 静的構造・ストレージ | M-10（テナント）, M-16（クラスタ） |
| 血（Blood） | データの流れ | M-01, M-02, M-03, M-09 |
| 神経（Nerve） | イベント・制御 | M-15（イベントバス）, M-04, M-05, M-08 |
| 筋肉（Muscle） | 動的拡張・実行 | M-06, M-12, M-14, M-13 |

## 2. システム全体像

```
┌─────────────────────────────────────────────┐
│       Client (Browser)                       │
│       M-12 Canvas Editor (Bevy WASM)         │
└──────────────────┬──────────────────────────┘
                   │ WebSocket + REST
┌──────────────────▼──────────────────────────┐
│       M-13 API Gateway (Rust)                │
│       - 認証 / 認可 / レート制限              │
└─────┬─────────┬──────────┬──────────┬────────┘
      │         │          │          │
      ▼         ▼          ▼          ▼
┌─────────┐ ┌──────┐ ┌──────┐ ┌────────────┐
│ M-11    │ │M-04  │ │M-14  │ │ M-15        │
│ RBAC +  │ │Orche-│ │Mod-  │ │  Central    │
│ Collab  │ │str.  │ │Reg.  │ │  Event Bus  │
└─────────┘ └──┬───┘ └──────┘ └─────┬──────┘
               │                    │
      ┌────────┼────────────────────┼────────┐
      │        │                    │        │
      ▼        ▼                    ▼        ▼
┌─────────┐ ┌──────┐ ┌─────────────┐ ┌──────┐
│ M-05    │ │M-08  │ │ M-01         │ │M-02  │
│ Control │ │Trig- │ │ Acq. Adapter │ │Norm- │
│ Flow    │ │ger   │ │              │ │alizer│
└────┬────┘ └──────┘ └──────┬──────┘ └──┬───┘
     │                       │            │
     └───────────┬───────────┴────────────┘
                 ▼
         ┌───────────────┐
         │   M-03 Data   │
         │  Flow Engine  │
         └───────┬───────┘
                 ▼
         ┌───────────────┐
         │   M-09 Exp.   │
         └───────┬───────┘
                 │
                 ▼
   ┌─────────────────────────────┐
   │ M-10 Tenant Middleware       │
   │ - 11 Tables + RLS            │
   │ - 6 PL/pgSQL 存過            │
   │ - audit_log (1 年)           │
   └─────────────────────────────┘
                 │
                 ▼
        ┌────────────────┐
        │ M-07 Debug     │
        │ M-16 Cluster   │
        └────────────────┘
```

## 3. 16 モジュール構成

| ID | モジュール | 層 | 責務 |
|---|---|---|---|
| M-01 | 取得アダプタ | 血 | データソース接続、CDC、Push 受信 |
| M-02 | 標準化 | 血 | NJSON スキーマ統一 |
| M-03 | データフロー | 血 | キャンバス実行エンジン |
| M-04 | オーケストレーション | 神経 | パイプライン制御 |
| M-05 | 制御フロー | 神経 | 条件分岐、ループ |
| M-06 | ノード SDK | 筋肉 | プラグイン開発 |
| M-07 | デバッグ | 筋肉 | トレース、ブレークポイント |
| M-08 | トリガー | 神経 | スケジュール、Webhook |
| M-09 | エクスポータ | 血 | 外部出力 |
| M-10 | テナント | 骨 | RLS、ストレージ |
| M-11 | RBAC | 骨 | 認証、認可 |
| M-12 | Canvas Editor | 筋肉 | UI（Bevy WASM） |
| M-13 | API Gateway | 骨 | ルーティング、認証 |
| M-14 | モジュール登録 | 筋肉 | 動的ロード、hot-swap |
| M-15 | 中央イベントバス | 神経 | イベント配信、Pub/Sub |
| M-16 | クラスタ調整 | 骨 | リーダー選出、シャード |

## 4. 技術スタック

| レイヤ | 技術 | バージョン | 理由 |
|---|---|---|---|
| Frontend | Bevy（ECS） | 0.14+ | Rust 統一、WASM 親和 |
| Backend | Rust | 1.74+ | 性能 + 安全性 |
| HTTP | axum | 0.7+ | Tokio エコシステム |
| WebSocket | tokio-tungstenite | latest | 標準的 |
| DB | PostgreSQL | 16+ | RLS、PL/pgSQL、JSONB |
| Cache | Redis | 7+ | 速度 |
| 監視 | Prometheus + Grafana | latest | 標準 |
| ログ | OpenTelemetry | latest | ベンダ中立 |
| CI | GitHub Actions | — | 標準 |
| K8s | Kubernetes | 1.28+ | 標準 |
| コンテナ | Docker | 24+ | 標準 |

詳細は [DOC-ARCH-002 技術スタック](../architecture/01-tech-stack.md)、[DOC-ARCH-007 Rust crate 選択](../architecture/06-rust-tech-selection.md) 参照。

## 5. データフロー

| 段階 | 入力 | 処理 | 出力 | レイテンシ目標 |
|---|---|---|---|---|
| 取得 | 業務 API | Adapter | raw メッセージ | < 100ms |
| 標準化 | raw | Normalizer | NJSON | < 50ms |
| フロー実行 | NJSON | Engine | 中間結果 | < 200ms |
| イベント配信 | 内部 | Bus | 購読者 | < 50ms |
| エクスポート | 結果 | Exporter | 外部 | < 1s |
| 全体（取得→エクスポート） | | | | < 1s |

## 6. 性能・可用性目標

| 項目 | 目標 | NF 区分 |
|---|---|---|
| 可用性 SLA | 99.9% | [NF-AVA] 必須 |
| 起動時間 | < 3s | [NF-PER] 必須 |
| 1k node 操作レイテンシ | < 100ms | [NF-PER] 必須 |
| 同時編集ユーザー | 100 | [NF-PER] 必須 |
| テナント数 | 10,000 | [NF-PER] 推奨 |
| ノード数/画布 | 10,000 | [NF-PER] 必須 |
| MTTR | < 30min | [NF-OPS] 必須 |
| バックアップ RPO | < 5min | [NF-MIG] 必須 |
| 切替時間 | < 5min | [NF-MIG] 必須 |

## 7. セキュリティ設計

詳細は [DOC-ARCH-003 横断リスク §3](../architecture/03-cross-cutting-risks.md) 参照。

| 項目 | 目標 | NF 区分 |
|---|---|---|
| 通信暗号化 | TLS 1.3 | [NF-SEC] 必須 |
| 保存時暗号化 | AES-256 (KMS) | [NF-SEC] 必須 |
| 認証 | JWT (15 分) + Refresh | [NF-SEC] 必須 |
| 認可 | RBAC + ABAC | [NF-SEC] 必須 |
| 監査ログ | 1 年保存、改ざん検知 | [NF-SEC] 必須 |
| 脆弱性 SLA | Critical 24h, High 72h | [NF-SEC] 必須 |
| GDPR / PIPL | 全対応 | [NF-SEC] 必須 |

## 8. 運用設計

| 項目 | 目標 | NF 区分 |
|---|---|---|
| デプロイ方式 | atomic swap | [NF-MIG] 必須 |
| 監視 | 100% 計装 | [NF-OPS] 必須 |
| ログ | 構造化、相関 ID | [NF-OPS] 必須 |
| バックアップ | 日次 + WAL | [NF-AVA] 必須 |
| DR | 別リージョン RPO < 5min | [NF-AVA] 必須 |
| OS 対応 | 3 OS | [NF-ENV] 必須 |

## 9. 用語集

| 用語 | 説明 |
|---|---|
| アーキテクチャビジョン | システム設計の大局的な方向性 |
| モジュール | システム内の機能単位 |
| NF | Non-Functional（非機能） |
| CDC | Change Data Capture |
| KMS | Key Management Service |

## 10. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
2. IPA「非機能要求グレード2018」、2018 年 4 月
3. Ada プロジェクトチーム「[DOC-ARCH-001 仿生モデル](../architecture/00-anatomy-model.md)」、2026-08-19
4. Ada プロジェクトチーム「[DOC-ARCH-002 技術スタック](../architecture/01-tech-stack.md)」、2026-08-19
5. Ada プロジェクトチーム「[DOC-UP-006 To-Be 業務](06-to-be-business.md)」、2026-08-20

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
