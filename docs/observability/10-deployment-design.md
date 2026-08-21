# 10 デプロイ設計（Deployment Design）

> **観測基盤は別 namespace で隔離**し、Helm + GitOps (ArgoCD) で宣言的に管理。  
> 業務サービス（`ada` namespace）と観測基盤（`observability` namespace）を明確に分離し、  
> フェイルオーバー・スケーリング・アップグレードを独立して実施できる構成とする。

> **ドキュメントID**：DOC-OBS-010
> **上位文書**：[DOC-OBS-INDEX](README.md)
> **下位文書**：[DOC-OBS-002 Architecture](02-architecture.md) / [DOC-OBS-009 Security](09-security-design.md) / [DOC-ARCH-002 Deployment](D:/Ada/docs/architecture/02-deployment-architecture.md)

---

## 改訂履歴

| バージョン | 日付 | 変更内容 |
|---|---|---|
| v1.0.0 | 2026-08-20 | 初版（K3s + Helm + ArgoCD 構成） |

---

## 目次

1. 設計原則
2. Namespace 構成
3. Helm Chart 構成
4. 各コンポーネントのデプロイ仕様
5. ストレージ設計
6. リソース割り当て
7. GitOps（ArgoCD）
8. 設定管理
9. バックアップ・リストア
10. アップグレード戦略
11. 用語集
12. 参考文献

---

## 1. 設計原則

| 原則 | 説明 |
|---|---|
| **Namespace 隔離** | 業務サービスとは別の `observability` namespace にデプロイ |
| **宣言的構成** | 全ての設定値を Helm + Git でコード化 |
| **GitOps** | ArgoCD で Git と同期、ドリフト検出 + 自動復旧 |
| **Immutable 設定** | ConfigMap に直接変更しない、Helm values で上書き |
| **ステートレス優先** | ステートフルは PostgreSQL のみ、他は S3 か ephemeral volume |
| **リソース分離** | ResourceQuota + LimitRange で観測基盤が業務サービスに影響しない |

## 2. Namespace 構成

```
ada (業務サービス)
├── ada namespace: m01-m16 16 サービス
└── DB / Cache / Storage は ada-data namespace

observability (観測基盤)
├── observability namespace
│   ├── grafana
│   ├── prometheus / mimir
│   ├── loki
│   ├── tempo
│   ├── alertmanager
│   ├── opentelemetry-collector
│   ├── exporters (node, k8s, postgres, redis)
│   └── cert-manager / external-secrets
└── observability-data (長期保存用 S3 / PV)
```

### 2.1 Namespace 定義

```yaml
# observability namespace
apiVersion: v1
kind: Namespace
metadata:
  name: observability
  labels:
    name: observability
    purpose: monitoring
    compliance-tier: high
    network-policy: default-deny

---
# observability namespace リソース制限
apiVersion: v1
kind: ResourceQuota
metadata:
  name: observability-quota
  namespace: observability
spec:
  hard:
    requests.cpu: "100"
    requests.memory: 200Gi
    limits.cpu: "200"
    limits.memory: 400Gi
    persistentvolumeclaims: "20"
    services: "30"
    secrets: "50"
    configmaps: "50"

---
# Pod あたりのデフォルト制限
apiVersion: v1
kind: LimitRange
metadata:
  name: observability-limits
  namespace: observability
spec:
  limits:
    - default:
        cpu: 1
        memory: 1Gi
      defaultRequest:
        cpu: 100m
        memory: 128Mi
      type: Container
```

## 3. Helm Chart 構成

### 3.1 チャート階層

```
charts/
├── observability-platform/          # 親チャート
│   ├── Chart.yaml
│   ├── values.yaml
│   └── templates/
│       ├── _helpers.tpl
│       ├── namespace.yaml
│       ├── resourcequota.yaml
│       └── networkpolicies.yaml
│
├── observability-grafana/           # Grafana サブチャート
├── observability-prometheus/        # Prometheus サブチャート
├── observability-loki/              # Loki サブチャート
├── observability-tempo/             # Tempo サブチャート
├── observability-alertmanager/      # AlertManager サブチャート
└── observability-otel-collector/    # OTel Collector サブチャート
```

### 3.2 親 Chart.yaml

