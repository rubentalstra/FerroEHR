# ITS-REST **EHR API** — spec audit + development-edition gap register

Read-only audit (2026-07-12) of the ITS-REST **EHR API** (development edition,
`STABLE`) against the implementation. The EHR surface passed the full ECC at
B6 (`docs/blueprint/00-THE-BLUEPRINT.md` §3, B6 close: 341 executed · 315
passed · 0 failed), so this is substantially compliant; the gaps below are the
**development-edition deltas** on top of the Release-1.0.3 contract the server
was built against. Each gap cites the governing spec text and the exact code
site.

## Spec oracle (read before any change)

- `docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
  — the cross-cutting protocol: HTTP methods, headers (`§Deprecated headers`,
  `§openehr-version and openehr-audit-details`, `§openehr-item-tag and
  openehr-version-item-tag`, `§openehr-template-id`, `§Location`, `§openehr-uri`,
  `§ETag and Last-Modified`, `§If-Match`), status codes, and
  `§Representation details negotiation` (`Prefer`).
- `docs/specs/openehr/ITS-REST/specifications/docs/overview/Resources.md`
  — resource identification (`versioned_object_uid` / `version_uid` /
  `uid_based_id`), `§Data representation` (canonical JSON/XML `415`/`406`
  discipline, Simplified-Format media types), `§Datetime format`.
- `docs/specs/openehr/ITS-REST/specifications/docs/ehr/Description.md`
  — the EHR-API purpose (thin; the normative detail lives in the operation
  YAMLs + OAS below).
- `docs/specs/openehr/ITS-REST/specifications/operations/{ehr,composition,directory,contribution,versioned_composition,versioned_ehr_status}_*.yaml`
  — the 33 EHR-group operations (per-operation parameters, request headers,
  response status codes).
- `docs/specs/openehr/ITS-REST/computable/OAS/ehr-codegen.openapi.yaml`
  — the machine-readable contract (paths, methods, response codes; the
  generation input for `openehr-its::rest::generated::ehr`).

The vendored prose + OAS are pinned at the **development edition** commit
`e8a093e9` (`app/ehrbase-rest/src/overview/mod.rs:6-9`).

## Verified current state (file:line)

- **Dispatch:** `app/ehrbase-rest/src/dispatch/ehr.rs` (930 lines) — one match
  arm per operation over the generated `*Params` structs
  (`openehr_its::rest::generated::ehr`, `ehr.rs:28-41`). All **33** EHR-group
  operations are routed, including the development-edition item-tag additions
  (`ehr_tags_get` `ehr.rs:787`, `composition_tags_{get,update,delete}`
  `ehr.rs:796-823`, `ehr_status_tags_{get,update,delete}` `ehr.rs:824-851`).
- **Routing / method + verb handling:** `app/ehrbase-rest/src/dispatch/mod.rs`
  — `mount()` groups methods per path into an axum `MethodRouter`
  (`mod.rs:117-174`); known-path/unknown-method → axum `405`, unrouted
  operation → `501` (`mod.rs:12-15`).
- **Negotiated responses + response headers:**
  `app/ehrbase-rest/src/overview/negotiate.rs` (1032 lines) —
  `set_resource_headers` (`:412-435`), `read_rm` (`:488-503`), `write_rm`
  (`:441-462`), `write_json` (`:466-483`), `deleted_with_headers` (`:507-517`),
  `error_with_meta` (`:522-533`), `prefers_representation` (`:371`),
  `prefers_resolve_refs` (`:385`), `template_upload_response` (`:540`).
- **Committal-header merge:** `app/ehrbase-rest/src/overview/committal.rs`
  (287 lines) — `merge_committal_headers` (`:62-77`), header constants
  (`:44-47`).
- **Identifier + If-Match parsing:**
  `app/ehrbase-rest/src/overview/version_id.rs` — `require_if_match`
  (`:107-119`).
- **`OPTIONS /` + `/status`:** `app/ehrbase-rest/src/overview/status.rs`
  (`system_options` `:60-74`, `OPENEHR_REST_API_VERSION` `:15`).
- **`Last-Modified` source:** populated from `VERSION.commit_audit.time_committed`
  in the service layer (`app/ehrbase/src/service/ehr.rs:217,614`) and emitted
  when present (`negotiate.rs:424-427`) — the B6 plumbing is live.

What is **correct and complete** (not re-litigated below): operation presence
(33/33); status-code sets per operation (201/400/409 create, 200/204/404 get,
200/204/400/404/412 update, 204/400/404/409|412 delete, 422 on composition
create/update); canonical JSON **and** XML negotiation with the `415`/`406`
discipline (`Resources.md §Data representation`); FLAT/STRUCTURED Simplified
media types on composition (`ehr.rs:377-383,423-428`); `If-Match` required-but-
missing → `400` and false → `412` with the latest `version_uid` echoed
(`version_id.rs:107-119`, `ehr.rs:296-311`); the committal-header **merge**
semantics (server defaults merged, absent headers left intact,
`committal.rs:62-77`); `Prefer: return=representation` re-read
(`ehr.rs:280-284`); `Prefer: resolve_refs` on `contribution_get`
(`ehr.rs:778-783`); the deleted-version → `204` mapping
(`ehr.rs:410-412,621-623`); the `OPTIONS /` conformance endpoint
(`status.rs:60-74`); body-uid cross-check on composition update
(`ehr.rs:455-469`).

---

## Gap register

Every gap is a development-edition delta over the Release-1.0.3 contract the
server implements. Severity is the spec's own modal verb.

| # | Gap | Spec citation | Today (file:line) | Severity |
|---|-----|---------------|-------------------|----------|
| G-1 | **Resource-identifier `ETag` lacks the weak `W/` indicator.** The dev edition made `W/` mandatory: "all `ETag` headers that hold a resource identifier MUST include a weakness indicator `W/` prefix"; "the bare quoted form is deprecated". The impl emits `ETag: "{uid}"` (no `W/`) for every EHR/COMPOSITION/EHR_STATUS/FOLDER/CONTRIBUTION resource; only the *template* upload uses `W/` (`negotiate.rs:571`). | `Requests_and_responses.md §ETag and Last-Modified` (+ `§Deprecated headers`) | `negotiate.rs:418` — `format!("\"{}\"", meta.uid)` | **MUST** |
| G-2 | **`Location` emitted on `GET` reads and `DELETE` — now forbidden/deprecated.** "It MUST NOT be used to indicate an alternate representation of an existing resource (e.g., via `GET` method)"; "The `Location` header MUST ONLY be used for resource creation … or redirect responses"; and "the `Location` response header was deprecated from responses of `DELETE` methods". The impl's single `set_resource_headers` **always** sets `Location`, and it is called from the 200-GET reads (`composition_get`, `ehr_status_get_*`, `versioned_*_version_get_at_time`) via `read_rm`, and from the 204 delete via `deleted_with_headers`. | `Requests_and_responses.md §Location` | `set_resource_headers` `negotiate.rs:421-422` (unconditional); called by `read_rm` `:500` and `deleted_with_headers` `:514`; read arms `ehr.rs:241-247,257-263,431-437,348-354,586-592`; delete `ehr.rs:518-522` | **MUST NOT** (GET); deprecated (DELETE) |
| G-3 | **Committal headers accepted only in the deprecated Release-1.0.3 spelling/shape.** The dev edition restructured them: the header **name** is `openehr-version` / `openehr-audit-details` with the attribute path carried **in the value** (worked example: `openehr-version: lifecycle_state.code_string="532"`, `openehr-audit-details: change_type.code_string="251"`). The impl parses header **names** `openEHR-VERSION.lifecycle_state` / `openEHR-AUDIT_DETAILS.<attr>` (attribute in the name, deprecated uppercase). HTTP name lookup is case-insensitive but these are structurally different names, so a dev-edition client's headers never match — a `MUST accept` failure. | `Requests_and_responses.md §Deprecated headers` (table) + `§openehr-version and openehr-audit-details` | `committal.rs:44-47` (`H_LIFECYCLE = "openEHR-VERSION.lifecycle_state"`, etc.); `merge_committal_headers` `:62-77` | **MUST** |
| G-4 | **`openehr-item-tag` / `openehr-version-item-tag` request-header wrappers on writes are ignored.** The dev edition: on `PUT`/`POST` these headers "instruct the system which ITEM_TAG list should be associated with the target". `composition_create`/`composition_update`/`ehr_status_update`/`directory_create`/`directory_update` all declare both headers as parameters, yet the write arms never read them to set tags (tags are reachable only through the dedicated `*_tags_*` operations). | `Requests_and_responses.md §openehr-item-tag and openehr-version-item-tag §Usage in Requests`; `operations/composition_create.yaml:6-7,16-17` | write arms `ehr.rs:372-387,439-501,265-312,677-702,626-676` — no `openehr-item-tag` read | **MUST** (server that supports ITEM_TAGs) |
| G-5 | **`system_id` committal attribute not parsed.** "clients MAY supply values for the AUDIT_DETAILS attributes `change_type`, `description`, `committer` and `system_id`." The merge handles the first three but never reads `system_id`. (The companion MUST — "when `system_id` is not provided … the server MUST set it to its own configured system identifier" — is met by the server default, so this is only the client-override half.) | `Requests_and_responses.md §openehr-version and openehr-audit-details` | `committal.rs:62-77` (no `system_id` branch) | client-override: MAY; server-default: MUST (met) |
| G-6 | **`Prefer: return=identifier` not honoured on EHR-group writes.** The dev edition adds this preference: a `201`/`200` with a body containing only the identifier (`{ "uid": "…" }`), "never `204 No Content`". `write_rm` handles only `minimal` (204/empty) vs `representation`; `return=identifier` falls through to minimal → wrong status + empty body. Only `template_upload_response` (`negotiate.rs:555-564`) implements it. | `Requests_and_responses.md §Prefer minimal, identifier or full representation response` + `§Prefer only identifier` | `write_rm` `negotiate.rs:441-462` (only `prefers_representation`); `prefers_*` `:371,385` (no `prefers_identifier`) | **SHOULD** (client MAY request) |
| G-7 | **`If-Match` does not tolerate a `W/` weak prefix.** Once G-1 is fixed the server's own weak `ETag` (`W/"…"`) will be echoed by clients into `If-Match`; `require_if_match` trims only surrounding `"`, leaving `W/"…"` which fails `ObjectVersionId::from_str`. The spec's `If-Match` example is unprefixed, but round-tripping our own weak ETag must not 400. | `Requests_and_responses.md §If-Match` (+ `§ETag`) | `version_id.rs:108` — `if_match.trim().trim_matches('"')` | interop / robustness |
| G-8 | **Item-tag response headers never emitted.** "Servers MAY include the `openehr-item-tag` … header in responses"; "When retrieving resources via `GET`, the server MAY also add these headers". Not set on any read/write response. | `Requests_and_responses.md §…openehr-version-item-tag §Usage in Responses` | not present in `negotiate.rs` / `ehr.rs` | MAY |
| G-9 | **`openehr-uri` response header never emitted.** MAY-level: a service that generates `DV_EHR_URI`-format URIs may add `openehr-uri` to applicable responses. | `Requests_and_responses.md §openehr-uri` | not present | MAY |
| G-10 | **Reported REST-API version is `1.0.3`, not the development contract actually served.** `OPTIONS /` (`restapi_specs_version`) and `/status` (`openehr_rest_api_version`) report `"1.0.3"` while the vendored prose + OAS are pinned at development `e8a093e9`. This is a self-description/identity mismatch (blueprint ch 7 D1). | `mod.rs:6-9` (contract identity) vs reported value | `status.rs:15` (`OPENEHR_REST_API_VERSION = "1.0.3"`), used `:65,32` | identity |

