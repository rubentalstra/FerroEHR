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
#   CONF_SUT        ehrbase-rs (default) | ehrbase-java | byo.
#                   ehrbase-rs: builds + composes the root stack (the current
#                     sources — the phase-gate zero-drift run).
#                   ehrbase-java: composes upstream EHRbase (Java) from
#                     docker/sut-ehrbase-java.yml (official images, fresh
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
#   CONF_SIGNING_MODE
#                   pgp: run the ehrbase-rs SUT in openPGP version-signing mode
#                   (see below). Default digest.
#   SKIP_BUILD      if set, compose up without --build (published image).
#   SUT_USER/SUT_PASS, SUT_ADMIN_USER/SUT_ADMIN_PASS,
#   SUT_RO_USER/SUT_RO_PASS
#                   credentials the ixit references (defaults: the dev
#                   compose users; override for byo).
set -Eeuo pipefail

FILTER="${1:-}"
SUT="${CONF_SUT:-ehrbase-rs}"
SUT_NAME="${CONF_SUT_NAME:-$SUT}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${CONF_OUT:-$REPO_ROOT/docs/conformance}/$SUT_NAME"
ROOT="$REPO_ROOT/tools/cnf-runner/artifacts"
PARTY="$REPO_ROOT/tools/cnf-runner/party/$SUT_NAME"
IXIT="${CONF_IXIT:-$PARTY/ixit.json}"
STATEMENT="${CONF_STATEMENT:-$PARTY/statement.json}"

# CONF_SIGNING_MODE=pgp runs the ehrbase-rs SUT in openPGP version-signing mode
# (the dual-mode signature run): overlay the pgp compose variant (mounts the
# test OpenPGP secret key, sets EHRBASE__SIGNING__MODE=pgp) and use the pgp ixit
# (which carries the verifying public key for SIG-VERSION verifiable). Default:
# digest. Only meaningful for CONF_SUT=ehrbase-rs.
SIGNING_MODE="${CONF_SIGNING_MODE:-digest}"
# The ehrbase-rs SUT composes as its OWN project (docs.docker.com/compose/how-tos/project-name)
# so it coexists with — and never tears down — a running dev stack (which
# defaults to the directory-name project `ehrbase-rs`). Its `down -v` is thus
# scoped to `ehrbase-rs-cnf` only (issue #282 D3/F7).
# The conformance posture for ehrbase-rs is the SMART resource-server role
# (docker/sut-smart.yml): SMART on, fail-closed scopes, the committed static
# test issuer trusted — the strongest claimed posture, so the ONE committed
# record covers the whole claimed surface (SMART cases included) in one run.
# The ixit's principals mint scoped Bearer tokens; the scope-governed
# families are exercised exactly as a SMART Application would reach them.
EHRBASE_RS_COMPOSE=(docker compose -p ehrbase-rs-cnf -f "$REPO_ROOT/docker-compose.yml" -f "$REPO_ROOT/docker/sut-smart.yml")
if [ "$SIGNING_MODE" = "pgp" ] && [ "$SUT" = "ehrbase-rs" ]; then
  EHRBASE_RS_COMPOSE+=(-f "$REPO_ROOT/docker/sut-signing-pgp.yml")
  [ -n "${CONF_IXIT:-}" ] || IXIT="$PARTY/ixit.pgp.json"
fi


# Build provenance for compose-built images: the OCI-standard REVISION arg (the
# compose build.args block forwards it; the server Dockerfile bridges it into
# build.rs). Degrades to `unknown` (never a broken run) outside a git checkout.
export REVISION="${REVISION:-$(git -C "$REPO_ROOT" rev-parse --short=12 HEAD 2>/dev/null || echo unknown)}"

# The dev-compose credentials (docker/ehrbase.dev.toml); override for byo.
export SUT_USER="${SUT_USER:-ehrbase}"
export SUT_PASS="${SUT_PASS:-ehrbase}"
export SUT_ADMIN_USER="${SUT_ADMIN_USER:-ehrbase-admin}"
export SUT_ADMIN_PASS="${SUT_ADMIN_PASS:-ehrbase}"
export SUT_RO_USER="${SUT_RO_USER:-ehrbase-readonly}"
export SUT_RO_PASS="${SUT_RO_PASS:-ehrbase}"

