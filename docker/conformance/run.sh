#!/usr/bin/env bash
# Run the openEHR CNF conformance suite against a containerised server and write a
# per-edition report set under docs/conformance/<edition>/.
#
#   ./run.sh rs      # ehrbase-rs   (this project)      → docs/conformance/rs/
#   ./run.sh java    # EHRbase Java (reference impl)    → docs/conformance/java/
#
# It brings the chosen stack up (app + its PostgreSQL) on a fixed port, waits for
# the server to answer HTTP, runs the *same* 322-case schedule over `--base-url`
# with Basic auth, then tears the stack down. The CNF framework is server-agnostic
# — identical suite, two SUTs — which is exactly what makes CNF_COMPARISON.md a
# fair head-to-head.
set -euo pipefail

EDITION="${1:-}"
case "$EDITION" in
  rs)   PORT=8090; PROFILE=rs;   ADMIN_USER=ehrbase ;;
  java) PORT=8091; PROFILE=java; ADMIN_USER=ehrbase-admin ;;
  *) echo "usage: $0 <rs|java>" >&2; exit 2 ;;
esac

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
BASE="http://localhost:${PORT}/ehrbase/rest/openehr/v1"
OUT="docs/conformance/${EDITION}"

cd "$HERE"
echo ">> [$EDITION] starting stack (docker compose --profile $PROFILE up -d)…"
docker compose --profile "$PROFILE" up -d --wait 2>/dev/null || docker compose --profile "$PROFILE" up -d

echo ">> [$EDITION] waiting for the server at $BASE …"
up=0
for _ in $(seq 1 120); do
  code="$(curl -s -o /dev/null -w '%{http_code}' -u ehrbase:ehrbase \
    -X POST -H 'content-type: application/json' "$BASE/ehr" || true)"
  if [ "$code" != "000" ]; then echo "   server up (HTTP $code)"; up=1; break; fi
  sleep 3
done
if [ "$up" != "1" ]; then
  echo "!! [$EDITION] server did not come up in time" >&2
  docker compose --profile "$PROFILE" logs --tail 40 || true
  docker compose --profile "$PROFILE" down; exit 1
fi

echo ">> [$EDITION] running the CNF suite → $OUT …"
cd "$ROOT"
cargo run -p conformance --bin conformance -- \
  run --base-url "$BASE" \
  --auth "basic:ehrbase:ehrbase" \
  --admin-auth "basic:${ADMIN_USER}:ehrbase" \
  --out "$OUT" || true   # exit 1 = there are findings; the report is still written

echo ">> [$EDITION] tearing down…"
cd "$HERE"
docker compose --profile "$PROFILE" down

echo ">> [$EDITION] done. Report: $ROOT/$OUT/RESULTS.md"
