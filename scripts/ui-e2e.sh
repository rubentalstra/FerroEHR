#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
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
#   UI_E2E_NO_BUILD     if set, `up --no-build` the compose stack (use images
#                       already present as the `*_IMAGE` refs) instead of
#                       building from source. CI pre-builds the app image with
#                       a layer cache exported to GHCR and sets this, so the
#                       cold runner does not pay the full compile.
#   UI_E2E_PREBUILT_CONSOLE
#                       if set, skip the host cargo-leptos build and run the
#                       console binary + site tree already at
#                       target/debug/ferroehr-admin-ui + target/site (CI builds
#                       them in a parallel job and downloads the artifact).
#   UI_E2E_NEXTEST_ARCHIVE
#                       if set, run the journeys from this prebuilt nextest
#                       archive (`cargo nextest archive -p ferroehr-admin-ui
#                       --features ssr`) via --archive-file/--workspace-remap
#                       instead of compiling the test binaries here.
#   UI_E2E_KEEP_UP      if set, skip teardown (local debugging).
#   UI_E2E_SHOTS_ONLY   if set, skip the journeys entirely (capture pass only).
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

# This lane composes as its OWN project (docs.docker.com/compose/how-tos/project-name):
# COMPOSE_PROJECT_NAME scopes EVERY `docker compose` call below — crucially the
# `down -v` teardown — to `ferroehr-e2e`, so it can never wipe a running dev
# (`ferroehr`) or conformance (`ferroehr-cnf`) stack (issue #282 D3/F7).
export COMPOSE_PROJECT_NAME=ferroehr-e2e
# Profiles are exported, never passed per-invocation: a service in a profile is
# excluded from the compose model of every call that does not enable it
# (docs.docker.com/compose/how-tos/profiles), so an inline `--profile keycloak`
# on the `up` left the trap's `down -v` blind to keycloak and leaked the
# container on every run. COMPOSE_PROFILES scopes EVERY compose call below —
# `up`, `stop` and the teardown alike. Comma-separate any profile added here.
# `admin-ui` is here because the console service now carries that profile: the
# export is what keeps the image mode's `stop ferroehr-admin-ui` cleanup and
# the trap's `down -v` able to see the container at all.
export COMPOSE_PROFILES=keycloak,admin-ui
# Build provenance for the compose-built images: the OCI-standard REVISION arg
# (forwarded by the compose build.args block, bridged into build.rs by the
# server Dockerfile). Degrades to `unknown` off-checkout.
export REVISION="${REVISION:-$(git -C "$ROOT" rev-parse --short=12 HEAD 2>/dev/null || echo unknown)}"

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

# ── Working-tree residue guard ──────────────────────────────────────────────
# A battery run must not change tracked files. The screenshot pass is the one
# legitimate writer (it regenerates website/book/src/admin-ui/img), so it is
# excluded from the comparison; everything else must match the state we started
# from — a pre-existing dirty tree is fine, NEW residue is not.
# Tolerates a non-checkout (no git, no repo): both samples come back empty and
# the comparison is trivially satisfied — there are no tracked files to dirty.
git_tree_state() {
  git -C "$ROOT" status --porcelain -- . ':(exclude)website/book/src/admin-ui/img' 2>/dev/null || true
}
TREE_STATE_BEFORE="$(git_tree_state)"
assert_no_tree_residue() {
  local after
  after="$(git_tree_state)"
  if [ "$after" != "$TREE_STATE_BEFORE" ]; then
    echo "FATAL: the e2e run left residue in the working tree:" >&2
    diff <(printf '%s\n' "$TREE_STATE_BEFORE") <(printf '%s\n' "$after") >&2 || true
    return 1
  fi
  echo "── working tree clean of run residue"
}

