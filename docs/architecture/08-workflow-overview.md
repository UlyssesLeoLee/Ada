# IPA 共通フレーム2018 ワークフロー全体俯瞰

> **本文件の目的**：IPA「共通フレーム2018」(SLCP-JCF2018) が定義する 150 工程を、本プロジェクト（Ada 无限画布跨平台数据集成系统）に**マッピング**する。各工程について **(a) 関連ドキュメント、(b) 現状ステータス、(c) 入口/出口基準、(d) 担当ロール、(e) 想定成果物** を一覧化する。  
> 本書は「実装着手判定」「マイルストーン管理」「監査証跡」の単一情報源（Single Source of Truth）として機能する。  
> 設計レビュー・進捗報告・受入判定・リリース判定（Go/No-Go）すべて本書 §5/§6/§7 を引用する。

> **ドキュメントID**：DOC-ARCH-009
> **文書分類**：横断文書
> **バージョン**：v1.1.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/architecture/00-anatomy-model.md`（DOC-ARCH-001）、`docs/architecture/01-tech-stack.md`（DOC-ARCH-002）
> **下位文書**：全モジュール文書（DOC-MOD-001〜016）、全 API 文書（DOC-API-001〜006）、全テスト文書（DOC-TST-001〜003 / DOC-ACC-001）、**工程別テンプレート集 [`docs/templates/`](../templates/README.md)（DOC-TPL-INDEX + DOC-TPL-REV/TST/PRC/RBK/OPS/CHG/QUA/CLO）**
> **関連文書**：
> - `docs/architecture/03-cross-cutting-risks.md`（DOC-ARCH-004，本書は「リスク」を補完）
> - `docs/architecture/07-qa-register.md`（DOC-ARCH-008，本書は「未決/仮定」を補完）
> - `docs/templates/README.md`（DOC-TPL-INDEX，本書の ⚪ 工程 80 件を 62 テンプレートでカバー）
> - `docs/CHANGELOG.md`（DOC-CHG-001，変更履歴の集約）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」第 5 章「プロセス」、第 6 章「システム開発プロセス」、第 7 章「ソフトウェア実装プロセス」、第 8 章「保守・運用プロセス」
> - IPA「非機能要求グレード2018」
> - JIS X 0160:2012「ソフトウェアライフサイクルプロセス」
> **機密区分**：社内
> **言語**：中文（简体）／メタデータは日本語

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 共通フレーム2018 の 150 工程を本プロジェクト既存ドキュメントにマッピング。16 カテゴリ × ステータス × 関連文書 × RACI を 1 冊に集約） | Ada プロジェクトチーム | TBD | TBD |
| v1.1.0 | 2026-08-20 | [`docs/templates/`](../templates/README.md)（62 テンプレート）と連携。⚪ 工程 25 件の関連文書列を新テンプレート参照に更新、§1.4「テンプレート集との関係」追加、§4 凡例に 🟣 雛形完成 追加 | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 概要と目的
2. 適用範囲と前提
3. IPA 共通フレーム2018 全体俯瞰
4. ステータス凡例
5. フェーズ一覧（150 フェーズ統合表）
6. カテゴリ別詳細（16 カテゴリ）
7. ゲート / マイルストーン定義
8. ロール / 責任分担表（RACI）
9. 監査 / チェックポイント
10. リスク / 前提 / 制約
11. 現在のクリティカルパス
12. 用語集
13. 参考文献

---

## 1. 概要と目的

### 1.1 位置付け

| 比較軸 | 本書 (DOC-ARCH-009) | DOC-ARCH-004 横断リスク | DOC-ARCH-008 QA 登録簿 | DOC-TPL-INDEX テンプレート集 | DOC-CHG-001 CHANGELOG |
|---|---|---|---|---|---|
| 焦点 | **いつ・誰が・何を作る**（工程） | 既知の **リスク** | 未知の **未決/仮定** | **どう記録する**（空フォーム） | **過去変更** |
| 単位 | フェーズ（150） | リスク項目 | Q&A 項目 | 62 テンプレート | 改訂エントリ |
| 用途 | 進捗 / ゲート判定 / RACI | リスク監視 | 実装前オープン Q&A | 各工程の証跡作成 | 何がいつ変わったか |
| 視点 | 時系列（プロセス） | 横串（リスク） | 実装前スナップ | 実行時の雛形 | 履歴 |

### 1.2 利用シーン

1. **スプリント計画**：§5/§6 を参照し「次着手フェーズ」を確定
2. **週次進捗報告**：§7 マイルストーンに対し現状を●/▲/× で報告
3. **ゲート判定（基本設計レビュー / 詳細設計レビュー / 受入判定 / Go-Live 判定）**：§7 該当ゲートの判定基準を使用
4. **監査対応**：§9 監査チェックポイント一覧と証跡
5. **新規参加者オンボーディング**：§3 全体像 → §5 一覧 → §6 詳細、の順で熟読

### 1.3 関連標準との対応

| IPA 共通フレーム2018 章 | 本書セクション | 備考 |
|---|---|---|
| 第 5 章 プロセス | §3, §5, §6 | 全プロセスの俯瞰 |
| 第 6 章 システム開発プロセス | §6.2〜§6.10（要件定義〜移行） | 開発の上流〜下流 |
| 第 7 章 ソフトウェア実装プロセス | §6.5, §6.6, §6.7, §6.8 | 実装・試験 |
| 第 8 章 保守・運用プロセス | §6.12, §6.13 | 運用・保守 |
| 第 9 章 サービスマネジメント | §6.11 | リリース・サービス開始 |
| 第 10 章 ファシリテーション | §6.14, §6.15, §6.16 | 品質・プロジェクト管理・終結 |

### 1.4 テンプレート集との関係

本書の ⚪（未着手）/🟡（設計完了）工程は、[`docs/templates/README.md`](../templates/README.md)（**DOC-TPL-INDEX**）の **62 テンプレート** でカバーする。各テンプレートは IPA 工程番号を明示し、実行時に派生版を作成して証跡とする。

| テンプレートファイル | DOC-ID | 対応 IPA 工程 | 派生版保管先 |
|---|---|---|---|
| [01-reviews.md](../templates/01-reviews.md) | DOC-TPL-REV | 20, 41, 52, 61, 89, 94, 103, 145 | `docs/records/reviews/` |
| [02-tests-execution.md](../templates/02-tests-execution.md) | DOC-TPL-TST | 60, 62-75, 78-88, 92-95 | `docs/records/tests/` |
| [03-process-management.md](../templates/03-process-management.md) | DOC-TPL-PRC | 132-142 | `docs/records/process/` |
| [04-runbooks.md](../templates/04-runbooks.md) | DOC-TPL-RBK | 53, 68, 97-101, 105-108 | `docs/runbooks/` |
| [05-operations.md](../templates/05-operations.md) | DOC-TPL-OPS | 109-117 | `docs/records/ops/` |
| [06-change-management.md](../templates/06-change-management.md) | DOC-TPL-CHG | 118-126 | `docs/records/changes/` |
| [07-quality.md](../templates/07-quality.md) | DOC-TPL-QUA | 128-130 | `docs/records/quality/` |
| [08-closure.md](../templates/08-closure.md) | DOC-TPL-CLO | 146-150 | `docs/records/closure/` |

§5 の各フェーズの「関連文書」列に本テンプレートの参照を順次追記する（[§11 クリティカルパス](#11-現在のクリティカルパス) 参照）。

---

## 2. 適用範囲と前提

### 2.1 適用範囲

- **対象システム**：Ada 无限画布跨平台数据集成系统 v1.x 系（[DOC-ARCH-001](../architecture/00-anatomy-model.md) 参照）
- **対象フェーズ**：超上流（01）〜 終結（150）の **全 150 工程**
- **対象読者**：
  - **R（実行）**：Rust 開発者 16 crate 担当、SRE、QA エンジニア
  - **A（説明責任）**：プロジェクトマネージャ、アーキテクト、テックリード
  - **C（協議）**：DBA、SRE マネージャ、セキュリティオフィサ
  - **I（報告先）**：プロダクトオーナー、経営層、監査担当

### 2.2 前提

1. IPA「共通フレーム2018」の 150 工程を**再解釈せず** 1:1 で採用する。工程の追加・統合は行わない。
2. 既存の [DOC-ARCH-002](../architecture/01-tech-stack.md)、[DOC-ARCH-007](../architecture/06-rust-tech-selection.md)、[DOC-ARCH-008](../architecture/07-qa-register.md) は本書の補完文書として併存する。
3. ステータスは本書制定日（2026-08-20）時点のもの。実装着手後は毎週更新する。
4. ドキュメント未作成の工程は「⚪ 未着手」とマークし、関連文書欄は `—` とする。

### 2.3 非機能要求タグ

| NF 区分 | 適用工程 | 必須/推奨 |
|---|---|---|
| `[NF-AVA]` 可用性 | 02, 37, 38, 85, 86, 87, 109, 110, 113 | 必須 |
| `[NF-PER]` 性能 | 26, 32, 80, 81, 82, 88 | 必須 |
| `[NF-OPS]` 運用・保守性 | 24, 35, 37, 38, 39, 50, 51, 109, 111, 112, 122, 123, 124, 125 | 必須 |
| `[NF-MIG]` 移行性 | 40, 96, 97, 98, 99, 100, 101, 102 | 必須 |
| `[NF-SEC]` セキュリティ | 17, 33, 34, 83, 84, 105, 114, 115, 123 | 必須 |
| `[NF-ENV]` 環境 | 35, 36, 41, 52, 88, 104 | 必須 |

---

## 3. IPA 共通フレーム2018 全体俯瞰

### 3.1 プロセス分類（16 カテゴリ）

```
[01] 超上流 (01-09)         経営要求確認、システム化構想、立上げ、As-Is/To-Be 調査・分析
    ↓
[02] 要件定義 (10-21)       UR/BR/SR/FR/NFR/データ/IF/セキュリティ/運用/移行 要件 → ベースライン化
    ↓
