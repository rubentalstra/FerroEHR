---
name: ehr-status-archetype-root-invariant
description: EHR_STATUS/EHR_ACCESS MUST carry archetype_details — app 422 is spec-correct; red rows are catalogue payload residue
metadata:
  type: project
---

CONFIRMED spec derivation (2026-07-27 triage, RM 1.2.0 vendored text):

- `RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc` §Invariants —
  `__Archetyped_valid__: is_archetype_root xor archetype_details = Void`.
- `RM/docs/UML/classes/org.openehr.rm.ehr.ehr_status.adoc` §Invariants —
  `__Is_archetype_root__: is_archetype_root` (unconditional; same for
  `org.openehr.rm.ehr.ehr_access.adoc`).

Conjunction: `is_archetype_root = True` forces `archetype_details = Void` to be
FALSE, i.e. **archetype_details is mandatory on every EHR_STATUS / EHR_ACCESS
instance**. Plus `locatable.adoc` archetype_node_id Meaning: "At an archetype
root point, the value of this attribute is always the stringified form of the
`_archetype_id_` found in the `_archetype_details_` object" → node id must equal
`archetype_details.archetype_id.value`.

`app/ehrbase/src/service/ehr/validation.rs::validate_root_locatable` (PR #431 /
issue #423) enforces exactly this and 422s — **spec-correct, not an app defect**.

**How to apply:** any `expected: created` red row on an EHR_STATUS/EHR_ACCESS
payload is a CATALOGUE defect (the payload is RM-invalid). PR #431 patched the
JSON fixtures but missed two generator/inline sites — see
[[cnf-red-2026-07-27-ehr-status-payloads]]. Self-check available inside the
catalogue: `cnf.ehr_status.invalid.missing-archetype-details` +
`I_EHR_SERVICE.create_ehr-invalid_status` already pin the 422 as the CORRECT
outcome for the very same shape, so any other case expecting `created` for it is
self-contradictory.
