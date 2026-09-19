# Demo / Production URL spec for gm-console

## Hosted preview (CI auto-deploys)

| Tier        | URL                                | Backing                                |
|-------------|------------------------------------|----------------------------------------|
| Demo URL    | `https://gm-console.kanvas.dev`    | Cloudflare Workers → envoy → gm-console Deployment |
| Staging URL | `https://staging.gm-console.kanvas.dev` | same path, separate Deployment       |
| Local dev   | `http://localhost:8080`            | `cargo run -p gm-console-server`       |

## TLS / Cert

- TLS via Cloudflare edge — managed cert.
- Production DNS `gm-console.kanvas.dev` → `104.21.x.x` (Cloudflare proxy).
- Wildcard `*.gm-console.kanvas.dev` covers staging / preview tenants.

## Routing upstream contract

- All `/api/*` requests are reverse-proxied by gm-console to
  `ada-m13-api-gateway:8080` (cluster-internal Service).
- Connection uses mTLS via envoy sidecar filter (see
  `deploy/k8s/gm-console.yaml` for the production posture).

## CI deploy lane

`mobile-build.yml` (worker-G target) builds the binary and exposes it via:

```yaml
- name: Run gm-console release binary in preview
  run: |
    ./target/release/gm-console-server &
    sleep 3
    curl -fsS http://127.0.0.1:8080/healthz
```

(no public-Internet publishes from CI directly; Cloudflare Tunnel or a similar CI launcher
is out of scope for v0.3.0 — `docs/commercial/DEMO_DEPLOY.md` tracks that path).

## Failure modes documented

| Symptom                                    | Likely cause                              | Action                                 |
|--------------------------------------------|-------------------------------------------|----------------------------------------|
| `Bad Gateway` from `/api/*`                | upstream api-gateway down                 | check `kubectl get pods -n observability -l app=ada-api-gateway` |
| TLS handshake fails on user browser        | cert pinned to wrong CN                   | see Cloudflare edge config             |
| `405` on `/api/*`                          | method not forwarded by client lib         | ensure `gm_console_app` uses HTTP 1.1   |