[03] 基本設計 (22-41)       方式設計、アーキテクチャ、機能/画面/API/DB/IF/バッチ/権限/セキュリティ/インフラ/NW/運用/監視/BK/移行 → BD Review
    ↓
[04] 詳細設計 (42-52)       プログラム構造/モジュール/クラス/ロジック/API/DB/SQL/バッチ/エラー/ログ → DD Review
    ↓
[05] 実装 (53-58)           環境構築、PG、SAST、CR、Build、CI
    ↓
[06] 単体試験 (59-65)       UT 計画/仕様/レビュー/実施/修正/Retest/承認
    ↓
[07] 結合試験 (66-75)       IT 計画/仕様/環境構築/ITa・ITb・API・DB・外部連携/障害/回帰
    ↓
[08] システム試験 (76-89)   ST 計画/仕様/機能/シナリオ/性能/負荷/ストレス/セキュリティ/障害/復旧/BK/可用性/運用/ST 完了承認
    ↓
[09] 受入試験 (90-95)       UAT 計画/仕様/実施/業務シナリオ/受入判定/検収
    ↓
[10] 移行 (96-101)          計画/手順/リハーサル/データ移行/システム移行/結果確認
    ↓
[11] リリース (102-108)     計画/判定/本番環境構築/デプロイ/Smoke/Go-Live/Hypercare
    ↓
[12] 運用 (109-117)         引継ぎ/監視/ジョブ/BK/キャパ/Incident/障害/問題/Support
    ↓
[13] 保守 (118-126)         CR/影響分析/変更/CM/Patch/Vulnerability/改修/Hotfix/回帰
    ↓
[14] 品質管理 (127-130)     QA Plan/QA Review/QA 評価/QA 監査
    ↓
[15] 管理 (131-144)         PJ Plan/WBS/進捗/課題/Risk/変更/CM/Deliverable/Review/Meeting/工数/Cost/Scope/Baseline
    ↓
[16] 終結 (145-150)         完了判定/引渡し/完了報告/Retrospective/KT/Archive
```

### 3.2 全体俯瞰図（テキスト版）

```
┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  超上流       │→ │  要件定義     │→ │  基本設計     │→ │  詳細設計     │
│  01-09       │  │  10-21        │  │  22-41        │  │  42-52        │
│  概念・立上げ │  │  UR/BR/SR/FR  │  │  SA/Arch/機能 │  │  構造/ロジック│
│  As-Is/To-Be  │  │  NFR/Data/IF  │  │  UI/API/DB    │  │  API/DB/SQL   │
│              │  │  セキュリティ │  │  権限/Infra   │  │  Err/Log      │
└──────┬───────┘  │  運用/移行    │  │  運用/監視/BK │  │               │
       │          │  RD Review    │  │  BD Review    │  │  DD Review    │
       │          │  Baseline化   │  │               │  │               │
       │          └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │                 │
       │                 └────────┬────────┘                 │
       │                          ↓                          ↓
       │                  ┌──────────────┐          ┌──────────────┐
       │                  │  実装         │          │  設計レビュー │
       │                  │  53-58        │          │  (BD/DD)     │
       │                  │  Env/PG/SAST  │          │  41, 52      │
       │                  │  CR/Build/CI  │          └──────────────┘
       │                  └──────┬───────┘
       │                         ↓
       │                  ┌──────────────┐
       │                  │  単体試験     │
       │                  │  59-65        │
       │                  │  Plan/仕様/   │
       │                  │  実施/承認    │
       │                  └──────┬───────┘
       │                         ↓
       │                  ┌──────────────┐
       │                  │  結合試験     │
       │                  │  66-75        │
       │                  │  ITa/ITb/API/ │
       │                  │  DB/外部/回帰 │
       │                  └──────┬───────┘
       │                         ↓
       │                  ┌──────────────┐
       │                  │  システム試験 │
       │                  │  76-89        │
       │                  │  機能/シナリオ│
       │                  │  PT/Load/     │
       │                  │  Stress/Sec/  │
       │                  │  DR/BK/OT     │
       │                  └──────┬───────┘
       │                         ↓
       │                  ┌──────────────┐
       │                  │  受入試験     │
       │                  │  90-95        │
       │                  │  UAT/業務/    │
       │                  │  判定/検収    │
       │                  └──────┬───────┘
       │                         ↓
       │                  ┌──────────────┐
       │                  │  移行         │
       │                  │  96-101       │
       │                  │  計画/リハ/   │
       │                  │  Data/本番/   │
       │                  │  結果確認     │
       │                  └──────┬───────┘
       │                         ↓
       │                  ┌──────────────┐         ┌──────────────┐
       │                  │  リリース     │         │  品質管理     │
       │                  │  102-108      │         │  127-130      │
       │                  │  計画/判定/   │         │  Plan/Review/ │
       │                  │  Deploy/Smk/  │         │  評価/監査    │
       │                  │  Go-Live/     │         └──────────────┘
       │                  │  Hypercare    │                  ↑
       │                  └──────┬───────┘                  │
       │                         ↓                          │
       │                  ┌──────────────┐         ┌──────────────┐
       │                  │  運用         │←────────│  管理         │
       │                  │  109-117      │         │  131-144      │
       │                  │  引継ぎ/監視  │         │  PJ/WBS/進捗/ │
       │                  │  ジョブ/BK/   │         │  課題/Risk/   │
       │                  │  Cap/Incident │         │  CM/Review/   │
       │                  └──────┬───────┘         │  コスト等     │
       │                         │                 └──────────────┘
       │                         ↓
       │                  ┌──────────────┐
       │                  │  保守         │
       │                  │  118-126      │
       │                  │  CR/変更/     │
       │                  │  Patch/Vuln/  │
       │                  │  改修/回帰    │
       │                  └──────┬───────┘
       │                         ↓
       │                  ┌──────────────┐
       │                  │  終結         │
       │                  │  145-150      │
       │                  │  判定/引渡/   │
       │                  │  報告/振返/   │
       │                  │  KT/Archive   │
       │                  └──────────────┘
       ↑                                                 ↑
       └─────────────── 全期間並行 ─────────────────────┘
