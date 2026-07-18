#!/usr/bin/env bash
# Admin-console E2E harness: compose up the CDR stack +
# Keycloak, build + run the console on the host, start chromedriver, run the
# e2e journeys via nextest, tear down. Mirrors scripts/conformance.sh.
#
# Usage:
#   scripts/ui-e2e.sh [FILTER]      # FILTER = nextest -E test(...) substring
#
# Env:
#   UI_E2E_IMAGE        if set, run the journeys against the COMPOSED console
#                       image (docker compose build of docker/admin-ui/
#                       Dockerfile + the e2e-env override) instead of a host
#                       cargo-leptos build — the shipped-artifact battery.
#   UI_E2E_IMAGE_REF    with UI_E2E_IMAGE: use this exact (already published)
#                       image reference instead of building — CI verifies the
#                       very artifact containers.yml pushed.
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

if [ -n "${UI_E2E_IMAGE:-}" ]; then
  # Image mode: the composed console publishes the quickstart port.
  CONSOLE_ADDR="127.0.0.1:3000"
else
  CONSOLE_ADDR="127.0.0.1:3300"
fi
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
  if [ -n "${UI_E2E_IMAGE:-}" ] && [ -z "${UI_E2E_KEEP_UP:-}" ]; then
    docker compose stop ehrbase-admin-ui >/dev/null 2>&1 || true
  fi
  [ -n "$DRIVER_PID" ] && kill "$DRIVER_PID" 2>/dev/null || true
  if [ -z "${UI_E2E_NO_COMPOSE:-}" ] && [ -z "${UI_E2E_KEEP_UP:-}" ]; then
    docker compose down -v >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

wait_http() { # url, tries
  local url="$1" tries="${2:-90}"
  for _ in $(seq 1 "$tries"); do
    if curl -sf -o /dev/null "$url"; then return 0; fi
    sleep 2
  done
  echo "FATAL: $url not reachable — recent service logs follow" >&2
  docker compose logs --tail 40 ehrbase keycloak 2>/dev/null || true
  return 1
}

# ── 1. The composed stack: postgres + CDR + Keycloak ────────────────────────
if [ -z "${UI_E2E_NO_COMPOSE:-}" ]; then
  echo "── compose up (postgres + ehrbase + keycloak)"
  docker compose up -d --build ehrbase-postgres ehrbase keycloak
fi
wait_http "$CDR_URL/ehrbase/rest/status" 150
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

# Register the console's redirect URI on the realm client (the shipped
# export's relative "/*" is rootUrl-relative and rejects our origin) — via
# the admin API, never by editing the realm file.
echo "── registering the console redirect URI on the ehrbase client"
CLIENT_ID=$(curl -sf -H "Authorization: Bearer $KC_TOKEN" \
  "$KEYCLOAK_URL/auth/admin/realms/ehrbase/clients?clientId=ehrbase" \
  | python3 -c "import json,sys;print(json.load(sys.stdin)[0]['id'])")
curl -sf -H "Authorization: Bearer $KC_TOKEN" \
  "$KEYCLOAK_URL/auth/admin/realms/ehrbase/clients/$CLIENT_ID" \
  | CONSOLE_URL="$CONSOLE_URL" python3 -c "
import json, os, sys
c = json.load(sys.stdin)
origin = os.environ['CONSOLE_URL']
uris = set(c.get('redirectUris', []))
uris.add(f'{origin}/*')
c['redirectUris'] = sorted(uris)
origins = set(c.get('webOrigins', []))
origins.add(origin)
c['webOrigins'] = sorted(origins)
print(json.dumps(c))
" > /tmp/kc-client.json
curl -sf -X PUT -H "Authorization: Bearer $KC_TOKEN" -H "Content-Type: application/json" \
  -d @/tmp/kc-client.json \
  "$KEYCLOAK_URL/auth/admin/realms/ehrbase/clients/$CLIENT_ID"

# ── 2b. Seed clinical data over REST (never the database): one template, one
#        EHR, one composition committed then updated (two versions) — powers
#        the composition-viewer journey and the data-bearing doc screenshots.
#        The composition body is the CDR's OWN generated example (spec-valid
#        by construction; no hand-built fixture).
echo "── seeding an EHR + a two-version composition"
CDR_V1="$CDR_URL/ehrbase/rest/openehr/v1"
SEED_OPT="crates/openehr-flat/tests/fixtures/sdk/minimal_evaluation.opt"
SEED_TEMPLATE="minimal_evaluation.en.v1"
# Template upload is idempotent for the harness: 201 (created) or 409 (there).
opt_status=$(curl -s -o /dev/null -w "%{http_code}" -u ehrbase:ehrbase -X POST \
  "$CDR_V1/definition/template/adl1.4" -H "Content-Type: application/xml" \
  --data-binary @"$SEED_OPT")
case "$opt_status" in 201|409) ;; *) echo "FATAL: template upload -> $opt_status" >&2; exit 1;; esac
SEEDED_EHR_ID=$(curl -sf -u ehrbase:ehrbase -X POST "$CDR_V1/ehr" \
  -H "Prefer: return=representation" -H "Accept: application/json" \
  | python3 -c "import json,sys; print(json.load(sys.stdin)['ehr_id']['value'])")
curl -sf -u ehrbase:ehrbase "$CDR_V1/definition/template/adl1.4/$SEED_TEMPLATE/example" \
  -H "Accept: application/json" > /tmp/ui-e2e-example.json
SEED_VUID=$(curl -sf -D - -o /dev/null -u ehrbase:ehrbase -X POST \
  "$CDR_V1/ehr/$SEEDED_EHR_ID/composition" \
  -H "Content-Type: application/json" -H "Accept: application/json" -H "Prefer: return=minimal" \
  --data-binary @/tmp/ui-e2e-example.json \
  | tr -d '\r' | sed -n 's/^[Ee][Tt]ag: W\/"\(.*\)"$/\1/p')
