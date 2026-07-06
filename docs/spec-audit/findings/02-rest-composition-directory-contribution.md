# 02 — REST: COMPOSITION / DIRECTORY / CONTRIBUTION

## Summary

Audit of the COMPOSITION, DIRECTORY (FOLDER), CONTRIBUTION endpoint groups and
VERSIONED_COMPOSITION against ITS-REST 1.0.3
(`docs/specs/openehr/ITS-REST/specifications/`) and the CNF Platform Conformance
Test Schedule (`docs/specs/openehr/CNF/docs/platform_test_schedule/master07`
[composition], `master08` [contribution], `master09` [directory]). Code audited:
`crates/ehrbase-rest/{dispatch/ehr.rs, negotiate.rs, error.rs, params.rs}` and
`crates/ehrbase/src/service/{composition.rs, directory.rs, contribution.rs,
versioned.rs, vobject.rs, api/ehr.rs, mod.rs}`.

The versioning core (temporal `vo_version`, decompose→nodes, contribution+audit
in one tx, `ensure_ehr_exists`→404, validation→422) is sound. But the layer has
one **critical** defect and several **major** conformance gaps at the HTTP edge:

- **Deleted objects break every read path with a 500** — a logical delete writes
  zero `node` rows, and every read reassembles nodes unconditionally, so
  reassembly of an empty row set errors out *before* the `if read.deleted`
  guards are ever reached. The spec's "deleted → 204" becomes an
  Internal Server Error, and the guard branches are dead code (F-02-01).
- **No `Location`/`ETag` response headers anywhere** — every create/update/delete
  response the spec defines carries `Location` + `ETag`; the server emits neither,
  so clients cannot discover the new `version_uid` to use in `If-Match`
  (F-02-02).
- **`Prefer` is ignored** — the default is `return=minimal` (headers only, no
  body); the server always returns the full representation (F-02-03).
- **Two spec endpoints return 501** — `versioned_composition_version_get_at_time`
  and `versioned_ehr_status_version_get_at_time` are dispatched but never
  implemented (F-02-04).
- **`preceding_version_uid` on delete is ignored** — no 409-on-stale, no
  precondition enforcement (F-02-05).
- **`AUDIT_DETAILS.change_type` uses non-numeric `code_string`** and **deleted
  VERSIONs report `lifecycle_state = complete`** instead of `523|deleted|`
  (F-02-06, F-02-07).

## Findings

### F-02-01: Deleted versioned objects cause HTTP 500 on every read (deleted-handling is dead code)
- **Severity:** critical
- **Spec:** `ITS-REST/.../responses/204_because_deleted_at_time.yaml`,
  `204_because_deleted.yaml`, `composition_get.yaml` (`204` response),
  `directory_get_at_time.yaml` (`204` response);
  `CNF/.../master07-func_tc_ehr_composition.adoc` §"Delete COMPOSITION" (logical
  delete = a new deleted VERSION).
- **Code:** `service/vobject.rs:256-281` (`Change::Delete` writes **no** node
  rows), `vobject.rs:469-493` (`read_current` calls `read_nodes` unconditionally),
  `vobject.rs:554-584` (`read_nodes`→`reassemble`), `storage/codec.rs:204-206`
  (`reassemble(&[])` → `StorageError::InvalidRows("no rows")`);
  `service/composition.rs:47-59, 167-182`; `service/directory.rs:42-53`.
- **Problem:** A logical delete inserts a `vo_version` row with `deleted = true`
  and **zero** `node` rows. But `read_current` / `read_version` / `version_at`
  all call `read_nodes` → `reassemble` *before* the caller inspects
  `read.deleted`. `reassemble` on an empty slice returns
  `InvalidRows("no rows")`, which maps to `ServiceError::Storage` →
  `ApiError::Internal` → **HTTP 500**. Consequences:
  - `GET .../composition/{uid}` when the latest version is deleted → 500 (spec:
    `204`).
  - `GET .../composition/{uid}?version_at_time=…` resolving to a deleted version
    → 500 (spec: `204`).
  - `GET .../versioned_composition/{id}/version/{version_uid}` for the deleted
    version → 500 (spec: `200` with an ORIGINAL_VERSION whose data is the
    deleted marker).
  - `DELETE` of an already-deleted COMPOSITION → `ensure_composition_in_ehr` →
    `read_current` → 500 (spec: `400_already_deleted`).
  The `if read.deleted { return NotFound(...) }` branches in
  `composition.rs`/`directory.rs` are **unreachable** — `read_nodes` errors
  first. The existing test (`tests/service_ehr.rs:216`) only asserts
  `.is_err()`, so it passes on the 500 and masks the bug (it also never checks
  the status code — a weak test, do not weaken further; strengthen it).
