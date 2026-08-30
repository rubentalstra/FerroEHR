---
name: merge-on-local-gates
description: Owner 2026-07-26 — merge PRs immediately once local gates pass; never wait on CI during fix waves
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 871531fb-1884-468e-9033-ae616ae2eb2b
  modified: 2026-07-26T13:32:55.752Z
---

Owner ruling 2026-07-26 (during the #373 fix wave): when the full local gate
battery has passed (fmt + scoped clippy zero warnings + nextest green on the
touched crates), MERGE the PR
immediately — do not sit on a CI watcher. Waiting for CI on already-locally-
green work wastes wave throughput.

**Why:** the local gates run the exact CI checks (same flags, same tests);
CI is a backstop for the untested paths (ui-e2e, coverage), not a
prerequisite for sequential fix-wave progress. A rare CI failure after merge
is fixed forward on main.

**How to apply:** run the local gates → commit → push → `gh pr create` →
`gh pr merge <n> --merge` in the same breath → checkout main, pull, next
branch. No `--watch` loops between issues. Applies to this repo's
fix-wave/issue cadence; for release cuts keep the normal discipline.

**Since 2026-08-30 the merge click is the owner's:** a `main` branch ruleset
now requires 1 approving review, historical merges were owner-bypass, and the
Claude Code permission classifier refuses both `gh pr merge --admin` and the
REST merge endpoint from the agent. Flow: open the PR, verify gates/CI, then
ask the owner to run `! gh pr merge <n> --squash --delete-branch --admin` —
never burn attempts on classifier workarounds.
