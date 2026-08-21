# 04 ロギング設計（Logging Design）

> 構造化ログ（JSON）+ 自動脱敏 + サンプリング + ライフサイクル管理。**GDPR/PIPL 対応**で PII ゼロトレランス。

> **ドキュメントID**：DOC-OBS-004
> **上位文書**：[DOC-OBS-INDEX](README.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版 |

---

## 目次

1. 設計目標
2. フォーマット
3. 必須フィールド
4. ログレベル戦略
5. PII 脱敏
6. ライフサイクル
7. サンプリング
8. 業務別ログ要件
9. コード規約
10. 用語集

---

## 1. 設計目標

| 目標 | 説明 |
|---|---|
| **構造化** | JSON Lines、grep 可能 |
| **機械可読** | Loki / Grafana がパース可能 |
| **相関可能** | `trace_id` / `request_id` で metric/log 紐付け |
| **PII ゼロ** | 自動脱敏、CI 検証 |
| **高性能** | < 1ms ログ 1 件 |
| **完全性** | クラッシュ時も失わない（非同期 + buffer） |

## 2. フォーマット

### 2.1 JSON Lines（NDJSON）

各ログ 1 行 JSON：

```json
{"timestamp":"2026-08-20T10:00:00.123Z","level":"INFO","service":"ada-m13-api-gateway","version":"0.1.0","environment":"production","instance":"ada-m13-api-gateway-7d8f-abc","trace_id":"4bf92f3577b34da6a3ce929d0e0e4736","span_id":"00f067aa0ba902b7","request_id":"a1b2c3d4-e5f6-7890","module":"api_gateway::auth","event":"jwt_validated","duration_ms":12,"tenant_id_hash":"t_a1b2c3","user_id_hash":"u_x9y8z7","result":"success","message":"JWT validated for tenant t_a1b2c3"}
```

### 2.2 ライブラリ

```toml
# Cargo.toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json", "fmt"] }
tracing-bunyan-formatter = "0.3"  # 代替: tracing-subscriber JSON layer
```

## 3. 必須フィールド

| フィールド | 型 | 必須 | 説明 | 例 |
|---|---|---|---|---|
| `timestamp` | string (RFC 3339 nano) | ✅ | ISO 8601 UTC | `2026-08-20T10:00:00.123Z` |
| `level` | string | ✅ | `TRACE` / `DEBUG` / `INFO` / `WARN` / `ERROR` | `INFO` |
| `service` | string | ✅ | crate 名 | `ada-m13-api-gateway` |
| `version` | string | ✅ | semver | `0.1.0` |
| `environment` | string | ✅ | `production` / `staging` / `dev` | `production` |
| `instance` | string | ✅ | Pod 名 | `ada-m13-api-gateway-7d8f-abc` |
| `trace_id` | string (32 hex) | 条件付き | W3C trace context | `4bf92f3577b34da6a3ce929d0e0e4736` |
| `span_id` | string (16 hex) | 条件付き | W3C trace context | `00f067aa0ba902b7` |
| `request_id` | UUID v7 | 条件付き | HTTP request 単位 | `a1b2c3d4-e5f6-7890` |
| `module` | string | ✅ | Rust module path | `api_gateway::auth` |
| `event` | string | ✅ | 機械可読イベント名（snake_case） | `jwt_validated` |
| `message` | string | ✅ | 人間可読メッセージ | `JWT validated for tenant t_a1b2c3` |
| `tenant_id_hash` | string | 条件付き | ハッシュ化テナント ID | `t_a1b2c3` |
| `user_id_hash` | string | 条件付き | ハッシュ化ユーザー ID | `u_x9y8z7` |
| `error_code` | string | 条件付き | [DOC-API-003](../api/error-codes.md) エラーコード | `AUTH_FAILED` |
| `duration_ms` | number | 条件付き | 処理時間（ミリ秒） | `12` |
| `http.method` | string | 条件付き | HTTP メソッド | `POST` |
| `http.path` | string | 条件付き | URL パス（**クエリ除く**） | `/api/v1/canvases` |
| `http.status` | number | 条件付き | HTTP ステータスコード | `201` |

## 4. ログレベル戦略

### 4.1 レベル定義

| レベル | 用途 | 出力 | サンプル |
|---|---|---|---|
| `ERROR` | 即時対応必要（[Sev1/Sev2](../templates/05-operations.md)） | Loki + Alert | `database connection failed` |
| `WARN` | 注意が必要（Sev3） | Loki | `deprecated API endpoint used` |
| `INFO` | 重要な状態変化、業務イベント | Loki | `user logged in`, `pipeline started` |
| `DEBUG` | 開発時のデバッグ情報 | Loki（dev/staging のみ） | `query parameters` |
| `TRACE` | 詳細な内部状態 | 開発時のみ、**本番では出力しない** | `internal state transition` |

### 4.2 環境別設定

```bash
# Production
RUST_LOG=info,ada_app_api_gateway=info,ada_app_tenant_middleware=warn

# Staging
RUST_LOG=debug,ada_app_api_gateway=debug

# Development
RUST_LOG=trace
```

### 4.3 構造化イベント名（`event` フィールド）

| 業務イベント | event 名 | level |
|---|---|---|
| ユーザー認証成功 | `auth_succeeded` | INFO |
| 認証失敗 | `auth_failed` | WARN |
| キャンバス作成 | `canvas_created` | INFO |
| キャンバス削除 | `canvas_deleted` | INFO |
| データ取得開始 | `acquisition_started` | INFO |
| データ取得成功 | `acquisition_succeeded` | INFO |
| データ取得失敗 | `acquisition_failed` | ERROR |
| プラグイン読み込み | `plugin_loaded` | INFO |
| atomic swap 成功 | `module_swap_succeeded` | INFO |
| atomic swap 失敗 | `module_swap_failed` | ERROR |
| イベントバス配信 | `event_published` | DEBUG |
| イベントコンシューム | `event_consumed` | DEBUG |
| イベント重複検知 | `event_duplicate_detected` | WARN |
| リーダー選出 | `leader_elected` | INFO |
| ハートビート失敗 | `heartbeat_failed` | WARN |
| クラスタスプリットブレイン | `split_brain_detected` | ERROR |
| GDPR 削除実行 | `gdpr_user_forgotten` | INFO（[audit_log](../modules/M-10-tenant-middleware.md) 連動） |
| 脆弱性検出 | `vulnerability_detected` | WARN |
| 設定変更 | `config_changed` | INFO |

## 5. PII 脱敏

### 5.1 禁止フィールド

❌ **絶対にログに含めてはいけない**：

| 禁止 | 理由 |
|---|---|
| パスワード（plain / hashed 以外） | 漏洩時完全破綻 |
| API キー | 同上 |
| JWT token (raw) | リプレイ攻撃 |
| OAuth refresh token | 同上 |
| Cookie 全体 | セッション乗っ取り |
| Authorization header | 同上 |
| クレジットカード番号 | PCI-DSS |
| マイナンバー / SSN | 各国法令 |
| メールアドレス（plain） | GDPR Art.4 個人情報 |
| 電話番号 | 同上 |
| 住所 | 同上 |
| 氏名 | 同上 |
| IP アドレス（v4 full） | GDPR、PIPL 該当の場合 |

### 5.2 脱敏戦略

| データ | 元 | 脱敏後 |
|---|---|---|
| Email | `user@example.com` | `u***@e****.com` |
| 電話番号 | `090-1234-5678` | `090-****-5678` |
| クレジットカード | `4111-1111-1111-1111` | `****-****-****-1111` |
| Tenant ID | UUID | SHA-256 hash の最初の 8 文字 |
| User ID | UUID | SHA-256 hash の最初の 8 文字 |
| JWT | `eyJhbGc...` | `eyJhbGc***`（先頭 8 文字 + マスク） |
| パスワード | `secret` | `[REDACTED]` |

### 5.3 実装（Rust）

```rust
use tracing::field;

#[instrument(skip(password, token))]
async fn login(req: LoginRequest) -> Result<Session, Error> {
    // 自動脱敏: skip でこれらのフィールドをログから除外
    info!(
        user_id = field::display(&req.user_id),
        email = field::display(mask_email(&req.email)),
        password = field::debug(&"[REDACTED]"),  // ダミー値
        "login attempt"
    );
    // ...
}
```

### 5.4 CI 検証

```yaml
# .github/workflows/log-check.yml
- name: PII Detection in Logs
  run: |
    # コードベースに hardcoded PII がないか
    ! grep -rE "(password|secret|api_key|token)\s*[:=]\s*[\"']" crates/
    # ログに plain email パターンがないか
    ! grep -rE "@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}" crates/*/src/**/*.rs
```

## 6. ライフサイクル

| 環境 | 保持期間 | サンプリング | 保管先 |
|---|---|---|---|
| Production | **30 日** | 100%（ERROR/WARN は必須） | Loki + 長期ストレージ (MinIO) |
| Staging | 14 日 | 100% | Loki |
| Development | 7 日 | 100% | Loki |
| Audit Log（業務） | 1 年（[SEC-19](../requirements/08-security-requirements.md)） | 100% | PostgreSQL `audit_log` テーブル |

### 6.1 コスト試算

| 項目 | 想定 | 計算 |
|---|---|---|
| ログ件数 / 日 | 100 万件（18 crate × 100 req/s × ピーク 5 時間） | 100 万 |
| 1 件あたりサイズ | 1 KB（JSON） | 1 GB/日 |
| 30 日保持 | 30 GB | — |
| 圧縮後 | ~10 GB | gzip ~70% |
| ストレージコスト | 無視できる | MinIO + 1TB ディスク ~$10/月 |
| 検索性能 | 7 日以内なら < 1s | Loki LogQL |

## 7. サンプリング

| レベル | サンプリング | 備考 |
|---|---|---|
| ERROR / WARN | **100% 必ず記録** | 監査必須 |
| INFO | 100%（業務イベント） | 主要イベント |
| INFO（ヘルスチェック等） | 1% | ノイズ削減 |
| DEBUG | dev/staging のみ | 本番 OFF |
| TRACE | dev のみ | 本番 OFF |

## 8. 業務別ログ要件

| 業務 | 必須 event | 必須フィールド | 保持 |
|---|---|---|---|
| 認証 | `auth_succeeded`, `auth_failed` | `user_id_hash`, `ip_hash`, `result` | 30 日 |
| 認可拒否 | `authorization_denied` | `user_id_hash`, `action`, `resource`, `reason` | 90 日（コンプラ） |
| データ変更 | `entity_created`, `entity_updated`, `entity_deleted` | `entity_type`, `entity_id`, `tenant_id_hash`, `user_id_hash` | 1 年（audit_log） |
| プラグイン実行 | `plugin_executed` | `plugin_id`, `module_id`, `duration_ms`, `result` | 30 日 |
| Atomic swap | `module_swap_*` | `module_id`, `from_version`, `to_version`, `result` | 90 日 |
| イベント配信 | `event_published`, `event_consumed`, `event_duplicate_detected` | `topic`, `event_id`, `result` | 30 日 |
| クラスタ | `leader_elected`, `split_brain_detected` | `node_id`, `old_leader`, `new_leader` | 90 日 |
| GDPR 削除 | `gdpr_user_forgotten` | `user_id_hash`, `requested_at`, `completed_at` | **永続** |
| 脆弱性 | `vulnerability_detected` | `cve_id`, `severity`, `affected_crate` | 1 年 |

## 9. コード規約

### 9.1 許可パターン

```rust
use tracing::{info, warn, error, instrument, Span};

#[instrument(
    name = "api_gateway.jwt_validate",
    skip(token),  // token はログから除外
    fields(
        tenant_id_hash = %tenant_id_hash,  // hash 化
        result = tracing::field::Empty,  // 後で設定
    )
)]
async fn validate_jwt(token: &str, tenant_id: &TenantId) -> Result<Claims, Error> {
    let start = std::time::Instant::now();
    
    match decode_jwt(token) {
        Ok(claims) => {
            Span::current().record("result", "success");
            info!(
                duration_ms = start.elapsed().as_millis() as u64,
                "JWT validated"
            );
            Ok(claims)
        }
        Err(e) => {
            Span::current().record("result", "fail");
            warn!(
                error_code = %e.code(),
                "JWT validation failed"
            );
            Err(e)
        }
    }
}
```

### 9.2 禁止パターン

```rust
// ❌ 禁止: 構造化なし
println!("User logged in: {}", user.email);  // email PII!

// ❌ 禁止: トークン全体
info!("Auth header: {}", auth_header);  // 漏洩リスク

// ❌ 禁止: パスワード
info!("Login attempt: {} / {}", username, password);  // 平文!

// ❌ 禁止: スタックトレース全体
error!("Error: {:#?}", error);  // 内部情報漏洩

// ❌ 禁止: カード番号
info!("Payment: {}", card_number);
```

## 10. 用語集

| 用語 | 説明 |
|---|---|
| 構造化ログ | JSON など機械可読形式 |
| NDJSON | Newline Delimited JSON |
| 脱敏 (Redaction) | 機密情報をマスク |
| Cardinality | ラベルの組み合わせ数 |
| Sampling | ログを間引く |
| TRACE ID | W3C Trace Context の 32 hex 文字列 |
| Span ID | W3C Trace Context の 16 hex 文字列 |
| GDPR | EU 一般データ保護規則 |
| PIPL | 中国個人情報保護法 |
| PII | Personally Identifiable Information |

## 11. 参考文献

1. OpenTelemetry Logs Semantic Conventions
2. Grafana Loki Best Practices
3. W3C Trace Context
4. GDPR Article 4, 5, 32
5. PIPL §4, §38, §51
6. Ada プロジェクトチーム「[DOC-REQ-SEC-001 セキュリティ要件](../requirements/08-security-requirements.md)」

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
