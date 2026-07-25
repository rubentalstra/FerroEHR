# Docker Compose

Docker Compose is the quickest way to run EHRbase-rs together with a
preconfigured PostgreSQL 18, for local development and evaluation. This chapter
describes the two published images, the Compose services, the environment
variables that tune them, and the optional observability overlay. For a
step-by-step first run, see [Getting started](../getting-started.md).

## The two images

EHRbase-rs publishes two container images to GHCR:

| Image | Contents |
|---|---|
| `ghcr.io/rubentalstra/ehrbase-rs` | The `ehrbase` server binary. A distroless, non-root, shell-less multi-arch image (amd64 + arm64). Configured entirely via `EHRBASE_*` environment variables. |
| `ghcr.io/rubentalstra/ehrbase-rs-postgres` | `postgres:18.4` with the application role, the layered group roles (`ehrbase_migrator`, `ehrbase_app`, `ehrbase_reader`), database, schemas (`ehr`, `ext`), and required extensions (`uuid-ossp`, `pgcrypto`, `pg_trgm`, `btree_gist`) pre-created, so the app role never needs superuser. |

The PostgreSQL image is **init-scripts only** — it creates roles, schemas, and
extensions, but does not bake in migration state. The server owns the schema
content and applies its migrations idempotently at every boot, so a fresh
database self-provisions and a restart is a no-op.

> [!NOTE]
> PostgreSQL init scripts run **only when the data volume is empty**. If you
> see startup notices like `skipping role creation (no CREATEROLE privilege)`
> or `roles absent`, your volume predates the image's role setup (or you are
> running a plain `postgres` image): either recreate the volume
> (`docker compose down -v` — **destroys data**) or create the three group
> roles once by hand as a superuser. The server runs fine either way — the
> grants are a defense-in-depth layer, not a functional requirement.

## Bringing up the stack

```shell
docker compose up --build
```

This starts the core services (no profile needed):

- **`ehrbase-postgres`** — the database image, with a named data volume and a
  `pg_isready` healthcheck.
- **`ehrbase`** — the server, which waits for the database to report healthy
  (`depends_on: condition: service_healthy`), then boots, migrates, and serves
  on port 8080. Its healthcheck is the binary's own `healthcheck` subcommand
  (there is no shell in the image).
- **`ehrbase-admin-ui`** — the admin console, which waits for the server to be
  healthy and serves on port 3000.

> [!NOTE]
> All base images are pinned by digest (`name:tag@sha256:…`), not by a mutable
> tag, so a rebuild always resolves the exact same base. Each service also
> declares a memory/CPU limit (`deploy.resources.limits`) mirroring the Helm
> chart, so a local stack cannot exhaust the host.

The server's development configuration is mounted read-only from
`docker/ehrbase.dev.toml` to `/etc/ehrbase/ehrbase.toml`, where the server
auto-discovers it. That file enables Basic auth with the throwaway users
`ehrbase` / `ehrbase` and `ehrbase-admin` / `ehrbase`, and turns on permissive
CORS — development only.

> [!WARNING]
> PostgreSQL 18's official image stores data in a major-version subdirectory,
> so the data volume mounts at `/var/lib/postgresql` (the parent), **not** the
> pre-18 `/var/lib/postgresql/data`. The bundled Compose file already does this
> correctly; keep the convention if you adapt it.

## Environment variables

The Compose file reads these host environment variables (with the defaults
shown), so you can retune without editing it:

| Variable | Default | Effect |
|---|---|---|
| `EHRBASE_IMAGE` | `ghcr.io/rubentalstra/ehrbase-rs:local` | Server image to run (set to a published tag to skip the build). |
| `EHRBASE_POSTGRES_IMAGE` | `ghcr.io/rubentalstra/ehrbase-rs-postgres:local` | Database image to run. |
| `EHRBASE_PORT` | `8080` | Host port mapped to the server. |
| `EHRBASE_DB_PORT` | `5432` | Host port mapped to PostgreSQL. |
| `PG_INIT_USER` / `PG_INIT_PASSWORD` / `PG_INIT_DB` | `ehrbase` | App role, password, and database created by the DB image's init script. |
| `POSTGRES_PASSWORD` | `postgres` | Bootstrap superuser password (init only). |
| `EHRBASE__LOG__FORMAT` | `pretty` | Log rendering for `docker compose logs`. Set `json` for log collectors. |

