---
name: cnf-strategy-doc-kept-permanently
description: "Owner ruling 2026-07-22: docs/plans/cnf-conformance-strategy.md is KEPT PERMANENTLY — exempt from the delete-on-implementation plan lifecycle"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 272d63aa-087c-4f6d-b24b-25937e06bca6
  modified: 2026-07-22T13:48:14.237Z
---

`docs/plans/cnf-conformance-strategy.md` is exempt from the
delete-on-implementation rule (owner ruling 2026-07-22, PR #236 — it was
deleted at the #202 cutover and the owner ordered it restored the same day).

**Why:** it is the normative internal design record for the CNF 2.0 runner —
especially the §8.14 population-anchored performance-class model (POC 2 ·
S 15 · L 150 · R 1,500 peak arrivals/s floors, p99 ≤ 1 s SLO, workload
mixes + their OECD/Eurostat/NHS derivation) that the performance chapter
(#233 / #202-W7) builds and measures against.

**How to apply:** never delete it in a `/phase-done` sweep; sessions doing
performance-chapter work must STUDY it first (§8.14 + §14.2/§14.3). Code
citations still point at `docs/specs/openehr/` only — the file carries its
own banner saying the same. See [[owner-work-style]].
