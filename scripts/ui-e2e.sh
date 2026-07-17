#!/usr/bin/env bash
# Admin-console E2E harness (design doc §8d): compose up the CDR stack +
# Keycloak, build + run the console on the host, start chromedriver, run the
# e2e journeys via nextest, tear down. Mirrors scripts/conformance.sh.
#
# Usage:
#   scripts/ui-e2e.sh [FILTER]      # FILTER = nextest -E test(...) substring
#
# Env:
#   UI_E2E_NO_COMPOSE   if set, assume CDR+Keycloak are already up.
#   UI_E2E_KEEP_UP      if set, skip teardown (local debugging).
#   UI_E2E_DOCS_SHOTS   if set, also run the --docs-shots capture pass
#                       (canonical per-screen screenshots for website/book).
#   CHROMEDRIVER        chromedriver binary (default: chromedriver on PATH).
#
# The journeys themselves read:
#   UI_E2E_BASE_URL        the console origin (set by this script)
#   UI_E2E_WEBDRIVER_URL   the chromedriver endpoint (set by this script)
#   UI_E2E_SHOTS_DIR       screenshot output dir (set by this script)
# and skip-with-reason when unset, so a plain `cargo nextest run --workspace`
# without Docker stays green (the B4 --tx-server-url precedent).
set -Eeuo pipefail

FILTER="${1:-}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CONSOLE_ADDR="127.0.0.1:3300"
CONSOLE_URL="http://${CONSOLE_ADDR}"
CDR_URL="http://localhost:8080"
KEYCLOAK_URL="http://localhost:8081"
DRIVER_PORT=9515
SHOTS_DIR="$ROOT/target/ui-e2e/screenshots"
mkdir -p "$SHOTS_DIR"

CONSOLE_PID=""
DRIVER_PID=""
cleanup() {
  [ -n "$CONSOLE_PID" ] && kill "$CONSOLE_PID" 2>/dev/null || true
  [ -n "$DRIVER_PID" ] && kill "$DRIVER_PID" 2>/dev/null || true
  if [ -z "${UI_E2E_NO_COMPOSE:-}" ] && [ -z "${UI_E2E_KEEP_UP:-}" ]; then
    docker compose down -v >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

wait_http() { # url, tries
  local url="$1" tries="${2:-60}"
  for _ in $(seq 1 "$tries"); do
    if curl -sf -o /dev/null "$url"; then return 0; fi
    sleep 2
  done
  echo "FATAL: $url not reachable" >&2
  return 1
}

# ── 1. The composed stack: postgres + CDR + Keycloak ────────────────────────
if [ -z "${UI_E2E_NO_COMPOSE:-}" ]; then
  echo "── compose up (postgres + ehrbase + keycloak)"
  docker compose up -d --build ehrbase-postgres ehrbase keycloak
fi
wait_http "$CDR_URL/rest/status"
wait_http "$KEYCLOAK_URL/auth/realms/ehrbase/.well-known/openid-configuration" 90

# ── 2. Deterministic Keycloak test passwords (the shipped realm export holds
#      only hashes; reset via the bootstrap admin API — never edit the realm) ─
echo "── resetting Keycloak test-user passwords"
KC_TOKEN=$(curl -sf -X POST \
  -d "client_id=admin-cli" -d "username=admin" -d "password=admin" -d "grant_type=password" \
  "$KEYCLOAK_URL/auth/realms/master/protocol/openid-connect/token" | python3 -c "import json,sys;print(json.load(sys.stdin)['access_token'])")
for pair in "ehrbase-admin:E2ePass-admin1!" "ehrbase-user:E2ePass-user1!"; do
  user="${pair%%:*}"; pass="${pair#*:}"
  uid=$(curl -sf -H "Authorization: Bearer $KC_TOKEN" \
    "$KEYCLOAK_URL/auth/admin/realms/ehrbase/users?username=$user&exact=true" \
    | python3 -c "import json,sys;print(json.load(sys.stdin)[0]['id'])")
  curl -sf -X PUT -H "Authorization: Bearer $KC_TOKEN" -H "Content-Type: application/json" \
    -d "{\"type\":\"password\",\"value\":\"$pass\",\"temporary\":false}" \
    "$KEYCLOAK_URL/auth/admin/realms/ehrbase/users/$uid/reset-password"
done

# ── 3. Build + run the console on the host (the same code the OCI image ships) ─
echo "── building the console (cargo-leptos)"
(cd app/ehrbase-admin-ui && LEPTOS_TAILWIND_VERSION=v4.3.3 cargo leptos build)
echo "── starting the console on $CONSOLE_ADDR"
LEPTOS_SITE_ROOT="$ROOT/target/site" \
LEPTOS_SITE_ADDR="$CONSOLE_ADDR" \
LEPTOS_OUTPUT_NAME="ehrbase-admin-ui" \
EHRBASE_ADMIN__CDR__BASE_URL="$CDR_URL" \
EHRBASE_ADMIN__AUTH__OIDC__ENABLED="true" \
EHRBASE_ADMIN__AUTH__OIDC__ISSUER="$KEYCLOAK_URL/auth/realms/ehrbase" \
EHRBASE_ADMIN__AUTH__OIDC__CLIENT_ID="ehrbase" \
EHRBASE_ADMIN__AUTH__OIDC__CLIENT_SECRET="bT5T4oWn3xNdBytQsl2cfpBDi1pp15Va" \
EHRBASE_ADMIN__AUTH__OIDC__PUBLIC_BASE_URL="$CONSOLE_URL" \
  "$ROOT/target/debug/ehrbase-admin-ui" &
CONSOLE_PID=$!
wait_http "$CONSOLE_URL/login"

# ── 4. chromedriver ──────────────────────────────────────────────────────────
CHROMEDRIVER_BIN="${CHROMEDRIVER:-chromedriver}"
if ! command -v "$CHROMEDRIVER_BIN" >/dev/null; then
  echo "FATAL: chromedriver not found (set CHROMEDRIVER)" >&2
  exit 1
fi
"$CHROMEDRIVER_BIN" --port=$DRIVER_PORT &
DRIVER_PID=$!
wait_http "http://127.0.0.1:$DRIVER_PORT/status"

# ── 5. The journeys ──────────────────────────────────────────────────────────
echo "── running e2e journeys"
NEXTEST_FILTER=(-E 'binary(/^e2e_/)')
[ -n "$FILTER" ] && NEXTEST_FILTER=(-E "test($FILTER)")
UI_E2E_BASE_URL="$CONSOLE_URL" \
UI_E2E_WEBDRIVER_URL="http://127.0.0.1:$DRIVER_PORT" \
UI_E2E_SHOTS_DIR="$SHOTS_DIR" \
UI_E2E_BASIC_USER="ehrbase" \
UI_E2E_BASIC_PASS="ehrbase" \
UI_E2E_OIDC_USER="ehrbase-admin" \
UI_E2E_OIDC_PASS="E2ePass-admin1!" \
  cargo nextest run -p ehrbase-admin-ui --features ssr "${NEXTEST_FILTER[@]}"

echo "── e2e complete; screenshots in $SHOTS_DIR"
