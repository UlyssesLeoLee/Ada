# gm-console — the commercial Web operations console.
#
# Referenced by deploy/k8s/gm-console.yaml as
#   ghcr.io/ulyssesleolee/gm-console:v0.1.0
# listening on containerPort 8080, probed at /healthz. The manifest maps
# Service port 80 to targetPort 8080.
#
# Build:
#   docker build -f deploy/docker/gm-console.Dockerfile -t gm-console:local .
#
# The binary name is gm-console-server, not gm-console: the crate is
# `gm-console` and its `[[bin]]` is renamed. Getting this wrong produces
# an image that builds cleanly and then exits 127 on start.

# syntax=docker/dockerfile:1

# ---------------------------------------------------------------- build ----
FROM rust:1.98-bookworm AS build
WORKDIR /src

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --release --locked -p gm-console --bin gm-console-server \
 && cp /src/target/release/gm-console-server /usr/local/bin/gm-console-server

# --------------------------------------------------------------- runtime ----
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# uid/gid are explicit so they match the runAsUser/runAsGroup pinned to
# 65532 in deploy/k8s/gm-console.yaml.
RUN groupadd --gid 65532 ada \
 && useradd --uid 65532 --gid 65532 --create-home --home-dir /home/ada --shell /usr/sbin/nologin ada

# Mount point for the SPA dist, created and owned by `ada` so the
# read-only bind mount documented at the bottom of this file works without
# chmod gymnastics on the host.
RUN mkdir -p /srv/static && chown ada:ada /srv/static

COPY --from=build /usr/local/bin/gm-console-server /usr/local/bin/gm-console-server

USER ada
WORKDIR /home/ada

ENV GM_CONSOLE_BIND=0.0.0.0:8080 \
    GM_CONSOLE_UPSTREAM=http://ada-api-gateway:8080 \
    RUST_LOG=info \
    RUST_BACKTRACE=1

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8080/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/gm-console-server"]

# ---------------------------------------------------------------- static ----
# The SPA is NOT baked into this image, and that is a known gap rather than
# an oversight. `gm-console` serves static files from disk via
# GM_CONSOLE_STATIC_DIR; it does not embed them (it still declares
# `rust-embed` in Cargo.toml, which nothing in the crate uses). There is
# also no frontend build in this repository to copy from, so
# GM_CONSOLE_STATIC_DIR is left unset here and / returns the placeholder
# the binary emits for a missing document root.
#
# Closing this needs a decision, not a Dockerfile: where the frontend
# lives, how it is built, and whether the dist is baked in or mounted as a
# volume. Until then, mount one to make the console serve real assets:
#   docker run -p 8080:8080 -v "$PWD/apps/gm-console-web/dist:/srv/static:ro" \
#     -e GM_CONSOLE_STATIC_DIR=/srv/static gm-console:local
# The directory is created and owned by `ada` so a read-only mount works
# without chmod gymnastics on the host.
