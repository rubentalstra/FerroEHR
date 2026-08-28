#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# String-built-SQL guard for every layer that builds SQL at runtime.
#
# The AQL engine builds SQL dynamically from attacker-controlled query text, so
# the one property that keeps it safe is that the SQL SHAPE comes from our own
# code and every user-supplied value arrives as a bound parameter. The OWASP SQL
# Injection Prevention Cheat Sheet names the failure mode this guard blocks:
# "Validated data is not necessarily safe to insert into SQL queries via string
# building"
# (https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html).
#
# Four shapes fail, over the three directories that build SQL at runtime —
# `app/ferroehr/src/aql/`, `app/ferroehr/src/storage/` and
# `app/ferroehr/src/system_log/` (the ITI-81 audit retrieval, the other
# `sea-query` site):
#   1. `format!`/`write!`/`writeln!` whose literal holds a SQL keyword AND an
#      interpolation — an interpolated clause is string-built SQL by definition.
#   2. `push_str("…<SQL keyword>…")` — assembling a statement fragment by
#      fragment onto a String, the shape step 1 usually grows into.
#   3. `push_str(<runtime value>)` — a value appended to a string being built.
#      Whether that string is SQL cannot be decided by grep, so the rule is
#      strict and legitimate sites are exempted by name below.
#   4. `AssertSqlSafe(format!(…))` — the sqlx injection guard waived over an
#      interpolated string, which is the assertion being false.
#
# What this guard CANNOT see, so that nobody mistakes a green run for the whole
# property: an identifier passed to `sea_query::Alias::new` is indistinguishable
# from our own generated alias at the grep level. That half is pinned by tests
# instead — `app/ferroehr/tests/it/sql_injection.rs` asserts that every quoted
# identifier in the emitted SQL comes from the closed set, over a corpus of
# queries carrying hostile text in every user-supplied position.
#
# Usage: scripts/checks/sql-string-building.sh [--all | <file>...]
#   no args  → the changed files that fall inside the scanned directories
#   --all    → every tracked .rs file in those directories
set -euo pipefail
cd "$(dirname "$0")/../.."

# The directories whose SQL is built at runtime. Everything else in the tree
# uses `sqlx::query!`/`query_as!` over static SQL, which the compiler checks.
SCOPE='app/ferroehr/src/aql/ app/ferroehr/src/storage/ app/ferroehr/src/system_log/'

# Clause-introducing keywords. A literal holding one of these is a statement
# fragment, not prose.
KEYWORDS='SELECT|FROM|WHERE|ORDER[[:space:]]+BY|GROUP[[:space:]]+BY|HAVING|JOIN|INSERT[[:space:]]+INTO|UPDATE[[:space:]]|DELETE[[:space:]]+FROM|UNION|LIMIT|OFFSET'

# Narrow, named exemptions. Each is a `file:needle` pair: the needle must appear
# on the flagged line for the exemption to apply, so it cannot silently widen to
# a neighbouring statement.
#
# `node_repo.rs` composes the promoted-column list of one INSERT header from
# `storage::promoted::PROMOTED_LEAVES`, whose `column` field is a
# `&'static str` in our own source — a closed set with no request-time input, and
# the values themselves are bound through `QueryBuilder::push_bind`.
EXEMPT='app/ferroehr/src/storage/node_repo.rs:push_str(leaf.column)'

collect() {
  if [[ "${1:-}" = "--all" ]]; then
    # shellcheck disable=SC2086 # SCOPE is a deliberate list of pathspecs
    git ls-files -- $SCOPE
  elif [[ "$#" -gt 0 ]]; then
    printf '%s\n' "$@"
  else
    # shellcheck disable=SC2086
    git diff --name-only origin/main...HEAD -- $SCOPE 2>/dev/null ||
      git ls-files -- $SCOPE
  fi
}

failures=0
report() {
  printf '%s\n' "$1" >&2
  failures=$((failures + 1))
}

# Whether `$1:$2` (file, line body) is one of the named exemptions.
exempted() {
  local file=$1 body=$2 entry needle
  for entry in $EXEMPT; do
    [[ "${entry%%:*}" = "$file" ]] || continue
    needle=${entry#*:}
    case $body in
    *"$needle"*) return 0 ;;
    *) ;;
    esac
  done
  return 1
}

# The double-quoted substrings of a line, one per output line. Rust string
# literals in the scanned code carry no escaped double quotes, so the naive
# split is exact here.
quoted() { printf '%s\n' "$1" | grep -o '"[^"]*"' || true; }

files=$(collect "$@")
[[ -n "$files" ]] || {
  echo "sql-string-building: no files in scope to check."
  exit 0
}

for f in $files; do
  [[ -f "$f" ]] || continue
  case $f in
  *.rs) ;;
  *) continue ;;
  esac

  while IFS=: read -r line body; do
    [[ -n "${line:-}" ]] || continue
    exempted "$f" "$body" && continue

    # (1) an interpolated literal that carries a clause keyword
    if printf '%s' "$body" | grep -qE '(format!|write!|writeln!)[[:space:]]*\('; then
      while IFS= read -r lit; do
        [[ -n "$lit" ]] || continue
        printf '%s' "$lit" | grep -qE "$KEYWORDS" || continue
        printf '%s' "$lit" | grep -q '{' || continue
        report "$f:$line: interpolated SQL literal $lit — build the clause with \
\`sea-query\`'s typed API and bind every value (scripts/checks/sql-string-building.sh)"
      done < <(quoted "$body")
    fi

    # (2) a clause keyword pushed onto a String
    if printf '%s' "$body" | grep -qE 'push_str[[:space:]]*\([[:space:]]*"'; then
      while IFS= read -r lit; do
        [[ -n "$lit" ]] || continue
        printf '%s' "$lit" | grep -qE "$KEYWORDS" || continue
        report "$f:$line: SQL fragment $lit pushed onto a String — assemble the \
statement with \`sea-query\` instead (scripts/checks/sql-string-building.sh)"
      done < <(quoted "$body")
    fi

    # (3) a runtime value appended to a string under construction. Each
    # `push_str(` on the line is judged separately, so a line that also pushes a
    # literal cannot shield a value push.
    while IFS= read -r push; do
      [[ -n "$push" ]] || continue
      printf '%s' "$push" | grep -qE 'push_str[[:space:]]*\([[:space:]]*r?#*"' && continue
      report "$f:$line: a runtime value is appended to a string ($push) — if that \
string is SQL, bind the value; if it is not, add a named exemption with its \
reason (scripts/checks/sql-string-building.sh)"
    done < <(printf '%s\n' "$body" | grep -oE 'push_str[[:space:]]*\([^)]*' || true)

    # (4) the sqlx injection guard waived over an interpolated string
    if printf '%s' "$body" | grep -qE 'AssertSqlSafe[[:space:]]*\([[:space:]]*format!'; then
      report "$f:$line: \`AssertSqlSafe\` over a \`format!\` — the assertion is \
false for an interpolated statement (scripts/checks/sql-string-building.sh)"
    fi
  done < <(grep -nE 'format!|write!|writeln!|push_str|AssertSqlSafe' "$f" || true)
done

if [[ "$failures" -gt 0 ]]; then
  echo "sql-string-building: $failures violation(s) — see above." >&2
  exit 1
fi
echo "sql-string-building: OK."
