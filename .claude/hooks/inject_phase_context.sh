#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# .claude/hooks/inject_phase_context.sh
#
# Claude Code SessionStart hook: prints the open GitHub issue list (the
# tracker — CLAUDE.md issue workflow) annotated with native issue
# relationships (parent/sub-issue progress + blocked-by/blocking, via one
# batched GraphQL call — see .claude/rules/issue-relationships.md), git
# status, and the last 10 commits so every session starts oriented. Also
# records the session-start HEAD and timestamp so phase_gate.sh (Stop hook)
# can tell whether a commit or issue activity happened during the session.
#
# No offline fallback (owner 2026-07-20): agents work online by definition;
# a failed `gh` call just surfaces its error.

set -uo pipefail

root="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
cd "$root" || exit 0

mkdir -p .claude
git rev-parse HEAD >.claude/.session-start-head 2>/dev/null || true
date -u +%Y-%m-%dT%H:%M:%SZ >.claude/.session-start-time 2>/dev/null || true

echo "=== spec oracle ==="
echo "Vendored openEHR spec text + CNF test schedule: docs/specs/openehr/ (index: its README.md; use /spec-lookup). Implement and review all spec-facing behaviour against that text — never from memory or EHRbase behaviour alone (.claude/rules/spec-adherence.md)."
echo
echo "=== tracker: open GitHub issues (gh issue view <n> --comments for the contract + discussion) ==="
echo "--- pinned (current focus) ---"
repo_nwo="$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null || true)"
gh api graphql \
  -f query='query($owner: String!, $name: String!) { repository(owner: $owner, name: $name) { pinnedIssues(first: 3) { nodes { issue { number title } } } } }' \
  -f owner="${repo_nwo%%/*}" -f name="${repo_nwo##*/}" \
  --jq '.data.repository.pinnedIssues.nodes[].issue | "#\(.number)  \(.title)"' 2>&1 || true
echo "--- open (child-of = sub-issue; {k/n} = sub-issue progress; BLOCKED-by = has an open blocker — not a /next-task candidate; relationships: .claude/rules/issue-relationships.md) ---"
# One batched GraphQL call yields each open issue's labels, milestone, parent,
# sub-issue progress, and open blockers/blocks — so the tracker shows work
# structure, not just a flat list. Falls back gracefully if gh/GraphQL fails.
gh api graphql \
  -f query='query($owner: String!, $name: String!) { repository(owner: $owner, name: $name) { issues(first: 100, states: OPEN, orderBy: {field: CREATED_AT, direction: DESC}) { nodes { number title labels(first: 20) { nodes { name } } milestone { title } parent { number } subIssuesSummary { total completed } blockedBy(first: 30) { nodes { number state } } blocking(first: 30) { nodes { number state } } } } } }' \
  -f owner="${repo_nwo%%/*}" -f name="${repo_nwo##*/}" \
  --jq '.data.repository.issues.nodes[]
    | ([.labels.nodes[].name] | join(", ")) as $labels
    | ([.blockedBy.nodes[] | select(.state == "OPEN") | "#\(.number)"]) as $b
    | ([.blocking.nodes[]  | select(.state == "OPEN") | "#\(.number)"]) as $k
    | "#\(.number)  \(.title)  [\($labels)]"
      + (if .milestone then "  (\(.milestone.title))" else "" end)
      + (if .parent then "  child-of #\(.parent.number)" else "" end)
      + (if .subIssuesSummary.total > 0 then "  {\(.subIssuesSummary.completed)/\(.subIssuesSummary.total)}" else "" end)
      + (if ($b | length) > 0 then "  BLOCKED-by \($b | join(","))" else "" end)
      + (if ($k | length) > 0 then "  blocks \($k | join(","))" else "" end)' 2>&1 || true
echo
echo "=== git status ==="
git status --short --branch 2>/dev/null | head -40
echo
echo "=== last 10 commits ==="
git log --oneline -10 2>/dev/null

exit 0
