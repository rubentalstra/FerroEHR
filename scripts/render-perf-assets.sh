#!/usr/bin/env bash
# Render the published performance SVG assets FROM the committed measurement
# records (docs/conformance/ehrbase-rs/results.json measurements block) via
# `cnf-runner perf-assets`. Deterministic by construction: the docs CI job
# re-runs this script and `git diff --exit-code`s the output, so a hand-drawn
# or stale asset fails the build (the same honesty rule as
# check-conformance-numbers.sh — published numbers derive from committed
# artifacts, never hands).
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="${1:-website/book/src/perf-assets}"
SUMMARY="${2:-website/book/generated/perf-summary.md}"

cargo run -q -p cnf-runner -- perf-assets \
  --root tools/cnf-runner/artifacts \
  --results docs/conformance/ehrbase-rs/results.json \
  --out "$OUT" \
  --summary "$SUMMARY"
