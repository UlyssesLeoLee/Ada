# MinIO — Long-term storage for Prometheus TSDB

> **Phase 6 prototype.** Prometheus ships samples to MinIO
> via `remote_write` (see `../prometheus/prometheus.yml`).
> In production a `mimir-distributor` or `thanos-receiver`
> is expected to sit in front of the bucket per
> `docs/observability/10-deployment-design.md` §4.3; the
> MinIO service and the `prometheus-tsdb` bucket are
> provisioned regardless so the lifecycle policy is in
> place when the adapter lands.

## Quick reference

| Service       | URL                          | Auth                    |
|---------------|------------------------------|-------------------------|
| MinIO API     | `http://localhost:9000`      | from `.env` (`minioadmin` / `minioadmin` by default) |
| MinIO Console | `http://localhost:9001`      | same credentials         |
| Bucket        | `prometheus-tsdb`            | 90-day lifecycle         |

## Bring-up

```bash
# 1. start the stack (MinIO + mc init + prometheus)
docker compose -f observability/docker-compose.yml up -d minio mc

# 2. run the init script inside the mc container
docker exec -it ada-minio-mc /scripts/init-bucket.sh
```

`init-bucket.sh` is idempotent — running it twice will only
update the lifecycle policy.

## Override credentials

The credentials default to `minioadmin` / `minioadmin` for
the local dev stack. **Change them before going to
production.** Edit `observability/.env` (the file is
auto-generated on first `init.sh` run):

```bash
MINIO_ROOT_USER=ada-obs
MINIO_ROOT_PASSWORD=replace-with-strong-passphrase
```

The same values land in the `alertmanager` and `prometheus`
services via docker-compose's `${MINIO_*}` interpolation.

## Why a placeholder remote_write?

Prometheus's `remote_write` protocol (snappy-compressed
protobuf) is not the S3 API. The current prometheus.yml
points `remote_write.url` at the MinIO HTTP endpoint as a
*placeholder*; samples will not actually land in the
bucket until a remote_write-compatible receiver
(`mimir-distributor` or `thanos-receiver`) is deployed in
front of MinIO. The bucket + lifecycle are created up
front so the receiver can attach to a working target.

See `docs/observability/10-deployment-design.md` §4.3 for
the Mimir / Thanos layout that replaces the placeholder.
