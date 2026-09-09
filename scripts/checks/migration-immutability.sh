#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# Migrations are append-only (owner ruling 2026-09-09).
#
# People run FerroEHR now, so wiping a volume is no longer an option and an
# installation upgrades its database in place. sqlx records a checksum of every
# applied migration and refuses a database whose recorded checksum no longer
# matches the file on disk, so editing a migration that has shipped does not
# change history: it locks every existing installation out of its own database
# at boot, with an error about a checksum rather than about the edit.
#
# The rule is therefore absolute and this guard has no escape-hatch label. A
# schema change is a NEW file; a migration that turned out wrong is superseded
# by a later one, never rewritten. Adding a file passes, and so does continuing
# to work on a file the branch itself added, because the comparison is against
# the merge base rather than against the previous commit.
#
# Usage:
#   scripts/checks/migration-immutability.sh --diff <base> [head]
#   scripts/checks/migration-immutability.sh --self-test
set -euo pipefail
cd "$(dirname "$0")/../.." || exit 1

readonly MIGRATIONS='app/ferroehr/migrations/'
readonly RULE='.claude/rules/sqlx-conventions.md §Migrations'

# The status letters git reports for a change that is not a pure addition.
# A = added (allowed); M = modified, D = deleted, R = renamed, C = copied,
# T = type change (all refused).
refused_changes() {
  local base="$1" head="$2"
  git diff --name-status --diff-filter=MDRCT "$base" "$head" -- "$MIGRATIONS" \
    | awk -F'\t' '{ print $1 "\t" $2 }'
}

report() {
  local found="$1"
  if [[ -z "$found" ]]; then
    echo "migration-immutability: OK (no migration file was modified, renamed or deleted)."
    return 0
  fi
  echo "error: a migration that already exists on the base branch was changed" >&2
  echo >&2
  printf '%s\n' "$found" >&2
  echo >&2
  echo "sqlx checksums every applied migration, so changing one refuses the" >&2
  echo "database of every installation that already ran it. Carry the change" >&2
  echo "forward in a NEW migration instead:" >&2
  echo >&2
  echo "  sqlx migrate add --source ${MIGRATIONS}<schema> --sequential <desc>" >&2
  echo >&2
  echo "The rule is $RULE." >&2
  return 1
}

# Proves the detector in both directions rather than trusting it: a scratch
# repository where one migration is added, one is modified and one is deleted,
# so a guard that silently matched nothing would fail here.
self_test() {
  local tmp
  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064 # expand $tmp now, while it is still in scope
  trap "rm -rf '$tmp'" RETURN
  (
    cd "$tmp" || exit 1
    git init -q .
    git config user.email t@example.invalid
    git config user.name t
    mkdir -p "$MIGRATIONS/ehr"
    printf 'SELECT 1;\n' > "$MIGRATIONS/ehr/0001_baseline.sql"
    printf 'SELECT 2;\n' > "$MIGRATIONS/ehr/0002_second.sql"
    git add -A && git commit -qm base
    git switch -qc work
    # One legal change and two illegal ones.
    printf 'SELECT 3;\n' > "$MIGRATIONS/ehr/0003_third.sql"
    printf 'SELECT 1; -- typo fix\n' > "$MIGRATIONS/ehr/0001_baseline.sql"
    rm "$MIGRATIONS/ehr/0002_second.sql"
    git add -A && git commit -qm work
  )
  local found
  found="$(cd "$tmp" && git diff --name-status --diff-filter=MDRCT main work -- "$MIGRATIONS" 2>/dev/null \
    || cd "$tmp" && git diff --name-status --diff-filter=MDRCT master work -- "$MIGRATIONS")"
  local failures=0
  grep -q '0001_baseline.sql' <<<"$found" || { echo "self-test: a MODIFIED migration was not caught" >&2; failures=1; }
  grep -q '0002_second.sql' <<<"$found" || { echo "self-test: a DELETED migration was not caught" >&2; failures=1; }
  if grep -q '0003_third.sql' <<<"$found"; then
    echo "self-test: an ADDED migration was wrongly caught" >&2
    failures=1
  fi
  if [[ "$failures" -ne 0 ]]; then
    return 1
  fi
  echo "migration-immutability: self-test OK (modified and deleted caught, added allowed)."
}

case "${1:-}" in
--self-test)
  self_test
  ;;
--diff)
  base="${2:?usage: --diff <base> [head]}"
  head="${3:-HEAD}"
  report "$(refused_changes "$base" "$head")"
  ;;
*)
  echo "usage: $0 [--diff <base> [head] | --self-test]" >&2
  exit 2
  ;;
esac
