#!/usr/bin/env bash
# The stale-numbers gate (CI, docs workflow): fails the build if a hand-typed
# conformance claim appears in the site SOURCES. Conformance numbers and
# verdicts on the website are derived at build time from the committed runner
# artifacts by scripts/render-conformance-stats.sh — sources must carry only
# the data-ecc markers / {{#include}} directives, never literal numbers.
set -euo pipefail
cd "$(dirname "$0")/.."

# Two claim shapes are forbidden in website/landing + website/book/src:
#   1. a 1-4 digit number adjacent to conformance-count vocabulary
#   2. a spelled-out profile verdict (CORE/STANDARD/OPTIONS with PASS/FAIL/OBTAINED)
# website/book/generated/ is the render OUTPUT and is exempt (and gitignored).
# Digit-anchored: a number immediately (allowing intervening HTML tags)
# followed by conformance-count vocabulary. Digit-less mechanism prose
# ("every executed case passed") is deliberately NOT matched.
pattern_counts='[0-9]{1,4}[[:space:]]*(<[^>]*>)*[[:space:]]*(cases?([- ]by[- ]format)?|executions?|passed|failed|skipped|executed|documented skips)([^_a-zA-Z]|$)'
pattern_verdicts='(CORE|STANDARD|OPTIONS|Core|Standard|Options)[[:space:]]*[:·][[:space:]]*(PASS|FAIL|OBTAINED|Pass|Fail|Obtained)'

fail=0
for pattern in "$pattern_counts" "$pattern_verdicts"; do
  if hits=$(grep -rInE "$pattern" website/landing website/book/src 2>/dev/null); then
    echo "$hits"
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "::error::Hand-typed conformance numbers/verdicts found in site sources (above). The website derives conformance claims at build time from docs/conformance/ehrbase-rs/ — use the data-ecc markers (landing) or the generated include (book), never literals. See scripts/render-conformance-stats.sh." >&2
  exit 1
fi
echo "check-conformance-numbers: no hand-typed conformance claims in site sources."
