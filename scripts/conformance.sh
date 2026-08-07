#!/usr/bin/env bash
# openEHR CNF 2.0 conformance pipeline — the acceptance instrument.
#
# Drives the CNF reference runner (tools/cnf-runner) end to end: bring up the
# SUT's compose stack on FRESH volumes (the exclusive-server ground), execute
# the committed catalogue, compute the verdicts through the pure pipeline,
# and write the per-SUT artefact set under $CONF_OUT/<sut-name>/:
#
#   results.json               the party results (§8.10 schema)
#   run-exceptions.json        the interpreter-coverage exception register
#   verdicts.json              the computed verdict report
#   CONFORMANCE_REPORT.md      rendered from (results, verdicts)
#   CONFORMANCE_STATEMENT.md   rendered from (statement, verdicts)
#   CONFORMANCE_CERTIFICATE.md rendered per the CNF certificate book
#   badge*.json                shields.io endpoints derived from verdicts.json
#
# Usage:
#   scripts/conformance.sh [FILTER]
#
# FILTER (optional) is passed to the runner's --filter (an id substring).
#
# Env:
#   CONF_SUT        ferroehr (default) | ehrbase | byo.
#                   ferroehr: builds + composes the self-contained SUT stack
#                     docker/sut-ferroehr.yml (the current sources — the
#                     phase-gate zero-drift run), PLUS a second
#                     deployment of the same image in the openPGP
#                     version-signing posture on its own project/ports, so
#                     both claimed signing modes are exercised in the one
#                     record (the ixit's `sut_pgp` instance).
#                   ehrbase: composes upstream EHRbase from
#                     docker/sut-ehrbase.yml (official images, fresh
#                     volumes, host port 8091) — the #232 comparison target
#                     with its committed party set.
#                   byo: no compose management — point CONF_BASE_URL at any
#                     deployed CDR and supply CONF_IXIT/CONF_STATEMENT.
#   CONF_BASE_URL   SUT base URL for byo (rewrites the ixit instances).
#   CONF_SUT_NAME   output name (default: $CONF_SUT).
#   CONF_IXIT       ixit topology file (default: the committed per-SUT one).
#   CONF_STATEMENT  party statement    (default: the committed per-SUT one).
#   CONF_OUT        artefact root      (default: docs/conformance; the SUT
#                   name is appended).
#   CONF_NO_COMPOSE if set, do not manage compose (assume the SUT is up;
#                   NOTE: the exclusive-server cases then run against
#                   whatever state the SUT holds).
#   CONF_PERF_CLASS if set (POC|S|L|R), run the measured performance stage
#                   between run and verdicts (seeds the scale corpus, holds
#                   the class's offered load for the sustained window,
#                   merges the record into results.json). Hour-plus act on
#                   the exclusive composed SUT.
#   CONF_PERF_HOURS sustained-window ladder for the perf stage:
#                   1 (default) | 2 | 4 | 6 | 8 | 12 — longer holds are
#                   stricter demonstrations; nothing shorter exists.
#   SKIP_BUILD      if set, compose up without --build (published image).
#   SUT_USER/SUT_PASS, SUT_ADMIN_USER/SUT_ADMIN_PASS,
#   SUT_RO_USER/SUT_RO_PASS
#                   credentials the ixit references (defaults: the dev
#                   compose users; override for byo).
set -Eeuo pipefail

FILTER="${1:-}"
SUT="${CONF_SUT:-ferroehr}"
SUT_NAME="${CONF_SUT_NAME:-$SUT}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${CONF_OUT:-$REPO_ROOT/docs/conformance}/$SUT_NAME"
ROOT="$REPO_ROOT/tools/cnf-runner/artifacts"
PARTY="$REPO_ROOT/tools/cnf-runner/party/$SUT_NAME"
IXIT="${CONF_IXIT:-$PARTY/ixit.json}"
STATEMENT="${CONF_STATEMENT:-$PARTY/statement.json}"

