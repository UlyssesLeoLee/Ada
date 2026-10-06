# gm-console: how it is reached

This file describes **what this repository actually deploys**. An earlier
version of this document described a hosted preview at
`https://gm-console.kanvas.dev` with staging, Cloudflare-managed TLS,
production DNS and a CI deploy lane. None of that existed: there is no
deploy job in any workflow, no second Deployment for staging, no envoy
manifest, and the document pointed readers at `docs/commercial/DEMO_DEPLOY.md`,
which was never created. It is rewritten here rather than deleted, because
the URL naming convention below is still the intended one -- it just is
not live yet.

## What exists today

| Tier          | URL                      | Backing                                          |
|---------------|--------------------------|--------------------------------------------------|
| Local dev     | `http://localhost:8080`  | `cargo run -p gm-console-server`                 |
| In-cluster    | `http://gm-console`      | Service `gm-console`, ClusterIP, port 80 → 8080  |

`deploy/k8s/gm-console.yaml` is a ClusterIP Service with **no Ingress**.
Nothing in this repository exposes it to the public Internet.

### Namespaces

Only `ada-remediation` declares a namespace (`observability`). The
gateway and the console declare none, so they land in `default`. This
matters when writing `kubectl` commands against the wrong one.

## Routing upstream contract

This part is real, and is the reason cross-origin policy does not apply
to the browser:

- All `/api/*` requests are reverse-proxied by gm-console to
  `ada-m13-api-gateway:8080`, the cluster-internal Service, configured
  by `GM_CONSOLE_UPSTREAM` in `deploy/k8s/gm-console.yaml`.
- The browser only ever issues **same-origin relative URLs**
  (`/api/v1/auth/login`, `/api/v1/pipelines`), so it never makes a
  cross-origin request and the CORS layer is not on the request path.
- `GM_CONSOLE_ALLOWED_ORIGINS` is declared in the manifest rather than
  left to the compiled-in default, so the deployed allow-list is stated
  where the deployment can see it.

## TLS and the edge tier

None of this is in this repository yet.

If an edge is added, the posture is fixed by
`deploy/k8s/gm-console.yaml`: **envoy runs as its own Deployment, not as
an istio sidecar.** A sidecar is specifically ruled out there. Do not
add a sidecar to satisfy an ingress requirement without revisiting that
decision.

## What a hosted tier would still need

None of the following exists, and each is a prerequisite rather than a
detail:

- an Ingress, or an envoy Deployment, in `deploy/k8s/`
- a deploy job in CI -- there is none today in any workflow
- a second manifest for a staging Deployment
- TLS termination and a certificate source

The `images` job in `ci.yml` is the closest thing that exists: it builds
each image, runs the container and probes it with curl. That runs on the
GitHub runner and is not a public URL.

## Troubleshooting

| Symptom                             | Likely cause                                  | Action |
|-------------------------------------|-----------------------------------------------|--------|
| `Bad Gateway` from `/api/*`         | upstream `ada-api-gateway` down               | `kubectl get pods -l app=ada-api-gateway` (namespace `default`) |
| Console pod not Ready               | `gm-console` failing its `/healthz` probe     | `kubectl describe pod -l app=gm-console` |
| `404` on a static asset             | `GM_CONSOLE_STATIC_DIR` does not match the mount | see `deploy/k8s/README.md` |
| Any TLS problem                     | not applicable -- there is no TLS in this repo | n/a |
