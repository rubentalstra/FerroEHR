# 03 — REST: QUERY / DEFINITION / ITEM_TAG / auth

## Summary

Audit of the QUERY, DEFINITION (templates + stored-query CRUD), ITEM_TAG REST
endpoint groups, plus auth (401/403) and the public status/health endpoints,
against the vendored openEHR ITS-REST 1.0.3 spec
(`docs/specs/openehr/ITS-REST/specifications/`), the RM ITEM_TAG class
(`docs/specs/openehr/RM/docs/common/` + `.../UML/classes/...item_tag.adoc`), and
the development-branch OAS actually driving the generated contract
(`crates/openehr-its/vendor/rest-oas/`).

**Auth is spec-correct** (401 with `WWW-Authenticate` for unauthenticated, 403
for the admin-scope gate — matches `auth.md` and the CNF security expectation).
**AQL execution is correctly deferred to P16** and honestly returns 501
(`impl QueryApi for EhrbaseService {}` inherits the generated `NotImplemented`
default), not a misleading 2xx or a 500 — acceptable interim behaviour.

The substantive divergences are all in the **implemented** surface: the
stored-query and template CRUD write paths use blind `ON CONFLICT DO UPDATE`
upserts and so **never produce the spec-mandated 409 conflicts**, **omit the
`Location` header**, and **return the wrong success status** (204 vs 200 for
stored-query store); the **ITEM_TAG PUT is an additive upsert rather than a
full-list replace** (empty-list-clears-all is broken); the **ITEM_TAG response
JSON does not match the RM/OAS `ITEM_TAG` shape** (`target`/`owner_id` are bare
strings, not `OBJECT_REF`, plus non-schema `id`/`target_type` fields under an
`additionalProperties: false` schema); and **semver-prefix / name-pattern
matching is unimplemented** for stored-query get and list.

Severity counts: **critical 0, major 9, minor 7, info 4.**

## Findings

### F-03-01: Stored-query store returns `204 No Content` with no `Location` header (spec: `200 OK` + `Location`)
- **Severity:** major
- **Spec:** `ITS-REST/specifications/operations/definition_query_store.yaml` +
  `responses/200_StoredQuery_stored.yaml` (`200 OK` … `headers: Location`) +
  `headers/Location_Query.yaml`; same for `definition_query_version_store.yaml`.
- **Code:** `app/ehrbase-rest/src/dispatch/definition.rs:156-178` (both store
  arms return `negotiate::empty(StatusCode::NO_CONTENT)`).
- **Problem:** The spec's only success response for storing a query is `200 OK`
  carrying a `Location` header pointing at the created resource
  (`…/definition/query/{name}/{version}`). The server returns `204 No Content`
  and never emits `Location`, so a client cannot discover the assigned version
  (critical because the no-version store auto-assigns one).
- **Fix:** Return `200 OK`; have `store_query` return the effective
  `{name, version}` and set `Location: {base}/definition/query/{name}/{version}`.
- [x] fixed (status + versioned Location) — both store arms now return `200 OK`
  (was `204`); the versioned store arm sets `Location:
  {base}/definition/query/{name}/{version}` (derived from the request params).
  The no-version arm's `Location` is left for the auto-increment redesign (see
  hygiene note): the generated `definition_query_store_yaml` trait method is
  bodyless (`()`), so the service-assigned version is not reachable at the
  dispatch edge to build the header (a `// TODO(port):` marks this).