```yaml
apiVersion: v2
name: observability-platform
description: "Ada 観測基盤プラットフォーム"
type: application
version: 1.0.0
appVersion: "2.2.0"
dependencies:
  - name: grafana
    version: 7.0.0
    repository: https://grafana.github.io/helm-charts
  - name: prometheus
    version: 25.0.0
    repository: https://prometheus-community.github.io/helm-charts
    condition: prometheus.enabled
  - name: mimir
    version: 5.0.0
    repository: https://grafana.github.io/helm-charts
    condition: mimir.enabled
  - name: loki
    version: 5.0.0
    repository: https://grafana.github.io/helm-charts
  - name: tempo
    version: 1.5.0
    repository: https://grafana.github.io/helm-charts
  - name: alertmanager
    version: 1.5.0
    repository: https://prometheus-community.github.io/helm-charts
  - name: opentelemetry-collector
    version: 0.80.0
    repository: https://open-telemetry.github.io/opentelemetry-helm-charts
```

### 3.3 親 values.yaml（抜粋）

```yaml
# グローバル
global:
  environment: production
  region: ap-northeast-1
  storageClass: ssd-retain
  certManager:
    clusterIssuer: internal-ca

# 業務サービスからのテレメトリ
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
        tls:
          cert_file: /etc/tls/tls.crt
          key_file: /etc/tls/tls.key
      http:
        endpoint: 0.0.0.0:4318
        tls: {}

# ストレージ（S3）
storage:
  s3:
    endpoint: minio.observability-data.svc.cluster.local:9000
    bucket:
      chunks: ada-loki-chunks
      ruler: ada-loki-ruler
      admin: ada-loki-admin
      traces: ada-tempo-traces
    accessKey: ${S3_ACCESS_KEY}
    secretKey: ${S3_SECRET_KEY}

# コンポーネント別有効化
grafana:
  enabled: true
  replicas: 2
prometheus:
  enabled: false  # 本番は mimir を使用
mimir:
  enabled: true
  replicas: 3
loki:
  enabled: true
  replicas: 3
tempo:
  enabled: true
  replicas: 3
alertmanager:
  enabled: true
  replicas: 3
opentelemetry-collector:
  enabled: true
  replicas: 5
  autoscaling:
    enabled: true
    minReplicas: 5
    maxReplicas: 20
    targetCPUUtilizationPercentage: 70
```

## 4. 各コンポーネントのデプロイ仕様

### 4.1 OpenTelemetry Collector（DaemonSet + Deployment 構成）

```yaml
opentelemetry-collector:
  mode: deployment
  replicaCount: 5
  autoscaling:
    enabled: true
    minReplicas: 5
    maxReplicas: 20
  resources:
    requests:
      cpu: 500m
      memory: 1Gi
    limits:
      cpu: 2
      memory: 4Gi
  config:
    exporters:
      prometheusremotewrite:
        endpoint: http://mimir-distributor.observability:9009/api/v1/push
        tls:
          insecure: false
          cert_file: /etc/tls/tls.crt
          key_file: /etc/tls/tls.key
      loki:
        endpoint: http://loki-gateway.observability:3100/loki/api/v1/push
      otlp:
        endpoint: tempo-distributor.observability:4317
        tls:
          insecure: false
```

### 4.2 Grafana

```yaml
grafana:
  replicas: 2
  image:
    tag: 10.3.0
  resources:
    requests:
      cpu: 200m
      memory: 512Mi
    limits:
      cpu: 1
      memory: 2Gi
  persistence:
    enabled: true
    size: 10Gi
    storageClass: ssd-retain
  ingress:
    enabled: true
    ingressClassName: nginx
    hosts:
      - grafana.ada.internal
    tls:
      - secretName: grafana-tls
        hosts:
          - grafana.ada.internal
  datasources:
    - name: Prometheus
      type: prometheus
      url: http://mimir-query-frontend.observability:9009/prometheus
    - name: Loki
      type: loki
      url: http://loki-gateway.observability:3100
    - name: Tempo
      type: tempo
      url: http://tempo-query-frontend.observability:3100
```

### 4.3 Prometheus / Mimir

```yaml
# 本番は Mimir（マルチテナント、スケーラブル）
mimir:
  replicas: 3
  image:
    tag: 2.10.0
  resources:
    requests:
      cpu: 1
      memory: 4Gi
    limits:
      cpu: 4
      memory: 16Gi
  persistence:
    enabled: true
    size: 100Gi
    storageClass: ssd-retain
  components:
    ingester:
      replicas: 3
    storeGateway:
      replicas: 3
    compactor:
      replicas: 1
    queryFrontend:
      replicas: 2
    distributor:
      replicas: 3
```

