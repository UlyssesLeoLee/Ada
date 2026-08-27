#!/usr/bin/env bash
# init-bucket.sh — MinIO bucket bootstrap.
#
# Usage (from the docker-compose host, after `docker compose up`):
#   docker exec -it ada-minio-mc /scripts/init-bucket.sh
#
# Or run the equivalent mc commands from a workstation that
# has the `mc` client configured. The script is idempotent:
# re-running it after the bucket exists will only update the
# lifecycle policy.
#
# Steps:
#   1. Wait for the `minio` service to be reachable on :9000.
#   2. `mc alias set ada http://minio:9000 <user> <password>`.
#      Credentials come from the env vars below; the
#      docker-compose file injects them from .env.
#   3. Create the `prometheus-tsdb` bucket (no-op if exists).
#   4. Set a 90-day expiration lifecycle on the bucket so
#      old samples get cleaned up automatically. The
#      Prometheus remote_write target in
#      `prometheus/prometheus.yml` writes into this bucket.
#   5. Print a one-line summary the operator can paste into
#      a runbook.

set -euo pipefail

# ---------------------------------------------------------------------
# Configuration — overridable via env (docker-compose sets
# MINIO_ROOT_USER / MINIO_ROOT_PASSWORD from .env).
# ---------------------------------------------------------------------
: "${MINIO_ROOT_USER:=minioadmin}"
: "${MINIO_ROOT_PASSWORD:=minioadmin}"
: "${MINIO_ENDPOINT:=http://minio:9000}"
: "${MINIO_BUCKET:=prometheus-tsdb}"
: "${MINIO_RETENTION_DAYS:=90}"
# Additional buckets owned by other backends. Phase 4
# (T002) added `tempo-blocks` for the Tempo trace store;
# the script creates both buckets and applies the same
# retention policy to each. The `init_minio_extra_buckets`
# env var is the documented extension point for future
# buckets (e.g. WAL, loki-chunks if we ever offload them).
: "${MINIO_EXTRA_BUCKETS:=tempo-blocks}"

# mc uses its own env-var convention; mirror ours.
: "${MC_HOST_minio:=${MINIO_ROOT_USER}:${MINIO_ROOT_PASSWORD}@${MINIO_ENDPOINT#http://}}"

echo "==> MinIO init: endpoint=${MINIO_ENDPOINT} bucket=${MINIO_BUCKET} retention=${MINIO_RETENTION_DAYS}d"

# ---------------------------------------------------------------------
# 1. wait for minio
# ---------------------------------------------------------------------
echo -n "    waiting for MinIO "
for _ in $(seq 1 30); do
    if mc --quiet ready "${MC_HOST_minio}" 2>/dev/null; then
        echo "ready"
        break
    fi
    echo -n "."
    sleep 1
done

if ! mc --quiet ready "${MC_HOST_minio}" 2>/dev/null; then
    echo " NOT ready" >&2
    echo "ERROR: MinIO did not become ready in 30s. Check docker logs ada-minio." >&2
    exit 1
fi

# ---------------------------------------------------------------------
# 2. alias (always set; cheap and lets `mc` discover the
# endpoint by name even outside the container).
# ---------------------------------------------------------------------
mc alias set ada "${MINIO_ENDPOINT}" "${MINIO_ROOT_USER}" "${MINIO_ROOT_PASSWORD}" >/dev/null

# ---------------------------------------------------------------------
# 3. bucket
# ---------------------------------------------------------------------
if mc --quiet ls "ada/${MINIO_BUCKET}" >/dev/null 2>&1; then
    echo "    bucket ada/${MINIO_BUCKET} already exists (skip create)"
else
    echo "    creating bucket ada/${MINIO_BUCKET}"
    mc mb "ada/${MINIO_BUCKET}"
fi

# ---------------------------------------------------------------------
# 4. lifecycle — expire objects after MINIO_RETENTION_DAYS days.
# Phase 6 design: 90 days per 10-deployment-design.md §5.1
# (MinIO = 2 TiB) and the 11-phased-rollout.md §6 note about
# "30 days for Loki chunks" leaving Prometheus / TSDB on the
# longer retention window. Edit MINIO_RETENTION_DAYS to tune.
# ---------------------------------------------------------------------
LIFECYCLE_FILE="$(mktemp)"
trap 'rm -f "${LIFECYCLE_FILE}"' EXIT
cat >"${LIFECYCLE_FILE}" <<EOF
{
  "Rules": [
    {
      "ID": "expire-after-retention",
      "Status": "Enabled",
      "Filter": { "Prefix": "" },
      "Expiration": { "Days": ${MINIO_RETENTION_DAYS} }
    }
  ]
}
EOF

echo "    setting ${MINIO_RETENTION_DAYS}-day lifecycle on ada/${MINIO_BUCKET}"
mc ilm import "ada/${MINIO_BUCKET}" <"${LIFECYCLE_FILE}"

# ---------------------------------------------------------------------
# 4b. additional buckets (Phase 4 adds tempo-blocks; future
# phases can extend by appending to MINIO_EXTRA_BUCKETS).
# Same lifecycle policy applies to all of them so the
# observability stack has a single 90-day retention floor.
# ---------------------------------------------------------------------
for bucket in ${MINIO_EXTRA_BUCKETS}; do
    if mc --quiet ls "ada/${bucket}" >/dev/null 2>&1; then
        echo "    bucket ada/${bucket} already exists (skip create)"
    else
        echo "    creating bucket ada/${bucket}"
        mc mb "ada/${bucket}"
    fi
    echo "    setting ${MINIO_RETENTION_DAYS}-day lifecycle on ada/${bucket}"
    mc ilm import "ada/${bucket}" <"${LIFECYCLE_FILE}"
done

# ---------------------------------------------------------------------
# 5. summary
# ---------------------------------------------------------------------
cat <<EOF

==> MinIO bucket ready.
    Bucket:    ada/${MINIO_BUCKET}
    Endpoint:  ${MINIO_ENDPOINT}
    Retention: ${MINIO_RETENTION_DAYS} days
    Console:   http://localhost:9001  (user: ${MINIO_ROOT_USER})

    S3 client config (e.g. awscli):
        export AWS_ACCESS_KEY_ID=${MINIO_ROOT_USER}
        export AWS_SECRET_ACCESS_KEY=${MINIO_ROOT_PASSWORD}
        aws --endpoint-url ${MINIO_ENDPOINT} s3 ls s3://${MINIO_BUCKET}
EOF
