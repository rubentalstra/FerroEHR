#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
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
# The three shapes below are all the same violation, and the guard has to see all
# three or it reports OK on a tree containing the pattern it exists to ban (which
# it did, on two live sites, until this was widened):
#
#     if status.as_u16() == 401 { … }                    // direct
#     let status = resp.status().as_u16(); if status == 401 { … }   // via a local
#     assert_eq!(resp.status(), 200);                    // via an assertion
#
# The second is caught by matching the COMPARISON of a status-named binding
# against a literal, wherever it sits — not by flagging the `as_u16()` binding
# itself, and not by flagging a `status: u16` parameter. Those two broader rules
# were tried and rejected on merit: they fire on code that legitimately RECEIVES a
# recorded wire number, which is the conformance runner's whole job (its catalogue
# stores expected statuses as JSON numbers and compares recorded against expected
# — a data-driven comparison with no literal in it, and correct as it stands).
# What the rule forbids is a LITERAL in the comparison, so that is what the guard
# looks for, in every form it can take: direct, through a local, in a `matches!`,
# and in an assertion.
#
# Usage: scripts/checks/typed-status.sh [--all | <file>...]
#   no args  → the files changed against origin/develop
#   --all    → every tracked .rs file
set -euo pipefail
cd "$(dirname "$0")/../.."

# The `--all` scope, and why it is not the whole tree yet.
#
# The rule is repo-wide, but two crates carry pre-existing violations of a shape
# that cannot be fixed as a side effect of adding this guard, so `--all` covers
# the server and the specification crates — where a mis-compared status IS a wire
# defect — and the two sweeps are tracked instead of quietly excluded forever:
#
#   * `app/ferroehr-admin-ui` — the console's own DTOs, in a crate parked pending
#     its full pass (#2055).
#
# `tools/cnf-runner` was the other one and is now clean (#2054): it holds
# `StatusCode` in memory and applies `.as_u16()` only where a number is rendered
# or serialized, with the committed artifacts proven byte-identical.
#
# `--all-really` still checks anything, so the #2055 sweep can verify itself
# as it lands. Explicit paths (the per-edit hook) honour the SAME parked-crate
# exclusion as `--all` — the hook must never block an edit CI accepts (#2441's
# QA pass hit exactly that on the console's u16 DTO fields).
collect() {
  if [ "${1:-}" = "--all-really" ]; then
    git ls-files '*.rs'
  elif [ "${1:-}" = "--all" ]; then
    git ls-files '*.rs' | grep -v '^app/ferroehr-admin-ui/'
  elif [ "$#" -gt 0 ]; then
    printf '%s\n' "$@" | grep -v 'app/ferroehr-admin-ui/' || true
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
  done < <(grep -nE \
    -e 'as_u16\(\)[[:space:]]*[=!]=' \
    -e 'status\(\)[[:space:]]*[=!]=[[:space:]]*[0-9]' \
    -e '\b[a-z_]*status[a-z_]*[[:space:]]*[=!]=[[:space:]]*[0-9]' \
    -e 'matches!\([[:space:]]*[a-z_]*status[a-z_]*[[:space:]]*,[[:space:]]*[0-9]' \
    -e 'assert(_eq|_ne)?!\([^,]*status\(\)[^,]*,[[:space:]]*[0-9]' \
    "$f" || true)
done

if [ "$failures" -gt 0 ]; then
  echo "typed-status: $failures violation(s) — see above." >&2
  exit 1
fi
echo "typed-status: OK."
