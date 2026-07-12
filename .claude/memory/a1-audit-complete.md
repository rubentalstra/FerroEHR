---
name: a1-audit-complete
description: "A1 full spec audit MERGED (PR #70, 2026-07-12): 24 chapters, 1,126 reqs, zero deferrals; ECC re-baselined 341/315/0; H1 follow-up = legacy ADR-citation sweep"
metadata: 
  node_type: memory
  type: project
  originSessionId: 5bbd1fbf-ddc2-477d-85d6-ea14273f175a
---

The A1 full spec audit is COMPLETE and merged to develop (PR #70,
2026-07-12). All 24 chapters closed with zero deferrals; consolidated
register at `docs/spec-audit/FINDINGS.md`, per-row verdicts in each
chapter's `verification.md`. ECC re-baselined at exactly 341 executed ·
315 passed · 0 failed (CORE PASS 9/9, STANDARD PASS 13/13) with all the
stricter validation in place.

**Why:** supersedes [[a1-audit-cadence]]'s "keep the loop running" — the
loop is done. Key late findings worth remembering: PR #69 left develop
with a stale ECC baseline (92 upload-406s + PARTY_SELF fixture cases —
always rerun ECC after merging anything that touches the runner or
validation); the AOM cardinality/existence split is load-bearing
(cardinality constrains a PRESENT container; absence is existence's
business — the vendored Multi_list corpus template + its valid
no-content composition is the canonical evidence).

**How to apply:** remaining follow-ups queued, not open-ended: H1 =
repo-wide legacy ADR-citation → spec-citation sweep (~1,000 mentions in
generated headers + E-arc extension modules; rule is scrub-on-touch
meanwhile). Next work per `docs/plans/current-phase.md`: X1 comparison
(awaiting owner review), P20, P99. Related: [[specs-over-adrs]],
[[ecc-own-conformance-framework]].
