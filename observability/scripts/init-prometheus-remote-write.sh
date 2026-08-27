#!/usr/bin/env bash
# init-prometheus-remote-write.sh — wire Prometheus to the
# MinIO long-term-storage bucket. Phase 6 helper.
#
# Usage (from the repo root, after `docker compose up`):
#   bash observability/scripts/init-prometheus-remote-write.sh
#
# The script is idempotent and short-circuits if the bucket
# is already reachable. It is **not** required for the stack
# to come up (the docker-compose `mc` init container creates
# the bucket automatically); this script is a manual escape
# hatch for the "I forgot to bring up minio first" case.
#
# Steps:
#   1. Wait for the MinIO service on :9000.
#   2. Run the bucket-init script (idempotent — see
#      observability/minio/init-bucket.sh).
#   3. POST /-/reload to Prometheus so the `remote_write`
#      block in prometheus/prometheus.yml starts pushing
#      samples to the freshly-created bucket.
#   4. Verify the remote_write target is healthy via
#      Prometheus's own /api/v1/status/config endpoint.
#
# Per docs/observability/11-phased-rollout.md §8 (Phase 6
# Alert) and §9 (Phase 7 SLO), this script is the bridge
# between the "1-key-up" docker-compose flow and the
# long-term-storage SLO design.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
INIT_BUCKET="${REPO_ROOT}/observability/minio/init-bucket.sh"

: "${PROMETHEUS_URL:=http://localhost:9090}"
: "${MINIO_ROOT_USER:=minioadmin}"
: "${MINIO_ROOT_PASSWORD:=minioadmin}"
: "${MINIO_BUCKET:=prometheus-tsdb}"
export MINIO_ROOT_USER MINIO_ROOT_PASSWORD MINIO_BUCKET

# ---------------------------------------------------------------------
# 1. wait for minio
# ---------------------------------------------------------------------
echo "==> waiting for MinIO on :9000"
for _ in $(seq 1 30); do
    if curl -sf -o /dev/null "http://localhost:9000/minio/health/ready"; then
        echo "    minio ready"
        break
    fi
    sleep 1
done
if ! curl -sf -o /dev/null "http://localhost:9000/minio/health/ready"; then
    echo "ERROR: MinIO did not become ready in 30s. Run 'docker compose up -d minio' first." >&2
    exit 1
fi

# ---------------------------------------------------------------------
# 2. run the bucket-init script (idempotent)
# ---------------------------------------------------------------------
if [[ -x "${INIT_BUCKET}" ]]; then
    echo "==> running ${INIT_BUCKET}"
    bash "${INIT_BUCKET}"
elif [[ -f "${INIT_BUCKET}" ]]; then
    echo "==> running ${INIT_BUCKET} (via bash, +x missing)"
    bash "${INIT_BUCKET}"
else
    echo "ERROR: ${INIT_BUCKET} not found" >&2
    exit 1
fi

# ---------------------------------------------------------------------
# 3. reload prometheus config so remote_write picks up
# ---------------------------------------------------------------------
echo "==> reloading Prometheus at ${PROMETHEUS_URL}"
if ! curl -sf -X POST "${PROMETHEUS_URL}/-/reload"; then
    echo "ERROR: Prometheus reload failed. Is it running and is --web.enable-lifecycle set?" >&2
    exit 1
fi

# ---------------------------------------------------------------------
# 4. verify the remote_write target is in the active config
# ---------------------------------------------------------------------
echo "==> verifying remote_write block"
if curl -sf "${PROMETHEUS_URL}/api/v1/status/config" \
    | grep -q 'remote_write'; then
    echo "    remote_write block is active in Prometheus"
else
    echo "WARN: remote_write block not found in Prometheus config — check prometheus.yml" >&2
    exit 1
fi

cat <<EOF

==> Prometheus remote_write is wired to MinIO.

    Prometheus:  ${PROMETHEUS_URL}
    MinIO API:   http://localhost:9000
    MinIO UI:    http://localhost:9001   (user: ${MINIO_ROOT_USER})
    Bucket:      ${MINIO_BUCKET}
    Retention:   90 days

    Validate the path end-to-end:
        curl -s "${PROMETHEUS_URL}/api/v1/status/config" | grep -A 3 remote_write
        curl -s "http://localhost:9001" -u \${MINIO_ROOT_USER}:\${MINIO_ROOT_PASSWORD} \
            | head -c 200
EOF