# The SUT stack is docker/sut-ferroehr.yml — SELF-CONTAINED, like the
# ehrbase lane's docker/sut-ehrbase.yml. The root docker-compose.yml
# is the END-USER quickstart (published images, inline config) and is not part
# of any conformance lane. All paths inside the sut-*.yml files are
# repo-root-relative, hence `--project-directory "$REPO_ROOT"`
# (docs.docker.com/reference/cli/docker/compose).
# The ferroehr SUT composes as its OWN project (docs.docker.com/compose/how-tos/project-name)
# so it coexists with — and never tears down — a running dev stack (which
# defaults to the directory-name project `ferroehr`). Its `down -v` is thus
# scoped to `ferroehr-cnf` only (issue #282 D3/F7).
# The conformance posture for ferroehr is the SMART resource-server role
# (docker/sut-smart.yml): SMART on, fail-closed scopes, the committed static
# test issuer trusted — the strongest claimed posture, so the ONE committed
# record covers the whole claimed surface (SMART cases included) in one run.
# The ixit's principals mint scoped Bearer tokens; the scope-governed
# families are exercised exactly as a SMART Application would reach them.
# The `terminology` profile is part of that posture too: openEHR resolves an
# archetype constraint binding in an external "terminology query server" (BASE
# docs/architecture_overview/master12-terminology.adoc §"Binding Terminology
# Value-sets to Archetypes"), so the ONE committed record only covers the
# terminology-routed surface — AQL TERMINOLOGY() and commit-time binding
# validation — when a real FHIR R4 server is composed and seeded beside the
# CDR. docker/sut-terminology.yml points the CDR at it in the fail-OPEN
# posture; the ixit declares the resulting servers/namespaces/posture.
FERROEHR_RS_COMPOSE=(docker compose -p ferroehr-cnf \
  --project-directory "$REPO_ROOT" --profile terminology \
  -f "$REPO_ROOT/docker/sut-ferroehr.yml" \
  -f "$REPO_ROOT/docker/sut-smart.yml" \
  -f "$REPO_ROOT/docker/sut-terminology.yml")
# A measurement lane composes one more overlay: docker/sut-measurement.yml turns
# the rate limiter off for the run, because the instruments offer load past the
# server's knee on purpose and a 429 would measure the limiter instead of the
# server. Declared as a file rather than an exported variable so the posture of
# a recorded run is readable after the fact.
if [ -n "${CONF_PERF_CLASS:-}" ] || [ -n "${CONF_STRESS:-}" ]; then
  FERROEHR_RS_COMPOSE+=(-f "$REPO_ROOT/docker/sut-measurement.yml")
  echo "==> Measurement lane: composing docker/sut-measurement.yml (rate limiting off)"
fi
# BOTH claimed version-signing modes are exercised in the ONE record: a SECOND
# deployment of the SAME image runs the openPGP posture in its own compose
# project on remapped host ports (docker/sut-pgp-parallel.yml), and the ixit
# declares it as the `sut_pgp` instance with its own `signing` block. The pgp
# SIG-VERSION cases address that instance; everything else drives the primary.
# RM common master06 §Digital Signature defines digest and openPGP as
# alternative depths of ONE mechanism and a deployment realizes exactly one —
# so testing both claims means running both deployments, never merging two runs.
# That second deployment ALSO carries the second terminology posture: a
# running server realizes exactly one `fail_on_error` branch, and no released
# openEHR text decides between accepting and refusing a composition whose bound
# value set cannot be resolved (register AMB-172), so both claimed branches are
# tested by running both deployments — the same law the two signing modes
# follow. Postures are independent properties of a deployment, so this project
# carries the pgp signing posture AND the fail-closed terminology posture, and
# the ixit declares both on its `sut_pgp` instance.
FERROEHR_RS_PGP_COMPOSE=(docker compose -p ferroehr-cnf-pgp \
  --project-directory "$REPO_ROOT" \
  -f "$REPO_ROOT/docker/sut-ferroehr.yml" \
  -f "$REPO_ROOT/docker/sut-smart.yml" \
  -f "$REPO_ROOT/docker/sut-signing-pgp.yml" \
  -f "$REPO_ROOT/docker/sut-terminology-failclosed.yml" \
  -f "$REPO_ROOT/docker/sut-pgp-parallel.yml")


# Build provenance for compose-built images: the OCI-standard REVISION arg (the
# compose build.args block forwards it; the server Dockerfile bridges it into
# build.rs). Degrades to `unknown` (never a broken run) outside a git checkout.
export REVISION="${REVISION:-$(git -C "$REPO_ROOT" rev-parse --short=12 HEAD 2>/dev/null || echo unknown)}"