### 4.4 Loki

```yaml
loki:
  image:
    tag: 2.9.0
  replicas: 3
  resources:
    requests:
      cpu: 500m
      memory: 1Gi
    limits:
      cpu: 2
      memory: 4Gi
  persistence:
    enabled: true
    size: 50Gi
    storageClass: ssd-retain
  storage:
    type: s3
    s3:
      bucketnames: ada-loki-chunks
  schemaConfig:
    configs:
      - from: 2024-01-01
        store: tsdb
        object_store: s3
        schema: v13
```

### 4.5 Tempo

```yaml
tempo:
  image:
    tag: 2.3.0
  replicas: 3
  resources:
    requests:
      cpu: 500m
      memory: 1Gi
    limits:
      cpu: 2
      memory: 4Gi
  storage:
    trace:
      backend: s3
      s3:
        bucket: ada-tempo-traces
      wal:
        path: /var/tempo/wal
  receivers:
    otlp:
      protocols:
        grpc:
          endpoint: 0.0.0.0:4317
        http:
          endpoint: 0.0.0.0:4318
```

### 4.6 AlertManager

```yaml
alertmanager:
  image:
    tag: 0.27.0
  replicas: 3
  resources:
    requests:
      cpu: 100m
      memory: 256Mi
    limits:
      cpu: 500m
      memory: 1Gi
  alertmanagerConfig:
    route:
      receiver: 'default'
      group_by: ['alertname', 'cluster', 'service']
      group_wait: 30s
      group_interval: 5m
      repeat_interval: 4h
      routes:
        - matchers:
            - severity = sev1
          receiver: 'pagerduty-sev1'
          group_wait: 10s
          repeat_interval: 1h
        - matchers:
            - severity = sev2
          receiver: 'slack-incident'
        - matchers:
            - severity = sev3
          receiver: 'slack-alerts'
        - matchers:
            - severity = sev4
          receiver: 'slack-alerts-info'
    receivers:
      - name: 'pagerduty-sev1'
        pagerdutyConfigs:
          - serviceKey: ${PAGERDUTY_KEY}
            severity: critical
      - name: 'slack-incident'
        slackConfigs:
          - apiUrl: ${SLACK_WEBHOOK_INCIDENT}
            channel: '#incident'
            title: '{{ .GroupLabels.alertname }}'
            text: '{{ range .Alerts }}{{ .Annotations.description }}\n{{ end }}'
```

## 5. ストレージ設計

### 5.1 容量計画

| コンポーネント | 種別 | 容量 | IOPS 要件 | バックアップ |
|---|---|---|---|---|
| Prometheus/Mimir | ブロック PV | 500 Gi / replica | 3000 IOPS | Daily snapshot → S3 |
| Loki | ブロック PV（キャッシュ）+ S3（メイン） | 100 Gi / replica + S3 無限 | 中 | S3 標準 |
| Tempo | ブロック PV（WAL）+ S3（メイン） | 50 Gi / replica + S3 無限 | 中 | S3 標準 |
| Grafana | ブロック PV | 10 Gi | 低 | Daily snapshot |
| AlertManager | ブロック PV | 5 Gi | 低 | Daily snapshot |
| MinIO（自前 S3） | 分散 PV | 2 TiB | 高 | Erasure coding |

### 5.2 StorageClass

```yaml
# ssd-retain（Retain で PV 保持）
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: ssd-retain
provisioner: local.csi.openebs.io
reclaimPolicy: Retain
volumeBindingMode: WaitForFirstConsumer

# bulk-s3（S3 バックエンド）
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: bulk-s3
provisioner: s3.csi.aws.com
reclaimPolicy: Retain
parameters:
  bucket: ada-bulk-storage
```

## 6. リソース割り当て

### 6.1 Namespace 全体の予算

| リソース | 合計予算 | 内訳 |
|---|---|---|
| **CPU requests** | 50 cores | OTel: 2.5, Mimir: 3, Loki: 1.5, Tempo: 1.5, Grafana: 0.4, AM: 0.3, Exporters: 1, headroom: 39.8 |
| **CPU limits** | 100 cores | 同上 ×2 |
| **Memory requests** | 100 GiB | OTel: 5, Mimir: 12, Loki: 3, Tempo: 3, Grafana: 1, AM: 0.75, Exporters: 4, headroom: 71.25 |
| **Memory limits** | 200 GiB | 同上 ×2 |
| **Storage (PV)** | 700 GiB | Mimir: 500, Loki: 100, Tempo: 50, Grafana: 10, AM: 5, MinIO: extra |
| **Storage (S3)** | 5 TiB | Loki chunks + Tempo traces |

