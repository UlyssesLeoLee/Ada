# 09 セキュリティ設計（Security Design）

> **観測基盤は攻撃面にならない**。  
> RBAC・認証・テナント分離・NetworkPolicy・Secret 管理・監査ログを統合的に設計し、  
> モニタリングシステムから情報漏洩しないことを保証する。

> **ドキュメントID**：DOC-OBS-009
> **上位文書**：[DOC-OBS-INDEX](README.md)
> **下位文書**：[DOC-OBS-002 Architecture](02-architecture.md) / [DOC-OBS-010 Deployment](10-deployment-design.md) / [DOC-REQ-008 Security](D:/Ada/docs/requirements/08-security-requirements.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版（ゼロトラスト + 多層防御 + GDPR/PIPL 準拠） |

---

## 目次

1. 設計原則
2. 攻撃面マトリクス
3. 認証・認可
4. テナント分離
5. ネットワークポリシー
6. シークレット管理
7. データ保護（PII / 暗号化）
8. 監査ログ
9. コンプライアンス
10. インシデント対応
11. 用語集
12. 参考文献

---

## 1. 設計原則

| 原則 | 説明 |
|---|---|
| **ゼロトラスト** | 内部ネットワークでも全通信を認証・暗号化 |
| **最小権限** | 各ロールに最小限の権限のみ付与 |
| **多層防御** | NetworkPolicy + RBAC + Secret + 監査 の多重化 |
| **テナント分離** | テナント間のメトリクス/ログ/トレースを完全分離 |
| **監査可能** | 全操作を audit log に記録、改ざん不可 |
| **GDPR / PIPL 準拠** | 個人データ最小化、削除権対応 |

## 2. 攻撃面マトリクス

| コンポーネント | 公開ポート | 認証 | 認可 | 暗号化 | 監査 |
|---|---|---|---|---|---|
| **Grafana** | 443 (HTTPS) | OIDC (Keycloak) | RBAC + Org | TLS 1.3 | ✅ 全操作 |
| **Prometheus** | 9090 (内部のみ) | Basic / mTLS | Read/Write ロール | TLS 1.2+ | ✅ クエリログ |
| **AlertManager** | 9093 (内部) | mTLS | Sender ACL | TLS 1.2+ | ✅ Alert 履歴 |
| **Loki** | 3100 (内部) | mTLS | Tenant ID | TLS 1.2+ | ✅ クエリログ |
| **Tempo** | 3200 (内部) | mTLS | Tenant ID | TLS 1.2+ | ✅ クエリログ |
| **OTel Collector** | 4317 (gRPC) / 4318 (HTTP) | mTLS / API Key | IP allowlist | TLS 1.2+ | ✅ 受信ログ |
| **Prometheus Node Exporter** | 9100 (内部) | mTLS | 読取専用 | TLS 1.2+ | ✅ |
| **PostgreSQL Exporter** | 9187 (内部) | mTLS | 読取専用 | TLS 1.2+ | ✅ |

> **重要**：Grafana のみ公開、他は全て `observability` namespace 内部 + NetworkPolicy で制限。

## 3. 認証・認可

### 3.1 Grafana 認証

```yaml
# grafana.ini (抜粋)
[auth]
mode = oauth

[auth.oauth]
name = Keycloak
enabled = true
client_id = grafana
client_secret = ${GRAFANA_OAUTH_SECRET}
auth_url = https://keycloak.ada.internal/auth/realms/ada/protocol/openid-connect/auth
token_url = https://keycloak.ada.internal/auth/realms/ada/protocol/openid-connect/token
api_url = https://keycloak.ada.internal/auth/realms/ada/protocol/openid-connect/userinfo

[auth.basic]
enabled = false
```

### 3.2 RBAC ロール定義

| ロール | 権限 | 対象ユーザー |
|---|---|---|
| **Grafana Admin** | 全権限、User / Org / DataSource 管理 | SRE Lead |
| **Grafana Editor** | Dashboard 作成 / 編集、Alert 作成 | SRE, SRE on-call |
| **Grafana Viewer** | 閲覧のみ | Dev, PM, QA |
| **Tenant Admin** | 自テナントデータのみ閲覧 + 編集 | 顧客管理者 |
| **Tenant Viewer** | 自テナントデータ閲覧のみ | 顧客一般 |
| **NoAccess** | ログイン不可 | 全員（デフォルト） |

### 3.3 ロールマッピング

```yaml
# Grafana role mapping (Keycloak claim)
role_attribute_path = "contains(realm_access.roles[*], 'sre-admin') && 'Admin' || contains(realm_access.roles[*], 'sre-editor') && 'Editor' || contains(realm_access.roles[*], 'tenant-admin') && 'Admin' || 'Viewer'"
```

### 3.4 API 認証

| API | 認証方式 | トークン形式 |
|---|---|---|
| Grafana HTTP API | OAuth Bearer | JWT (Keycloak) |
| Prometheus query | mTLS | クライアント証明書 |
| Prometheus remote_write | mTLS + Bearer | JWT |
| Loki query | mTLS + X-Scope-OrgID | テナント ID |
| Tempo query | mTLS + X-Scope-OrgID | テナント ID |
| OTel Collector | mTLS | クライアント証明書 |

## 4. テナント分離

### 4.1 データ分離モデル

```
物理層：単一クラスタ、単一 Loki/Tempo インスタンス
論理層：テナント ID ラベルで完全分離
アクセス層：RBAC + X-Scope-OrgID ヘッダでテナント絞り込み
```

### 4.2 Loki テナント分離

```yaml
# loki.yaml
auth_enabled: true

# テナント別レート制限
limits_config:
  per_tenant_override_config: /etc/loki/overrides.yaml
  retention_period: 30d
  ingestion_rate_mb: 10
  ingestion_burst_size_mb: 20

# テナント別設定例
overrides:
  tenant-a-enterprise:
    retention_period: 90d
    ingestion_rate_mb: 100
  tenant-b-standard:
    retention_period: 30d
    ingestion_rate_mb: 10
```

### 4.3 Tempo テナント分離

```yaml
# tempo.yaml
multitenancy:
  enabled: true
  tenant_id:
    header: X-Scope-OrgID
```

### 4.4 Prometheus マルチテナント

> Prometheus 自体はシングルテナントのため、**テナント別 Prometheus インスタンス**を分離デプロイ  
> （UN-P0-12 完了後に確定：共有 vs 分離）。初期は **Grafana Mimir** でマルチテナント化。

```yaml
# mimir.yaml
multitenancy:
  enabled: true
  tenant_id:
    header: X-Scope-OrgID
```

## 5. ネットワークポリシー

### 5.1 Namespace 間通信許可リスト

```yaml
# observability namespace への Ingress 許可
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-from-ada
  namespace: observability
spec:
  podSelector: {}
  policyTypes:
    - Ingress
  ingress:
    # ada namespace からのメトリクス/ログ/トレース受信
    - from:
        - namespaceSelector:
            matchLabels:
              name: ada
      ports:
        - port: 4317  # OTel gRPC
          protocol: TCP
        - port: 4318  # OTel HTTP
          protocol: TCP
    # Prometheus remote_write 受信
    - from:
        - namespaceSelector:
            matchLabels:
              name: ada
      ports:
        - port: 9090
          protocol: TCP
    # Grafana へのユーザーアクセス（ingress controller 経由のみ）
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress
      ports:
        - port: 3000
          protocol: TCP
```

### 5.2 observability namespace 内部ポリシー

```yaml
# 内部コンポーネント間通信のみ許可
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: internal-only
  namespace: observability
spec:
  podSelector: {}
  policyTypes:
    - Egress
  egress:
    # DNS 解決
    - to:
        - namespaceSelector: {}
      ports:
        - port: 53
          protocol: UDP
    # Kubernetes API
    - to:
        - namespaceSelector:
            matchLabels:
              name: kube-system
      ports:
        - port: 443
          protocol: TCP
    # S3 / S3-compatible storage
    - to:
        - namespaceSelector: {}
      ports:
        - port: 9000
          protocol: TCP
    # AlertManager -> Slack/PagerDuty webhook
    - to:
        - ipBlock:
            cidr: 0.0.0.0/0
            except:
              - 10.0.0.0/8
              - 172.16.0.0/12
              - 192.168.0.0/16
```

### 5.3 外部送信制限

| 送信先 | 許可 | 用途 |
|---|---|---|
| Keycloak (内部) | ✅ | OAuth |
| Slack / PagerDuty (外部) | ✅ 限定 | Alert 通知 |
| S3 (内部) | ✅ | Long-term storage |
| インターネット全般 | ❌ | Default deny |
| 任意 IP への送信 | ❌ | データ流出防止 |

## 6. シークレット管理

### 6.1 戦略

```
HashiCorp Vault (dev/staging) + AWS KMS (prod)
        ↓
External Secrets Operator で Secret 同期
        ↓
Kubernetes Secret (AES-256 encrypted at rest)
        ↓
Pod には envFrom / volumeMount で注入
```

### 6.2 シークレット一覧

| シークレット | 用途 | ローテーション |
|---|---|---|
| `grafana-oauth-client-secret` | Keycloak OAuth | 90 日 |
| `grafana-admin-password` | 緊急用 | 90 日 |
| `loki-s3-access-key` | S3 ストレージ | 90 日 |
| `tempo-s3-access-key` | S3 ストレージ | 90 日 |
| `alertmanager-slack-webhook` | Slack 通知 | 365 日 |
| `alertmanager-pagerduty-key` | PagerDuty | 365 日 |
| `otel-tls-cert` | mTLS サーバー証明書 | 90 日 |
| `prometheus-remote-write-token` | Remote write 認証 | 90 日 |

### 6.3 External Secrets Operator 設定

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: grafana-oauth
  namespace: observability
spec:
  secretStoreRef:
    name: vault-backend
    kind: SecretStore
  target:
    name: grafana-oauth
  data:
    - secretKey: client-secret
      remoteRef:
        key: observability/grafana
        property: oauth_client_secret
  refreshInterval: 1h
```

### 6.4 cert-manager で TLS 証明書自動管理

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: otel-collector-cert
  namespace: observability
spec:
  secretName: otel-collector-tls
  issuerRef:
    name: internal-ca
    kind: ClusterIssuer
  duration: 2160h  # 90 日
  renewBefore: 720h  # 30 日前に更新
```

## 7. データ保護

### 7.1 PII 分類

| 分類 | 該当フィールド | 取り扱い |
|---|---|---|
| **完全禁止** | password, api_key, jwt, cookie, card_number, national_id, email, phone, full_ip | ログ・トレース・メトリクスに含めない |
| **ハッシュ化許可** | tenant_id, user_id | SHA-256 先頭 8 文字 |
| **マスキング許可** | request_body（特定フィールドのみ） | 部分マスク `xxx@xxx.com` |
| **自由** | 集約メトリクス（カウント、p99 等） | OK |

### 7.2 自動 redaction

```rust
// ada-telemetry crate
pub fn redact_pii(input: &str) -> String {
    let patterns = [
        (r"\b[\w.-]+@[\w.-]+\.\w+\b", "<EMAIL>"),
        (r"\b\d{3}-\d{4}-\d{4}\b", "<PHONE>"),
        (r"\b\d{16}\b", "<CARD>"),
        (r"Bearer\s+[A-Za-z0-9._-]+", "Bearer <JWT>"),
        (r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b", "<IP>"),
    ];
    // regex apply → 出力
}
```

### 7.3 暗号化

| 対象 | 暗号化方式 |
|---|---|
| 通信路（南北） | TLS 1.3 |
| 通信路（東西） | mTLS 1.2+ |
| 保存データ（Prometheus） | AES-256（ボリューム暗号化） |
| 保存データ（Loki/Tempo） | AES-256-S3 (SSE-S3 or SSE-KMS) |
| ログ内 PII | 自動 redaction（書込前） |
| メトリクスラベル | ハッシュ化（SHA-256） |

### 7.4 GDPR / PIPL 対応

| 権利 | 対応 |
|---|---|
| **削除権（Right to erasure）** | `DELETE /v1/tenants/{id}/user-data` → Loki/Tempo に `delete_tenant` API 発行、PL/pgSQL の `purge_user_data` トリガ |
| **データポータビリティ** | ユーザーデータを JSON エクスポート（30 日以内に納品） |
| **処理制限** | ユーザ削除要求から 30 日以内に処理完了（GDPR Article 17） |
| **同意管理** | テレメトリー収集のオプトイン / オプトアウト（テナント設定） |

## 8. 監査ログ

### 8.1 監査対象操作

| 操作 | 監査場所 | 保持期間 |
|---|---|---|
| Grafana ログイン | Keycloak イベントログ + Grafana 監査 | 365 日 |
| Grafana Dashboard 変更 | Grafana 監査ログ | 365 日 |
| Grafana DataSource クエリ | Grafana 監査ログ | 90 日 |
| Prometheus クエリ | Prometheus access log → Loki | 90 日 |
| Loki クエリ | Loki audit log | 90 日 |
| Tempo クエリ | Tempo audit log | 90 日 |
| AlertManager 通知送信 | AlertManager log | 365 日 |
| K8s Secret 読み取り | K8s audit log (API server) | 365 日 |
| cert-manager 証明書発行 | cert-manager log | 365 日 |

### 8.2 監査ログの不変性

```bash
# 監査ログは write-once S3 バケットにエクスポート
aws s3api put-object --bucket ada-audit-archive \
  --key audit/$(date +%Y/%m/%d)/audit.json \
  --body audit.json \
  --object-lock-mode COMPLIANCE \
  --object-lock-retain-until-date 2027-08-20
```

## 9. コンプライアンス

### 9.1 準拠規格

| 規格 | 対応範囲 |
|---|---|
| **GDPR** | EU ユーザーデータ処理、削除権 |
| **PIPL（中国個人情報保護法）** | 中国ユーザーデータ越境転送制限 |
| **ISO 27001** | 情報セキュリティマネジメント |
| **SOC 2 Type II** | アクセス制御、暗号化、監査 |
| **IPA セキュリティ要件** | IPA 共通フレーム2018 6.2 節 |

### 9.2 コンプライアンスチェック四半期レビュー

| 項目 | 担当 | 頻度 |
|---|---|---|
| RBAC 棚卸し | SRE Lead | 四半期 |
| Secret ローテーション確認 | Security | 四半期 |
| NetworkPolicy 監査 | SRE | 四半期 |
| 監査ログ確認 | Security | 月次 |
| 脆弱性スキャン（Trivy） | SRE | 週次（CI） |

## 10. インシデント対応

### 10.1 観測基盤関連インシデント分類

| 種別 | 影響 | 重大度 | 初期対応 |
|---|---|---|---|
| **メトリクス欠損** | あるサービスが観測不能 | Sev3 | 影響範囲特定 + exporter 確認 |
| **Grafana ログイン不可** | 監視画面閲覧不可 | Sev2 | Keycloak 確認、admin 緊急アクセス |
| **Prometheus ストレージ満杯** | 過去データ消失 | Sev2 | retention 削減 + ボリューム拡張 |
| **OTel Collector ダウン** | データ受信停止 | **Sev1** | 自動再起動 → 別ノードへ再配置 |
| **監査ログ欠損** | コンプライアンス違反 | **Sev1** | 緊急調査 + 影響範囲報告 |
| **Secret 漏洩疑い** | 認証情報漏洩 | **Sev1** | 即時無効化 + 再発行 |

### 10.2 緊急アクセス手順

```bash
# 1. Grafana 緊急 admin アクセス
kubectl -n observability exec -it deploy/grafana -- \
  grafana-cli admin reset-admin-password <NEW_PASSWORD>

# 2. Prometheus 緊急クエリ（RBAC バイパス）
kubectl -n observability port-forward svc/prometheus 9090:9090
curl -k https://localhost:9090/api/v1/query?query=up

# 3. 監査ログ取得
kubectl -n observability logs -l app=audit-exporter --since=24h
```

## 11. 用語集

| 用語 | 説明 |
|---|---|
| **ゼロトラスト** | 内部ネットワークでも全通信を認証・暗号化するモデル |
| **mTLS** | 相互 TLS 認証（クライアント証明書も検証） |
| **RBAC** | Role-Based Access Control |
| **NetworkPolicy** | K8s の Namespace/Pod 間通信制御 |
| **PII** | Personally Identifiable Information（個人識別情報） |
| **GDPR** | EU 一般データ保護規則 |
| **PIPL** | 中国個人情報保護法 |
| **OIDC** | OpenID Connect（OAuth 2.0 上の認証層） |
| **JWKS** | JSON Web Key Set（公開鍵配布） |
| **Redaction** | 機密情報の自動削除・マスキング |
| **SSE-KMS** | S3 の KMS 暗号化オプション |

## 12. 参考文献

1. Kubernetes Network Policy  
   <https://kubernetes.io/docs/concepts/services-networking/network-policies/>
2. Grafana Authentication Overview  
   <https://grafana.com/docs/grafana/latest/auth/>
3. Loki Multi-tenancy  
   <https://grafana.com/docs/loki/latest/operations/multi-tenancy/>
4. Tempo Multi-tenancy  
   <https://grafana.com/docs/tempo/latest/operations/multi-tenancy/>
5. External Secrets Operator  
   <https://external-secrets.io/>
6. cert-manager Documentation  
   <https://cert-manager.io/docs/>
7. OWASP Logging Cheat Sheet  
   <https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html>
8. GDPR Article 17 - Right to erasure  
   <https://gdpr-info.eu/art-17-gdpr/>
9. 中国個人情報保護法（PIPL）  
   <http://www.npc.gov.cn/npc/c30834/202108/11408dovidfd8478588060f3ba62b98b2.shtml>

---

> **IPA 末尾注記**  
> 本ドキュメントは IPA 共通フレーム2018 (SLCP-JCF2018) 6.2「セキュリティ設計」および  
> IPA 非機能要求グレード2018 NF-SEC 項目に準拠する。  
> 記載内容は初期設計であり、UN-P0-06/07/08 完了後に最終化する。  
> 商用利用前にセキュリティ専門家によるレビューを必須とする。
