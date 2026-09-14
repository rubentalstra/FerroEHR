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
# RETIRING A WHOLE SET IS THE ONE THING THAT IS NOT AN EDIT, and it is accepted
# only in the shape that keeps an installation told rather than locked out:
#
#   * every file of `app/ferroehr/migrations/<schema>/` is gone at head, so no
#     half-set survives for a database to apply against, AND
#   * `<schema>` is named in `FIRST_GENERATION_SCHEMAS` in
#     `app/ferroehr/src/db/mod.rs` at head, which is the list the server's boot
#     refusal reads: a database carrying that schema's migration bookkeeping is
#     refused by name, with the remedy.
#
# Both halves are machine-checked here, so the acceptance cannot be claimed by
# a comment or a label. A single-file edit, a rename, a partial deletion, or a
# whole-set deletion whose schema the boot refusal does not name stays refused.
#
# Usage:
#   scripts/checks/migration-immutability.sh --diff <base> [head]
#   scripts/checks/migration-immutability.sh --self-test
set -euo pipefail
cd "$(dirname "$0")/../.." || exit 1

readonly MIGRATIONS='app/ferroehr/migrations/'
readonly RULE='.claude/rules/sqlx-conventions.md §Migrations'
# The file whose `FIRST_GENERATION_SCHEMAS` const the boot refusal reads.
readonly REFUSAL_FILE='app/ferroehr/src/db/mod.rs'

# The schemas the boot refusal names at `$1`, one per line.
refused_schemas_at() {
  local rev="$1"
  git show "$rev:$REFUSAL_FILE" 2>/dev/null \
    | sed -n 's/^const FIRST_GENERATION_SCHEMAS[^=]*= *&\[\(.*\)\];$/\1/p' \
    | grep -o '"[A-Za-z0-9_]*"' \
    | tr -d '"'
}

# Whether `$2` (a schema directory name) has no file left at `$1`.
whole_set_deleted() {
  local rev="$1" schema="$2" remaining
  remaining="$(git ls-tree -r --name-only "$rev" -- "${MIGRATIONS}${schema}/" | head -n 1)"
  [[ -z "$remaining" ]]
}

# Whether deleting `$2` at `$1` is a whole-set retirement the boot refusal covers.
retired_with_the_refusal() {
  local rev="$1" schema="$2"
  whole_set_deleted "$rev" "$schema" || return 1
  refused_schemas_at "$rev" | grep -qx "$schema"
}

# The status letters git reports for a change that is not a pure addition.
# A = added (allowed); M = modified, D = deleted, R = renamed, C = copied,
# T = type change (all refused, except the whole-set retirement above).
refused_changes() {
  local base="$1" head="$2" fork status path schema
  # The comparison is against the MERGE BASE, not the base branch's tip: a
  # branch that fell behind a main which added a migration must not be told
  # it deleted that file (#3358).
  fork="$(git merge-base "$base" "$head")"
  while IFS=$'\t' read -r status path _; do
    [[ -n "${status:-}" ]] || continue
    if [[ "$status" == D ]]; then
      schema="${path#"$MIGRATIONS"}"
      schema="${schema%%/*}"
      retired_with_the_refusal "$head" "$schema" && continue
    fi
    printf '%s\t%s\n' "$status" "$path"
  done < <(git diff --name-status --diff-filter=MDRCT "$fork" "$head" -- "$MIGRATIONS")
}

report() {
  local found="$1"
  if [[ -z "$found" ]]; then
    echo "migration-immutability: OK (no migration file was modified, renamed or partially deleted)."
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
  echo "A whole SET is retired rather than edited, and only together with the" >&2
  echo "boot refusal that names its schema: delete every file of" >&2
  echo "${MIGRATIONS}<schema>/ and name <schema> in FIRST_GENERATION_SCHEMAS in" >&2
  echo "${REFUSAL_FILE}, so an existing database is told what happened instead" >&2
  echo "of being locked out by a checksum." >&2
  echo >&2
  echo "The rule is $RULE." >&2
  return 1
}

