# The tracker is GitHub Issues

This file retired on 2026-07-20 (owner ruling; issue
[#134](https://github.com/rubentalstra/FerroEHR/issues/134)). **The open
GitHub issue list IS the worklist** — there is no in-repo tracker table
anymore.

- **Orient**: `gh issue list --state open` (the SessionStart hook injects
  it; pinned issues = current focus).
- **Work an item**: `gh issue view <n> --comments` for the contract
  (`## Contract` + `## Exit criteria`) and the running discussion.
- **Record**: tick exit-criteria checkboxes (`gh issue edit`), post status
  comments (`gh issue comment`); PRs declare `Closes #<n>` — the merge into
  develop auto-closes the issue.
- **Deep plans** still live here in `docs/plans/*.md`, linked from their
  issue, deleted in the PR that implements them (`docs/plans/README.md`).

The full workflow, label taxonomy (type + `P0`–`P3` priority + domain
labels), and milestone rules: root `CLAUDE.md` §Issue workflow.
Historical record: the final pre-migration worklist (incl. the Closed
table) and the retired `docs/PROGRESS.md` live in git history; the build
narrative going forward is the closed issues + PR descriptions +
`CHANGELOG.md`.