- **Fix:** Make deleted-ness a first-class read outcome that never touches
  reassembly. In `read_current`/`read_version`/`version_at`, read the
  `deleted` flag from `vo_version` first and **skip `read_nodes` when
  `deleted`** (return `VersionRead { deleted: true, canonical: Value::Null, .. }`).
  Then have the service map a deleted read to the spec status per operation:
  `composition_get`/`directory_get_at_time` → **204 No Content**;
  `composition_delete` on an already-deleted object → **400** (`already_deleted`);
  `versioned_*_version_get_by_id` of a deleted version → **200** with an
  ORIGINAL_VERSION carrying the deleted lifecycle_state (see F-02-07) and no
  data. This requires a distinct service return (e.g. an enum
  `Read::Live(Value) | Read::Deleted`) so the dispatch can choose 204 vs 200 —
  a `ServiceError::NotFound` cannot express 204.
- [x] fixed

### F-02-02: No `Location` or `ETag` headers on any create/update/delete response
- **Severity:** major
- **Spec:** `ITS-REST/.../responses/201_COMPOSITION.yaml`,
  `200_COMPOSITION_updated.yaml`, `204_COMPOSITION_deleted.yaml`,
  `201_directory.yaml`, `200_directory_updated.yaml`, `201_CONTRIBUTION.yaml`,
  `409_COMPOSITION_with_uid_based_id.yaml`, `412_COMPOSITION.yaml` — all declare
  `headers: ETag + Location`. `headers/ETag_COMPOSITION.yaml` (the `version_uid`
  in double quotes), `headers/Location_COMPOSITION.yaml` (URL of the resource).
- **Code:** `ehrbase-rest/src/negotiate.rs` (`respond`, `respond_rm`, `empty`
  set only `Content-Type`); `dispatch/ehr.rs:176-243, 304-343` (no header
  wiring); `grep` for `etag`/`location` in `ehrbase-rest/src` returns nothing.
