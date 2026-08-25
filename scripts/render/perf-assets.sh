#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# Render the published performance SVG assets FROM the committed measurement
# records (docs/conformance/<sut>/results.json measurements block, plus the
# stress report when one exists) via `cnf-runner perf-assets`. Deterministic
# by construction: the docs CI job re-runs this script and
# `git diff --exit-code`s the output, so a hand-drawn or stale asset fails
# the build (the same honesty rule as check-conformance-numbers.sh).
#
# Env:
#   CONF_SUT   which SUT's committed artifacts to render (default ferroehr
#              — the product's assets, published into the book; any other
#              value renders beside that SUT's party artifacts).
# Args:
#   $1  output dir override
#   $2  Markdown summary path override
set -euo pipefail
cd "$(dirname "$0")/../.."

SUT="${CONF_SUT:-ferroehr}"
if [[ "$SUT" = "ferroehr" ]]; then
  OUT="${1:-website/book/src/perf-assets}"
  SUMMARY="${2:-website/book/generated/perf-summary.md}"
else
  OUT="${1:-docs/conformance/$SUT/perf-assets}"
  SUMMARY="${2:-docs/conformance/$SUT/PERF_SUMMARY.md}"
fi
RESULTS="docs/conformance/$SUT/results.json"
STRESS="docs/conformance/$SUT/stress.json"

args=(perf-assets
  --root tools/cnf-runner/artifacts
  --results "$RESULTS"
  --out "$OUT"
  --summary "$SUMMARY")
# The latency-throughput stress curve renders only once a committed stress
# report exists (cnf-runner stress — exploration, never a conformance record).
[[ -f "$STRESS" ]] && args+=(--stress "$STRESS")

cargo run -q -p cnf-runner -- "${args[@]}"
