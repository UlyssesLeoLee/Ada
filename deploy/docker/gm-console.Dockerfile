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

# The frontend, baked in. `apps/gm-console-web/dist/` is the committed,
# hand-authored console shell: this repository has no bundler and no
# package.json (see the note at the bottom of this file), so there is
# nothing to build and the committed dist IS the build output.
#
# Baking it in is what makes the image self-contained: the Deployment in
# deploy/k8s/gm-console.yaml needs no volume for it, which is the point --
# that pod runs readOnlyRootFilesystem: true, so a document root on a
# bind mount is a second thing to get right, and an empty one is a
# console that serves its placeholder shell.
#
# Owned by `ada` and world-readable, so the `readOnlyRootFilesystem` pod
# can read it without any chmod on the host side.
RUN mkdir -p /srv/static && chown ada:ada /srv/static

# Copied from the build stage, not from the context, so the only tree this
# image ever sees is the one `cargo build` already consumed. A build stage
# that failed to receive the dist (a `.dockerignore` pattern, a moved
# directory) fails the build at the cargo step instead of quietly
# producing an image with no frontend in it.
COPY --from=build /src/apps/gm-console-web/dist/ /srv/static/

# An image cannot serve a console it does not have, and a pod running
# readOnlyRootFilesystem cannot repair that at runtime. So the baked dist
# is checked for completeness here, at image-build time, and a missing
# file fails the build instead of shipping a console that 404s on its own
# assets.
#
# index.html, robots.txt and sitemap.xml are the files routes.rs
# include_str!s out of this directory, so the build would already have
# failed if the directory were empty; this catches a dist that has the
# shell but lost the assets it references. tokens.css is referenced by
# index.html and login.html, and login.html is reachable at /login.
RUN set -eu; \
    for f in index.html login.html robots.txt sitemap.xml tokens.css; do \
      if [ ! -f "/srv/static/$f" ]; then \
        echo "gm-console: /srv/static/$f is not present in apps/gm-console-web/dist/ -- refusing to publish an image with an incomplete console" >&2; \
        exit 1; \
      fi; \
    done; \
    chmod -R a+rX /srv/static

COPY --from=build /usr/local/bin/gm-console-server /usr/local/bin/gm-console-server

USER ada
WORKDIR /home/ada

ENV GM_CONSOLE_BIND=0.0.0.0:8080 \
    GM_CONSOLE_UPSTREAM=http://ada-api-gateway:8080 \
    GM_CONSOLE_STATIC_DIR=/srv/static \
    RUST_LOG=info \
    RUST_BACKTRACE=1

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8080/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/gm-console-server"]

# ---------------------------------------------------------------- static ----
# The console's document root is /srv/static, baked in above, and
# GM_CONSOLE_STATIC_DIR points at it. deploy/k8s/gm-console.yaml sets the
# same value; `baked_static_dir_agrees_between_the_image_and_the_manifest`
# in crates/gm-console/tests/baked_static.rs fails if they ever diverge.
#
# ## "CI builds the frontend" is not what this image does
#
# It cannot, because there is nothing to build. `apps/gm-console-web/`
# holds `dist/` and `scripts/` and no package.json, no vite/webpack/next
# config, and no JS or TS source tree anywhere in the repository -- the
# only package.json is a template for the wasm canvas editor, and the only
# JavaScript is screenshot-rendering tooling. `dist/index.html` is
# hand-authored and tracked, and its own meta tag says
# `<meta name="generator" content="gm-console shell v0.1.0">`.
#
# So the committed dist is the frontend, and "bake it in" is the whole of
# the build step. The completeness check above is the part of "CI builds
# dist" that could be made real: it catches a dist missing files the
# console needs. It cannot catch a dist that is *stale*, and nothing
# here pretends otherwise. If a real bundler is introduced later, this
# COPY becomes `RUN npm ci && npm run build` and the check above stays.
#
# For local iteration, override the baked tree with a bind mount:
#   docker run -p 8080:8080 -v "$PWD/apps/gm-console-web/dist:/srv/static:ro" \
#     gm-console:local
# No `-e GM_CONSOLE_STATIC_DIR` needed -- the image already sets it, and
# the mounted path is the same one. /srv/static is owned by `ada` so a
# read-only mount needs no chmod on the host.
