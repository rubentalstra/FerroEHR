#!/usr/bin/env bash
# The hospital-day benchmark runner (register 00/01) — mirrors
# scripts/conformance.sh: bring up the selected SUT's compose stack, measure its
# cold-start, drive the workload via `bench run`, write the per-SUT artefact set
# under $BENCH_OUT/<sut-name>/, tear down.
#
# Usage:
#   scripts/benchmark.sh
#
# Env:
#   BENCH_SUT        ehrbase-rs (default) | ehrbase-java | byo.
#                     ehrbase-rs: builds + composes the ROOT stack (current
#                       sources — --build unless SKIP_BUILD=1).
#                     ehrbase-java: composes the upstream official image from
#                       docker/benchmark/docker-compose.yml --profile java.
#                     byo: no compose management — point BENCH_BASE_URL at any
#                       deployed CDR (no resource/storage/cold-start sampling).
#   BENCH_BASE_URL   SUT base URL (defaults per BENCH_SUT).
#   BENCH_AUTH       regular credential spec (default: basic:ehrbase:ehrbase).
#   BENCH_ADMIN_AUTH admin credential spec   (default per BENCH_SUT).
#   BENCH_PROFILE    smoke (default) | hour | day.
#   BENCH_SCALE      empty (default) | 10k | 100k | 1m.
#   BENCH_WARD_SIZE  admitted patients        (default: 20).
#   BENCH_LOAD_FACTOR arrival-rate factor L   (default: 1.0).
#   BENCH_SEED       deterministic generator seed (default: the CLI's fixed seed).
#   BENCH_NO_SEED    if set, skip seeding (DB already at the scale rung).
#   BENCH_KNEE       if set, run `bench knee` (the maximum-sustained-throughput
#                     ladder) instead of `bench run` — same compose management.
#   BENCH_KNEE_STEPS the ascending load-factor ladder (default: 1,2,4,8,16,32).
#   BENCH_STEP_WINDOW per-step measurement window in seconds (default: 120).
#   BENCH_WARMUP     per-step warmup floor in seconds        (default: 15).
#   BENCH_OUT        artefact root            (default: docs/benchmarks;
#                    the SUT name is appended by the CLI).
#   BENCH_DB_POOL    connection-pool ceiling applied to BOTH stacks in
#                    lockstep (ehrbase-rs EHRBASE_DB_MAX_CONNECTIONS /
#                    upstream HikariCP maximumPoolSize; default 50 — the
#                    config-parity dominant tunable; each stack's own
#                    PostgreSQL max_connections default of 100 covers it).
#   BENCH_NO_COMPOSE if set, do not manage compose (assume the SUT is up).
#   SKIP_BUILD       ehrbase-rs: compose up without --build (published-image run).
set -Eeuo pipefail

SUT="${BENCH_SUT:-ehrbase-rs}"
AUTH="${BENCH_AUTH:-basic:ehrbase:ehrbase}"
PROFILE="${BENCH_PROFILE:-smoke}"
SCALE="${BENCH_SCALE:-empty}"
OUT="${BENCH_OUT:-docs/benchmarks}"
# Pool parity: exported for both compose files (see the env docs above).
export EHRBASE_DB_MAX_CONNECTIONS="${BENCH_DB_POOL:-50}"
export BENCH_DB_POOL="${BENCH_DB_POOL:-50}"
# Signing parity (benchmarking.md §3.4): version signing is an ehrbase-rs
# extension upstream does not perform — running it on-for-us/absent-for-them
# is an unfair self-handicap in throughput comparisons. OFF for benchmark
# runs, labeled in the report env; set BENCH_SIGNING=1 to measure with it.
if [ "${BENCH_SIGNING:-0}" != "1" ]; then
  export EHRBASE_SIGNING_ENABLED=false
fi
WARD_SIZE="${BENCH_WARD_SIZE:-20}"
LOAD_FACTOR="${BENCH_LOAD_FACTOR:-1.0}"

# Millisecond epoch (bash 3.2 on macOS lacks EPOCHREALTIME/date %N).
now_ms() {
  if command -v python3 >/dev/null 2>&1; then
    python3 -c 'import time; print(int(time.time()*1000))'
  else
    echo $(( $(date +%s) * 1000 ))
  fi
}

# macOS bash 3.2 treats an empty array expansion as unbound under `set -u`.
COMPOSE_ARGS=()
compose() { docker compose ${COMPOSE_ARGS[@]+"${COMPOSE_ARGS[@]}"} "$@"; }

case "$SUT" in
  ehrbase-rs)
    BASE_URL="${BENCH_BASE_URL:-http://localhost:8080/ehrbase/rest/openehr/v1}"
    ADMIN_AUTH="${BENCH_ADMIN_AUTH:-basic:ehrbase-admin:ehrbase}"
    APP_SERVICE=ehrbase
    CORE_SERVICES=(ehrbase-postgres ehrbase)
    # Root compose sets `name: ehrbase-rs` → <project>-<service>-<index>.
    APP_CONTAINER=ehrbase-rs-ehrbase-1
    DB_CONTAINER=ehrbase-rs-ehrbase-postgres-1
    DB_USER=ehrbase
    DB_NAME=ehrbase
    ;;
  ehrbase-java)
    BASE_URL="${BENCH_BASE_URL:-http://localhost:8091/ehrbase/rest/openehr/v1}"
    ADMIN_AUTH="${BENCH_ADMIN_AUTH:-basic:ehrbase-admin:ehrbase}"
    APP_SERVICE=ehrbase-java
    CORE_SERVICES=(ehrbase-java-db ehrbase-java)
    COMPOSE_ARGS=(-f docker/benchmark/docker-compose.yml --profile java)
    # docker/benchmark/ has no `name:` → project defaults to the dir `benchmark`.
    APP_CONTAINER=benchmark-ehrbase-java-1
    DB_CONTAINER=benchmark-ehrbase-java-db-1
    DB_USER=ehrbase
    DB_NAME=ehrbase
    ;;
  byo)
    BASE_URL="${BENCH_BASE_URL:?BENCH_SUT=byo requires BENCH_BASE_URL}"
    ADMIN_AUTH="${BENCH_ADMIN_AUTH:-$AUTH}"
    APP_SERVICE=""
    CORE_SERVICES=()
    APP_CONTAINER=""
    DB_CONTAINER=""
    BENCH_NO_COMPOSE=1
    ;;
  *)
    echo "::error::unknown BENCH_SUT '$SUT' (ehrbase-rs|ehrbase-java|byo)" >&2
    exit 2
    ;;
