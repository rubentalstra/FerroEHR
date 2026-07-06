# Container images — application + preconfigured PostgreSQL (GHCR)

- **Status:** design → implementing (2026-07-06)
- **Prior art:** official EHRbase ships `ehrbase/ehrbase` (the server) and
  `ehrbase/ehrbase-v2-postgres` (a PostgreSQL image with roles/schemas/
  extensions pre-created so the app user needs no superuser). We ship the same
  two-image model, designed fresh for this stack (Rust binary, sqlx migrations,
  PG 18).

## The two images

| Image | GHCR name | Contents |
|---|---|---|
| Application | `ghcr.io/rubentalstra/ehrbase-rs` | the `ehrbase` binary (axum server), config via `EHRBASE_*` env |
| Database | `ghcr.io/rubentalstra/ehrbase-rs-postgres` | `postgres:18.4` + init scripts: roles, database, schemas, required extensions |

## 1. Application image (`docker/Dockerfile`)

**Multi-stage, multi-arch (amd64 + arm64):**

1. **Planner/builder** — `rust:1.96-slim-bookworm` (must match
   `rust-toolchain.toml`; the Dockerfile ARG-pins it and CI cross-checks so the
   two never drift) with **`cargo-chef`**: `chef prepare` → `chef cook`
   (dependency layer cached independently of source changes) → `cargo build
   --release -p ehrbase`. `mold` not needed in release CI; `--locked` builds
   (Cargo.lock is currently gitignored — **decision: commit Cargo.lock**; it is
   a binary deliverable, reproducibility wins and the workspace ships a server,
   not a library).
2. **Runtime** — `gcr.io/distroless/cc-debian12:nonroot`. The binary is pure
   Rust (rustls, no OpenSSL), so distroless-cc suffices (needs only libc/CA
   certs, both present). Runs as `nonroot`; `EXPOSE 8080`;
   `ENTRYPOINT ["/usr/local/bin/ehrbase"]`.
3. **Healthcheck without a shell:** the binary gains a
   `ehrbase healthcheck [--url …]` subcommand (clap; hits `/rest/status`,
   exit 0/1) so `HEALTHCHECK CMD ["/usr/local/bin/ehrbase","healthcheck"]`
   works in distroless. Same subcommand serves compose/K8s probes.
4. **Config = env only** (figment already maps `EHRBASE_*`); document the
   minimum set: `EHRBASE_DB_URL` (or discrete host/port/user/pass vars — follow
   the existing `ehrbase::db::Settings` figment keys), auth mode, bind address.
   Migrations run at boot (existing `run_migrations`), so first start against a
   fresh DB self-provisions the schema objects it owns.
5. **OCI labels** (`org.opencontainers.image.*`: source, revision, version,
   licenses, description) stamped in CI.

## 2. PostgreSQL image (`docker/postgres/`)

`FROM postgres:18.4` + `/docker-entrypoint-initdb.d/` init script(s), mirroring
what our testcontainers bootstrap does today (**read
`crates/ehrbase/src/db/` — the image must create exactly what
`run_migrations`'s bootstrap expects to already exist or be creatable**):

- role `ehrbase` (login, password via `EHRBASE_DB_PASSWORD` env with a
  documented default for dev), database `ehrbase` owned by it;
- schemas `ehr` and `ext` owned by `ehrbase`;
- extensions installed **by superuser** (the whole point of the image — the
  app role never needs superuser): `uuid-ossp`, `pgcrypto`, `pg_trgm`,
  `btree_gist` (temporal `WITHOUT OVERLAPS` needs it);
- sane defaults tuned for a CDR appended to `postgresql.conf` conservatively
  (`shared_buffers`/`work_mem` left to the operator; only what correctness
  needs is set).

**Not pre-migrated:** the app's sqlx migrators own the schema content and run
idempotently at boot (`_sqlx_migrations` per schema). Baking migration state
into the image would couple the two images' release cycles for zero gain —
the EHRbase precedent (init-scripts only) is also the correct call here.
The image README says exactly this.

## 3. CI/CD (`.github/workflows/containers.yml`)

- Triggers: push to `develop` (tags `develop`, `sha-<short>`), version tags
  `v*` (tags `latest`, `X.Y.Z`, `X.Y`), manual dispatch.
- Jobs: `docker/build-push-action` + `buildx` QEMU matrix (linux/amd64,
  linux/arm64), GHCR login via `GITHUB_TOKEN` (`packages: write`), layer cache
  `type=gha`.
- The app image build runs the test suite? **No** — tests run in the existing
  CI; the container workflow only builds what a green commit produced
  (`needs:`-gate it on the test workflow where trigger context allows).
- Metadata via `docker/metadata-action`; SBOM + provenance attestations on
  (`provenance: true`, `sbom: true`) — free with buildx.
- Postgres image builds on changes under `docker/postgres/**` + the same tag
  events, same multi-arch matrix.

## 4. Quickstart compose (`docker-compose.yml`, repo root)

Two services: `ehrbase-postgres` (the DB image, volume, healthcheck
`pg_isready`) and `ehrbase` (the app image, `depends_on:
condition: service_healthy`, healthcheck = the new subcommand, port 8080).
`docs/` gets a short "Run with Docker" section in the README. Local dev builds
use `docker compose build` against the same Dockerfiles CI publishes — one
definition, no drift.

## 5. Testing the images

- CI smoke job after build (amd64 only): start both containers
  (`docker compose up`), wait healthy, then: `/rest/status` 200; create an
  EHR via curl (Basic auth default dev creds); restart the app container and
  confirm the second boot is a migration no-op (idempotency proof); teardown.
- The postgres image's init script is additionally covered by pointing the
  existing testcontainers suite at it (follow-up, noted not blocking).

## 6. Decisions (binding)

1. **Commit `Cargo.lock`** (remove from `.gitignore`) — reproducible binary
   builds; required for `--locked`.
2. Distroless-cc runtime + `healthcheck` subcommand (no shell/curl in the
   final image).
3. The postgres image is **init-scripts only** (roles/schemas/extensions),
   never pre-baked migration state.
4. Image names: `ehrbase-rs` and `ehrbase-rs-postgres` under
   `ghcr.io/rubentalstra`.
5. Toolchain pin single-sourced from `rust-toolchain.toml` (ARG + CI check).
