# ferroehr-postgres

A preconfigured PostgreSQL 18 image for FerroEHR, mirroring the official
`ehrbase/ehrbase-v2-postgres` two-image model built fresh for this stack
(the greenfield PG18 storage design, `docs/architecture.md` §Storage). It is `postgres:18.6` plus Debian
security updates applied at image build (the pinned upstream base rebuilds on its own cadence, so
trixie-security fixes are pulled in at OUR build time) plus one-time init scripts.

## What the init scripts create

Run once, on an empty data directory, as the bootstrap superuser
(`POSTGRES_USER`) — see `initdb/10-ferroehr-init.sh`:

- a **non-superuser** login role (default `ferroehr`) and a **database it owns**
  (default `ferroehr`);
- schemas **`ehr`** and **`ext`**, both owned by the app role;
- extensions installed **by the superuser** so the app role never needs one:
  `uuid-ossp`, `pgcrypto`, `pg_trgm`, and **`btree_gist`** (required by the
  temporal `vo_version` `PRIMARY KEY (... WITHOUT OVERLAPS)`), all in `ext`.

This is exactly the set the application's migrator
(`ferroehr::db::run_migrations`) expects to find when it connects as the app
role: its bootstrap `CREATE SCHEMA IF NOT EXISTS {ehr,ext}` and
`CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA ext` all become no-ops,
and it then migrates the schema **content** into `ehr`/`ext`.

## Init-scripts only — NO baked migration state (policy)

The image ships **roles, schemas, and extensions only**. It never bakes the
migrated schema (tables, functions, `_sqlx_migrations`) into the image.

Why: the app's sqlx migrators own the schema content and run idempotently at
every boot (a per-schema `_sqlx_migrations` ledger makes re-runs no-ops).
Baking migration state into the image would couple the two images' release
cycles for zero gain and risk a checksum mismatch between a stale baked schema
and the running binary's embedded migrations. The single source of truth for
schema content is `app/ferroehr/migrations/{ext,ehr}/`, applied at boot. This
is also the official EHRbase precedent (its postgres image is init-scripts
only).

## Configuration

| Env var | Default | Meaning |
|---|---|---|
| `POSTGRES_PASSWORD` | *(required by the base image)* | bootstrap **superuser** (`postgres`) password |
| `PG_INIT_USER` | `ferroehr` | app login role created by the init script |
| `PG_INIT_PASSWORD` | `ferroehr` | app role password (**dev default — override in production**) |
| `PG_INIT_DB` | `ferroehr` | app database created by the init script |

These `PG_INIT_*` vars configure this DB container; they are intentionally
outside the server's reserved `FERROEHR_` namespace (the server rejects unknown
`FERROEHR_*` vars at boot). The app then connects with, e.g.,
`FERROEHR__DB__URL=postgres://ferroehr:ferroehr@<host>:5432/ferroehr`.

Init scripts only run on first initialisation (empty volume). To re-provision,
remove the data volume (`docker compose down -v`).
