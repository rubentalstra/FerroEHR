# 01 — REST: EHR / EHR_STATUS / VERSIONED_EHR_STATUS

## Summary

The routing, parameter binding, JSON/XML request decoding, and error→status
mapping for the EHR / EHR_STATUS / VERSIONED_EHR_STATUS groups are largely
sound, and the happy-path reads/writes work. However the surface diverges from
ITS-REST 1.0.3 in several conformance-relevant ways: **no `ETag`/`Location`
response headers are emitted anywhere** (they are mandated on EHR-create and
every EHR_STATUS response), the **`Prefer` header is completely ignored** (wrong
default status/body), one operation (`versioned_ehr_status_version_get_at_time`)
is **unimplemented (501)**, `ehr_status_get_by_version_id` returns the **wrong
payload type**, duplicate-subject EHR creation is **not rejected with 409**
(a CNF test case), and the audit/version wrappers have **wrong terminology codes
and missing mandatory fields**. These will fail CNF `master06` (EHR) test cases.

Authorities used: `docs/specs/openehr/ITS-REST/specifications/` (OAS operations,
responses, headers, parameters, schemas), `docs/specs/openehr/RM/docs/UML/classes/`
(EHR / EHR_STATUS / VERSIONED_OBJECT / VERSION cardinalities),
`docs/specs/openehr/TERM/computable/XML/en/openehr_terminology.xml` (audit change
type codes), and `docs/specs/openehr/CNF/docs/platform_test_schedule/master06-func_tc_ehr.adoc`.

## Findings

### F-01-01: No `ETag`/`Location` headers on any EHR or EHR_STATUS response
- **Severity:** critical
- **Spec:** `ITS-REST/specifications/responses/201_EHR.yaml` (`headers: ETag` → `headers/ETag_EHR.yaml`, `Location` → `headers/Location_EHR.yaml`); `responses/200_EHR_STATUS_retrieved.yaml`, `responses/200_EHR_STATUS_updated.yaml`, `responses/204_EHR_STATUS.yaml`, `responses/412_EHR_STATUS.yaml` (all declare `ETag` + `Location`); `responses/200_VERSION_at_time.yaml` (`ETag_VERSION` + `Location_VERSIONED_EHR_STATUS_VERSION`).
- **Code:** `crates/ehrbase-rest/src/negotiate.rs:293` (`respond_rm`), `:270` (`respond`), `:334` (`empty`), `:349` (`json_response`); backend trait returns bare `Value` with no header channel — `crates/ehrbase-rest/src/dispatch/ehr.rs:57-164`; `crates/ehrbase/src/service/api/ehr.rs` (every method returns `Value`). `grep -niE "etag|location" crates/ehrbase-rest/src` finds none.
- **Problem:** The response builders never set `ETag` or `Location`. Per spec, `POST/PUT /ehr` MUST return `ETag` (the `ehr_id`, quoted) and `Location` (the EHR URL); every EHR_STATUS GET/PUT (200/204) MUST return `ETag` (the `version_uid`, quoted) and `Location` (the EHR_STATUS URL); `412` MUST return the *latest* `version_uid` in both. Without `ETag` the client cannot obtain the `preceding_version_uid` needed for the `If-Match` on a subsequent update — the whole optimistic-concurrency workflow is broken, and CNF header assertions fail.
- **Fix:** Give the dispatch layer a way to carry headers out of the service. Cleanest: change the EHR/EHR_STATUS service methods (or a thin response-metadata wrapper) to return the resource plus its `OBJECT_VERSION_ID`/`ehr_id`, and have `dispatch/ehr.rs` set `ETag` (quoted uid) and `Location` (absolute resource URL built from the configured base path) on 201/200/204/412 responses. Add a `respond_rm_with_headers`/builder variant in `negotiate.rs`. The service already computes the `object_version_id` (`service/ehr.rs:209`) — surface it rather than only injecting it into the body `uid`.
- [x] fixed — W2-A typed response envelope (`ServiceResponse`/`ResourceMeta` on the `EhrService` seam) + header-aware `negotiate` helpers

