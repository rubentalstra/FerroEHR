#!/usr/bin/env bash
# Preconfigure the ehrbase-rs database. Runs once, as the bootstrap superuser
# (POSTGRES_USER), on first initialisation of an empty data directory.
#
# Creates exactly what the app's sqlx migrator (`ehrbase::db::run_migrations`)
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

APP_USER="${EHRBASE_DB_USER:-ehrbase}"
APP_PASSWORD="${EHRBASE_DB_PASSWORD:-ehrbase}"   # dev default; override in prod
APP_DB="${EHRBASE_DB_NAME:-ehrbase}"

# psql as the bootstrap superuser, ON_ERROR_STOP so a failure aborts init.
psql_super() { psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" "$@"; }
psql_app() { psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$APP_DB" "$@"; }

echo "ehrbase-rs init: creating role '${APP_USER}' and database '${APP_DB}'"

# 1) Login role (idempotent) — non-superuser by design.
psql_super <<SQL
DO \$do\$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = '${APP_USER}') THEN
    CREATE ROLE "${APP_USER}" LOGIN PASSWORD '${APP_PASSWORD}';
  ELSE
    ALTER ROLE "${APP_USER}" LOGIN PASSWORD '${APP_PASSWORD}';
  END IF;
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

-- Installed by the superuser so the app role never needs it.
CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA ext;
CREATE EXTENSION IF NOT EXISTS pgcrypto   WITH SCHEMA ext;
CREATE EXTENSION IF NOT EXISTS pg_trgm    WITH SCHEMA ext;
-- Required by the temporal vo_version PRIMARY KEY (... WITHOUT OVERLAPS).
CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA ext;

-- The app role owns both schemas already; make the intent explicit.
GRANT ALL ON SCHEMA ehr TO "${APP_USER}";
GRANT ALL ON SCHEMA ext TO "${APP_USER}";
SQL

echo "ehrbase-rs init: done"
