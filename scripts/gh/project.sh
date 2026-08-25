#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# scripts/gh/project.sh — the deterministic GitHub Projects (v2) board helper.
#
# WHY THIS EXISTS: the public roadmap board is a GitHub Project (v2), and its
# write commands (`gh project item-edit`) take OPAQUE GraphQL node ids — the
# project id, the Status field id, the option id, and the per-item id — never
# the issue #number a human knows. Hand-resolving four ids per status move is
# the same foot-gun class scripts/gh/rel.sh exists for, so this wrapper
# resolves everything from the issue #number and fails loud.
#
# The board is a VIEW, not a tracker: Status (Todo / In Progress / Done) is
# the ONLY board-managed datum, and this script deliberately exposes nothing
# else. Policy: .claude/rules/project-board.md.
#
# Official docs (durable references — the ONLY citations allowed for this):
#   gh project commands .. https://cli.github.com/manual/gh_project
#   Projects v2 API ...... https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-api-to-manage-projects
#   Built-in workflows ... https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-built-in-automations
#
# Requires the `project` token scope (`gh auth refresh -s project`).
#
# Usage:
#   scripts/gh/project.sh status <issue> <todo|in-progress|done>  # move an issue's board Status
#   scripts/gh/project.sh add    <issue>                          # add an issue to the board (auto-add normally does this)
#   scripts/gh/project.sh show   <issue>                          # print the issue's current board Status
#   scripts/gh/project.sh board                                   # print the whole board grouped by Status
#   scripts/gh/project.sh url                                     # print the project URL
#   scripts/gh/project.sh update <on-track|at-risk|off-track|complete|inactive> "<message>" \
#                        [--start YYYY-MM-DD] [--target YYYY-MM-DD]
#                                                                 # post a project status update (board
#                                                                 # header/side panel; markdown body)
#   scripts/gh/project.sh updates                                 # print recent status updates, newest first
#   scripts/gh/project.sh sync-dates                              # derive every item's "Target date" from its
#                                                                 # milestone's due date (the Roadmap view places
#                                                                 # items by this field; milestones only draw markers)
#
# The project is found by title (FERROEHR_PROJECT_TITLE overrides; default
# "FerroEHR Roadmap") under the repository owner.

set -euo pipefail

TITLE="${FERROEHR_PROJECT_TITLE:-FerroEHR Roadmap}"

die() {
  echo "gh-project: $*" >&2
  exit 1
}

need_int() {
  case "${1:-}" in
    '' | *[!0-9]*) die "expected an issue number, got '${1:-}'" ;;
  esac
}

command -v gh >/dev/null 2>&1 || die "the GitHub CLI (gh) is not installed"

REPO="$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null)" ||
  die "could not resolve the current repository (run inside a gh-authenticated clone)"
OWNER="${REPO%%/*}"

# Resolve the project's number + node id by title. One call, cached per run.
PROJ_NUMBER="" PROJ_ID=""
resolve_project() {
  [ -n "$PROJ_ID" ] && return 0
  local row
  row="$(gh project list --owner "$OWNER" --format json \
    --jq ".projects[] | select(.title == \"$TITLE\") | \"\(.number) \(.id)\"" 2>/dev/null)" ||
    die "could not list projects for $OWNER (missing 'project' token scope? run: gh auth refresh -s project)"
  [ -n "$row" ] || die "no project titled '$TITLE' under $OWNER"
  PROJ_NUMBER="${row%% *}"
  PROJ_ID="${row##* }"
}

# Resolve the Status single-select field id and one option id by label.
status_field_id() {
  resolve_project
  gh project field-list "$PROJ_NUMBER" --owner "$OWNER" --format json \
    --jq '.fields[] | select(.name == "Status") | .id'
}

status_option_id() {
  resolve_project
  local want="$1" id
  id="$(gh project field-list "$PROJ_NUMBER" --owner "$OWNER" --format json \
    --jq ".fields[] | select(.name == \"Status\") | .options[] | select(.name == \"$want\") | .id")"
  [ -n "$id" ] || die "the board has no Status option named '$want'"
  printf '%s' "$id"
}