```

---

## 4. ステータス凡例

| 記号 | 意味 | 該当数（本制定日時点：2026-08-20） |
|---|---|---|
| ✅ **完了** | 設計・実装・運用すべて完了し承認済み | 0 |
| 🟢 **実装中** | コード記述・試験実施中 | 0 |
| 🟡 **設計完了・実行待ち** | ドキュメント完成、実装/実施未着手（QA 登録簿 [DOC-ARCH-008](../architecture/07-qa-register.md) §5 解消が前提） | 約 70 |
| 🟣 **雛形完成** | ⚪ → 🟣：[`docs/templates/`](../templates/README.md) でテンプレート完成、実行時に派生版を作成（2026-08-20 付で 62 テンプレート整備完了） | 約 62 |
| ⚪ **未着手** | 計画も未策定（残 14 件 = 設計不要 / 設計で充足） | 約 14 |
| 🚧 **計画中** | 一部計画・一部完了 | 5 |
| ⊘ **対象外** | 本システムでは実施しない（理由を備考に記載） | 0 |

**集計（2026-08-20 改訂後）**：150 フェーズ中、設計完了（🟡）= 70、雛形完成（🟣）= 62、未着手（⚪）= 14、計画中（🚧）= 5。**未着手 ⚪ 14 件は 設計で充足できる工程**（例：要件承認は CHANGELOG で記録済、構成管理は Git + Cargo.lock で自動化 等）。  
実装着手判定は [DOC-ARCH-008](../architecture/07-qa-register.md) §5 の P0 解消後となる。各 ⚪/🟡 工程は対応する [DOC-TPL-NNN](../templates/README.md) を派生して実行する。

---

## 5. フェーズ一覧（150 フェーズ統合表）

> 150 行を 16 カテゴリ別に集約。各行の「関連文書」は本プロジェクト内に**実在する DOC-ID** のみ記載。`—` は未作成。**略称**は IPA 共通フレームの慣用略称。**状態**は §4 凡例に従う。

### 5.1 超上流（01-09）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 01 | 経営要求確認 | — | [DOC-REQ-001](../legacy/requirements.md) §1, §2 | 🟡 | PO | — |
| 02 | システム化構想 | — | [DOC-REQ-001](../legacy/requirements.md) §3, §4 | 🟡 | PO + アーキ | [NF-AVA] |
| 03 | システム化計画 | — | [DOC-ARCH-001](../architecture/00-anatomy-model.md), [DOC-ARCH-002](../architecture/01-tech-stack.md) | 🟡 | アーキ + PM | — |
| 04 | 企画 | — | [DOC-REQ-001](../legacy/requirements.md) §1 | 🟡 | PO | — |
| 05 | プロジェクト立上げ | PJ立上げ | [DOC-CHG-001](CHANGELOG.md), [README](README.md) | 🟡 | PM | — |
| 06 | 現行業務調査 | As-Is | [DOC-REQ-001](../legacy/requirements.md) §5 | 🟡 | Biz + PO | — |
| 07 | 現行システム調査 | As-Is | [DOC-REQ-001](../legacy/requirements.md) §5 | 🟡 | Biz + アーキ | — |
| 08 | 課題分析 | — | [DOC-REQ-001](../legacy/requirements.md) §6, [DOC-ARCH-004](../architecture/03-cross-cutting-risks.md) | 🟡 | Biz + アーキ | — |
| 09 | 新業務設計 | To-Be | [DOC-REQ-001](../legacy/requirements.md) §6, [DOC-ARCH-001](../architecture/00-anatomy-model.md) | 🟡 | Biz + PO + アーキ | — |

### 5.2 要件定義（10-21）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 10 | ユーザー要求定義 | UR | [DOC-REQ-001](../legacy/requirements.md) §7 | 🟡 | PO + Biz | — |
| 11 | 業務要件定義 | BR | [DOC-REQ-001](../legacy/requirements.md) §7 | 🟡 | Biz + PO | — |
| 12 | システム要件定義 | SR | [DOC-REQ-001](../legacy/requirements.md) §8, [DOC-ARCH-001](../architecture/00-anatomy-model.md) | 🟡 | アーキ | — |
| 13 | 機能要件定義 | FR | [DOC-REQ-001](../legacy/requirements.md) §9, [DOC-MOD-001〜016](../modules/M-01-acquisition-adapter.md) §1 | 🟡 | アーキ + Dev | — |
| 14 | 非機能要件定義 | NFR | [DOC-REQ-001](../legacy/requirements.md) §10, [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) | 🟡 | アーキ + SRE | [NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV] 必須 |
| 15 | データ要件定義 | — | [DOC-MOD-010](../modules/M-10-tenant-middleware.md) §3, [DOC-ARCH-001](../architecture/00-anatomy-model.md) | 🟡 | DBA + アーキ | — |
| 16 | 外部インターフェース要件定義 | IF | [DOC-API-001](../api/rest-endpoints.md), [DOC-MOD-001](../modules/M-01-acquisition-adapter.md) §2 | 🟡 | アーキ + Biz | [NF-SEC] |
| 17 | セキュリティ要件定義 | — | [DOC-MOD-011](../modules/M-11-rbac-collab.md), [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) §3 | 🟡 | SecO + アーキ | [NF-SEC] 必須 |
| 18 | 運用要件定義 | — | [DOC-ARCH-004](../architecture/04-atomic-deployment.md), [DOC-ARCH-005](../architecture/05-admin-operations-ui.md) | 🟡 | SRE + PM | [NF-OPS] |
| 19 | 移行要件定義 | — | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2, [DOC-ARCH-003](../architecture/02-deployment.md) | 🟡 | アーキ + SRE | [NF-MIG] |
| 20 | 要件レビュー | RD Review |[DOC-TPL-REV §A.1](../templates/01-reviews.md#a1-要件レビューチェックリストipa-工程-20--g1)| ⚪ | PO + アーキ + PM | — |
| 21 | 要件承認・ベースライン化 | Baseline | [DOC-CHG-001](CHANGELOG.md) v1.0.0（要件定義書 v1.2.1 を baseline 化済） | 🟡 | PO | — |

### 5.3 基本設計（22-41）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 22 | システム方式設計 | SA | [DOC-ARCH-001](../architecture/00-anatomy-model.md), [DOC-ARCH-002](../architecture/01-tech-stack.md) | 🟡 | アーキ | — |
| 23 | ソフトウェア方式設計 | — | [DOC-ARCH-002](../architecture/01-tech-stack.md), [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) | 🟡 | アーキ | — |
| 24 | アーキテクチャ設計 | Architecture | [DOC-ARCH-001](../architecture/00-anatomy-model.md), [DOC-ARCH-002](../architecture/01-tech-stack.md), [DOC-ARCH-004](../architecture/04-atomic-deployment.md) | 🟡 | アーキ | [NF-OPS] |
| 25 | 機能設計 | — | [DOC-MOD-001〜016](../modules/M-01-acquisition-adapter.md) §2 | 🟡 | アーキ + Dev | — |
| 26 | 画面設計 | UI | [DOC-MOD-012](../modules/M-12-canvas-editor-frontend.md) §2, [DOC-ARCH-005](../architecture/05-admin-operations-ui.md) | 🟡 | FE | [NF-PER] |
| 27 | 帳票設計 | — | —（PDF/CSV 出力は M-09 の API レスポンスとして実装） | ⊘ | — | — |
| 28 | API設計 | API | [DOC-API-001](../api/rest-endpoints.md), [DOC-API-002](../api/websocket-events.md), [DOC-API-004](../api/admin-modules.md), [DOC-API-005](../api/admin-events.md), [DOC-API-006](../api/admin-cluster.md) | 🟡 | アーキ + Dev | — |
| 29 | 外部インターフェース設計 | IF | [DOC-MOD-001](../modules/M-01-acquisition-adapter.md) §2, [DOC-MOD-014](../modules/M-14-module-registry.md) §2 | 🟡 | アーキ | — |
| 30 | データベース基本設計 | DB | [DOC-MOD-010](../modules/M-10-tenant-middleware.md) §4 | 🟡 | DBA + アーキ | — |
| 31 | データモデル設計 | ER | [DOC-MOD-010](../modules/M-10-tenant-middleware.md) §4.1〜4.5 | 🟡 | DBA | — |
| 32 | バッチ設計 | Batch | [DOC-MOD-015](../modules/M-15-central-event-bus.md) §2.5（Outbox パターン）, [DOC-MOD-008](../modules/M-08-trigger-service.md) §2 | 🟡 | アーキ | [NF-PER] |
| 33 | 権限設計 | — | [DOC-MOD-011](../modules/M-11-rbac-collab.md) §2, [DOC-MOD-013](../modules/M-13-api-gateway.md) §2 | 🟡 | SecO | [NF-SEC] |
| 34 | セキュリティ設計 | — | [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) §3, [DOC-MOD-011](../modules/M-11-rbac-collab.md) §3 | 🟡 | SecO | [NF-SEC] |
| 35 | インフラ基本設計 | Infra | [DOC-ARCH-002](../architecture/01-tech-stack.md), [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §18 | 🟡 | SRE | [NF-ENV] |
| 36 | ネットワーク基本設計 | NW | [DOC-ARCH-002](../architecture/02-deployment.md)（旧 DOC-ARCH-003）§3, [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §12 | 🟡 | SRE | [NF-ENV] |
| 37 | 運用設計 | — | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §3, [DOC-ARCH-005](../architecture/05-admin-operations-ui.md) | 🟡 | SRE | [NF-OPS][NF-AVA] |
| 38 | 監視設計 | — | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §3, [DOC-ARCH-005](../architecture/05-admin-operations-ui.md) §5 | 🟡 | SRE | [NF-OPS][NF-AVA] |
| 39 | バックアップ設計 | — | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §3.4 | 🟡（[UN-P0-10](../architecture/07-qa-register.md) 待ち） | DBA + SRE | [NF-AVA] |
| 40 | 移行設計 | — | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2, [DOC-ARCH-003](../architecture/02-deployment.md) §4 | 🟡 | アーキ + SRE | [NF-MIG] |
| 41 | 基本設計レビュー | BD Review |[DOC-TPL-REV §A.2](../templates/01-reviews.md#a2-基本設計レビューチェックリストipa-工程-41--g2)| ⚪ | アーキ + PM + 外部有識者 | [NF-ENV] |

### 5.4 詳細設計（42-52）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 42 | プログラム構造設計 | — | [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §18（Cargo Workspace 16 crate 構造） | 🟡 | アーキ + テックリード | — |
| 43 | モジュール設計 | — | [DOC-MOD-001〜016](../modules/M-01-acquisition-adapter.md) §3 | 🟡 | アーキ + Dev | — |
| 44 | クラス設計 | — | [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §6（Rust 型設計パターン） | 🟡 | アーキ | — |
| 45 | ロジック設計 | — | [DOC-MOD-001〜016](../modules/M-01-acquisition-adapter.md) §3.3 | 🟡 | Dev | — |
| 46 | API詳細設計 | — | [DOC-API-001](../api/rest-endpoints.md), [DOC-API-002](../api/websocket-events.md), [DOC-API-003](../api/error-codes.md) | 🟡 | Dev + アーキ | — |
| 47 | DB詳細設計 | — | [DOC-MOD-010](../modules/M-10-tenant-middleware.md) §4.1〜4.5（11 テーブル DDL） | 🟡 | DBA | — |
| 48 | SQL設計 | — | [DOC-MOD-010](../modules/M-10-tenant-middleware.md) §4.6（6 PL/pgSQL 存過） | 🟡 | DBA | — |
| 49 | バッチ詳細設計 | — | [DOC-MOD-015](../modules/M-15-central-event-bus.md) §3, [DOC-MOD-008](../modules/M-08-trigger-service.md) §3 | 🟡 | Dev | — |
| 50 | エラー処理設計 | — | [DOC-API-003](../api/error-codes.md), [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) §4 | 🟡 | Dev | [NF-OPS] |
| 51 | ログ設計 | — | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §3.3, [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §10（tracing） | 🟡 | Dev + SRE | [NF-OPS] |
| 52 | 詳細設計レビュー | DD Review |[DOC-TPL-REV §A.3](../templates/01-reviews.md#a3-詳細設計レビューチェックリストipa-工程-52--g3)| ⚪ | アーキ + テックリード + PM | [NF-ENV] |

### 5.5 実装（53-58）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 53 | 開発環境構築 | — |[DOC-TPL-RBK §A.1](../templates/04-runbooks.md#a1-開発環境構築手順書ipa-工程-53), [DOC-ARCH-007 §18](../architecture/06-rust-tech-selection.md)| ⚪ | Dev + SRE | [NF-ENV] |
| 54 | コーディング | PG |[DOC-TPL-CHG §A.7](../templates/06-change-management.md#a7-改修-pr-テンプレipa-工程-124), [DOC-ARCH-007](../architecture/06-rust-tech-selection.md)（crate 単位）| ⚪ | Dev 16 名 | — |
| 55 | 静的解析 | SAST |[DOC-ARCH-007 §15](../architecture/06-rust-tech-selection.md)（clippy/rustfmt/deny）, [DOC-TPL-CHG §A.5](../templates/06-change-management.md#a5-patch-ログipa-工程-122)| ⚪ | Dev + SecO | [NF-SEC] |
| 56 | コードレビュー | CR |[DOC-ARCH-007 §15.4](../architecture/06-rust-tech-selection.md)（CODEOWNERS）, [DOC-TPL-CHG §A.7](../templates/06-change-management.md#a7-改修-pr-テンプレipa-工程-124)| ⚪ | テックリード + 担当 Dev | — |
| 57 | ビルド | Build |[DOC-ARCH-007 §16](../architecture/06-rust-tech-selection.md)（cargo build）| ⚪ | Dev | — |
| 58 | CI | CI |[DOC-ARCH-007 §17](../architecture/06-rust-tech-selection.md)（GitHub Actions）| ⚪ | Dev + SRE | — |

### 5.6 単体試験（59-65）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 59 | 単体試験計画 | UT Plan | [DOC-TST-INDEX](../tests/README.md) | 🟡 | QA | — |
| 60 | 単体試験仕様書作成 | UT | [DOC-TST-001](../tests/UT-design.md)（214 ケース） | 🟡 | QA + Dev | — |
| 61 | 単体試験レビュー | UT Review |[DOC-TPL-REV §A.4](../templates/01-reviews.md#a4-単体試験レビューチェックリストipa-工程-61)| ⚪ | QA + Dev | — |
| 62 | 単体試験実施 | UT | —（cargo test 実行証跡） | ⚪ | Dev | — |
| 63 | 不具合修正 | Bug Fix |[DOC-TPL-TST §A.3](../templates/02-tests-execution.md#a3-不具合修正記録ipa-工程-63)| ⚪ | Dev | — |
| 64 | 再試験 | Retest |[DOC-TPL-TST §A.4](../templates/02-tests-execution.md#a4-再試験記録ipa-工程-64)| ⚪ | Dev + QA | — |
| 65 | 単体試験完了承認 | — |[DOC-TPL-TST §A.5](../templates/02-tests-execution.md#a5-ut-完了承認書ipa-工程-65)| ⚪ | QA + テックリード | — |

### 5.7 結合試験（66-75）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 66 | 結合試験計画 | IT Plan | [DOC-TST-INDEX](../tests/README.md) §3 | 🟡 | QA | — |
| 67 | 結合試験仕様書作成 | IT | [DOC-TST-002](../tests/IT-design.md)（47 ケース） | 🟡 | QA + Dev | — |
| 68 | 結合試験環境構築 | — | [DOC-ARCH-002](../architecture/02-deployment.md) §5 | ⚪ | SRE + QA | — |
| 69 | 内部結合試験 | ITa | [DOC-TST-002](../tests/IT-design.md) §2（モジュール間） | ⚪ | QA + Dev | — |
| 70 | 外部結合試験 | ITb | [DOC-TST-002](../tests/IT-design.md) §3（FE↔BE） | ⚪ | QA + FE | — |
| 71 | API結合試験 | — | [DOC-TST-002](../tests/IT-design.md) §4（OpenAPI 準拠） | ⚪ | QA + Dev | — |
| 72 | DB結合試験 | — | [DOC-TST-002](../tests/IT-design.md) §5（マイグレーション + RLS 検証） | ⚪ | QA + DBA | — |
| 73 | 外部システム連携試験 | — | [DOC-MOD-001](../modules/M-01-acquisition-adapter.md) §4.3, [DOC-TST-002](../tests/IT-design.md) §6 | ⚪ | QA + アーキ | — |
| 74 | 障害・不具合対応 | — |[DOC-TPL-TST §A.11](../templates/02-tests-execution.md#a11-障害対応記録ipa-工程-74)| ⚪ | Dev + SRE | — |
| 75 | 回帰試験 | Regression | [DOC-TST-002](../tests/IT-design.md) §7 | ⚪ | QA | — |

### 5.8 システム試験（76-89）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 76 | システム試験計画 | ST Plan | [DOC-TST-INDEX](../tests/README.md) §4 | 🟡 | QA + PM | — |
| 77 | システム試験仕様書作成 | ST | [DOC-TST-003](../tests/ST-design.md)（E2E + NFR + ACC + SMK + DDI + DR + AD = 約 100 ケース） | 🟡 | QA | — |
| 78 | 機能試験 | — | [DOC-TST-003](../tests/ST-design.md) §2（E2E 31 ケース） | ⚪ | QA | — |
| 79 | シナリオ試験 | — | [DOC-TST-003](../tests/ST-design.md) §3 | ⚪ | QA + Biz | — |
| 80 | 性能試験 | PT | [DOC-TST-003](../tests/ST-design.md) §4（NFR 29 ケース） | ⚪ | QA + SRE | [NF-PER] 必須 |
| 81 | 負荷試験 | Load Test | [DOC-TST-003](../tests/ST-design.md) §4.3 | ⚪ | SRE | [NF-PER] |
| 82 | ストレス試験 | Stress | [DOC-TST-003](../tests/ST-design.md) §4.4 | ⚪ | SRE | [NF-PER] |
| 83 | セキュリティ試験 | Security | [DOC-TST-003](../tests/ST-design.md) §5（DevSecOps pipeline scan） | ⚪ | SecO + QA | [NF-SEC] 必須 |
| 84 | 障害試験 | — | [DOC-TST-003](../tests/ST-design.md) §6（DR 6 ケース） | ⚪ | SRE + QA | [NF-SEC] |
| 85 | 復旧試験 | Recovery | [DOC-TST-003](../tests/ST-design.md) §6 | ⚪ | SRE | [NF-AVA] |
| 86 | バックアップ・リストア試験 | B/R | [DOC-TST-003](../tests/ST-design.md) §6（[UN-P0-10](../architecture/07-qa-register.md) 解消後） | ⚪ | DBA + SRE | [NF-AVA] |
| 87 | 可用性試験 | — | [DOC-TST-003](../tests/ST-design.md) §4.5（SLA 99.9% 検証） | ⚪ | SRE | [NF-AVA] 必須 |
| 88 | 運用試験 | OT | [DOC-ARCH-005](../architecture/05-admin-operations-ui.md) §6（Runbook シナリオ） | ⚪ | SRE + QA | [NF-PER][NF-ENV] |
| 89 | システム試験完了承認 | — |[DOC-TPL-REV §A.5](../templates/01-reviews.md#a5-システム試験完了承認書ipa-工程-89--g7)| ⚪ | QA + PM | — |

### 5.9 受入試験（90-95）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 90 | 受入試験計画 | UAT Plan | [DOC-ACC-001](../tests/ST-design.md) §7（UAT 8 ケース） | 🟡 | PO + PM | — |
| 91 | 受入試験仕様書作成 | UAT | [DOC-ACC-001](../tests/ST-design.md) §7 | 🟡 | PO + Biz | — |
| 92 | ユーザー受入試験 | UAT |[DOC-TPL-TST §A.14](../templates/02-tests-execution.md#a14-uat-実施ログipa-工程-92)| ⚪ | Biz ユーザー | — |
| 93 | 業務シナリオ試験 | — |[DOC-TPL-TST §A.15](../templates/02-tests-execution.md#a15-業務シナリオ試験ログipa-工程-93)| ⚪ | Biz + PO | — |
| 94 | 受入判定 | — |[DOC-TPL-REV §A.6](../templates/01-reviews.md#a6-受入判定書ipa-工程-94--g8)| ⚪ | PO | — |
| 95 | 検収 | Acceptance |[DOC-TPL-TST §A.16](../templates/02-tests-execution.md#a16-検収書ipa-工程-95)（契約条件による）| ⚪ | PO | — |

### 5.10 移行（96-101）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 96 | 移行計画 | Migration Plan | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2 | 🟡 | アーキ + SRE | [NF-MIG] |
| 97 | 移行手順作成 | — |[DOC-TPL-RBK §A.3](../templates/04-runbooks.md#a3-移行手順書ipa-工程-97)| ⚪ | SRE | [NF-MIG] |
| 98 | 移行リハーサル | Rehearsal |[DOC-TPL-RBK §A.4](../templates/04-runbooks.md#a4-移行リハーサル記録ipa-工程-98)| ⚪ | SRE + PM | [NF-MIG] |
| 99 | データ移行 | Data Migration |[DOC-TPL-RBK §A.5](../templates/04-runbooks.md#a5-データ移行ログipa-工程-99), [DOC-MOD-010 §4.7](../modules/M-10-tenant-middleware.md)| ⚪ | DBA | — |
| 100 | システム移行 | — |[DOC-TPL-RBK §A.6](../templates/04-runbooks.md#a6-システム移行ログipa-工程-100), [DOC-ARCH-002 §4](../architecture/02-deployment.md)| ⚪ | SRE | [NF-MIG] |
| 101 | 移行結果確認 | — |[DOC-TPL-RBK §A.7](../templates/04-runbooks.md#a7-移行結果確認書ipa-工程-101)| ⚪ | SRE + PM | [NF-MIG] |

### 5.11 リリース（102-108）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 102 | リリース計画 | Release Plan | [DOC-ARCH-002](../architecture/02-deployment.md) §4 | 🟡 | PM + SRE | [NF-MIG] |
| 103 | リリース判定 | Go/No-Go |[DOC-TPL-REV §A.7](../templates/01-reviews.md#a7-リリース-gono-go-判定書ipa-工程-103--g10)（G10 ゲート 4 で定義）| ⚪ | PM + PO + SRE | — |
| 104 | 本番環境構築 | Production |[DOC-TPL-RBK §A.8](../templates/04-runbooks.md#a8-本番デプロイ記録ipa-工程-105), [DOC-ARCH-002 §3](../architecture/02-deployment.md)| ⚪ | SRE | [NF-ENV] |
| 105 | 本番デプロイ | Deploy |[DOC-MOD-014 §2.4](../modules/M-14-module-registry.md)（atomic swap）, [DOC-ARCH-004 §2](../architecture/04-atomic-deployment.md), [DOC-TPL-RBK §A.8](../templates/04-runbooks.md#a8-本番デプロイ記録ipa-工程-105)| ⚪ | SRE | [NF-SEC] |
| 106 | 稼働確認 | Smoke Test |[DOC-TPL-RBK §A.9](../templates/04-runbooks.md#a9-smoke-test-実施ログipa-工程-106), [DOC-TST-003 §8](../tests/ST-design.md)（SMK 8 ケース）| ⚪ | SRE + QA | — |
| 107 | サービス開始 | Go-Live |[DOC-TPL-RBK §A.10](../templates/04-runbooks.md#a10-go-live-宣言書ipa-工程-107)| ⚪ | PM + PO | — |
| 108 | 初期流動対応 | Hypercare |[DOC-TPL-RBK §A.11](../templates/04-runbooks.md#a11-hypercare-計画書ipa-工程-108)| ⚪ | Dev + SRE + サポート | — |

### 5.12 運用（109-117）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 109 | 運用引継ぎ | Handover | [DOC-ARCH-005](../architecture/05-admin-operations-ui.md) §6 | 🟡 | SRE + PM | [NF-OPS] |
| 110 | システム監視 | Monitoring | [DOC-ARCH-005](../architecture/05-admin-operations-ui.md) §5, [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §3.3 | 🟡 | SRE | [NF-AVA] 必須 |
| 111 | ジョブ管理 | Job | [DOC-MOD-015](../modules/M-15-central-event-bus.md) §2.5（Outbox/Scheduler） | 🟡 | SRE | [NF-OPS] |
| 112 | バックアップ | Backup |[DOC-TPL-OPS §A.4](../templates/05-operations.md#a4-backup-スケジュール--ログipa-工程-112), [DOC-ARCH-004 §3.4](../architecture/04-atomic-deployment.md)| ⚪ | DBA + SRE | [NF-AVA] |
| 113 | キャパシティ管理 | Capacity | [DOC-ARCH-002](../architecture/02-deployment.md) §6（スケール方針） | 🟡 | SRE | [NF-AVA] |
| 114 | インシデント管理 | Incident |[DOC-TPL-OPS §A.6](../templates/05-operations.md#a6-incident-response-runbookipa-工程-114)（Incident Response Runbook）| ⚪ | SRE | [NF-SEC] |
| 115 | 障害管理 | — |[DOC-TPL-OPS §A.7](../templates/05-operations.md#a7-postmortem-テンプレートipa-工程-115)（Postmortem テンプレ）| ⚪ | SRE | [NF-SEC] |
| 116 | 問題管理 | Problem |[DOC-TPL-OPS §A.8](../templates/05-operations.md#a8-問題管理台帳ipa-工程-116)| ⚪ | SRE + Dev | — |
| 117 | 問い合わせ管理 | Support | [DOC-ARCH-005](../architecture/05-admin-operations-ui.md) §7 | 🟡 | サポート | — |

### 5.13 保守（118-126）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 118 | 変更要求 | CR | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2（[DOC-MOD-014](../modules/M-14-module-registry.md) §2.4 が中核） | 🟡 | PO | — |
| 119 | 影響分析 | Impact Analysis | [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) §5 | 🟡 | アーキ | — |
| 120 | 変更管理 | Change | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2 | 🟡 | PM + SRE | — |
| 121 | 構成管理 | CM | [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §16.4（cargo metadata + lockfile） | 🟡 | テックリード | — |
| 122 | パッチ適用 | Patch | [DOC-MOD-014](../modules/M-14-module-registry.md) §2.4 | 🟡 | SRE | [NF-OPS] |
| 123 | 脆弱性対応 | Vulnerability | [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) §3.2, [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §15（cargo-deny / cargo-audit） | 🟡 | SecO + Dev | [NF-SEC][NF-OPS] |
| 124 | 改修 | Maintenance | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2 | 🟡 | Dev | [NF-OPS] |
| 125 | 緊急改修 | Hotfix | [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2.5 | 🟡 | Dev + SRE | [NF-OPS] |
| 126 | リグレッションテスト | Regression | [DOC-TST-002](../tests/IT-design.md) §7, [DOC-TST-003](../tests/ST-design.md) §2 | 🟡 | QA | — |

### 5.14 品質管理（127-130）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 127 | 品質計画 | QA Plan | [DOC-ARCH-008](../architecture/07-qa-register.md) | 🟡 | QA + PM | — |
| 128 | 品質レビュー | QA Review | [DOC-ARCH-008](../architecture/07-qa-register.md) §2, §5 | 🚧 | テックリード + QA | — |
| 129 | 品質評価 | QA | [DOC-ARCH-008](../architecture/07-qa-register.md) §8（実装着手判定チェックリスト） | 🚧 | PM + QA | — |
| 130 | 品質監査 | Audit | —（監査チェックポイント一覧は本書 §9） | 🟡 | 監査担当 | — |

### 5.15 管理（131-144）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 131 | プロジェクト計画 | PJ Plan | [README](README.md) §3, [DOC-CHG-001](CHANGELOG.md) | 🟡 | PM | — |
| 132 | WBS管理 | WBS |[DOC-TPL-PRC §A.1](../templates/03-process-management.md#a1-wbs-テンプレートipa-工程-132)| ⚪ | PM | — |
| 133 | 進捗管理 | Progress | [README](README.md) §9, [DOC-CHG-001](CHANGELOG.md) | 🚧 | PM | — |
| 134 | 課題管理 | Issue | [DOC-ARCH-008](../architecture/07-qa-register.md) §5 | 🟡 | PM | — |
| 135 | リスク管理 | Risk | [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) | 🟡 | PM + アーキ | — |
| 136 | 変更管理 | Change | [DOC-CHG-001](CHANGELOG.md), [DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2 | 🟡 | PM | — |
| 137 | 構成管理 | CM | [DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §16.4 | 🟡 | テックリード | — |
| 138 | 成果物管理 | Deliverable | [README](README.md) §4〜§8 | 🟡 | PM | — |
| 139 | レビュー管理 | Review | §7（本書のゲート定義） | 🟡 | PM | — |
| 140 | 会議・報告 | Meeting/Report |[DOC-TPL-PRC §A.5](../templates/03-process-management.md#a5-会議アジェンダ--議事録テンプレートipa-工程-140)| ⚪ | PM | — |
| 141 | 工数管理 | Effort |[DOC-TPL-PRC §A.6](../templates/03-process-management.md#a6-工数管理表ipa-工程-141)| ⚪ | PM | — |
| 142 | コスト管理 | Cost |[DOC-TPL-PRC §A.7](../templates/03-process-management.md#a7-コスト管理表ipa-工程-142)| ⚪ | PM | — |
| 143 | スコープ管理 | Scope | [README](README.md) §10 | 🟡 | PM | — |
| 144 | ベースライン管理 | Baseline | [DOC-CHG-001](CHANGELOG.md) v1.0.0 entry | 🟡 | PM | — |

### 5.16 終結（145-150）

| # | 工程名 | 略称 | 関連文書 | 状態 | 担当 | NF |
|---|---|---|---|---|---|---|
| 145 | プロジェクト完了判定 | — |[DOC-TPL-REV §A.8](../templates/01-reviews.md#a8-プロジェクト完了判定書ipa-工程-145--g11)| ⚪ | PO + PM | — |
| 146 | 成果物引渡し | Handover |[DOC-TPL-CLO §A.1](../templates/08-closure.md#a1-成果物引渡し書ipa-工程-146)| ⚪ | PM | — |
| 147 | 完了報告 | Closure |[DOC-TPL-CLO §A.2](../templates/08-closure.md#a2-完了報告書ipa-工程-147)（最終版として記録）, [DOC-CHG-001](CHANGELOG.md)| ⚪ | PM | — |
| 148 | 振り返り | Retrospective |[DOC-TPL-CLO §A.3](../templates/08-closure.md#a3-retrospective-議事録ipa-工程-148)| ⚪ | PM + チーム全員 | — |
| 149 | ナレッジ移管 | KT |[DOC-TPL-CLO §A.4](../templates/08-closure.md#a4-ナレッジ移管資料ipa-工程-149)| ⚪ | テックリード | — |
| 150 | アーカイブ | Archive |[DOC-TPL-CLO §A.5](../templates/08-closure.md#a5-アーカイブ手順書ipa-工程-150)（`docs/legacy/` 慣行）| ⚪ | PM | — |

---

## 6. カテゴリ別詳細

> §5 の表では触れられなかった各カテゴリの「**入口/出口基準**」「**主要成果物**」「**ゲート条件**」「**監査ポイント**」を詳述する。

### 6.1 超上流（01-09）

| 項目 | 内容 |
|---|---|
| 入口基準 | 経営層のシステム化検討開始の意思決定 |
| 出口基準 | システム化計画書承認、PJ 予算確保、PJ マネージャー任命 |
| 主要成果物 | システム化構想書、システム化計画書、PJ 計画書、As-Is/To-Be モデル |
| 関連 DOC | [DOC-REQ-001](../legacy/requirements.md) §1〜§6、[DOC-ARCH-001](../architecture/00-anatomy-model.md) |
| ゲート | **G0：PJ 立上げ判定**（PM + PO 合議） |
| 監査ポイント | 経営要求のトレーサビリティ（[DOC-REQ-001](../legacy/requirements.md) §1 → システム化計画） |

### 6.2 要件定義（10-21）

| 項目 | 内容 |
|---|---|
| 入口基準 | システム化計画書承認 |
| 出口基準 | 全要件（UR/BR/SR/FR/NFR/データ/IF/セキュリティ/運用/移行）のベースライン化 + レビュー/承認完了 |
| 主要成果物 | 要件定義書（[DOC-REQ-001](../legacy/requirements.md) v1.2.1, baseline）、各 [DOC-MOD-NNN](../modules/M-01-acquisition-adapter.md) §1 の「需求来源」 |
| NF タグ | NFR は 6 区分全カバー必須（[NF-AVA\|PER\|OPS\|MIG\|SEC\|ENV]【必須】） |
| ゲート | **G1：要件ベースライン化**（PO + PM + アーキ + SecO 承認） |
| 監査ポイント | 各 F-ID のトレーサビリティ、NF タグ網羅率、用語集整合 |

### 6.3 基本設計（22-41）

| 項目 | 内容 |
|---|---|
| 入口基準 | 要件ベースライン確立 |
| 出口基準 | 16 モジュール（[DOC-MOD-001〜016](../modules/M-01-acquisition-adapter.md)）§2「基本设计」 + [DOC-ARCH-001〜008](../architecture/00-anatomy-model.md) 完了 + BD Review 通過 |
| 主要成果物 | 16 モジュール §2、8 横切文書、6 API 仕様書 |
| NF タグ | アーキ/IF/権限/セキュリティ/インフラ/運用/監視/バックアップ/移行 全項目に NF タグ付与 |
| ゲート | **G2：基本設計レビュー (BD Review)**（[NF-ENV]【必須】網羅率 ≥ 90%） |
| 監査ポイント | モジュール間インタフェース整合、NF タグ網羅率、未決事項管理（[DOC-ARCH-008](../architecture/07-qa-register.md) 連動） |

### 6.4 詳細設計（42-52）

| 項目 | 内容 |
|---|---|
| 入口基準 | BD Review 通過 |
| 出口基準 | 16 モジュール §3「详细设计」 + 全 API 詳細 + DB 詳細（11 テーブル DDL） + 6 PL/pgSQL 存過 + Rust crate 構造（[DOC-ARCH-007](../architecture/06-rust-tech-selection.md) §18）+ DD Review 通過 |
| 主要成果物 | 16 モジュール §3、6 API 文書、11 DDL、6 存過、Cargo Workspace 16 crate 設計 |
| NF タグ | 詳細設計段階では [NF-OPS]（ログ・エラー）が特に重要、NF タグ全網羅必須 |
| ゲート | **G3：詳細設計レビュー (DD Review)**（[NF-ENV]【必須】網羅率 ≥ 95%） |
| 監査ポイント | SQL インジェクション対策、RLS ポリシー網羅、PL/pgSQL 権限昇格範囲、ログ設計の PII マスキング |

### 6.5 実装（53-58）

| 項目 | 内容 |
|---|---|
| 入口基準 | DD Review 通過 + [DOC-ARCH-008](../architecture/07-qa-register.md) §5 の P0（11 件）全解消 + Rust 16 crate 担当人員確保 |
| 出口基準 | 16 crate 全て `cargo build` 成功 + clippy/rustfmt/cargo-deny/cargo-audit 全パス + CI green |
| 主要成果物 | 16 crate ソースコード、CI pipeline 設定、SAST レポート、コードレビュー記録 |
| NF タグ | [NF-SEC]（SAST）、[NF-ENV]（CI マトリクス 3 OS × 16 crate） |
| ゲート | **G4：実装着手判定**（[DOC-ARCH-008](../architecture/07-qa-register.md) §8 全チェック） |
| 監査ポイント | CODEOWNERS 遵守、dangerzone レビュー、SAST 重大脆弱性 0 件、CI 通過率 |

### 6.6 単体試験（59-65）

| 項目 | 内容 |
|---|---|
| 入口基準 | G4（実装着手）通過後、コードベース ≥ 80% 実装 |
| 出口基準 | [DOC-TST-001](../tests/UT-design.md) 214 ケース全実行 + 合格率 100% + 完了承認 |
| 主要成果物 | UT 仕様書、UT 実施ログ、修正パッチ、Retest 記録、UT 完了報告 |
| NF タグ | なし（[NF-PER] は 80 へ） |
| ゲート | **G5：UT 完了** |
| 監査ポイント | コードカバレッジ ≥ 80%、境界値・異常系網羅、モック利用方針遵守 |

### 6.7 結合試験（66-75）

| 項目 | 内容 |
|---|---|
| 入口基準 | G5（UT 完了）通過 |
| 出口基準 | [DOC-TST-002](../tests/IT-design.md) 47 ケース全実行 + DB マイグレーション成功 + RLS 検証 + 外部 IF 疎通 + 回帰合格 |
| 主要成果物 | IT 仕様書、結合環境構築手順、ITa/ITb/API/DB/外部 IF 試験ログ、回帰ログ |
| NF タグ | なし（[NF-SEC] は 83 へ） |
| ゲート | **G6：IT 完了** |
| 監査ポイント | モジュール間インタフェース不整合 0 件、トランザクション整合、RLS ポリシー漏れ 0 件 |

### 6.8 システム試験（76-89）

| 項目 | 内容 |
|---|---|
| 入口基準 | G6（IT 完了）通過、本番相当環境構築済 |
| 出口基準 | [DOC-TST-003](../tests/ST-design.md) 全 100 ケース（E2E + NFR + ACC + SMK + DDI + DR + AD）合格 + SLA 99.9% 検証 + セキュリティ脆弱性 0 件 + 完了承認 |
| 主要成果物 | ST 仕様書、性能試験レポート、負荷/ストレスレポート、セキュリティ診断書、DR 試験ログ、ST 完了承認書 |
| NF タグ | [NF-PER] 必須（80）、[NF-SEC] 必須（83）、[NF-AVA] 必須（87）、[NF-ENV] 必須（88） |
| ゲート | **G7：ST 完了** |
| 監査ポイント | NFR 6 区分全カバー、脆弱性 0 件目標、SLA 達成、DR 復旧時間目標達成 |

### 6.9 受入試験（90-95）

| 項目 | 内容 |
|---|---|
| 入口基準 | G7（ST 完了）通過、ユーザー教育完了 |
| 出口基準 | [DOC-ACC-001](../tests/ST-design.md) §7 UAT 8 ケース + 業務シナリオ全合格 + ユーザー受入判定 + 検収 |
| 主要成果物 | UAT 計画、UAT 仕様書、業務シナリオ、受入判定書、検収書 |
| NF タグ | なし |
| ゲート | **G8：受入判定**（PO 最終承認） |
| 監査ポイント | 業務要件 [DOC-REQ-001](../legacy/requirements.md) §7 との整合、運用トレーニング完了 |

### 6.10 移行（96-101）

| 項目 | 内容 |
|---|---|
| 入口基準 | G8（受入判定）通過 |
| 出口基準 | 移行リハーサル 2 回成功 + データ移行整合検証 + 本番切替 + 結果確認 |
| 主要成果物 | 移行計画書、手順書、リハーサルログ、データ移行ログ、移行結果報告書 |
| NF タグ | [NF-MIG] 必須（96, 97, 98, 100, 101） |
| ゲート | **G9：移行判定** |
| 監査ポイント | ロールバック手順、ダウンタイム目標、データ整合 100%、監査ログ連続性 |

### 6.11 リリース（102-108）

| 項目 | 内容 |
|---|---|
| 入口基準 | G9（移行判定）通過 |
| 出口基準 | 本番環境構築 + デプロイ（[DOC-MOD-014](../modules/M-14-module-registry.md) atomic swap） + Smoke 通過 + Go-Live 宣言 + Hypercare 期間（2 週間）設定 |
| 主要成果物 | リリース計画書、Go/No-Go 判定書、本番環境構築ログ、Smoke ログ、Go-Live 宣言、Hypercare 計画 |
| NF タグ | [NF-MIG] 必須（102）、[NF-SEC] 必須（105）、[NF-ENV] 必須（104） |
| ゲート | **G10：Go-Live 判定**（PM + PO + SRE 全会一致） |
| 監査ポイント | Smoke 全合格、SLA 計測開始、Hypercare 体制確立、ロールバック即応体制 |

### 6.12 運用（109-117）

| 項目 | 内容 |
|---|---|
| 入口基準 | G10（Go-Live）通過、Hypercare 終了 |
| 出口基準 | 監視ダッシュボード稼働、ジョブ全成功、Backup 自動運用開始、Incident Response 手順確立 |
| 主要成果物 | 監視設定、ジョブ定義書、Backup スケジュール、Incident ログ、問題管理台帳、問い合わせ対応記録 |
| NF タグ | [NF-OPS] 必須（109, 111）、[NF-AVA] 必須（110, 113）、[NF-SEC] 必須（114, 115） |
| ゲート | なし（常時運用、改善サイクル） |
| 監査ポイント | SLA 遵守率、MTBF/MTTR、Backup 成功率、Incident 対応時間、Capacity 使用率 |

### 6.13 保守（118-126）

| 項目 | 内容 |
|---|---|
| 入口基準 | 運用中、機能追加/不具合修正の要求発生 |
| 出口基準 | 変更要求のトリアージ + 影響分析 + 承認 + atomic 反映 + 脆弱性対応 + 回帰合格 |
| 主要成果物 | CR チケット、影響分析レポート、変更承認記録、構成管理台帳、Patch ログ、脆弱性対応ログ、改修 PR、回帰テストログ |
| NF タグ | [NF-OPS] 必須（122, 124, 125）、[NF-SEC] 必須（123） |
| ゲート | 変更ごとに小さな Gate あり（[DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2 参照） |
| 監査ポイント | atomic swap 成功率、ロールバック成功率、脆弱性 SLA 遵守、回帰影響範囲 |

### 6.14 品質管理（127-130）

| 項目 | 内容 |
|---|---|
| 入口基準 | PJ 立上げ時点から開始 |
| 出口基準 | 品質計画策定 + 定期レビュー + 評価 + 監査対応 |
| 主要成果物 | [DOC-ARCH-008](../architecture/07-qa-register.md)（品質計画・QA 登録簿）、品質レビュー記録、品質評価レポート、監査対応記録 |
| NF タグ | 全 NF 区分横断 |
| ゲート | 各 Gate 通過時の品質評価 |
| 監査ポイント | NF タグ網羅率、未決事項解消率、テスト合格率、監査指摘事項対応率 |

### 6.15 管理（131-144）

| 項目 | 内容 |
|---|---|
| 入口基準 | PJ 立上げ時点から開始 |
| 出口基準 | PJ 計画 + WBS + 進捗/課題/リスク/変更/CM/Deliverable/レビュー/工数/コスト/Scope/Baseline の各管理プロセス確立 |
| 主要成果物 | [DOC-CHG-001](CHANGELOG.md)、[README](README.md)、[DOC-ARCH-008](../architecture/07-qa-register.md) §5、WBS、進捗レポート、課題管理表、リスク登録簿、構成管理台帳 |
| NF タグ | なし |
| ゲート | 各 Gate の判定プロセスに組み込み |
| 監査ポイント | 計画と実績の乖離、課題/Risk 早期検出、変更管理記録、ベースライン保護 |

### 6.16 終結（145-150）

| 項目 | 内容 |
|---|---|
| 入口基準 | 主要開発/移行フェーズ完了、Hypercare 終了 |
| 出口基準 | 完了判定 + 成果物引渡し + 完了報告 + Retrospective + KT + Archive |
| 主要成果物 | 完了判定書、引渡し書、完了報告書、Retrospective 議事録、KT 資料、Archive データ |
| NF タグ | なし |
| ゲート | **G11：PJ 完了判定**（PO + PM + 経営層） |
| 監査ポイント | 残存課題引き継ぎ、ナレッジ完全移管、Archive 完全性、契約上の検収条件充足 |

---

## 7. ゲート / マイルストーン定義

### 7.1 ゲート一覧（11 個）

| Gate # | 名称 | 関連フェーズ | 判定者 | 通過基準 | 現在の状態 |
|---|---|---|---|---|---|
| **G0** | PJ 立上げ判定 | 05 | PM + PO | システム化計画書承認、予算確保、PM 任命 | 🟡 設計完了（[DOC-ARCH-001](../architecture/00-anatomy-model.md) ベース） |
| **G1** | 要件ベースライン化 | 21 | PO + PM + アーキ + SecO | 全要件レビュー、NF タグ網羅、ベースライン確定 | 🟡 設計完了（要件 v1.2.1 baseline 済） |
| **G2** | 基本設計レビュー (BD Review) | 41 | アーキ + PM + 外部有識者 | 16 モジュール §2 + 8 横切文書 + BD Review 通過、NF 網羅率 ≥ 90% | ⚪ 未実施 |
| **G3** | 詳細設計レビュー (DD Review) | 52 | アーキ + テックリード + PM | 16 モジュール §3 + 全 API + 11 DDL + 6 存過 + Cargo 構造、NF 網羅率 ≥ 95% | ⚪ 未実施 |
| **G4** | 実装着手判定 | 53 | PM + アーキ + テックリード | [DOC-ARCH-008](../architecture/07-qa-register.md) §5 P0（11 件）全解消、Rust 16 crate 担当人員確保、CI 環境準備 | ⚪ 未実施 |
| **G5** | UT 完了 | 65 | QA + テックリード | UT 214 ケース全合格、コードカバレッジ ≥ 80% | ⚪ 未実施 |
| **G6** | IT 完了 | 75 | QA + アーキ | IT 47 ケース全合格、RLS 検証、API 整合 | ⚪ 未実施 |
| **G7** | ST 完了 | 89 | QA + PM | ST 100 ケース全合格、SLA 99.9% 検証、脆弱性 0 件 | ⚪ 未実施 |
| **G8** | 受入判定 | 94 | PO | UAT 8 ケース全合格、業務シナリオ適合、検収条件充足 | ⚪ 未実施 |
| **G9** | 移行判定 | 101 | PM + SRE + PO | リハーサル 2 回成功、データ整合 100%、ロールバック準備完了 | ⚪ 未実施 |
| **G10** | Go-Live 判定 | 103 | PM + PO + SRE | 全 Smoke 合格、Hypercare 体制確立、ダウンタイム許容確認 | ⚪ 未実施 |
| **G11** | PJ 完了判定 | 145 | PO + PM + 経営層 | 全成果物引渡し、残存課題引き継ぎ、KT 完了 | ⚪ 未実施 |

### 7.2 マイルストーンタイムライン（暫定）

```
2026-08-20 (本日) ──── 全 docs 完了
    │  G0, G1 通過（docs ベース）
    │
    ├─ G2 / G3 通過 ── G4（実装着手判定）
    │     ↑
    │  BD Review / DD Review 実施
    │
    ├─ G4 → 実装開始 (M-01〜M-16 順次)
    │     │
    │     ├─ 53-58 実装
    │     │     │
    │     │     └─ 59-65 UT (G5)
    │     │           │
    │     │           └─ 66-75 IT (G6)
    │     │                 │
    │     │                 └─ 76-89 ST (G7)
    │     │                       │
    │     │                       └─ 90-95 UAT (G8)
    │     │                             │
    │     │                             └─ 96-101 移行 (G9)
    │     │                                   │
    │     │                                   └─ 102-108 リリース (G10)
    │     │
    │     └─ 並行：109-117 運用、118-126 保守
    │
    └─ G11 PJ 完了 (145)