Doc/code mismatch worth flagging: `overview/mod.rs:11-31` **documents** all of
G-1/G-2/G-3/G-4 as "implemented … new-form-first with the deprecated forms
still accepted/emitted", but the code (`negotiate.rs:418`, `committal.rs:44-47`,
the unconditional `Location`) implements only the deprecated forms. The module
prose is aspirational, not descriptive — scrub or realize it.

---

## Target design

The fix is a focused header-policy pass in `app/ehrbase-rest/src/overview/`;
no route, service, or storage change is required. All operations already exist
and pass ECC, so this is compliance-hardening, not new capability.

### 2.1 `negotiate.rs` — ETag + Location policy (G-1, G-2)

- **Weak ETag** (G-1): `set_resource_headers` emits `W/"{uid}"` for the
  resource-identifier ETag (matching the template path's `W/` at
  `negotiate.rs:571`). A single `weak_etag(uid) -> HeaderValue` helper used by
  both. Keep accepting the bare form inbound (the spec permits it) — this is
  an emission change only.
- **Location split** (G-2): separate the two responsibilities that
  `set_resource_headers` currently fuses. Introduce
  `set_read_headers` (ETag + `Last-Modified`, **no** `Location`) for the 200
  GET reads (`read_rm`) and the 204 delete (`deleted_with_headers`), and keep
  `Location` **only** on the `201`/create + representation-write path
  (`write_rm` / `write_json` when the status is `201`, and the create arms).
  Update the per-arm doc comments in `ehr.rs` that assert "ETag + Location" on
  GET (e.g. `ehr.rs:207,240,429,577`) to "ETag + Last-Modified, no Location".

  *Note (spec-silent boundary):* whether a `200`-status representation write
  (`return=representation` on update) carries `Location` is not explicit; the
  spec restricts `Location` to "resource creation … or redirect", so update
  responses drop it too — documented as our reading of §Location.

