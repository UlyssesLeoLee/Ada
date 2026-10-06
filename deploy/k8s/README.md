# ada-remediation k8s deployment (v0.7.1)

This directory contains a minimal k8s deployment for the
`ada-remediation` binary built from `crates/ada-remediation/`, plus the
`ada-api-gateway` and `gm-console` services that make up the rest of
the deployed topology.

## Files

- `ada-api-gateway.yaml` — Deployment + Service + PodDisruptionBudget for the gateway
- `ada-remediation.yaml` — ConfigMap + Secret (placeholders) + Deployment + Service + NetworkPolicy
- `gm-console.yaml` — Deployment + Service + HPA + PodDisruptionBudget for the web console
- `kustomization.yaml` — kustomize entry point, and the single place
  the image registry is named

This directory holds **only images this repository publishes**, and
`crates/ada-core/tests/deploy_images.rs` fails the build if one of them
resolves to a different registry or a different tag convention. That is
why the gateway's session store is not here: `deploy/infra/ada-session-redis.yaml`
holds it, alongside the other operator-supplied prerequisites below.

## The images build, but nothing publishes them

**Applying these manifests produces three Deployments that never start.**
All three images are referenced, and none of them can be pulled:

```text
ghcr.io/ulyssesleolee/ada-api-gateway:v0.1.0
ghcr.io/ulyssesleolee/ada-remediation:v0.7.1
ghcr.io/ulyssesleolee/gm-console:v0.1.0
```

The Dockerfile gap that used to cause this is closed — there are now
three, under `deploy/docker/`, and the `images` CI job builds each one
and starts it to confirm it serves its probe path. What is still missing
is publication: that job runs with `push: false`, and `release.yml` cuts
a GitHub release from a compiled `gm-console-server` binary rather than
from an image. So `ghcr.io/ulyssesleolee/*` is empty, and a fresh
`kubectl apply -k deploy/k8s/` lands every pod in `ImagePullBackOff`.

Two details that make this awkward to diagnose:

- The three services are Rust workspace members, so a build needs the
  whole workspace, not one crate directory. `ada-remediation` in
  particular only produces its binary with `--features bin`.
- `gm-console` ships its UI. The SPA is a committed, hand-authored
  `dist/` (there is no bundler in this repository), and the image copies
  it to `/srv/static` and points `GM_CONSOLE_STATIC_DIR` there, with a
  build-time completeness check that refuses to publish an image whose
  `/srv/static` is missing files the committed dist has.

This is recorded rather than fixed here because writing three
Dockerfiles that cannot be built is the same mistake as the webhook
signing example this file used to carry: an artifact that looks right and
fails on first use. The build needs a base image and a crate registry,
and neither was reachable while this was written.

### One registry, one tag format

The three references used to disagree, which nothing flagged — each line
is a valid image reference and each manifest reads fine alone:

```text
ghcr.io/ada-project/ada-api-gateway:0.1.0
ghcr.io/ulysse/ada-remediation:v0.7.1     <- a second registry
ghcr.io/ada-project/gm-console:0.1.0      <- and no `v` prefix
```

Neither original namespace matched the account that owns this repository
(`git@github.com:UlyssesLeoLee/Ada.git`): `ada-project` is not a namespace
that owner can push to, and `ulysse` reads like the username truncated. All
three now point at `ghcr.io/ulyssesleolee/`, which is what a `docker push`
from that account actually produces.

`kustomization.yaml` now declares all three in an `images:` block, so a
retarget is one edit (or `kustomize edit set image <name>=<ref>`), and
`crates/ada-core/tests/deploy_images.rs` fails the build if they drift
apart again. The manifests keep the full reference on purpose:
`kubectl apply -f <file>` bypasses the kustomization, and a bare name
there would silently resolve against the node's default registry.


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

## Authentication and authorization

`/health`, `/health/live` and `/health/ready` are unauthenticated, and
that is deliberate: a kubelet probe cannot carry a bearer token, and a
probe that 401s takes the pod out of service while the API is healthy.

Everything under `/api` requires `Authorization: Bearer <token>`:

| Route | Auth | Authorization |
|---|---|---|
| `GET /health` | no | — |
| `GET /health/live` | no | — |
| `GET /health/ready` | no | — |
| `POST /api/v1/auth/login` | no (this is the credential exchange) | — |
| `GET /api/v1/ping` | bearer | none (smoke endpoint) |
| `GET /api/v1/whoami` | bearer | none (echoes the principal) |
| `GET /api/v1/canvases/:id` | bearer | `Read` on `canvas` |
| `POST /api/v1/canvases/:id/run` | bearer | `Execute` on `canvas` |

An unmatched path is a **404**, not a 401 — see the note on `fallback`
in `crates/ada-m13-api-gateway/src/router.rs` for why that distinction
is enforced by a test.

### The token is an opaque server-side session, not a JWT

`ada-identity` has no asymmetric crypto dependency, so
`mint_jwt` and `verify_jwt_stub` both fail closed for **every** input.
A verifier that merely *decodes* a token would let a caller mint their
own `roles` and their own `tenant_id`, and `tenant_id` is the isolation
key for the whole multi-tenant model. So the gateway issues an opaque
session token instead and looks it up server-side:
`mint` → opaque token → `lookup` → `Session { user_id, tenant_id,
roles, expires_at }`.

The store behind that lookup is `SharedSessionStore`, over Redis, so a
session minted on one replica is a credential on all of them and survives
a restart. The in-process `SessionStore` is **test-only** and is behind a
`cfg`/feature gate, so a production build cannot even name it —
`crates/ada-identity/src/session.rs` carries a `compile_fail` doctest that
fails if that gate is ever removed. Stateless verification is future work
and must not be faked by decoding.

### The tenant is never taken from a header

`gm-console` forwards the browser's `x-tenant-id` verbatim. The
gateway ignores it for every decision. The tenant comes from the
server-side session, so a client asserting a tenant it does not own
changes nothing. This is pinned by two tests — one on `/whoami`, one
on a business route — because a single test on the echo endpoint would
not show that the business path is also safe.

### A fresh pod will not start at all

`AuthContext::bootstrap` connects to `ADA_SESSION_REDIS_URL` and treats
"cannot reach it" as fatal. There is no in-process fallback and no
`SessionStore` you can fall back to, because a per-process table would
401 a valid credential the moment a request landed on the other replica
and would invalidate every outstanding session on restart. An
authentication service that cannot check a credential must not serve.

`ada-api-gateway.yaml` sets the variable to `redis://ada-session-redis:6379`,
but **that Service is not defined in this directory** — see
`deploy/infra/ada-session-redis.yaml`. Applying only `deploy/k8s/` gives
you a gateway in `CrashLoopBackOff`, which is the intended outcome rather
than a silent one. For a running reference topology:

```bash
kubectl apply -f deploy/infra/ada-session-redis.yaml
kubectl apply -k deploy/k8s/
```

The store that ships there is a placeholder — one replica, no
persistence, no auth, no TLS. Sessions are the credential authority for
the whole multi-tenant model, so point `ADA_SESSION_REDIS_URL` at a
managed instance for anything real.

### Even started, it authenticates nobody until you configure it

`POST /api/v1/auth/login` exists and issues an opaque session token, but
the credential directory it reads comes from `ADA_GATEWAY_LOGIN_USERS`,
which no manifest in this directory sets. Unset means the directory is
empty, which means every login is a 401 — and the gateway logs a warning
at startup and serves anyway, because "no credentials configured" and
"credentials configured and all wrong" must be indistinguishable from
outside.

So a green rollout of these manifests is a gateway that is genuinely
fail-closed, not one that has been given a way in.

## Prerequisites

- k8s cluster (tested against 1.27+; 1.24+ should also work)
- `kubectl` configured with cluster admin in the `observability` namespace
- An existing `observability` namespace (or change `namespace:` in `kustomization.yaml`)
- **A reachable Redis for `ada-api-gateway`.** Without it the gateway
  refuses to start. Apply `deploy/infra/ada-session-redis.yaml` for a
  placeholder, or set `ADA_SESSION_REDIS_URL` to a managed instance.
- **A credential directory for the login endpoint** — set
  `ADA_GATEWAY_LOGIN_USERS` to the JSON array described in
  `crates/ada-m13-api-gateway/src/login.rs` (`USERS_ENV_VAR`). Without
  it the gateway serves but authenticates nobody.