esac
STATUS_URL="${BASE_URL%/openehr/v1}/status"

manage_compose=1
[ -n "${BENCH_NO_COMPOSE:-}" ] && manage_compose=0

cleanup() {
  if [ "$manage_compose" = "1" ]; then
    compose down -v || true
  fi
}
trap cleanup EXIT

wait_healthy() {
  # Containers with a compose healthcheck report .State.Health; ones without
  # (the upstream ehrbase-java image) are probed over HTTP instead.
  local cid health code
  for _ in $(seq 1 60); do
    cid=$(compose ps -q "$APP_SERVICE")
    if [ -n "$cid" ]; then
      health=$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$cid")
      if [ "$health" = "healthy" ]; then
        return 0
      fi
      if [ "$health" = "none" ]; then
        code=$(curl -s -o /dev/null -w '%{http_code}' -u ehrbase:ehrbase "$BASE_URL/ehr" || true)
        [ "$code" != "000" ] && return 0
      fi
    fi
    sleep 5
  done
  echo "::error::$APP_SERVICE container did not become healthy" >&2
  compose logs "$APP_SERVICE" || true
  return 1
}

COLD_MS=""
if [ "$manage_compose" = "1" ]; then
  echo "==> Starting $SUT services"
  T0=$(now_ms)
  if [ "$SUT" = "ehrbase-rs" ] && [ "${SKIP_BUILD:-0}" != "1" ]; then
    compose up -d --build "${CORE_SERVICES[@]}"
  else
    compose up -d "${CORE_SERVICES[@]}"
  fi
  echo "==> Waiting for $APP_SERVICE to become healthy"
  wait_healthy
  T1=$(now_ms)
  COLD_MS=$(( T1 - T0 ))
  echo "==> Cold start (compose-up → healthy): ${COLD_MS} ms"
fi

if [ "$SUT" = "ehrbase-rs" ]; then
  echo "==> GET $STATUS_URL must be reachable"
  curl -fsS -o /dev/null "$STATUS_URL"
fi

if [ -n "${BENCH_KNEE:-}" ]; then
  echo "==> Running knee/saturation ladder (sut=$SUT scale=$SCALE ward=$WARD_SIZE)"
  kargs=(knee --sut "$SUT" --base-url "$BASE_URL" --auth "$AUTH" --admin-auth "$ADMIN_AUTH"
         --scale "$SCALE" --ward-size "$WARD_SIZE" --out "$OUT")
  [ -n "$APP_CONTAINER" ] && kargs+=(--app-container "$APP_CONTAINER")
  [ -n "$DB_CONTAINER" ] && kargs+=(--db-container "$DB_CONTAINER" --db-user "$DB_USER" --db-name "$DB_NAME")
  [ -n "${BENCH_SEED:-}" ] && kargs+=(--seed "$BENCH_SEED")
  [ -n "${BENCH_NO_SEED:-}" ] && kargs+=(--no-seed)
  [ -n "${BENCH_KNEE_STEPS:-}" ] && kargs+=(--steps "$BENCH_KNEE_STEPS")
  [ -n "${BENCH_STEP_WINDOW:-}" ] && kargs+=(--step-window "$BENCH_STEP_WINDOW")
  [ -n "${BENCH_WARMUP:-}" ] && kargs+=(--warmup "$BENCH_WARMUP")
  # Exit code is the CLI's: 0 ok · 2 runner/SUT error.
  cargo run -q -p benchmark --bin bench -- "${kargs[@]}"
else
  echo "==> Running benchmark (sut=$SUT profile=$PROFILE scale=$SCALE ward=$WARD_SIZE L=$LOAD_FACTOR)"
  args=(run --sut "$SUT" --base-url "$BASE_URL" --auth "$AUTH" --admin-auth "$ADMIN_AUTH"
        --profile "$PROFILE" --scale "$SCALE" --ward-size "$WARD_SIZE"
        --load-factor "$LOAD_FACTOR" --out "$OUT")
  [ -n "$APP_CONTAINER" ] && args+=(--app-container "$APP_CONTAINER")
  [ -n "$DB_CONTAINER" ] && args+=(--db-container "$DB_CONTAINER" --db-user "$DB_USER" --db-name "$DB_NAME")
  [ -n "$COLD_MS" ] && args+=(--cold-start-ms "$COLD_MS")
  [ -n "${BENCH_SEED:-}" ] && args+=(--seed "$BENCH_SEED")
  [ -n "${BENCH_NO_SEED:-}" ] && args+=(--no-seed)

  # Exit code is the CLI's: 0 ok · 1 error-rate flag breached · 2 runner/SUT error.
  cargo run -q -p benchmark --bin bench -- "${args[@]}"
fi