```

※ 各 Gate 通過の具体的な日付は Rust 人員確保・[DOC-ARCH-008](../architecture/07-qa-register.md) §5 P0 解消後に決定。

---

## 8. ロール / 責任分担表（RACI）

### 8.1 ロール定義

| 略称 | ロール名 | 想定人数（本 PJ） |
|---|---|---|
| PO | プロダクトオーナー | 1 |
| PM | プロジェクトマネージャー | 1 |
| アーキ | システムアーキテクト | 2（+ 1 補欠） |
| テックリード | テックリード | 1 |
| Dev | バックエンド開発者（Rust） | 16（crate ごと） |
| FE | フロントエンド開発者（Bevy WASM） | 2 |
| DBA | データベース管理者 | 1（兼務） |
| SRE | SRE エンジニア | 2 |
| SecO | セキュリティオフィサ | 1（兼務） |
| QA | QA エンジニア | 2 |
| サポート | サポート担当 | 1（運用フェーズから） |
| 監査担当 | 内部/外部監査 | 0.5（兼務） |
| 外部有識者 | セキュリティ/アーキ外部レビュー | 0.5（特定フェーズのみ） |

### 8.2 フェーズ × ロール RACI（主要フェーズ抜粋）

| フェーズ | PO | PM | アーキ | テックリード | Dev | FE | DBA | SRE | SecO | QA | 監査 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 01-04（経営要求確認〜企画） | A | C | C | I | I | I | I | I | I | I | I |
| 05（PJ 立上げ） | A | R | C | I | I | I | I | I | I | I | I |
| 06-09（As-Is/To-Be） | C | A | R | C | I | I | C | I | C | I | I |
| 10-19（要件定義） | A | C | R | C | C | C | C | C | C | I | I |
| 20（RD Review） | A | R | R | C | I | I | C | C | C | C | I |
| 21（Baseline 化） | A | R | C | I | I | I | I | I | I | I | I |
| 22-40（基本設計） | C | C | A | R | C | C | C | C | C | I | I |
| 41（BD Review） | C | A | R | R | C | C | C | C | C | C | C（外部有識者） |
| 42-51（詳細設計） | I | C | A | R | R | R | R | C | C | I | I |
| 52（DD Review） | I | A | R | R | C | C | C | C | C | C | C |
| 53（開発環境構築） | I | C | C | A | R | R | C | R | I | I | I |
| 54-58（実装） | I | C | C | A | R | R | C | C | C | I | I |
| 59-65（UT） | I | C | I | A | R | R | I | I | I | R | I |
| 66-75（IT） | I | C | C | A | R | R | R | C | C | R | I |
| 76-89（ST） | I | C | C | A | C | C | C | R | R | R | C |
| 90-95（UAT） | A | R | C | C | C | C | I | I | I | R | I |
| 96-101（移行） | C | A | C | C | C | I | R | R | I | C | I |
| 102-108（リリース） | A | R | C | C | C | I | C | R | C | C | I |
| 109-117（運用） | I | C | I | C | I | I | C | A | C | I | C |
| 118-126（保守） | A | C | C | R | R | R | C | R | R | C | I |
| 127-130（品質管理） | C | A | C | C | C | C | C | C | C | R | R |
| 131-144（管理） | C | A/R | C | C | I | I | I | I | I | I | I |
| 145-150（終結） | A | R | C | C | C | C | C | C | C | C | I |

**凡例**：A = 説明責任（Accountable）、R = 実行（Responsible）、C = 協議（Consulted）、I = 報告先（Informed）

---

## 9. 監査 / チェックポイント

### 9.1 監査チェックポイント一覧

| # | 監査項目 | 関連フェーズ | 頻度 | 証跡 | 担当 |
|---|---|---|---|---|---|
| AUD-01 | NF タグ網羅率 | 14, 20, 41, 52 | 設計レビュー時 | [DOC-ARCH-008](../architecture/07-qa-register.md) §2 | アーキ + QA |
| AUD-02 | トレーサビリティ（要求 → 設計 → 実装 → 試験） | 10-19, 25, 45, 60, 67, 77 | ベースライン化時、試験着手時 | トレーサビリティマトリクス | QA |
| AUD-03 | 変更管理記録 | 21, 118-120, 136 | 全変更時 | [DOC-CHG-001](CHANGELOG.md) | PM |
| AUD-04 | リスクレビュー | 08, 135, 144 | 月次 | [DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) | PM + アーキ |
| AUD-05 | 構成管理（CM） | 121, 137 | リリース時 | Cargo.lock、tag 記録 | テックリード |
| AUD-06 | セキュリティ脆弱性 | 17, 34, 55, 83, 123 | 週次 + リリース前 | SAST レポート、cargo-deny/cargo-audit ログ、penetration test レポート | SecO |
| AUD-07 | コードカバレッジ | 62 | UT 完了時 | cargo tarpaulin レポート | QA |
| AUD-08 | SLA 計測 | 87, 110, 113 | 運用中 | 監視ダッシュボード | SRE |
| AUD-09 | Backup 成功 | 86, 112 | 日次 | Backup ログ | DBA + SRE |
| AUD-10 | Incident/障害対応 | 84, 85, 114, 115 | 発生時 | Incident レポート、Postmortem | SRE |
| AUD-11 | 監査ログ連続性 | 105, 110 | 日次 | audit_log テーブル検証 | SecO |
| AUD-12 | PII マスキング | 14, 51, 88 | ログ監査時 | ログサンプル調査 | SecO + SRE |
| AUD-13 | 未決事項解消 | 20, 41, 52, 129, 134 | 週次 | [DOC-ARCH-008](../architecture/07-qa-register.md) §5 | PM + QA |
| AUD-14 | ベースライン保護 | 21, 144 | 変更時 | [DOC-CHG-001](CHANGELOG.md) v-tag | PM |
| AUD-15 | 監査証跡アーカイブ | 130, 150 | 監査時 + PJ 完了時 | Archive ディレクトリ | PM + 監査担当 |

### 9.2 監査証跡の保管場所

- **設計書**：`docs/` 配下（Git 管理）
- **コードレビュー記録**：GitHub PR + レビューコメント
- **CI/SAST ログ**：GitHub Actions + cargo-deny/cargo-audit
- **試験ログ**：`tests/` 配下に証跡ディレクトリ（実装時に作成）
- **運用ログ**：監視ダッシュボード + ログ基盤
- **監査レポート**：`docs/audit/`（将来作成）

---

## 10. リスク / 前提 / 制約

### 10.1 リスク（[DOC-ARCH-003](../architecture/03-cross-cutting-risks.md) との補完関係）

| リスク ID | カテゴリ | 内容 | 影響 | 対応フェーズ | 担当 |
|---|---|---|---|---|---|
| WK-R-01 | スケジュール | 16 crate 担当人員未確定 [UN-P0-01](../architecture/07-qa-register.md) | G4 以降すべて遅延 | 53-58 | PM |
| WK-R-02 | 品質 | Rust エンジニア習熟度ばらつき | UT 合格率低下 | 59-65 | テックリード |
| WK-R-03 | セキュリティ | KMS 選定未 [UN-P0-06](../architecture/07-qa-register.md) | 鍵管理方式確定不可 | 17, 34, 123 | SecO |
| WK-R-04 | 運用 | Backup 戦略未策定 [UN-P0-10](../architecture/07-qa-register.md) | DR 試験不可 | 39, 86, 112 | DBA + SRE |
| WK-R-05 | コンプラ | GDPR/PIPL 「忘れられる権利」未 [UN-P0-08](../architecture/07-qa-register.md) | 法的リスク | 17, 18, 124 | SecO + PO |
| WK-R-06 | データ | ログ基盤未選定 [UN-P0-09](../architecture/07-qa-register.md) | 監視/分析不可 | 38, 51, 110 | SRE |
| WK-R-07 | ガバナンス | 起草/レビュー/承認組織未確定 [UN-P0-02](../architecture/07-qa-register.md) | ドキュメント承認不可 | 20, 21, 41, 52, 89, 94, 103, 107, 145 | PM |
| WK-R-08 | 技術 | ADR レビュー会未 [UN-P0-11](../architecture/07-qa-register.md) | 技術選定妥当性未検証 | 22-24, 42-44 | テックリード |
| WK-R-09 | 性能 | Bevy WASM bundle サイズ未測定 | 起動時間 SLA 影響 | 26, 53, 80, 81 | FE + SRE |
| WK-R-10 | 拡張 | 16 crate 単一 vs 独立バージョニング未決定 [QA-A05](../architecture/07-qa-register.md) | リリース戦略影響 | 121, 137, 57 | テックリード |

### 10.2 前提条件

1. **人員**：Rust 16 crate 担当 + 補助人員の確保（[UN-P0-01](../architecture/07-qa-register.md)）
2. **環境**：3 OS 開発環境（macOS / Linux / Windows）+ GitHub Actions
3. **データ**：PL/pgSQL 6 存過は [DOC-MOD-010](../modules/M-10-tenant-middleware.md) §4.6 で確定
4. **承認プロセス**：起草/レビュー/承認組織確定（[UN-P0-02](../architecture/07-qa-register.md)）
5. **契約**：受入条件・検収条件の事前合意

### 10.3 制約

1. **主言語**：Rust（変更不可）
2. **準拠規格**：IPA 共通フレーム2018 + IPA 非機能要求グレード2018 + JIS X 0160
3. **NF タグ**：6 区分全網羅、必須/推奨の 2 段階評価
4. **ドキュメント形式**：IPA 表頭/頁脚/改訂履歴/用語集/参考文献 必須
5. **デプロイ方式**：atomic 反映必須（[DOC-ARCH-004](../architecture/04-atomic-deployment.md) §2）

---

## 11. 現在のクリティカルパス

### 11.1 クリティカルパス上の最重要未決事項（[DOC-ARCH-008](../architecture/07-qa-register.md) §5 連動）

```
[UN-P0-01] Rust 人員確保
    ↓
