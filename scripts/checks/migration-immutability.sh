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
#   * every file the merge base had under `app/ferroehr/migrations/<schema>/` is
#     gone at head, so no half-set survives for a database to apply against. New
#     files may appear in the same directory: a schema NAME can outlive the set
#     that used it, and re-authoring `ext`, `linkage` or `audit` is exactly that.
#     What may not survive is any file of the old set, edited or untouched.
#   * `<schema>` is named in `FIRST_GENERATION_SETS` in
#     `app/ferroehr/src/db/mod.rs` at head, which is the table the server's boot
#     refusal reads: a database carrying that set's bookkeeping is refused by
#     name, with the remedy, instead of failing on a checksum.
#
# Both halves are machine-checked here, so the acceptance cannot be claimed by
# a comment or a label. A single-file edit, a rename, a partial turnover, or a
# whole-set turnover whose schema the boot refusal does not name stays refused.
#
# Usage:
#   scripts/checks/migration-immutability.sh --diff <base> [head]
#   scripts/checks/migration-immutability.sh --self-test
set -euo pipefail
cd "$(dirname "$0")/../.." || exit 1

readonly MIGRATIONS='app/ferroehr/migrations/'
readonly RULE='.claude/rules/sqlx-conventions.md §Migrations'
# The file whose `FIRST_GENERATION_SETS` table the boot refusal reads.
readonly REFUSAL_FILE='app/ferroehr/src/db/mod.rs'

# The schemas the boot refusal names at `$1`, one per line.
#
# The table is a list of `FirstGenerationSet { schema: "…", … }` records, so the
# schema names are read out of it rather than restated here: a schema the guard
# accepted a retirement for is by construction one the server refuses a database
# for.
refused_schemas_at() {
  local rev="$1"
  git show "$rev:$REFUSAL_FILE" 2>/dev/null \
    | sed -n '/^const FIRST_GENERATION_SETS/,/^];$/p' \
    | grep -o 'schema: "[A-Za-z0-9_]*"' \
    | sed 's/schema: "\(.*\)"/\1/'
}

# Whether every file `$1` (the merge base) had under `$3` (a schema directory)
# is absent at `$2` (head).
#
# Turnover, not emptiness: the directory may hold a whole new set at head, which
# is what re-authoring a surviving schema name looks like. What must be gone is
# every file of the old one.
whole_set_turned_over() {
  local base="$1" head="$2" schema="$3" file
  while IFS= read -r file; do
    [[ -n "$file" ]] || continue
    if git cat-file -e "$head:$file" 2>/dev/null; then
      return 1
    fi
  done < <(git ls-tree -r --name-only "$base" -- "${MIGRATIONS}${schema}/")
  return 0
}

# Whether deleting a file of `$3` is part of a whole-set retirement the boot
# refusal covers.
retired_with_the_refusal() {
  local base="$1" head="$2" schema="$3"
  whole_set_turned_over "$base" "$head" "$schema" || return 1
  refused_schemas_at "$head" | grep -qx "$schema"
}

# The status letters git reports for a change that is not a pure addition.
# A = added (allowed); M = modified, D = deleted, R = renamed, C = copied,
# T = type change (all refused, except the whole-set retirement above).
#
# A RENAME is judged like a deletion of its old path, because that is what an
# installed database sees: the row it recorded for the old file has nothing left
# to match, checksum or not. Git reports the old path first, which is the one
# the retirement test reads.
refused_changes() {
  local base="$1" head="$2" fork status path schema
  # The comparison is against the MERGE BASE, not the base branch's tip: a
  # branch that fell behind a main which added a migration must not be told
  # it deleted that file (#3358).
  fork="$(git merge-base "$base" "$head")"
  while IFS=$'\t' read -r status path _; do
    [[ -n "${status:-}" ]] || continue
    case $status in
    D* | R* | C*)
      schema="${path#"$MIGRATIONS"}"
      schema="${schema%%/*}"
      retired_with_the_refusal "$fork" "$head" "$schema" && continue
      ;;
    *) ;;
    esac
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
  echo "boot refusal that names its schema: every file the base branch had under" >&2
  echo "${MIGRATIONS}<schema>/ must be gone (a new set may take its place), and" >&2
  echo "<schema> must be named in FIRST_GENERATION_SETS in ${REFUSAL_FILE}, so an" >&2
  echo "existing database is told what happened instead of being locked out by a" >&2
  echo "checksum." >&2
  echo >&2
  echo "The rule is $RULE." >&2
  return 1
}

