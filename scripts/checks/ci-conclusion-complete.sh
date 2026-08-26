#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# Every CI job must be reachable from the `conclusion` gate.
#
# Branch protection points at `conclusion` alone, on purpose: that is what lets
# jobs be added, renamed or path-gated without editing repository settings. The
# cost of that design is that a job missing from `conclusion.needs` still RUNS
# and still goes red, while merging stays green — a gate that looks enforced and
# is not.
#
# That is not hypothetical: on 2026-08-12 four gates (`chart-boot`,
# `error-source-chain`, `no-python`, `spec-citations`) were found running
# without gating anything. This check exists so the next one fails loudly.
set -euo pipefail

cd "$(dirname "$0")/../.."

readonly WORKFLOW='.github/workflows/ci.yml'

# Job keys are the only two-space-indented `name:` lines under `jobs:`.
jobs=$(yq -o=json '.jobs | keys' "$WORKFLOW" | jq -r '.[]' | grep -vx 'conclusion' | sort)
needs=$(yq -o=json '.jobs.conclusion.needs' "$WORKFLOW" | jq -r '.[]' | sort)

if missing=$(comm -23 <(echo "$jobs") <(echo "$needs")) && [[ -n "$missing" ]]; then
  echo "error: CI jobs that do not gate the merge" >&2
  echo >&2
  # shellcheck disable=SC2001 # bullets EVERY line of a multi-line list; ${//} has no ^ anchor
  echo "$missing" | sed 's/^/  - /' >&2
  echo >&2
  echo "Branch protection requires only the 'conclusion' check, so a job absent" >&2
  echo "from its 'needs' list can fail without blocking a merge. Add each job" >&2
  echo "above to jobs.conclusion.needs in $WORKFLOW." >&2
  exit 1
fi

# The mirror direction: a needs entry naming a job that no longer exists makes
# `conclusion` fail permanently on an unresolvable dependency.
if stale=$(comm -13 <(echo "$jobs") <(echo "$needs")) && [[ -n "$stale" ]]; then
  echo "error: conclusion.needs names jobs that do not exist" >&2
  # shellcheck disable=SC2001 # bullets EVERY line of a multi-line list; ${//} has no ^ anchor
  echo "$stale" | sed 's/^/  - /' >&2
  exit 1
fi

echo "ok: all $(echo "$jobs" | wc -l | tr -d ' ') CI jobs gate the merge"
