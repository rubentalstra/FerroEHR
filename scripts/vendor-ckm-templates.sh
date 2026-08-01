#!/usr/bin/env bash
# Vendor the official openEHR CKM template library as OPT 1.4 XML.
#
# Source: the public openEHR Clinical Knowledge Manager REST API
# (https://ckm.openehr.org/ckm/rest/v1). Every file is CKM's own
# Operational Template export, vendored verbatim with provenance.
#
# TWO PACKS, one script:
#
#   * the CURATED journey pack -> corpus/templates/ckm/<slug>.opt
#     Hand-picked, all COMPOSITION-rooted, each mapped to a role in the
#     measured-performance hospital-simulation journeys. The slugs are
#     REFERENCED BY NAME from corpus/MANIFEST.yaml, the journey definitions
#     and scripts/generate-ckm-examples.sh — never rename or drop one.
#
#   * the FULL library -> corpus/templates/ckm/full/<slug>.opt
#     Every template CKM publishes (slug derived from the display name),
#     for breadth gates over the OPT 1.4 reader / WebTemplate builder.
#
# CKM REST PAGINATION GOTCHA (cost an afternoon once — do not relearn it):
# the list endpoints page with `?page=N&size=M`. `limit`, `pageSize`,
# `maxResults`, `offset`, `count`, `rows` are all silently IGNORED and you
# get a 20-row first page, which reads exactly like "CKM only publishes 20
# templates". Always page with page/size, and assert the count grew.
#
# Some CKM resources live in a private incubator and 404 without an account;
# those are recorded as unreachable in the provenance file rather than
# silently skipped.
#
# Usage:
#   scripts/vendor-ckm-templates.sh              # curated pack + full library
#   scripts/vendor-ckm-templates.sh --curated    # curated pack only
#   scripts/vendor-ckm-templates.sh --full       # full library only
#   CKM_JOBS=8 scripts/vendor-ckm-templates.sh   # parallel fetches (default 4)
#
# Example skeletons (`*.example.json`) for the curated pack are generated
# separately against the composed SUT by scripts/generate-ckm-examples.sh.
set -Eeuo pipefail

CKM="https://ckm.openehr.org/ckm/rest/v1"
OUT="tools/cnf-runner/artifacts/corpus/templates/ckm"
FULL="$OUT/full"
JOBS="${CKM_JOBS:-4}"

# ── re-entrant single fetch (the xargs worker; not a user-facing mode) ────
if [[ "${1:-}" == "--fetch-one" ]]; then
  cid=$2
  dest=$3
  for attempt in 1 2 3; do
    if curl -fsS --max-time 240 \
        "$CKM/templates/$cid/opt" -H "Accept: application/xml" -o "$dest"; then
      if head -c 2048 "$dest" | grep -q "<template"; then
        echo "OK   $cid $dest"
        exit 0
      fi
      rm -f "$dest"
      echo "BAD  $cid $dest (response is not an OPT)"
      exit 0
    fi
    sleep $((attempt * 2))
  done
  rm -f "$dest"
  echo "FAIL $cid $dest"
  exit 0
fi

MODE="${1:-both}"
case "$MODE" in
  both | --both) MODE=both ;;
  --curated) MODE=curated ;;
  --full) MODE=full ;;
  *)
    echo "usage: $0 [--curated|--full]" >&2
    exit 2
    ;;
esac

