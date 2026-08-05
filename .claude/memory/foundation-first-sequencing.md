---
name: foundation-first-sequencing
description: "Owner 2026-08-02 — systemic defects found by audits get a full-repo fix phase BEFORE the program continues; typed-codec sweep #1686 is the live case"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: af8ec1a8-3953-4ae1-a5d1-355a712f597b
  modified: 2026-08-05T08:12:22.190Z
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

**The standing conventions the first such sweep pinned (owner 2026-08-02,
#1695 — the strict-typing mandate covers the `openehr-*` crates
themselves):** the emitter emits `Option<Vec<T>>` for a 0..1 list and a
non-empty container for 1..*, every generated type constructs through a
validated `new() -> Result` (invariant cores + terminology), and strong types
are used wherever the spec closes a set — so construction IS validation and
no downstream re-check is needed. Canonical WIRE bytes stay
proven-unchanged by the contract gates; everything internal may break freely.

**Reaffirmed + extended 2026-08-05 (during #1935):** "parse, don't validate"
is the standing doctrine — an invalid object must be UNCONSTRUCTIBLE, and a
manual check that restates a construction fact is duplication to DELETE once
the typed door covers its lane (keep only checks serving lanes the door
skips, e.g. the master06 553|incomplete| relaxation, or raw-JSON-only rules).
Where the spec deliberately OPENS a set (ACCESS_CONTROL_SETTINGS schemes),
the answer is a validated open carrier emitted by the generator
(`OpenSubtype`), never a raw-Value hole and never a closed-set refusal of
legal instances.
Related: [[owner-work-style]] (no quick fixes, big-bang convergence).