CONSOLE_PID=""
DRIVER_PID=""
cleanup() {
  [ -n "$CONSOLE_PID" ] && kill "$CONSOLE_PID" 2>/dev/null || true
  if [ -n "${UI_E2E_IMAGE:-}" ] && [ -z "${UI_E2E_KEEP_UP:-}" ]; then
    docker compose stop ferroehr-admin-ui >/dev/null 2>&1 || true
  fi
  [ -n "$DRIVER_PID" ] && kill "$DRIVER_PID" 2>/dev/null || true
  if [ -z "${UI_E2E_NO_COMPOSE:-}" ] && [ -z "${UI_E2E_KEEP_UP:-}" ]; then
    # --remove-orphans: belt-and-braces against a future profiled/renamed
    # service surviving the teardown the way keycloak used to.
    docker compose down -v --remove-orphans >/dev/null 2>&1 || true
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
  docker compose logs --tail 40 ferroehr keycloak 2>/dev/null || true
  return 1
}

# ── 1. The composed stack: postgres + CDR + Keycloak ────────────────────────
if [ -z "${UI_E2E_NO_COMPOSE:-}" ]; then
  echo "── compose up (postgres + ferroehr + keycloak)"
  # keycloak is behind the `keycloak` profile, enabled for every call in this
  # lane by the COMPOSE_PROFILES export above, so the OIDC journeys have an
  # issuer AND the teardown can see the container.
  BUILD_ARGS=(--build)
  [ -n "${UI_E2E_NO_BUILD:-}" ] && BUILD_ARGS=(--no-build)
  # The e2e overlay rides BOTH lanes: without it here, its `ferroehr:` env
  # block (tenancy on, terminology on) never reached the host-mode CDR — the
  # lane CI actually runs.
  docker compose -f docker-compose.yml -f docker-compose.override.yml \
    -f docker/admin-ui/e2e-env.yml \
    up -d "${BUILD_ARGS[@]}" ferroehr-postgres ferroehr keycloak
fi
wait_http "$CDR_URL/ferroehr/rest/status" 150
wait_http "$KEYCLOAK_URL/auth/realms/ferroehr/.well-known/openid-configuration" 90

# ── 2. Deterministic Keycloak test passwords (the shipped realm export holds
#      only hashes; reset via the bootstrap admin API — never edit the realm) ─
echo "── resetting Keycloak test-user passwords"
KC_TOKEN=$(curl -sf -X POST \
  -d "client_id=admin-cli" -d "username=admin" -d "password=admin" -d "grant_type=password" \
  "$KEYCLOAK_URL/auth/realms/master/protocol/openid-connect/token" | jq -r '.access_token')
for pair in "ferroehr-admin:E2ePass-admin1!" "ferroehr-user:E2ePass-user1!"; do
  user="${pair%%:*}"; pass="${pair#*:}"
  uid=$(curl -sf -H "Authorization: Bearer $KC_TOKEN" \
    "$KEYCLOAK_URL/auth/admin/realms/ferroehr/users?username=$user&exact=true" \
    | jq -r '.[0].id')
  curl -sf -X PUT -H "Authorization: Bearer $KC_TOKEN" -H "Content-Type: application/json" \
    -d "{\"type\":\"password\",\"value\":\"$pass\",\"temporary\":false}" \
    "$KEYCLOAK_URL/auth/admin/realms/ferroehr/users/$uid/reset-password"
done

# Register the console's redirect URI on the realm client (the shipped
# export's relative "/*" is rootUrl-relative and rejects our origin) — via
# the admin API, never by editing the realm file.
echo "── registering the console redirect URI on the ferroehr client"
CLIENT_ID=$(curl -sf -H "Authorization: Bearer $KC_TOKEN" \
  "$KEYCLOAK_URL/auth/admin/realms/ferroehr/clients?clientId=ferroehr" \
  | jq -r '.[0].id')
