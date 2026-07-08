# 06 — Versioning / CONTRIBUTION / AUDIT (RM change_control)

## Summary

Audit of the versioning + contribution + audit model in
`app/ehrbase/src/service/` (`vobject.rs`, `versioned.rs`, `contribution.rs`,
`ehr.rs`, `composition.rs`, `directory.rs`) and the schema
(`app/ehrbase/migrations/ehr/0001_schema.sql`) against the vendored openEHR
RM `common.change_control` + `common.generic` packages, the `ehr` package, the
BASE `identification` package, and the openEHR terminology bundle.

The **storage substrate is sound and in several respects better than EHRbase**:
logical deletion is version-based (a `deleted` row, never a physical delete —
matches `change_control` §"Logical Deletion"); `ALL_VERSIONS` is a first-class
query over one temporal table; time-travel (`version_at`) keys on commit time
(`sys_period` lower-bound = `now()` at insert), which is exactly the spec's
"previous informational state … the time of committal to an EHR server"
(`change_control` §"Committal and Audits" + `ehr` §"Historical Views of the
Record"); every write emits a `contribution` + `audit` row in the same
transaction; the `OBJECT_VERSION_ID` is a correct 3-part
`object_id::creating_system_id::version_tree_id`; `VERSION.owner_id ==
uid.object_id` holds.

However, the **RM object *rendering* at the REST edge is materially
non-conformant**: the `ORIGINAL_VERSION` and `AUDIT_DETAILS` objects returned to
clients omit or misencode several mandatory attributes and violate three named
RM invariants. The most serious are (1) `ORIGINAL_VERSION` is emitted without
its mandatory `commit_audit`, (2) `AUDIT_DETAILS.change_type.defining_code.
code_string` carries the rubric text (`"creation"`) instead of the numeric
terminology code (`"249"`), violating `AUDIT_DETAILS.Change_type_valid`, and
(3) `preceding_version_uid` is never emitted, violating
`VERSION.Preceding_version_uid_validity` for every version after the first.
Separately, the version **lifecycle state machine is collapsed to a boolean**
(`vo_version.deleted`), so `lifecycle_state` is always rendered `532|complete|`
— even for deleted versions — and `incomplete`/`inactive`/`abandoned` cannot be
represented at all.

Counts: **3 critical, 4 major, 4 minor, 3 info.**

## Findings

### F-06-01: `ORIGINAL_VERSION` returned without its mandatory `commit_audit`
- **Severity:** critical
- **Spec:** `RM/docs/common/master06-change_control_package.adoc` §"Class
  Descriptions" → `VERSION` class, attribute `commit_audit: AUDIT_DETAILS`
  cardinality **1..1**; §"Committal and Audits" (every Version records the local
  commit audit).
- **Code:** `app/ehrbase/src/service/versioned.rs:80-104`
  (`original_version`), fed by `app/ehrbase/src/service/vobject.rs:84-92`
  (`VersionRead`) and `read_version` (`vobject.rs:496-521`).
- **Problem:** `VERSION.commit_audit` is a mandatory (1..1) attribute of every
  `VERSION`. The `original_version()` builder emits only `uid`, `contribution`,
  `lifecycle_state`, and `data`. `VersionRead` never loads the version's audit
  columns (`read_version` selects `ehr_id, deleted, contribution_id` but not
  `audit_id`/audit fields), so the builder cannot populate `commit_audit`. Every
  `.../version/{uid}` response (COMPOSITION, EHR_STATUS, DIRECTORY) therefore
  returns a structurally invalid `ORIGINAL_VERSION`. The audit data *is* stored
  (`vo_version.audit_id → audit`); it is simply not read back or rendered.
- **Fix:** join `audit` on `vo_version.audit_id` in `read_version`/`version_at`/
  `read_current`, carry `system_id/change_type/description/committer/
  time_committed` on `VersionRead`, and emit `commit_audit` via the existing
  `EhrbaseService::audit_details(...)` helper inside `original_version()`.
- [x] fixed

### F-06-02: `change_type.defining_code.code_string` is the rubric text, not the terminology code
- **Severity:** critical
- **Spec:** `RM/docs/common/master04-generic_package.adoc` →
  `AUDIT_DETAILS` invariant **`Change_type_valid`**:
  `terminology(Terminology_id_openehr).has_code_for_group_id(Group_id_audit_change_type, change_type.defining_code)`;
  terminology group members in
  `TERM/computable/XML/en/openehr_terminology.xml` (`group openehr_id=
  "audit_change_type"`): `249 creation`, `250 amendment`, `251 modification`,
  `252 synthesis`, `523 deleted`, `666 attestation`, `253 unknown`,
  `816 restoration`, `817 format conversion`. `change_control` §"Contributions"
  binds the change kinds to codes `249|creation|`, `250|amendment|`,
  `251|modification|`, `523|deleted|`.
- **Code:** `app/ehrbase/src/service/contribution.rs:207-236`
  (`audit_details`); the stored code strings originate at
  `vobject.rs:54-58` (`change_type` module: `"creation"`/`"modification"`/
  `"deleted"`) and `contribution.rs:27-33` (`Action::change_type`).
- **Problem:** `audit_details()` sets `defining_code.code_string = change_type`
  where `change_type` is the *rubric* string (`"creation"`, `"modification"`,
  `"deleted"`). The `has_code_for_group_id` invariant requires
  `code_string` to be a **code in the group** — i.e. the numeric `"249"`,
  `"251"`, `"523"`. As emitted, `code_string:"creation"` is not a member code,
  so every `AUDIT_DETAILS` (in CONTRIBUTION, REVISION_HISTORY, and — once
  F-06-01 is fixed — `commit_audit`) fails the RM invariant. The persistence
  layer stores the rubric as its canonical `change_type` value, so the numeric
  code is lost end-to-end.
- **Fix:** store/track the numeric code (or map rubric→code at the render edge):
  emit `defining_code.code_string = "249"` and `DV_CODED_TEXT.value =
  "creation"` (the rubric). Prefer persisting the numeric code in
  `audit.change_type` and mapping to a rubric for `value` via the
  `openehr-term` bundle. Also widen the accepted set to the full group
  (amendment/synthesis/unknown) so inbound `commit_audit.change_type` codes
  round-trip (see F-06-06).
- [x] fixed

### F-06-03: `preceding_version_uid` never emitted — violates `Preceding_version_uid_validity`
- **Severity:** critical
- **Spec:** `RM/docs/common/master06-change_control_package.adoc` →
  `VERSION` invariant **`Preceding_version_uid_validity`**:
  `uid.version_tree_id.is_first xor preceding_version_uid /= Void`
  (i.e. present for every non-first version, absent for the first);
  §"Version and its Subtypes" ("knows on which version in the tree it was
  based … Void if it is the first version"). `ORIGINAL_VERSION.
  preceding_version_uid` cardinality 0..1.
- **Code:** `app/ehrbase/src/service/versioned.rs:80-104`
  (`original_version`).
- **Problem:** The `ORIGINAL_VERSION` builder never sets
  `preceding_version_uid`. For any version `N > 1` the invariant *requires* it
  to be the `OBJECT_VERSION_ID` of version `N-1`
  (`{vo_id}::{system_id}::{N-1}`). Its absence makes every rendered
  non-first `ORIGINAL_VERSION` invalid. The value is trivially derivable from
  the version number the builder already has.
- **Fix:** in `original_version()`, when `read.sys_version > 1`, add
  `preceding_version_uid` = `OBJECT_VERSION_ID` for `read.sys_version - 1`
  (reuse `object_version_id(vo_id, sys_version - 1)`); leave it absent for
  version 1.
- [x] fixed

### F-06-04: version lifecycle state collapsed to a boolean; `lifecycle_state` always rendered `complete`
- **Severity:** major
- **Spec:** `RM/docs/common/master06-change_control_package.adoc`
  §"Version Lifecycle" — `ORIGINAL_VERSION.lifecycle_state` is coded from the
  openEHR `version lifecycle state` group with values `532|complete|`,
  `553|incomplete|`, `523|deleted|`, `800|inactive|`, `801|abandoned|`;
  §"Logical Deletion" — a deleted version has `lifecycle_state` = `deleted`;
  `VERSION` invariant `Lifecycle_state_valid`. Terminology group
  `version_lifecycle_state` in `openehr_terminology.xml` confirms the five codes.
- **Code:** `app/ehrbase/src/service/versioned.rs:93-101` (hardcoded
  `value:"complete"`, `code_string:"532"`); schema
  `migrations/ehr/0001_schema.sql:62` (`deleted boolean`, no lifecycle column);
  `vobject.rs:256-280` (`Change::Delete` sets `deleted = true`, no state).
- **Problem:** The lifecycle is stored only as `vo_version.deleted boolean`, so
  the full state machine (`complete`/`incomplete`/`deleted`/`inactive`/
  `abandoned`) cannot be represented. `original_version()` hardcodes
  `532|complete|` unconditionally — meaning a **deleted version's**
  `ORIGINAL_VERSION` is rendered `complete` (should be `523|deleted|`), and any
  inbound `lifecycle_state` on a contribution VERSION is silently dropped
  (`contribution.rs` reads `change_type` and `data` but never `lifecycle_state`).
  A client can never commit `incomplete` content (permitted by the spec's
  "holding area" semantics) nor `deactivate`/`abandon` a version.
- **Fix:** persist `lifecycle_state` (numeric code) on `vo_version` instead of /
  in addition to the `deleted` bool; render it from the stored value in
  `original_version()`; accept and validate inbound `lifecycle_state` against the
  `version_lifecycle_state` group on contribution commits. Minimum for
  conformance: render `523|deleted|` when `read.deleted` is true.
- [x] fixed

### F-06-05: `VERSIONED_OBJECT` returned without its mandatory `time_created`
- **Severity:** major
- **Spec:** `RM/docs/common/master06-change_control_package.adoc` → class
  `VERSIONED_OBJECT`, attribute `time_created: DV_DATE_TIME` cardinality
  **1..1** ("Time of initial creation of this versioned object").
- **Code:** `app/ehrbase/src/service/versioned.rs:65-76`
  (`versioned_object`).
- **Problem:** The `VERSIONED_OBJECT` builder emits `uid` and `owner_id` only.
  `time_created` (1..1) is missing from every `.../versioned_composition`,
  `.../versioned_ehr_status`, and directory versioned-object response. The value
  is available — it is the `sys_period` lower bound of version 1 (equivalently
  the `time_committed` of the creating audit).
- **Fix:** carry the object's first-version commit time (min `lower(sys_period)`
  / version-1 audit `time_committed`) into `versioned_object()` and emit it as a
  `DV_DATE_TIME`. Note `versioned_object` is currently a pure fn with no DB
  access — it will need the timestamp threaded in from the caller.
- [x] fixed

### F-06-06: change kinds narrowed to creation/modification/deleted — no amendment/synthesis; correction rendered as modification
- **Severity:** major
- **Spec:** `RM/docs/common/master06-change_control_package.adoc`
  §"Contributions" (correction ⇒ `250|amendment|`, content change ⇒
  `251|modification|`, attestation ⇒ `666|attestation|`); `ehr`
  §"Versioning Scenarios" Case 1 (local correction sets change type to a
  correction/amendment); full `audit_change_type` group has 9 codes.
- **Code:** `app/ehrbase/src/service/vobject.rs:54-58` (only CREATION /
  MODIFICATION / DELETED constants); `contribution.rs:17-34` (`Action` enum has
  only Create/Modify/Delete); `contribution.rs:242-254` (`version_action`
  buckets *any* non-creation/non-deleted code into `Action::Modify`).
- **Problem:** `contribution_create` accepts a client `commit_audit.change_type`
  but only distinguishes creation/deleted from "everything else = modify"; an
  inbound `250|amendment|`, `252|synthesis|`, `666|attestation|`, etc. is
  coerced to `modification` semantics and — because the code is then re-derived
  from `Action::change_type()` — the stored/echoed code is *rewritten* to
  `"modification"`, discarding the client's actual change type. Single-object
  update paths (`composition.rs:121`, `ehr.rs:133`, `directory.rs:86`) always
  hardcode `modification`, so a correction cannot be recorded as `amendment`.
  Contribution-level audit change_type is likewise defaulted arbitrarily to
  `creation` (`contribution.rs:57`) rather than the spec's aggregate guidance.
- **Fix:** preserve the client-supplied `commit_audit.change_type` code verbatim
  (validate it is in the `audit_change_type` group) rather than re-deriving it
  from a 3-way `Action`; keep `Action` only for the storage branch
  (create/modify/delete node handling). Allow the update endpoints to pass a
  caller-specified change type where the API supports it.
- [x] fixed *(2026-07-06 W2-C — `contribution.rs::classify` resolves each
  VERSION's `commit_audit.change_type` through the single code⇄rubric home
  (`codes.rs`; `normalize_change_type`'s pass-unknown-verbatim behaviour
  replaced by a validating `change_type_code` → out-of-group tokens are now
  rejected 422, `AUDIT_DETAILS.Change_type_valid`) and **preserves the code
  verbatim** in the stored audit: `250|amendment|`, `252|synthesis|`,
  `253|unknown|`, `816`, `817` are all content-carrying commits against an
  existing object, never rewritten to `modification`. `Action` survives only
  as the storage branch. Spec-invalid combos rejected per RM `change_control`
  §"Contributions": `249|creation|` with a `preceding_version_uid` (creation
  commits a *new* VERSIONED_OBJECT); any non-`249` code without one (a first
  version is `249`); `523|deleted|` carrying data ("data attribute is set to
  Void") or missing `preceding_version_uid`; `666|attestation|` rejected as
  not-a-version-commit (Stage-1 — no ATTESTATION storage, F-06-10). The
  contribution-level audit, when the client supplies none, now follows the
  spec's aggregate guidance (all-same → that code, mixture → `251`) instead of
  a hardcoded `creation`; a supplied one is validated + preserved. Unit tests
  (`classify_*`, `contribution_aggregate_change_type`) + PG e2e
  (`contribution_preserves_the_client_change_type_and_rejects_invalid_combos`)
  cite the clauses. PORT NOTE on the remaining "where the API supports it"
  half: the ITS-REST single-object update operations (PUT composition /
  ehr_status / directory) define **no** audit/change_type channel (no request
  body audit, no `openEHR-AUDIT_DETAILS` param in the 1.0.3 contract), so
  their `modification` default stands and an amendment is recorded via the
  CONTRIBUTION endpoint — which now supports it end to end.)*

### F-06-07: EHR_ACCESS not created; `EHR.ehr_access` omitted from the EHR response
- **Severity:** major
- **Spec:** `RM/docs/ehr/master04-ehr_package.adoc` §"EHR Creation" ("the
  result should be a root EHR object, an EHR Status object, **and an EHR Access
  object**"); `EHR` class `ehr_access: OBJECT_REF` cardinality **1..1** with
  invariant `Ehr_access_valid: ehr_access.type.is_equal("VERSIONED_EHR_ACCESS")`.
- **Code:** `app/ehrbase/src/service/ehr.rs:15-37` (`create_ehr` creates only
  `EHR_STATUS`); `ehr.rs:82-97` (`ehr_summary` emits `system_id`, `ehr_id`,
  `ehr_status`, `time_created` — no `ehr_access`).
- **Problem:** No `EHR_ACCESS`/`VERSIONED_EHR_ACCESS` is created at EHR creation,
  and the mandatory `EHR.ehr_access` reference is absent from the EHR summary,
  violating cardinality 1..1 and `Ehr_access_valid`. (This mirrors an EHRbase
  simplification, but the spec is the oracle here.)
- **Fix:** create a minimal `EHR_ACCESS` versioned object at EHR creation (or at
  minimum synthesize a stable `VERSIONED_EHR_ACCESS` `OBJECT_REF`) and include
  `ehr_access` in `ehr_summary`. If EHR_ACCESS is intentionally deferred, record
  a `// PORT NOTE:` citing this section and the invariant.
- [x] fixed — a **real** `EHR_ACCESS` versioned object (`Kind::EhrAccess`,
  `vo_version.kind = 'EHR_ACCESS'`, decomposed/versioned like every other
  versioned object) is created with the EHR — `EHR_STATUS` + `EHR_ACCESS` are
  committed under **one** CONTRIBUTION per §"EHR Creation"; `ehr_summary` emits
  `ehr_access` as an OBJECT_REF of type `VERSIONED_EHR_ACCESS` referencing the
  version container (`Ehr_access_valid`). Verified by `service_ehr.rs`
  `ehr_creation_produces_an_ehr_access`.

### F-06-08: `EHR.ehr_status` OBJECT_REF type is `EHR_STATUS`, not `VERSIONED_EHR_STATUS`
- **Severity:** minor
- **Spec:** `RM/docs/ehr/master04-ehr_package.adoc` → `EHR` invariant
  `Ehr_status_valid: ehr_status.type.is_equal("VERSIONED_EHR_STATUS")`.
- **Code:** `app/ehrbase/src/service/ehr.rs:86-91`.
- **Problem:** The EHR summary emits `ehr_status` as an `OBJECT_REF` with
  `type:"EHR_STATUS"` and an `OBJECT_VERSION_ID` id. The RM invariant requires
  the reference `type` to be `VERSIONED_EHR_STATUS` (a reference to the version
  *container*, not one version). There is genuine tension with the ITS-REST EHR
  example (which commonly shows `EHR_STATUS` + `OBJECT_VERSION_ID`); flagging per
  the spec-adherence mandate — cross-check the CNF EHR test schedule before
  changing, and record the decision either way.
- **Fix:** either set `type:"VERSIONED_EHR_STATUS"` with the container
  `HIER_OBJECT_ID`, or keep the REST-convention form and record a `// PORT NOTE:`
  citing the `Ehr_status_valid` invariant and the CNF EHR test case that governs
  the wire shape.
- [ ] fixed

### F-06-09: branch `version_tree_id`s unsupported; only integer trunk versions
- **Severity:** minor
- **Spec:** `RM/docs/common/master06-change_control_package.adoc`
  §"Local Versioning"/§"Version Identification"; BASE
  `version_tree_id.adoc` (`trunk_version[.branch_number.branch_version]`,
  1- or 3-part).
- **Code:** `ehr.rs:209-211` (`object_version_id` always
  `{vo_id}::{system}::{int}`); `ehr.rs:279-286` (`parse_expected_version` parses
  a trailing integer); `contribution.rs:279-305` (`parse_preceding` parses the
  version tail as `i32`).
- **Problem:** `sys_version` is a plain integer, so only trunk versions
  (`"1"`,`"2"`,…) are representable; a `preceding_version_uid` with a 3-part
  branch id (`"2.1.1"`) fails `i32` parsing and is rejected as unprocessable. The
  spec permits branching (rarely used locally — translation/merge), and merge
  fields (`other_input_version_uids`) are entirely absent. Acceptable as a
  documented Stage-1 scope boundary, but currently undocumented.
- **Fix:** record a `// PORT NOTE:` that branch/merge versioning is out of
  Stage-1 scope (trunk-only), and reject branch ids with a clear typed error
  rather than a generic "needs a version" message. Full support would require a
  string `version_tree_id` model.
- [ ] fixed

### F-06-10: attestations unsupported
- **Severity:** minor
- **Spec:** `RM/docs/common/master06-change_control_package.adoc`
  §"Attestation"; `ORIGINAL_VERSION.attestations: List<ATTESTATION>` (0..1),
  invariant `Attestations_valid`; `common.generic` §"Attestation".
- **Code:** `versioned.rs:80-104` (`original_version` never emits
  `attestations`); no attestation table or endpoint.
- **Problem:** `attestations` is optional (0..1), so omission is RM-valid, but
  there is no way to attach an attestation to a version (no `ATTESTATION`
  storage, no `666|attestation|` change path). Fine for Stage 1; noted for
  completeness against the change_control model.
- **Fix:** none required for RM validity; track as a future capability (record a
  `// PORT NOTE:` if the CNF schedule exercises attestation).
- [ ] fixed

### F-06-11: `AUDIT_DETAILS.value` uses the `version lifecycle`/`instruction` rubric collision; description-only committer for system writes
- **Severity:** info
- **Spec:** `TERM/.../openehr_terminology.xml` (known-issue note on concept
  `532`: rubric `complete` in the lifecycle group vs `completed` in the
  instruction group, SPECPR-51); `common.generic` §"Audit Details" (committer is
  a `PARTY_PROXY`).
- **Code:** `versioned.rs:96` (`value:"complete"`); `ehr.rs:253-273`
  (`committer`).
- **Problem:** Once lifecycle rendering is real (F-06-04), take the rubric from
  the `version_lifecycle_state` group (`complete`), not the instruction group
  (`completed`). The system-write committer (`PARTY_IDENTIFIED name:"EHRbase"`)
  is a valid `PARTY_PROXY`; the authenticated-principal path is well formed.
  Informational only.
- **Fix:** source rubrics from the correct terminology group via `openehr-term`.
- [ ] fixed

### F-06-12: `object_id` in `OBJECT_VERSION_ID` uses the DB `vo_id`, coupling the virtual-version-tree uid to a single system's key
- **Severity:** info
- **Spec:** `change_control` §"The 'Virtual Version Tree'" (the
  `VERSIONED_OBJECT._uid_` is the uid of the virtual version tree, identical
  across copies); BASE `object_version_id.adoc`.
- **Code:** `vobject.rs:207` (`Uuid::now_v7()` as `vo_id`); `ehr.rs:209-211`.
- **Problem:** `vo_id` (a `uuidv7` primary key) doubles as the
  `VERSIONED_OBJECT.uid` / `OBJECT_VERSION_ID.object_id`. This is spec-legal (a
  GUID version-container uid), and correct for a non-distributed system. Only
  relevant if import/copy is added later: an imported version must reuse the
  *source* container uid rather than mint a new `vo_id` (§"Copying"). No action
  now; noted so the import path (if built) preserves the incoming `object_id`.
- **Fix:** none for Stage 1; constrain future import to reuse the source
  `object_id` as `vo_id`.
- [ ] fixed

## Hygiene notes

- **Strengths worth preserving.** Version-based logical deletion (no physical
  delete), `ALL_VERSIONS` over one temporal table, commit-time time-travel, the
  correct 3-part `OBJECT_VERSION_ID`, the `owner_id.value == uid.object_id.value`
  relationship, and the one-transaction contribution+audit write are all
  faithful to `change_control`/`ehr` and are cleanly implemented in `vobject.rs`.
- **The core data is stored; the gaps are at the render edge.** F-06-01/03/05 are
  all "the value exists in the DB but the builder doesn't emit it" — cheap,
  high-value fixes concentrated in `versioned.rs` + the three `read_*` selects in
  `vobject.rs`. Do these together.
- **Terminology coding is the systemic issue.** F-06-02 (numeric code vs rubric)
  and F-06-04/F-06-11 (lifecycle codes) both stem from storing/emitting rubric
  strings where the RM invariants require *group codes*. Route all coded audit /
  lifecycle values through the `openehr-term` bundle (code ⇄ rubric) once, rather
  than string literals scattered across `vobject.rs`/`contribution.rs`.
- **`change_type` code strings live in three places** (`vobject::change_type`,
  `contribution::Action::change_type`, `contribution::version_action`) with the
  numeric codes only appearing as match arms in `version_action`. Consolidate to
  a single code⇄rubric map to avoid the current lossy rubric round-trip.
- **Suggested test additions (cite the invariant each encodes):** assert
  `AUDIT_DETAILS.change_type.defining_code.code_string` matches
  `[0-9]+` and is a member of `audit_change_type`
  (`AUDIT_DETAILS.Change_type_valid`); assert a v2 `ORIGINAL_VERSION` has
  `preceding_version_uid` and v1 does not
  (`VERSION.Preceding_version_uid_validity`); assert a deleted version renders
  `lifecycle_state = 523`; assert `VERSIONED_OBJECT.time_created` is present;
  assert the returned `ORIGINAL_VERSION` carries `commit_audit`. None of these
  is currently covered by the P12 service e2e tests.
- Cross-check every wire-shape decision (F-06-08 especially) against
  `docs/specs/openehr/CNF/docs/platform_test_schedule/` before changing — the
  CNF test case wins over a prose reading (spec-adherence.md).
