---
name: phase-status
description: >
  Prints the tracker's open issues (pinned first), any linked plan file's
  unchecked tasks, and a short git status. Use when the user asks "where are
  we", "what's in flight", or at the start of a work session to orient.
allowed-tools: [Read, Bash]
argument-hint: (none)
---

# /phase-status

A fast orientation dump — step 1 of the issue workflow (`CLAUDE.md`).
Read-only; makes no changes. The live state below is injected at invocation
time — ground the answer in it, not in stale conversation memory.

## Live state (injected)

### The tracker — open GitHub issues (with native relationships)

Each line carries `{k/n}` sub-issue progress, `child-of #parent`, and open
`BLOCKED-by`/`blocks` edges (`.claude/rules/issue-relationships.md`).

```!
gh api graphql -f query='query($owner: String!, $name: String!) { repository(owner: $owner, name: $name) { issues(first: 100, states: OPEN, orderBy: {field: CREATED_AT, direction: DESC}) { nodes { number title labels(first: 20) { nodes { name } } milestone { title } parent { number } subIssuesSummary { total completed } blockedBy(first: 30) { nodes { number state } } blocking(first: 30) { nodes { number state } } } } } }' -f owner="$(gh repo view --json owner --jq .owner.login)" -f name="$(gh repo view --json name --jq .name)" --jq '.data.repository.issues.nodes[] | ([.labels.nodes[].name] | join(", ")) as $labels | ([.blockedBy.nodes[] | select(.state == "OPEN") | "#\(.number)"]) as $b | ([.blocking.nodes[] | select(.state == "OPEN") | "#\(.number)"]) as $k | "#\(.number)  \(.title)  [\($labels)]" + (if .milestone then "  (\(.milestone.title))" else "" end) + (if .parent then "  child-of #\(.parent.number)" else "" end) + (if .subIssuesSummary.total > 0 then "  {\(.subIssuesSummary.completed)/\(.subIssuesSummary.total)}" else "" end) + (if ($b | length) > 0 then "  BLOCKED-by \($b | join(","))" else "" end) + (if ($k | length) > 0 then "  blocks \($k | join(","))" else "" end)'
```

### Git

```!
cd "${CLAUDE_PROJECT_DIR}" && git status --short | head -40 && echo "---" && git log --oneline -5
```

## Steps

1. Summarize the tracker state: which issue is the current focus (pinned /
   milestone-assigned / the one the branch implements) and what its stated
   next action is. For the in-flight issue, run
   `gh issue view <n> --comments` and report the latest status comment and
   the unchecked `## Exit criteria` boxes, plus `scripts/gh-rel.sh tree <n>`
   for its parent/children/blockers. **Flag any open issue shown `BLOCKED-by`
   an open blocker** — it is stuck until the blocker closes and is not a
   pickup candidate (`.claude/rules/issue-relationships.md`).
2. If that issue links a plan file (`docs/plans/*.md`), **Read it** and list
   every unchecked `- [ ]` line under its Tasks and Exit criteria, in
   order, so it is clear how close the work is to done.
3. Summarize the git state from the injected output: current branch work,
   uncommitted files, last commits — flag uncommitted work that looks
   finished (the owner rule: never leave finished work sitting unmerged).
4. **Do not** modify any issue, plan file, or make any commit — this is a
   read-only status check. If the user wants the next task turned into a
   work plan, point them at `/next-task` instead.
