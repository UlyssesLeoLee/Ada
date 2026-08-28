# 14 Auto-remediation 設計（Phase 8）

> Alertmanager 発火 → 宣言的 runbook → 自動修復。`ada-remediation` crate が in-process エンジン、`config/remediation/*.json` が宣言的設定、`db/migrations/V003__phase8_remediation.sql` が永続履歴 + cooldown テーブル。

> **ドキュメントID**：DOC-OBS-014
> **上位文書**：[DOC-OBS-INDEX](README.md)
> **下位文書**：[DOC-OBS-011 §10 Phased Rollout](11-phased-rollout.md) / [DOC-OBS-006 Dashboard Catalog](06-dashboard-catalog.md) / [DOC-OBS-007 Alert Policy](07-alert-policy.md) / [DOC-OBS-008 SLO Design](08-slo-design.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-27 | 初版（Phase 8 v0.6.0 実装完了） |
| v1.1.0 | 2026-08-27 | Phase 8.5 SRE ハードニング v0.7.0 実装完了。Real executor (commit 0) / Prometheus exporter + /metrics (commit-1) / hot-reload watcher polling fallback (commit-2) / webhook shared-secret auth (commit-3) / manual trigger auth (commit-4) / SLO 7.5 + Error Budget policy (commit-5) / ドキュメント (commit-6) を §12 に記録。§9 已知の制約の 9.1 / 9.2 / 9.3 / 9.4 は v0.7.0 で解消 (残 9.5 のみ)。 |

---

## 目次

1. スコープと設計原則
2. アーキテクチャ
3. runbook ファイル形式
4. cooldown とリトライ戦略
5. デフォルト runbook 一覧
6. HTTP API
7. セキュリティと秘匿
8. テスト方針
9. 既知の制約
10. 用語集
11. 参考文献
12. v0.7.0 ハードニング完了サマリ (Phase 8.5)

---

## 1. スコープと設計原則

### 1.1 スコープ

Phase 8 (per `docs/observability/11-phased-rollout.md` §10) は、Alertmanager から発火される alert に対し、**宣言的に定義された runbook ファイルを in-process で実行する** ことに尽きる。具体的には:

- 30+ alert のうち、定型的で副作用が予測可能な 5 カテゴリ (Disk / ServiceDown / DB Pool / SLO Burn) を自動修復対象とする
- 残りの alert (例: SLO Burn Fast = 2%/1h) は **page-and-forget** — 自動修復は試みず、page_operator だけ発火する
- すべての実行は永続化され、`remediation_history` テーブルに監査ログが残る

### 1.2 設計原則

1. **宣言的設定 > 命令的コード**。`config/remediation/*.json` の編集で挙動を変えられる。コード変更は原則不要。
2. **cooldown は主防御**。自動修復は retry storm を作りやすい。1 つの action が同じ alert に対し cooldown window (`runbook.cooldown`) 以内に再実行されることはない。
3. **失敗は短 cooldown で再試行可能**。`succeeded` のときだけ full cooldown を課し、`failed` のときは 10s 短 cooldown (application 層 + persistent 層のデフォルト)。flapping している alert を即座に沈黙させないため。
4. **dry-run デフォルト**。Phase 8 v0.6.0 リリース時点で、`HttpCall` / `PgFunction` / `NotifySlack` / `PageOperator` ステップは **dry-run パス**で動作する (intent を記録して成功を返す)。`RunCommand` のみ `tokio::process::Command` で実際に走る。これは offline build 制約 (reqwest / sqlx が `Cargo.lock` に無い) による。
5. **可観測性ファースト**。実行結果は `remediation_history` に必ず書き込まれ、`/remediation/history` / `/remediation/cooldowns` で取得できる。Grafana dashboard 80-01 (auto-remediation overview) が 24h サマリを提供する。

### 1.3 非スコープ (out of scope, v0.6.x follow-up)

- 実 HTTP / SQL 実行 (現在は dry-run)
- ChatOps 統合 (Slack `/remediation` コマンド)
- 容量予測 ML モデル
- DR 訓練 (Phase 8 全体でも継続)
- 既存 5 以外の alert の自動修復

---

## 2. アーキテクチャ

### 2.1 コンポーネント

```
┌─────────────────┐  webhook   ┌──────────────────────────┐
│  Alertmanager   │ ─────────▶ │  ada-remediation HTTP    │
│ (Phase 6)       │  POST      │  POST /webhook/...       │
└─────────────────┘            └────────────┬─────────────┘
                                            │
                                            ▼
                              ┌──────────────────────────┐
                              │  RemediationEngine       │
                              │  evaluate(alert)         │
                              │   → Vec<RemediationAction>│
                              │  execute(action)         │
                              │   → ActionOutcome        │
                              └────────────┬─────────────┘
                                           │
                  ┌────────────────────────┼────────────────────────┐
                  ▼                        ▼                        ▼
        ┌──────────────┐         ┌──────────────┐         ┌──────────────┐
        │ MemoryStore  │         │ runbooks     │         │ PG (Phase 8) │
        │ (fast path)  │         │ config/*.json│         │ remediation_ │
        │ cooldowns +  │         │              │         │ history /    │
        │ history      │         │              │         │ cooldowns    │
        └──────────────┘         └──────────────┘         └──────────────┘
```

### 2.2 状態機械

```
             evaluate()                      all steps OK
   Idle ────────────────▶ Evaluating ────────────────────▶ Cooldown
                               │
                               │ step fails
                               ▼
                           Executing
                               │
                   ┌───────────┼───────────┐
                   ▼           ▼           ▼
               Failed     Retrying     Cooldown
            (max_retries  (backoff)    (window elapses
             exhausted)                → back to Idle)
```

`crates/ada-remediation/src/state.rs` の `EngineState::can_transition_to` が合法エッジを定義する。不正な遷移はテストで弾く (state.rs に 5 ケースの unit test)。

### 2.3 データモデル

```rust
pub struct RemediationAction {
    pub id: String,                    // "disk-space-low"
    pub name: String,                  // human-readable
    pub trigger: Trigger,              // Exact("ServiceDown") | Glob("SLIBurn*")
    pub severities: Vec<String>,       // ["P1"] / [] = all
    pub steps: Vec<ActionStep>,        // ordered
    pub cooldown: Duration,            // 300s default
    pub max_retries: u32,              // 2 default
}

pub enum ActionStep {
    RunCommand { cmd, args, timeout_secs },
    HttpCall   { url, method, body, headers },
    PgFunction { name, args },
    NotifySlack { channel, message },
    PageOperator { severity, runbook_url },
    Sequence { steps: Vec<ActionStep> },
}
```

### 2.4 永続層

- `remediation_history (id, action_id, alert_name, executed_at, outcome, retry_count, error_msg)` — 監査ログ
- `remediation_cooldowns (action_id, last_executed_at, cooldown_seconds, expires_at)` — 永続 cooldown
- `remediation_record_execution(...)` — 冪等 history 書込 + succeeded 時 UPSERT cooldown
- `remediation_check_cooldown(action_id) → bool` — 永続 cooldown 問い合わせ

Migration は `db/migrations/V003__phase8_remediation.sql` (実存の slot 番号; task spec は "V006" だが本仓の現存 migration は V001 + V002 のみ)。

---

## 3. runbook ファイル形式

### 3.1 基本構造

```json
{
  "version": 1,
  "actions": [
    {
      "id": "disk-space-low",
      "name": "Disk space low on {{ $labels.instance }}",
      "trigger": "DiskSpaceFillingFast",
      "severities": ["P2", "P3"],
      "steps": [
        { "kind": "run_command", "cmd": "du", "args": ["-sh", "/var/log"], "timeout_secs": 30 }
      ],
      "cooldown": 1800,
      "max_retries": 1
    }
  ]
}
```

トップレベル `version: 1` は schema version。`_comment` フィールドは許可 (デシリアライザは unknown field を無視) で、runbook ごとの設計判断を残すのに使う。

### 3.2 trigger

- `Trigger::Exact("ServiceDown")` — 厳密一致
- `Trigger::Glob("SLIBurn*")` — シェル風 glob。`*` 任意の run、`?` 任意の 1 文字。character class はサポート外 (hand-rolled matcher; crate 追加を最小化するため)

`severities` フィルタが空のとき、severity によらずマッチする。`["P1"]` のように指定すると、`alert.labels.severity` がその値のいずれかである alert のみマッチする。

### 3.3 step

各 step は `kind` フィールドでタグされた enum バリアント。6 種類:

| kind | 用途 | Phase 8 v0.6.0 実動作 |
|---|---|---|
| `run_command` | shell コマンド実行 | **実実行** (`tokio::process::Command` + timeout) |
| `http_call` | 内部 control plane への HTTP 呼び出し | dry-run (intent 記録) |
| `pg_function` | PL/pgSQL 関数呼び出し | dry-run |
| `notify_slack` | Slack 投稿 | dry-run |
| `page_operator` | ページャ送信 | dry-run |
| `sequence` | 複数 step の合成 | 内部的に inline 展開 |

dry-run path の出力は `ActionOutcome::step_results[i].message` に `"dry-run <kind> ..."` として残る。実 executor (Phase 8 v0.6.x follow-up) はトレイト `Executor` を実装し、`RemediationEngine::execute` 内でこのトレイトに delegate する。

### 3.4 テンプレート変数

`message` と `name` フィールド内で `{{ $labels.X }}` プレースホルダを alert の labels で置換する。**未知の placeholder はそのまま残す** (destination メッセージで「label 不足」が自明になる)。

例: `message = "service={{ $labels.service }} cluster={{ $labels.cluster }}"` → `service=m13-api-gateway cluster=prod-us-east-1`

`{{ $labels.absent }}` のような未知キーは置換されず `{{ $labels.absent }}` のまま destination に届く。

### 3.5 JSON vs. YAML

Phase 8 design 草案は "YAML" と書いていたが、v0.6.0 実装は **JSON (`.json`)** を採用した。理由は offline build 環境 (reqwest / sqlx に加え `serde_yaml` も `Cargo.lock` に存在しない) の制約。JSON は YAML の strict subset なので、同じ shape は任意の YAML リーダで parse 可能 (YAML 1.2 仕様)。`serde_yaml` が workspace に追加されれば、loader を 1 行で swap できる。

---

## 4. cooldown とリトライ戦略

### 4.1 永続 cooldown (source of truth)

PG テーブル `remediation_cooldowns` が source of truth。プロセス再起動 / replica を跨いで同じ cooldown が見える。`remediation_check_cooldown(action_id)` で「cooldown 生效中なら true」を返す。

### 4.2 in-memory cooldown (fast path)

`MemoryStore` (in `crates/ada-remediation/src/history.rs`) が同等の判定を lock-free read で提供する。production wiring はまず memory を見て、`true` なら即 skip、`false` なら PG に fallback する (double-checked locking 相当)。

### 4.3 cooldown 値の選択

| 結果 | 永続 cooldown | memory cooldown | 理由 |
|---|---|---|---|
| succeeded | `runbook.cooldown` (default 300s) | 同じ | 短すぎると retry storm |
| failed | 10s (application 層 + PG 層デフォルト) | 10s | flapping を即黙らせない |
| skipped | (書き込まない) | (書き込まない) | cooldown 中の再評価 |

`failed` の 10s は PL/pgSQL `remediation_record_execution(..., p_cooldown_seconds)` 引数で上書き可能だが、v0.6.0 の production wiring では固定 10s を使う。

### 4.4 リトライ

`max_retries` は `RemediationAction` のフィールド。`0` = 再試行しない (page-and-forget)、`1` = 1 回再試行、`2` = 2 回再試行 (合計 3 attempt)。各 attempt は別 `remediation_history` 行として記録される (`retry_count` で識別)。

retry の backoff は **直線** (attempt n の前に n × 5s 待機)。指数 backoff は Phase 8 v0.6.x follow-up。

---

## 5. デフォルト runbook 一覧

`config/remediation/` 配下の 5 ファイル:

| id | trigger | severity | step 主要 | cooldown |
|---|---|---|---|---|
| `disk-space-low` | `DiskSpaceFillingFast` | P2/P3 | du → find -delete → notify | 1800s |
| `service-down-restart-and-page` | `ServiceDown` | P1 | pg restart → page high | 600s |
| `db-pool-exhausted-kill-idle` | `DBConnectionPoolExhausted` | P2/P3 | pg kill_idle → notify | 900s |
| `slo-burn-fast-page` | `SLIBurnRateFast` | P1 | page high のみ | 3600s |
| `slo-burn-slow-notify` | `SLIBurnRateSlow` | P2/P3 | notify `#ada-warnings` | 7200s |

### 5.1 disk-space-low

- **想定**: `/var/log` が肥大化。古い `.gz` を消して free space を戻す
- **ステップ詳細**:
  1. `du -sh /var/log` (sizing — 出力は step_results に残る)
  2. `find /var/log -type f -name '*.gz' -mtime +7 -delete` (7 日以上前の .gz 削除)
  3. `#ada-ops` に Slack 通知 (dry-run)
- **cooldown 1800s** = 30 分。disk fill rate が高い alert なので、再評価までの余裕を持たせる
- **誤適用リスク**: 低 (削除対象は 7 日以上前の .gz のみ)

### 5.2 service-down-restart-and-page

- **想定**: P1 service が落ちた。restart を試行しつつ、並行して on-call を page
- **ステップ詳細**:
  1. `remediation_restart_service(service, instance)` pg function (dry-run)
  2. `page_operator` high severity with runbook URL
- **cooldown 600s** = 10 分。restart に失敗したら 10 分以内に再 page しない (operator が対応中のため)
- **誤適用リスク**: 中 (restart がユーザーの transaction を切る可能性)。`p1` severity に限定し、`page` を必ず併用

### 5.3 db-pool-exhausted-kill-idle

- **想定**: DB connection pool が枯渇。idle な session を kill して pool を空ける
- **ステップ詳細**:
  1. `remediation_kill_idle(database, 300)` — 5 分以上 idle な session を kill (dry-run)
  2. `#ada-ops` 通知
- **cooldown 900s** = 15 分。idle な session は継続的に再生成されるので、間隔をあけて様子を見る
- **誤適用リスク**: 低 (idle session kill は進行中 transaction を持たないことが前提)

### 5.4 slo-burn-fast-page

- **想定**: SLO budget が 1h で 2% 以上消費 (14.4× burn)。on-call page のみ
- **ステップ詳細**:
  1. `page_operator` high severity
- **cooldown 3600s** = 1 時間。sustained burn 中に double-page しない
- **誤適用リスク**: ゼロ (page だけ。コード実行なし)

### 5.5 slo-burn-slow-notify

- **想定**: SLO budget が 6h で 5% 消費 (6× burn)。Slack 通知
- **ステップ詳細**:
  1. `#ada-warnings` に Slack 投稿
- **cooldown 7200s** = 2 時間
- **誤適用リスク**: ゼロ

---

## 6. HTTP API

axum 0.7 ベースの in-process HTTP server。`crates/ada-remediation/src/http.rs` に全 handler 定義。

| Method | Path | 用途 |
|---|---|---|
| `GET`  | `/health` | liveness probe |
| `POST` | `/webhook/alertmanager` | Alertmanager v4 payload 受信 |
| `GET`  | `/remediation/history` | 実行履歴 (query: `action_id`, `since`, `limit`) |
| `GET`  | `/remediation/cooldowns` | live cooldown 一覧 |
| `POST` | `/remediation/trigger` | operator-trigger / dashboard "run now" |

### 6.1 webhook レスポンス

```json
{
  "received": 1,     // 受信した alert 数
  "matched": 1,      // マッチした action 数
  "executed": 1,     // 実際に実行した数 (cooldown skip 除く)
  "skipped": 0,      // cooldown / no-match で skip した数
  "outcomes": [...]  // ActionOutcome の配列
}
```

`outcomes[i].step_results[]` に per-step 結果 (index, kind, status, message, duration_ms) を含む。

### 6.2 manual trigger

`POST /remediation/trigger` body:

```json
{
  "alert_name": "DiskSpaceFillingFast",
  "labels": { "instance": "host-42", "severity": "P2" },
  "severity": "P2",
  "force": false
}
```

`force: true` で cooldown を bypass する (緊急 operator 介入用)。

---

## 7. セキュリティと秘匿

### 7.1 secret 取扱

`docs/observability/09-security-design.md` §4 に従い、**secret は env var のみ**。runbook ファイルには **絶対** 書かない:

- Slack webhook URL → `$SLACK_WEBHOOK_WARNINGS` 等
- PagerDuty routing key → `$PAGERDUTY_ROUTING_KEY`
- DB 接続情報 → `$DATABASE_URL`
- SMTP パスワード → `$SMTP_AUTH_PASSWORD`

runbook で `channel: "#ada-ops"` と書くと、executor が `SLACK_WEBHOOK_OPS` env var から URL を解決する。`PageOperator::runbook_url` も `https://runbooks.ada.local/...` のような **公開 URL のみ**。社内 SSO で守られた runbook サイトでも URL 自体は secret ではない。

### 7.2 webhook 認証

`/webhook/alertmanager` は v0.6.0 では **無認証**。同一 namespace 内の Alertmanager Pod からのみ到達できるネットワーク設計を前提とする (k8s NetworkPolicy / Service mesh で外向き遮断)。外部公開する場合は IP allowlist or HMAC 署名ヘッダの追加を v0.6.x follow-up で行う。

### 7.3 authorization

`/remediation/trigger` は dashboard "run now" ボタン用。operator 権限を要求すべきだが、v0.6.0 では **Grafana 側の proxy auth** に依存 (Grafana 認証済みユーザのみが当該 panel を表示できる) する。実 API 直叩きは Phase 8 v0.6.x で HMAC トークン or OAuth2 を追加する。

### 7.4 監査ログ

すべての `evaluate → execute` フローは `remediation_history` に書かれる。`alert_name` カラムから元の Alertmanager alert までトレース可能。RPO/RTO は PG バックアップ戦略 (Phase 6 既存) に従う。

---

## 8. テスト方針

### 8.1 単体テスト (per-module, `#[cfg(test)]`)

`crates/ada-remediation/src/` 配下の各モジュールに `tests` サブモジュール:

| モジュール | テスト | 件数 |
|---|---|---|
| `alert.rs` | builder, severity, template rendering | 3 |
| `action.rs` | glob matcher, trigger, default timeout | 5 |
| `state.rs` | happy-path, retry, illegal transitions | 5 |
| `config.rs` | parse, schema version, dir loading | 3 |
| `history.rs` | success, cooldown expiry, failure, history query | 5 |
| `engine.rs` | exact match, glob, severity filter, dry-run exec, short-circuit | 6 |
| `http.rs` | health, webhook dispatch, unknown alert, history endpoint | 4 |
| `lib.rs` (doctest) | quick start | 1 |
| **小計** | | **32** |

### 8.2 統合テスト (`tests/remediation_e2e.rs`)

`[[test]]` ターゲットとして宣言。フル E2E:

1. `load_runbooks_from_dir("config/remediation/")` で 5 ファイルを読み込み
2. `RemediationEngine::with_runbooks(...)` 構築
3. `AlertmanagerPayload` (本番 webhook と同じ shape) を `POST /webhook/alertmanager` に送信
4. レスポンス shape, history rows, cooldown state を assert
5. 同じ alert を **再送** して、cooldown による skip を assert
6. 異なる severity の alert 送信で、severity フィルタの動作を assert

### 8.3 PL/pgSQL テスト (`db/tests/V003__phase8_remediation_test.sql`)

7 ケース (SAVEPOINT 単位):

- t_tables_exist
- t_record_success (succeeded → cooldown 行も書かれる)
- t_record_failure (failed → cooldown 行は書かれない)
- t_record_cooldown_idempotent (UPSERT で 1 行のみ)
- t_record_invalid (空 / 非法 outcome / 負 retry / 0 cooldown)
- t_check_cooldown_active / inactive
- t_outcome_chk (CHECK 制約の挙動)

`make -C db test` で実行。

### 8.4 五門 baseline

```
cargo check  --workspace --all-targets
cargo test   --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt    --all -- --check
cargo clippy --workspace
```

5 門すべて GREEN を v0.6.0 release gate とする。

---

## 9. 既知の制約

### 9.1 ~~dry-run default~~ (v0.7.0 で解消)

§1.2 参照。`HttpCall` / `PgFunction` / `NotifySlack` / `PageOperator` の **dry-run 動作**は v0.7.0 で **trait 化** された (`StepExecutor` trait + `DryRunExecutor` + `RealExecutor`)。`RealExecutor::with_logging_client()` 経由の in-memory 検証は v0.7.0 で完了。`reqwest` ベースの本実装は v0.7.1 follow-up。

### 9.2 ~~no Prometheus metrics~~ (v0.7.0 で解消)

エンジンから Prometheus への直接 export は v0.7.0 で実装。`crates/ada-remediation/src/metrics.rs` に `metrics 0.24` + `metrics-exporter-prometheus 0.18` ベースの recorder を追加。`GET /metrics` で以下 4 メトリクスを公開:

- `ada_remediation_actions_total{action_id, outcome}` — Counter
- `ada_remediation_action_duration_seconds{action_id}` — Histogram
- `ada_remediation_engine_state_transitions_total{from, to}` — Counter
- `ada_remediation_cooldown_active` — Gauge

Grafana dashboard 80-01 は Prometheus 経由のスクレイプに移行可能 (PG 直 query は v0.7.0 互換のため残置)。

### 9.3 ~~no hot-reload~~ (v0.7.0 で polling fallback)

`config/remediation/*.json` の更新を **プロセス再起動なし** で反映。`crates/ada-remediation/src/watcher.rs` で 5s polling + 500ms debounce を実装。`Engine::reload_runbooks(new)` で in-process な表 swap。`notify` crate (FS event watcher) は offline cache 制約で取得不可のため polling で代用。最大 5 秒の staleness あり。`notify` 移行は v0.7.1 follow-up。

### 9.4 ~~no webhook auth~~ (v0.7.0 で shared-secret 化)

`POST /webhook/alertmanager` と `POST /remediation/trigger` を `X-Webhook-Token` header 必須に変更。`crates/ada-remediation/src/auth.rs` で `constant_time_eq 0.4.2` ベースの比較。`REMEDIATION_WEBHOOK_SECRET` env var 起動時読み込み、missing で **fail-closed 503** (本番運用安全)。HMAC-SHA256 + per-request nonce 化は v0.7.1 follow-up (`hmac` / `sha2` crate は offline cache 不可)。

### 9.5 1 replica 前提 (継続 — 解消せず)

in-memory `MemoryStore` はプロセスローカル。複数 replica で運用する場合、永続 cooldown (`remediation_cooldowns` テーブル) を **必ず** 通すこと。`MemoryStore` は fast path のみ。v0.7.0 では `runbooks` フィールドを `Arc<RwLock<Vec<...>>>` 化したので **runbook 自体は replica 横断で同期不要** (ファイルシステム共有が前提)。`MemoryStore` の replica 間同期は v0.7.x follow-up。

### 9.6 V003 vs. V006 naming

`db/migrations/V003__phase8_remediation.sql` の slot 番号は、task spec の "V006" ではなく **V003** (現存 V001 + V002 の次の slot)。`make -C db migrate` は `V*.sql` glob なのでファイル名ベースで順序解決する。v0.7.0 で **新 migration は追加なし** (SLO 7.5 multi-service 化は application 層で完結、PL/pgSQL 変更なし)。

---

## 10. 用語集

| 用語 | 説明 |
|---|---|
| **Runbook** | 宣言的リメディエーション設定ファイル (1 JSON = 1+ action) |
| **Action** | 1 つの alert → 1 つの step 列 + cooldown + retry |
| **Step** | 6 種類の enum バリアントの 1 つ |
| **Cooldown** | 同じ action の再評価を拒否する時間窓 |
| **Dry-run** | intent を記録するだけで副作用を実行しないモード |
| **Trigger** | alert_name のマッチパターン (Exact or Glob) |
| **EngineState** | Idle / Evaluating / Executing / Cooldown / Failed / Retrying |
| **Fast path** | in-memory MemoryStore での cooldown 判定 (lock-free read) |
| **Source of truth** | PG テーブル (`remediation_cooldowns`) での永続 cooldown |

---

## 11. 参考文献

1. Google SRE Workbook — Implementing SLOs
   <https://sre.google/workbook/implementing-slos/>
2. Google SRE Book — Eliminating Toil
   <https://sre.google/sre-book/eliminating-toil/>
3. Alertmanager webhook payload spec
   <https://prometheus.io/docs/alerting/latest/configuration/#webhook_config>
4. Ada `docs/observability/07-alert-policy.md` — alert 命名と severity mapping
5. Ada `docs/observability/08-slo-design.md` — SLIBurnRateFast/Slow の origin
6. Ada `docs/observability/09-security-design.md` §4 — secret 取扱
7. Ada `docs/observability/11-phased-rollout.md` §10 — Phase 8 scope
8. axum 0.7 documentation
   <https://docs.rs/axum/0.7.9/axum/>

---

## 12. v0.7.0 ハードニング完了サマリ (Phase 8.5)

> 本セクションは v0.7.0 で実装した 5 atomic commits の **変更点の俯瞰**。  
> 詳細は [`11-phased-rollout.md §11`](11-phased-rollout.md) および [08-slo-design.md §11](08-slo-design.md) / [15-error-budget-policy.md](15-error-budget-policy.md) を参照。

### 12.1 Real executor (commit 0, 前任 worker `31213a7`)

- `StepExecutor` trait を `async-trait 0.1.92` ベースで object-safe に定義
- `DryRunExecutor` (default) / `RealExecutor` (env-var fallback) / `LoggingClient` (in-memory request 記録) を実装
- `HttpCall` / `PgFunction` / `NotifySlack` / `PageOperator` 4 step の dry-run 動作を trait 化、将来 `reqwest` / `sqlx` 実装に swap 可能
- 既定 executor は `DryRunExecutor` で v0.6.0 動作を完全互換

### 12.2 Prometheus exporter + `/metrics` endpoint (commit-1)

- `crates/ada-remediation/src/metrics.rs` — 4 メトリクス + 7 unit test
- `GET /metrics` — `text/plain; version=0.0.4` content-type で公開
- engine.execute 内で state transition + step outcome + duration を記録
- cooldown gauge は HTTP scrape 時に live in-memory store から再計算 (cheap)

### 12.3 Hot-reload watcher (commit-2, polling fallback)

- `crates/ada-remediation/src/watcher.rs` — 5s polling + 500ms debounce
- `Engine::reload_runbooks(Vec<RemediationAction>)` で in-process swap
- 3 unit test (file_addition / file_modification / debounce 5→1)
- `notify` crate は offline cache 制約で取得不可 → polling 採用、最大 5 秒の staleness は §9.3 既知の制約に転記

### 12.4 Webhook shared-secret auth (commit-3)

- `crates/ada-remediation/src/auth.rs` — `AuthState` (enabled/disabled/from_env)
- `constant_time_eq 0.4.2` (blake3 の transitive dep) ベース比較
- 起動時 `REMEDIATION_WEBHOOK_SECRET` env var 読み込み、missing で `tracing::warn!` + 503 fail-closed
- 6 unit test (valid / invalid / missing / disabled / require_enabled panic / silent) + 3 HTTP e2e test
- HMAC-SHA256 化は v0.7.1 follow-up

### 12.5 Manual trigger auth (commit-4)

- `POST /remediation/trigger` に `X-Webhook-Token` 必須化 (webhook と同じ secret 共有)
- `force=true` で cooldown bypass 可能なため、webhook と同等の脅威モデルで gating
- 2 HTTP unit test (requires_token / accepts_valid_token)

### 12.6 SLO 7.5 + Error Budget policy (commit-5)

- [08-slo-design.md §11](08-slo-design.md) — SLI-005~008 / SLO-004~006 (Auto-remediation 専用)
- [15-error-budget-policy.md](15-error-budget-policy.md) (新) — 5 段階 Error Budget 行動マトリクス + 4 段階 Burn Rate 行動プロトコル + クロスリージョン挙動
- `config/alertmanager/slo-rem-fast-burn-{1h,6h}.yaml` + `slo-rem-slow-burn-{24h,72h}.yaml` — PrometheusRule 4 ファイル

### 12.7 既知の制約 (v0.7.0 リリース時点)

| # | 内容 | 計画 |
|---|---|---|
| C-001 | `RealExecutor` の `NetworkClient` は `LoggingClient` のみ。`reqwest` ベースの本実装は v0.7.1 | v0.7.1 |
| C-002 | `notify` crate (FS event watcher) 不可。`polling` 5s + debounce で代用 | v0.7.1 |
| C-003 | Webhook 認証は shared-secret のみ。HMAC-SHA256 + per-request nonce は v0.7.1 | v0.7.1 |
| C-004 | k8s deployment manifest (Helm chart 同期 / NetworkPolicy) は本 commit では未実装 | v0.7.1 |
| C-005 | `reqwest` / `sqlx` / `prometheus` 直接依存は offline cache 制約で不可。`metrics-exporter-prometheus` 0.18 経由で同等機能を実現 | 制約継続 |

> 制約の詳細は [11-phased-rollout.md §11.4](11-phased-rollout.md) を参照。

---

> **IPA 末尾注記**  
> 本ドキュメントは IPA 共通フレーム2018 (SLCP-JCF2018) 第 7 章「システム化計画の立案」に準拠する。
> Phase 8 (Auto-remediation) の v0.6.0 リリースに同期して作成。v0.7.0 で Phase 8.5 SRE ハードニングを完了し、本番投入可能な品質に引き上げた (制約 C-001~C-005 は v0.7.1 で解消予定)。
> PO（プロダクトオーナー）の承認と SRE Lead の技術承認を必須とする。
---

## 13. v0.7.1 ハードニング完了サマリ (Phase 8.5 production-ready)

v0.7.0 で残った既知制約（§9.7 参照）のうち、主要 4 件を v0.7.1 で解消。残 1 件（`notify` crate / IETF HMAC-SHA256）は v0.7.2 / v0.8.0 へ送る。

### 13.1 HMAC-SHA256 webhook auth (commit-1, `a386006`)

v0.7.0 の shared-secret header 認証を HMAC-SHA256 + replay protection に格上げ:

- `crates/ada-remediation/src/auth.rs` (+510 行)
  - `sign(secret, payload) -> hex` (blake3 keyed_hash + 手書き hex encode)
  - `verify(secret, payload, sig_hex) -> bool` (constant_time_eq)
  - `now_unix_secs() -> i64`
  - `verify_request(headers, secret, payload) -> Result<(), AuthError>`
  - `AuthError` バリアント追加: `MissingSignature` / `InvalidSignature` / `MissingTimestamp` / `Expired`
  - 5 unit tests: deterministic / valid / tampered / wrong secret / replay rejected
- `crates/ada-remediation/src/http.rs` (+460 行)
  - `handle_alertmanager_webhook` を **生 body バイト** + signature 検証に書き換え
  - manual trigger エンドポイントも同様に
  - 4 webhook E2E tests を新スキームに更新
- `crates/ada-remediation/src/executor.rs` (+32 行)
  - `LoggingClient::sign_request(secret, payload) -> String` (client 側 helper)
- root hotfix: `is_multiple_of` 1.98 対応 + `now_unix_secs` を `map_or` に + cargo fmt
- 既知ギャップ: `blake3 keyed_hash` は HMAC 標準ではない。strict compliance 監査では v0.7.2 で `hmac` + `sha2` crate ship 後に IETF HMAC-SHA256 へ切替予定。

### 13.2 Hot-reload polling 1s 強化 (commit-2, `338bb7b`)

v0.7.0 の polling 5s → 1s に短縮:

- `crates/ada-remediation/src/watcher.rs`
  - `DEFAULT_INTERVAL: 5s -> 1s`
  - doc comment で「`notify` crate は offline cache に未収録」の事実と CPU コスト見積を明記
- 既存 3 unit tests (file_addition / file_modification / debounce) は interval 短縮でより高速に通過
- 既知ギャップ: 真の `notify` crate 採用は v0.7.2 へ送る

### 13.3 k8s deployment manifest (commit-3, `b6411f5`)

`deploy/k8s/` 配下に 3 ファイル新規:

- `ada-remediation.yaml` (5999 bytes)
  - **ConfigMap**: 環境変数 (REMEDIATION_RUNBOOK_DIR / REMEDIATION_BIND_ADDR / RUST_LOG)
  - **Secret**: webhook + trigger secret (stringData **PLACEHOLDER** 値)
  - **Deployment**: 2 replicas / rolling update / securityContext (runAsNonRoot=65532, seccomp=RuntimeDefault, readOnlyRootFilesystem, drop ALL caps) / resources (100m/128Mi req, 500m/512Mi limit) / liveness + readiness probe / terminationGracePeriodSeconds=30
  - **Service**: ClusterIP 9100
  - **NetworkPolicy**: ingress from observability ns + prometheus pod / egress all ns 80/443 + kube-system DNS
- `kustomization.yaml` (1083 bytes): namespace=observability, kustomize 共通 label 付与
- `README.md` (5190 bytes): prerequisites / secret bootstrap 3 選択肢 / apply 手順 / HMAC curl 検証例

yaml 静的検証: python `yaml.safe_load_all` で 5 kinds パース成功。`kubectl --dry-run=client` は D:/Ada に cluster がないため CI へ送る。

### 13.4 Production wiring (commit-4, `e98975f`)

`crates/ada-remediation/src/main.rs` 新規 (362 行):

- **tokio::main** バイナリ入口（`#[cfg(feature = "bin")]` gate）
- 環境変数: REMEDIATION_BIND_ADDR (default 0.0.0.0:9100) / REMEDIATION_RUNBOOK_DIR (default ./config/remediation) / REMEDIATION_WEBHOOK_SECRET (required) / REMEDIATION_TRIGGER_SECRET (required) / RUST_LOG
- 起動シーケンス: tracing → metrics::install → AuthState/Engine/Store → disk load → Watcher spawn → 1s polling 経由で reload_runbooks → axum serve + with_graceful_shutdown
- **Graceful shutdown**: SIGINT + SIGTERM (k8s preStop) → 25s drain
- 5 unit tests (workspace -F unsafe-code 制約下の静的アサート方式)

### 13.5 既知の制約 (v0.7.1 リリース時点)

| 制約 | 解消計画 |
|---|---|
| blake3 keyed_hash は HMAC 標準ではない | v0.7.2 で `hmac` + `sha2` ship 後に IETF HMAC-SHA256 切替 |
| `notify` crate offline cache 不在 | v0.7.2 で `cargo ship` または CI 経由で採用 |
| Runbook ConfigMap 読み取り専用 | k8s では CSI-backed RWX volume または Reloader sidecar で代替 |
| kubectl dry-run 未実施 | CI で k8s 接続時に実施 |
| TRIGGER_SECRET と WEBHOOK_SECRET が現状同じ AuthState を共有 | v0.7.2 で AuthState を 2 secret 受け取りに拡張 |
