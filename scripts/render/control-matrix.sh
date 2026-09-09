#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# Render website/book/src/compliance/control-matrix.md FROM the tracker.
#
# A hand-maintained compliance table is wrong the first time a control ships.
# This script reads the tracker instead: every issue whose body carries a
# structured line
#
#     Control: <legal source> <article or clause>
#
# becomes a row, joined to its legal source URL from the registry declared
# below and to its current status (shipped with the closing pull request, in
# progress from the roadmap board, or planned). An issue may declare several
# controls, one per line.
#
# DETERMINISM. The rendered page is a pure function of tracker state: no
# clock, no HEAD commit, no run counter. The docs CI job re-runs this script
# with --check and fails when the committed page differs, so anything that
# changed between two runs of an unchanged tracker would fail the build every
# day. When the page was generated, and from which commit, is git history of
# the file itself.
#
# Usage:
#   scripts/render/control-matrix.sh            write the page
#   scripts/render/control-matrix.sh --check    fail when the committed page is stale
set -euo pipefail
cd "$(dirname "$0")/../.."

PAGE="website/book/src/compliance/control-matrix.md"
SCRIPT="scripts/render/control-matrix.sh"

# The roadmap board is the only source of "in progress"
# (.claude/rules/project-board.md); the title matches scripts/gh/project.sh.
BOARD_TITLE="${FERROEHR_PROJECT_TITLE:-FerroEHR Roadmap}"

# Fetch ceiling. The tracker holds ~2100 issues, so a low limit silently drops
# the older half of the matrix; the truncation guard below turns a tracker that
# outgrows this number into a loud failure rather than a short page.
FETCH_LIMIT=10000

# The legal-source registry: short name, then the official publisher URL. A
# control naming a short name that is not declared here is a hard failure, so
# no page cell can ever carry free text where a citation belongs. Order here is
# the order the table groups by.
LEGAL_SOURCES=(
  "GDPR|https://eur-lex.europa.eu/eli/reg/2016/679/oj"
  "EHDS|https://eur-lex.europa.eu/eli/reg/2025/327/oj"
  "UAVG|https://wetten.overheid.nl/BWBR0040940"
  "Wabvpz|https://wetten.overheid.nl/BWBR0023864"
  "NEN 7510|https://www.nen.nl/zorg-en-welzijn/informatiebeveiliging-in-de-zorg/nen-7510"
  "NEN 7513|https://www.nen.nl"
  "IHE ATNA|https://profiles.ihe.net/ITI/TF/Volume1/ch-9.html"
)

die() {
  echo "control-matrix: $*" >&2
  exit 1
}

command -v gh >/dev/null 2>&1 || die "the GitHub CLI (gh) is not installed"
command -v jq >/dev/null 2>&1 || die "jq is required (brew install jq / preinstalled on CI runners)"

CHECK=0
case "${1:-}" in
  '') ;;
  --check) CHECK=1 ;;
  -h | --help)
    sed -n '/^# Usage:/,/--check    fail/p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
  *) die "unknown argument '$1' (expected --check)" ;;
esac

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ── the registry, as JSON ────────────────────────────────────────────────────
printf '%s\n' "${LEGAL_SOURCES[@]}" |
  jq -R -s 'split("\n") | map(select(length > 0) | split("|") | {name: .[0], url: .[1]})
            | to_entries | map(.value + {rank: .key})' > "$WORK/sources.json"

# ── the tracker ──────────────────────────────────────────────────────────────
# One call for everything a repository token can read. The roadmap board is a
# Projects (v2) board, which needs a different scope, so it is queried
# separately below and only for the issues that actually declare a control.
gh issue list --state all --limit "$FETCH_LIMIT" \
  --json number,title,body,state,stateReason,url,closedByPullRequestsReferences \
  > "$WORK/issues.json" || die "could not read the tracker (is gh authenticated?)"

fetched="$(jq 'length' "$WORK/issues.json")"
[[ "$fetched" -lt "$FETCH_LIMIT" ]] ||
  die "the tracker returned $fetched issues, the fetch ceiling — raise FETCH_LIMIT in $SCRIPT"