- **Problem:** The server never emits `Location` or `ETag`. `ETag` carries the
  new `version_uid`, which the client must echo in `If-Match` for the next
  update/delete (`parameters/header/If-Match.yaml`: "always a `version_uid`
  … enclosed by double quotes"). Without `ETag`, the update/delete precondition
  flow is undiscoverable; without `Location`, created-resource discovery fails.
  Also required on the 409/412 error responses (return the latest `version_uid`).
- **Fix:** Thread the committed `OBJECT_VERSION_ID` out of the service (it is
  already computed — `EhrbaseService::object_version_id`) and set on
  create/update/delete responses:
  `ETag: "<version_uid>"` and
  `Location: <base>/ehr/{ehr_id}/composition/<version_uid>` (resp. `directory`,
  `contribution`). The service methods currently return the reassembled body
  only; extend them to also return the `version_uid` (and for CONTRIBUTION the
  contribution uid), and add a header-setting helper in `negotiate.rs`. On the
  409/412 paths, include the *current* latest `version_uid`.
- [ ] fixed

### F-02-03: `Prefer` header ignored — body always returned (default must be `return=minimal`)
- **Severity:** major
- **Spec:** `parameters/header/Prefer.yaml` (`enum: [return=representation,
  return=minimal]`, **`default: return=minimal`**); `201_COMPOSITION.yaml`,
  `200_COMPOSITION_updated.yaml`, `201_directory.yaml`,
  `200_directory_updated.yaml`, `201_CONTRIBUTION.yaml`: "Content body is only
  returned when `Prefer` header has `return=representation`, otherwise only
  headers are returned."
- **Code:** `dispatch/ehr.rs:165-343` — `composition_create`/`_update`,
  `directory_create`/`_update`, `contribution_create` always call
  `respond_rm`/`respond` with the full body; `Prefer` is only mapped as a param
  name in `params.rs:81` and never read. `200_directory_updated.yaml` +
  `204_directory_updated.yaml` show the update should return **204** with no
  body under the (default) minimal preference.
- **Problem:** Default behaviour is inverted: with no `Prefer` (the common case)
  the server should return **201/200/204 with headers and no body**, but it
  returns the full representation. Only when `Prefer: return=representation` is
  present should the body be included.
- **Fix:** Parse `Prefer` in the create/update dispatch arms. When it is absent
  or `return=minimal`, return the status + `Location`/`ETag` headers with an
  empty body (directory_update → 204; composition/create → 201 no body;
  update → 200 no body per `200_*_updated`, or 204 for directory). When
  `return=representation`, return the body as today. Centralise as a
  `respond_prefer(...)` helper alongside F-02-02's header wiring.
- [ ] fixed

### F-02-04: `versioned_composition_version_get_at_time` and `versioned_ehr_status_version_get_at_time` return 501
- **Severity:** major
- **Spec:** `operations/versioned_composition_version_get_at_time.yaml`
  (`200_VERSION_of_COMPOSITION_at_time.yaml`; retrieves the VERSION extant at
  `version_at_time`, or the latest when omitted).
- **Code:** `dispatch/ehr.rs:263-273` dispatches to
  `backend().versioned_composition_version_get_at_time(p)`, but
  `service/api/ehr.rs` does **not** implement it (nor
  `versioned_ehr_status_version_get_at_time`) — neither param type is imported
  (`api/ehr.rs:11-23`), so both fall through to the generated trait default
  `Err(ApiError::NotImplemented)` (`openehr-its/.../rest/generated/ehr.rs:749-754`)
  → **HTTP 501**.
- **Problem:** Two in-scope VERSIONED_COMPOSITION / VERSIONED_EHR_STATUS
  endpoints are unimplemented. The building blocks exist —
  `vobject::version_at` + `EhrbaseService::original_version` — so this is a
  wiring gap, not a design gap.
- **Fix:** Implement both on `EhrbaseService`: parse `versioned_object_uid` +
  optional `version_at_time`; when a timestamp is given call
  `vobject::version_at`, else `read_current`; wrap the loaded `VersionRead` with
  `original_version` (honouring F-02-07 for a deleted version) and return it.
  Return `404_unknown_ehr_id_or_versioned_object_uid_or_no_version_at_time` when
  no version is extant at the requested time.
- [ ] fixed

### F-02-05: `composition_delete` ignores `preceding_version_uid` — no 409 on stale, no precondition
- **Severity:** major
- **Spec:** `operations/composition_delete.yaml`: "The `uid_based_id` MUST be in
  a form of an OBJECT_VERSION_ID … representing the `preceding_version_uid` to
  be deleted"; `409_COMPOSITION_with_uid_based_id.yaml` ("409 … when supplied
  `uid_based_id` doesn't match the latest version"); `400_already_deleted.yaml`.
  `CNF/.../master07` §"Delete COMPOSITION" uses the created version's
  `version_uid` as `preceding_version_uid`.
- **Code:** `service/api/ehr.rs:156-160` — `composition_delete` does
  `let (vo_id, _) = parse_object_id(&params.uid_based_id)` (discards the version
  tail) and calls `delete_composition(ehr_id, vo_id)`;
  `service/composition.rs:140-152` calls `vobject::delete(..., expected=None, ...)`
  — no version check at all.
- **Problem:** The mandatory `preceding_version_uid` is neither required nor
  checked. Deleting with a stale `version_uid` should be **409** (with the
  latest `version_uid` in `Location`/`ETag`); deleting an already-deleted
  composition should be **400** (`already_deleted`; currently 500, see F-02-01).
  As written a delete of any/no version tail silently succeeds against the
  latest.
- **Fix:** Parse `uid_based_id` as an OBJECT_VERSION_ID (reject a bare
  HIER_OBJECT_ID with 400), extract the expected version, and pass it as
  `expected` to `vobject::delete` (which already raises `VersionConflict` on
  mismatch). Map that mismatch to **409** for delete (not 412 — delete has no
  `If-Match`; the precondition is the path `uid_based_id`), and include the
  latest `version_uid` in the response headers. Return 400 for an
  already-deleted target.
- [x] fixed

### F-02-06: `AUDIT_DETAILS.change_type.defining_code.code_string` is a word, not the openEHR numeric code
- **Severity:** major
- **Spec:** openEHR Terminology group `audit change type`
  (`crates/openehr-term/assets/en/openehr_terminology.xml:31-42`): `249`
  creation, `251` modification, `523` deleted. `AUDIT_DETAILS.change_type` is a
  `DV_CODED_TEXT` whose `defining_code` is a `CODE_PHRASE` in terminology
  `openehr` — the `code_string` MUST be the numeric concept id.
  `CNF/.../master07:588` cites `openehr::523|deleted|`.
- **Code:** `service/vobject.rs:54-58` stores `change_type` as the literal
  strings `"creation"`/`"modification"`/`"deleted"`;
  `service/contribution.rs:207-236` (`audit_details`) sets both
  `value` and `defining_code.code_string` to that same stored string. So the
  emitted `CODE_PHRASE` is `{ terminology: "openehr", code_string: "creation" }`
  — `"creation"` is not a valid code in the group.
- **Problem:** Every `AUDIT_DETAILS` the server emits (in CONTRIBUTION,
  REVISION_HISTORY, ORIGINAL_VERSION) carries a non-conformant `defining_code`.
  A conformance check that the code is in the `openehr` audit-change-type group
  fails. The contribution path is *also* inconsistent: when a client submits a
  numeric `change_type` (`contribution.rs:coded_value` reads `code_string`
  "249"), it stores `"249"` and then emits `value: "249"` — the `value` should
  be the rubric, not the code.
- **Fix:** Store the canonical numeric code (`249`/`251`/`523`) in
  `audit.change_type`, and in `audit_details` emit
  `defining_code.code_string = <code>` with `value = <rubric>` (look the rubric
  up in `openehr-term`, or map the three known codes). Normalise inbound
  contribution `change_type` (accept either the code or the rubric) to the code
  before storing.
- [x] fixed

### F-02-07: Deleted VERSION reports `lifecycle_state = 532|complete|` instead of `523|deleted|`
- **Severity:** major
- **Spec:** `CNF/.../master07-func_tc_ehr_composition.adoc:579,588,602`: the
  deleted VERSION's `lifecycle_state` value is `openehr::523|deleted|`. Version
  lifecycle-state group in `openehr_terminology.xml:137` (`532` complete),
  and `523` deleted is the shared deleted code.
- **Code:** `service/versioned.rs:93-101` — `original_version` hardcodes
  `lifecycle_state = { value: "complete", code_string: "532" }` for **every**
  version, regardless of `read.deleted`.
- **Problem:** An ORIGINAL_VERSION produced for a deleted version misreports its
  lifecycle state as complete. Fails the CNF delete post-condition check.
- **Fix:** Make `original_version` set `lifecycle_state` from `read.deleted`:
  deleted → `{ value: "deleted", code_string: "523" }`, else
  `{ value: "complete", code_string: "532" }`. (Ties into F-02-01's need to
  carry `deleted` through the read without failing on empty nodes.)
- [x] fixed

### F-02-08: `If-Match` (required) is silently bypassed when unparseable
- **Severity:** minor
- **Spec:** `parameters/header/If-Match.yaml` (`required: true` for
  composition_update, directory_update, directory_delete); `412_COMPOSITION.yaml`
  / `412_directory.yaml` (mismatch → 412).
- **Code:** `service/api/ehr.rs:379-385` — `expected_from_if_match` returns
  `None` when the header cannot be parsed to a trailing integer; the service
  then treats `expected = None` as "no concurrency check"
  (`vobject::next_version` skips the check when `expected` is `None`).
- **Problem:** A malformed or missing `If-Match` (the header struct field is a
  required `String`, but a value like `"garbage"` parses to `None`) silently
  disables the optimistic-concurrency guard instead of failing. A wrong-but-
  well-formed `If-Match` correctly yields 412; a malformed one incorrectly
  succeeds.
- **Fix:** Distinguish "absent/unparseable" from "parsed version". Treat an
  unparseable required `If-Match` as **400**, and always enforce the parsed
  version as `expected` (never fall through to `None`). Applies to
  composition_update, directory_update, directory_delete.
- [ ] fixed

### F-02-09: CONTRIBUTION-supplied `uid` ignored; `audit.system_id` not validated
- **Severity:** minor
- **Spec:** `operations/contribution_create.yaml`: "`uid`: when provided, it will
  be accepted in case it is not in-use, otherwise error will be returned";
  "`audit.system_id`: when provided, it will be validated". `409.yaml`
  ("resource with same identifier(s) already exists").
- **Code:** `service/contribution.rs:43-109` — the incoming `body.uid` is never
  read; `commit_contribution` always mints a fresh `uuidv7` contribution id
  (`vobject.rs:420-427`). `parse_audit` (`contribution.rs:135-138`) accepts any
  `system_id` verbatim.
- **Problem:** A client-supplied contribution `uid` is dropped (should be honoured
  and, if already in use, → 409). `system_id` is stored without validation.
- **Fix:** If `body.uid.value` is present, use it as the contribution id and
  return **409** when a contribution with that id already exists in the EHR.
  Validate `audit.system_id` against the configured system id (or accept per a
  documented policy) rather than silently trusting it.
- [ ] fixed

### F-02-10: CONTRIBUTION change-type/operation mismatch returns 422, spec says 400
- **Severity:** minor
- **Spec:** `400_CONTRIBUTION.yaml`: 400 "when … the modification type doesn't
  match the operation - i.e. first version of a composition with MODIFICATION".
- **Code:** `service/contribution.rs:76-98` — a MODIFICATION/DELETE version
  without a resolvable `preceding_version_uid`, or a `_type` that is not a
  versioned root, raises `ServiceError::Unprocessable` → **422**
  (`mod.rs:109`).
- **Problem:** The spec classifies operation/type mismatches in a CONTRIBUTION as
  **400 Bad Request**, not 422. (422 is reserved for semantically-invalid but
  well-formed content.)
- **Fix:** Map CONTRIBUTION structural/operation-mismatch errors (missing
  `preceding_version_uid` for modify/delete, non-versioned `_type`, missing
  `versions`) to `ApiError::BadRequest` (400). Keep 422 for template/RM
  validation of the contained objects.
- [ ] fixed

### F-02-11: `composition_update` does not cross-check body `uid` against `uid_based_id`
- **Severity:** minor
- **Spec:** `operations/composition_update.yaml`: "If the request body already
  contains a COMPOSITION.uid.value, it must match the `uid_based_id` in the URL."
- **Code:** `service/api/ehr.rs:143-154` / `service/composition.rs:110-137` —
  the body's `uid` is never compared to the path `uid_based_id`.
- **Problem:** A COMPOSITION body whose `uid.value` names a different object is
  accepted; the spec requires a mismatch to be rejected (400).
- **Fix:** When the body carries `uid.value`, verify its object part equals
  `uid_based_id`; on mismatch return 400.
- [ ] fixed

### F-02-12: `directory_get_at_time`/`_by_version_id` `path` navigation matches folder *name*, not archetype path
- **Severity:** minor
- **Spec:** `operations/directory_get_at_time.yaml` / `directory_get_by_version_id.yaml`:
  "If `path` is supplied, retrieves … the sub-FOLDER that is associated with that
  path." `parameters/query/path.yaml` and the openEHR DIRECTORY conventions
  treat the folder path as the hierarchical folder route.
- **Code:** `service/directory.rs:143-155` — `select_subfolder` walks
  `folders[].name.value` segment-by-segment.
- **Problem:** Matching on `name.value` is a reasonable reading but is not clearly
  the spec's intended path semantics (which may be name-based per level — this is
  underspecified in the prose). `directory_get_by_version_id` does **not** apply
  `path` at all (`api/ehr.rs:230-237` calls `directory_version` which ignores the
  `path` query param), so the two get-directory operations are inconsistent.