# Resolve the board item id for issue #n ("" when the issue is not on the board).
item_id_for_issue() {
  resolve_project
  need_int "$1"
  # Looked up from the ISSUE side (one cheap node query), never by listing the
  # whole project: the board keeps every closed item, so `gh project
  # item-list --limit 1000` costs enough GraphQL points that GitHub's
  # secondary rate limit rejects it once the board is large (measured
  # 2026-08-25 at ~2.6k items).
  gh api graphql \
    -f owner="${REPO%%/*}" -f name="${REPO##*/}" -F number="$1" \
    -f query='query($owner:String!,$name:String!,$number:Int!){
      repository(owner:$owner,name:$name){issue(number:$number){
        projectItems(first:20){nodes{id project{id}}}}}}' \
    --jq ".data.repository.issue.projectItems.nodes[] | select(.project.id == \"$PROJ_ID\") | .id"
}

canonical_status() {
  case "$1" in
    todo | Todo) echo "Todo" ;;
    in-progress | in_progress | 'In Progress') echo "In Progress" ;;
    done | Done) echo "Done" ;;
    *) die "unknown status '$1' (use todo | in-progress | done)" ;;
  esac
}

cmd_add() {
  local n="${1:?issue number}"
  need_int "$n"
  resolve_project
  gh project item-add "$PROJ_NUMBER" --owner "$OWNER" \
    --url "https://github.com/$REPO/issues/$n" >/dev/null
  echo "ok: #$n is on the board"
}

cmd_status() {
  local n="${1:?issue number}" want
  want="$(canonical_status "${2:?status (todo|in-progress|done)}")"
  need_int "$n"
  resolve_project
  local item
  item="$(item_id_for_issue "$n")"
  if [ -z "$item" ]; then
    # Auto-add normally races ahead of a manual move; add explicitly and retry.
    cmd_add "$n" >/dev/null
    item="$(item_id_for_issue "$n")"
    [ -n "$item" ] || die "could not place #$n on the board"
  fi
  gh project item-edit --id "$item" --project-id "$PROJ_ID" \
    --field-id "$(status_field_id)" \
    --single-select-option-id "$(status_option_id "$want")" >/dev/null
  echo "ok: #$n → $want"
}

cmd_show() {
  local n="${1:?issue number}"
  need_int "$n"
  resolve_project
  # The issue-side lookup, for the same rate-limit reason as item_id_for_issue.
  local status
  status="$(gh api graphql \
    -f owner="${REPO%%/*}" -f name="${REPO##*/}" -F number="$n" \
    -f query='query($owner:String!,$name:String!,$number:Int!){
      repository(owner:$owner,name:$name){issue(number:$number){
        projectItems(first:20){nodes{project{id}
          fieldValueByName(name:"Status"){
            ... on ProjectV2ItemFieldSingleSelectValue{name}}}}}}}' \
    --jq ".data.repository.issue.projectItems.nodes[] | select(.project.id == \"$PROJ_ID\") | .fieldValueByName.name")"
  echo "#$n: ${status:-(not on the board)}"
}

cmd_board() {
  resolve_project
  gh project item-list "$PROJ_NUMBER" --owner "$OWNER" --limit 1000 --format json \
    --jq '.items | group_by(.status)[] | "== \(.[0].status // "(no status)") (\(length))", (.[] | "  #\(.content.number)  \(.title)")'
}

cmd_url() {
  resolve_project
  gh project view "$PROJ_NUMBER" --owner "$OWNER" --format json --jq '.url'
}

# Post a project status update (createProjectV2StatusUpdate — verified live in
# the GraphQL schema 2026-08-04; the concept docs describe the UI only). The
# $start/$target variables are nullable Dates: omitted flags are simply not
# sent, which GraphQL reads as null.
cmd_update() {
  local status="${1:?status (on-track|at-risk|off-track|complete|inactive)}"
  local body="${2:?message body (markdown)}"
  shift 2
  local start="" target="" enum
  while [ $# -gt 0 ]; do
    case "$1" in
      --start) start="${2:?--start needs YYYY-MM-DD}"; shift 2 ;;
      --target) target="${2:?--target needs YYYY-MM-DD}"; shift 2 ;;
      *) die "unknown flag '$1' (only --start/--target)" ;;
    esac
  done
  case "$status" in
    on-track) enum=ON_TRACK ;;
    at-risk) enum=AT_RISK ;;
    off-track) enum=OFF_TRACK ;;
    complete) enum=COMPLETE ;;
    inactive) enum=INACTIVE ;;
    *) die "unknown status '$status' (use on-track|at-risk|off-track|complete|inactive)" ;;
  esac
  resolve_project
  local -a dateargs=()
  [ -n "$start" ] && dateargs+=(-f startDate="$start")
  [ -n "$target" ] && dateargs+=(-f targetDate="$target")
  # shellcheck disable=SC2016  # GraphQL $variables are literal, never shell expansion
  gh api graphql \
    -f query='mutation($projectId: ID!, $body: String!, $status: ProjectV2StatusUpdateStatus!, $startDate: Date, $targetDate: Date) {
      createProjectV2StatusUpdate(input: {projectId: $projectId, body: $body, status: $status, startDate: $startDate, targetDate: $targetDate}) {
        statusUpdate { id }
      }
    }' \
    -f projectId="$PROJ_ID" -f body="$body" -f status="$enum" \
    "${dateargs[@]+"${dateargs[@]}"}" >/dev/null
  echo "ok: status update posted ($status)"
}

