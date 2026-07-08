#!/usr/bin/env bash
# Dual-stack benchmark comparison runner (docs/design/benchmarking.md §3.1, §6).
#
# Honest one-at-a-time protocol: bring up ehrbase-rs alone, benchmark it, take
# it down; then EHRbase Java alone, benchmark it (merged into the same report),
# take it down — so neither server contends with the other for host resources.
#
# Usage:  docker/benchmark/run.sh [--smoke] [--scenario W2] [--only rs|java]
# Output: docs/benchmarks/REPORT.md + results.json (host machine auto-captured).
set -Eeuo pipefail

cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"
COMPOSE="docker compose -f docker-compose.yml"
OUT="$REPO_ROOT/docs/benchmarks"
BENCH_ARGS=()
ONLY=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --smoke) BENCH_ARGS+=(--smoke); shift ;;
    --scenario) BENCH_ARGS+=(--scenario "$2"); shift 2 ;;
    --only) ONLY="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

# Build the harness once (release for realistic client-side cost).
echo "==> building bench harness"
( cd "$REPO_ROOT" && cargo build --release -p benchmark --bin bench )
BENCH="$REPO_ROOT/target/release/bench"

wait_ready() {
  local url="$1" name="$2"
  echo "==> waiting for $name at $url"
  for _ in $(seq 1 90); do
    # Probe with Basic auth: ehrbase-rs's /rest/status is public (auth ignored),
    # EHRbase Java protects it (needs auth). A 2xx means fully up + DB-migrated.
    code=$(curl -s -o /dev/null -w '%{http_code}' -u ehrbase:ehrbase "$url" || echo 000)
    if [[ "$code" == "200" ]]; then echo "    $name is up (200)"; return 0; fi
    sleep 5
  done
  echo "::error:: $name did not become ready (last HTTP $code)" >&2
  return 1
}

bench_target() {
  local profile="$1" impl="$2" port="$3" merge="$4"
  echo "==> starting $impl stack"
  $COMPOSE --profile "$profile" up -d
  # ITS-REST status endpoint (public, unauthenticated).
  wait_ready "http://localhost:${port}/ehrbase/rest/status" "$impl"
  echo "==> benchmarking $impl"
  local args=(run
    --base-url "http://localhost:${port}/ehrbase/rest/openehr/v1"
    --implementation "$impl"
    --auth "basic:ehrbase:ehrbase"
    --out "$OUT"
    "${BENCH_ARGS[@]}")
  [[ "$merge" == "merge" ]] && args+=(--merge)
  "$BENCH" "${args[@]}"
  echo "==> stopping $impl stack"
  $COMPOSE --profile "$profile" down -v
}

trap '$COMPOSE --profile rs --profile java down -v >/dev/null 2>&1 || true' EXIT

if [[ "$ONLY" != "java" ]]; then
  bench_target rs ehrbase-rs 8090 fresh
fi
if [[ "$ONLY" != "rs" ]]; then
  # Merge into the ehrbase-rs results so the report compares both.
  merge_mode="merge"; [[ "$ONLY" == "java" ]] && merge_mode="fresh"
  bench_target java ehrbase-java 8091 "$merge_mode"
fi

echo "==> report written to docs/benchmarks/REPORT.md"
