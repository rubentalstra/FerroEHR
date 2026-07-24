---
name: issue-relationships
description: Owner ruling 2026-07-24 — use GitHub's four native issue relationships (sub-issue/blocked-by/blocking/security-alert) as first-class tracker structure, set ONLY via scripts/gh-rel.sh
metadata:
  type: project
---

Owner ruling (2026-07-24): the tracker uses GitHub's **four native issue
relationships** as first-class structure, not prose. Codified in root
CLAUDE.md §Issue workflow + `.claude/rules/issue-relationships.md`; the
sanctioned command surface is **`scripts/gh-rel.sh`**
(`parent`/`unparent`/`blocked-by`/`unblock`/`blocking`/`unblocking`/`tree`/`id`).

- **Sub-issues** (`Add parent`): decompose a multi-part issue into
  individually-closeable children (progress rolls up; ≤100 children, ≤8
  levels). **NOT for release grouping — milestones stay the release spine, no
  per-release epic issues.**
- **Blocked-by / blocking** (GA Aug 2025): in-repo issue→issue sequencing
  (≤50/direction). An *upstream* wait stays the `blocked-upstream` label —
  you cannot be `blocked_by` a Jira ticket.
- **Security alerts**: link a code-scanning alert to an issue — **UI-only, no
  API**; code scanning enabled by `.github/workflows/codeql.yml` (CodeQL Rust,
  build-mode none, GA Oct 2025).

**Why gh-rel.sh, not raw gh api:** `gh` has NO native subcommand for these
(verified gh 2.88.1), and every WRITE endpoint takes the issue's **database
id**, not its `#number` (`sub_issue_id`/`issue_id` in the body) — a foot-gun.
The helper resolves `#number → id` and fails loud. Reads are `#number`-keyed
(no resolution needed). All facts docs-verified live against the repo (the
four REST endpoints returned `[]` cleanly; GraphQL exposes
`parent`/`subIssuesSummary`/`blockedBy`/`blocking`).

**How to apply:** the SessionStart hook + `/phase-status` annotate every open
issue with `{k/n}` progress, `child-of #x`, and open `BLOCKED-by`/`blocks`
(one batched GraphQL call). `/next-task` skips `BLOCKED-by` issues and prefers
a parent's next open child; `/phase-done` refuses to close a parent with open
children. Cite only the official GitHub docs (durable), never an internal md.
See [[tracker-is-github-issues]], [[conventional-branch-naming]].