- **Fix:** Confirm the intended folder-path semantics against
  `docs/specs/openehr/RM/docs/common/` (FOLDER) + the CNF directory Robot suite,
  document the chosen semantics with a `// PORT NOTE:`, and apply the same `path`
  navigation in `directory_get_by_version_id` as in `directory_get_at_time`.
- [ ] fixed

### F-02-13: Stale/misleading module doc comment in `service/api/ehr.rs`
- **Severity:** info
- **Spec:** n/a (documentation accuracy).
- **Code:** `service/api/ehr.rs:1-5` states "Methods not yet wired (revision
  history, time-travel reads, item tags, `ehr_get_by_subject`,
  `contribution_create`) inherit the generated `NotImplemented` default" — but
  all of those *are* implemented in the same file; the actually-unimplemented
  methods are the two `*_version_get_at_time` (F-02-04).
- **Problem:** The comment misstates what is wired, obscuring the real 501 gap.
- **Fix:** Rewrite the doc comment to list the genuinely-unimplemented operations
  (the two version-at-time reads) once F-02-04 is addressed, remove it.
- [ ] fixed

## Hygiene notes

- **Deleted-read design smell (root of F-02-01/F-02-07):** the service models a
  read result as `Value` and encodes "deleted" only via a post-hoc
  `if read.deleted` check that is unreachable because `read_nodes` runs first.
  The clean fix is a `Read::{Live(Value), Deleted}` (or `Option`-of-body plus a
  `deleted` flag) returned from `vobject`, with `read_nodes` skipped for deleted
  rows — this simultaneously fixes the 500, enables 204, and lets
  `original_version` set the deleted lifecycle_state. Worth doing as one
  refactor rather than three patches.

