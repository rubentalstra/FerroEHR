---
name: console-wire-spec-check
description: "Owner correction 2026-07-26: every admin-UI feature implicitly claims the SERVER wire is spec-right — verify each newly consumed endpoint against the vendored docs text at integration time; a divergence is fixed in the CDR (+ CNF case), never accommodated in the UI"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: dbe2cdfe-9ee8-494a-a4d0-c26ced12a9cf
  modified: 2026-07-26T07:44:57.856Z
---

Owner correction (2026-07-26, during the #152 console program): the console
work was verifying UI discipline (Leptos rules, hydration, e2e) but not
consistently re-deriving the WIRE behaviour from the official openEHR docs.

**Why:** the console is a REST client — every feature it ships is an implicit
claim that the CDR's wire is spec-conformant. If the server diverges, building
UI on top bakes the divergence in; the official docs are always right, so the
fix belongs in the SERVER (and a CNF catalogue case to pin it), never in the
client accommodating it. Same law as CNF red-run triage
(`.claude/rules/cnf-triage.md`): the server is a suspect, never the reference.

**How to apply:**
- At every console-feature integration, take the worker's "wire calls" list
  and verify each endpoint's behaviour first-hand against the vendored
  ITS-REST docs text + RM (`/spec-lookup`); `docs/endpoint-map.md` is an
  index for finding things, never the authority — check the source + spec.
- A found divergence → CDR-side fix (or tracker issue) + a CNF coverage case
  per `.claude/rules/testing.md` §CNF coverage; the UI is built against the
  SPEC-required behaviour.
- Worker briefs must hand spec paths + verified source paths; workers cite
  those, never endpoint-map/OAS yaml (see [[session-workflow-gotchas]]).
- Related precedent: the terminology-browser brief was course-corrected the
  same day (bundle vs external FHIR TS routing verified in
  `service/terminology/routing.rs` after first citing only endpoint-map).