SEEDED_VO_ID="${SEED_VUID%%::*}"
curl -sf -o /dev/null -u ehrbase:ehrbase -X PUT \
  "$CDR_V1/ehr/$SEEDED_EHR_ID/composition/$SEEDED_VO_ID" \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -H "If-Match: \"$SEED_VUID\"" --data-binary @/tmp/ui-e2e-example.json
echo "   seeded EHR $SEEDED_EHR_ID / composition $SEEDED_VO_ID (2 versions)"

# ── 3. The console under test ────────────────────────────────────────────────
if [ -n "${UI_E2E_IMAGE:-}" ]; then
  # Image mode — the TRUE shipped artifact: compose-build the console image
  # (docker/admin-ui/Dockerfile) with the e2e-env override supplying the OIDC
  # test wiring; the issuer (http://keycloak:8081) resolves in-network via
  # docker DNS and in the E2E browser via the harness host-resolver mapping.
  if [ -n "${UI_E2E_IMAGE_REF:-}" ]; then
    echo "── compose up the PUBLISHED console image ($UI_E2E_IMAGE_REF)"
    EHRBASE_ADMIN_UI_IMAGE="$UI_E2E_IMAGE_REF" \
      docker compose -f docker-compose.yml -f docker/admin-ui/e2e-env.yml \
      up -d --no-build --pull always ehrbase-admin-ui
  else
    echo "── compose up the console image (build from source)"
    docker compose -f docker-compose.yml -f docker/admin-ui/e2e-env.yml \
      up -d --build ehrbase-admin-ui
  fi
else
  echo "── building the console (cargo-leptos)"
  (cd app/ehrbase-admin-ui && LEPTOS_TAILWIND_VERSION=v4.3.3 cargo leptos build)
  echo "── starting the console on $CONSOLE_ADDR"
  LEPTOS_SITE_ROOT="$ROOT/target/site" \
  LEPTOS_SITE_ADDR="$CONSOLE_ADDR" \
  LEPTOS_OUTPUT_NAME="ehrbase-admin-ui" \
  EHRBASE_ADMIN__CDR__BASE_URL="$CDR_URL" \
  EHRBASE_ADMIN__AUTH__OIDC__ENABLED="true" \
  EHRBASE_ADMIN__AUTH__OIDC__ISSUER="http://keycloak:8081/auth/realms/ehrbase" \
  EHRBASE_ADMIN__AUTH__OIDC__RESOLVE="keycloak=127.0.0.1:8081" \
  EHRBASE_ADMIN__AUTH__OIDC__CLIENT_ID="ehrbase" \
  EHRBASE_ADMIN__AUTH__OIDC__CLIENT_SECRET="bT5T4oWn3xNdBytQsl2cfpBDi1pp15Va" \
  EHRBASE_ADMIN__AUTH__OIDC__PUBLIC_BASE_URL="$CONSOLE_URL" \
    "$ROOT/target/debug/ehrbase-admin-ui" &
  CONSOLE_PID=$!
fi
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
# The docs-screenshot binary (e2e_docs_shots) matches `binary(/^e2e_/)` too, so
# exclude it here (nextest set-difference `-`); it runs only in the gated pass
# below. An explicit FILTER arg scopes by test name and never picks it up.
NEXTEST_FILTER=(-E 'binary(/^e2e_/) - binary(e2e_docs_shots)')
[ -n "$FILTER" ] && NEXTEST_FILTER=(-E "test($FILTER)")
UI_E2E_BASE_URL="$CONSOLE_URL" \
UI_E2E_WEBDRIVER_URL="http://127.0.0.1:$DRIVER_PORT" \
UI_E2E_SHOTS_DIR="$SHOTS_DIR" \
UI_E2E_BASIC_USER="ehrbase" \
UI_E2E_BASIC_PASS="ehrbase" \
UI_E2E_OIDC_USER="ehrbase-admin" \
UI_E2E_OIDC_PASS="E2ePass-admin1!" \
UI_E2E_SEEDED_EHR_ID="$SEEDED_EHR_ID" \
UI_E2E_SEEDED_VO_ID="$SEEDED_VO_ID" \
  cargo nextest run -p ehrbase-admin-ui --features ssr -j 1 "${NEXTEST_FILTER[@]}"

# ── 6. The documentation-screenshot pass (opt-in) ────────────────────────────
# When UI_E2E_DOCS_SHOTS is set, capture the canonical per-screen screenshots
# for website/book (writes into website/book/src/admin-ui/img). Runs after the
# journeys so the browse journeys have seeded the fixture template.
if [ -n "${UI_E2E_DOCS_SHOTS:-}" ]; then
  echo "── capturing documentation screenshots"
  UI_E2E_BASE_URL="$CONSOLE_URL" \
  UI_E2E_WEBDRIVER_URL="http://127.0.0.1:$DRIVER_PORT" \
  UI_E2E_SHOTS_DIR="$SHOTS_DIR" \
  UI_E2E_BASIC_USER="ehrbase" \
  UI_E2E_BASIC_PASS="ehrbase" \
  UI_E2E_SEEDED_EHR_ID="$SEEDED_EHR_ID" \
  UI_E2E_SEEDED_VO_ID="$SEEDED_VO_ID" \
  UI_E2E_DOCS_SHOTS=1 \
    cargo nextest run -p ehrbase-admin-ui --features ssr -j 1 -E 'binary(e2e_docs_shots)'
fi

echo "── e2e complete; screenshots in $SHOTS_DIR"
