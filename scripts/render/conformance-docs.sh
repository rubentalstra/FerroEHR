#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# Regenerate the DERIVED party documents (verdicts.json, the three
# CONFORMANCE_*.md renders, the shields.io badges) FROM the committed inputs —
# the party statement, that party's committed results.json, and the catalogue —
# via `cnf-runner verdicts`, the same pure pipeline scripts/conformance.sh runs
# at the end of a campaign.
#
# Why it exists as its own lane: the published SVGs are regenerate-and-diff
# guarded in CI while these documents were not, so a catalogue or statement
# change could leave a published statement/certificate silently stale (#2377).
# The docs CI job runs this script and `git diff --exit-code`s the party
# directories, which makes that impossible.
#
# Nothing here re-measures: results.json is an INPUT and is never written.
#
# Env:
#   CONF_SUT   which parties to regenerate (default: every party that has both
#              a committed statement and committed results)
set -euo pipefail
cd "$(dirname "$0")/../.."

if [ -n "${CONF_SUT:-}" ]; then
  parties=("$CONF_SUT")
else
  parties=()
  for statement in tools/cnf-runner/party/*/statement.json; do
    party="$(basename "$(dirname "$statement")")"
    [ -f "docs/conformance/$party/results.json" ] && parties+=("$party")
  done
fi

for party in "${parties[@]}"; do
  statement="tools/cnf-runner/party/$party/statement.json"
  results="docs/conformance/$party/results.json"
  for f in "$statement" "$results"; do
    [ -f "$f" ] || {
      echo "render-conformance-docs: $f missing — run the conformance suite first" >&2
      exit 1
    }
  done
  echo "==> $party"
  cargo run -q -p cnf-runner -- verdicts \
    --statement "$statement" \
    --results "$results" \
    --root tools/cnf-runner/artifacts \
    --out "docs/conformance/$party"
done
