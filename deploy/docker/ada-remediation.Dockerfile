# ada-remediation — the webhook receiver that executes runbooks.
#
# Referenced by deploy/k8s/ada-remediation.yaml as
#   ghcr.io/ulyssesleolee/ada-remediation:v0.7.1
# listening on containerPort 9100, probed at /health and scraped at
# /metrics.
#
# Build:
#   docker build -f deploy/docker/ada-remediation.Dockerfile -t ada-remediation:local .
#
# `--features bin` is not optional. The `[[bin]]` carries
# `required-features = ["bin"]` and `default = []`, so without it cargo
# reports no bin target at all and the image would have nothing to copy.

# syntax=docker/dockerfile:1

# ---------------------------------------------------------------- build ----
FROM rust:1.98-bookworm AS build
WORKDIR /src

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --release --locked -p ada-remediation --features bin --bin ada-remediation \
 && cp /src/target/release/ada-remediation /usr/local/bin/ada-remediation

# --------------------------------------------------------------- runtime ----
FROM debian:bookworm-slim AS runtime

# This is the one service that shells out: ActionStep::RunCommand exists,
# and the runbooks it reads name real programs. A scratch/distroless base
# would be smaller, and would also change what that step is able to run.
# The security boundary is documented in
# crates/ada-core/tests/remediation_command_boundary.rs -- notably that
# run_shell_command uses Command::new(cmd).args(args) and never a shell, so
# these packages are reachable programs, not an injection surface.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# uid/gid are explicit so they match the runAsUser/runAsGroup pinned to
# 65532 in deploy/k8s/ada-remediation.yaml -- that manifest already
# carried the 65532 pin, and this image is the other half of it.
RUN groupadd --gid 65532 ada \
 && useradd --uid 65532 --gid 65532 --create-home --home-dir /home/ada --shell /usr/sbin/nologin ada

COPY --from=build /usr/local/bin/ada-remediation /usr/local/bin/ada-remediation

USER ada
WORKDIR /home/ada

ENV RUST_LOG=info \
    RUST_BACKTRACE=1

EXPOSE 9100

# /health, not /healthz. The k8s manifest probes /health, and the binary
# has never served /healthz -- a Deployment probing the wrong path starts,
# fails its probe, and is restarted forever with no useful log line.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -fsS http://127.0.0.1:9100/health || exit 1

ENTRYPOINT ["/usr/local/bin/ada-remediation"]

# ---------------------------------------------------------------- secrets ----
# No secret is baked in or defaulted here. The manifest mounts
# ada-remediation-secrets and ada-remediation-runbooks (a ConfigMap that
# the repo references but does not define, with `optional: true`); supply
# the webhook secret through the same mechanism:
#   -e ADA_REMEDIATION_WEBHOOK_SECRET_FILE=/run/secrets/webhook-secret
