---
name: baseline-commit-is-where-the-run-was-recorded
description: A results.json regression is attributed to the commit that RE-RAN the pipeline, not the commit that caused it — always diff the range since the previous baseline
metadata:
  type: feedback
---

`docs/conformance/ferroehr/results.json` is regenerated only when someone runs
`scripts/conformance.sh`, not per PR. So "the regression landed in commit X" is
almost always false: X is where the run was RECORDED.

**Why:** on 2026-08-12 the 8 red rows were handed over as "landed in 11ee41ea7
(#2293), the prime suspect". `git log --oneline 6ffd14ed6..11ee41ea7 --
docs/conformance/ferroehr/results.json` shows 11ee41ea7 is the ONLY commit in
the range that touched it — and the range is **145 commits**. Neither cluster's
cause was in #2293: the terminology cluster came from bdf68d3f1 (#2282) and the
PGP cluster from 7a8a8c9a3, both earlier in the range.

**How to apply:** step 0 of any regression triage is
`git log --oneline <prev-baseline>..<red-baseline> -- docs/conformance/<sut>/results.json`
to establish the true suspect RANGE, then
`git diff <prev>..<red> -- <the module the failure implicates>` to find the one
semantic change. Never accept a handed-down "prime suspect" commit without
that check.