- **Response-header wiring is entirely missing** (F-02-02/F-02-03): there is no
  single place that knows an operation's result `version_uid`/`Location`. Adding
  a small `Committed`-carrying return from the service write methods (the
  `Committed` struct already exists in `vobject.rs` but is discarded by the
  service, and its `contribution_id` field is `#[allow(dead_code)]`) plus a
  `respond_created(status, headers, prefer, body)` helper in `negotiate.rs`
  would cover create/update/delete uniformly across COMPOSITION/DIRECTORY/
  CONTRIBUTION and the 409/412 error paths in one shot.

- **Duplication:** the `if read.deleted { NotFound }` block is copy-pasted across
  `composition.rs` (×3: `read_composition`, `composition_at_time`,
  `ensure_composition_in_ehr`) and `directory.rs` (×2). Fold into the
  `vobject` read layer so the deleted policy lives in one place.

- **`audit_details` builds the same string for `value` and `code_string`**
  (F-02-06). Once codes are numeric, add a tiny `(code, rubric)` map (or a
  `openehr-term` lookup) so the DV_CODED_TEXT is well-formed everywhere it is
  emitted (contribution get, revision history, original version) — currently
  each of those calls the one shared `audit_details`, which is good; fix it in
  that single function.

- **`parse_object_id` version extraction** (`api/ehr.rs:355-365`) uses
  `raw.rsplit("::").next()` for the version and `raw.split("::").next()` for the
  id, tolerating malformed 2-part ids silently. Given F-02-05 needs a strict
  OBJECT_VERSION_ID for delete, consider a single strict parser
  (`{uuid}::{system}::{int}`) reused by delete + `If-Match`, with explicit 400s.

- **Test masking:** `crates/ehrbase/tests/service_ehr.rs:216-223` asserts only
  `.is_err()` on a get-after-delete, which passes on the current 500. After
  F-02-01, strengthen it to assert the 204 status (do not weaken it). The delete
  call there also passes a bare `vo_id` as `uid_based_id`, which only "works"
  because of F-02-05.