The server container itself is passed `EHRBASE__DB__URL` (assembled from the
DB variables); its `ehrbase.toml` is the mounted `docker/ehrbase.dev.toml`,
auto-discovered at `/etc/ehrbase/ehrbase.toml`. Any other `EHRBASE_*` setting
from the [configuration reference](configuration.md) can be added under the
`ehrbase` service's `environment:` block.

## Optional services (Compose profiles)

The Compose file also defines two services the quickstart does **not** depend
on — the server defaults to Basic auth and inline multimedia, so neither is
required. Each sits behind a [Compose
profile](https://docs.docker.com/compose/how-tos/profiles/) and stays down
until you enable it:

- **`seaweedfs`** (`--profile s3`) — an S3 gateway for large `DV_MULTIMEDIA`
  externalization (development/test only). Start it with
  `docker compose --profile s3 up`, then set `EHRBASE__MULTIMEDIA__ENABLED=true`,
  `EHRBASE__MULTIMEDIA__ENDPOINT=http://seaweedfs:8333`,
  `EHRBASE__MULTIMEDIA__BUCKET=openehr-multimedia`, and (dev only)
  `EHRBASE__MULTIMEDIA__ALLOW_HTTP=true`. In production, point the multimedia
  settings at a real, credentialed, HTTPS S3 endpoint instead.
- **`keycloak`** (`--profile keycloak`) — an OIDC provider with a preloaded
  `ehrbase` realm, on port 8081. Start it with
  `docker compose --profile keycloak up`; to use bearer auth instead of Basic,
  point the auth OIDC settings at `http://localhost:8081/auth/realms/ehrbase`.
  Its healthcheck probes the realm's OIDC discovery document, so services can
  gate their startup on it being fully ready.

## Build provenance

Images built by CI (and any `docker compose build` you drive) embed a build
SHA reported at `/management/info` and on the `ehrbase_build_info` metric. The
build does not read `.git`; instead the SHA is passed as the standard
`REVISION` build argument — the same value that fills the
`org.opencontainers.image.revision` image label (CI uses the commit SHA; the
project's own scripts export `git rev-parse --short=12 HEAD`). When no value is
supplied the identity falls back to the workspace version with an `unknown`
SHA — the build never fails for lack of it.

## Observability overlay

A second Compose file adds a full local telemetry stack — an OTLP collector,
Prometheus, Tempo, Loki, and Grafana with a provisioned service-overview
dashboard:

```shell
docker compose -f docker-compose.yml -f docker-compose.observability.yml up --build
# Grafana → http://localhost:3000
```

The overlay reconfigures the server for that stack: it exports traces over
OTLP/gRPC (`EHRBASE__TELEMETRY__OTLP_ENDPOINT`), switches stdout to JSON lines
(`EHRBASE__LOG__FORMAT=json`), and enables the management surface on its own
internal port 9464 (`EHRBASE__MANAGEMENT__ENABLED`, `EHRBASE__MANAGEMENT__PORT`)
with `info`, `metrics`, and `prometheus` set to `public` so the bundled
Prometheus can scrape `/management/prometheus` without credentials. That port is
only reachable on the Compose network — Grafana's 3000 is the sole published
port. Every variable uses the same `EHRBASE__…` grammar as the rest of the
[configuration reference](configuration.md); a single-underscore spelling is
rejected at startup, not ignored.

This is the easiest way to see the server's metrics and traces without wiring
up a collector by hand. See [Operations](../operations.md) for what the server
exports and how to consume it in production.

## Next

- [Configuration reference](configuration.md) — every setting you can pass.
- [Kubernetes & Helm](kubernetes.md) — the production deployment.
