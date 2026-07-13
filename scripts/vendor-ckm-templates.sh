#!/usr/bin/env bash
# Vendor official openEHR CKM templates (as OPT 1.4 XML) for the benchmark's
# hospital-day workload (docs/design/benchmark/00-workload-model.md §4).
#
# Source: the public openEHR Clinical Knowledge Manager REST API
# (https://ckm.openehr.org/ckm/rest/v1). Each template is exported as an
# Operational Template by CKM itself; we vendor the response verbatim with
# provenance. Re-run to refresh; the curated set below is the workload's
# template pack (event-class mapping in the comment per entry).
set -Eeuo pipefail

CKM="https://ckm.openehr.org/ckm/rest/v1"
OUT="tools/benchmark/templates/ckm"
mkdir -p "$OUT"

# cid | slug | workload role
SET=(
  "1013.26.380|vital-signs|E2 shift observations (small event composition)"
  "1013.26.408|generic-lab-test-result|E4 lab results (contribution batches)"
  "1013.26.80|eprescription-fhir|E3 medication rounds (ePrescription, COMPOSITION-rooted)"
  "1013.26.2|ereferral|E1 admission / E9 discharge (large clinical summary, COMPOSITION-rooted)"
  "1013.26.376|international-patient-summary|vendored, NOT wired: the server example/validator mismatch on ACTION.medication description is W-12"
  "1013.26.191|gp-data-set|E7 documentation corrections (GP encounter data set, COMPOSITION-rooted)"
)

PROV="$OUT/PROVENANCE.md"
{
  echo "# CKM template pack — provenance"
  echo
  echo "Vendored from the official openEHR CKM (\`$CKM\`) by"
  echo "\`scripts/vendor-ckm-templates.sh\` on $(date -u +%Y-%m-%dT%H:%M:%SZ)."
  echo "Each file is CKM's own OPT export for the cited template, verbatim."
  echo
  echo "| cid | slug | display name | status | modified | workload role |"
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