curl -sf -H "Authorization: Bearer $KC_TOKEN" \
  "$KEYCLOAK_URL/auth/admin/realms/ferroehr/clients/$CLIENT_ID" \
  | jq --arg origin "$CONSOLE_URL" '
      # Union-then-sort, as the previous version did: `unique` in jq both
      # de-duplicates and sorts, so re-running this script is idempotent rather
      # than accumulating duplicate redirect URIs on the Keycloak client.
      .redirectUris = ((.redirectUris // []) + [$origin + "/*"] | unique)
      | .webOrigins = ((.webOrigins // []) + [$origin] | unique)
    ' > /tmp/kc-client.json
curl -sf -X PUT -H "Authorization: Bearer $KC_TOKEN" -H "Content-Type: application/json" \
  -d @/tmp/kc-client.json \
  "$KEYCLOAK_URL/auth/admin/realms/ferroehr/clients/$CLIENT_ID"

# ── 2b. Seed clinical data over REST (never the database): one template, one
#        EHR, one composition committed then updated (two versions) — powers
#        the composition-viewer journey and the data-bearing doc screenshots.
#        The composition body is the CDR's OWN generated example (spec-valid
#        by construction; no hand-built fixture).
echo "── seeding an EHR + a two-version composition"
CDR_V1="$CDR_URL/ferroehr/rest/openehr/v1"
SEED_OPT="app/ferroehr-admin-ui/tests/fixtures/minimal_evaluation.opt"
SEED_TEMPLATE="minimal_evaluation.en.v1"
# Template upload is idempotent for the harness: 201 (created) or 409 (there).
opt_status=$(curl -s -o /dev/null -w "%{http_code}" -u ferroehr:ferroehr -X POST \
  "$CDR_V1/definition/template/adl1.4" -H "Content-Type: application/xml" \
  --data-binary @"$SEED_OPT")
case "$opt_status" in 201|409) ;; *) echo "FATAL: template upload -> $opt_status" >&2; exit 1;; esac
SEEDED_EHR_ID=$(curl -sf -u ferroehr:ferroehr -X POST "$CDR_V1/ehr" \
  -H "Prefer: return=representation" -H "Accept: application/json" \
  | jq -r '.ehr_id.value')
curl -sf -u ferroehr:ferroehr "$CDR_V1/definition/template/adl1.4/$SEED_TEMPLATE/example" \
  -H "Accept: application/json" > /tmp/ui-e2e-example.json
SEED_VUID=$(curl -sf -D - -o /dev/null -u ferroehr:ferroehr -X POST \
  "$CDR_V1/ehr/$SEEDED_EHR_ID/composition" \
  -H "Content-Type: application/json" -H "Accept: application/json" -H "Prefer: return=minimal" \
  --data-binary @/tmp/ui-e2e-example.json \
  | tr -d '\r' | sed -n 's/^[Ee][Tt]ag: W\/"\(.*\)"$/\1/p')
SEEDED_VO_ID="${SEED_VUID%%::*}"
curl -sf -o /dev/null -u ferroehr:ferroehr -X PUT \
  "$CDR_V1/ehr/$SEEDED_EHR_ID/composition/$SEEDED_VO_ID" \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -H "If-Match: \"$SEED_VUID\"" --data-binary @/tmp/ui-e2e-example.json
echo "   seeded EHR $SEEDED_EHR_ID / composition $SEEDED_VO_ID (2 versions)"
# Three more single-composition EHRs committed as FLAT with real quantity
# magnitudes (the example generator emits only the skeleton — issue #94 —
# so charts/tables need explicitly valued data points).
day=14
for magnitude in 36.5 37.8 39.1; do
  extra_ehr=$(curl -sf -u ferroehr:ferroehr -X POST "$CDR_V1/ehr" \
    -H "Prefer: return=representation" -H "Accept: application/json" \
    | jq -r '.ehr_id.value')
  curl -sf -o /dev/null -u ferroehr:ferroehr -X POST \
    "$CDR_V1/ehr/$extra_ehr/composition" \
    -H "Content-Type: application/openehr.wt.flat+json" -H "Accept: application/json" \
    -H "openehr-template-id: $SEED_TEMPLATE" -H "Prefer: return=minimal" \
    -d "{
      \"ctx/language\": \"en\", \"ctx/territory\": \"US\",
      \"ctx/composer_name\": \"Seed composer\",
      \"ctx/time\": \"2026-07-${day}T08:00:00Z\",
      \"minimal/minimal/quantity|magnitude\": $magnitude,
      \"minimal/minimal/quantity|unit\": \"kg\"
    }"
  day=$((day + 1))
done
echo "   seeded 3 extra FLAT compositions with quantity magnitudes"

# ── 3. The console under test ────────────────────────────────────────────────
if [ -n "${UI_E2E_IMAGE:-}" ]; then
  # Image mode — the TRUE shipped artifact: compose-build the console image
  # (docker/admin-ui/Dockerfile) with the e2e-env override supplying the OIDC
  # test wiring; the issuer (http://keycloak:8081) resolves in-network via
  # docker DNS and in the E2E browser via the harness host-resolver mapping.
  # The explicit -f chain here MATCHES the CDR up in step 1 (base + override +
  # the e2e overlay): compose recreates any dependency whose model differs, so
  # if this call's model gave `ferroehr` a different config than the running
  # container, `up ferroehr-admin-ui` would RECREATE the server mid-run —
  # breaking the lane. The bare calls (stop/down/logs) recreate nothing.
  if [ -n "${UI_E2E_IMAGE_REF:-}" ]; then
    echo "── compose up the PUBLISHED console image ($UI_E2E_IMAGE_REF)"
    FERROEHR_ADMIN_UI_IMAGE="$UI_E2E_IMAGE_REF" \
      docker compose -f docker-compose.yml -f docker-compose.override.yml \
      -f docker/admin-ui/e2e-env.yml \
      up -d --no-build --pull always ferroehr-admin-ui
  else
    echo "── compose up the console image (build from source)"
    docker compose -f docker-compose.yml -f docker-compose.override.yml \
      -f docker/admin-ui/e2e-env.yml \
      up -d --build ferroehr-admin-ui
  fi
else
  if [ -n "${UI_E2E_PREBUILT_CONSOLE:-}" ]; then
    echo "── using the prebuilt console (UI_E2E_PREBUILT_CONSOLE)"
    for p in "$ROOT/target/debug/ferroehr-admin-ui" "$ROOT/target/site/pkg"; do
      [ -e "$p" ] || { echo "FATAL: UI_E2E_PREBUILT_CONSOLE set but $p is missing" >&2; exit 1; }
    done
  else
    echo "── building the console (cargo-leptos)"
    (cd app/ferroehr-admin-ui && LEPTOS_TAILWIND_VERSION=v4.3.3 cargo leptos build)
  fi
  echo "── starting the console on $CONSOLE_ADDR"
  LEPTOS_SITE_ROOT="$ROOT/target/site" \
  LEPTOS_SITE_ADDR="$CONSOLE_ADDR" \
  LEPTOS_OUTPUT_NAME="ferroehr-admin-ui" \
  FERROEHR_ADMIN__CDR__BASE_URL="$CDR_URL" \
  FERROEHR_ADMIN__AUTH__OIDC__ENABLED="true" \
  FERROEHR_ADMIN__AUTH__OIDC__ISSUER="http://keycloak:8081/auth/realms/ferroehr" \
  FERROEHR_ADMIN__AUTH__OIDC__RESOLVE="keycloak=127.0.0.1:8081" \
  FERROEHR_ADMIN__AUTH__OIDC__CLIENT_ID="ferroehr" \
  FERROEHR_ADMIN__AUTH__OIDC__CLIENT_SECRET="bT5T4oWn3xNdBytQsl2cfpBDi1pp15Va" \
  FERROEHR_ADMIN__AUTH__OIDC__PUBLIC_BASE_URL="$CONSOLE_URL" \
    "$ROOT/target/debug/ferroehr-admin-ui" &
  CONSOLE_PID=$!
fi
wait_http "$CONSOLE_URL/login"

# Warm the served asset chain once: the first journey after a cold console
# start otherwise pays the whole debug-build wasm read (measured 100s+ cold)
# against the harness's hydration budget.
curl -sf "$CONSOLE_URL/login" \
  | grep -oE '(href|src)="/pkg/[^"]+"' \
  | sed -E 's/^(href|src)="//; s/"$//' \
  | sort -u \
  | while IFS= read -r asset; do
      curl -sf -o /dev/null "$CONSOLE_URL$asset" || true
    done

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
# Prebuilt archive vs in-tree compile: one selection switch, used by both the
# journey run and the docs-shots pass so the two can never diverge. The
# archive path runs the binaries `cargo nextest archive` packed (permissions
# survive inside the tarball); --workspace-remap points nextest's own
# workspace metadata back at this checkout (compile-time env!(...) paths are
# unaffected — CI builds the archive on the same runner image + workspace
# path, which the workflow documents).
NEXTEST_TARGET=(-p ferroehr-admin-ui --features ssr)
if [ -n "${UI_E2E_NEXTEST_ARCHIVE:-}" ]; then
  NEXTEST_TARGET=(--archive-file "$UI_E2E_NEXTEST_ARCHIVE" --workspace-remap "$ROOT")
fi
if [ -n "${UI_E2E_SHOTS_ONLY:-}" ]; then
  echo "── journeys skipped (UI_E2E_SHOTS_ONLY)"
else
echo "── running e2e journeys"
# The one e2e binary is `it` (tests/it/main.rs, one-binary layout #1887); the
# docs-screenshot journeys live in its `e2e_docs_shots` module and run only in
# the gated pass below (nextest set-difference `-`). An explicit FILTER arg
# scopes by test name and never picks them up.
# UI_E2E_CDR_URL is exported to the journeys too: a journey that needs a listing
# of its own (table paging) seeds and removes its fixtures over ITS-REST rather
# than through the UI, whose own paths have their own journeys.
NEXTEST_FILTER=(-E 'binary(it) - test(/^e2e_docs_shots::/)')
[ -n "$FILTER" ] && NEXTEST_FILTER=(-E "test($FILTER)")
UI_E2E_BASE_URL="$CONSOLE_URL" \
UI_E2E_WEBDRIVER_URL="http://127.0.0.1:$DRIVER_PORT" \
UI_E2E_SHOTS_DIR="$SHOTS_DIR" \
UI_E2E_BASIC_USER="ferroehr" \
UI_E2E_BASIC_PASS="ferroehr" \
UI_E2E_ADMIN_USER="ferroehr-admin" \
UI_E2E_ADMIN_PASS="ferroehr" \
UI_E2E_CDR_URL="$CDR_URL" \
UI_E2E_OIDC_USER="ferroehr-admin" \
UI_E2E_OIDC_PASS="E2ePass-admin1!" \
UI_E2E_SEEDED_EHR_ID="$SEEDED_EHR_ID" \
UI_E2E_SEEDED_VO_ID="$SEEDED_VO_ID" \
  cargo nextest run "${NEXTEST_TARGET[@]}" -j 1 "${NEXTEST_FILTER[@]}"
fi

# ── 6. The documentation-screenshot pass (opt-in) ────────────────────────────
# When UI_E2E_DOCS_SHOTS is set, capture the canonical per-screen screenshots
# for website/book (writes into website/book/src/admin-ui/img). Runs after the
# journeys so the browse journeys have seeded the fixture template.
if [ -n "${UI_E2E_DOCS_SHOTS:-}" ]; then
  echo "── capturing documentation screenshots"
  UI_E2E_BASE_URL="$CONSOLE_URL" \
  UI_E2E_WEBDRIVER_URL="http://127.0.0.1:$DRIVER_PORT" \
  UI_E2E_SHOTS_DIR="$SHOTS_DIR" \
  UI_E2E_BASIC_USER="ferroehr" \
  UI_E2E_BASIC_PASS="ferroehr" \
  UI_E2E_ADMIN_USER="ferroehr-admin" \
  UI_E2E_ADMIN_PASS="ferroehr" \
  UI_E2E_CDR_URL="$CDR_URL" \
  UI_E2E_SEEDED_EHR_ID="$SEEDED_EHR_ID" \
  UI_E2E_SEEDED_VO_ID="$SEEDED_VO_ID" \
  UI_E2E_DOCS_SHOTS=1 \
    cargo nextest run "${NEXTEST_TARGET[@]}" -j 1 -E 'test(/^e2e_docs_shots::/)'
fi

# ── 7. Nothing may have leaked into the checkout ─────────────────────────────
assert_no_tree_residue

echo "── e2e complete; screenshots in $SHOTS_DIR"
