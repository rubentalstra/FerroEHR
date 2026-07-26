---
name: its-audit-fix-first-cadence
description: "ITS-REST audit program (#373) runs fix-first — every fix issue from a group audit is implemented+merged before the next group's audit starts"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 871531fb-1884-468e-9033-ae616ae2eb2b
  modified: 2026-07-26T10:46:27.596Z
---

Owner ruling 2026-07-26, during the #373 full ITS-REST surface audit: the audit
proceeds **group by group, fix-first**. After auditing a group (one sub-issue
of #373) and filing its divergence fix issues, ALL of those fix issues are
implemented, tested, PR'd, and merged BEFORE the next group's audit begins.
Never accumulate an audit-finding backlog across groups.

**Why:** the owner was already angry about recurring spec divergences; a pile
of open bug issues from parallel audits is exactly the failure mode this
program exists to end. Fixing per group also means later group audits (e.g.
demographic) audit the FIXED shared layer instead of re-documenting known
defects. Matches the standing [[owner-work-style]] "defer nothing".

**How to apply:** cadence per group = audit → file fix issues (bug, spec:ITS,
same milestone) → implement every one (P1 first; orchestrator takes
versioning/critical-path fixes itself, workers take mechanical ones, max 2) →
each fix PR carries its CNF pinning case + changelog entry → merge all → CNF
run zero-drift → close the group's audit sub-issue → next group. The audit
sub-issues (#377-#393) and fix issues live under milestone v3.12.0.
