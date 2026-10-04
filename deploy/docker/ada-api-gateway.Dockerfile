# ada-api-gateway — the public API surface.
#
# Referenced by deploy/k8s/ada-api-gateway.yaml as
#   ghcr.io/ulyssesleolee/ada-api-gateway:v0.1.0
# listening on containerPort 8080, probed at /health/ready and
# /health/live. Keep the three in step: a probe path that the binary does
# not serve is a CrashLoopBackOff that no build error explains, which is
# how the remediation service used to fail to start.
#
# Build:
#   docker build -f deploy/docker/ada-api-gateway.Dockerfile -t ada-api-gateway:local .
#
# The 1.98 in the builder tag is not decoration. CI pins toolchain 1.98
# (see .github/workflows/ci.yml) and the workspace sets rust-version, so
# the image is built by the same compiler that gates the commit.

# syntax=docker/dockerfile:1

# ---------------------------------------------------------------- build ----
FROM rust:1.98-bookworm AS build
WORKDIR /src

# Cache mounts rather than the copy-the-manifests-then-build-a-dummy-crate
# dance: the dummy trick has to stub a `src/` for every one of the 24
# workspace members and every declared `[[bin]]` path, and any stub that
# does not match the real manifest is usually papered over with `|| true`,
# which leaves a cache layer that is quietly wrong. A cache mount keeps the
# incremental speed and cannot go stale in that way.
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --release --locked -p ada-m13-api-gateway --bin ada-api-gateway \
 && cp /src/target/release/ada-api-gateway /usr/local/bin/ada-api-gateway

# --------------------------------------------------------------- runtime ----
FROM debian:bookworm-slim AS runtime

# ca-certificates: this build uses rustls + webpki-roots, which carries its
# own trust store, so nothing here needs the system CAs today. It is
# installed anyway because the alternative is an image that breaks the day
# a dependency reaches for native-tls, and the failure would look like an
# unexplained TLS error in production.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# Non-root. The binary binds 8080, which needs no privilege, and a
# compromised process should not find itself root.
#
# The uid/gid are explicit, not left to `useradd --system`: the three
# Deployment manifests in deploy/k8s/ pin runAsUser/runAsGroup to 65532,
# so the image and the manifest have to agree on a number. A
# build-order-dependent system uid would make that pin meaningless.
RUN groupadd --gid 65532 ada \
 && useradd --uid 65532 --gid 65532 --create-home --home-dir /home/ada --shell /usr/sbin/nologin ada

COPY --from=build /usr/local/bin/ada-api-gateway /usr/local/bin/ada-api-gateway

# The casbin model and policy, in the RUNTIME stage.
#
# They were previously copied in the build stage, where they did nothing:
# the runtime image is assembled from `debian:bookworm-slim` and copies in
# only the binary, so every layer written to the build stage is discarded.
# The gateway then started, read the env var set below, and died on a
# directory that was never in the image:
#
#   fatal: internal error: build rbac enforcer: policy reload failed:
#   policy file not found: /etc/ada-rbac/policies/model.conf
#
# A COPY into the wrong stage builds cleanly and fails only at run time.
#
# This path is only meaningful because PolicySet::bundled() prefers
# ADA_RBAC_POLICY_DIR over its compiled-in default. Both files are small text
# config and part of the authorization model rather than runtime state, so
# baking them in is correct.
COPY crates/ada-rbac-casbin/policies/ /etc/ada-rbac/policies/

USER ada
WORKDIR /home/ada

ENV ADA_GATEWAY_BIND=0.0.0.0:8080 \
    ADA_RBAC_POLICY_DIR=/etc/ada-rbac/policies \
    RUST_LOG=info \
    RUST_BACKTRACE=1

EXPOSE 8080

# Same path the manifest's readinessProbe uses. A HEALTHCHECK that probes a
# different endpoint than the orchestrator probes is a second source of
# truth that drifts, so this deliberately reuses /health/ready.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8080/health/ready || exit 1

ENTRYPOINT ["/usr/local/bin/ada-api-gateway"]
