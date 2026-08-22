# syntax=docker/dockerfile:1
#
# Preconfigured PostgreSQL 18 for ferroehr (mirrors the official
# `ehrbase/ehrbase-v2-postgres` two-image model, built fresh for this stack).
#
# The stock postgres:18.6 image, plus init scripts that create the app login
# role, its owned database, the `ehr` + `ext` schemas it owns, and the required
# extensions installed by the bootstrap superuser — so the app role never needs
# superuser. Schema CONTENT is owned by the app's sqlx migrators at boot; this
# image bakes NO migration state (see README.md).
#
# Digest-pinned (resolved 2026-08-18): a mutable tag can be re-pointed, a digest
# cannot (docs.docker.com/build/building/best-practices — "pinning to a digest
# guarantees the same image version"). 18.6 is the latest PG 18 patch
# (docs/VERSIONS.md; 18.5 was never released upstream); bump the tag AND the
# digest together — the full pin-site checklist is
# .claude/rules/image-security.md.
FROM postgres:18.6@sha256:06cad38a5d9f5d24b4d83d86def30795d5e4b757fedbf5281172b576dedcd941

# Debian security updates, applied at OUR build time (issue #2408). The
# upstream image rebuilds on its own cadence, so a fix already published in
# trixie-security can sit unpulled for weeks in the pinned base — postgres:18.6
# (built 2026-08-13) still ships the util-linux packages whose CVE-2026-53615
# fix trixie-security carries. Plain `upgrade`, never `dist-upgrade`: packages
# are only patched, none added or removed.
RUN set -eux; \
    apt-get update; \
    apt-get -y upgrade; \
    rm -rf /var/lib/apt/lists/*

# Init scripts run once, on an empty data directory, as the bootstrap superuser.
COPY initdb/ /docker-entrypoint-initdb.d/

# See docker/Dockerfile for the split: build-independent keys here, the four
# build-dependent ones from the publishing workflow. base.* names the `FROM`
# above, kept in step by scripts/checks/image-labels.sh.
#
# `licenses` is an SPDX EXPRESSION and this image genuinely is one: our init
# scripts are MIT, and every layer under them is upstream `postgres`, which is
# PostgreSQL-licensed. Claiming only MIT would understate what the image
# contains.
LABEL org.opencontainers.image.title="FerroEHR PostgreSQL" \
      org.opencontainers.image.description="PostgreSQL 18 preconfigured (role/db/schemas/extensions) for ferroehr" \
      org.opencontainers.image.url="https://ferroehr.eu/" \
      org.opencontainers.image.documentation="https://ferroehr.eu/docs/latest/" \
      org.opencontainers.image.source="https://github.com/rubentalstra/FerroEHR" \
      org.opencontainers.image.vendor="FerroEHR" \
      org.opencontainers.image.authors="Ruben Talstra <https://github.com/rubentalstra>" \
      org.opencontainers.image.licenses="MIT AND PostgreSQL" \
      org.opencontainers.image.base.name="postgres:18.6" \
      org.opencontainers.image.base.digest="sha256:06cad38a5d9f5d24b4d83d86def30795d5e4b757fedbf5281172b576dedcd941"
