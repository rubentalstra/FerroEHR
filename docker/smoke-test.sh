#!/usr/bin/env bash
# End-to-end smoke test for the two container images (amd64).
#
# Brings up ONLY the two core services (postgres + app; Keycloak is not needed
# for the Basic-auth path), then:
#   1. waits for the app healthcheck to report healthy;
#   2. asserts GET /rest/status returns 200;
#   3. creates an EHR with the dev Basic credentials (ehrbase/ehrbase);
#   4. restarts the app and proves the second-boot migration run is a no-op
#      (the sqlx ledger is unchanged — the library migrator is silent on no-op,
#      so the row count is the reliable idempotency signal; app logs are dumped
#      for visibility);
#   5. tears everything down.
#
# EHRBASE_IMAGE / EHRBASE_POSTGRES_IMAGE select the images to run.
set -Eeuo pipefail

# Own project (docs.docker.com/compose/how-tos/project-name) so the `down -v`
# teardown is scoped to `ehrbase-rs-smoke` and never wipes a running dev
# (`ehrbase-rs`) stack (issue #282 D3). Only the two core services are started
# (keycloak/seaweedfs are behind profiles and stay down).
export COMPOSE_PROJECT_NAME=ehrbase-rs-smoke
BASE="http://localhost:8080/ehrbase/rest"
CORE_SERVICES=(ehrbase-postgres ehrbase)

cleanup() {
  echo "::group::app logs"
  docker compose logs ehrbase || true
  echo "::endgroup::"
  docker compose down -v || true
}
trap cleanup EXIT

wait_healthy() {
  local cid
  for _ in $(seq 1 60); do
    cid=$(docker compose ps -q ehrbase)
    if [ -n "$cid" ] && [ "$(docker inspect -f '{{.State.Health.Status}}' "$cid")" = "healthy" ]; then
      return 0
    fi
    sleep 5
  done
  echo "::error::app container did not become healthy"
  docker compose ps
  docker compose logs ehrbase || true
  return 1
}

migration_count() {
  # Query as the bootstrap superuser (peer auth over the container's unix
  # socket maps the postgres OS user to the postgres role).
  docker compose exec -T ehrbase-postgres \
    psql -U postgres -d "${PG_INIT_DB:-ehrbase}" -tAc \
    "SELECT count(*) FROM ehr._sqlx_migrations" | tr -d '[:space:]'
}

echo "==> Starting core services"
docker compose up -d --no-build "${CORE_SERVICES[@]}"

echo "==> Waiting for app to become healthy"
wait_healthy

echo "==> GET /rest/status must be 200"
code=$(curl -fsS -o /dev/null -w '%{http_code}' "$BASE/status")
[ "$code" = "200" ] || { echo "::error::/rest/status returned $code"; exit 1; }

echo "==> POST /ehr with dev Basic creds must be 201"
code=$(curl -sS -o /dev/null -w '%{http_code}' -u ehrbase:ehrbase \
  -X POST "$BASE/openehr/v1/ehr" \
  -H 'Prefer: return=minimal')
[ "$code" = "201" ] || { echo "::error::POST /ehr returned $code (expected 201)"; exit 1; }

echo "==> Recording migration ledger before restart"
before=$(migration_count)
echo "    ehr._sqlx_migrations count = $before"
[ "$before" -ge 1 ] || { echo "::error::migrations did not apply on first boot"; exit 1; }

echo "==> Restarting app (second boot must be a migration no-op)"
docker compose restart ehrbase
wait_healthy
after=$(migration_count)
echo "    ehr._sqlx_migrations count = $after"
[ "$before" = "$after" ] || {
  echo "::error::migration count changed across restart ($before -> $after); not idempotent"
  exit 1
}

echo "==> Smoke test passed"