# cid | slug | journey role — every entry COMPOSITION-rooted (committable
# as a composition; ENTRY/CLUSTER-rooted CKM "item" templates cannot carry
# a commit and are deliberately absent).
SET=(
  # ── monitoring streams ────────────────────────────────────────────────
  "1013.26.380|vital-signs|vitals_round (full observation round)"
  # NOTE: 1013.26.61 (ODL Report Vital Signs) is EXCLUDED: its OPT carries an
  # AOM defect (an assumed_value outside its constrained code list — AM 1.4
  # Assumed_value_valid), rejected by conformant AOM validation.
  # ── laboratory / imaging pipelines ────────────────────────────────────
  "1013.26.408|generic-lab-test-result|lab_pipeline (result contribution)"
  "1013.26.2|ereferral|lab_pipeline / imaging_pipeline (order)"
  "1013.26.386|ccta-report|imaging_pipeline (report)"
  # ── medication (the eMAR loop) ────────────────────────────────────────
  "1013.26.80|eprescription-fhir|medication_round (order + administrations)"
  "1013.26.357|medicines-list|medicines_reconciliation (ward-seeded, updated)"
  # ── encounter documents & summaries ───────────────────────────────────
  "1013.26.191|gp-data-set|correction target (ward-seeded, amended)"
  "1013.26.376|international-patient-summary|admission / discharge summary"
  "1013.26.360|problem-list|admission (problem list)"
  # ── specialist & registry reporting ───────────────────────────────────
  "1013.26.199|bc-breast-cancer-report|specialist_report (cancer synoptic report)"
  "1013.26.40|treat-registry-report|registry_submission (registry export)"
  # ── public-health surveillance ────────────────────────────────────────
  "1013.26.377|sars-event-notification|public_health_notification (statutory notification)"
  "1013.26.282|covid19-infection-report|public_health_notification (confirmed-case follow-up)"
  "1013.26.988|poisoning-case-investigation|case_investigation"
  "1013.26.980|diphtheria-case-investigation|case_investigation"
)

mkdir -p "$OUT"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

STAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# ── the curated journey pack ──────────────────────────────────────────────
PROV="$OUT/PROVENANCE.md"
if [[ "$MODE" != full ]]; then
  {
    echo "# CKM template pack — provenance"
    echo
    echo "Vendored from the official openEHR CKM (\`$CKM\`) by"
    echo "\`scripts/vendor-ckm-templates.sh\` on $STAMP."
    echo "Each file is CKM's own OPT export for the cited template, verbatim."
    echo "Example skeletons (\`*.example.json\`) are generated once against the"
    echo "composed SUT by \`scripts/generate-ckm-examples.sh\` and committed"
    echo "(byte-identical payload ground for every SUT; never fetched at run"
    echo "time). Manifest entries: \`tools/cnf-runner/artifacts/corpus/MANIFEST.yaml\`."
    echo
    echo "The **curated journey pack** below is referenced by slug from the"
    echo "manifest, the journey definitions and the example generator — the"
    echo "slugs are a stable contract. The **full library** is a separate pack"
    echo "under \`full/\` with its own provenance file."
    echo
    echo "| cid | slug | display name | status | modified | journey role |"
    echo "|---|---|---|---|---|---|"
  } > "$PROV"

  for entry in "${SET[@]}"; do
    IFS='|' read -r cid slug role <<< "$entry"
    echo "==> curated $cid ($slug)"
    meta=$(curl -fsS "$CKM/templates/$cid" -H "Accept: application/json")
    # one field per call: display names contain spaces AND pipes, so a
    # word-split read of a combined line is not safe here
    name=$(echo "$meta" | python3 -c 'import json,sys; print(json.load(sys.stdin)["resourceMainDisplayName"].replace("|","/"))')
    status=$(echo "$meta" | python3 -c 'import json,sys; print(json.load(sys.stdin)["status"])')
    modified=$(echo "$meta" | python3 -c 'import json,sys; print(json.load(sys.stdin)["modificationTime"])')
    bash "$0" --fetch-one "$cid" "$OUT/$slug.opt" | tee -a "$WORK/curated.log"
    grep -q "^OK   $cid " "$WORK/curated.log" || {
      echo "::error::$cid ($slug) did not yield an OPT — the curated pack is a contract" >&2
      exit 1
    }
    echo "| $cid | $slug | $name | $status | $modified | $role |" >> "$PROV"
  done
  echo "==> curated pack: $(grep -c '^OK' "$WORK/curated.log") OPTs → $OUT"
fi

# ── the full library ─────────────────────────────────────────────────────
if [[ "$MODE" != curated ]]; then
  mkdir -p "$FULL"
  echo "==> listing the full CKM template library (page/size pagination)"
  curl -fsS "$CKM/templates?page=0&size=10000" -H "Accept: application/json" \
    -o "$WORK/templates.json"

  python3 - "$WORK/templates.json" "$WORK/jobs.txt" "$WORK/rows.tsv" "$FULL" <<'PY'
