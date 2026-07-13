#!/usr/bin/env bash
# openEHR CNF conformance runner — the acceptance instrument, W-10 multi-SUT.
#
# The thin wrapper satisfying the /run-conformance skill contract: bring up the
# selected SUT's compose stack, run the conformance CLI against it, write the
# per-SUT artefact set (results.json + report + Statement + Certificate +
# badges) under $CONF_OUT/<sut-name>/, tear down.
#
# Usage:
#   scripts/conformance.sh [FILTER]
#
# FILTER (optional) is passed to the CLI's --filter (an id substring).
#
# Env:
#   CONF_SUT        ehrbase-rs (default) | ehrbase-java | byo.
#                   ehrbase-rs: builds + composes the root stack (the current
#                     sources — the phase-gate zero-drift run, edition PINNED
#                     to development).
#                   ehrbase-java: composes the upstream official image from
#                     docker/benchmark/ (fairness register auto-applied;
#                     results are comparison DATA, never a gate).
#                   byo: no compose management — point CONF_BASE_URL at any
#                     deployed CDR (edition ladder = auto).
#   CONF_BASE_URL   SUT base URL (defaults per CONF_SUT).
#   CONF_SUT_NAME   output/lookup name for byo (default: byo).
#   CONF_AUTH       regular credential spec (default: basic:ehrbase:ehrbase).
#   CONF_ADMIN_AUTH admin credential spec   (default per CONF_SUT).
#   CONF_EDITION    auto|development|1.0.3  (default: the target's default).
#   CONF_PROFILE    all|core|standard|options (default: all).
#   CONF_FORMAT     json|xml|both           (default: both).
#   CONF_OUT        artefact root           (default: docs/conformance;
#                   the SUT name is appended by the CLI).
#   CONF_NO_COMPOSE if set, do not manage compose (assume the SUT is up).
set -Eeuo pipefail

FILTER="${1:-}"
SUT="${CONF_SUT:-ehrbase-rs}"
AUTH="${CONF_AUTH:-basic:ehrbase:ehrbase}"
PROFILE="${CONF_PROFILE:-all}"
FORMAT="${CONF_FORMAT:-both}"
OUT="${CONF_OUT:-docs/conformance}"

COMPOSE_ARGS=()
case "$SUT" in
  ehrbase-rs)
    BASE_URL="${CONF_BASE_URL:-http://localhost:8080/ehrbase/rest/openehr/v1}"
    # The compose dev config ships an ADMIN-role account (ehrbase-admin/ehrbase,
    # docker/ehrbase.dev.toml) — without it the AdminApi cases 403.
    ADMIN_AUTH="${CONF_ADMIN_AUTH:-basic:ehrbase-admin:ehrbase}"
    APP_SERVICE=ehrbase
    CORE_SERVICES=(ehrbase-postgres ehrbase)
    ;;
  ehrbase-java)
    # The upstream official image + its postgres, from the benchmark dual-stack
    # definitions (same images/credentials the X1 comparison pins).
    BASE_URL="${CONF_BASE_URL:-http://localhost:8091/ehrbase/rest/openehr/v1}"
    ADMIN_AUTH="${CONF_ADMIN_AUTH:-basic:ehrbase-admin:ehrbase}"
    APP_SERVICE=ehrbase-java
    CORE_SERVICES=(ehrbase-java-db ehrbase-java)
    COMPOSE_ARGS=(-f docker/benchmark/docker-compose.yml --profile java)
    ;;
  byo)
    BASE_URL="${CONF_BASE_URL:?CONF_SUT=byo requires CONF_BASE_URL}"
    ADMIN_AUTH="${CONF_ADMIN_AUTH:-$AUTH}"
    APP_SERVICE=""
    CORE_SERVICES=()
    CONF_NO_COMPOSE=1
    ;;
  *)
    echo "::error::unknown CONF_SUT '$SUT' (ehrbase-rs|ehrbase-java|byo)" >&2
    exit 2
    ;;
esac
STATUS_URL="${BASE_URL%/openehr/v1}/status"

manage_compose=1
[ -n "${CONF_NO_COMPOSE:-}" ] && manage_compose=0

cleanup() {
  if [ "$manage_compose" = "1" ]; then
    docker compose "${COMPOSE_ARGS[@]}" down -v || true
  fi
}
trap cleanup EXIT

wait_healthy() {
  local cid
  for _ in $(seq 1 60); do
    cid=$(docker compose "${COMPOSE_ARGS[@]}" ps -q "$APP_SERVICE")
    if [ -n "$cid" ] && [ "$(docker inspect -f '{{.State.Health.Status}}' "$cid")" = "healthy" ]; then
      return 0
    fi
    sleep 5
  done
  echo "::error::$APP_SERVICE container did not become healthy"
  docker compose "${COMPOSE_ARGS[@]}" logs "$APP_SERVICE" || true
  return 1
}

if [ "$manage_compose" = "1" ]; then
  echo "==> Starting $SUT services"
  if [ "$SUT" = "ehrbase-rs" ] && [ "${SKIP_BUILD:-0}" != "1" ]; then
    # --build: a conformance verdict on OUR server is only meaningful against
    # the current sources — a stale image once produced a silently-wrong drift
    # verdict (2026-07-09). Opt out via SKIP_BUILD=1 for a published-image run.
    docker compose "${COMPOSE_ARGS[@]}" up -d --build "${CORE_SERVICES[@]}"
  else
    docker compose "${COMPOSE_ARGS[@]}" up -d "${CORE_SERVICES[@]}"
  fi
  echo "==> Waiting for $APP_SERVICE to become healthy"
  wait_healthy
fi

if [ "$SUT" = "ehrbase-rs" ]; then
  echo "==> GET $STATUS_URL must be reachable"
  curl -fsS -o /dev/null "$STATUS_URL"
fi

echo "==> Running conformance suite (sut=$SUT profile=$PROFILE format=$FORMAT filter='${FILTER}')"
args=(run --sut "$SUT" --base-url "$BASE_URL" --auth "$AUTH" --admin-auth "$ADMIN_AUTH"
      --format "$FORMAT" --out "$OUT")
[ "$PROFILE" != "all" ] && args+=(--profile "$PROFILE")
[ -n "$FILTER" ] && args+=(--filter "$FILTER")
[ -n "${CONF_EDITION:-}" ] && args+=(--edition "$CONF_EDITION")
[ -n "${CONF_SUT_NAME:-}" ] && args+=(--sut-name "$CONF_SUT_NAME")

# Exit code is the CLI's: 0 pass · 1 failures · 2 runner/SUT error.
cargo run -q -p conformance --bin conformance -- "${args[@]}"
