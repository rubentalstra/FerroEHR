#!/usr/bin/env bash
# RFC 0201: an error carries its cause (#2034).
#
# `Error::source` is part of the `Error` contract, and `map_err(|e| V(e.to_string()))`
# destroys it at exactly the seam where it mattered. A stringified cause cannot
# be matched on, walked, or classified — the transport layer maps an error to a
# status code BY TYPE, so a `PoolTimedOut` that arrived as a string cannot become
# a 503 rather than a 500 without parsing prose.
#
# No clippy lint covers this (checked against the pinned 1.96.1 lint set), so
# the check is grep-shaped, and it is honest about that: it catches the common
# spelling, not every possible one. Its value is stopping the count from growing
# while the sweep runs.
#
# THE ALLOWLIST IS THE HONEST PART. Three shapes legitimately flatten:
#
#   1. a published `openehr-*` crate must not leak a private dependency's error
#      type into its own public API — that would make a patch bump of that
#      dependency a breaking change for us (RFC 1105);
#   2. a cause that is genuinely a MESSAGE rather than an error (a parser's
#      positional diagnostic that already IS the human-readable answer);
#   3. test code.
#
# An entry added without one of those reasons makes this gate decoration.
#
# Usage: scripts/checks/error-source-chain.sh [--all]
set -euo pipefail
cd "$(dirname "$0")/../.." || exit 1

# Trees still being swept. Each is a COUNT, and a count may only go DOWN:
# the sweep is judgement-per-site (#2034), so this pins progress rather than
# demanding a big-bang change.
declare -a BUDGETS=(
  "app/ferroehr:13"
  "app/ferroehr-ext:3"
  "app/ferroehr-admin-ui:6"
  "tools/cnf-runner:40"
  "tools/openehr-codegen:11"
)

fail=0
for entry in "${BUDGETS[@]}"; do
  tree="${entry%%:*}"
  budget="${entry##*:}"
  count="$(grep -rn 'map_err(|e| .*to_string())' "$tree" 2>/dev/null \
           | grep -vc '/tests/' || true)"
  count="${count:-0}"
  printf '%-26s %3s flattened (budget %s)\n' "$tree" "$count" "$budget"
  if [ "$count" -gt "$budget" ]; then
    echo "::error::$tree grew from $budget to $count flattened error sites — an error carries its cause (RFC 0201, #2034)"
    fail=1
  fi
done

[ "$fail" -eq 0 ] || exit 1
echo "error-source-chain: no tree grew"