[ -f "$IXIT" ] || { echo "conformance: ixit not found: $IXIT" >&2; exit 2; }
[ -f "$STATEMENT" ] || { echo "conformance: statement not found: $STATEMENT" >&2; exit 2; }

# byo: rewrite the ixit's base URLs into a temp copy.
if [ "$SUT" = "byo" ] && [ -n "${CONF_BASE_URL:-}" ]; then
  TMP_IXIT="$(mktemp -t cnf-ixit.XXXXXX)"
  python3 - "$IXIT" "$CONF_BASE_URL" "$TMP_IXIT" <<'PY'
import json
import sys

ixit = json.load(open(sys.argv[1]))
for inst in ixit["instances"].values():
    inst["base_url"] = sys.argv[2].rstrip("/")
json.dump(ixit, open(sys.argv[3], "w"), indent=2)
PY
  IXIT="$TMP_IXIT"
fi

if [ "$SUT" = "ehrbase-java" ]; then
  # The upstream image tag is the SUT version (default pinned in the compose).
  JAVA_IMAGE="${EHRBASE_JAVA_IMAGE:-ehrbase/ehrbase:2.34.0}"
  SUT_VERSION="${JAVA_IMAGE#*:}"
else
  SUT_VERSION="$(grep -m1 '^version' "$REPO_ROOT/Cargo.toml" | sed 's/.*"\(.*\)"/\1/')"
fi

# ehrbase-java composes as its own project so it can coexist with (and never
# tear down) the ehrbase-rs stack.
JAVA_COMPOSE=(docker compose -p cnf-ehrbase-java -f "$REPO_ROOT/docker/sut-ehrbase-java.yml")

compose_down() {
  if [ "$SUT" = "ehrbase-java" ]; then
    "${JAVA_COMPOSE[@]}" down -v || true
  else
    (cd "$REPO_ROOT" && "${EHRBASE_RS_COMPOSE[@]}" down -v) || true
  fi
}

if [ -z "${CONF_NO_COMPOSE:-}" ] && [ "$SUT" != "byo" ]; then
  trap compose_down EXIT
  echo "==> Composing $SUT on fresh volumes (the exclusive-server ground)"
  if [ "$SUT" = "ehrbase-java" ]; then
    "${JAVA_COMPOSE[@]}" down -v || true
    "${JAVA_COMPOSE[@]}" up -d
    # The official EHRbase image has no in-container health tooling (no
    # wget/curl), so poll the status endpoint EXTERNALLY: any HTTP answer
    # (200 with credentials, 401 without) means the server is serving.
    echo "==> Waiting for upstream EHRbase on :${EHRBASE_JAVA_PORT:-8091}"
    ready=""
    for _ in $(seq 1 60); do
      code=$(curl -s -o /dev/null -w '%{http_code}' \
        "http://localhost:${EHRBASE_JAVA_PORT:-8091}/ehrbase/rest/status" || true)
      case "$code" in
        200|401|403) ready=1; break ;;
      esac
      sleep 5
    done
    [ -n "$ready" ] || { echo "conformance: upstream EHRbase never became ready" >&2; exit 2; }
  else
    (cd "$REPO_ROOT" && "${EHRBASE_RS_COMPOSE[@]}" down -v) || true
    if [ -n "${SKIP_BUILD:-}" ]; then
      (cd "$REPO_ROOT" && "${EHRBASE_RS_COMPOSE[@]}" up -d --wait ehrbase)
    else
      # A conformance verdict on OUR server is only meaningful against the
      # CURRENT sources — build the image unless explicitly skipped.
      (cd "$REPO_ROOT" && "${EHRBASE_RS_COMPOSE[@]}" up -d --build --wait ehrbase)
    fi
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

echo "==> Computing the verdicts (pure pipeline)"
verdict_rc=0
"$REPO_ROOT/target/debug/cnf-runner" verdicts \
  --statement "$STATEMENT" --results "$OUT/results.json" \
  --root "$ROOT" --out "$OUT" || verdict_rc=$?
if [ "$verdict_rc" -ge 2 ]; then
  echo "conformance: verdict pipeline defect (exit $verdict_rc)" >&2
  exit "$verdict_rc"
fi

