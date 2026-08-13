#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# Preconfigure the ferroehr database. Runs once, as the bootstrap superuser
# (POSTGRES_USER), on first initialisation of an empty data directory.
#
# Creates exactly what the app's sqlx migrator (`ferroehr::db::run_migrations`)
# expects to already exist or be a no-op when it connects as the NON-superuser
# app role:
#   * a LOGIN role (non-superuser) and a database it owns;
#   * schemas `ehr` and `ext` owned by that role (the app's
#     `CREATE SCHEMA IF NOT EXISTS` then no-ops);
#   * the extensions the stack needs, installed here by the superuser
#     (`CREATE EXTENSION` on non-trusted extensions requires superuser — the
#     whole reason this image exists). The app's bootstrap
#     `CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA ext` then no-ops.
set -Eeuo pipefail

# PG_INIT_* configure the DB container's app role/database. They are
# deliberately NOT in the server's reserved FERROEHR_ namespace (which the
# server rejects unknown vars from at boot) — these belong to this init script.
APP_USER="${PG_INIT_USER:-ferroehr}"
APP_PASSWORD="${PG_INIT_PASSWORD:-ferroehr}"   # dev default; override in prod
APP_DB="${PG_INIT_DB:-ferroehr}"

# psql as the bootstrap superuser, ON_ERROR_STOP so a failure aborts init.
psql_super() { psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" "$@"; }
psql_app() { psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$APP_DB" "$@"; }

echo "ferroehr init: creating role '${APP_USER}' and database '${APP_DB}'"

# 1) Login role (idempotent) — non-superuser by design — plus the three
#    NOLOGIN group roles of the layered role architecture. The app
#    role has no CREATEROLE, so the baseline migration can only grant to these
#    roles if they already exist; creating them here gives dev/compose the
#    same grant topology as a hardened deployment (no "roles absent" NOTICEs).
psql_super <<SQL
DO \$do\$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = '${APP_USER}') THEN
    CREATE ROLE "${APP_USER}" LOGIN PASSWORD '${APP_PASSWORD}';
  ELSE
    ALTER ROLE "${APP_USER}" LOGIN PASSWORD '${APP_PASSWORD}';
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_migrator') THEN
    CREATE ROLE ferroehr_migrator NOLOGIN;
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
    CREATE ROLE ferroehr_app NOLOGIN;
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_reader') THEN
    CREATE ROLE ferroehr_reader NOLOGIN;
  END IF;
  -- In dev the single app user plays both migrator and writer.
  GRANT ferroehr_migrator TO "${APP_USER}";
  GRANT ferroehr_app TO "${APP_USER}";
END
\$do\$;
SQL

# 2) Database owned by the app role (CREATE DATABASE cannot run in a DO block).
if ! psql_super -tAc "SELECT 1 FROM pg_database WHERE datname = '${APP_DB}'" | grep -q 1; then
  psql_super -c "CREATE DATABASE \"${APP_DB}\" OWNER \"${APP_USER}\";"
fi

# 3) Schemas owned by the app role + superuser-installed extensions, in the
#    app database. `ext` holds the openEHR helper functions and, by convention,
#    the extensions; both schemas are on the app's search_path (ehr, ext, public).
psql_app <<SQL
CREATE SCHEMA IF NOT EXISTS ehr AUTHORIZATION "${APP_USER}";
CREATE SCHEMA IF NOT EXISTS ext AUTHORIZATION "${APP_USER}";
-- The local IHE ATNA Audit Record Repository, deliberately its own schema.
CREATE SCHEMA IF NOT EXISTS audit AUTHORIZATION "${APP_USER}";

-- Installed by the superuser so the app role never needs it.
CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA ext;
CREATE EXTENSION IF NOT EXISTS pgcrypto   WITH SCHEMA ext;
CREATE EXTENSION IF NOT EXISTS pg_trgm    WITH SCHEMA ext;
-- Required by the temporal vo_version PRIMARY KEY (... WITHOUT OVERLAPS).
CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA ext;

-- The app role owns the schemas already; make the intent explicit.
GRANT ALL ON SCHEMA ehr   TO "${APP_USER}";
GRANT ALL ON SCHEMA ext   TO "${APP_USER}";
GRANT ALL ON SCHEMA audit TO "${APP_USER}";
SQL

echo "ferroehr init: done"