### F-01-02: `Prefer` header ignored — wrong default status and body on create/update
- **Severity:** major
- **Spec:** `ITS-REST/specifications/parameters/header/Prefer.yaml` (`default: return=minimal`); `operations/ehr_create.yaml` + `responses/201_EHR.yaml` ("resource returned in the body when `Prefer` is `return=representation`, otherwise only headers"); `operations/ehr_status_update.yaml` → `200_EHR_STATUS_updated.yaml` (body only on `return=representation`) vs `204_EHR_STATUS.yaml` ("returned when the `Prefer` header is missing or is set to `return=minimal`").
- **Code:** `crates/ehrbase-rest/src/dispatch/ehr.rs:67-95` (`ehr_create`/`ehr_create_with_id` always `respond_rm` with the full EHR body), `:114-123` (`ehr_status_update` always returns `ok`=200 with body). `EhrCreateParams.prefer` / `EhrStatusUpdateParams.prefer` (`openehr-its/.../generated/ehr.rs:141,210`) are parsed but never read.
- **Problem:** The default (`return=minimal`) is not honoured. `POST/PUT /ehr` should return a bodyless 201 by default (body only when `return=representation`); `PUT .../ehr_status` should return **204 No Content** by default and 200 + body only for `return=representation`. The impl unconditionally returns 201-with-body and 200-with-body respectively — a wrong default status for the update and a spurious body for create.
- **Fix:** Read `Prefer` in the dispatch arms. For `ehr_status_update`: return `204` (bodyless, with ETag/Location) unless `Prefer: return=representation`, in which case `200` + body. For `ehr_create`/`ehr_create_with_id`: return `201` with headers only unless `return=representation`. Add a small `negotiate::prefers_representation(headers)` helper.
- [x] fixed — W2-A typed response envelope (`ServiceResponse`/`ResourceMeta` on the `EhrService` seam) + header-aware `negotiate` helpers

### F-01-03: `ehr_status_get_by_version_id` returns an ORIGINAL_VERSION instead of the EHR_STATUS
- **Severity:** major
- **Spec:** `operations/ehr_status_get_by_version_id.yaml` → `responses/200_EHR_STATUS_retrieved.yaml` → `schemas/ehr/EhrStatus.yaml`. The endpoint `GET /ehr/{ehr_id}/ehr_status/{version_uid}` returns the bare **EHR_STATUS** resource (with its `uid`), *not* a VERSION wrapper.
- **Code:** `crates/ehrbase/src/service/api/ehr.rs:78-85` calls `self.status_version(...)`, which returns `original_version(...)` (`crates/ehrbase/src/service/ehr.rs:175-186` → `service/versioned.rs:80` builds an `ORIGINAL_VERSION`). Dispatched via `respond_rm::<EhrStatus>` (`dispatch/ehr.rs:96-104`).
- **Problem:** The response payload is an `ORIGINAL_VERSION` (`{_type:"ORIGINAL_VERSION", uid, contribution, lifecycle_state, data}`), but the spec/OAS require an `EHR_STATUS`. Contrast `ehr_status_get_at_time` (`api/ehr.rs:65`) which correctly returns the bare EHR_STATUS via `status_at`. Additionally, on `Accept: application/xml` the `respond_rm::<EhrStatus>` re-type step (`negotiate.rs:308`) will fail to deserialize the ORIGINAL_VERSION into `EhrStatus` and return **500**.
- **Fix:** Route `ehr_status_get_by_version_id` to a status-by-version reader that returns the EHR_STATUS canonical value with its `uid` set (mirror `status_at` but pinned to a specific `sys_version` via `vobject::read_version`), not `status_version`/`original_version`. Keep `status_version`/`original_version` for the `versioned_ehr_status/version/{version_uid}` (VERSION) endpoint only.
- [x] fixed — W2-A typed response envelope (`ServiceResponse`/`ResourceMeta` on the `EhrService` seam) + header-aware `negotiate` helpers

