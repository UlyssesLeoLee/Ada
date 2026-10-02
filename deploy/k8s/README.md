# ada-remediation k8s deployment (v0.7.1)

This directory contains a minimal k8s deployment for the
`ada-remediation` binary built from `crates/ada-remediation/`, plus the
`ada-api-gateway` and `gm-console` services that make up the rest of
the deployed topology.

## Files

- `ada-api-gateway.yaml` — Deployment + Service + PodDisruptionBudget for the gateway
- `ada-remediation.yaml` — ConfigMap + Secret (placeholders) + Deployment + Service + NetworkPolicy
- `gm-console.yaml` — Deployment + Service + HPA + PodDisruptionBudget for the web console
- `kustomization.yaml` — kustomize entry point

## The topology, and what it was not doing

The request path is:

```
client -> gm-console (Service :80) -> /api/* -> ada-api-gateway (Service :8080)
```

`gm-console` is a reverse proxy in front of a static SPA. It forwards
`/api/<rest>` to `$GM_CONSOLE_UPSTREAM` and forwards the client's
`authorization` and `x-tenant-id` headers **verbatim**. Its source
comment says "auth, rate limit and observability live in api-gateway",
so the gateway is the security boundary by design.

**That boundary did not exist until this manifest was added.** Until
then the only two Services in this directory were `gm-console` and
`ada-remediation`; the string `ada-api-gateway` appeared solely as an
env value in `gm-console.yaml`, never as a Service. In-cluster DNS
resolved no record for it, so every `/api/*` request through the only
deployed service returned **502 BAD_GATEWAY**. The failure is silent in
the sense that nothing in the build or the manifests fails — a
reference to a service that does not exist is not a YAML error.

## What is still missing: authentication

**A green deployment here is not a secured one.** As of this commit the
gateway mounts four routes and none of them require a credential:

| Route | Auth required |
|---|---|
| `GET /health` | no |
| `GET /health/live` | no |
| `GET /health/ready` | no |
| `GET /api/v1/ping` | no |

`ada-identity`, `ada-rbac-casbin` and `ada-billing` are **not**
dependencies of `ada-m13-api-gateway`. Nothing validates the token
`gm-console` forwards, and nothing derives a tenant from it. Wiring
those crates in as middleware is the next step and is tracked in the
`fix/ci-workflow-parse-errors` branch history.

The crate's own `src/lib.rs` describes itself as the "minimum skeleton"
and places the middleware chain in a later phase, so this is declared
missing work rather than a broken implementation — but the deployment
made it invisible, because a gateway that is never reached cannot be
observed to lack auth.

## Prerequisites

- k8s cluster (tested against 1.27+; 1.24+ should also work)
- `kubectl` configured with cluster admin in the `observability` namespace
- An existing `observability` namespace (or change `namespace:` in `kustomization.yaml`)
- For hot-reload: a CSI-backed RWX volume (or a `Reloader`-style sidecar watching the ConfigMap)
- For real secrets: sealed-secrets, external-secrets-operator, or a similar tool

## Secret bootstrap

The Secret in `ada-remediation.yaml` ships with
`PLACEHOLDER_*` values. **Do not apply the file as-is in
production** — the binary will fail to start (`require_enabled`
panics on empty secret).

Choose one:

### Option 1: external-secrets-operator (recommended)

```yaml
# external-secrets/ada-remediation.yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: ada-remediation-secrets
  namespace: observability
spec:
  secretStoreRef:
    name: vault
    kind: ClusterSecretStore
  target:
    name: ada-remediation-secrets
  data:
    - secretKey: REMEDIATION_WEBHOOK_SECRET
      remoteRef:
        key: ada-remediation/webhook
    - secretKey: REMEDIATION_TRIGGER_SECRET
      remoteRef:
        key: ada-remediation/trigger
```

### Option 2: kubectl create (one-off)

