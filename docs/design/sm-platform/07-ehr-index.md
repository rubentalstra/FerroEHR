# EHR Index Service (EHR_INDEX) — spec-compliance audit (W-3)

Read-only audit of our `I_EHR_INDEX` realization against the vendored SM
Platform Service Model. Unlike the Subject Proxy skeleton, the EHR Index is a
**substantially complete, spec-faithful** implementation: all five SM
operations are present, the `RESOURCE_STATUS` / `RESOURCE_INSTANCE_TYPE`
metadata is fully modelled, and the N:M semantics are tested against a real
PG18. The gaps below are narrow — error-name granularity, absence of a wire
surface + ECC evidence, and unimplemented duplicate-detection — not missing
behaviour.

**Spec oracle** (read these before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master07-ehr_index_service.adoc`
  (the chapter: overview, N:M error conditions, class-definition includes)
- `docs/specs/openehr/SM/docs/UML/classes/i_ehr_index.adoc`
  (`I_EHR_INDEX`: the five operations, signatures, declared errors)
- `docs/specs/openehr/SM/docs/UML/classes/resource_status.adoc`
  (`RESOURCE_STATUS`: `instance_type [1]`, `start_valid_time [0..1]`,
  `end_valid_time [0..1]`, `notes [0..1]`)
- `docs/specs/openehr/SM/docs/UML/classes/resource_instance_type.adoc`
  (`RESOURCE_INSTANCE_TYPE` enumeration: Primary / Duplicate / Supplementary)
- `docs/specs/openehr/SM/docs/UML/classes/location_desc.adoc`
  (`LOCATION_DESC` — an **empty stub**: a class with a description and **no
  attributes**)
- Adjacent: `master07` §Overview references a demographic/MPI service (subject
  id as key) and `OBJECT_REF` (BASE base_types) for the subject argument.

**Current implementation** (verified 2026-07-12):

- Trait (SM native API):
  `app/ehrbase-sm/src/services/ehr_index.rs:26-107` (`EhrIndexService`, 5 SM
  ops + 2 design-filled reads, all defaulting to `NotImplemented`/empty).
- Information structures: `app/ehrbase-sm/src/types.rs:455-572`
  (`SubjectRef`, `ResourceInstanceType`, `ResourceStatus`, `LocationDesc`,
  `EhrIndexEntry`).
- Trait adapter: `app/ehrbase/src/service/api/ehr_index.rs:23-77`
  (parses `ehr_id`, delegates to the domain layer).
- Domain logic: `app/ehrbase/src/service/ehr_index.rs:44-261`
  (upsert/update/remove + reads over the `ehr_index` table).
- Storage: `app/ehrbase/migrations/ehr/0001_baseline.sql:525-551`
  (`ehr_index` table); `Iden` at `app/ehrbase/src/db/iden.rs:159-162`.
- Wire (`ehrbase-rest`): **none** — `EhrIndexService` is only imported into the
  `Platform` supertrait bound (`app/ehrbase-rest/src/lib.rs:44`); no
  `/ehr_index` routes exist. ITS-REST vendors no EHR Index binding.
- Tests: `app/ehrbase/tests/service_sm3.rs:326-512` (4 integration tests:
  defaults + reads, N:M, status/loc update + remove, subject-wide remove +
  unknown-EHR/unknown-subject errors); migration-applies check in
  `app/ehrbase/tests/persistence.rs:122`.
- ECC: **zero** EHR Index cases (`tools/conformance` has no `ehr_index`
  references).

---

## 1. Faithful realizations (verified, for the record)

Recorded because the audit brief requires honest crediting of what is correct,
not only gaps.

| Item | Spec | Evidence |
|------|------|----------|
| **All 5 SM operations present with matching signatures.** `add_ehr_subject`, `update_ehr_subject_status`, `update_ehr_subject_loc_desc`, `remove_ehr_subject`, `remove_subject` — argument sets match `i_ehr_index.adoc` (ehr_id: UUID, subject: OBJECT_REF, optional status/loc). | `i_ehr_index.adoc:16-72` | `ehr_index.rs:30-94` (trait); `api/ehr_index.rs:25-67` (adapter); `service/ehr_index.rs:58-176` (domain) |
| **`RESOURCE_STATUS` fully modelled** — mandatory `instance_type` + optional `start_valid_time`/`end_valid_time`/`notes`, all four attributes present and round-tripped through storage. | `resource_status.adoc:15-29` | `types.rs:519-529`; stored `service/ehr_index.rs:70-90`; reassembled `service/ehr_index.rs:233-240` |
| **`RESOURCE_INSTANCE_TYPE` enumeration complete** — Primary/Duplicate/Supplementary, default Primary, DB CHECK constraint enforces the three values. | `resource_instance_type.adoc:15-25` | `types.rs:481-511`; `0001_baseline.sql:545` (`ck_ehr_index_instance_type`) |
| **N:M associations supported both directions** — PK `(ehr_id, subject_id, subject_namespace)` permits many subjects per EHR and many EHRs per subject; both `master07` error cases are representable. | `master07-…adoc:13-16` | `0001_baseline.sql:544`; test `service_sm3.rs:356-397` (two subjects on one EHR; one subject on two EHRs) |
| **`remove_ehr_subject` narrow vs `remove_subject` wide** — one drops a single association (subject may remain on other EHRs), the other drops all. | `i_ehr_index.adoc:59,72` | `service/ehr_index.rs:146-176`; test `service_sm3.rs:449-486` |
| **Index writes are NOT versioned objects** — plain relational writes, no CONTRIBUTION/AUDIT_DETAILS. The SM defines no versioning for the index, so this is correct (not a shortcut). | `master07` (silent on versioning); `i_ehr_index.adoc` (no version envelope) | PORT NOTE `service/ehr_index.rs:1-8`; schema comment `0001_baseline.sql:527` |
| **`add_ehr_subject` defaults to a Primary-instance status when omitted** — matches `RESOURCE_STATUS[0..1]` optionality + Primary as the authoritative kind. | `i_ehr_index.adoc:19`; `resource_status.adoc:15` | `service/ehr_index.rs:66-67`; test `service_sm3.rs:326-354` |

---

## 2. Gap register (what is not spec-true today)

Every gap cites the governing spec text. None of these is a missing SM
operation; they are conformance-surface and honesty gaps.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **The two declared errors collapse to one generic status.** `i_ehr_index.adoc` declares distinct `ehr_id_does_not_exist` and `subject_id_does_not_exist` errors per operation. The domain layer raises `ServiceError::NotFound` for **both** the unknown-EHR check and the missing-association check, and `From<ServiceError> for SmError` maps every `NotFound` to `CallStatusType::VersionedObjectDoesNotExist` (`versioned_object_does_not_exist`). The dedicated `EhrIdDoesNotExist` / `SubjectIdDoesNotExist` variants **exist in the enum but are never used by this service**. | `i_ehr_index.adoc:36-37,50-52,64-66,77-78`; error names at `ehrbase-sm/src/error.rs:134,155` | `service/ehr_index.rs:53,214-218` both `NotFound`; collapsed at `service/mod.rs:339`; tests assert only the generic `VersionedObjectDoesNotExist` (`service_sm3.rs:459-462,495-498,506-510`) — the granularity is not tested because it is not produced. |
| G-2 | **`update_*` / `remove_ehr_subject` do not distinguish unknown-EHR from unknown-association.** All three first call `ehr_exists` (→ generic 404) then, on zero rows affected, raise the same generic 404 labelled "subject_id_does_not_exist" in prose only. A caller cannot tell which precondition failed. | `i_ehr_index.adoc:50-52,64-66,77-78` (two separate errors per op) | `service/ehr_index.rs:102,119,130,141,161,212-218` |
| G-3 | **Duplicate/error-state detection is not implemented — only representable.** `master07` §Overview says the dangerous "multiple subject identifiers for one EHR" and the "multiple EHRs for a given subject" states "**need to be detected and rectified**". We store `instance_type` (Primary/Duplicate/Supplementary) so an operator *can* flag them, but nothing detects a second Primary subject on one EHR, nor surfaces the N:M conflict at write time. | `master07-…adoc:13-18` | `service/ehr_index.rs:58-92` (upsert never inspects sibling rows); no detection query anywhere. |
| G-4 | **No wire surface + zero ECC evidence.** ITS-REST vendors no EHR Index binding, so this is native-API-only by the spec's silence — but the service is therefore untested end-to-end and contributes no conformance evidence. `master07`'s "usually used as a key into a demographic or MPI service" round-trip is exercised only in Rust integration tests. | `master07-…adoc:11`; ITS-REST (no EHR Index path) | `api/ehr_index.rs:4-6` ("No wire is mounted this phase"); no `ehr_index` in `tools/conformance`. |
| G-5 | **`LOCATION_DESC` is a designed contract over an empty spec stub — undocumented as an extension in code.** The SM class defines **no attributes** (`location_desc.adoc` is description-only). We invent `{system_id, uri?, description?}`. This is legitimate (the spec is silent), and there is a PORT NOTE at `types.rs:545-547`, but the schema comment (`0001_baseline.sql:541`) and JSON shape (`service/ehr_index.rs:34-42`) should each carry the "no openEHR spec governs this — our own design" flag rather than a bare "(PORT NOTE)". | `location_desc.adoc:1-11` | Contract at `types.rs:549-556`; stored as jsonb `0001_baseline.sql:542`. |
| G-6 | **Subject is reduced from `OBJECT_REF` to three fields.** The SM types the subject argument as a full `OBJECT_REF` (id: OBJECT_ID, namespace, type). We carry `{id: String, namespace, type}` and key associations on `(id, namespace)`. This drops the `OBJECT_ID` subtype distinction (HIER_OBJECT_ID vs GENERIC_ID etc.) and cannot round-trip a structured object id. Documented as a PORT NOTE. | `i_ehr_index.adoc:18,27,42,57,70` (OBJECT_REF); `types.rs:452-454` | `types.rs:455-463` |
| G-7 | **`add_ehr_subject` is a silent idempotent upsert.** The spec verb is "Add"; our `ON CONFLICT … DO UPDATE` silently overwrites an existing association's status + location on a repeat call rather than surfacing that the association already existed. Acceptable (the spec does not forbid it and cardinality is `0..1`), but the "add" vs "already present" outcome is invisible to the caller and untested. | `i_ehr_index.adoc:15-22` | `service/ehr_index.rs:70-90` (`ON CONFLICT (ehr_id, subject_id, subject_namespace) DO UPDATE`) |
| G-8 | **No auto-population from EHR creation; the index and `ehr.subject_id` are fully decoupled.** `master07` §Overview frames the index as *the* mechanism to recover a subject id from an EHR id in a privacy-supporting deployment. Creating an EHR with a subject sets `ehr.subject_id`/`subject_namespace` (`service/ehr.rs:97-110`) but writes **no** `ehr_index` row — the index is populated only by explicit `add_ehr_subject` calls. This is a deliberate decoupling (PORT NOTE `service/ehr_index.rs:6-8`), but means the index is empty for every EHR created through the normal EHR API, undercutting the "obtain the subject identifier from the EHR id" use case unless callers separately register. | `master07-…adoc:11`; `service/ehr_index.rs:6-8` | No index write in the EHR-create path; `ehr_by_subject` (`service/ehr.rs:97-110`) uses the promoted column, not the index. |
| G-9 | **`start_valid_time`/`end_valid_time` typed `@@` in the SM (unresolved placeholder).** We implement them as ISO-8601 date-time strings stored `timestamptz`, with an unparseable value → 400. This is a reasonable resolution of a spec TBD and is PORT-NOTEd, but the `@@` placeholder is a genuine spec defect to record verbatim, not silently fill. | `resource_status.adoc:20,24` (`@@`); `types.rs:516-518` | Parsed `service/ehr_index.rs:23-31`; stored `0001_baseline.sql:538-539` |

---

## 3. Target design (closing the gaps)

The service is close to done; the plan is corrective, not a rebuild.

### 3.1 Error granularity (G-1, G-2) — the substantive fix

Split the single `NotFound` into the two spec errors the interface declares:

- `ehr_exists` failure → an EHR-scoped error mapping to
  `CallStatusType::EhrIdDoesNotExist` (`ehr_id_does_not_exist`).
- zero-rows-affected on an association write, and `remove_subject` on an
  unknown subject → `CallStatusType::SubjectIdDoesNotExist`
  (`subject_id_does_not_exist`).

Both variants already exist (`ehrbase-sm/src/error.rs:134,155`); the fix is to
carry them from the domain layer instead of the generic `NotFound`. Concretely,
add `ServiceError` variants (or a typed enum) that map 1:1 in `service/mod.rs`
so the SM status is not flattened. Update the four existing tests to assert the
specific status. *(Cite `i_ehr_index.adoc` §Errors in the code, not an ADR.)*

### 3.2 Duplicate detection (G-3)

On `add_ehr_subject` / `update_ehr_subject_status`, when the incoming
`instance_type` is `Primary`, query for an existing **different** Primary
subject on the same EHR (the "multiple subjects for one EHR" danger) and for
existing Primary EHRs for the same subject (the "multiple EHRs for a subject"
case). The spec says these "need to be detected"; a minimal, spec-honest
realization is a design-filled read
(`ehr_index_conflicts(ehr_id) -> Vec<EhrIndexEntry>`) plus a warning surfaced in
the response/log — **not** a hard reject (the spec calls them error *conditions*
to detect and rectify, not writes to forbid). Flag as "no openEHR spec defines
the detection algorithm — our own design over the master07 requirement".

### 3.3 Wire + ECC (G-4)

If EHR Index is to contribute conformance evidence, add a **config-gated
extension** API (the `/terminology` extension pattern — out of CORE/STANDARD
scope, documented as an extension since ITS-REST has no EHR Index binding):

```
POST   /rest/ehr_index/associations                add_ehr_subject
PUT    .../{ehr_id}/{subject}/status               update_ehr_subject_status
PUT    .../{ehr_id}/{subject}/location             update_ehr_subject_loc_desc
DELETE .../{ehr_id}/{subject}                       remove_ehr_subject
DELETE .../subjects/{subject}                       remove_subject
GET    .../ehrs/{ehr_id}/subjects                   ehr_subjects   (design-filled)
GET    .../subjects/{subject}/ehrs                  subject_ehrs   (design-filled)
```

with a new ECC `EHR_INDEX` area (register → N:M → duplicate-detection →
error-name assertions). Alternatively, keep native-API-only and record that the
2 SM read ops + all 5 writes are covered by the `service_sm3.rs` integration
tests as the evidence of record — but then G-4 stays open as a documented
conformance-surface limitation.

### 3.4 Documentation-only corrections (G-5, G-6, G-9)

- Rewrite the `LOCATION_DESC` schema comment and the `SubjectRef`/valid-time
  PORT NOTEs to the exact "no openEHR spec governs this — our own design" /
  "spec defect (`@@` placeholder)" phrasings the hard rules mandate. The
  `docs/design/sm-platform/03-…` and `08-…` citations inside
  `types.rs`/`ehr_index.rs` should be scrubbed if those design docs no longer
  exist (mirror the Subject Proxy cleanup).

### 3.5 Auto-population decision (G-8)

Decide explicitly (and PORT-NOTE the outcome): either (a) on EHR creation with a
subject, insert a Primary `ehr_index` row in the same transaction — making the
index the live subject↔EHR map `master07` describes — or (b) keep the index a
separate operator-managed registry and document that clients must call
`add_ehr_subject`. Option (a) better realizes the `master07` §Overview use case;
option (b) is the current behaviour and must be stated as intentional rather
than left implicit.

---

## 4. Standing PORT NOTEs (the honest residue)

- **`LOCATION_DESC` contract** `{system_id, uri?, description?}` is our own
  design over an attribute-less spec stub (`location_desc.adoc`).
- **`RESOURCE_STATUS.start/end_valid_time`** are resolved to ISO-8601 date-time
  from the SM's unresolved `@@` placeholder (`resource_status.adoc:20,24`) — a
  recorded spec defect.
- **Subject modelled as `{id, namespace, type}`**, not a full `OBJECT_REF` —
  the `OBJECT_ID` subtype is not preserved (`i_ehr_index.adoc`).
- **Index entries are not versioned objects** — no CONTRIBUTION/AUDIT_DETAILS;
  the SM defines no versioning for the index (correct realization of spec
  silence).
- **No ITS-REST binding** — EHR Index is native-API-only by the spec's silence;
  any wire is an extension out of CORE/STANDARD conformance scope.
- **The two design-filled reads** (`ehr_subjects` / `subject_ehrs`) fill the
  SM's absence of any read operation — flagged on the trait
  (`ehr_index.rs:19-20`).

---

## 5. Verdict

**Substantially compliant.** All five `I_EHR_INDEX` operations, the full
`RESOURCE_STATUS` metadata, the `RESOURCE_INSTANCE_TYPE` enumeration, and the
N:M semantics are implemented and tested against a real PG18. The material gap
is **G-1/G-2 (error-name granularity)**: the interface's declared
`ehr_id_does_not_exist` / `subject_id_does_not_exist` errors are both flattened
to the generic `versioned_object_does_not_exist`, even though the precise
variants already exist in the enum. Everything else is a conformance-surface
absence (no wire, no ECC — G-4), an unimplemented advisory (duplicate detection
— G-3), or documentation hygiene (G-5/G-6/G-9). No SM behaviour is missing.
