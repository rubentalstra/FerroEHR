#!/usr/bin/env bash
# P20 profiling harness (phase-20-optimization.md T1): run ONE capacity step
# of the hospital-day workload against the composed ehrbase-rs stack and dump
# the PostgreSQL statement profile + the AQL dashboard plans into a committed
# evidence file. Never optimize without a before/after pair of these.
#
# Usage: scripts/profile.sh [L] [STEP_WINDOW_S]      (defaults: 32, 120)
# Env:   PROFILE_SCALE (default 10k) · SKIP_BUILD=1 to reuse the local image.
set -Eeuo pipefail

L="${1:-32}"
WINDOW="${2:-120}"
SCALE="${PROFILE_SCALE:-10k}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="docs/benchmarks/profiles/${STAMP}-L${L}-${SCALE}.md"
mkdir -p docs/benchmarks/profiles

# Pool/signing parity identical to the benchmark harness.
export EHRBASE_DB_MAX_CONNECTIONS="${BENCH_DB_POOL:-50}"
export EHRBASE_SIGNING_ENABLED=false

PSQL() { docker exec ehrbase-rs-ehrbase-postgres-1 psql -U ehrbase -d ehrbase -Atc "$1"; }
PSQLF() { docker exec ehrbase-rs-ehrbase-postgres-1 psql -U postgres -d ehrbase -c "$1"; }

echo "==> composing stack"
docker compose down -v >/dev/null 2>&1 || true
if [ "${SKIP_BUILD:-0}" != "1" ]; then
  docker compose up -d --build ehrbase-postgres ehrbase
else
  docker compose up -d ehrbase-postgres ehrbase
fi
for _ in $(seq 1 60); do
  code=$(curl -s -o /dev/null -w '%{http_code}' http://localhost:8080/ehrbase/rest/status || true)
  [ "$code" = "200" ] && break; sleep 3
done

echo "==> seeding ${SCALE}"
cargo run -q -p benchmark --bin bench -- seed --sut ehrbase-rs --scale "$SCALE" \
  --auth basic:ehrbase:ehrbase --admin-auth basic:ehrbase-admin:ehrbase

echo "==> resetting pg_stat_statements"
# Extension creation needs the superuser (dev compose: postgres/postgres).
docker exec ehrbase-rs-ehrbase-postgres-1 psql -U postgres -d ehrbase -Atc "CREATE EXTENSION IF NOT EXISTS pg_stat_statements" >/dev/null
docker exec ehrbase-rs-ehrbase-postgres-1 psql -U postgres -d ehrbase -Atc "SELECT pg_stat_statements_reset()" >/dev/null

echo "==> driving one capacity step L=${L} (${WINDOW}s)"
cargo run -q -p benchmark --bin bench -- knee --sut ehrbase-rs --scale "$SCALE" --no-seed \
  --steps "$L" --step-window "$WINDOW" --warmup 15 \
  --auth basic:ehrbase:ehrbase --admin-auth basic:ehrbase-admin:ehrbase \
  --out /tmp/p20-profile 2>&1 | grep -a "L=" || true

echo "==> writing $OUT"
{
  echo "# P20 statement profile — L=${L}, scale ${SCALE}, window ${WINDOW}s (${STAMP})"
  echo
  echo "Pool parity 50 · signing off · shed 256 (the benchmark parity config)."
  echo
  echo "## Top statements by total time"
  echo
  echo '```'
  PSQLF "SELECT round(total_exec_time::numeric,0) AS total_ms, calls, round(mean_exec_time::numeric,1) AS mean_ms, rows, left(regexp_replace(query, '\s+', ' ', 'g'), 140) AS statement FROM pg_stat_statements ORDER BY total_exec_time DESC LIMIT 25"
  echo '```'
  echo
  echo "## Wait profile (snapshot)"
  echo
  echo '```'
  PSQLF "SELECT wait_event_type, wait_event, count(*) FROM pg_stat_activity WHERE state='active' GROUP BY 1,2 ORDER BY 3 DESC"
  echo '```'
  echo
  echo "## AQL patient-dashboard plan (10k-shape, EXPLAIN ANALYZE)"
  echo
  echo '```'
  EHR=$(PSQL "SELECT id FROM ehr.ehr LIMIT 1" || true)
  PSQLF "EXPLAIN (ANALYZE, BUFFERS) SELECT n1.vo_id FROM ehr.node n1, ehr.vo_version v1 WHERE n1.rm_type='COMPOSITION' AND n1.vo_id=v1.vo_id AND n1.sys_version=v1.sys_version AND upper_inf(v1.sys_period) AND v1.branch_number=0 AND n1.ehr_id='${EHR}' LIMIT 20" 2>&1 || true
  echo '```'
} > "$OUT"

docker compose down -v >/dev/null 2>&1 || true
echo "==> profile written: $OUT"