# ── controls ─────────────────────────────────────────────────────────────────
# One object per declared control. An unresolvable short name is emitted as an
# error row rather than a table cell.
jq --slurpfile sources "$WORK/sources.json" '
  ($sources[0]) as $reg
  | def source_for($text):
      [ $reg[] | select(.name as $n | $text == $n or ($text | startswith($n + " "))) ]
      | sort_by(-(.name | length)) | first;
    [ .[] as $issue
      | ($issue.body // "")
      | split("\n")[]
      | select(test("^[[:space:]]*(-[[:space:]]+|\\*[[:space:]]+)?Control:"))
      | sub("^[[:space:]]*(-[[:space:]]+|\\*[[:space:]]+)?Control:[[:space:]]*"; "")
      | sub("[[:space:]]+$"; "")
      | . as $text
      | source_for($text) as $src
      | if $src == null or ($text | length) == 0 then
          {error: true, issue: $issue.number, text: $text}
        else
          ($text[($src.name | length):] | sub("^[[:space:]]+"; "")) as $clause
          | {
              error: false,
              rank: $src.rank,
              source: $src.name,
              source_url: $src.url,
              clause: $clause,
              title: $issue.title,
              number: $issue.number,
              url: $issue.url,
              state: $issue.state,
              state_reason: ($issue.stateReason // ""),
              pr: ([ $issue.closedByPullRequestsReferences[]? | {number, url} ])
            }
        end
    ]
  | sort_by(.rank, .clause, .number)
' "$WORK/issues.json" > "$WORK/controls.json"

# A control naming an undeclared legal source stops the render, listing every
# offender so one run fixes them all.
if [[ "$(jq '[.[] | select(.error)] | length' "$WORK/controls.json")" -gt 0 ]]; then
  jq -r '.[] | select(.error)
         | "control-matrix: issue #\(.issue) declares legal source \"\(.text | split(" ")[0] // "")\" (from \"Control: \(.text)\"), which is not in the registry"' \
    "$WORK/controls.json" >&2
  die "add the source to LEGAL_SOURCES in $SCRIPT, or fix the issue body"
fi

# ── the roadmap board ────────────────────────────────────────────────────────
# "In progress" comes from the public board and nowhere else
# (.claude/rules/project-board.md). A Projects (v2) read needs the `project`
# token scope, which a plain repository token does not carry, so the query runs
# only for the OPEN issues that declare a control: a tracker with no open
# control never touches the board at all, and a refusal names what to grant.
echo '{}' > "$WORK/board.json"
open_controls="$(jq -r '[ .[] | select(.state == "OPEN") | .number ] | unique | .[]' "$WORK/controls.json")"
if [[ -n "$open_controls" ]]; then
  repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)" ||
    die "could not resolve the current repository (run inside a gh-authenticated clone)"
  : > "$WORK/board.jsonl"
  for issue in $open_controls; do
    # shellcheck disable=SC2016 # $owner/$name/$number are GraphQL variables, bound by the -f flags
    if ! gh api graphql -f owner="${repo%%/*}" -f name="${repo##*/}" -F number="$issue" \
      -f query='query($owner:String!,$name:String!,$number:Int!){
        repository(owner:$owner,name:$name){issue(number:$number){
          projectItems(first:20){nodes{project{title}
            fieldValueByName(name:"Status"){
              ... on ProjectV2ItemFieldSingleSelectValue{name}}}}}}}' > "$WORK/item.json"; then
      die "could not read the roadmap board status of issue #$issue. A Projects (v2) read needs the 'project' token scope (gh auth refresh -s project); in CI a repository token cannot read Projects v2, so the control-matrix job needs GH_TOKEN set to a token that can."
    fi
    jq -c --arg t "$BOARD_TITLE" --arg n "$issue" \
      '{key: $n, value: ([ .data.repository.issue.projectItems.nodes[]?
                           | select(.project.title == $t)
                           | .fieldValueByName.name // "" ] | first // "")}' \
      "$WORK/item.json" >> "$WORK/board.jsonl"
  done
  jq -s 'from_entries' "$WORK/board.jsonl" > "$WORK/board.json"
