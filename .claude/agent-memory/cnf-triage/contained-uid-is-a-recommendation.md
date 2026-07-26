---
name: contained-uid-is-a-recommendation
description: The uid copied into a served top-level object (COMPOSITION/EHR_STATUS) is SHOULD-strength in every released source — it cannot gate CORE
metadata:
  type: project
---

The `uid` on a VERSION-contained top-level object is **never required** by any
released component (confirmed first-hand 2026-07-27):

- `RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc` — `uid` is `0..1`.
- `RM/docs/common/master03-archetyped_package.adoc` §Unique Node Identification —
  "it is **recommended** to set the `_uid_` value to a copy of the
  `_uid.object_id()_` value of the owning `VERSION` object … i.e. the leading
  Uid"; same section: "The `_uid_` attribute will usually be empty in most EHR
  data in most openEHR EHR systems."
- `ITS-REST .../docs/overview/Resources.md` §Identifier types NOTE — "it is
  **strongly recommended** that the inherited `uid` attribute **in COMPOSITION
  objects** be populated … this value **should** be copied" (scoped to
  COMPOSITION; no MUST/SHALL anywhere).
- `RM/docs/UML/classes/org.openehr.rm.ehr.ehr_status.adoc` NOTE — "**strongly
  recommended** … using the UID copied from the `_object_id()_` of the `_uid_`
  field of the enclosing VERSION object", but its worked example copies the FULL
  three-part `87284370-…::uk.nhs.ehr1::2`. That NOTE therefore contradicts
  ITSELF (wording=object_id, example=full) and its example AGREES with ITS-REST
  — the AMB-65 `source:` field misstates this as "the same object_id()-only rule
  restated for EHR_STATUS".

So AMB-65 legitimately fixes the FORM *given presence*; it does not and cannot
make PRESENCE a requirement.

SUT shape (2026-07-27): `service/ehr/meta.rs::with_uid` injects the full
three-part OBJECT_VERSION_ID on **bare** reads only; the ORIGINAL_VERSION
envelope serves `read.canonical` verbatim
(`versioning/wire.rs::build_original_version`), which has no uid — the builder is
shared with the commit-time signer, so a read-time injection would break the
signature; the only sound app-side fix would stamp the uid into the STORED
canonical before signing.

**How to apply:** any assertion of `data/uid/...` presence must be non-gating
(the runner's mechanism is `verdict.rs::is_report_only` — a case whose
`ambiguities:` names a `report_only` register entry gets `gating = false`).
A `capabilities: [EhrStatus]` tag makes a case CORE-gating regardless of its
`profiles:` field (`vocab/capability_matrix.yaml`: EhrStatus tier CORE).