### F-03-02: Versioned stored-query store never returns `409 Conflict` on an existing version
- **Severity:** major
- **Spec:** `operations/definition_query_version_store.yaml` →
  `responses/409_StoredQuery_version.yaml` ("`409 Conflict` … when a query with
  the given `qualified_query_name` and `version` already exists").
- **Code:** `app/ehrbase/src/service/stored_query.rs:20-33` (`INSERT … ON
  CONFLICT (reverse_domain_name, semantic_id, semver) DO UPDATE`); wired at
  `service/api/definition.rs:82-90`.
- **Problem:** Re-storing the same name+version silently overwrites the prior
  definition and resets `created_at`; the mandated `409` is never produced. A
  stored query at an explicit version is meant to be immutable.
- **Fix:** For the versioned store path, detect the existing row and return
  `ServiceError::Conflict` (→ 409); reserve upsert only for the no-version
  auto-versioning path (and there, auto-increment rather than reuse `1.0.0`).
- [x] fixed — the versioned store path is now insert-only (`ON CONFLICT … DO
  NOTHING`; 0 affected rows → `ServiceError::Conflict` → 409), never an
  overwrite; the no-version path keeps its spec-permitted upsert. Verified by
  `service_query.rs` (re-store same version → 409). (The `1.0.0` no-version
  auto-increment remains the deferred hygiene item.)

### F-03-03: Template upload never returns `409 Conflict` on a duplicate `template_id`
- **Severity:** major
- **Spec:** `operations/definition_template_adl1.4_upload.yaml` →
  `responses/409_template_already_exists.yaml` ("`409 Conflict` … when a
  template with same `template_id` … already exists").
- **Code:** `app/ehrbase/src/service/template.rs:73-87` (`INSERT INTO
  template_store … ON CONFLICT (template_id) DO UPDATE`).
- **Problem:** Uploading an OPT whose `template_id` already exists silently
  replaces the stored template instead of returning `409`. Templates are
  effectively immutable in openEHR; overwrite loses the original artifact.
- **Fix:** Check for an existing `template_id` first and return
  `ServiceError::Conflict` (→ 409) when present; do not upsert.
- [x] fixed — `store_template` is now insert-only (`ON CONFLICT (template_id) DO
  NOTHING`; 0 affected rows → `ServiceError::Conflict` → 409); the existing
  template is never overwritten. Shared fix with F-09-01; verified by
  `service_template.rs`.

### F-03-04: Template upload omits `Location`, ignores `Prefer`, and returns a metadata JSON body instead of the spec's representation
- **Severity:** major
- **Spec:** `responses/201_Template_adl1_4_upload.yaml` — "Server assigned
  `template_id` SHOULD be returned as part of the `Location` response header …
  Depending on the header `Prefer` either an empty body or a full representation
  body is returned … `content: application/xml: OperationalTemplate`";
  `parameters/header/Prefer.yaml` (default `return=minimal`).
- **Code:** `dispatch/definition.rs:56-69` (returns `respond(h, CREATED,
  &metadata)`); `service/api/definition.rs:23-34`.
- **Problem:** No `Location` header is set. `Prefer` is not consulted: the
  server always returns a JSON metadata descriptor, whereas the spec body is
  `application/xml` (the `OperationalTemplate`) and should be *empty* for the
  default `return=minimal`.
- **Fix:** Set `Location: {base}/definition/template/adl1.4/{template_id}`;
  honour `Prefer` (empty body for `return=minimal`, the OPT XML for
  `return=representation`).
- [ ] fixed

### F-03-05: ITEM_TAG `PUT` is an additive upsert, not a full-list replace; empty list does not clear tags
- **Severity:** major
- **Spec:** `vendor/rest-oas/ehr-codegen.openapi.yaml` `composition_tags_update`
  / `ehr_status_tags_update` (lines ~821, ~913): "Updates the list of **all**
  ITEM_TAG resources associated with a given target … **Providing an empty list
  will effectively remove all ITEM_TAG** associated with the given target."
- **Code:** `app/ehrbase/src/service/item_tag.rs:55-88` (`upsert_tags` only
  inserts/updates each posted tag; nothing deletes tags absent from the body).
- **Problem:** `PUT` semantics are "replace the whole tag set for the target".
  The implementation only merges in the posted tags — tags previously present
  but omitted from the body survive, and an empty-list `PUT` (the documented way
  to clear all tags) is a no-op. This is a wrong-semantics divergence.
- **Fix:** In one transaction, delete all existing tags for
  `(ehr_id, target_vo_id)` (or reconcile via `MERGE … WHEN NOT MATCHED BY
  SOURCE`, PG17) then insert the posted set; an empty body removes all.
- [x] fixed — `replace_tags` (renamed from `upsert_tags`) deletes the target's
  whole tag collection and inserts the posted set in one transaction; an empty
  list clears all. Verified by `service_ehr.rs`
  `item_tag_put_replaces_the_whole_collection`.

### F-03-06: ITEM_TAG response JSON does not match the RM/OAS `ITEM_TAG` shape
- **Severity:** major
- **Spec:** RM `.../UML/classes/org.openehr.rm.common.item_tag.adoc`
  (`target: UID_BASED_ID`, `owner_id: OBJECT_REF`; no `id` attribute); OAS
  `ItemTag` schema `ehr-codegen.openapi.yaml:3169-3187` (`target:
  ObjectRef`, `owner_id: ObjectRefOfHierObjectId`, `additionalProperties:
  false`, properties = `key,value,target_path,target,owner_id`).
- **Code:** `app/ehrbase/src/service/item_tag.rs:111-130` (`tag_json`).
- **Problem:** The emitted object uses `"target": "<uuid>"` and `"owner_id":
  "<uuid>"` as **bare strings**, whereas the schema requires `OBJECT_REF`
  objects (`{ "id": {"_type":"HIER_OBJECT_ID","value":…}, "namespace":…,
  "type":… }`). It also emits `"id"` and `"target_type"`, which are **not** in
  the schema — and the schema is `additionalProperties: false`, so a
  schema-validating client rejects the payload.
- **Fix:** Emit `target`/`owner_id` as `OBJECT_REF` (owner_id → EHR
  HIER_OBJECT_ID; target → the version_uid/versioned_object_uid with the right
  `type`); drop `id`/`target_type` (fold `target_type` into `target.type`).
- [x] fixed — `tag_json` now emits `target`/`owner_id` as OBJECT_REF objects
  (`target.type` = the stored RM kind; `owner_id` → the EHR) and drops the
  non-schema `id`/`target_type` fields. Verified by `service_ehr.rs`
  `item_tag_wire_shape_matches_the_oas_schema`.

### F-03-07: Stored-query GET does not honour semver prefix/partial version matching
- **Severity:** major
- **Spec:** `parameters/path/version.yaml` — the `version` "can be an exact
  version (e.g. `1.7.1`), or a pattern as partial prefix, in a form of
  `{major}` or `{major}.{minor}` … the highest (latest) version matching the
  prefix will be considered."
- **Code:** `app/ehrbase/src/service/stored_query.rs:44-50` (exact
  `semver = $3` match).
- **Problem:** `GET …/query/org.openehr::x/1` or `…/1.0` returns `404` instead
  of resolving to the latest `1.x`/`1.0.x`. Only exact triples resolve.
- **Fix:** When `version` is a `{major}` or `{major}.{minor}` prefix, match
  rows whose `semver` starts with that prefix (on a dot boundary) and pick the
  highest by numeric segment ordering (the query already orders by
  `string_to_array(semver,'.')::int[]`). Same rule applies to the deferred
  `query_execute_stored_query_version` path at P16.
- [x] fixed — `get_stored_query` matches a `{major}`/`{major}.{minor}` prefix
  on a dot boundary (`left(semver, length($3)+1) = $3 || '.'`) and picks the
  highest by numeric segment ordering; exact triples stay exact. Verified by
  `service_ehr.rs` `stored_query_semver_prefix_resolves_to_latest_match`. (The
  P16 stored-query *execution* path must reuse the same rule.)

### F-03-08: Stored-query list ignores the name pattern/prefix (exact-name only, no wildcard)
- **Severity:** major
- **Spec:** `operations/definition_query_list.yaml` — "Retrieves list of all
  stored queries … matched by `qualified_query_name` as **pattern** … when
  empty, treated as wildcard. `GET …/definition/query/org.openehr` will list
  all versions of all queries with names starting with `org.openehr`."
- **Code:** `app/ehrbase/src/service/stored_query.rs:67-84`
  (`list_stored_queries` splits on `::` and matches `reverse_domain_name` **and**
  `semantic_id` exactly).
- **Problem:** A prefix such as `org.openehr` (no `::`) is parsed as a bare
  semantic id with empty domain and matches nothing; there is no
  "starts-with" behaviour. Only a fully-qualified exact name returns rows.
- **Fix:** Treat `qualified_query_name` as a prefix pattern over the fully
  qualified name (`{rdn}::{semantic}`); empty ⇒ all. Match with a prefix
  predicate rather than equality on both columns.
- [x] fixed — `list_stored_queries` matches `qualified_query_name` as a prefix
  over the full qualified name (empty ⇒ wildcard) and each row now carries its
  own name (built from `reverse_domain_name`/`semantic_id`). Verified by
  `service_ehr.rs` `stored_query_list_matches_name_prefix`.

### F-03-09: Invalid template content is reported as `422`, not the spec's `400`
- **Severity:** major
- **Spec:** `operations/definition_template_adl1.4_upload.yaml` declares only
  `201 / 400 / 409`; `responses/400_invalid_template_content.yaml` = "`400 Bad
  Request` … because of invalid content." There is no `422` on this operation.
- **Code:** `app/ehrbase/src/service/template.rs:57-66` maps OPT parse
  failure + missing `template_id` to `ServiceError::Unprocessable` (→ 422 via
  `service/mod.rs:109`).
- **Problem:** A malformed OPT XML upload returns `422 Unprocessable Entity`
  where the spec (and any conformance client) expects `400 Bad Request`.
  (`422` is the composition-commit code, not the template-upload code.)
- **Fix:** Map malformed/incomplete OPT-upload errors to
  `ServiceError`→`ApiError::BadRequest` (400) on this endpoint.
- [ ] fixed

### F-03-10: ITEM_TAG `key`/`value` RM invariants not enforced
- **Severity:** minor
- **Spec:** RM `item_tag.adoc` — `Inv_key_valid`: key "may not be empty or
  contain leading or trailing whitespace" (`not key.is_empty and
  key.is_justified`); `Inv_value_valid`: "If set, may not be empty."
- **Code:** `app/ehrbase/src/service/item_tag.rs:64-70` (only checks `key`
  is present as a string; no emptiness/whitespace check on key, none on value).
- **Problem:** A tag with `key: ""`, `key: " x "`, or `value: ""` is accepted
  and stored, violating the RM invariants (should be `400`/`422`).
- **Fix:** Reject empty/whitespace-padded keys and empty-when-present values in
  `upsert_tags` before insert.
- [x] fixed (with F-03-05) — `replace_tags` enforces `Inv_key_valid` (non-empty,
  no leading/trailing whitespace) and `Inv_value_valid` (a set value may not be
  empty) → 422. Verified by `service_ehr.rs`
  `item_tag_put_replaces_the_whole_collection`.

### F-03-11: ITEM_TAG uniqueness keyed on `key` only, not `(key, target_path)`
- **Severity:** minor
- **Spec:** OAS `ehr-codegen.openapi.yaml` (composition/ehr_status tags GET/PUT
  descriptions) + `overview-html.openapi.yaml:281`: "More than one ITEM_TAG may
  be associated with a single target, … uniquely identified by their `key` and
  `target_path` **pair**."
- **Code:** `service/item_tag.rs:74` (`ON CONFLICT (ehr_id, target_vo_id,
  key)`); delete at `:97-104` matches on `key` only.
- **Problem:** Two tags on the same target with the same `key` but different
  `target_path` are legal and distinct per spec, but the DB unique key collapses
  them — the second overwrites the first. `DELETE …/tags/{key}` likewise cannot
  target one of several same-key tags.
- **Fix:** Make the tag identity `(ehr_id, target_vo_id, key, target_path)`
  (treat NULL target_path as a distinct slot); adjust upsert + delete.
- [ ] fixed

### F-03-12: Unknown target / unknown EHR not surfaced as `404` for tag reads/writes
- **Severity:** minor
- **Spec:** OAS `ehr_tags_get` → `404_unknown_ehr_id`; `composition_tags_get` /
  `_update` → `404_unknown_ehr_id_or_uid_based_id`.
- **Code:** `service/item_tag.rs:13-52` (`ehr_tags`, `target_tags` run no
  existence check); `upsert_tags:62` checks the EHR but **not** the target
  object; `service/api/ehr.rs:261-320`.
- **Problem:** `GET …/tags` for a non-existent `ehr_id` returns `200 []` instead
  of `404`; `GET`/`PUT` composition/ehr_status tags for an unknown
  `uid_based_id` do not return the mandated `404_unknown_ehr_id_or_uid_based_id`
  (tags on a non-existent target can even be created).
- **Fix:** Verify EHR existence in the read paths; verify the target VO exists
  (and belongs to the EHR) in target read/update paths, returning `404`
  otherwise.
- [ ] fixed

### F-03-13: Template GET performs no `Accept` negotiation — no `406`, no `400` invalid `template_id`
- **Severity:** minor
- **Spec:** `operations/definition_template_adl1.4_get.yaml` declares
  `200 / 400 (invalid template_id) / 404 / 406`; the 200 content is
  `application/xml` **or** `application/openehr.wt+json` per `Accept`.
- **Code:** `dispatch/definition.rs:70-83` serves `wt+json` only when explicitly
  requested, otherwise returns the OPT XML for **any** other `Accept` (including
  `application/json`); never returns `406`; never validates `template_id`.
- **Problem:** A client sending `Accept: application/json` (a format with no
  canonical template representation) gets XML rather than `406`. `400` for a
  syntactically invalid `template_id` is not produced. (Serving the original XML
  verbatim from `template_store.content` is otherwise correct — GET does return
  the exact uploaded artifact.)
- **Fix:** Negotiate `Accept` explicitly (xml → OPT, `openehr.wt+json` →
  WebTemplate, else `406`); optionally validate `template_id` shape → `400`.
- [ ] fixed

### F-03-14: `store_query` ignores the `query_type` parameter (hardcodes `AQL`, no `400` on unknown type)
- **Severity:** minor
- **Spec:** `operations/definition_query_store.yaml` `parameters:
  query_type` (`parameters/query/query_type.yaml`, default `AQL`);
  `responses/400_StoredQuery.yaml` — 400 "unknown query type".
- **Code:** `service/stored_query.rs:20-23` hardcodes `query_type = 'AQL'`;
  `service/api/definition.rs:72-80` never reads a `query_type` param (the
  generated `DefinitionQueryStoreYamlParams` may not even carry it).
- **Problem:** A `query_type` other than AQL is silently accepted and recorded
  as AQL rather than rejected with `400`.
- **Fix:** Read `query_type`, default AQL, reject unsupported values with `400`,
  and persist the declared type.
- [ ] fixed

### F-03-15: ITEM_TAG target collapses VERSION vs VERSIONED_OBJECT to a single uuid
- **Severity:** minor
- **Spec:** RM `item_tag.adoc` — `target: UID_BASED_ID`, "which may be a
  `VERSIONED_OBJECT<T>` or a `VERSION<T>`"; OAS descriptions distinguish a
  `version_uid` (OBJECT_VERSION_ID) target from a `versioned_object_uid`
  (HIER_OBJECT_ID) target.
- **Code:** `service/api/ehr.rs:279,289,300,306,316,327` all use
  `parse_object_id(...).0` (uuid head only), discarding the version tail;
  storage keys on `target_vo_id` uuid.
- **Problem:** A tag placed on a specific COMPOSITION *version* and one on the
  VERSIONED_COMPOSITION container are stored/queried identically — the
  version-vs-container distinction the spec draws is lost.
- **Fix:** Persist the target discriminator (and version, when a version_uid was
  supplied) so the two target kinds remain distinguishable and the emitted
  `target.type` is accurate.
- [ ] fixed

### F-03-16: ITEM_TAG `PUT` ignores `Prefer` (always `200` with body; never `204`)
- **Severity:** minor
- **Spec:** OAS `composition_tags_update` / `ehr_status_tags_update` declare
  both `200_…ItemTagList_updated` and `204_updated`, with a `Prefer` parameter
  (default `return=minimal`).
- **Code:** `dispatch/ehr.rs:368-397` always `respond(h, ok, &list)` (200 + body).
- **Problem:** Under the default `Prefer: return=minimal` the server should
  answer `204 No Content`; it always returns `200` with the full list. (This is
  the same `Prefer`-handling gap flagged elsewhere for the RM write paths, noted
  here for the tag scope.)
- **Fix:** Return `204` for `return=minimal`, `200` + list for
  `return=representation`.
- [ ] fixed

### F-03-17: AQL execution returns `501 Not Implemented` (interim; not a spec-listed status)
- **Severity:** info
- **Spec:** `query.openapi.yaml` execute ops list `200/400/404/408` only — no
  `501`.
- **Code:** `app/ehrbase/src/service/api/mod.rs:21` (`impl QueryApi for
  EhrbaseService {}`) → generated `NotImplemented` default
  (`openehr-its/.../generated/query.rs:168-210`).
- **Problem:** None for now — 501 is an honest "not built yet" (better than a
  wrong 2xx or a 500) and AQL execution is scheduled for P16. Flagged so it is
  not mistaken for conformant behaviour: a conformant server must eventually
  answer `200`/`400`/`404`/`408`.
- **Fix:** Implement the AQL engine at P16 with the spec's status set + the
  `RESULT_SET` body (see F-03-19).
- [ ] fixed

### F-03-18: No `408 Request Timeout` mapping in the error taxonomy for query execution
- **Severity:** info
- **Spec:** `responses/408_Query.yaml` — `408` on query-execution timeout.
- **Code:** `crates/openehr-its/src/rest/runtime.rs:24-73` (`ApiError` has no
  `Timeout`/`408` variant). Note the global `TimeoutLayer` in
  `router.rs:74-77` returns `408` for whole-request timeouts, but there is no
  *query-execution* timeout path.
- **Problem:** When AQL execution lands (P16) there is no typed way to signal
  the query-execution timeout `408` distinct from the middleware timeout.
- **Fix:** Add an `ApiError::Timeout` (→ 408) variant and use it for the AQL
  execution-time budget at P16.
- [ ] fixed

### F-03-19: `RESULT_SET` `ETag` header not emitted (deferred with execution)
- **Severity:** info
- **Spec:** `responses/200_Query.yaml` `headers: ETag`
  (`headers/ETag_RESULT_SET.yaml`) + the `RESULT_SET` schema
  (`schemas/query/ResultSet.yaml`: `meta/name/q/columns/rows`, `rows` required).
- **Code:** query dispatch (`dispatch/query.rs`) uses plain `respond(...)`; no
  `ETag`. Moot while execution is 501.
- **Problem:** Tracked so the P16 implementation emits the `ETag` and the exact
  `RESULT_SET` shape (`meta`, `columns[] {name,path}`, `rows[][]`).
- **Fix:** At P16, set the `ETag` on the 200 and build the `RESULT_SET` per the
  schema.
- [ ] fixed

### F-03-20: Implemented tag surface targets the development-branch OAS, not the stable 1.0.3 spec
- **Severity:** info
- **Spec:** `docs/VERSIONS.md` pins ITS-REST **1.0.3**; the stable 1.0.3 spec
  under `docs/specs/openehr/ITS-REST/specifications/` has **no** ITEM_TAG
  endpoints (grep finds none). The generated contract instead comes from the
  development-branch OAS in `crates/openehr-its/vendor/rest-oas/`
  (`ehr-codegen.openapi.yaml:77` `name: ITEM_TAG`, RM development `_item_tag_class`).
- **Code:** `crates/openehr-its/src/rest/generated/ehr.rs:967-996`;
  `dispatch/ehr.rs:352-403`.
- **Problem:** Not a bug, but a version-provenance mismatch worth recording: the
  ITEM_TAG behaviour is validated against a *development* spec surface while the
  project's stated REST target is 1.0.3. The RM ITEM_TAG class *is* in the
  pinned RM 1.2.0, so the model is in-scope; the REST binding is dev-branch.
- **Fix:** None required now; note the provenance in the phase/ADR record and
  re-audit ITEM_TAG when ITS-REST publishes it as stable.
- [ ] fixed

## Hygiene notes

- **Upstream OAS `operationId` typo propagated into method/route names.** The
  vendored operation files carry `operationId: definition_query_store.yaml` and
  `definition_query_version_store.yaml` (the `.yaml` is an upstream mistake).
  This flows through codegen into the route ids `"definition_query_store.yaml"`
  / `"definition_query_version_store.yaml"` and the trait methods
  `definition_query_store_yaml` / `definition_query_version_store_yaml`
  (`generated/definition.rs:721,731`; `dispatch/definition.rs:156,170`;
  `service/api/definition.rs:72,82`). Harmless (the dispatch matches the exact
  string) but reads as a bug. Consider an emitter override to strip a trailing
  `.yaml`/`.yml` from `operationId`s so the generated names are clean.
- **`stored_query` no-version store hardcodes `1.0.0`.** `store_query(..., None,
  ...)` always writes `semver = '1.0.0'` (`stored_query.rs:19`), so a
  no-version store of a *different* query text under the same name overwrites
  `1.0.0` rather than auto-incrementing (`1.0.1`, …). Combined with F-03-01/02,
  the versioning story needs a single coherent design (auto-increment on
  no-version store; immutable explicit versions).
- **`split_qualified` treats a name with no `::` as an empty reverse domain.**
  (`stored_query.rs:104-108`) Fine as a storage convenience, but interacts with
  F-03-08 (a bare `org.openehr` prefix is neither a domain nor a wildcard).
  Resolve alongside the list-pattern fix.
- **`tag_json` / `template_json` / `stored_query_json` swallow row-decode errors
  with `unwrap_or_default()`.** A decode failure yields silent empty strings /
  `Uuid::nil()` rather than an error — acceptable for infallible columns, but
  masks schema drift; consider surfacing decode failures as `Internal`.
- **Status endpoint `/rest/status` shape is an EHRbase-style extension** (not in
  ITS-REST); it correctly advertises `openehr_rest_api_version: "1.0.3"`
  (`status.rs:13,28`). No conformance issue; just note it is not spec-defined.
