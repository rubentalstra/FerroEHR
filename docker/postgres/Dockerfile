# syntax=docker/dockerfile:1
#
# Preconfigured PostgreSQL 18 for ehrbase-rs (mirrors the official
# `ehrbase/ehrbase-v2-postgres` two-image model, built fresh for this stack).
#
# The stock postgres:18.4 image, plus init scripts that create the app login
# role, its owned database, the `ehr` + `ext` schemas it owns, and the required
# extensions installed by the bootstrap superuser — so the app role never needs
# superuser. Schema CONTENT is owned by the app's sqlx migrators at boot; this
# image bakes NO migration state (see README.md).
#
# Digest-pinned (resolved 2026-07-24): a mutable tag can be re-pointed, a digest
# cannot (docs.docker.com/build/building/best-practices — "pinning to a digest
# guarantees the same image version"). 18.4 is the latest PG 18 patch
# (docs/VERSIONS.md); bump the tag AND the digest together.
FROM postgres:18.4@sha256:3a82e1f56c8f0f5616a11103ac3d47e632c3938698946a7ad26da0df1334744a

# Init scripts run once, on an empty data directory, as the bootstrap superuser.
COPY initdb/ /docker-entrypoint-initdb.d/

LABEL org.opencontainers.image.title="ehrbase-rs-postgres" \
      org.opencontainers.image.description="PostgreSQL 18 preconfigured (role/db/schemas/extensions) for ehrbase-rs" \
      org.opencontainers.image.source="https://github.com/rubentalstra/ehrbase-rs" \
      org.opencontainers.image.licenses="Apache-2.0"
