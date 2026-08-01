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