# The dev-compose credentials (docker/ferroehr.dev.toml); override for byo.
export SUT_USER="${SUT_USER:-ferroehr}"
export SUT_PASS="${SUT_PASS:-ferroehr}"
export SUT_ADMIN_USER="${SUT_ADMIN_USER:-ferroehr-admin}"
export SUT_ADMIN_PASS="${SUT_ADMIN_PASS:-ferroehr}"
export SUT_RO_USER="${SUT_RO_USER:-ferroehr-readonly}"
export SUT_RO_PASS="${SUT_RO_PASS:-ferroehr}"

[ -f "$IXIT" ] || { echo "conformance: ixit not found: $IXIT" >&2; exit 2; }
[ -f "$STATEMENT" ] || { echo "conformance: statement not found: $STATEMENT" >&2; exit 2; }

# byo: rewrite the ixit's base URLs into a temp copy.
if [ "$SUT" = "byo" ] && [ -n "${CONF_BASE_URL:-}" ]; then
  TMP_IXIT="$(mktemp -t cnf-ixit.XXXXXX)"
  # jq, not python: this is a two-key JSON edit, and jq is the tool for it.
  jq --arg url "${CONF_BASE_URL%/}" \
    '.instances |= map_values(.base_url = $url)' \
    "$IXIT" > "$TMP_IXIT"
  IXIT="$TMP_IXIT"
fi

if [ "$SUT" = "ehrbase" ]; then
  # The upstream image tag is the SUT version (default pinned in the compose).
  EHRBASE_IMAGE="${FERROEHR_EHRBASE_IMAGE:-ehrbase/ehrbase:2.34.0}"
  SUT_VERSION="${EHRBASE_IMAGE#*:}"
else
  SUT_VERSION="$(grep -m1 '^version' "$REPO_ROOT/Cargo.toml" | sed 's/.*"\(.*\)"/\1/')"
fi

# ehrbase composes as its own project so it can coexist with (and never
# tear down) the ferroehr stack.
EHRBASE_COMPOSE=(docker compose -p cnf-ehrbase -f "$REPO_ROOT/docker/sut-ehrbase.yml")

compose_down() {
  if [ "$SUT" = "ehrbase" ]; then
    "${EHRBASE_COMPOSE[@]}" down -v || true
  else
    (cd "$REPO_ROOT" && "${FERROEHR_RS_COMPOSE[@]}" down -v) || true
    (cd "$REPO_ROOT" && "${FERROEHR_RS_PGP_COMPOSE[@]}" down -v) || true
  fi
}

if [ -z "${CONF_NO_COMPOSE:-}" ] && [ "$SUT" != "byo" ]; then
  trap compose_down EXIT
  echo "==> Composing $SUT on fresh volumes (the exclusive-server ground)"
  if [ "$SUT" = "ehrbase" ]; then
    "${EHRBASE_COMPOSE[@]}" down -v || true
    "${EHRBASE_COMPOSE[@]}" up -d
    # The official EHRbase image has no in-container health tooling (no
    # wget/curl), so poll the status endpoint EXTERNALLY: any HTTP answer
    # (200 with credentials, 401 without) means the server is serving.
    echo "==> Waiting for upstream EHRbase on :${FERROEHR_EHRBASE_PORT:-8091}"
    ready=""
    for _ in $(seq 1 60); do
      code=$(curl -s -o /dev/null -w '%{http_code}' \
        "http://localhost:${FERROEHR_EHRBASE_PORT:-8091}/ehrbase/rest/status" || true)
      case "$code" in
        200|401|403) ready=1; break ;;
      esac
      sleep 5
    done
    [ -n "$ready" ] || { echo "conformance: upstream EHRbase never became ready" >&2; exit 2; }
  else
    (cd "$REPO_ROOT" && "${FERROEHR_RS_COMPOSE[@]}" down -v) || true
    (cd "$REPO_ROOT" && "${FERROEHR_RS_PGP_COMPOSE[@]}" down -v) || true
    if [ -n "${SKIP_BUILD:-}" ]; then
      (cd "$REPO_ROOT" && "${FERROEHR_RS_COMPOSE[@]}" up -d --wait ferroehr)
    else
      # A conformance verdict on OUR server is only meaningful against the
      # CURRENT sources — build the image unless explicitly skipped.
      (cd "$REPO_ROOT" && "${FERROEHR_RS_COMPOSE[@]}" up -d --build --wait ferroehr)
    fi
    # The parallel pgp-posture deployment of the SAME image. NEVER --build:
    # docker/sut-ferroehr.yml pins explicit `image:` tags, which are project-
    # independent, so the artefact the primary just built (or, under
    # SKIP_BUILD, pulled) is the artefact this project starts — one build,
    # two postures, and no chance of the two records describing different code.
    # `up ferroehr` starts only that service and its depends_on (postgres).
    (cd "$REPO_ROOT" && "${FERROEHR_RS_PGP_COMPOSE[@]}" up -d --wait ferroehr)
  fi
