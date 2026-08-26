---
name: cnf-strategy-doc-kept-permanently
description: "Owner rulings 2026-07-22 + 2026-07-23 + 2026-08-26: the CNF 2.0 design record is PERMANENT and now lives in the Veredictum repository as its root ARCHITECTURE.md — never delete, never re-create a copy in FerroEHR"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 272d63aa-087c-4f6d-b24b-25937e06bca6
  modified: 2026-08-26T00:00:00.000Z
---

The CNF 2.0 design record — a living reference document with a
community-facing head — is exempt from the delete-on-implementation rule and
is kept permanently (owner rulings 2026-07-22 + 2026-07-23). It moved with
the instrument on 2026-08-26 (#2811): it is now `ARCHITECTURE.md` at the root
of the **Veredictum** repository, and FerroEHR carries no copy.

**Why:** it is the normative design record for the conformance instrument —
especially the §8.14 population-anchored performance-class model (POC 2 ·
S 15 · L 150 · R 1,500 peak arrivals/s floors, p99 ≤ 1 s SLO, workload
mixes + their OECD/Eurostat/NHS derivation) that every measured performance
claim is built and judged against.

**How to apply:** read it in the Veredictum checkout the pin resolves
(`scripts/lib/veredictum.sh` → `veredictum_root`, then `ARCHITECTURE.md`)
before any performance-chapter work — §8.14 + §14.2/§14.3. Never re-create a
FerroEHR copy: two copies of a design record is how one of them goes stale.
Code citations still point at `docs/specs/openehr/` only.
See [[owner-work-style]].
