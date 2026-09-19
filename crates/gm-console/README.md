# gm-console

> Commercial-grade Web operations console for the Ada platform.

Part of the [Ada](https://github.com/UlyssesLeoLee/ada) workspace. Sits at the user-facing
edge of the platform: serves the Web SPA bundle and reverse-proxies `/api/*` into the
`ada-m13-api-gateway` upstream.

## Layer model

```
+-----------+      /api/*      +---------------------+
|  Browser  | ----------------> |     gm-console      | --(cluster-internal svc://ada-api-gateway)--> ada-m13-api-gateway
+-----------+                  +---------------------+
        \__ static (rust-embed dist/) __/
```

## Layout

- `src/lib.rs`          — crate root, public surface
- `src/config.rs`       — env-driven config (no secrets)
- `src/error.rs`        — typed errors → user-safe JSON
- `src/routes.rs`       — healthz/version/license/terms/privacy + /api proxy + SPA fallback
- `src/server.rs`       — axum bootstrap + tower layers
- `src/bin/server.rs`   — `gm-console-server` entry point

## Endpoints

| Path        | Method | Purpose                                    |
|-------------|--------|--------------------------------------------|
| /healthz    | GET    | Liveness probe                             |
| /version    | GET    | service version payload                    |
| /license    | GET    | license metadata (JSON)                    |
| /terms      | GET    | Terms of Service (text/markdown)           |
| /privacy    | GET    | Privacy Policy (text/markdown)             |
| /api/*      | *      | reverse-proxy to upstream api-gateway      |
| /*          | GET    | SPA fallback (index.html)                  |

## Configuration

| Env var                 | Default                       | Notes                              |
|-------------------------|-------------------------------|------------------------------------|
| `GM_CONSOLE_BIND`       | `0.0.0.0:8080`                | bind socket address                |
| `GM_CONSOLE_UPSTREAM`   | `http://ada-api-gateway:8080` | upstream api-gateway svc URL       |
| `GM_CONSOLE_STATIC_DIR` | unset                         | optional disk path for static dir  |
| `RUST_LOG`              | `info`                        | tracing filter                     |

**Note**: No environment variable values are ever logged. They are read and used as-is.
Logging is via `tracing` with the configured env-filter. Secrets never enter logs.

## Status

Mavis (auto-scaffold 2026-09-19):
- Cargo crate stub ✓
- License / Privacy / Terms placeholder routes ✓
- /api reverse-proxy scaffolded, real wiring pending worker-A
- Web dist placeholder pending worker-A
- App + screenshots + store descriptions → worker-B + worker-C

## License

AGPL v3 + WRITTEN-CONSENT addendum. See [`/LICENSE`](../../LICENSE).
