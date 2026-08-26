---
name: cnf-red-2026-07-27-ehr-status-payloads
description: The two EHR_STATUS payload sites PR #431 missed (recipe + contribution case core) — catalogue defects, not app
metadata:
  type: project
---

PR #431 (issue #423) swept the RM-invalid EHR_STATUS fixtures but reached only
`source:`-backed JSON files. Two payload sites were missed and turned red in the
2026-07-27 run (28 rows):

1. `corpus/recipes/ehr_status.md` (the digest-pinned
   contract) + its implementation Veredictum's `src/exec/recipes.rs`
   `fn ehr_status` + the two digests in
   `artifacts/corpus/MANIFEST.yaml` (`cnf.ehr_status.provided`) — drives
   `I_EHR_SERVICE.create_ehr-main` rows 2..17.
2. The inline `versions[0].data` block in
   `artifacts/schedule/contribution/I_EHR_CONTRIBUTION.commit_contribution-ehr_status_valid_combinations.yaml`
   — all 12 rows.

Both emit `archetype_node_id: openEHR-EHR-EHR_STATUS.generic.v1` with no
`archetype_details` → see [[ehr-status-archetype-root-invariant]].

**How to apply:** a corpus **recipe** defect is a CATALOGUE bin defect even
though the code lives in Veredictum's `src` — the authority is the
digest-pinned `corpus/recipes/*.md` contract; the `.rs` is its implementation and
moves in lockstep with a re-stamped digest. Sweep for further sites with:
`for f in $(grep -rl '_type.*EHR_STATUS' "$VD/artifacts"); do grep -q archetype_details "$f" || echo "$f"; done` (`$VD` = the pinned Veredictum checkout)
(read-only cases and the deliberately-invalid fixture are the expected hits).
