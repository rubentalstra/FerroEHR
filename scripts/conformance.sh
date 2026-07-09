#!/usr/bin/env bash
# openEHR CNF conformance runner (the ADR-008 acceptance instrument).
#
# The thin wrapper satisfying the /run-conformance skill contract
# (docs/design/conformance-framework.md §4.4): bring up the compose stack, run
# the conformance CLI against it, write docs/conformance/, tear down.
#
# Usage:
#   scripts/conformance.sh [FILTER]
#
# FILTER (optional) is passed to the CLI's --filter (an id substring).
#
# Env:
#   CONF_BASE_URL   SUT base URL (default: the compose stack's app).
#   CONF_AUTH       regular credential spec (default: basic:ehrbase:ehrbase).
#   CONF_ADMIN_AUTH admin credential spec   (default: same as CONF_AUTH).
#   CONF_PROFILE    all|core|standard|options (default: all — the full catalogue).
#   CONF_FORMAT     json|xml|both           (default: both).
#   CONF_OUT        report output dir       (default: docs/conformance).
#   CONF_NO_COMPOSE if set, do not manage compose (assume the SUT is already up).
set -Eeuo pipefail

FILTER="${1:-}"
BASE_URL="${CONF_BASE_URL:-http://localhost:8080/ehrbase/rest/openehr/v1}"
STATUS_URL="${BASE_URL%/openehr/v1}/status"
AUTH="${CONF_AUTH:-basic:ehrbase:ehrbase}"
# The compose dev config ships an ADMIN-role account (ehrbase-admin/ehrbase,
# docker/ehrbase.dev.toml) — without it the AdminApi cases 403.
ADMIN_AUTH="${CONF_ADMIN_AUTH:-basic:ehrbase-admin:ehrbase}"
# "all" (the default) omits --profile: the runner then executes the whole
# catalogue — the shape of the committed baseline (318 executions). A profile
# name (core|standard|options) restricts to cases that profile requires.
PROFILE="${CONF_PROFILE:-all}"
FORMAT="${CONF_FORMAT:-both}"
OUT="${CONF_OUT:-docs/conformance}"
CORE_SERVICES=(ehrbase-postgres ehrbase)

manage_compose=1
[ -n "${CONF_NO_COMPOSE:-}" ] && manage_compose=0

cleanup() {
  if [ "$manage_compose" = "1" ]; then
    docker compose down -v || true
  fi
}
trap cleanup EXIT

wait_healthy() {
  # Reuse docker/smoke-test.sh's wait-healthy approach.
  local cid
  for _ in $(seq 1 60); do
    cid=$(docker compose ps -q ehrbase)
    if [ -n "$cid" ] && [ "$(docker inspect -f '{{.State.Health.Status}}' "$cid")" = "healthy" ]; then
      return 0
    fi
    sleep 5
  done
  echo "::error::app container did not become healthy"
  docker compose logs ehrbase || true
  return 1
}

if [ "$manage_compose" = "1" ]; then
  # --build: a conformance verdict is only meaningful against the current
  # sources. Without it, compose reuses whatever image exists — a stale image
  # once produced a silently-wrong drift verdict (2026-07-09). Opt out only
  # for a deliberate against-a-published-image run via SKIP_BUILD=1.
  echo "==> Starting core services"
  if [ "${SKIP_BUILD:-0}" = "1" ]; then
    docker compose up -d "${CORE_SERVICES[@]}"
  else
    docker compose up -d --build "${CORE_SERVICES[@]}"
  fi
  echo "==> Waiting for app to become healthy"
  wait_healthy
fi

echo "==> GET $STATUS_URL must be reachable"
curl -fsS -o /dev/null "$STATUS_URL"

echo "==> Running conformance suite (profile=$PROFILE format=$FORMAT filter='${FILTER}')"
args=(run --base-url "$BASE_URL" --auth "$AUTH" --admin-auth "$ADMIN_AUTH"
      --format "$FORMAT" --out "$OUT")
[ "$PROFILE" != "all" ] && args+=(--profile "$PROFILE")
[ -n "$FILTER" ] && args+=(--filter "$FILTER")

# Exit code is the CLI's: 0 pass · 1 failures · 2 runner/SUT error.
cargo run -q -p conformance --bin conformance -- "${args[@]}"