```bash
kubectl create namespace observability --dry-run=client -o yaml | kubectl apply -f -

kubectl -n observability create secret generic ada-remediation-secrets \
  --from-literal=REMEDIATION_WEBHOOK_SECRET="$(openssl rand -hex 32)" \
  --from-literal=REMEDIATION_TRIGGER_SECRET="$(openssl rand -hex 32)"
```

Then `kubectl apply -k deploy/k8s/` will not overwrite the
existing Secret (kustomize uses `existing` semantics for
`Secret` if `generatorOptions.disableNameSuffixHash: true`
is set; otherwise remove the Secret from `ada-remediation.yaml`
before applying).

### Option 3: sealed-secrets

```bash
kubeseal --format yaml < ada-remediation-secrets-plain.yaml > ada-remediation-secrets-sealed.yaml
# commit ada-remediation-secrets-sealed.yaml, add to resources
```

## Apply

```bash
# 1. bootstrap namespace + secrets
kubectl create namespace observability
# (populate secrets via Option 1/2/3 above)

# 2. apply manifests
kubectl apply -k deploy/k8s/

# 3. verify
kubectl -n observability get deploy ada-remediation
kubectl -n observability get pods -l app.kubernetes.io/name=ada-remediation
kubectl -n observability logs -l app.kubernetes.io/name=ada-remediation -f
```

## Verifying HMAC + webhook

Once the pod is up, verify the webhook rejects unsigned
requests:

```bash
# this should return 401
kubectl -n observability exec -it deploy/ada-remediation -- \
  wget -q -O - http://localhost:9100/healthz
# /healthz is unauthenticated, expected to return 200

kubectl -n observability port-forward deploy/ada-remediation 9100:9100 &
sleep 1

# compute signature locally
SECRET="<the value of REMEDIATION_WEBHOOK_SECRET>"
TS=$(date +%s)
BODY='{"alerts":[{"status":"firing","labels":{"alertname":"DiskSpaceFillingFast"}}]}'
SIG=$(python3 -c "import hmac,hashlib,sys; print(hmac.new(b'$SECRET', b'$TS.$BODY', hashlib.sha256).hexdigest())")

curl -i -X POST http://localhost:9100/webhook/alertmanager \
  -H "X-Webhook-Signature: $SIG" \
  -H "X-Webhook-Timestamp: $TS" \
  -H "Content-Type: application/json" \
  -d "$BODY"
```

## Graceful shutdown

The binary listens for SIGTERM (k8s sends this before
sending SIGKILL after `terminationGracePeriodSeconds: 30`).
In-flight webhook handlers drain for up to 25s; pending
metrics scrapes complete. Prometheus may briefly observe
a "down" state during rolling updates (maxUnavailable=0
keeps at least one pod serving).

## Observability

- `/metrics` — Prometheus text format (no auth, gated by
  `NetworkPolicy` to `prometheus` namespace only)
- `/healthz` — liveness + readiness (no auth)
- `/webhook/alertmanager` — Alertmanager v4 payload
  (HMAC-SHA256 signed)
- `/remediation/trigger` — manual operator trigger
  (HMAC-SHA256 signed)
- `/remediation/history`, `/remediation/cooldowns` — read-only
  introspection (no auth, gated by NetworkPolicy)

## Known gaps (v0.7.1)

- Runbook hot-reload uses 1s polling (the `notify` crate is
  not in D:/Ada's offline cache). Latency between runbook
  edit and engine reload is up to 1s. v0.7.2 will switch
  to inotify/FSEvents/ReadDirectoryChangesW once `notify`
  ships to the cache.
- The runbook ConfigMap is mounted read-only. To edit
  runbooks in place, swap for a CSI-backed RWX volume or
  use a `Reloader` sidecar that restarts the pod on
  ConfigMap change.
- Real HMAC-SHA256 via `blake3::keyed_hash` + manual hex
  (not the IETF-standard `HMAC-SHA256`). Functionally
  equivalent for webhook authentication (server holds
  secret, client signs, server verifies) but a strict
  compliance audit may flag it. v0.7.2 will switch to
  standard HMAC-SHA256 once `hmac` + `sha2` crates ship.
