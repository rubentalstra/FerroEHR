#!/usr/bin/env bash
# Regenerate the published conformance visuals (the capability heat grid +
# the per-chapter outcome bars) FROM the committed party artifacts — the
# render-perf-assets.sh pattern for functional conformance: deterministic,
# CI regenerate-and-diff guarded, light+dark, numbers rendered never
# hand-typed.
#
#   CONF_SUT=ehrbase-rs  (default)  → website/book/src/conformance-assets/
#                                     + the landing page's committed copy
#   CONF_SUT=ehrbase-java           → website/book/src/comparison-assets/
#                                     (file stems suffixed -java)
set -euo pipefail
cd "$(dirname "$0")/.."

SUT="${CONF_SUT:-ehrbase-rs}"
ART="docs/conformance/$SUT"
for f in "$ART/results.json" "$ART/verdicts.json"; do
  [ -f "$f" ] || {
    echo "render-conformance-assets: $f missing — run the conformance suite first" >&2
    exit 1
  }
done

case "$SUT" in
ehrbase-rs)
  OUT="website/book/src/conformance-assets"
  SUFFIX=""
  ;;
ehrbase-java)
  OUT="website/book/src/comparison-assets"
  SUFFIX="-java"
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

# The landing page embeds the product's heat grid (a committed copy of the
# same generated output; the stale-numbers guard exempts *.svg because its
# labels are rendered, never hand-typed).
if [ "$SUT" = "ehrbase-rs" ]; then
  cp "$OUT/conformance-heat-grid.svg" website/landing/assets/conformance-heat-grid.svg
  echo "wrote website/landing/assets/conformance-heat-grid.svg"
fi
