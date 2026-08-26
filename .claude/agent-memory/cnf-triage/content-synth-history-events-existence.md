---
name: content-synth-history-events-existence
description: Runner defect — content_synth HISTORY builders hardcode events existence 1..1, failing zero-event+summary rows
metadata:
  type: project
---

Confirmed 2026-07-24 (branch feat/cnf-catalogue-audit). The 4 red
CONT-HIST-events_card_{any,opt}-summary_ex_{mand,opt} rows (all row 3 =
summary present, events_count 0, expected accepted) are a RUNNER machinery
defect in Veredictum's `src/exec/content_synth.rs`.

`observation_history` (L427) and `history_events_cardinality_summary` (L553)
hardcode the `events` C_MULTIPLE_ATTRIBUTE **existence** to `(1,1)`, unlike
the sibling `composition_content_cardinality` (L487) which correctly makes
existence follow the cardinality token (`0..1` for `any`/`opt`, `1..1` for
`1plus`/`3plus`/`mand`/`3to5`). Because canonical JSON omits empty lists and
the app's `attr_absent` (openehr-flat validation/mod.rs L1404-1409) treats
`events: []` as ABSENT, `check_existence` (L1018, fires on existence min>=1)
correctly rejects a zero-events instance under existence 1..1 —
`validation_failed`. So the SUT is spec-correct; the runner over-tightened
the template.

Spec: RM `data_structures` HISTORY (`org.openehr.rm.data_structures.history.adoc`)
— events multiplicity 0..1, `Events_valid: (events /= Void and then not
events.is_empty) or summary /= Void`; summary present satisfies it at zero
events. The generated `history_basic_core` (openehr-rm validate/generated.rs
L324) implements this correctly — the RM invariant layer is NOT the defect.

Only `any`(0..*)/`opt`(0..1) fail: their 0-event rows are expected accepted;
`1plus`/`3plus`/`3to5`/`mand` pass because their 0-event rows are
expected-rejected regardless (cardinality lower bound), so outcome matches.

Fix: content_synth events existence follows the cardinality token (mirror
composition_content_cardinality's content_exist logic). Confirming capture:
re-run row 3, expect the 422 body "mandatory attribute 'events' is missing
(existence 1..)".