echo "==> Deriving the badges from verdicts.json"
python3 - "$OUT" "$ROOT/vocab/capability_matrix.yaml" <<'PY'
import json
import pathlib
import re
import sys

out = pathlib.Path(sys.argv[1])
verdicts = json.load(open(out / "verdicts.json"))
results = json.load(open(out / "results.json"))

# Tier membership from the capability matrix (Name: { tier: X, family: Y }).
tier_caps: dict[str, list[str]] = {}
family: dict[str, str] = {}
for line in open(sys.argv[2]):
    m = re.match(r"^(\w+):\s*\{(.*)\}", line)
    if not m:
        continue
    name, body = m.group(1), m.group(2)
    tier = re.search(r"tier:\s*([A-Z-]+)", body)
    fam = re.search(r"family:\s*(\w+)", body)
    if tier:
        tier_caps.setdefault(tier.group(1), []).append(name)
    if fam:
        family[name] = fam.group(1)

evidence = {name: ev for name, ev in verdicts["capabilities"]}
tiers = {tier: verdict for tier, verdict in verdicts["profiles"]}
security = verdicts.get("security")
if security is not None:
    tiers["SEC-BASIC"] = security if isinstance(security, str) else security.get("verdict", "")

def satisfied(names):
    ok = sum(1 for n in names if evidence.get(n) in ("passed", "unrealized"))
    return ok, len(names)

counts = {}
for tier, names in tier_caps.items():
    counts[tier] = satisfied(names)
counts["SEC-BASIC"] = satisfied([n for n, f in family.items() if f == "Security"])

colors = {"pass": "brightgreen", "fail": "red", "not_claimed": "lightgrey"}
slug = {"CORE": "core", "STANDARD": "standard", "OPTIONS": "options", "SEC-BASIC": "sec-basic"}
for tier, verdict in tiers.items():
    token = verdict if isinstance(verdict, str) else str(verdict)
    ok_n, total_n = counts.get(tier, (0, 0))
    amount = f" {ok_n}/{total_n}" if total_n else ""
    badge = {
        "schemaVersion": 1,
        "label": f"openEHR CNF {tier}",
        "message": f"{token.upper().replace('_', ' ')}{amount} capabilities" if total_n else f"{token.upper().replace('_', ' ')}",
        "color": colors.get(token, "lightgrey"),
    }
    path = out / f"badge-{slug.get(tier, tier.lower())}.json"
    path.write_text(json.dumps(badge, indent=2) + "\n")

# Performance badge — from the measured class verdicts (§8.14). The badge
# always NAMES the class it speaks about ("class POC earned" / "class POC
# not earned") — a bare verdict is meaningless without the volumetric
# class it was measured against — and an un-measured state writes "not
# measured" so no stale badge outlives its record.
perf = verdicts.get("performance") or []
ladder = {"POC": 0, "S": 1, "L": 2, "R": 3}
earned = [p["class"] for p in perf if p["verdict"] == "earned"]
measured = [p["class"] for p in perf]
if earned:
    best = max(earned, key=lambda c: ladder.get(c, -1))
    message, color = f"class {best} earned", "brightgreen"
elif measured:
    best = max(measured, key=lambda c: ladder.get(c, -1))
    message, color = f"class {best} not earned", "red"
else:
    message, color = "not measured", "lightgrey"
(out / "badge-performance.json").write_text(json.dumps({
    "schemaVersion": 1,
    "label": "openEHR CNF performance",
    "message": message,
    "color": color,
}, indent=2) + "\n")

by_status = {}
for o in results.get("outcomes", []):
    by_status[o["status"]] = by_status.get(o["status"], 0) + 1
driven = by_status.get("passed", 0) + by_status.get("failed", 0) + by_status.get("errored", 0)
ok = tiers.get("CORE") == "pass" and tiers.get("STANDARD") == "pass"
(out / "badge.json").write_text(json.dumps({
    "schemaVersion": 1,
    "label": "openEHR conformance",
    "message": (
        f"CORE+STANDARD PASS · {by_status.get('passed', 0)}/{driven} cases"
        if ok
        else f"NOT PASSING · {by_status.get('passed', 0)}/{driven} cases"
    ),
    "color": "brightgreen" if ok else "red",
}, indent=2) + "\n")
print("badges written")
PY

echo "==> Artefacts in $OUT"
ls -1 "$OUT"
