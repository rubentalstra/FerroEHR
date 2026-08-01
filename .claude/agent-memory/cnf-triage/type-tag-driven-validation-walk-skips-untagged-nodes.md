---
name: type-tag-driven-validation-walk-skips-untagged-nodes
description: APP defect class — both RM validation passes dispatch on the JSON `_type` tag, so any node whose `_type` is legitimately omitted (concretely-declared slot) escapes every invariant on the canonical-JSON commit path
metadata:
  type: project
---

Confirmed 2026-08-01 (red row
`I_EHR_COMPOSITION.create_composition-setting_invalid`, 201 instead of 422).

**Mechanism.** `crates/openehr-its/src/flat/validation/terminology.rs:38`
(`obj.get("_type")…unwrap_or("")` → `slots_for("")` = empty) and
`crates/openehr-its/src/rm_validate.rs:147`/`164` (`let Some(ty) = value.get("_type")
… else { return }`) both key on the **wire tag**. Canonical JSON only requires
`_type` where the declared slot type is abstract (ITS-JSON
`crates/openehr-its/schemas/json/openehr_rm_1.1.0_all.json`: `COMPOSITION.context`
is a bare `$ref` to `EVENT_CONTEXT`, whose `required` list is
`["start_time","setting"]` — no `_type`), so a spec-valid payload can omit it and
every invariant on that node — terminology AND core — silently does not run.
Canonical **XML** bodies go typed (`app/ferroehr-rest/src/overview/negotiate.rs:511`
→ `to_canonical_value`, which injects `_type`), so the SAME content is refused as
XML and accepted as JSON — the asymmetry is the proof it is a walk defect, not a
spec-permitted leniency.

Reproduced on the composed SUT, one EHR, `Content-Type: application/json`:
- setting 999, untagged context → **201**; `+ "_type":"EVENT_CONTEXT"` → **422**
  `/context/setting: code '999' is not a valid setting (openEHR terminology)`
- same content as canonical XML → **422** (same message)
- `PARTICIPATION.mode` 999 untagged → **201**; tagged → **422** (Mode_valid)
- `EVENT_CONTEXT.location: ""` untagged → **201**; tagged → **422**
  `Invariant location_valid failed on type EVENT_CONTEXT` (core invariants escape too)
- `COMPOSITION.category` 999 with the ROOT `_type` stripped → still 422, but only
  from the WebTemplate pass ("not in the constrained value set") — which is why the
  three sibling cases (category/language/territory) pass and setting does not:
  the OPT constrains `category`, nothing constrains RM-level `context/setting`.

**How to apply.** The fix is *effective-type resolution*, not a new slot entry:
resolve each child node's RM type as `_type` if present else
`openehr_rm::model::attribute(parent_type, field).declared_type` when that class
is concrete (`RmClass.is_abstract == false`) — the generated static model already
carries it (`crates/openehr-rm/src/model/data.rs:1451` context → EVENT_CONTEXT,
:4093 participations → PARTICIPATION). Both walk sites are HAND-WRITTEN
(`openehr-its` is a mixed crate) → normal edit, **no emitter/regen needed**.
Suspect this pattern for any red row where a nested-node invariant did not fire
on a JSON commit but the same shape is refused via XML; the escape class is every
concretely-declared slot (context, participations, archetype_details, ism_transition,
activities, mappings, reference ranges, instruction_details, …).
