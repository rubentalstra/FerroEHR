#!/usr/bin/env bash
# scripts/gh-project.sh — the deterministic GitHub Projects (v2) board helper.
#
# WHY THIS EXISTS: the public roadmap board is a GitHub Project (v2), and its
# write commands (`gh project item-edit`) take OPAQUE GraphQL node ids — the
# project id, the Status field id, the option id, and the per-item id — never
# the issue #number a human knows. Hand-resolving four ids per status move is
# the same foot-gun class scripts/gh-rel.sh exists for, so this wrapper
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
#   scripts/gh-project.sh status <issue> <todo|in-progress|done>  # move an issue's board Status
#   scripts/gh-project.sh add    <issue>                          # add an issue to the board (auto-add normally does this)
#   scripts/gh-project.sh show   <issue>                          # print the issue's current board Status
#   scripts/gh-project.sh board                                   # print the whole board grouped by Status
#   scripts/gh-project.sh url                                     # print the project URL
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
  gh project item-list "$PROJ_NUMBER" --owner "$OWNER" --limit 1000 --format json \
    --jq ".items[] | select(.content.repository == \"$REPO\" and .content.number == $1) | .id"
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
  local status
  status="$(gh project item-list "$PROJ_NUMBER" --owner "$OWNER" --limit 1000 --format json \
    --jq ".items[] | select(.content.repository == \"$REPO\" and .content.number == $n) | .status")"
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

usage() {
  sed -n '2,33p' "$0" | sed 's/^# \{0,1\}//'
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
    -h | --help | help) usage ;;
    *) die "unknown command '$sub' (run with no args for usage)" ;;
  esac
}

main "$@"