### F-01-04: Duplicate-subject EHR creation not rejected with 409
- **Severity:** major
- **Spec:** `operations/ehr_create.yaml` + `responses/409_EHR.yaml` ("conflict with an already existing EHR with the same subject id, namespace pair, whenever EHR_STATUS is supplied"); CNF `master06-func_tc_ehr.adoc` Test Case `I_EHR_SERVICE.create_ehr-two_ehrs_same_patient` ("The server should answer with a negative response, related with the EHR already existing for the provided subject").
- **Code:** `crates/ehrbase/src/service/ehr.rs:15-37` (`create_ehr`) only guards `ehr.id` uniqueness (`INSERT ... ON CONFLICT DO NOTHING`); it never checks the supplied EHR_STATUS subject against existing EHRs.
- **Problem:** `POST /ehr` with an EHR_STATUS whose `subject.external_ref` matches an existing EHR returns 201 instead of 409. Fails the CNF test case.
- **Fix:** In `create_ehr`, when the supplied status carries a `subject.external_ref` (id.value + namespace), run the `ehr_by_subject` lookup (already implemented, `service/ehr.rs:41`) inside the transaction before insert; if a match exists, return `ServiceError::Conflict` (→ 409). Do this only when EHR_STATUS is client-supplied (the default PARTY_SELF has no external_ref, so no conflict).
- [ ] fixed