cmd_updates() {
  resolve_project
  # shellcheck disable=SC2016  # GraphQL $variables are literal, never shell expansion
  gh api graphql \
    -f query='query($owner: String!, $number: Int!) {
      user(login: $owner) { projectV2(number: $number) { statusUpdates(last: 10) {
        nodes { status startDate targetDate createdAt creator { login } body }
      } } }
    }' \
    -f owner="$OWNER" -F number="$PROJ_NUMBER" \
    --jq '.data.user.projectV2.statusUpdates.nodes | reverse | .[] | "== \(.status)  \(.createdAt)  by \(.creator.login)" + (if .startDate then "  start \(.startDate)" else "" end) + (if .targetDate then "  target \(.targetDate)" else "" end), .body, ""'
}

# Derive "Target date" from each item's milestone due date. The roadmap
# layout places items only by date/iteration fields (milestone due dates draw
# markers, never item bars — the roadmap-layout docs, read 2026-08-04), so
# this field exists solely as a machine-derived mirror: never hand-edit it,
# re-run this after changing a milestone due date or re-milestoning issues.
cmd_sync_dates() {
  resolve_project
  local field_id
  field_id="$(gh project field-list "$PROJ_NUMBER" --owner "$OWNER" --format json \
    --jq '.fields[] | select(.name == "Target date") | .id')"
  [ -n "$field_id" ] || die 'the board has no "Target date" field'
  local cursor="" page rows=0 set=0 cleared=0
  while :; do
    # shellcheck disable=SC2016  # GraphQL $variables are literal, never shell expansion
    page="$(gh api graphql \
      -f query='query($proj: ID!, $after: String) {
        node(id: $proj) { ... on ProjectV2 { items(first: 100, after: $after) {
          pageInfo { hasNextPage endCursor }
          nodes {
            id
            fieldValueByName(name: "Target date") { ... on ProjectV2ItemFieldDateValue { date } }
            content { ... on Issue { number milestone { dueOn } } }
          }
        } } }
      }' \
      -f proj="$PROJ_ID" ${cursor:+-f after="$cursor"} \
      --jq '.data.node.items')"
    while IFS=$'\t' read -r item want have; do
      rows=$((rows + 1))
      [ "$want" = "$have" ] && continue
      if [ -n "$want" ]; then
        gh project item-edit --id "$item" --project-id "$PROJ_ID" \
          --field-id "$field_id" --date "$want" >/dev/null
        set=$((set + 1))
      else
        gh project item-edit --id "$item" --project-id "$PROJ_ID" \
          --field-id "$field_id" --clear >/dev/null
        cleared=$((cleared + 1))
      fi
    done < <(printf '%s' "$page" | jq -r '.nodes[] | [.id, (.content.milestone.dueOn // "" | .[0:10]), (.fieldValueByName.date // "")] | @tsv')
    if [ "$(printf '%s' "$page" | jq -r '.pageInfo.hasNextPage')" = "true" ]; then
      cursor="$(printf '%s' "$page" | jq -r '.pageInfo.endCursor')"
    else
      break
    fi
  done
  echo "ok: $rows items scanned, $set dates set, $cleared cleared"
}

usage() {
  sed -n '2,38p' "$0" | sed 's/^# \{0,1\}//'
}

main() {
  local sub="${1:-}"
  [ -n "$sub" ] || {
    usage
    exit 1
  }
  shift
  case "$sub" in
    status) cmd_status "$@" ;;
    add) cmd_add "$@" ;;
    show) cmd_show "$@" ;;
    board) cmd_board "$@" ;;
    url) cmd_url "$@" ;;
    update) cmd_update "$@" ;;
    updates) cmd_updates "$@" ;;
    sync-dates) cmd_sync_dates "$@" ;;
    -h | --help | help) usage ;;
    *) die "unknown command '$sub' (run with no args for usage)" ;;
  esac
}

main "$@"