# Proves the detector in every direction rather than trusting it: a scratch
# repository carrying one added file, one edited file, and two retired sets —
# one named by the boot refusal, one not.
self_test() {
  local tmp
  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064 # expand $tmp now, while it is still in scope
  trap "rm -rf '$tmp'" RETURN
  (
    cd "$tmp" || exit 1
    # The base branch is NAMED, so the diff below does not guess at the
    # machine's init.defaultBranch (#3285: a main/master fallback chained with
    # `||` always ran the master diff and died on a checkout defaulting to main).
    git init -q --initial-branch=base .
    git config user.email t@example.invalid
    git config user.name t
    mkdir -p "$MIGRATIONS/ehr" "$MIGRATIONS/demographic" "$MIGRATIONS/keep" "$(dirname "$REFUSAL_FILE")"
    printf 'SELECT 1;\n' > "$MIGRATIONS/ehr/0001_baseline.sql"
    printf 'SELECT 2;\n' > "$MIGRATIONS/ehr/0002_second.sql"
    printf 'SELECT 3;\n' > "$MIGRATIONS/demographic/0001_baseline.sql"
    printf 'SELECT 4;\n' > "$MIGRATIONS/keep/0001_baseline.sql"
    printf 'SELECT 5;\n' > "$MIGRATIONS/keep/0002_second.sql"
    printf 'const FIRST_GENERATION_SCHEMAS: &[&str] = &[];\n' > "$REFUSAL_FILE"
    git add -A && git commit -qm base

    # (a) the whole `ehr` set is retired AND the boot refusal names it: accepted.
    # (b) the whole `demographic` set is retired and the refusal does NOT name
    #     it: refused, because such a database is locked out rather than told.
    # (c) one file of `keep` is edited: refused, the plain rule.
    # (d) a file is added: allowed.
    git switch -qc work
    rm -r "${MIGRATIONS:?}/ehr" "${MIGRATIONS:?}/demographic"
    printf 'SELECT 5; -- typo fix\n' > "$MIGRATIONS/keep/0002_second.sql"
    printf 'SELECT 6;\n' > "$MIGRATIONS/keep/0003_third.sql"
    printf 'const FIRST_GENERATION_SCHEMAS: &[&str] = &["ehr"];\n' > "$REFUSAL_FILE"
    git add -A && git commit -qm work

    # Meanwhile the base branch gained a migration the work branch never saw:
    # a tip-to-head diff would report it as deleted (#3358).
    git switch -q base
    printf 'SELECT 7;\n' > "$MIGRATIONS/keep/0004_later.sql"
    git add -A && git commit -qm later
  )
  local found failures=0
  found="$(cd "$tmp" && refused_changes base work)"
  if grep -q '/ehr/' <<<"$found"; then
    echo "self-test: a whole-set retirement the boot refusal NAMES was wrongly caught" >&2
    failures=1
  fi
  grep -q '/demographic/' <<<"$found" \
    || { echo "self-test: a whole-set retirement the boot refusal does NOT name was not caught" >&2; failures=1; }
  grep -q '0002_second.sql' <<<"$found" \
    || { echo "self-test: an EDITED migration was not caught" >&2; failures=1; }
  if grep -q '0003_third.sql' <<<"$found"; then
    echo "self-test: an ADDED migration was wrongly caught" >&2
    failures=1
  fi
  if grep -q '0004_later.sql' <<<"$found"; then
    echo "self-test: a migration the BASE gained after the branch point was wrongly caught" >&2
    failures=1
  fi
  # A PARTIAL deletion of a retired set is the shape that must never pass: the
  # same tree with one file of `ehr` still present is refused even though the
  # boot refusal names the schema.
  (
    cd "$tmp" || exit 1
    git switch -q work
    mkdir -p "$MIGRATIONS/ehr"
    printf 'SELECT 1;\n' > "$MIGRATIONS/ehr/0001_baseline.sql"
    git add -A && git commit -qm partial
  )
  local partial
  partial="$(cd "$tmp" && refused_changes base work)"
  grep -q '0002_second.sql' <<<"$partial" \
    || { echo "self-test: a PARTIAL deletion of a retired set was not caught" >&2; failures=1; }
  if [[ "$failures" -ne 0 ]]; then
    return 1
  fi
  echo "migration-immutability: self-test OK (edit, partial deletion and unannounced retirement caught; announced whole-set retirement, addition and behind-main allowed)."
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
