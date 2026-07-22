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
#   CONF_SUT        ehrbase-rs (default) | byo.
#                   ehrbase-rs: builds + composes the root stack (the current
#                     sources — the phase-gate zero-drift run).
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

SUT_VERSION="$(grep -m1 '^version' "$REPO_ROOT/Cargo.toml" | sed 's/.*"\(.*\)"/\1/')"

compose_down() {
  (cd "$REPO_ROOT" && docker compose down -v) || true
}

if [ -z "${CONF_NO_COMPOSE:-}" ] && [ "$SUT" != "byo" ]; then
  trap compose_down EXIT
  echo "==> Composing $SUT on fresh volumes (the exclusive-server ground)"
  (cd "$REPO_ROOT" && docker compose down -v) || true
  if [ -n "${SKIP_BUILD:-}" ]; then
    (cd "$REPO_ROOT" && docker compose up -d --wait ehrbase)
  else
    # A conformance verdict on OUR server is only meaningful against the
    # CURRENT sources — build the image unless explicitly skipped.
    (cd "$REPO_ROOT" && docker compose up -d --build --wait ehrbase)
  fi
fi

echo "==> Building the CNF runner"
(cd "$REPO_ROOT" && cargo build -q -p cnf-runner)

mkdir -p "$OUT"

echo "==> Executing the catalogue (sut=$SUT_NAME filter='${FILTER}')"
run_args=(run --root "$ROOT" --ixit "$IXIT" --out "$OUT"
          --sut-name "$SUT_NAME" --sut-version "$SUT_VERSION")
[ -n "$FILTER" ] && run_args+=(--filter "$FILTER")
# Exit 1 = failing cases (data for the verdict pipeline, not a pipeline
# abort); only 2 (runner defect) stops the run.
run_rc=0
"$REPO_ROOT/target/debug/cnf-runner" "${run_args[@]}" || run_rc=$?
if [ "$run_rc" -ge 2 ]; then
  echo "conformance: runner defect (exit $run_rc)" >&2
  exit "$run_rc"
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
python3 - "$OUT" <<'PY'
import json
import pathlib
import sys

out = pathlib.Path(sys.argv[1])
verdicts = json.load(open(out / "verdicts.json"))
tiers = {tier: verdict for tier, verdict in verdicts["profiles"]}
security = verdicts.get("security")
if security is not None:
    tiers["SEC-BASIC"] = security if isinstance(security, str) else security.get("verdict", "")
colors = {"Pass": "brightgreen", "Fail": "red", "NotClaimed": "lightgrey"}
slug = {"CORE": "core", "STANDARD": "standard", "OPTIONS": "options", "SEC-BASIC": "sec-basic"}
for tier, verdict in tiers.items():
    token = verdict if isinstance(verdict, str) else str(verdict)
    badge = {
        "schemaVersion": 1,
        "label": f"openEHR CNF {tier}",
        "message": f"{token.upper()} (CNF 2.0)",
        "color": colors.get(token, "lightgrey"),
    }
    path = out / f"badge-{slug.get(tier, tier.lower())}.json"
    path.write_text(json.dumps(badge, indent=2) + "\n")
ok = tiers.get("CORE") == "Pass" and tiers.get("STANDARD") == "Pass"
(out / "badge.json").write_text(json.dumps({
    "schemaVersion": 1,
    "label": "openEHR conformance",
    "message": "CORE+STANDARD PASS (CNF 2.0)" if ok else "NOT PASSING (CNF 2.0)",
    "color": "brightgreen" if ok else "red",
}, indent=2) + "\n")
print("badges written")
PY

echo "==> Artefacts in $OUT"
ls -1 "$OUT"