# Proves the detector in every direction rather than trusting it, over the four
# shapes the rule turns on: turnover WITH the refusal, turnover WITHOUT it,
# PARTIAL turnover, and a one-file edit.
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
    mkdir -p "$MIGRATIONS/ext" "$MIGRATIONS/demographic" "$MIGRATIONS/audit" \
             "$MIGRATIONS/keep" "$(dirname "$REFUSAL_FILE")"
    printf 'SELECT 1;\n' > "$MIGRATIONS/ext/0001_openehr_functions.sql"
    printf 'SELECT 2;\n' > "$MIGRATIONS/ext/0002_tenant_context.sql"
    printf 'SELECT 3;\n' > "$MIGRATIONS/demographic/0001_baseline.sql"
    printf 'SELECT 4;\n' > "$MIGRATIONS/audit/0001_baseline.sql"
    printf 'SELECT 5;\n' > "$MIGRATIONS/audit/0002_chain.sql"
    printf 'SELECT 6;\n' > "$MIGRATIONS/keep/0001_baseline.sql"
    printf 'SELECT 7;\n' > "$MIGRATIONS/keep/0002_second.sql"
    printf 'const FIRST_GENERATION_SETS: &[FirstGenerationSet] = &[\n];\n' > "$REFUSAL_FILE"
    git add -A && git commit -qm base

    # (a) `ext` turns over WHOLE — every base file gone, a new set in its place —
    #     and the boot refusal names it: accepted, which is the shape this
    #     branch's own `ext`, `linkage` and `audit` sets have.
    # (b) `audit` turns over whole and the refusal does NOT name it: refused,
    #     because such a database is locked out rather than told.
    # (c) `demographic` turns over only PARTIALLY (one base file survives
    #     untouched): refused, even though the refusal names it.
    # (d) one file of `keep` is edited: refused, the plain rule.
    # (e) a file is added: allowed.
    git switch -qc work
    # A rename out of the old set is a deletion of the old path, so the
    # turnover still counts: git reports it as R, not D.
    git mv "$MIGRATIONS/ext/0001_openehr_functions.sql" "$MIGRATIONS/ext/0001_schema_and_roles.sql"
    printf 'SELECT 8;\n' > "$MIGRATIONS/ext/0001_schema_and_roles.sql"
    rm "${MIGRATIONS:?}/ext/0002_tenant_context.sql"
    rm "${MIGRATIONS:?}/audit/0001_baseline.sql" "${MIGRATIONS:?}/audit/0002_chain.sql"
    printf 'SELECT 9;\n' > "$MIGRATIONS/audit/0001_schema_and_roles.sql"
    printf 'SELECT 10; -- typo fix\n' > "$MIGRATIONS/keep/0002_second.sql"
    printf 'SELECT 11;\n' > "$MIGRATIONS/keep/0003_third.sql"
    printf 'const FIRST_GENERATION_SETS: &[FirstGenerationSet] = &[\n    FirstGenerationSet { schema: "ext", first_description: Some("openehr functions") },\n    FirstGenerationSet { schema: "demographic", first_description: None },\n];\n' > "$REFUSAL_FILE"
    git add -A && git commit -qm work

    # Meanwhile the base branch gained a migration the work branch never saw:
    # a tip-to-head diff would report it as deleted (#3358).
    git switch -q base
    printf 'SELECT 12;\n' > "$MIGRATIONS/keep/0004_later.sql"
    git add -A && git commit -qm later
  )
  local found failures=0
  found="$(cd "$tmp" && refused_changes base work)"
  if grep -q '/ext/' <<<"$found"; then
    echo "self-test: a whole-set turnover the boot refusal NAMES was wrongly caught" >&2
    failures=1
  fi
  grep -q '/audit/' <<<"$found" \
    || { echo "self-test: a whole-set turnover the boot refusal does NOT name was not caught" >&2; failures=1; }
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

  # PARTIAL turnover of a named set: one base file of `demographic` is deleted
  # while the other survives. Never acceptable — half a set is what a database
  # would apply against.
  (
    cd "$tmp" || exit 1
    git switch -q work
    printf 'SELECT 13;\n' > "$MIGRATIONS/demographic/0002_second.sql"
    git add -A && git commit -qm "a second demographic file, on the work branch"
    git switch -q base
    printf 'SELECT 13;\n' > "$MIGRATIONS/demographic/0002_second.sql"
    git add -A && git commit -qm "the same file on base, so it is a BASE file"
    git switch -q work
    git merge -q --no-edit base >/dev/null 2>&1 || true
    rm "${MIGRATIONS:?}/demographic/0001_baseline.sql"
    git add -A && git commit -qm "only half of the demographic set is gone"
  )
  local partial
  partial="$(cd "$tmp" && refused_changes base work)"
  grep -q '0001_baseline.sql' <<<"$partial" \
    || { echo "self-test: a PARTIAL turnover of a named set was not caught" >&2; failures=1; }
  if [[ "$failures" -ne 0 ]]; then
    return 1
  fi
  echo "migration-immutability: self-test OK (edit, partial turnover and unannounced turnover caught; announced whole-set turnover, addition and behind-main allowed)."
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
