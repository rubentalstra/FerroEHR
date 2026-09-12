---
name: main-ruleset-review-lifted
description: The one-reviewer requirement on main is lifted PERMANENTLY (owner ruling 2026-09-12) because Claude reviews every PR in-session; ruleset 18942581 stays at approvals 0, use gh pr merge --auto, never --admin
metadata: 
  node_type: memory
  type: project
  originSessionId: d296933f-5b31-42a5-a863-ff65cae9b32a
  modified: 2026-09-11T15:33:20.533Z
---

On 2026-09-11 the owner had the `main` ruleset (id 18942581) relaxed for the
duration of v4.2.0: `required_approving_review_count` 1 → 0 and
`require_code_owner_review` true → false. Everything else stayed (PR required,
signed commits, `conclusion` status check with strict up-to-date policy, no
deletion, no force push). Restoring it is tracker issue #3245 in v4.2.0.

**Why:** one person works the milestone; a required reviewer made
`gh pr merge --auto` never fire, and `--admin` bypasses are not the wanted
habit.

**How to apply:** merge flow during v4.2.0 is `gh pr merge --auto --squash
--delete-branch` once local gates are green; it merges when the
`conclusion` check passes. Do not use `--admin`. At the v4.2.0 cut, put the
review requirement back via a `PUT` of the full ruleset (the owner's call per
#3245). Related: [[merge-on-local-gates]], [[autonomous-phase-flow]].

**Update 2026-09-12 (owner ruling, v4.2.0 cut):** the requirement is not
restored at the cut and #3245 is closed as superseded. "Without the reviewer
setting on GitHub because you review it for me": every PR gets its in-session
review (gates, spec reading, diff read) before auto-merge is armed, and each
milestone from v4.3.0 on is worked the same way as v4.2.0 (batches of
related issues per PR, auto-merge on the final head).
