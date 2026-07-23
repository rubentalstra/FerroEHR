#!/usr/bin/env bash
# Vendor official openEHR CKM templates (as OPT 1.4 XML) for the CNF
# runner's hospital-simulation journey workload (the measured-performance
# catalogue's template pack).
#
# Source: the public openEHR Clinical Knowledge Manager REST API
# (https://ckm.openehr.org/ckm/rest/v1). Each template is exported as an
# Operational Template by CKM itself; we vendor the response verbatim with
# provenance. Re-run to refresh; the curated set below is the journey
# catalogue's template pack (journey mapping in the comment per entry).
# Example skeletons are generated separately against the composed SUT by
# scripts/generate-ckm-examples.sh (committed once, byte-identical for
# every SUT).
set -Eeuo pipefail

CKM="https://ckm.openehr.org/ckm/rest/v1"
OUT="tools/cnf-runner/artifacts/corpus/templates/ckm"
mkdir -p "$OUT"

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

PROV="$OUT/PROVENANCE.md"
{
  echo "# CKM template pack — provenance"
  echo
  echo "Vendored from the official openEHR CKM (\`$CKM\`) by"
  echo "\`scripts/vendor-ckm-templates.sh\` on $(date -u +%Y-%m-%dT%H:%M:%SZ)."
  echo "Each file is CKM's own OPT export for the cited template, verbatim."
  echo "Example skeletons (\`*.example.json\`) are generated once against the"
  echo "composed SUT by \`scripts/generate-ckm-examples.sh\` and committed"
  echo "(byte-identical payload ground for every SUT; never fetched at run"
  echo "time). Manifest entries: \`tools/cnf-runner/artifacts/corpus/MANIFEST.yaml\`."
  echo
  echo "| cid | slug | display name | status | modified | journey role |"
  echo "|---|---|---|---|---|---|"
} > "$PROV"

for entry in "${SET[@]}"; do
  IFS='|' read -r cid slug role <<< "$entry"
  echo "==> $cid ($slug)"
  meta=$(curl -fsS "$CKM/templates/$cid" -H "Accept: application/json")
  name=$(echo "$meta" | python3 -c "import json,sys; print(json.load(sys.stdin)['resourceMainDisplayName'])")
  status=$(echo "$meta" | python3 -c "import json,sys; print(json.load(sys.stdin)['status'])")
  modified=$(echo "$meta" | python3 -c "import json,sys; print(json.load(sys.stdin)['modificationTime'])")
  curl -fsS "$CKM/templates/$cid/opt" -H "Accept: application/xml" -o "$OUT/$slug.opt"
  # sanity: must be an OPT
  head -c 512 "$OUT/$slug.opt" | grep -q "<template" || { echo "::error::$cid did not return an OPT" >&2; exit 1; }
  echo "| $cid | $slug | $name | $status | $modified | $role |" >> "$PROV"
done

echo "==> vendored $(ls "$OUT"/*.opt | wc -l | tr -d ' ') OPTs → $OUT (provenance: $PROV)"
