# ehrbase-rs-postgres

A preconfigured PostgreSQL 18 image for ehrbase-rs, mirroring the official
`ehrbase/ehrbase-v2-postgres` two-image model built fresh for this stack
(ADR-008). It is `postgres:18.4` plus one-time init scripts.

## What the init scripts create

Run once, on an empty data directory, as the bootstrap superuser
(`POSTGRES_USER`) — see `initdb/10-ehrbase-init.sh`:

- a **non-superuser** login role (default `ehrbase`) and a **database it owns**
  (default `ehrbase`);
- schemas **`ehr`** and **`ext`**, both owned by the app role;
- extensions installed **by the superuser** so the app role never needs one:
  `uuid-ossp`, `pgcrypto`, `pg_trgm`, and **`btree_gist`** (required by the
  temporal `vo_version` `PRIMARY KEY (... WITHOUT OVERLAPS)`), all in `ext`.

This is exactly the set the application's migrator
(`ehrbase::db::run_migrations`) expects to find when it connects as the app
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
schema content is `app/ehrbase/migrations/{ext,ehr}/`, applied at boot. This
is also the official EHRbase precedent (its postgres image is init-scripts
only).

## Configuration

| Env var | Default | Meaning |
|---|---|---|
| `POSTGRES_PASSWORD` | *(required by the base image)* | bootstrap **superuser** (`postgres`) password |
| `EHRBASE_DB_USER` | `ehrbase` | app login role created by the init script |
| `EHRBASE_DB_PASSWORD` | `ehrbase` | app role password (**dev default — override in production**) |
| `EHRBASE_DB_NAME` | `ehrbase` | app database created by the init script |

The app then connects with, e.g.,
`EHRBASE_DB_URL=postgres://ehrbase:ehrbase@<host>:5432/ehrbase`.

Init scripts only run on first initialisation (empty volume). To re-provision,
remove the data volume (`docker compose down -v`).