fi

# ── render ───────────────────────────────────────────────────────────────────
OUT="$WORK/control-matrix.md"

cat > "$OUT" <<'HEADER'
# Control matrix

FerroEHR is software. It is not a controller, not a processor and not a
certified organisation, so this page makes no compliance claim on anyone's
behalf. It lists the technical controls the product ships or plans, and the
article or clause each one is designed to support. Whether a deployment
satisfies a legal obligation depends on how the deploying organisation runs
it.

Every row comes from the tracker. A control is declared on the issue that
delivers it, as a line in the issue body:

```text
Control: <legal source> <article or clause>
```

The short name resolves to an official publisher URL from a registry inside
the generator, so a legal citation on this page is never free text. An issue
may declare several controls, one per line.

## How this page is built

`scripts/render/control-matrix.sh` queries the tracker with the GitHub CLI,
joins each declared control to its legal source and to its current state, and
writes this file. A CI job re-runs the generator with `--check` and fails the
build when the committed page no longer matches the tracker, which is what
keeps a shipped control from sitting here as "planned".

- **Shipped:** the issue is closed as completed. The closing pull request is
  linked in the last column.
- **In progress:** the issue is open and its card on the public roadmap board
  is in the In Progress column.
- **Planned:** the issue is open and work has not started.
- **Not planned:** the issue was closed without the control being built. The
  row stays visible so the record does not quietly lose it.

The page carries no generation timestamp and no build commit. Both change on
every run or every push while the tracker has not moved, which would make the
CI staleness check fail on days when nothing was wrong. When this page was
last regenerated, and from which commit, is the file's own git history.

## Controls

HEADER

rows="$(jq -r --slurpfile board "$WORK/board.json" '
  ($board[0]) as $status_of
  | def esc: gsub("\\|"; "\\|");
  def status:
    if .state == "CLOSED" and .state_reason == "NOT_PLANNED" then "Not planned"
    elif .state == "CLOSED" then "Shipped"
    elif $status_of[.number | tostring] == "In Progress" then "In progress"
    else "Planned" end;
  def prs:
    if (.pr | length) == 0 then "—"
    else [ .pr[] | "[#\(.number)](\(.url))" ] | join(", ") end;
  .[] | "| [\(.source)](\(.source_url)) | \(.clause | esc) | \(.title | esc) | [#\(.number)](\(.url)) | \(status) | \(prs) |"
' "$WORK/controls.json")"

if [[ -n "$rows" ]]; then
  cat >> "$OUT" <<'TABLE'
| Legal source | Article or clause | Control | Issue | Status | Closing PR |
|---|---|---|---|---|---|
TABLE
  printf '%s\n' "$rows" >> "$OUT"
else
  cat >> "$OUT" <<'EMPTY'
No issue in the tracker declares a control yet, so this table is empty. It
fills itself as the compliance program lands: the first issue to carry a
`Control:` line appears here on the next regeneration.
EMPTY
fi

cat >> "$OUT" <<'FOOTER'

## Legal sources

The short names above resolve to these publishers. The linked text is the
authority; nothing on this page restates it.

| Short name | Source |
|---|---|
FOOTER

jq -r '.[] | "| \(.name) | [\(.url)](\(.url)) |"' "$WORK/sources.json" >> "$OUT"

if [[ "$CHECK" -eq 1 ]]; then
  [[ -f "$PAGE" ]] || die "$PAGE does not exist — run $SCRIPT"
  if ! diff -u "$PAGE" "$OUT" > "$WORK/diff"; then
    cat "$WORK/diff" >&2
    die "$PAGE is stale against the tracker — run $SCRIPT and commit the result"
  fi
  echo "control-matrix: $PAGE matches the tracker ($(jq 'length' "$WORK/controls.json") controls)"
else
  mkdir -p "$(dirname "$PAGE")"
  cp "$OUT" "$PAGE"
  echo "control-matrix: wrote $PAGE ($(jq 'length' "$WORK/controls.json") controls from $fetched issues)"
fi
