---
name: never-lax-strictness-rule
description: "Owner hard rule 2026-08-01 — NEVER LAX; the CDR accepts exactly what the released spec admits, refusals are asserted negative tests, deprecations warn, spec-silent acceptance needs a docs-text citation chain"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 39213bee-2a89-4d7c-8dca-5e502effeeab
  modified: 2026-08-01T13:58:17.434Z
---

Owner hard rule (2026-08-01, stated with maximum emphasis during the CKM
corpus program): **we never lax the setup — as strict as possible, always.**
Codified in `.claude/rules/spec-adherence.md` §NEVER LAX.

**Why:** clinical repository; a silently loosened reader/validator is the
failure class the whole conformance apparatus exists to prevent. The owner
also rejects carrying stalled/contradictory upstream information silently
(the #1469 trigger).

**How to apply:** strict = EXACT — refuse everything the spec refuses (as
asserted negative tests pinning the error code — that pattern IS the machine
enforcement); enforce deprecations as Warning findings (#1470 pattern);
accept a spec-silent form only on a first-hand released-docs-text citation
chain (a chapter's own embedded grammar counts; stalled .g4/fixtures/Robot
material corroborates but never decides), recorded on an issue, with any
upstream contradiction filed as spec-update; weakening any refusal follows
the #1465 pattern (re-derived citations + flipped gate + accepting twin).
Inventing prohibitions the spec lacks is equally forbidden — strict in both
directions. Extends [[cnf-spec-oracle-attribution]] and
[[valid-invalid-twins-rule]].

**Extension (owner 2026-08-02, the strict-reader ruling):** the canonical
reader REFUSES unknown keys — enforcement derives from the BMM-generated
model (the generated types ARE the closed schema), implemented at the
emitter with named-key refusals + twins. The old tolerant-reader
("RM-version skew superset") rationale is retired: future-RM attributes
arrive via the spec watcher + pin bump, never via silent tolerance. A lax
upstream computable artifact (ITS-JSON open objects) is an upstream defect
to report, never a reason to accept more than the model. Parse-don't-
validate follows: typed carriers everywhere internal; construction =
validation; downstream re-checks deleted with refusals retargeted at the
constructor (#1694).
**Scope (owner 2026-08-02): the strict-typing mandate covers the openehr-*
crates themselves.** The emitter's conventions are re-adjudicated under it
(#1695): Option<Vec> for 0..1 lists (kills the JSON-level non-empty check
class), non-empty containers for 1..*, validated construction (new() ->
Result running cores + terminology) on every generated type, strong types
wherever the spec closes a set. Canonical WIRE bytes stay proven-unchanged
(the contract gates); everything internal breaks freely.
