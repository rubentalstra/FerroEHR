---
name: foundation-first-sequencing
description: "Owner 2026-08-02 — systemic defects found by audits get a full-repo fix phase BEFORE the program continues; typed-codec sweep #1686 is the live case"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: af8ec1a8-3953-4ae1-a5d1-355a712f597b
  modified: 2026-08-03T01:17:54.654Z
---

Owner ruling 2026-08-02 (during the RM ch.6 audits): when an audit exposes a
SYSTEMIC defect class (the live case: app code hand-crafting canonical wire
shapes with `json!` instead of constructing generated `openehr-rm` types
through the codec, and bare terminology literals instead of `openehr-term`),
do NOT keep auditing forward and fixing instances opportunistically — finish
the in-flight section, then run a FULL-REPO sweep as its own phase before the
next section starts. "Otherwise we are building on a bad foundation."

**Why:** each later audit would keep rediscovering instances of the same
class, and new code written during the program would copy the bad pattern.

**How to apply:** encode the sequencing as a native blocked-by edge (the next
audit unit blocked by the sweep issue) + P1; the sweep issue carries the
ruling in its opening. Missing constructors/constants are emitter or
openehr-term gaps fixed at the generator (visible), never app literals.
FULLY BREAKING changes are the owner's explicit expectation for such phases
(2026-08-02): every crate is publish=false, so no signature/shape/layout is
preserved for compatibility — best shape wins, wide call-site churn
included; only spec conformance (never-lax, adjudicated wire changes with
twins + changelog) and green gates are unbreakable.

**The live case COMPLETED 2026-08-03** (PR #1739, squash-merged): 7 legs —
#1686 typed codec, #1687 *_impl.rs push-down, #1690 rm→term move, #1695
container bounds + validated construction + strict reader, #1702 the serde
rewrite (manual emitted impls, reader 13.7% FASTER), #1694 errors-as-data
(partially; typed commit seam gated on owner decision #1727 — 400 vs 422
for structurally-invalid bodies), #1718. Pipeline 879/0 at close.
Related: [[owner-work-style]] (no quick fixes, big-bang convergence).