### 2.2 `committal.rs` — dev-edition header shape + `system_id` (G-3, G-5)

- Parse the **new** header names `openehr-version` and `openehr-audit-details`
  where the attribute path lives in the value
  (`lifecycle_state.code_string="532"`, `change_type.code_string="251"`,
  `description.value="…"`, `committer.name="…",committer.external_ref.id="…"`,
  `system_id="…"`). Because `openehr-audit-details` legitimately repeats (one
  per attribute), read **all** values of the header (`HeaderMap::get_all`) and
  merge each. The tolerant `parse_attr_pairs` (`committal.rs:117`) is reused
  on the value tail after the leading `<attr>.` selector is split off.
- Keep the deprecated `openEHR-VERSION.<attr>` / `openEHR-AUDIT_DETAILS.<attr>`
  names as a fallback (the spec keeps them "available for backward
  compatibility") — new-form-first, deprecated-form-accepted, which is exactly
  what `mod.rs:11-31` already claims.
- Add the `system_id` branch (G-5): merge a client-supplied
  `AUDIT_DETAILS.system_id` into the commit envelope; leave the server default
  when absent (the existing MUST).
- YAML/example fixture: assert the `Requests_and_responses.md` worked example
  (`§openehr-version and openehr-audit-details`) parses verbatim, both forms.

### 2.3 `ehr.rs` — item-tag write wrappers (G-4)

On `composition_create`/`composition_update`/`ehr_status_update`/
`directory_create`/`directory_update`, after the successful commit, if the
request carried `openehr-item-tag` (VERSIONED_OBJECT target) or
`openehr-version-item-tag` (the committed VERSION target), replace that
target's ITEM_TAG list via the existing `target_tags_replace` seam
(`ehr.rs:811,839`) — an empty header value clears all tags
(`Requests_and_responses.md §Usage in Requests`). A shared
`apply_item_tag_headers(backend, ehr_id, target, version_uid, headers)` helper
keeps the five arms uniform. Servers that do not support ITEM_TAGs ignore the
headers (spec: "these headers will also be unsupported").

### 2.4 `version_id.rs` — If-Match weak-prefix tolerance (G-7)

`require_if_match` strips an optional leading `W/` before trimming quotes, so a
client echoing our weak `ETag` round-trips. One-line change + a test asserting
`W/"vo::sys::2"` and `"vo::sys::2"` both parse to the same OVID.

### 2.5 Optional (MAY) + identity (G-8, G-9, G-10)

- G-8/G-9 (MAY): emit `openehr-item-tag`/`openehr-version-item-tag` on GET/
  write responses reflecting the stored tags, and `openehr-uri` on applicable
  reads — low value, deferrable; record as MAY-scoped if not taken.
- G-10: derive the reported version from the vendored contract provenance
  (the blueprint's B5/D1 pattern — `SpecVersions.its_rest` from provenance, not
  a literal) so `OPTIONS /` and `/status` report `development` (or the resolved
  release label), not a hard-coded `1.0.3`.

### 2.6 Verification

- Unit: weak-ETag emission; no-`Location`-on-GET; `Location`-on-201 retained;
  both committal header forms + `system_id`; item-tag write wrapper (set +
  empty-clears); `If-Match` `W/` tolerance; `Prefer: return=identifier` body
  shape.
- ECC: the EHR-group cases must stay zero-drift; add cases asserting the
  development-edition header deltas (weak ETag, absent `Location` on GET,
  lowercase committal headers, item-tag write wrapper) so the deltas are
  evidenced, not just prose.
- Gates: workspace suites green, clippy clean, full ECC zero-drift
  (blueprint §4 rule 4).

---

## Standing PORT NOTEs (the honest residue after the pass)

- **`Prefer: return=identifier` on writes** (G-6) is a `SHOULD`; if deferred,
  record a `// PORT NOTE:` on `write_rm` citing
  `Requests_and_responses.md §Prefer only identifier` rather than silently
  treating it as `minimal`.
- **Committal `key="value"` grammar is example-only** — the spec gives no ABNF
  (`committal.rs:23-29` already records this); the dev-edition value-embedded
  `<attr>.` selector inherits the same "grammar by example" caveat.
- **`committer.external_ref.id` subtype** is assumed `HIER_OBJECT_ID` (the
  example is a bare UUID; `committal.rs:31-34`) — kept.
- **Item-tag response headers + `openehr-uri`** (G-8/G-9) are `MAY`; a
  deliberate non-emission is spec-conformant and noted as such.
- **EHR_STATUS `is_modifiable = False` write guard** and incomplete-lifecycle
  relaxation are RM-change-control items tracked elsewhere
  (`docs/blueprint/00-THE-BLUEPRINT.md` §2.3 ch 1 item 2), not EHR-API-wire
  gaps — out of scope here.
