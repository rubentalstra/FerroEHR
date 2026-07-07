# Container images — application + preconfigured PostgreSQL (GHCR)

- **Status:** implemented (2026-07-06); **CI build architecture rewritten for
  speed (2026-07-07)** — §3 describes the current native-runner pipeline (the
  original QEMU + cargo-chef-in-Docker pipeline took ~50 min per push and was
  replaced wholesale).
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

## 1. Application image

Two Dockerfiles, one runtime shape:

- **`docker/Dockerfile`** — the from-source path (local dev + the compose
  quickstart). `rust:1.96.1-slim-bookworm` builder (ARG-pinned to
  `rust-toolchain.toml`; CI cross-checks so the two never drift), a **single
  `cargo build --release --locked -p ehrbase`** with BuildKit cache mounts for
  the registry + target dir (cargo-chef was dropped: it compiled itself and
  the dependency graph a second time for no gain over cache mounts), then the
  distroless runtime stage.
- **`docker/Dockerfile.runtime`** — the CI packaging path: **COPY-only** (no
  RUN), expects prebuilt native binaries at `bin/{amd64,arm64}/ehrbase` in the
  context. A multi-arch buildx of it executes no foreign-arch code, so it
  needs no QEMU and finishes in seconds.

Runtime (both files):

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

## 3. CI/CD (`.github/workflows/containers.yml`) — native-runner pipeline (2026-07-07)

Designed around one fact: compiling the Rust workspace under QEMU arm64
emulation is 10–20× slower than native, and the original pipeline did exactly
that (twice: cargo-chef cook + build), then rebuilt the image a third time in
the smoke job. The rewrite:

1. **`binary (amd64|arm64)`** — matrix over native runners (`ubuntu-24.04`,
   `ubuntu-24.04-arm`; free for public repos), each inside a
   `rust:1.96.1-slim-bookworm` **job container** so the binary links Debian 12
   glibc 2.36 — exactly the distroless runtime's libc (building on the Ubuntu
   24.04 host would link glibc 2.39 and not run on the runtime image).
   `Swatinem/rust-cache` (per-platform shared key) makes warm builds
   incremental; a verify step asserts container rustc == `rust-toolchain.toml`
   == the Dockerfile ARG. Binaries are stripped and uploaded as artifacts.
2. **`app image`** — downloads both artifacts, one COPY-only multi-arch
   buildx push of `docker/Dockerfile.runtime` (seconds; no QEMU, no gha layer
   cache needed). Metadata via `docker/metadata-action`; `provenance: true`,
   `sbom: true`.
3. **`postgres image`** — unchanged trigger logic (changes under
   `docker/postgres/**` or tags), but QEMU dropped: the Dockerfile is
   COPY-only on `postgres:18.4`, so multi-arch needs no emulation.
4. **`smoke`** — **pulls the pushed app image by digest** (no rebuild) and
   does a plain amd64 `docker build` of the postgres image (seconds), then
   runs `docker/smoke-test.sh`.

Triggers unchanged: push to `develop` (tags `develop`, `sha-<short>`), version
tags `v*` (`latest`, `X.Y.Z`, `X.Y`), manual dispatch. Tests still live in the
main CI workflow, not here.

Expected wall clock: warm ≈ 4–8 min (binary compile dominates), cold ≈ 15–20
min — vs ~50 min+ for the QEMU pipeline. The two arch builds run in parallel,
so wall clock ≈ the slower native compile + ~1 min packaging + smoke.

## 4. Quickstart compose (`docker-compose.yml`, repo root)

Two services: `ehrbase-postgres` (the DB image, volume, healthcheck
`pg_isready`) and `ehrbase` (the app image, `depends_on:
condition: service_healthy`, healthcheck = the new subcommand, port 8080).
**PG 18 volume convention:** the data volume mounts at `/var/lib/postgresql`
(the parent), NOT the pre-18 `/var/lib/postgresql/data` — the 18+ official
image stores data in a major-version subdirectory to support
`pg_upgrade --link` (docker-library/postgres#1259) and refuses to start with
the old mount point.
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
