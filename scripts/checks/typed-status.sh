#!/usr/bin/env bash
# Typed-status guard (owner directive 2026-08-06).
#
# An HTTP status is compared as a TYPE, never as a number:
#
#     if status == StatusCode::OK { … }          // yes
#     if status.as_u16() == 200 { … }            // no
#     if status == 200 { … }                     // no
#
# `http::StatusCode` exists so the compiler catches a typo'd or misremembered
# code; `as_u16() == 401` throws that away, and a bare literal tells a reader
# nothing about which of the 4xx family was meant. The `http` crate names every
# registered code (docs.rs/http/latest/http/status/struct.StatusCode.html), so
# there is always a constant to use.
#
# Rendering the number is fine and stays legal — a log field, a metric label, a
# recorded wire outcome (`status.as_u16()` inside `format!`, a struct field, a
# `/ 100` class bucket). Only COMPARISON against a numeric literal is refused.
#
# Usage: scripts/checks/typed-status.sh [--all | <file>...]
#   no args  → the files changed against origin/develop
#   --all    → every tracked .rs file
set -euo pipefail
cd "$(dirname "$0")/../.."

collect() {
  if [ "${1:-}" = "--all" ]; then
    git ls-files '*.rs'
  elif [ "$#" -gt 0 ]; then
    printf '%s\n' "$@"
  else
    git diff --name-only origin/develop...HEAD -- '*.rs' 2>/dev/null || git ls-files '*.rs'
  fi
}

failures=0
files=$(collect "$@")
[ -n "$files" ] || { echo "typed-status: no Rust files to check."; exit 0; }

for f in $files; do
  [ -f "$f" ] || continue
  # `.as_u16()` immediately compared, or a `.status()` compared to a literal.
  while IFS=: read -r line body; do
    [ -n "${line:-}" ] || continue
    printf '%s:%s: numeric status comparison (%s) — compare the typed \n' \
      "$f" "$line" "$(printf '%s' "$body" | sed 's/^[[:space:]]*//' | cut -c1-60)" >&2
    printf '    `http::StatusCode` constant instead (scripts/checks/typed-status.sh)\n' >&2
    failures=$((failures + 1))
  done < <(grep -nE '(as_u16\(\)[[:space:]]*[=!]=)|(status\(\)[[:space:]]*[=!]=[[:space:]]*[0-9])' "$f" || true)
done

if [ "$failures" -gt 0 ]; then
  echo "typed-status: $failures violation(s) — see above." >&2
  exit 1
fi
echo "typed-status: OK."
