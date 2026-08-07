#!/usr/bin/env bash
# Docs-claim guard: a documented KEY must exist in the thing it documents.
#
# The v3.17.4 book sweep found five substantive defects that had shipped to the
# published site — a fabricated Helm values table, a page asserting endpoints did
# not exist when six did, a `SPEC_VERSION` constant taught for crates that have
# none. None was caught by a gate: `mdbook-lint` checks style, `lychee` checks
# link reachability, and both pass happily on a page whose every technical claim
# is false. The same-PR docs rule depended entirely on an author noticing.
#
# This guard checks the claims that have a machine-readable authority to check
# against, and nothing else. Three of them:
#
#   1. Every `FERROEHR__…` environment form a page shows resolves to a real key
#      in `app/ferroehr/assets/ferroehr.default.toml`.
#   2. Every Helm values path a chart page shows resolves — against
#      `values.yaml`, or for the `config.*` subtree against the TOML schema,
#      because the chart renders `config:` VERBATIM to ferroehr.toml (so
#      `config.<any-toml-path>` is legitimate whether or not values.yaml
#      enumerates it, and the one non-TOML child, `config.files`, resolves in
#      values.yaml instead).
#   3. Every committed generated chart under a `*-assets/` directory is embedded
#      by some page. An unreferenced chart is either a page that lost its figure
#      or a renderer emitting output nobody reads.
#
# What is NOT checked, and why — an unenforced rule is labelled as such
# (`.claude/rules/reliability.md`):
#
#   * **Documented Rust paths.** A grep-level existence check for
#     `crate::a::b` / `openehr_x::y::Z` was measured and rejected: prose names
#     types that exist under a different module path than the sentence implies,
#     and the generated crates' generation modules mean one type has several
#     valid paths. It produced more false positives than findings. Review-only.
#   * **Prose claims of any kind** — "six endpoints exist", "the chart renders
#     one key" — have no machine authority. Review-only, and the reason the
#     same-PR docs rule still matters.
#
# Scope note on check 2: the values vocabulary is only in scope on pages that
# document the chart. `configuration.md` legitimately writes
# `` `service.name` `` for the OpenTelemetry resource attribute, which collides
# with the chart's top-level `service:` key and is not a values path at all.
# Widening the check to every page turns that into a false failure, so the check
# reads the chart pages and the chart pages only.
#
# Usage: scripts/checks/docs-claims.sh [--all | <file>...]
#   no args  → the book files changed against origin/develop
#   --all    → every tracked book page
set -euo pipefail
cd "$(dirname "$0")/../.."

BOOK=website/book/src
TOML=app/ferroehr/assets/ferroehr.default.toml
VALUES=deploy/helm/ferroehr/values.yaml
# The pages that document the Helm chart, and therefore the only pages on which a
# dotted token is read as a chart values path.
CHART_PAGES="$BOOK/installation/kubernetes.md $BOOK/installation/kubernetes-hardening.md"

for required in "$TOML" "$VALUES"; do
  [ -f "$required" ] || { echo "docs-claims: missing authority file $required" >&2; exit 1; }
done

