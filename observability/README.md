# Ada Observability Stack — Phase 0 + Phase 1 + Phase 6

> 業務コードを**ゼロインパクト**で観測可能にする最小スタック。  
> 設計 source of truth: `docs/observability/01-13` (14 docs, 210 KB)。

このディレクトリは **Phase 0 (インフラ) / Phase 1 (コア metrics + logging) / Phase 6 (Alert + Long-term storage)** を
1-key-up で立ち上げるための実装ファイル群です。Phase 2 以降は docs の phased-rollout
に順次従います (`docs/observability/11-phased-rollout.md`)。

## 3 行で立ち上げる

```bash
# 1. Linux / macOS
bash observability/scripts/init.sh

# 2. Windows (PowerShell)
pwsh -File observability/scripts/init.ps1

# 3. 手動 (どちらの OS でも)
docker compose -f observability/docker-compose.yml up -d
```

3 つ目のコマンドで `prometheus` / `loki` / `grafana` / `jaeger` / `otel-collector` /
`promtail` / `node-exporter` / `postgres-exporter` / **`alertmanager` / `minio` / `mc`**
の **11 サービス** が立ち上がります。Phase 6 で追加された 3 サービス:

- **`alertmanager`** — Phase 6 アラートルーティング層。Prometheus が発火した
  アラートを受信し、PagerDuty / Slack / メールへ振り分ける。
  設定: `./alertmanager/alertmanager.yml` + `./alertmanager/templates/`
- **`minio`** — S3 互換の長期ストレージ。Prometheus TSDB の remote_write ターゲット。
  API :9000, Console :9001, 認証は `.env` の `MINIO_ROOT_USER` / `MINIO_ROOT_PASSWORD`。
- **`mc`** — MinIO クライアント init コンテナ。起動時にバケット
  `prometheus-tsdb` を作成 + 90 日 lifecycle を設定して終了する。

## 起動後の URL

| Service           | URL                       | 認証                          |
|-------------------|---------------------------|-------------------------------|
| Grafana           | http://localhost:3000     | `.env` の `GRAFANA_ADMIN_*`   |
| Prometheus        | http://localhost:9090     | なし                          |
| **Alertmanager**  | **http://localhost:9093** | **なし**                      |
| **MinIO Console** | **http://localhost:9001** | **`.env` の `MINIO_ROOT_*`**  |
| **MinIO S3 API**  | **http://localhost:9000** | **同上** (S3 client から)     |
| Loki              | http://localhost:3100     | UI なし (Grafana から操作)    |
| Jaeger UI         | http://localhost:16686    | なし                          |
| OTLP gRPC         | localhost:4317            | なし (Phase 6+ で mTLS)       |

## 検証 (init 後 30 秒以内に実行)

```bash
# 1. Prometheus が up
curl -sf http://localhost:9090/-/healthy && echo "prometheus ok"

# 2. Loki が ready
curl -sf http://localhost:3100/ready && echo "loki ok"

# 3. Jaeger UI が応答
curl -sf -o /dev/null http://localhost:16686/ && echo "jaeger ok"

# 4. Grafana ヘルス
curl -sf http://localhost:3000/api/health && echo "grafana ok"

# 5. OTLP collector ヘルス
curl -sf http://localhost:13133/ && echo "otel-collector ok"

# 6. 5 つの alert rule が読み込まれている
curl -s http://localhost:9090/api/v1/rules | jq '.data.groups[].name'
# 期待値: app_down / high_error_rate / high_latency / low_disk / scaling_alert

# 7. 3 つの dashboard が provisioning されている
curl -s http://localhost:3000/api/search?query=& | jq '.[].title'
# 期待値: "App Overview (30-01)" / "Rust Runtime (70-03)" / "DB Overview (40-01)"

# 8. Alertmanager が Prometheus と接続されている
curl -s http://localhost:9090/api/v1/alertmanagers | jq
curl -sf http://localhost:9093/-/ready && echo "alertmanager ok"

# 9. MinIO bucket 作成済み
docker exec -it ada-minio-mc mc ls ada/
# 期待値: prometheus-tsdb/
```

## ディレクトリ構成