### 6.2 Pod 単位（HPA 連動）

| Pod | min | max | CPU target | Memory target |
|---|---|---|---|---|
| OTel Collector | 5 | 20 | 70% | 75% |
| Mimir Ingester | 3 | 6 | 70% | 75% |
| Mimir Store Gateway | 3 | 6 | 70% | 80% |
| Loki | 3 | 8 | 70% | 75% |
| Tempo | 3 | 8 | 70% | 75% |
| Grafana | 2 | 5 | 70% | 75% |
| AlertManager | 3 | 3 | - | - |

### 6.3 HPA 設定例

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: otel-collector-hpa
  namespace: observability
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: opentelemetry-collector
  minReplicas: 5
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Pods
      pods:
        metric:
          name: otelcol_accepted_spans_per_sec
        target:
          type: AverageValue
          averageValue: "1000"
```

## 7. GitOps（ArgoCD）

### 7.1 ArgoCD Application 構成

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: observability-platform
  namespace: argocd
spec:
  project: observability
  source:
    repoURL: https://github.com/ada/helm-charts
    targetRevision: main
    path: charts/observability-platform
    helm:
      releaseName: observability
      valueFiles:
        - values-production.yaml
  destination:
    server: https://kubernetes.default.svc
    namespace: observability
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
      allowEmpty: false
    syncOptions:
      - CreateNamespace=true
      - ServerSideApply=true
    retry:
      limit: 5
      backoff:
        duration: 10s
        factor: 2
        maxDuration: 5m
  revisionHistoryLimit: 10
```

### 7.2 App of Apps 構成

```
argocd/
├── apps/
│   ├── platform.yaml        # observability-platform 親
│   ├── grafana.yaml         # Grafana サブアプリ
│   ├── mimir.yaml           # Mimir サブアプリ
│   ├── loki.yaml            # Loki サブアプリ
│   ├── tempo.yaml           # Tempo サブアプリ
│   ├── alertmanager.yaml    # AlertManager サブアプリ
│   └── otel-collector.yaml  # OTel Collector サブアプリ
└── app-of-apps.yaml         # 親 App
```

### 7.3 Sync Wave（順序制御）

```yaml
# sync wave による依存順序
metadata:
  annotations:
    argocd.argoproj.io/sync-wave: "-1"  # namespace / secret
---
metadata:
  annotations:
    argocd.argoproj.io/sync-wave: "0"  # ConfigMap / ServiceAccount
---
metadata:
  annotations:
    argocd.argoproj.io/sync-wave: "1"  # Deployment / StatefulSet
---
metadata:
  annotations:
    argocd.argoproj.io/sync-wave: "2"  # HPA / NetworkPolicy
```

## 8. 設定管理

### 8.1 設定値のソース優先順位

```
1. Helm values (Git)
    ↓
2. ConfigMap (immutable, Git 管理)
    ↓
3. Secret (External Secrets で同期)
    ↓
4. 環境変数 (Pod spec で注入)
    ↓
5. 業務コードが読む
```

### 8.2 環境別 values

| 環境 | ブランチ | Replica | Storage | 特徴 |
|---|---|---|---|---|
| **dev** | `main` | 1 | 10Gi | 単一ノード、外部 S3 不要 |
| **staging** | `staging` | 2 | 50Gi | HA、外部 S3 |
| **production** | `production` | 3+ | 100Gi+ | Full HA、Multi-AZ |

### 8.3 設定検証 CI

```yaml
# .github/workflows/observability-validate.yaml
name: observability-validate
on:
  pull_request:
    paths:
      - 'charts/observability-platform/**'
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: helm lint
        run: |
          helm lint charts/observability-platform
          helm template charts/observability-platform \
            --values charts/observability-platform/values-production.yaml \
            | kubeconform -strict -summary
      - name: promtool check
        run: |
          find charts/observability-platform/alerts/ -name '*.yaml' \
            -exec promtool check rules {} \;
```

## 9. バックアップ・リストア

### 9.1 バックアップ対象