collect() {
  if [ "${1:-}" = "--all" ]; then
    git ls-files "$BOOK/**/*.md" "$BOOK/*.md"
  elif [ "$#" -gt 0 ]; then
    printf '%s\n' "$@"
  else
    git diff --name-only origin/develop...HEAD -- "$BOOK/*.md" "$BOOK/**/*.md" 2>/dev/null \
      || git ls-files "$BOOK/**/*.md" "$BOOK/*.md"
  fi
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# ── the authorities ──────────────────────────────────────────────────────────

# Every TOML path, plus every ancestor table (a page may document `[audit]` as
# `config.audit`). A `#?` line documents a real optional/secret/derived key and
# counts; a plain `#` comment does not.
toml_paths() {
  awk '
    { line = $0
      sub(/^[[:space:]]*/, "", line)
      if (line ~ /^#\?[[:space:]]*/) sub(/^#\?[[:space:]]*/, "", line)
      else if (line ~ /^#/) next
      if (line ~ /^\[\[?[A-Za-z0-9_.]+\]\]?/) { sec = line; gsub(/^\[\[?|\]\]?.*$/, "", sec); next }
      if (line ~ /^[A-Za-z0-9_]+[[:space:]]*=/) {
        k = line; sub(/[[:space:]]*=.*$/, "", k)
        path = (sec == "" ? k : sec "." k)
        print path
        n = split(path, seg, ".")
        acc = seg[1]
        for (i = 2; i < n; i++) { print acc; acc = acc "." seg[i] }
        if (n > 1) print acc
      }
    }' "$1" | sort -u
}

# values.yaml flattened to dotted paths by indentation. Our own file, two-space
# indented; list items and comments are skipped.
values_paths() {
  awk '
    /^[[:space:]]*#/ { next }
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*-/ { next }
    { line = $0
      sub(/[[:space:]]+#.*$/, "", line)
      match(line, /^[[:space:]]*/); ind = RLENGTH
      key = line; sub(/^[[:space:]]*/, "", key)
      if (key !~ /^[A-Za-z_][A-Za-z0-9_]*:/) next
      sub(/:.*$/, "", key)
      lvl = int(ind / 2)
      stack[lvl] = key
      path = stack[0]
      for (i = 1; i <= lvl; i++) path = path "." stack[i]
      print path
    }' "$1" | sort -u
}

toml_paths "$TOML" > "$work/toml"
values_paths "$VALUES" > "$work/values"
# Env forms: the TOML path upper-cased with `__` between segments.
sed 's/\./__/g' "$work/toml" | tr 'a-z' 'A-Z' | sed 's/^/FERROEHR__/' | sort -u > "$work/env"
grep -vE '\.' "$work/values" > "$work/values_top"

failures=0
report() {
  printf '%s: %s\n' "$1" "$2" >&2
  failures=$((failures + 1))
}

files=$(collect "$@")

# ── 1. configuration keys ────────────────────────────────────────────────────
for f in $files; do
  [ -f "$f" ] || continue
  # A form ending in `__` is prose naming a prefix ("FERROEHR__AUDIT__…"), not a
  # key. Anything else must resolve.
  while read -r form; do
    [ -n "$form" ] || continue
    case "$form" in *__) continue ;; esac
    grep -qxF "$form" "$work/env" \
      || report "$f" "documents \`$form\`, which is not a key in $TOML"
  done < <(grep -ohE 'FERROEHR__[A-Z0-9_]+' "$f" | sort -u)
done

# ── 2. Helm values paths (chart pages only) ──────────────────────────────────
for f in $files; do
  case " $CHART_PAGES " in *" $f "*) ;; *) continue ;; esac
  [ -f "$f" ] || continue
  while read -r path; do
    [ -n "$path" ] || continue
    first=${path%%.*}
    # Only a token rooted at a real top-level values key is read as a values
    # path; anything else is ordinary prose that happens to contain a dot.
    grep -qxF "$first" "$work/values_top" || continue
    if grep -qxF "$path" "$work/values"; then continue; fi
    # `config.*` is rendered verbatim into ferroehr.toml, so the TOML schema is
    # its authority.
    case "$path" in
      config.*) grep -qxF "${path#config.}" "$work/toml" && continue ;;
    esac
    report "$f" "documents Helm value \`$path\`, which the chart does not define"
  done < <(grep -ohE '`[a-z][A-Za-z0-9]*(\.[A-Za-z0-9_]+)+`' "$f" | tr -d '`' | sort -u)
  # `--set` is unambiguous wherever it appears, so it is checked without the
  # top-level-key filter above.
  while read -r path; do
    [ -n "$path" ] || continue
    grep -qxF "$path" "$work/values" && continue
    case "$path" in
      config.*) grep -qxF "${path#config.}" "$work/toml" && continue ;;
    esac
    report "$f" "\`--set $path=\` names a value the chart does not define"
  done < <(grep -ohE '\-\-set +[a-z][A-Za-z0-9_.]*=' "$f" | sed -E 's/--set +//; s/=$//' | sort -u)
done

# ── 3. generated charts nothing embeds ───────────────────────────────────────
# Whole-corpus by nature: a page deleting its figure is exactly the case to
# catch, and that page may not be in a narrow diff.
#
# The reference may sit in an mdBook `{{#include}}` source rather than in a page
# — `comparison.md` pulls its figures in from `website/book/generated/` — so that
# tree is searched too. Searching only the pages reported a chart as orphaned when
# it was in fact published, which is the failure mode that gets a guard ignored.
#
# And `website/book/generated/` is a BUILD ARTIFACT: it is not committed and does
# not exist in a fresh checkout, so on its own it never matches and a figure
# published only through an include is reported as orphaned regardless — measured
# on `perf-stress-compare.svg`, which `scripts/render/comparison.sh` both renders
# AND embeds, while this guard called it unpublished. The durable reference for
# generated markdown is therefore the GENERATOR, so the render scripts are the
# third haystack.
while read -r svg; do
  [ -n "$svg" ] || continue
  base=${svg##*/}
  grep -rqF "$base" --include='*.md' "$BOOK" website/book/generated \
    || grep -rqF "$base" scripts/render \
    || report "$svg" "is committed but neither a page, a generated include, nor a render script embeds it"
done < <(git ls-files "$BOOK/*-assets/*.svg")

if [ "$failures" -gt 0 ]; then
  echo "docs-claims: $failures unresolvable claim(s) — see above." >&2
  echo "  A documented key must exist in its source. Fix the page, or fix the" >&2
  echo "  source if the page is describing what the software should do." >&2
  exit 1
fi
echo "docs-claims: OK."
