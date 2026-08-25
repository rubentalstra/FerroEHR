#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# Regenerate the published conformance visuals (the capability heat grid +
# the per-chapter outcome bars) FROM the committed party artifacts — the
# render-perf-assets.sh pattern for functional conformance: deterministic,
# CI regenerate-and-diff guarded, light+dark, numbers rendered never
# hand-typed.
#
#   CONF_SUT=ferroehr  (default)  → website/book/src/conformance-assets/
#                                     + the landing page's committed copy
#   CONF_SUT=ehrbase           → website/book/src/comparison-assets/
#                                     (file stems suffixed -java)
set -euo pipefail
cd "$(dirname "$0")/../.."

SUT="${CONF_SUT:-ferroehr}"
ART="docs/conformance/$SUT"
for f in "$ART/results.json" "$ART/verdicts.json"; do
  [[ -f "$f" ]] || {
    echo "render-conformance-assets: $f missing — run the conformance suite first" >&2
    exit 1
  }
done

case "$SUT" in
ferroehr)
  OUT="website/book/src/conformance-assets"
  SUFFIX=""
  ;;
ehrbase)
  OUT="website/book/src/comparison-assets"
  SUFFIX="-ehrbase"
  ;;
*)
  OUT="docs/conformance/$SUT/conformance-assets"
  SUFFIX=""
  ;;
esac

cargo run -q -p cnf-runner -- conformance-assets \
  --root tools/cnf-runner/artifacts \
  --results "$ART/results.json" \
  --verdicts "$ART/verdicts.json" \
  --out "$OUT" \
  --suffix="$SUFFIX"

# The landing page embeds the product's heat grid straight from the book's
# committed conformance-assets copy (served at /docs/latest/) — no second
# committed copy exists.