### F-01-05: `versioned_ehr_status_version_get_at_time` unimplemented → 501
- **Severity:** major
- **Spec:** `operations/versioned_ehr_status_version_get_at_time.yaml` (`GET /ehr/{ehr_id}/versioned_ehr_status/version`, query `version_at_time`) → `200_VERSION_at_time.yaml` / `400_invalid_version_at_time.yaml` / `404_unknown_ehr_id_or_no_version_at_time.yaml`. Route registered at `openehr-its/.../generated/ehr.rs:908`.
- **Code:** `crates/ehrbase-rest/src/dispatch/ehr.rs:143-153` dispatches to `backend().versioned_ehr_status_version_get_at_time(p)`, but `crates/ehrbase/src/service/api/ehr.rs` never overrides it (the param type isn't even imported), so it inherits the generated `NotImplemented` default → **501**.
- **Problem:** A required, CNF-scheduled read (latest VERSION, or the VERSION extant at a given time) returns 501. This is the VERSION-returning sibling of `ehr_status_get_at_time` and must return an `ORIGINAL_VERSION` (with `ETag`/`Location`, per F-01-01).
- **Fix:** Implement `versioned_ehr_status_version_get_at_time` on `EhrbaseService`: resolve the EHR_STATUS `vo_id`, pick the version current at `version_at_time` (or latest), and return `original_version(...)`. Reuse `vobject::version_at` + the loaded `VersionRead`. Validate `version_at_time` format → 400; unknown EHR / no version at time → 404.
- [ ] fixed

### F-01-06: AUDIT_DETAILS `change_type` uses the rubric string as the terminology code
- **Severity:** major
- **Spec:** `docs/specs/openehr/TERM/computable/XML/en/openehr_terminology.xml:32-36` (audit change type group: `249 creation`, `251 modification`, `523 deleted`); `ITS-REST/.../schemas/common/AuditDetails.yaml` `change_type` is a `DV_CODED_TEXT`; the CNF/OAS example `schemas/common/RevisionHistoryItem.yaml` shows `change_type: { value: creation, defining_code: { terminology_id: {value: openehr}, code_string: '249' } }`.
- **Code:** `crates/ehrbase/src/service/contribution.rs:218-226` sets `defining_code.code_string = change_type` where `change_type` is the rubric string; `crates/ehrbase/src/service/vobject.rs:54-58` defines `CREATION="creation"`, `MODIFICATION="modification"`, `DELETED="deleted"`.
- **Problem:** The emitted `defining_code.code_string` is `"creation"`/`"modification"`/`"deleted"` — not a valid openEHR terminology code. The `code_string` MUST be `"249"`/`"251"`/`"523"` (with the rubric in `value`). This corrupts every `AUDIT_DETAILS` in `REVISION_HISTORY` and VERSION responses, and would fail terminology-binding validation of the audit.
- **Fix:** Change the `change_type` constants (or `audit_details`) to store the numeric code, and set `value` = rubric ("creation") while `code_string` = code ("249"). Best: a small map (`creation→249`, `modification→251`, `deleted→523`) in `vobject::change_type`, and have `audit_details` emit `{ value: rubric, defining_code.code_string: code }`. Verify against `openehr-term`'s `is_valid_audit_change_type` (`crates/openehr-term/src/bundle.rs:700`).
- [x] fixed

### F-01-07: VERSION responses omit the mandatory `commit_audit` (AUDIT_DETAILS)
- **Severity:** major
- **Spec:** `ITS-REST/.../schemas/common/Version.yaml` — `required: [contribution, commit_audit, data]`. `ORIGINAL_VERSION` (`schemas/common/OriginalVersion.yaml`) is `allOf` Version.
- **Code:** `crates/ehrbase/src/service/versioned.rs:80-104` (`original_version`) emits `uid`, `contribution`, `lifecycle_state`, `data` — but no `commit_audit`.
- **Problem:** Every VERSION returned by `versioned_ehr_status_version_get_by_id` (and, once fixed, `_at_time`) is missing the mandatory `commit_audit`. A conformant client/validator will reject the payload.
- **Fix:** Include `commit_audit` in `original_version` by loading the version's audit columns (the same data `revision_history` already reads) and emitting `Self::audit_details(...)`. `VersionRead` should carry (or the reader should join) `system_id`/`change_type`/`description`/`committer`/`time_committed` so `original_version` can build the AUDIT_DETAILS. Also populate `preceding_version_uid` when `sys_version > 1` (OriginalVersion optional but expected for modifications).
- [x] fixed

### F-01-08: VERSIONED_EHR_STATUS omits the mandatory `time_created`
- **Severity:** major
- **Spec:** `ITS-REST/.../schemas/common/VersionedObject.yaml` — `required: [uid, owner_id, time_created]`. RM `org.openehr.rm.common.change_control.versioned_object` — `time_created` is 1..1.
- **Code:** `crates/ehrbase/src/service/versioned.rs:65-76` (`versioned_object`) emits only `uid` + `owner_id`.
- **Problem:** `GET /ehr/{ehr_id}/versioned_ehr_status` returns a VERSIONED_OBJECT missing the mandatory `time_created` (the commit time of version 1). Fails schema validation.
- **Fix:** Load version 1's `time_committed` for the object and add `time_created: { _type: DV_DATE_TIME, value: <ts> }`. `versioned_object` is a free function taking only ids; give it the timestamp (or make it a method that queries the first version's audit time).
- [x] fixed

### F-01-09: `If-Match` precondition is bypassable and only compares the version number
- **Severity:** minor
- **Spec:** `ITS-REST/specifications/parameters/header/If-Match.yaml` ("The operation will be performed only if the existing latest `version_uid` … matches this header's value"; format is the full quoted `OBJECT_VERSION_ID`). `operations/ehr_status_update.yaml` → `412_EHR_STATUS.yaml`.
- **Code:** `crates/ehrbase/src/service/ehr.rs:279-286` (`parse_expected_version`) and `crates/ehrbase/src/service/api/ehr.rs:379-385` (`expected_from_if_match`) extract only the trailing integer; `vobject.rs` `next_version` skips the check entirely when `expected == None`.
- **Problem:** Two issues: (1) a malformed/unparseable `If-Match` yields `None`, and the update then proceeds with **no** precondition check (should be 412/400, never a silent success); (2) only the `version_tree_id` integer is compared — the UUID and system-id of the `OBJECT_VERSION_ID` are ignored, so `"wrong-uuid::wrong-sys::2"` passes as long as the current version is 2. This weakens the "mid-air collision" guarantee.
- **Fix:** Compare the full `OBJECT_VERSION_ID` against the current version's computed `object_version_id(vo_id, current)`. Treat an absent/unparseable `If-Match` on an update as a client error (the param is `required`; malformed → 400) rather than skipping the check. Deduplicate the two If-Match parsers into one shared function.
- [ ] fixed

