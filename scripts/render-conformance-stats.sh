#!/usr/bin/env bash
# Derive every website conformance claim from the committed runner artifacts
# (docs/conformance/ehrbase-rs/) at BUILD time — the site sources carry no
# hand-typed conformance numbers (enforced by scripts/check-conformance-numbers.sh
# in CI), so the stale-numbers failure mode is impossible by construction.
#
#   render-conformance-stats.sh includes          write website/book/generated/*.md
#   render-conformance-stats.sh fill-html FILE    fill data-ecc markers in FILE in place
#
# Consumed by scripts/build-site.sh; runnable standalone for local previews.
set -euo pipefail
cd "$(dirname "$0")/.."

ART="docs/conformance/ehrbase-rs"
command -v jq >/dev/null 2>&1 || {
  echo "render-conformance-stats: jq is required (brew install jq / preinstalled on CI runners)" >&2
  exit 1
}
[ -f "$ART/results.json" ] || {
  echo "render-conformance-stats: $ART/results.json missing — run the conformance suite first" >&2
  exit 1
}

executed=$(jq '[.outcomes[] | select(.status != "not_applicable" and .status != "skipped")] | length' "$ART/results.json")
passed=$(jq '[.outcomes[] | select(.status == "passed")] | length' "$ART/results.json")
failed=$(jq '[.outcomes[] | select(.status == "failed" or .status == "errored")] | length' "$ART/results.json")
skipped=$(jq '[.outcomes[] | select(.status == "skipped" or .status == "not_applicable")] | length' "$ART/results.json")
# The results artifact is deliberately clock-free (deterministic re-runs);
# the run date is the artifact's last commit date (fallback: file mtime).
run_date=$(git log -1 --format=%cs -- "$ART/results.json" 2>/dev/null || true)
[ -n "$run_date" ] || run_date=$(date -r "$ART/results.json" +%Y-%m-%d)
# Verdicts come from the runner-generated badge JSONs (first word of message).
core=$(jq -r '.message' "$ART/badge-core.json" | awk '{print $1}')
standard=$(jq -r '.message' "$ART/badge-standard.json" | awk '{print $1}')
options=$(jq -r '.message' "$ART/badge-options.json" | awk '{print $1}')

# Sanity: refuse to render nonsense (a truncated artifact must fail the build).
for v in "$executed" "$passed" "$failed" "$skipped"; do
  [[ "$v" =~ ^[0-9]+$ ]] || { echo "render-conformance-stats: non-numeric stat '$v'" >&2; exit 1; }
done
for v in "$core" "$standard" "$options"; do
  [ -n "$v" ] || { echo "render-conformance-stats: empty verdict in badge JSON" >&2; exit 1; }
done

case "${1:-includes}" in
  includes)
    mkdir -p website/book/generated
    cat > website/book/generated/conformance-stats.md <<EOF
- **${executed} case-by-format executions, ${passed} passed, ${failed} failed,
  ${skipped} documented skips** (run of ${run_date}).
- **Core: ${core}. Standard: ${standard}. Options: ${options}.**
EOF
    echo "render-conformance-stats: wrote website/book/generated/conformance-stats.md (${passed}/${executed}, run ${run_date})"
    ;;
  fill-html)
    file="${2:?fill-html needs a file argument}"
    perl -pi -e "
      s/(data-ecc=\"executed\"[^>]*>)[^<]*/\${1}${executed}/g;
      s/(data-ecc=\"passed\"[^>]*>)[^<]*/\${1}${passed}/g;
      s/(data-ecc=\"failed\"[^>]*>)[^<]*/\${1}${failed}/g;
      s/(data-ecc=\"verdict-core\"[^>]*>)[^<]*/\${1}CORE · ${core}/g;
      s/(data-ecc=\"verdict-standard\"[^>]*>)[^<]*/\${1}STANDARD · ${standard}/g;
      s/(data-ecc=\"verdict-options\"[^>]*>)[^<]*/\${1}OPTIONS · ${options}/g;
    " "$file"
    # A marker that survived filling means the HTML and this script drifted.
    if grep -qE 'data-ecc="[^"]*"[^>]*>—<' "$file"; then
      echo "render-conformance-stats: unfilled data-ecc marker left in $file" >&2
      exit 1
    fi
    echo "render-conformance-stats: filled $file"
    ;;
  *)
    echo "usage: $0 [includes | fill-html FILE]" >&2
    exit 2
    ;;
esac
