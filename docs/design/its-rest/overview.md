# ITS-REST Overview (development edition) — spec audit + protocol design

Read-only audit of the ITS-REST **Overview** specification (development
edition) against the implementation in `app/ehrbase-rest/src/overview/` and its
call sites. The Overview is the cross-cutting protocol every openEHR REST
resource API inherits: HTTP methods, headers, status codes, content
negotiation, resource identification, `Prefer`/`If-Match` discipline, and the
error body. Our stack was originally built to **Release-1.0.3**; the vendored
prose is the **development edition** (commit `e8a093e9`, the same ref as the
vendored OAS), whose `§Deprecated headers` changed the protocol. Every such
delta is a gap row below.

**Spec oracle** (read before any change):

- `docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
  — HTTP methods, authn, headers (`§Deprecated headers`, `§openehr-version and
  openehr-audit-details`, `§openehr-item-tag…`, `§Location`, `§openehr-uri`,
  `§ETag and Last-Modified`, `§If-Match…`), status codes, `§Representation
  details negotiation` (`Prefer` minimal/identifier/representation,
  `Preference-Applied`, `resolve_refs`).
- `docs/specs/openehr/ITS-REST/specifications/docs/overview/Resources.md`
  — resource identification (`versioned_object_uid`/`version_uid`/`uid_based_id`),
  data representation (canonical JSON/XML MUSTs, `415`/`406`, Simplified-Format
  media types), `§Datetime format`.
- `docs/specs/openehr/ITS-REST/specifications/docs/overview/Glossary_and_conventions.md`
  — the identifier vocabulary.
- Shared OAS pieces: `…/specifications/{headers,parameters,responses}/` —
  `headers/ETag*.yaml`, `headers/Location_*.yaml`,
  `headers/openehr-item-tag.yaml`, `headers/openehr-version-item-tag.yaml`,
  `parameters/header/`, `responses/2xx|4xx.yaml`.

**Current implementation** (verified 2026-07-12):

- Content negotiation + response assembly: `app/ehrbase-rest/src/overview/negotiate.rs`
  (1033 lines incl. tests).
- Committal metadata headers: `app/ehrbase-rest/src/overview/committal.rs` (287).
- Status-code / error body: `app/ehrbase-rest/src/overview/error.rs` (185).
- Resource-id + `If-Match` decode: `app/ehrbase-rest/src/overview/version_id.rs` (120).
- Param assembly: `app/ehrbase-rest/src/overview/params.rs` (376).
- `/status`, health, `OPTIONS /`: `app/ehrbase-rest/src/overview/status.rs` (87).
- Module map + delta note: `app/ehrbase-rest/src/overview/mod.rs` (85).
- Call sites: `app/ehrbase-rest/src/dispatch/ehr.rs` (committal merge :167;
  `require_if_match` :274/:475/:635/:709; `resolve_refs` :778) and
  `dispatch/demographic.rs`.
- `ApiError` status map: `crates/openehr-its/src/rest/runtime.rs:24-73`.

---

## 1. Verified current state (what is already spec-true)

Compliant findings are findings — the layer gets a great deal right.

- **HTTP status-code table** — `ApiError`
  (`crates/openehr-its/src/rest/runtime.rs:54-73`) covers 400/401/403/404/409/
  412/415/406/422/500/501 and maps each to the code the table in
  `Requests_and_responses.md §HTTP status codes` prescribes. `422` splits into
  a generic `Unprocessable` and a structured `ValidationFailed`
  (`error.rs:107-122`).
- **Canonical JSON/XML negotiation MUSTs** — a non-JSON/XML `Content-Type` on a
  JSON-only op → `415` (`negotiate.rs:258-270`, `require_json`); an
  un-serviceable XML `Accept` on a JSON-only payload → `406`
  (`negotiate.rs:288-295`, `respond`); `Content-Type` is set on every
  non-empty response (`negotiate.rs:588-602`, `json_response`; `:329-336` XML).
  Matches `Resources.md §XML Format` / `§JSON Format`.
- **Simplified-Format media types** — the current
  `application/openehr.wt{,.flat,.structured}+json` spellings are the ones
  wired (`negotiate.rs:46-50`); the deprecated `*.schema+json` names are
  correctly *not* emitted (`Resources.md §Simplified Formats`).
- **`If-Match` discipline (partial, correct where present)** — when the header
  is required and the client omits or empties it → `400 Bad Request`
  (`version_id.rs:107-119`, `require_if_match`), matching the development
  edition's "expected-but-missing → `400`" rule
  (`Requests_and_responses.md §If-Match…`). A false condition → `412` with the
  latest `version_uid` in `ETag` is wired through `error_with_meta`
  (`negotiate.rs:522-533`).
- **`Last-Modified`** — derived from the version commit time and emitted on
  versioned responses as an RFC 7231 IMF-fixdate (`negotiate.rs:166-172`,
  `http_date`; `:424-428`), exactly `VERSION.commit_audit.time_committed.value`
  per `§ETag and Last-Modified`. (Test `set_resource_headers_emits_last_modified`,
  `negotiate.rs:1010`.)
- **`Prefer: resolve_refs`** — detected (`negotiate.rs:385-393`) and acted on
  for the COMPOSITION-list read (`dispatch/ehr.rs:778`), the SHOULD in
  `§Prefer resolving Object references`.
- **`OPTIONS /`** — a `200` with an `Allow` header + an `Options` conformance
  body, mounted above the CORS layer so it is not swallowed as a preflight
  (`status.rs:60-74`).
- **Datetime path/query values** — parsed strictly as `OBJECT_VERSION_ID`/UUID
  at the edge (`version_id.rs`); temporal query values flow through untouched.
  `Resources.md §Datetime format`'s "preserve as sent" requirement for *body*
  values is a node-codec concern (out of this layer's scope).

---

## 2. Gap register (what is not spec-true today)

Every row cites the governing spec text and the code evidence. G-1..G-3 are
MUST-level protocol breaks introduced by the development edition; the module
doc (`overview/mod.rs:11-31`) *claims* these deltas are implemented, but the
code still targets Release-1.0.3 — so the doc is currently aspirational, not
descriptive.

| # | Gap | Spec citation | Today (file:line) |
|---|-----|---------------|-------------------|
| **G-1** | **`ETag` omits the mandatory weak `W/` indicator.** The development edition makes it a **MUST**: "all `ETag` headers that hold a resource identifier MUST include a weakness indicator `W/`". `set_resource_headers` emits the bare quoted form `"{uid}"` for every COMPOSITION / EHR_STATUS / EHR / FOLDER / directory read, write, delete, and `409`/`412` error. Only the template-upload path uses `W/`. | `Requests_and_responses.md §Deprecated headers` (lines 63-65) + `§ETag and Last-Modified` (line 172); `headers/ETag*.yaml` | Bare form at `negotiate.rs:418` (`format!("\"{}\"", meta.uid)`); `W/` used only at `negotiate.rs:571` (`template_upload_response`). MUST violation across the whole EHR surface. |
| **G-2** | **Committal headers parse the deprecated Release-1.0.3 name structure, not the development-edition value structure.** The dev edition moved the attribute path *into the header value*: header name `openehr-version` with value `lifecycle_state.code_string="532"`; `openehr-audit-details` with values `change_type.code_string=…`, `description.value=…`, `committer.name=…`, `system_id=…`. Our code keys off header **names** `openEHR-VERSION.lifecycle_state`, `openEHR-AUDIT_DETAILS.change_type`, etc. Because HTTP header names are case-insensitive, these match the deprecated `openehr-version.lifecycle_state` spelling but are a **different name** from `openehr-version`. A development-edition client is silently ignored on every committal attribute. This is a **MUST** ("services MUST accept `openehr-version` and `openehr-audit-details` … whatever is provided it MUST be merged"). | `Requests_and_responses.md §openehr-version and openehr-audit-details` (lines 72-96, worked example lines 85-91) | `committal.rs:44-47` constants `openEHR-VERSION.lifecycle_state` / `openEHR-AUDIT_DETAILS.*`; value parser `parse_attr_pairs` (`:117`) reads only the trailing `key="value"` list, never the `attr.key="value"` value prefix the new form carries. |
| **G-3** | **Client-supplied `system_id` is never merged; the server default is never explicitly asserted here.** The spec: clients MAY supply `system_id` via `openehr-audit-details`, and "when `system_id` is not provided by the client, the server MUST set it to its own configured system identifier". `committal.rs` reads no `system_id`, and `UpdateAudit` (`app/ehrbase-sm/src/common/version_update.rs:34-43`) has no `system_id` field to carry it into the commit envelope. | `Requests_and_responses.md §openehr-version and openehr-audit-details` (lines 81, 94) | `committal.rs:62-77` merges only lifecycle/change_type/description/committer; no `system_id` path. (The server-default assertion may exist downstream in versioning — verify it there and cite it.) |
| **G-4** | **`Location` is emitted on `GET` reads and `DELETE` responses — both deprecated.** The dev edition: "`Location` … MUST NOT be used to indicate an alternate representation of an existing resource (e.g. via `GET`)" and it was "deprecated from responses of `DELETE` methods". `read_rm` and `deleted_with_headers` both call `set_resource_headers`, which unconditionally sets `Location`. | `Requests_and_responses.md §Location` (lines 132-140, 58-61) | `negotiate.rs:488-503` (`read_rm`) and `:507-517` (`deleted_with_headers`) → `set_resource_headers` `:421-423` always inserts `Location`. |
| **G-5** | **`return=identifier` is not honoured on RM writes** — it silently degrades to `return=minimal` (empty body). The dev edition steers clients toward an eventual `identifier` default and defines it: status `200`/`201`, body = a single `{ "uid": … }` object. `write_rm` only branches minimal-vs-representation; only the template-upload path implements `identifier`. | `Requests_and_responses.md §Prefer minimal, identifier or full representation response` + `§Prefer only identifier` (lines 290-322); `§Deprecated headers` (lines 67-68) | `negotiate.rs:453-457` (`write_rm`) tests only `prefers_representation`; no `identifier` branch. `identifier` handled only at `:556-564` (`template_upload_response`). |
| **G-6** | **`Preference-Applied` response header is never emitted.** The spec says the service MAY include it to confirm the honoured preference; the module doc implies the `Prefer` deltas are handled. It is a MAY, so not a conformance break, but its absence means a client cannot detect which preference was applied (relevant once the default shifts to `identifier`). | `Requests_and_responses.md §Representation details negotiation` (line 278); example lines 147, 315 | No occurrence anywhere in `ehrbase-rest/src` (grep: none). |
| **G-7** | **`openehr-item-tag` / `openehr-version-item-tag` request headers are not merged on write, nor emitted on read.** The dev edition defines them as convenient wrappers over the ITEM_TAG operations: on `PUT`/`POST` they set the tag list for the target VERSION/VERSIONED_OBJECT (empty value ⇒ remove all); servers MAY echo them on responses. `params.rs` canonicalises the names for the generated params struct, but no dispatch path reads or acts on them. | `Requests_and_responses.md §openehr-item-tag and openehr-version-item-tag` (lines 98-126); `headers/openehr-item-tag.yaml`, `headers/openehr-version-item-tag.yaml` | No consumer in `dispatch/ehr.rs` (grep: none); the header name is only canonicalised generically in `params.rs:77-85`. |
| **G-8** | **The reported REST-spec version is the literal `"1.0.3"`, not the development ref the layer targets.** `overview/mod.rs` states the oracle is development@`e8a093e9`; `status.rs` reports `"1.0.3"` on both `/status` and the `OPTIONS /` conformance body. The two should agree with the vendored provenance (blueprint D1 resolved the identity to development@`e8a093e`). | `overview/mod.rs:6-9` vs the vendored provenance; `Resources.md`/`Specifications.md` version identity | `status.rs:15` `OPENEHR_REST_API_VERSION = "1.0.3"`, used at `:33` and `:65`. |
| **G-9** | **`openehr-uri` response header not emitted.** MAY-level: "If the service supports generating resource URIs in the `DV_EHR_URI` format, it MAY include the `openehr-uri` response header". Not a break; recorded so the MAY surface is complete. | `Requests_and_responses.md §openehr-uri` (lines 150-159) | No emission (grep: none). |
| **G-10** | **`405`/`501` method-status rules are not asserted at the router.** The spec: an unrecognised/unimplemented method SHOULD → `501`; a known-but-not-allowed method → `405`. `ApiError` can represent `501` (`NotImplemented`), but nothing maps axum's method-not-allowed / unknown-method routing outcomes onto `405`/`501` at the overview layer (axum's default is a bare `405` with no openEHR body, and no `501` path). | `Requests_and_responses.md §HTTP Methods` (lines 24-25) | No `405`/`501` mapping in `error.rs`; `ApiError::NotImplemented` reached only from the SM error map (`error.rs:79`). |

---

## 3. Target design

The fix is localised to `overview/` plus the committal envelope type; no
generated code changes. Do it new-form-first, keeping the deprecated forms
accepted where the spec says implementations MAY.

### 3.1 `negotiate.rs` — headers

1. **Weak `ETag` (G-1).** Change `set_resource_headers` to emit
   `W/"{uid}"` for every resource-identifier `ETag` (reads, writes, deletes,
   `409`/`412`). A single `resource_etag(uid) -> HeaderValue` helper feeds both
   `set_resource_headers` and the template path (which already uses `W/`). Keep
   accepting a bare inbound `ETag`/`If-Match` (the spec's "MAY still support"),
   but always *emit* the weak form. Update the tests that assert
   `Some("\"v::s::1\"")` to the weak form.
2. **`Location` only on create/redirect (G-4).** Split
   `set_resource_headers` into `set_versioning_headers` (ETag + Last-Modified,
   used by `read_rm`/`deleted_with_headers`/`error_with_meta`) and a
   create-only `set_location` (used by `write_rm` on `201`/create). Reads and
   deletes stop emitting `Location`; the `412`/`409` error path keeps `ETag`
   only (the spec asks for `ETag` there, not `Location`).
3. **`return=identifier` (G-5).** Add `prefers_identifier(headers)` and an
   `identifier` branch to `write_rm`: status `200`/`201` (never `204`), body a
   single `{ "uid": "<version_uid>" }` JSON object (or the XML equivalent when
   XML is negotiated), from `resp.meta.uid`. `minimal` stays the current
   default; steer nothing until the spec flips the default.
4. **`Preference-Applied` (G-6).** When a `Prefer` was received and honoured,
   set `Preference-Applied: return=<minimal|identifier|representation>` on the
   response (in `write_rm`/`template_upload_response`). Cheap, and it future-
   proofs the identifier-default transition.
5. **`openehr-uri` (G-9).** Optional: emit `openehr-uri:
   ehr:/{ehr_id}/compositions/{uid}` on single-resource reads when the resource
   maps cleanly to a `DV_EHR_URI`. MAY — gate behind config or defer with a
   PORT NOTE.

### 3.2 `committal.rs` — the development-edition header form (G-2, G-3)

Reparse against the new structure while still accepting the old:

- **Header names:** read `openehr-version` and `openehr-audit-details` (the
  new form) *and* the deprecated `openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*`
  names (backward compat, spec MAY). A header may appear multiple times
  (`audit-details` does in the example) — merge across all occurrences with
  `get_all`.
- **Value grammar:** the value is a comma-separated list of
  `attr_path.key="value"` pairs, where `attr_path` is `lifecycle_state`,
  `change_type`, `description`, `committer`, `system_id`. Extend
  `parse_attr_pairs` to split the leading `attr.` prefix from the key so
  `lifecycle_state.code_string="532"` maps to (target `lifecycle_state`, key
  `code_string`). The old form (where `attr` was in the header name and the
  value was a bare `key="value"`) collapses into the same internal
  `(target, key, value)` triples.
- **`system_id` (G-3):** add `system_id: Option<String>` to `UpdateAudit`
  (`app/ehrbase-sm/src/common/version_update.rs`) and merge the header value
  into it; the versioning layer sets its own configured `system_id` when the
  field is `None` (assert and cite that server-default line —
  `Requests_and_responses.md` line 94 — at the versioning seam, not here).
- Keep the two existing PORT NOTEs (example-only grammar; committer typing),
  re-pointed at the new value shape.

### 3.3 `error.rs` / router — method status (G-10)

- Map axum's `MethodNotAllowedLayer` / fallback so a known-route-wrong-method
  yields `405` with the openEHR `{ error, message }` body, and an unrecognised
  method yields `501`. A `method_not_allowed`/`not_implemented` fallback on the
  mounted router, rendering through `RestError`, is enough; no new `ApiError`
  variant is needed beyond the existing `NotImplemented` (add a
  `MethodNotAllowed` variant → `405`).

### 3.4 `status.rs` — version identity (G-8)

- Derive `OPENEHR_REST_API_VERSION` from the vendored ITS-REST provenance the
  same way the blueprint's D1 reconciliation does (development@`e8a093e`), or at
  minimum make `/status` and `OPTIONS /` agree with `overview/mod.rs`. If the
  conformance target is still labelled `1.0.3` for the STANDARD profile, say so
  explicitly and cite the provenance file.

### 3.5 `dispatch/ehr.rs` — item-tag headers (G-7)

- On COMPOSITION/EHR_STATUS/FOLDER `PUT`/`POST`, parse `openehr-item-tag` /
  `openehr-version-item-tag` (semicolon-separated `key/value/target_path`
  triples per lines 106-114) and forward them to the existing ITEM_TAG service
  operations for the target VERSIONED_OBJECT / VERSION (empty value ⇒ delete
  all). On reads, optionally echo the stored tags back in the same headers
  (MAY). Reuse the tag model already behind the dedicated ITEM_TAG endpoints —
  these headers are only a wrapper.

### 3.6 Verification

- **Unit:** weak-`ETag` emission on every helper; `Location` present on create,
  absent on read/delete; `identifier` body shape (JSON + XML); dev-edition
  committal parse (`openehr-version: lifecycle_state.code_string="532"`,
  multi-line `openehr-audit-details`, `system_id`), plus the deprecated-form
  regression; `405`/`501` bodies; item-tag header round-trip.
- **ECC / conformance:** re-run the full suite; the committal-header and ETag
  changes touch the versioning + composition areas — hold zero-drift, expect
  newly-green cases only. The dev-edition header forms should be added to the
  runner's request fixtures so the new parse path is actually exercised.

---

## 4. Standing PORT NOTEs (the honest residue after the redesign)

- **Committal value grammar is example-only.** `Requests_and_responses.md`
  gives no ABNF for the `attr.key="value"` list, its quoting, or escaping; we
  keep the tolerant comma-separated parser (quoted values opaque) and ignore a
  header that yields no usable attribute (spec: "merge whatever is provided").
- **`committer.external_ref.id` subtype.** The spec example is a UUID with no
  `OBJECT_ID` subtype; we wrap it as `HIER_OBJECT_ID` and drop the committer to
  the server default if the assembled `PARTY_IDENTIFIED` fails to type.
- **`openehr-uri` (G-9) is a MAY** — emit only if `DV_EHR_URI` generation is
  enabled; otherwise a documented no-op.
- **`Preference-Applied` (G-6) is a MAY** — emitted as a courtesy, not required.
- **Server-default `system_id` assertion** lives at the versioning seam, not in
  the header layer; the overview layer only carries a client-supplied value
  through. Cite `Requests_and_responses.md` line 94 at that seam.
- **Body-value datetime preservation** (`Resources.md §Datetime format`) is a
  node-codec/storage responsibility, audited in the RM/storage chapters, not
  here.