### F-01-10: EHR resource omits the mandatory `ehr_access`
- **Severity:** minor
- **Spec:** RM `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.ehr.ehr.adoc` — `ehr_access: OBJECT_REF` is **1..1**. (`ITS-REST/.../schemas/ehr/Ehr.yaml` lists `ehr_access` as a property with no `required` array, so the OAS is looser than the RM.)
- **Code:** `crates/ehrbase/src/service/ehr.rs:82-96` (`ehr_summary`) emits `system_id`, `ehr_id`, `ehr_status`, `time_created` — no `ehr_access`.
- **Problem:** The RM (the ADR-008 oracle) makes `ehr_access` mandatory; the returned EHR omits it. In practice EHR_ACCESS is widely deprecated, so this is minor, but it is an RM-cardinality divergence worth a decision.
- **Fix:** Either emit a minimal `ehr_access` OBJECT_REF (referencing an EHR_ACCESS object) or record a `// PORT NOTE:` documenting the deliberate omission with the RM reference and the EHR_ACCESS-deprecation rationale.
- [ ] fixed

### F-01-11: XML offered for EHR/EHR_STATUS though the OAS declares JSON-only
- **Severity:** info
- **Spec:** `responses/200_EHR.yaml`, `201_EHR.yaml`, `200_EHR_STATUS_retrieved.yaml`, `200_EHR_STATUS_updated.yaml` and `operations/ehr_create*.yaml` request bodies declare `content: application/json` only (no `application/xml`).
- **Code:** `crates/ehrbase-rest/src/dispatch/ehr.rs:60,70,79,89,98,107,117` route EHR/EHR_STATUS through `respond_rm` (honours `Accept: application/xml`) and `optional_rm_value`/`rm_value` (accept XML request bodies).
- **Problem:** Not a conformance failure (the ITS-REST prose supports canonical XML generally, and offering a superset is acceptable), but the per-operation OAS contract for these resources is JSON-only, so this is drift worth noting — and it interacts with F-01-03 (XML on `ehr_status_get_by_version_id` currently 500s).
- **Fix:** Keep the XML capability, but confirm it is intended for these resources and that the drift-check (`utoipa` OAS vs vendored OAS) tolerates it; ensure every RM-typed value actually deserializes back into its declared `openehr-rm` type before advertising XML (see F-01-03).
- [ ] fixed

## Hygiene notes

- **Duplicated `If-Match` parsing:** `parse_expected_version` (`service/ehr.rs:279`) and `expected_from_if_match` (`service/api/ehr.rs:379`) are two near-identical copies. Collapse into one shared helper (and fix per F-01-09).
- **Overloaded `status_version`:** `status_version` (`service/ehr.rs:175`) is called by both `ehr_status_get_by_version_id` (needs bare EHR_STATUS) and `versioned_ehr_status_version_get_by_id` (needs a VERSION). This single method returning `original_version` is the root cause of F-01-03; split the two response shapes.
- **`uid` injection vs headers:** `with_uid` (`service/ehr.rs:214`) computes the `OBJECT_VERSION_ID` for the body but the same value is needed for the `ETag`/`Location` headers (F-01-01) — surface it once from the service rather than recomputing at two layers.
- **Redundant `EhrApi::` disambiguation:** `contribution_create`/`contribution_get` are called via fully-qualified `EhrApi::` (`dispatch/ehr.rs:341,349`) because the method names also exist on `DemographicApi`; fine, but a short comment already exists — no action, just noting the shared-name coupling.