[UN-P0-02] 起草/レビュー/承認組織確定
    ↓
[UN-P0-11] ADR レビュー会 GO/NO-GO
    ↓
[UN-P0-03] canvas 循環 FK レビュー
    ↓
[UN-P0-04] Module Manifest JSON Schema
    ↓
[UN-P0-05] audit_log パーティション DDL
    ↓
[UN-P0-06] KMS 選定
    ↓
[UN-P0-07] JWT 鍵ローテーション
    ↓
[UN-P0-08] 忘れられる権利フロー
    ↓
[UN-P0-09] ログ基盤選定
    ↓
[UN-P0-10] Backup/Restore 戦略
    ↓
G4（実装着手判定）通過
    ↓
53-58 実装着手
```

### 11.2 推奨解消順序

1. **UN-P0-01**（人員）：PM が直ちにアサイン調整。なければ外部委託を視野。
2. **UN-P0-02**（組織）：PO + PM で同日内決定可能。
3. **UN-P0-11**（ADR レビュー）：[DOC-ARCH-007](../architecture/06-rust-tech-selection.md) 完成済。レビュー会日程設定のみ。
4. **UN-P0-03, 04, 05**（DB 関連）：DBA 確保後 1 週間で解消可能。
5. **UN-P0-06, 07, 08**（セキュリティ）：SecO + アーキで 1〜2 週間。
6. **UN-P0-09, 10**（運用基盤）：SRE 確保後 1 週間。

**合計**：人員確保を最優先とすれば、4〜6 週間で G4 到達可能。

---

## 12. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| IPA | 独立行政法人情報処理推進機構 | — |
| 共通フレーム | IPA が定める SLCP（システムライフサイクルプロセス）標準 | IPA「共通フレーム2018」 |
| SLCP-JCF2018 | 共通フレーム2018 の正式呼称 | IPA「共通フレーム2018」 |
| プロセス | 入力 → アクティビティ → 出力 の単位 | IPA 共通フレーム §3 |
| アクティビティ | プロセス内の個々の作業単位 | IPA 共通フレーム §3 |
| タスク | IPA が定義する 150 工程の最小単位 | IPA 共通フレーム §5〜§10 |
| フェーズ | 本書では IPA の 150 工程を意味（プロセスより細かい単位） | 本書 §1 |
| ゲート | フェーズ通過判定の節目 | IPA 共通フレーム §5 |
| マイルストーン | スケジュール上の重要到達点 | PMBOK |
| RACI | Responsible / Accountable / Consulted / Informed の責任分担表 | PMBOK |
| トレーサビリティ | 要求から実装・試験までの追跡可能性 | IPA 共通フレーム §6 |
| ベースライン | 承認済み文書の不変スナップショット | IPA 共通フレーム §6 |
| 構成管理 (CM) | 成果物の変更履歴と整合性管理 | IPA 共通フレーム §10 |
| リスク | 不確実な事象で影響度と発生確率を持つもの | IPA 共通フレーム §5 |
| 品質監査 | 品質要求への適合度独立評価 | ISO 9001 |
| インシデント | サービス中断/品質低下事象 | ITIL |
| SLA | Service Level Agreement | ITIL |
| MTBF | Mean Time Between Failures | 信頼性工学 |
| MTTR | Mean Time To Repair | 信頼性工学 |
| RTO | Recovery Time Objective | DR |
| RPO | Recovery Point Objective | DR |
| Bevy | Rust 製ゲーム/インタラクティブアプリ ECS フレームワーク | DOC-ARCH-002 |
| PL/pgSQL | PostgreSQL の手続き言語 | DOC-MOD-010 §4.6 |
| RLS | Row-Level Security（PostgreSQL 行レベルセキュリティ） | DOC-MOD-010 §2.2 |
| Outbox パターン | イベントと DB トランザクションを整合させるパターン | DOC-MOD-015 §2.5 |
| CRDT | Conflict-free Replicated Data Type | DOC-MOD-011 §3.2 |
| KMS | Key Management Service | DOC-ARCH-008 UN-P0-06 |
| ADR | Architecture Decision Record | DOC-ARCH-007 |

---

## 13. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018 年 3 月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018 年 4 月
3. IPA「ソフトウェア開発データ白書」、独立行政法人情報処理推進機構、各年度版
4. IPA「プロセス品質マネジメントガイドブック」、独立行政法人情報処理推進機構
5. JIS X 0160:2012「ソフトウェアライフサイクルプロセス」、日本工業標準調査会、2012 年
6. JIS Q 9001:2015「品質マネジメントシステム—要求事項」、日本工業標準調査会
7. PMBOK Guide 第 7 版、Project Management Institute、2021 年
8. ITIL 4、AXELOS、2019 年
9. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 要件定義書 v1.2.1」、2026-08-18（[DOC-REQ-001](../legacy/requirements.md)）
10. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 基本設計書 v1.3.0」、2026-08-18（[DOC-BSC-001](../legacy/basic-design.md)）
11. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）
12. Ada プロジェクトチーム「[DOC-ARCH-001 仿生モデル](../architecture/00-anatomy-model.md)」、2026-08-19
13. Ada プロジェクトチーム「[DOC-ARCH-002 技術スタック](../architecture/01-tech-stack.md)」、2026-08-19
14. Ada プロジェクトチーム「[DOC-ARCH-003 横断リスク](../architecture/03-cross-cutting-risks.md)」、2026-08-19
15. Ada プロジェクトチーム「[DOC-ARCH-004 原子化デプロイ](../architecture/04-atomic-deployment.md)」、2026-08-19
16. Ada プロジェクトチーム「[DOC-ARCH-005 管理画面](../architecture/05-admin-operations-ui.md)」、2026-08-19
17. Ada プロジェクトチーム「[DOC-ARCH-007 Rust crate 選択](../architecture/06-rust-tech-selection.md)」、2026-08-19
18. Ada プロジェクトチーム「[DOC-ARCH-008 QA 登録簿](../architecture/07-qa-register.md)」、2026-08-19
19. Ada プロジェクトチーム「[DOC-TST-INDEX テスト総覧](../tests/README.md)」、2026-08-19
20. Ada プロジェクトチーム「[DOC-TST-001 UT 設計](../tests/UT-design.md)」、2026-08-19
21. Ada プロジェクトチーム「[DOC-TST-002 IT 設計](../tests/IT-design.md)」、2026-08-19
22. Ada プロジェクトチーム「[DOC-TST-003 ST 設計](../tests/ST-design.md)」、2026-08-19

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*