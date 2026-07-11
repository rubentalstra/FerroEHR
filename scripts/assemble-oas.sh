#!/usr/bin/env bash
# Copy the 7 documentation OAS bundles into the served copy; --check fails on drift.
#
# The served specs are a byte copy of the vendored `-html` ITS-REST bundles
# (which the codegen-drift gate already ties to upstream), so drift is caught at
# two hops: upstream -> vendor (existing gate) -> website/api/spec (this gate).
#
# NOTE(portability): the phase file §5a expresses the source->dest name map as a
# bash-4 associative array. Every entry is the identity (`ehr`->`ehr`, ...), so
# this uses a plain word-list loop instead — behaviourally identical and portable
# to any POSIX shell. Edit OAS_GROUPS to change the served set.
set -euo pipefail
cd "$(dirname "$0")/.."
SRC="crates/openehr-its/vendor/rest-oas"
DST="website/api/spec"
OAS_GROUPS="ehr definition query demographic admin system overview"
mkdir -p "$DST"
for group in $OAS_GROUPS; do
  cp "$SRC/${group}-html.openapi.yaml" "$DST/${group}.openapi.yaml"
done
if [[ "${1:-}" == "--check" ]]; then
  if ! git diff --quiet -- "$DST"; then
    echo "::error::website/api/spec is out of sync with the vendored ITS-REST OAS. Run scripts/assemble-oas.sh and commit." >&2
    git diff --stat -- "$DST" >&2
    exit 1
  fi
  echo "✓ served OAS == vendored ITS-REST bundles."
fi