fi

echo "==> Building the CNF runner"
(cd "$REPO_ROOT" && cargo build -q -p cnf-runner)

mkdir -p "$OUT"

echo "==> Executing the catalogue (sut=$SUT_NAME filter='${FILTER}')"
run_args=(run --root "$ROOT" --ixit "$IXIT" --out "$OUT"
          --sut-name "$SUT_NAME" --sut-version "$SUT_VERSION"
          --statement "$STATEMENT")
[ -n "$FILTER" ] && run_args+=(--filter "$FILTER")
# Exit 1 = failing cases (data for the verdict pipeline, not a pipeline
# abort); only 2 (runner defect) stops the run.
run_rc=0
"$REPO_ROOT/target/debug/cnf-runner" "${run_args[@]}" || run_rc=$?
if [ "$run_rc" -ge 2 ]; then
  echo "conformance: runner defect (exit $run_rc)" >&2
  exit "$run_rc"
fi

# Optional measured performance run (§8.14, conformance-by-measurement):
# CONF_PERF_CLASS=POC|S|L|R seeds the scale corpus into the freshly
# composed SUT (the workflow always seeds an empty database) and drives
# the class's open-loop sustained case, merging the measurement records
# into results.json BEFORE the verdict pipeline runs. This is an
# hour-plus act (5 min warmup + the sustained window, after corpus seeding)
# and needs the exclusive composed SUT — never on by default.
# CONF_PERF_HOURS=1|2|4|6|8|12 extends the sustained window (default 1 —
# the case's normative hour; longer holds are stricter demonstrations).
if [ -n "${CONF_PERF_CLASS:-}" ]; then
  # A measured record is environment-bound, and the parallel pgp deployment is
  # part of the environment while it is resident (a second server + database
  # holding their own CPU/memory limits). The functional catalogue is done with
  # it by now, and perf drives the primary alone, so tear it down BEFORE the
  # window opens — the measured envelope must be the one the ixit declares.
  if [ -z "${CONF_NO_COMPOSE:-}" ] && [ "$SUT" = "ferroehr" ]; then
    echo "==> Tearing down the parallel pgp deployment before the measured window"
    (cd "$REPO_ROOT" && "${FERROEHR_RS_PGP_COMPOSE[@]}" down -v) || true
  fi
  echo "==> Measured performance run (class $CONF_PERF_CLASS, ${CONF_PERF_HOURS:-1} h window)"
  perf_args=(perf --root "$ROOT" --ixit "$IXIT" --results "$OUT/results.json"
             --class "$CONF_PERF_CLASS" --hours "${CONF_PERF_HOURS:-1}")
  perf_rc=0
  "$REPO_ROOT/target/debug/cnf-runner" "${perf_args[@]}" || perf_rc=$?
  if [ "$perf_rc" -ge 2 ]; then
    echo "conformance: perf run defect (exit $perf_rc)" >&2
    exit "$perf_rc"
  fi
fi

# The verdicts AND the shields.io badges: the badge counts quantify over the
# same capability sets the tier verdicts do (`verdict::tier_members`), so there
# is no second derivation here to contradict them.
echo "==> Computing the verdicts + badges (pure pipeline)"
verdict_rc=0
"$REPO_ROOT/target/debug/cnf-runner" verdicts \
  --statement "$STATEMENT" --results "$OUT/results.json" \
  --root "$ROOT" --out "$OUT" || verdict_rc=$?
if [ "$verdict_rc" -ge 2 ]; then
  echo "conformance: verdict pipeline defect (exit $verdict_rc)" >&2
  exit "$verdict_rc"
fi

echo "==> Artefacts in $OUT"
ls -1 "$OUT"
