---
name: tracker-is-github-issues
description: Owner ruling 2026-07-20 — the tracker is GitHub Issues (the open list IS the worklist); WORKLIST.md is a pointer stub; issue state only via gh
metadata:
  type: feedback
---

Owner ruling (2026-07-20, issue #134, codified in root CLAUDE.md §Issue
workflow): `docs/plans/WORKLIST.md` is retired to a pointer stub — the open
GitHub issue list IS the tracker. Issue body = plain opening summary +
`## Acceptance criteria` checklist; status/decisions go as issue comments, never
ever-growing body cells. Taxonomy: exactly one type label
(bug/enhancement/documentation/chore/refactor/perf/ci ↔ conventional-commit
types), P0–P3 priority labels, domain labels (spec:*, spec-update,
spec-impact:*, admin-ui), milestones = releases, pinned issues (max 3) =
current focus. PRs declare `Closes #N`; the merge into develop auto-closes
— never close by hand when a PR carries the work.

**Why:** native state + timeline + auto-close beat a hand-maintained
markdown table; the ADL2 row's ever-growing status cell was the failure
mode. Owner corrections en route: no house `worklist` label, no redundant
"(conventional type: X)" clutter in label descriptions, NO archive file
(git history is the archive), priority labels ARE wanted (P0–P3, industry
standard), `docs/PROGRESS.md` is RETIRED too (the closing PR description +
issue handoff comment carry the build narrative), ROADMAP.md is RETIRED
(owner 2026-08-04, #1867 — direction themes live in the public FerroEHR
Roadmap board's readme, live status on the board itself; the board is a
VIEW, Status is its only managed datum, writes only via
scripts/gh-project.sh — .claude/rules/project-board.md) — actionable
roadmap items live as issues, any
number quoted in a doc or issue body must be re-derived from the committed
artifacts, never copied forward ("stalled information is very very bad"),
and RELEASES are milestone-driven: cut when the vX.Y.Z milestone hits zero
open issues, tag on the merge commit, close the milestone, ensure the next
exists (procedure: .claude/rules/changelog.md).

**How to apply:** orient with `gh issue list --state open` (the
SessionStart hook injects it); record work by ticking issue checkboxes /
commenting via `gh`; new work discovered en route gets `gh issue create`,
not a prose deferral; deep plans stay `docs/plans/*.md` files linked from
their issue (delete-on-implementation unchanged). The Stop hook gates on
"commit made OR issue activity since session start". See
[[autonomous-phase-flow]], [[conventional-branch-naming]].