| 対象 | 方法 | 頻度 | 保持期間 | RPO / RTO |
|---|---|---|---|---|
| **Grafana DB** | Velero backup | 日次 | 30 日 | 1h / 1h |
| **AlertManager 設定** | Git 管理 | 継続 | 永続 | 0 / 15 min |
| **Prometheus / Mimir** | Thanos sidecar → S3 | 継続 | 365 日 | 5 min / 30 min |
| **Loki chunks** | S3 標準 | 継続 | 30 日（設定可能） | 0 / 30 min |
| **Tempo traces** | S3 標準 | 継続 | 7 日 | 0 / 30 min |
| **Helm values** | Git | 継続 | 永続 | 0 / 15 min |
| **監査ログ** | S3 Object Lock | 継続 | 365 日 | 0 / 1h |

### 9.2 Velero バックアップ

```bash
# 日次 cron
velero backup create observability-daily-$(date +%Y%m%d) \
  --include-namespaces observability \
  --include-resources persistentvolumeclaims,services,configmaps,secrets,deployments,statefulsets \
  --ttl 720h \
  --storage-location s3-ada-backup
```

### 9.3 リストア手順

```bash
# 1. ArgoCD で再同期（GitOps の正）
argocd app sync observability-platform

# 2. Velero リストア（クラスタ障害時）
velero restore create --from-backup observability-daily-20260820

# 3. データ復元
# Prometheus: Thanos query
# Loki: S3 から再ロード
# Tempo: S3 から再ロード
```

## 10. アップグレード戦略

### 10.1 アップグレードポリシー

| 種別 | 頻度 | 手順 |
|---|---|---|
| **パッチ (z)** | 必要時 | Helm upgrade → ArgoCD sync |
| **マイナー (y)** | 四半期 | staging 検証 1 週 → prod 適用 |
| **メジャー (x)** | 半年 | 専用検証期間 + Rollback 計画必須 |

### 10.2 アップグレード手順

```
G1. Git で values 更新（version bump）
    ↓ PR + レビュー
G2. CI: helm lint + kubeconform + promtool
    ↓
G3. staging ArgoCD sync
    ↓ 24 時間観察
G4. production ArgoCD sync
    ↓
G5. 1 時間モニタリング（dashboard 90 確認）
    ↓
G6. 問題発生時: argocd app rollback
```

### 10.3 Rollback

```bash
# ArgoCD Rollback
argocd app history observability-platform
argocd app rollback observability-platform --revision 5

# Helm Rollback
helm history observability -n observability
helm rollback observability 3 -n observability
```

## 11. 用語集

| 用語 | 説明 |
|---|---|
| **Helm** | K8s のパッケージマネージャ |
| **GitOps** | Git を Single Source of Truth とする運用モデル |
| **ArgoCD** | GitOps ツール |
| **ResourceQuota** | Namespace 単位のリソース上限 |
| **LimitRange** | Pod/Container 単位のデフォルト制限 |
| **StorageClass** | ストレージの抽象化レイヤー |
| **HPA** | Horizontal Pod Autoscaler |
| **Sync Wave** | ArgoCD のリソース適用順序制御 |
| **Velero** | K8s リソース / PV バックアップツール |
| **Object Lock** | S3 の WORM（Write Once Read Many）機能 |
| **RPO** | Recovery Point Objective（目標復旧時点） |
| **RTO** | Recovery Time Objective（目標復旧時間） |

## 12. 参考文献

1. Helm Documentation  
   <https://helm.sh/docs/>
2. ArgoCD Documentation  
   <https://argo-cd.readthedocs.io/>
3. Grafana Helm Chart  
   <https://github.com/grafana/helm-charts>
4. Mimir Documentation  
   <https://grafana.com/docs/mimir/latest/>
5. K3s Documentation  
   <https://docs.k3s.io/>
6. Velero Documentation  
   <https://velero.io/docs/>
7. cert-manager Documentation  
   <https://cert-manager.io/docs/>
8. External Secrets Operator  
   <https://external-secrets.io/>

---

> **IPA 末尾注記**  
> 本ドキュメントは IPA 共通フレーム2018 (SLCP-JCF2018) 6.3「デプロイ設計」および  
> DOC-ARCH-002 デプロイアーキテクチャ文書に準拠する。  
> 記載内容は K3s 環境での初期値であり、UN-P0-12（ストレージ戦略）完了後に最終化する。  
> 商用利用前に SRE レビューと Disaster Recovery 訓練を必須とする。