import collections
import json
import re
import sys

src, jobs_path, rows_path, out_dir = sys.argv[1:5]
templates = json.load(open(src))
if len(templates) <= 20:
    raise SystemExit(
        f"::error::the list endpoint returned only {len(templates)} rows — "
        "CKM ignored the pagination parameters (use ?page=N&size=M)"
    )

seen = collections.Counter()
jobs, rows = [], []
for t in sorted(templates, key=lambda x: x["cid"]):
    name = t["resourceMainDisplayName"]
    slug = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")[:80] or "template"
    seen[slug] += 1
    if seen[slug] > 1:
        slug = f"{slug}-{seen[slug]}"
    jobs.append(f"{t['cid']} {out_dir}/{slug}.opt")
    rows.append(
        "\t".join(
            (
                t["cid"],
                slug,
                name.replace("|", "/"),
                t["status"],
                t["modificationTime"],
                str(t.get("versionAssetLatest", "")),
            )
        )
    )

open(jobs_path, "w").write("\n".join(jobs) + "\n")
open(rows_path, "w").write("\n".join(rows) + "\n")
print(f"==> {len(jobs)} templates published by CKM")
PY

  # fetch everything; a per-file failure is recorded, never fatal (private
  # incubator resources 404 without a CKM account)
  find "$FULL" -name '*.opt' -delete
  xargs -P "$JOBS" -n 2 bash "$0" --fetch-one < "$WORK/jobs.txt" \
    | tee "$WORK/full.log"

  python3 - "$WORK/rows.tsv" "$WORK/full.log" "$FULL/PROVENANCE.md" "$STAMP" "$CKM" <<'PY'
import sys

rows_path, log_path, prov_path, stamp, ckm = sys.argv[1:6]
outcome = {}
for line in open(log_path):
    parts = line.split()
    if len(parts) >= 2 and parts[0] in {"OK", "BAD", "FAIL"}:
        outcome[parts[1]] = parts[0]

rows = [line.rstrip("\n").split("\t") for line in open(rows_path) if line.strip()]
ok = [r for r in rows if outcome.get(r[0]) == "OK"]
bad = [r for r in rows if outcome.get(r[0]) in {"BAD", "FAIL"}]

with open(prov_path, "w") as fh:
    w = fh.write
    w("# CKM template library (full pack) — provenance\n\n")
    w(f"Every template the official openEHR CKM (`{ckm}`) publishes, exported\n")
    w("by CKM itself as an Operational Template and vendored verbatim by\n")
    w(f"`scripts/vendor-ckm-templates.sh` on {stamp}.\n\n")
    w("This is the BREADTH pack: real-world OPT 1.4 shapes for the reader /\n")
    w("WebTemplate builder gates. The curated hospital-simulation journey\n")
    w("pack is the parent directory (its own `PROVENANCE.md`); the slugs here\n")
    w("are derived from CKM display names and are NOT a naming contract.\n\n")
    w(f"- published by CKM: **{len(rows)}**\n")
    w(f"- vendored: **{len(ok)}**\n")
    w(f"- unreachable: **{len(bad)}**\n\n")
    if bad:
        w("## Unreachable (recorded, not skipped)\n\n")
        w("CKM answers 404 for resources held in a private incubator; they are\n")
        w("only exportable by a signed-in account with access.\n\n")
        w("| cid | display name | status |\n|---|---|---|\n")
        for cid, _slug, name, status, *_ in bad:
            w(f"| {cid} | {name} | {status} |\n")
        w("\n")
    w("## Vendored\n\n")
    w("| cid | file | display name | status | modified | asset version |\n")
    w("|---|---|---|---|---|---|\n")
    for cid, slug, name, status, modified, version in ok:
        w(f"| {cid} | `{slug}.opt` | {name} | {status} | {modified} | {version} |\n")

print(f"==> full library: {len(ok)} vendored, {len(bad)} unreachable → {prov_path}")
if bad:
    print("    unreachable: " + ", ".join(r[0] for r in bad))
PY
fi