- Images published to `ghcr.io/ulyssesleolee/*` (see above — nothing
  publishes them yet)
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

## Verifying the webhook signature

The scheme is **blake3 keyed-hash** over `timestamp || 0x00 || body`
(see `crates/ada-remediation/src/auth.rs`) — *not* HMAC-SHA256. Python's
stdlib `hmac`/`hashlib` therefore cannot produce a signature this server
accepts, even when it is given the right secret and the right bytes. An
earlier version of this file showed an `hmac.new(secret, TS.BODY, sha256)`
example; every request signed that way was rejected.

Produce the signature with the crate's own helper, `auth::sign_at`
(also exposed as `LoggingClient::sign_request`):

```rust
let ts  = ada_remediation::auth::now_unix_secs().to_string();
let sig = ada_remediation::auth::sign_at(secret.as_bytes(), ts.as_bytes(), &body);
```

The timestamp is *inside* the signed material, and that is load-bearing.
A signature over the body alone makes a captured `(body, signature)` pair
a permanent credential: the 5-minute window compares the **supplied**
header against the server clock, so an attacker simply attaches a current
timestamp and the window never expires for them. With the timestamp signed,
refreshing the header invalidates the signature (`403`).

Once the pod is up, verify the webhook rejects unsigned requests:

```bash
# this should return 200 -- /health is unauthenticated
kubectl -n observability exec -it deploy/ada-remediation -- \
  wget -q -O - http://localhost:9100/health
# note: this service does NOT serve /healthz, which is what gm-console serves

kubectl -n observability port-forward deploy/ada-remediation 9100:9100 &
sleep 1

# signed request, with $SIG from auth::sign_at and $TS matching the
# timestamp that was signed:
curl -i -X POST http://localhost:9100/webhook/alertmanager \
  -H "X-Webhook-Signature: $SIG" \
  -H "X-Webhook-Timestamp: $TS" \
  -H "Content-Type: application/json" \
  -d "$BODY"
```

Two failure modes are worth knowing, because the status codes differ:

- `401` — missing/unparseable header, or the timestamp outside the window.
- `403` — the signature did not match. This includes the replay case above.

Two other things about this endpoint as it stands:

- The service starts with **no runbooks**. The five runbooks do
  exist, in `config/remediation/`, and `crates/ada-remediation`
  loads them from there. But the Deployment mounts a ConfigMap
  named `ada-remediation-runbooks` at that path, and that ConfigMap
  is **referenced and never defined** in `deploy/k8s/`. Marked
  `optional: true`, so the mount succeeds with an empty directory
  and nothing warns. A validly signed alert is therefore accepted
  and then matches zero actions — the service runs, answers its
  probes, and remediates nothing.
- `/health`, not `/healthz`, is the health route; see the
  note in `ada-remediation.yaml`.

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
- `/health` — liveness + readiness (no auth). **This service serves
  `/health`, not `/healthz`.** The three services in this directory do
  not agree on the spelling: `gm-console` serves `/healthz`,
  `ada-api-gateway` serves `/health/live` + `/health/ready`, and this
  one serves `/health`. Each manifest has to name its own, and this
  manifest used to name `gm-console`'s — which 404s, so readiness
  never went green and liveness killed the container after ~35s. The
  invariant is now pinned by a test in `ada-core`
  (`probe_paths_match_the_routes_they_call`) that reads every manifest
  and the corresponding router source together.
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
- The signature is `blake3::keyed_hash` with manual hex encoding,
  not the IETF-standard HMAC-SHA256. Functionally equivalent for
  webhook authentication (server holds the secret, client signs,
  server verifies) but a strict compliance audit may flag it.
  v0.7.2 will switch to standard HMAC-SHA256 once `hmac` + `sha2`
  ship. Note that Python's stdlib `hmac`/`hashlib` cannot produce
  a signature this server accepts today, so an external client
  cannot be wired up without a small signing shim.
- The signature covers `timestamp || 0x00 || body`, not the body
  alone. A body-only signature made a captured request replayable
  forever, because the replay window reads the header the caller
  supplied. The cost is that the two parties must agree on the
  timestamp: a client cannot sign once and send twice, and cannot
  batch a queued payload with a stale clock.
