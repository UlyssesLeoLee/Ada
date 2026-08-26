# Ada Observability Stack — Phase 0 + Phase 1

> 業務コードを**ゼロインパクト**で観測可能にする最小スタック。  
> 設計 source of truth: `docs/observability/01-13` (14 docs, 210 KB)。

このディレクトリは **Phase 0 (インフラ)** と **Phase 1 (コア metrics + logging)** を
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
`promtail` / `node-exporter` / `postgres-exporter` の 8 サービスが立ち上がります。

## 起動後の URL

| Service     | URL                       | 認証                          |
|-------------|---------------------------|-------------------------------|
| Grafana     | http://localhost:3000     | `.env` の `GRAFANA_ADMIN_*`   |
| Prometheus  | http://localhost:9090     | なし                          |
| Loki        | http://localhost:3100     | UI なし (Grafana から操作)    |
| Jaeger UI   | http://localhost:16686    | なし                          |
| OTLP gRPC   | localhost:4317            | なし (Phase 6+ で mTLS)       |

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

# 6. 4 つの alert rule が読み込まれている
curl -s http://localhost:9090/api/v1/rules | jq '.data.groups[].name'
# 期待値: app_down / high_error_rate / high_latency / low_disk

# 7. 3 つの dashboard が provisioning されている
curl -s http://localhost:3000/api/search?query=& | jq '.[].title'
# 期待値: "App Overview (30-01)" / "Rust Runtime (70-03)" / "DB Overview (40-01)"
```

## ディレクトリ構成

```
observability/
├── README.md                          ← 本ファイル
├── docker-compose.yml                 ← 1-key-up 8 サービス
├── .env.example                       ← 認証情報のテンプレート
├── prometheus/
│   ├── prometheus.yml                 ← scrape config (4 jobs)
│   └── alerts/
│       ├── app_down.yml               ← ALT-001 ServiceDown (Sev1)
│       ├── high_error_rate.yml        ← ALT-101 HighErrorRate (Sev2)
│       ├── high_latency.yml           ← ALT-102 HighLatency (Sev2)
│       └── low_disk.yml               ← ALT-106 LowDiskSpace (Sev2)
├── loki/
│   ├── loki-config.yaml               ← 30d retention, fs store
│   └── promtail-config.yaml           ← docker service log shipper
├── grafana/
│   ├── provisioning/
│   │   ├── datasources/datasources.yml  ← 自動配 datasources
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
    └── init.ps1                       ← Windows 1-key-up
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

- Alertmanager (Phase 6)
- PagerDuty 統合 (Phase 6)
- Tempo 長期 trace 保存 (Phase 6+)
- Anomaly detection (Phase 4+)
- Multi-tenant 隔離 (Phase 5+)
- mTLS / 認証 (Phase 6+)

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