```
observability/
├── README.md                          ← 本ファイル
├── docker-compose.yml                 ← 1-key-up 11 サービス
├── .env.example                       ← 認証情報のテンプレート
├── prometheus/
│   ├── prometheus.yml                 ← scrape (4 jobs) + alerting + remote_write
│   └── alerts/
│       ├── app_down.yml               ← ALT-001 ServiceDown (P1)
│       ├── high_error_rate.yml        ← ALT-101 HighErrorRate (P2)
│       ├── high_latency.yml           ← ALT-102 HighLatency (P2)
│       ├── low_disk.yml               ← ALT-106 LowDiskSpace (P2)
│       └── scaling_alert.yml          ← ALT-203 CPUHigh (P3, Phase 6 で追加)
├── alertmanager/
│   ├── alertmanager.yml               ← 5 routes / 5 receivers / 4 inhibit rules
│   └── templates/
│       ├── slack.tmpl                 ← Slack 通知テンプレート
│       ├── email.tmpl                 ← email 通知 (html + text)
│       └── default.tmpl               ← フォールバック用
├── minio/
│   ├── init-bucket.sh                 ← mc 経由で bucket 作成 + 90d lifecycle
│   └── README.md                      ← Phase 6 long-term-storage メモ
├── loki/
│   ├── loki-config.yaml               ← 30d retention, fs store
│   └── promtail-config.yaml           ← docker service log shipper
├── grafana/
│   ├── provisioning/
│   │   ├── datasources/datasources.yml  ← Prometheus / Loki / Jaeger / Postgres / Alertmanager
│   │   └── dashboards/dashboards.yml    ← 自動配 dashboards
│   └── dashboards/
│       ├── app-overview.json          ← 30-01 (RPS / Errors / Latency / CPU / Mem)
│       ├── rust-runtime.json          ← 70-03 (CPU / RSS / FDs / threads)
│       └── db-overview.json           ← 40-01 (pg_up / conns / cache hit / locks)
├── jaeger/
│   ├── jaeger-config.yaml             ← Jaeger v1 all-in-one env (doc)
│   └── otel-collector-config.yaml     ← OTel collector ingress config
└── scripts/
    ├── init.sh                        ← Linux / macOS 1-key-up
    ├── init.ps1                       ← Windows 1-key-up
    ├── init-prometheus-remote-write.sh ← MinIO + Prometheus remote_write 結線 helper
    └── validate-configs.py            ← YAML/JSON lint
```

## 設計マッピング

| 実装                              | 設計ドキュメント                          |
|-----------------------------------|-------------------------------------------|
| 4 つの alert rule                 | `07-alert-policy.md` §4 + `11-phased-rollout.md` §3 (G1 ゲート) |
| `prometheus.yml` の scrape job   | `02-architecture.md` §2.3 + `11-phased-rollout.md` §3.2  |
| dashboard の PromQL パネル        | `06-dashboard-catalog.md` §3-12  |
| OTel collector ルート             | `02-architecture.md` §1 + §4.1  |
| Loki / Promtail 設定              | `04-logging-design.md` + `10-deployment-design.md`  |
| `ada_app_*` メトリクス名         | `03-metrics-design.md` §2.1 + §5  |

## 範囲外 (Phase 2+ で追加)

- Tempo 長期 trace 保存 (Phase 7+)
- Mimir / Thanos sidecar (Phase 6 本番化, 現状は placeholder remote_write)
- Anomaly detection (Phase 4+)
- Multi-tenant 隔離 (Phase 5+)
- mTLS / 認証 (Phase 6+)

## Phase 6 — Alert + Long-term storage 補足

Phase 6 で追加された **Alertmanager + MinIO** は設計ドキュメントの
`docs/observability/11-phased-rollout.md` §8 (Phase 6 Alert) と
`docs/observability/10-deployment-design.md` §4.6 / §5.1 に従います。

- **3 段階重大度** (P1 / P2 / P3) — `alertmanager.yml` のルートツリーは
  P1 → PagerDuty、P2 → Slack、P3 → Email digest に振り分け。
  設計ドキュメント `07-alert-policy.md` の 4 段階 (sev1〜sev4) を
  Phase 6 実装では 3 段階に集約している (sev3 / sev4 をまとめて P3)。
- **Inhibit rules** — `ServiceDown` が P2/P3 を抑制、
  `ClusterDegraded` が `Pod.*` を抑制、 `DatabaseDown` が `DB.*` を抑制、
  `PlannedMaintenance` が全 alert を抑制。すべて `alertmanager.yml` で宣言。
- **Long-term storage** — Prometheus が `remote_write` 経由で MinIO バケット
  `prometheus-tsdb` に push。**現状は placeholder** (MinIO は S3 API のみ
  話すので Prometheus remote_write プロトコルを直接受信しない) であり、
  本番化では `mimir-distributor` もしくは `thanos-receiver` をバケット手前に
  配置する想定。バケット + 90 日 lifecycle は up front で作成済み。
- **Secrets** — `alertmanager.yml` の `${PAGERDUTY_ROUTING_KEY}` /
  `${SLACK_WEBHOOK_*}` / `${SMTP_*}` は docker-compose が `.env` から
  補間する。実 secret は `observability/.env` (git-ignore) に置き、
  `.env.example` (committed) は dev 用の placeholder のみ。

## 業務コードへの影響

**ゼロ**。本ディレクトリの変更はワークスペース全体に波及しません。  
`crates/ada-telemetry/Cargo.toml` には Phase 2+ で有効化する feature flag
(`prometheus`) のスタブをコメントとしてのみ残しています (WT-1 のスコープと
衝突回避のため、main branch ではまだ enable しません)。

`crates/ada-m09-exporter/src/otlp.rs` には `OtlpPushExporter` を追加しました
(Phase 1 の OTLP push 経路)。`Exporter` / `OtlpExporter` の既存 trait には
触れていないので v0.1.0 のテストはすべて通過します。

## 停止

```bash
docker compose -f observability/docker-compose.yml down            # データ保持
docker compose -f observability/docker-compose.yml down -v         # データ削除
```

## 参考文献

- `docs/observability/README.md` — 設計 index
- `docs/observability/11-phased-rollout.md` — Phase 0-8 ロードマップ
- `docs/observability/02-architecture.md` — 4 シグナル統合アーキテクチャ
- <https://opentelemetry.io/docs/collector/configuration/>
- <https://grafana.com/docs/loki/latest/configuration/>
- <https://www.jaegertracing.io/docs/1.55/>
