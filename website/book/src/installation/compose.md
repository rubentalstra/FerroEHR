# Docker Compose

Docker Compose is the quickest way to run FerroEHR together with a
preconfigured PostgreSQL 18, for local development and evaluation. The
quickstart is **one file and zero configuration**: download
`docker-compose.yml`, run `docker compose up`, and the published images are
pulled and started: no repository checkout, no bind mounts, no environment
variables. This chapter describes the three published images, the services, the
authentication posture the quickstart ships with, the optional profiles and
overlays, and the variables that tune them. For a step-by-step first run, see
[Getting started](../getting-started.md).

> [!NOTE]
> This chapter applies equally to the **standalone** `docker-compose.yml`
> downloaded into an empty directory and to a bare `docker compose up` in a
> repository checkout — both run the published images. Building from source
> is an explicit opt-in (`-f docker-compose.yml -f docker-compose.dev.yml`) —
> see [Repository development](#repository-development) below.

<!-- toc -->

> [!NOTE]
> The quickstart file carries the server configuration **inline** (a Compose
> `configs` entry with `content:`), which requires **Docker Compose 2.23.1 or
> newer**. Check with `docker compose version`.

## The three images

FerroEHR publishes three container images to GHCR:

| Image | Contents |
|---|---|
| `ghcr.io/rubentalstra/ferroehr` | The `ferroehr` server binary on a distroless, non-root, shell-less multi-arch base (amd64 + arm64). Configured by a mounted TOML file and/or `FERROEHR__*` environment variables. |
| `ghcr.io/rubentalstra/ferroehr-postgres` | `postgres:18.6` (with Debian security updates applied at image build) plus init scripts that pre-create the application login role, the three group roles (`ferroehr_migrator`, `ferroehr_app`, `ferroehr_reader`), the database, the schemas (`ehr`, `ext`, `audit`) and the extensions (`uuid-ossp`, `pgcrypto`, `pg_trgm`, `btree_gist`), so the app role never needs superuser. |
| `ghcr.io/rubentalstra/ferroehr-admin-ui` | The [admin console](../admin-ui/index.md), a standalone web application that talks to the CDR strictly over ITS-REST. Optional; see the `admin-ui` profile below. |

Each image is published under several tags:

| Tag | Published from |
|---|---|
| `X.Y.Z`, `X.Y` | every release |
| `latest` | the newest release |
| `develop` | every push to the development branch |
| `sha-<commit>` | every push, for exact pinning |

The quickstart Compose file pins the **exact release version it shipped with**
on all three images, so a downloaded file always runs one known-good, mutually
compatible set; a CI guard (`scripts/checks/compose-image-tags.sh`) fails the
build whenever those pinned tags disagree with the version being released, so
they cannot be forgotten at a cut. To run something else, set the image
variables in [the table below](#variables-the-compose-files-read).

The role the server connects as in the quickstart owns the database and belongs
to **both** `ferroehr_migrator` and `ferroehr_app`, which is what lets it apply
migrations at boot. A least-privilege deployment instead sets
`db.migrate = "verify"`, connects as `ferroehr_app` alone, and runs
`ferroehr db migrate` out of band under the migrator DSN; see
[Operations → Applying migrations](../operations.md#applying-migrations).

The PostgreSQL image is **init-scripts only**: it creates roles, schemas and
extensions, and bakes in no migration state. The server owns the schema content
and applies its migrations idempotently at every boot, so a fresh database
self-provisions and a restart is a no-op.

> [!NOTE]
> PostgreSQL init scripts run **only when the data volume is empty**. If you
> see startup notices like `skipping role creation (no CREATEROLE privilege)`
> or `roles absent`, your volume predates the image's role setup (or you are
> running a plain `postgres` image): either recreate the volume
> (`docker compose down -v`, which **destroys data**) or create the three group
> roles once by hand as a superuser. The server runs fine either way: the
> grants are a defense-in-depth layer, not a functional requirement.

## Bringing up the stack

Download `docker-compose.yml` (attached to every
[release](https://github.com/rubentalstra/FerroEHR/releases/latest)) into an
empty directory and start it:

```shell
docker compose up
```

This pulls and starts the two core services (no profile needed):

- **`ferroehr-postgres`:** the database image, with a named data volume, a
  `pg_isready` healthcheck, `pg_stat_statements` preloaded, and a modest tuning
  floor (shared buffers, WAL size, work memory, lz4-compressed WAL full-page
  images — which cut per-commit WAL volume roughly in half — and a larger
  `/dev/shm` than Docker's default, which otherwise starves parallel workers).
- **`ferroehr`:** the server, which waits for the database to report healthy
  (`depends_on: condition: service_healthy`), then boots, migrates, and serves
  on port 8080. Its healthcheck is the binary's own `healthcheck` subcommand,
  because the image has no shell.

The API is then at `http://localhost:8080/ferroehr/rest/openehr/v1`, with
Swagger UI at `http://localhost:8080/ferroehr/rest/swagger-ui`.

> [!IMPORTANT]
> **Every published port binds the loopback interface by default**
> (`127.0.0.1:HOST:CONTAINER`), and that is a security property rather than a
> style choice: a published port is DNAT'd by rules Docker inserts *ahead* of
> the host firewall's own chains, so a `ufw deny 5432` does not stop a port
> published on `0.0.0.0`. To reach the stack from another machine, name the
> interface explicitly (`FERROEHR_BIND_HOST=0.0.0.0 docker compose up`, or a
> single address) and prefer a reverse proxy on the host over publishing the
> database at all.

> [!NOTE]
> Third-party base images are pinned by digest (`name:tag@sha256:…`), not by a
> mutable tag, so a pull always resolves the exact same image. Each service also
> declares a CPU and memory ceiling (`deploy.resources.limits`), so a local
> stack cannot exhaust the host; the server's and the database's are overridable
> for constrained machines.

> [!WARNING]
> PostgreSQL 18's official image stores data in a major-version subdirectory,
> so the data volume mounts at `/var/lib/postgresql` (the parent), **not** the
> pre-18 `/var/lib/postgresql/data`. The bundled Compose file already does this
> correctly; keep the convention if you adapt it.

## Container engines: Docker and Podman

The quickstart runs on Docker and on Podman. Both were verified first-hand on
2026-08-24: the core stack, the `admin-ui` and `s3` profiles, and both
overlays boot to healthy under `podman compose` exactly as under
`docker compose`, with the same commands:

```shell
podman compose up -d --wait
```

Podman-specific notes:

- `podman compose` delegates to an external Compose provider. With Docker's
  own Compose CLI installed (the common case on a machine that has ever run
  Docker Desktop) you get identical Compose behaviour; with the Python
  `podman-compose` package, flag coverage differs — prefer the Docker Compose
  provider.
- The default `podman machine` on macOS ships with 2048 MiB of memory. That
  is enough for the quickstart and both overlays, but not for building the
  server image from source inside the VM (a single spec-crate compile at
  release optimization holds more than that). If you want the
  build-from-source developer path under Podman, resize the machine first:
  `podman machine set --memory 8192`.
- Building from source under Podman (Buildah) is not part of the verified
  quickstart surface; the supported quickstart path is the published images.

The repository's own CI and measurement lanes run Docker; the compose-driven
scripts (`scripts/conformance.sh`, the deployment probe, the UI journey
battery) are exercised against Docker only.

## The isolation posture

Every service in every committed compose file drops **all** Linux capabilities
and adds back only what its entrypoint provably needs, sets
`no-new-privileges:true`, bounds its file descriptors, restarts unless stopped,
and keeps Docker's default seccomp profile (the posture is the *absence* of an
`unconfined` override, so that absence is what a CI guard checks). No service is
privileged and none mounts the Docker daemon socket.

Most services additionally run a read-only root filesystem with a tmpfs for the
paths that must be writable. Two do not, and each says why in place: the S3
gateway writes its volume store, and the server container hits a real mutual
exclusion rather than an oversight: Compose refuses an inline `config` in a
read-only service, and that inline config is what makes this file standalone. A
deployment that mounts its configuration from a file instead can add
`read_only: true` and a tmpfs at `/tmp`. On Kubernetes the conflict does not
arise: the configuration arrives as a mounted ConfigMap and the chart sets
`readOnlyRootFilesystem: true`.

## The quickstart's authentication posture

The server configuration travels inside the Compose file and is written for
evaluation, not for production:

- **Basic auth with one user:** `ferroehr` / `ferroehr` (stored as an Argon2id
  hash), carrying the `ADMIN` and `USER` roles.
- **RBAC disabled:** so any authenticated caller may use every enabled
  surface, including the admin API. The `ADMIN` / `USER` / `READONLY`
  separation is switched on by setting `[authz.rbac] enabled = true` and giving
  each user an explicit `roles` list; see
  [Security & multi-tenancy](../security.md).
- **Admin API enabled:** so the optional admin console's panels work.
- **Management introspection enabled**, with `info` and `metrics` at `private`,
  `prometheus` at `public`, and `env` and `loggers` at `admin_only`.
- **Permissive CORS:** any origin may call the API from a browser.
- **No TLS:** plain HTTP on port 8080.

> [!WARNING]
> These are development credentials and development defaults. Before exposing
> a server, replace the user store (or point it at an identity provider), turn
> RBAC on, restrict CORS, close the admin API, and terminate TLS in front of the
> server. The [configuration reference](configuration.md) and
> [Security & multi-tenancy](../security.md) cover each of these.

To change any of it without editing the Compose file, add `FERROEHR__*`
variables to the `ferroehr` service's `environment:` block; the environment
layer wins over the inline file. The Basic-auth user store is the one setting
the environment cannot express (it is an array of tables), so replacing the
users means editing the inline config or mounting your own TOML file.

## The OIDC variant (Keycloak overlay)

A second standalone file adds a ready-made identity provider, so you can
exercise bearer-token authentication without registering a client anywhere.
Download `docker-compose.keycloak.yml` from the same release, beside the base
file, and stack the two:

```shell
docker compose -f docker-compose.yml -f docker-compose.keycloak.yml up
```

That starts **Keycloak** on port 8081 with a small demo realm (`ferroehr`)
defined inline and points the server's bearer validation at it. Basic auth from
the base file keeps working, so the server then advertises `Basic, Bearer`.

The overlay also **turns RBAC on**, deliberately: an identity provider whose
roles decide nothing is a login screen, not authorization. So the realm ships
four users whose roles are load-bearing:

| User | Password | Realm roles | What it demonstrates |
|---|---|---|---|
| `ferroehr` | `ferroehr` | `ADMIN`, `USER` | the full surface, admin API included |
| `clinician` | `clinician` | `USER` | the clinical API without the admin routes |
| `auditor` | `auditor` | `USER`, `READONLY` | reads succeed, every write is refused `403` |
| `nobody` | `nobody` | none | an authenticated caller with no grant |

One confidential client is defined (`ferroehr` /
`ferroehr-quickstart-secret`, with the password grant enabled so a token can be
fetched by curl). Fetch a token and call the API with it:

```shell
TOKEN=$(curl -s -d client_id=ferroehr -d client_secret=ferroehr-quickstart-secret \
  -d username=ferroehr -d password=ferroehr -d grant_type=password \
  http://localhost:8081/auth/realms/ferroehr/protocol/openid-connect/token \
  | jq -r .access_token)

curl -H "Authorization: Bearer $TOKEN" -X POST -i \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr
```

> [!WARNING]
> The realm, the client secret and the user passwords are demo values, served
> over plain HTTP by a Keycloak in `start-dev` mode, which is also why the
> overlay sets the server's `allow_insecure_issuer` opt-in. For a real
> deployment, drop this overlay and point `[auth.oidc]` (or
> `FERROEHR__AUTH__OIDC__*`) at your own issuer over HTTPS.

## Optional services (Compose profiles)

The core services are profile-less and start on every `up`. Further services
sit behind a [Compose
profile](https://docs.docker.com/compose/how-tos/profiles/) and stay down until
you ask for them:

- **`ferroehr-admin-ui`** (`--profile admin-ui`): the
  [admin console](../admin-ui/index.md) on port 3000, pointed at the server
  inside the Compose network. Start the stack with it:

  ```shell
  docker compose --profile admin-ui up
  # → http://localhost:3000  (log in with ferroehr / ferroehr)
  ```

- **`seaweedfs`** and **`seaweedfs-init`** (`--profile s3`): an S3 gateway for
  large `DV_MULTIMEDIA` externalization (development and test only). Point the
  server at it and bring the profile up:

  ```shell
  export FERROEHR__MULTIMEDIA__ENABLED=true
  export FERROEHR__MULTIMEDIA__ENDPOINT=http://seaweedfs:8333
  export FERROEHR__MULTIMEDIA__BUCKET=openehr-multimedia
  export FERROEHR__MULTIMEDIA__ALLOW_HTTP=true    # dev only; production S3 is HTTPS

  docker compose --profile s3 up -d --wait ferroehr seaweedfs
  ```

  The compose file passes the whole `FERROEHR__MULTIMEDIA__*` set through from
  your shell, so there is no file to edit, and `seaweedfs-init` creates the
  bucket by issuing an idempotent `CreateBucket` against the gateway, which
  ships with none, and answers an S3 write into a missing bucket with
  `403 AccessDenied`, a reply that reads as a credentials problem and is not
  one. It reads the same bucket variable the server does, so the two cannot
  drift.

  To turn it off again, `unset` those variables (or set
  `FERROEHR__MULTIMEDIA__ENABLED=false`) and re-up: an unset variable is
  *removed* from the container's environment rather than passed as empty, so
  the server falls back to its own default of `enabled = false`.

  Confirm the server took them with
  `curl -s -u ferroehr:ferroehr http://localhost:8080/management/env | jq .multimedia`
  (`"enabled": true` and a non-empty `endpoint` mean the wiring is right). In
  production, point the multimedia settings at a real, credentialed, HTTPS S3
  endpoint instead; see [S3 multimedia](../beyond-core/s3-multimedia.md).

Two things that used to be profiles of this file are not any more. Both have to
*change* the server's configuration to be useful, which a profile cannot do, so
each is a separate file instead:

- **OIDC / Keycloak** is the `docker-compose.keycloak.yml` overlay above (and,
  in a repository checkout, a `keycloak` profile of the development override
  described below).
- **A real FHIR terminology server** now lives in the repository's
  self-contained conformance stack; see
  [Terminology servers](../beyond-core/terminology.md) for the invocation and
  what is seeded.

## Port already in use?

Every published port is a variable, so a conflict never requires editing any
file. When `docker compose up` refuses with `port is already allocated` (or
`address already in use`), pick a free port and pass it:

```shell
FERROEHR_PORT=8081 docker compose up
```

The same works for the other ports (`FERROEHR_DB_PORT`, `FERROEHR_ADMIN_UI_PORT`,
`FERROEHR_S3_PORT`). The defaults stay fixed on purpose: every URL in this
book assumes `localhost:8080`, and Docker's automatic ephemeral-port
allocation exists only when a mapping
[omits the host port entirely](https://docs.docker.com/engine/network/port-publishing/)
— a server that silently moved ports would break every printed URL, so here
you always choose the port and always know it.

## Variables the compose files read

Set these in your shell (or an `.env` file) to retune without editing anything:

| Variable | Default | Effect |
|---|---|---|
| `FERROEHR_IMAGE` | `ghcr.io/rubentalstra/ferroehr:<release>` | Server image to run. |
| `FERROEHR_POSTGRES_IMAGE` | `ghcr.io/rubentalstra/ferroehr-postgres:<release>` | Database image to run. |
| `FERROEHR_ADMIN_UI_IMAGE` | `ghcr.io/rubentalstra/ferroehr-admin-ui:<release>` | Admin console image (the `admin-ui` profile). |
| `FERROEHR_BIND_HOST` | `127.0.0.1` | Host interface every published port binds. |
| `FERROEHR_PORT` | `8080` | Host port mapped to the server. |
| `FERROEHR_ADMIN_UI_PORT` | `3000` | Host port mapped to the admin console. |
| `FERROEHR_DB_PORT` | `5432` | Host port mapped to PostgreSQL. |
| `FERROEHR_S3_PORT` | `8333` | Host port mapped to the S3 gateway (the `s3` profile). |
| `FERROEHR_CPUS` / `FERROEHR_MEM` | `4` / `4G` | Server container resource ceiling. |
| `FERROEHR_DB_CPUS` / `FERROEHR_DB_MEM` | `4` / `4G` | Database container resource ceiling. |
| `PG_INIT_USER` / `PG_INIT_PASSWORD` / `PG_INIT_DB` | `ferroehr` | App role, password, and database created by the DB image's init script. |
| `POSTGRES_PASSWORD` | `postgres` | Bootstrap superuser password (init only). |
| `FERROEHR__LOG__FORMAT` | `pretty` | Log rendering for `docker compose logs`. Set `json` for log collectors. |
| `FERROEHR__LOG__FILTER` | `info` | Log level filter. |
| `FERROEHR__DB__MAX_CONNECTIONS` | `10` | Server connection-pool ceiling. |
| `FERROEHR__SERVER__MAX_IN_FLIGHT` | `256` | In-flight request admission cap (`503` past it; `0` disables). |
| `FERROEHR__SIGNING__ENABLED` | `true` | Version signing. |

The Keycloak overlay adds `KEYCLOAK_PORT` (default `8081`),
`KEYCLOAK_HOSTNAME`, `KEYCLOAK_ADMIN_USER` and `KEYCLOAK_ADMIN_PASSWORD` (both
`admin`); the observability overlay adds `GRAFANA_PORT` (default `3000`). The
`FERROEHR__MULTIMEDIA__*` set is declared with no defaults on purpose, so an
unset key reaches the server as absent rather than as an empty string.

The server container is passed `FERROEHR__DB__URL` (assembled from the database
variables), and its configuration file is the inline quickstart config,
delivered to `/etc/ferroehr/ferroehr.toml` where the server auto-discovers it.
Any other setting from the [configuration reference](configuration.md) can be
added under the `ferroehr` service's `environment:` block and takes precedence
over the file. Every variable uses the same `FERROEHR__…` grammar as the rest of
that reference; a single-underscore spelling is rejected at startup, not
ignored.

## Repository development

A bare `docker compose up` in a checkout runs the same published-images
quickstart as the downloaded file. Building from the current sources is an
explicit opt-in: the `docker-compose.dev.yml` overlay, passed as a
[second file](https://docs.docker.com/compose/how-tos/multiple-compose-files/),
switches the stack to the from-source developer posture:

```shell
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build
```

That builds the server, database and console images from the current sources
(the `:local` tags) instead of pulling published ones, and replaces the inline
quickstart configuration with `docker/ferroehr.dev.toml`: three Basic users
(`ferroehr`, `ferroehr-admin`, `ferroehr-readonly`, all with password
`ferroehr`), **RBAC enabled** so the role separation is exercised, and trust for
the development Keycloak realm. The override also defines a `keycloak` profile
that imports the full development realm
(`docker compose --profile keycloak up`).

Downloaders of the standalone quickstart file never see any of this; it is
purely a convenience for working on FerroEHR itself, and it only ever
applies when its `-f` is passed — nothing merges it in silently.

## Build provenance

Images built by CI (and any `docker compose build` you drive from a checkout)
embed a build SHA reported at `/management/info` and on the
`ferroehr_build_info` metric. The build does not read `.git`; instead the SHA is
passed as the standard `REVISION` build argument, the same value that fills the
`org.opencontainers.image.revision` image label (CI uses the commit SHA; the
project's own scripts export `git rev-parse --short=12 HEAD`). The argument is
declared by the files that carry `build:` blocks (the development override and
the conformance stack) not by the pull-only quickstart file. When no value is
supplied the identity falls back to the workspace version with an `unknown`
SHA; the build never fails for lack of it.

## Observability overlay

A further standalone file adds a full local telemetry stack: an OTLP
collector, Prometheus, Tempo, Loki and Grafana with a provisioned
service-overview dashboard. Like the Keycloak overlay it travels inline, so
downloading it beside `docker-compose.yml` is enough:

```shell
docker compose -f docker-compose.yml -f docker-compose.observability.yml up
# Grafana → http://localhost:3000
```

The overlay reconfigures the server for that stack: it exports **traces and
metrics** over OTLP/gRPC (`FERROEHR__TELEMETRY__OTLP_ENDPOINT` plus
`FERROEHR__TELEMETRY__METRICS_PUSH=true`), switches stdout to JSON lines
(`FERROEHR__LOG__FORMAT=json`), and enables the management surface on its own
internal port 9464 with `info`, `metrics` and `prometheus` reachable there. That
port is only published on the Compose network; Grafana's 3000 is the sole
published port.

Metrics are **pushed, not scraped**, and that is a property of the bundled
image rather than a preference: `grafana/otel-lgtm` runs Prometheus with a
config file carrying no `scrape_configs` at all, and receives metrics over OTLP
from its own collector. A scrape job dropped into that image is read by nothing,
which is what an earlier version of this overlay did: every metric panel was
empty with no error anywhere. Since one meter provider feeds both readers, the
push carries every metric family the scrape endpoint would, and
`/management/prometheus` stays reachable on the Compose network for anyone who
wants to compare the two.

Grafana's port 3000 is the same one the admin console would use, but the two
never collide by accident: the console only starts when you ask for the
`admin-ui` profile. If you want both at once, move one of them
(`FERROEHR_ADMIN_UI_PORT=3001` or `GRAFANA_PORT=3001`).

This is the easiest way to see the server's metrics and traces without wiring
up a collector by hand. See [Operations](../operations.md) for what the server
exports and how to consume it in production.

## Next

- [Configuration reference](configuration.md) — every setting you can pass.
- [Kubernetes & Helm](kubernetes.md) — the production deployment.
