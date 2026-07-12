# ITS-REST Definition API — spec-compliance audit

Read-only audit (2026-07-12) of the openEHR **ITS-REST Definition API** (the
`/definition/template/*` and `/definition/query/*` wire surface) against its
realization in the tree. This is the **wire-side** companion to
`docs/design/sm-platform/04-definition.md` (the SM `I_DEFINITION_*` interface +
validity-depth audit): where the two overlap, this document cites 04-definition
and does not re-derive. Scope here is the HTTP contract — routes, media types,
query/path params, status codes, response headers, and `Prefer`/`Accept`
negotiation — as the ITS-REST development-edition specs define it.

**Spec oracle** (read these before any change):

- `docs/specs/openehr/ITS-REST/specifications/docs/definition/Description.md`
  (the API purpose + `STABLE` status)
- `docs/specs/openehr/ITS-REST/specifications/operations/` — the 13 definition
  operation YAMLs: `definition_template_adl1.4_{upload,get,list,example_get}.yaml`,
  `definition_template_adl2_{upload,get,list,example_get,version_get}.yaml`,
  `definition_query_{list,store.yaml,version_get,version_store.yaml}.yaml`
- `docs/specs/openehr/ITS-REST/specifications/parameters/` —
  `query/{query_type,at_version,filter_template_id,filter_version,concept,offset,fetch}.yaml`,
  `path/{qualified_query_name,template_id,template_id_adl2,version}.yaml`,
  `header/{Prefer,Accept_Template,Accept_Template_adl2,Accept_LOCATABLE,Accept_JSON,ContentType_text}.yaml`
- `docs/specs/openehr/ITS-REST/specifications/responses/` —
  `201_Template_adl1_4_upload.yaml`, `201_Template_adl2_upload.yaml`,
  `200_Template_adl1_4_retrieved.yaml`, `200_Template_adl2_retrieved.yaml`,
  `200_StoredQuery_stored.yaml`, `409_StoredQuery_version.yaml`, `406.yaml`,
  `404_unknown_template_id.yaml`
- `docs/specs/openehr/ITS-REST/specifications/schemas/others/TemplateIdentifier.yaml`,
  `.../schemas/query/QueryName.yaml`, `.../headers/{Location_Query,ETag_Template_adl1_4}.yaml`
- `docs/specs/openehr/ITS-REST/computable/OAS/definition-codegen.openapi.yaml`
  (the authoritative generated OAS the contract is emitted from — `info.version: latest`,
  i.e. the **development edition**, superset of Release-1.0.3)
- Adjacent: `docs/design/sm-platform/04-definition.md` (SM interface + validity
  depth — cross-referenced, not repeated); WORKLIST **W-4** owns the full
  spec-exact ADL2 parser/AOM2 pipeline referenced by the ADL2 gaps.

**Current implementation** (verified 2026-07-12):

- REST dispatch (all 13 routes): `app/ehrbase-rest/src/dispatch/definition.rs`
  (406 lines) — one match arm per operation id (dotted ids kept verbatim,
  e.g. `"definition_query_store.yaml"`).
- Generated contract (DTOs + `*Params` + `ROUTES`):
  `crates/openehr-its/src/rest/generated/definition.rs` (all five list/store
  param structs decode the query params; `ROUTES` at `:700-733`).
- Wire-shaped adapter trait (`DefinitionAdapter`):
  `app/ehrbase-sm/src/extensions/adapters.rs:64-120`; impl
  `app/ehrbase/src/service/api/definition.rs:37-124`.
- Stored-query CRUD: `app/ehrbase/src/service/stored_query.rs` (248 lines);
  query helpers (qualify/split/validate): `app/ehrbase/src/service/definition.rs:460-575,:722-765`.
- Response negotiation: `app/ehrbase-rest/src/overview/negotiate.rs`
  (`template_upload_response`, `wants_web_template`, `xml_body`, `wt_json_body`,
  `empty_with_location`); ADL2 text/upload helpers inline in `dispatch/definition.rs:296-337`.
- `ApiError` → HTTP status: `crates/openehr-its/src/rest/runtime.rs:59-71`.

**Overall verdict — SUBSTANTIALLY COMPLIANT on the wire contract, with a real
list-parameter gap and a cluster of minor negotiation/header omissions.** All
13 development-edition routes are wired to live handlers with the correct paths,
media types, and (for queries) status codes; the versioned-store 409, the
no-version upsert + `Location`, SEMVER-prefix resolution, `Prefer` handling on
both uploads, the two-part query-name scheme, and the `ApiError`→HTTP mapping
are all faithful. The honest gaps are: (G-1) **template `list` drops every
filter and pagination param** — the adapter signature has nowhere to put them;
(G-2) the `query_type` param is not even read; (G-3) the ADL1.4 upload
`return=identifier` body diverges from the JSON `TemplateIdentifier` shape;
(G-4/G-5) the OPT GET omits the mandated `ETag` and never returns `406` on an
unsupported `Accept`; (G-6/G-7) the two ADL2 `example`/`version` operations are
`501` (deprecated/optional, tied to W-4). No fabricated capability; the `501`
seams are exactly where the spec deprecates the op or where a cADL/AOM2 source
model the tree lacks would be required.

---

## 1. Gap register (what is not spec-true today)

Each row cites the governing spec artefact and the code evidence. G-1 is the one
behavioural gap with client-visible impact; G-2/G-3 are contract divergences;
G-4/G-5 are header/negotiation omissions; G-6/G-7 are documented spec-deprecation
/ optional-feature `501`s.

| # | Gap | Spec citation | Today (file:line) |
|---|-----|---------------|-------------------|
| G-1 | **Template `list` silently ignores all filter + pagination params (both ADL1.4 and ADL2).** `template_id` (wildcard pattern), `concept`, `version` (filter_version), `offset`, `fetch` are declared on both list operations and **decoded** by the generated params, but the dispatch calls the adapter with **no arguments** and the adapter trait method itself takes none — so every call returns the full, unfiltered, unpaginated template set. A client's `?template_id=vital*&offset=10&fetch=5` has no effect. | `operations/definition_template_adl1.4_list.yaml` + `..._adl2_list.yaml` (params `filter_template_id`, `concept`, `filter_version`, `offset`, `fetch`); `parameters/query/filter_template_id.yaml` ("supports wildcards `*`") | params decoded: `generated/definition.rs:389-398` (adl14), `:445-454` (adl2); dispatch drops them: `dispatch/definition.rs:67-73`, `:168-174` (`template_adl14_list()` / `template_adl2_list()` — no args); adapter signature has no params: `service/api/definition.rs:47`, `:91`; `ehrbase-sm/src/extensions/adapters.rs:68`, `:95` |
| G-2 | **`query_type` query param is not read on either store operation.** Both `definition_query_store` and `definition_query_version_store` declare `query_type` (default `AQL`, and `QUERY_DESCRIPTOR.formalism` permits "any other string value"). The dispatch reads only `qualified_query_name`/`version`; the store hardcodes `query_type = 'AQL'` and validates the body with the AQL parser, so `?query_type=<non-AQL>` is silently treated as AQL — a non-AQL body then fails as "invalid AQL" (400) rather than an honest unsupported-formalism reject, and the descriptor always reports `AQL`. (Wire face of 04-definition **G-6**; the net-new wire fact is that the param is never even inspected.) | `operations/definition_query_store.yaml`, `..._version_store.yaml` (`query_type`); `parameters/query/query_type.yaml`; `schemas/others` `QUERY_DESCRIPTOR.formalism` | param decoded `generated/definition.rs:525,:554`; dispatch ignores it `dispatch/definition.rs:230-255`, `:267-285`; store hardcodes AQL `service/stored_query.rs:46-50,:59,:77` |
| G-3 | **ADL1.4 upload `return=identifier` body is a `text/plain` scalar, not the JSON `TemplateIdentifier` object.** The `201` `application/json` content is `TemplateIdentifier` = `{"template_id": "..."}`. The ADL1.4 path returns the bare id string with `Content-Type: text/plain`. The ADL2 upload path returns the correct JSON `{"template_id": hrid}` — so the two upload endpoints are inconsistent and only ADL1.4 diverges. | `responses/201_Template_adl1_4_upload.yaml` (`application/json` → `TemplateIdentifier`); `schemas/others/TemplateIdentifier.yaml` | ADL1.4: `overview/negotiate.rs` `template_upload_response` (identifier branch → `text/plain` scalar); ADL2 (correct): `dispatch/definition.rs:327-333` |
| G-4 | **ADL1.4 template GET omits the mandated `ETag` response header.** `200_Template_adl1_4_retrieved` requires an `ETag` (`W/"<uid>"`) alongside `Content-Type`; the GET dispatch returns the XML or wt+json body with neither an `ETag` nor an explicit content-type header. Only the *upload* sets `ETag`. | `responses/200_Template_adl1_4_retrieved.yaml` (headers `ETag` + `Content-Type`); `headers/ETag_Template_adl1_4.yaml` | `dispatch/definition.rs:121-129` (`xml_body` / `web_template_response`, no `ETag`) |
| G-5 | **Template GET does not `406` on an unsupported `Accept`.** `adl1.4_get`/`adl2_get` both list a `406` response. The ADL1.4 GET serves wt+json when asked and **XML for everything else** (so an `Accept: application/json`, which the retrieved response does not offer, silently yields XML). The ADL2 GET serves `text/plain` unconditionally (never the JSON `OperationalTemplateV2` form nor a `406`). Only the `example_get` path negotiates `406` correctly. | `operations/definition_template_adl1.4_get.yaml` + `..._adl2_get.yaml` (`406`); `responses/200_Template_adl1_4_retrieved.yaml` (xml + wt+json only); `parameters/header/Accept_Template_adl2.yaml` | adl1.4 GET `dispatch/definition.rs:111-130`; adl2 GET `:200-209`; correct `406` for comparison `:152-158` |
| G-6 | **ADL2 `example`/`version` operations → `501`.** `definition_template_adl2_example_get` and `definition_template_adl2_version_get` return `NotImplemented`; both need a cADL/AOM2 source model (example generator / `OperationalTemplateV2` JSON projection) the tree lacks. `version_get` is marked `deprecated: true` in the spec and ADL2 is OPTIONAL for CNF. Same as 04-definition **G-13**; tied to WORKLIST **W-4** — reference, do not re-plan. | `operations/definition_template_adl2_example_get.yaml`, `..._version_get.yaml` (`deprecated: true`) | `dispatch/definition.rs:210-221` (`ApiError::NotImplemented`) |
| G-7 | **ADL2 upload `at_version` (`version`) query param ignored — but it is deprecated.** `parameters/query/at_version.yaml` is `deprecated: true`; the generated params carry `version` (`generated/definition.rs:461`) but the dispatch reads only `prefer`. Ignoring a deprecated param is acceptable; recorded as residue, not a defect. | `parameters/query/at_version.yaml` (`deprecated: true`) | `dispatch/definition.rs:176-198` (reads `p.prefer`, not `p.version`) |

**Cross-referenced (owned by 04-definition.md / WORKLIST W-4 — not re-derived
here):** ADL1.4 archetype validity is lexical (04 G-1); ADL2 validity is a
registration subset (04 G-2, W-4); non-AQL formalisms rejected (04 G-6, the SM
face of G-2 above); **`template_overlay` upload → 500** on the storage CHECK
constraint (04 **G-7** — a real defect, fix at the schema/validator, still open);
the ADL2 wire **409 vs SM "replace"** split (04 **G-12**). Full spec-exact ADL2
parsing (cADL2/ODIN, AOM2 master08, flattening, OPT2) is **WORKLIST W-4**.

---

## 2. What is faithful (evidence, not intent)

| Claim | Evidence |
|-------|----------|
| All 13 development-edition definition routes are wired to live handlers; paths + dotted operation ids match the OAS verbatim | `dispatch/definition.rs:66-289`; `ROUTES` `generated/definition.rs:700-733`; OAS operation ids `definition-codegen.openapi.yaml:103-486` |
| ADL2 templates served as `text/plain` source on upload + GET (dev-OAS `text/plain` `OperationalTemplateV2 \| string`) | upload `dispatch/definition.rs:176-199`; get `:200-209`; `adl2_text_response` `:296-305`; matches `responses/200_Template_adl2_retrieved.yaml` (`text/plain`) |
| Versioned stored-query store correctly **409**s on an existing `(name, version)`; no-version store upserts (200) — exactly the operations' declared response sets | insert-only `ON CONFLICT … DO NOTHING`, 0 rows → `Conflict` `service/stored_query.rs:75-92`; no-version upsert `:53-69`; `responses/409_StoredQuery_version.yaml` vs `definition_query_store.yaml` (200/400 only) |
| Query store emits `Location` (`/definition/query/{name}/{version}`) on success; version recovered via the list seam for the bodyless no-version store | `dispatch/definition.rs:245-254` (no-version), `:280-284` (versioned); `headers/Location_Query.yaml` |
| Store-time AQL validation → **400** (not 422) on a syntactically invalid / non-AQL body, per the store ops' `200/400` set | `service/stored_query.rs:46-50` (`parse_str` → `BadRequest`); mapped `runtime.rs:59` |
| SEMVER-prefix version resolution (`1` / `1.0` → highest matching) on stored-query GET | `service/stored_query.rs:100-142`, `:207-213`; `parameters/path/version.yaml` |
| Query name uses the ITS-REST two-part `[{namespace}::]{query-name}` split (the three-part form is an SM-only concern — 04 G-4, out of wire scope) | `service/stored_query.rs:198-202`; `schemas/query/QueryName.yaml` |
| `Prefer` honoured on both uploads (`representation` → source; `identifier` → id; missing/`minimal` → empty) with `Location` on every case | ADL1.4 `overview/negotiate.rs` `template_upload_response`; ADL2 `adl2_upload_response` `dispatch/definition.rs:311-337` |
| `adl1.4/{id}/example` negotiates the four `Accept_LOCATABLE` forms (json/xml/wt.flat/wt.structured) and returns a real `406` otherwise | `dispatch/definition.rs:131-167`, `:347-364` |
| wt+json served on `adl1.4/{id}` GET as a documented EHRbase-compatible extension via the single `WebTemplateService` seam | `dispatch/definition.rs:125-126,:385-405` |
| Query `list` honours `qualified_query_name` as a prefix pattern (wildcard on empty), unlike the template lists | `service/api/definition.rs:98-100`; `service/stored_query.rs:149-169`; `operations/definition_query_list.yaml` |
| `ApiError` → HTTP status mapping complete for the definition surface (400/404/406/409/501) | `runtime.rs:59-71` |

---

## 3. Target design (to close the gaps)

Ordered by client-visible value. All items are wire/adapter-local and
independent of WORKLIST W-4 (which subsumes G-6 and the ADL2 validity depth).

### 3.1 Template list filtering + pagination (G-1) — the one with real impact

Widen the two adapter methods to carry the params the wire already decodes:

- `template_adl14_list(&self, filter: TemplateListFilter, page: Page)` and the
  ADL2 twin, where `TemplateListFilter { template_id: Option<String>` (glob →
  the existing `compile_pattern`/`LIKE` machinery, `*` wildcard),
  `concept: Option<String>`, `version: Option<String> }` and `Page` is the
  existing `types::Page` (`item_offset`/`items_to_fetch`, already used by the
  SM list ops).
- Push the filter + `OFFSET/LIMIT` into the `template_store` / `adl2_artefact`
  queries (mirroring `stored_query` pagination at `service/definition.rs:526-541`).
- Dispatch: read `p.template_id/concept/version/offset/fetch` and pass them
  (`dispatch/definition.rs:67-73`, `:168-174`).
- Tests: wildcard match, `concept` filter, `offset`+`fetch` window, empty →
  all. Cite `parameters/query/filter_template_id.yaml`.

### 3.2 `query_type` on store (G-2)

Read `p.query_type` and thread it to `store_query`. Align with the 04-definition
G-6 decision (accept-and-store vs typed reject): the spec-aligned target is to
persist the declared formalism in `stored_query.query_type` (default `AQL`) and
run the AQL syntactic check only when the formalism is AQL; a non-AQL formalism
that the build cannot validate gets a distinct typed reject (not a blanket
"invalid AQL" 400). Whichever is chosen, the param must stop being silently
dropped, and the descriptor's `type` must reflect it.

### 3.3 ADL1.4 upload identifier body (G-3)

In `template_upload_response`, make the `return=identifier` branch emit
`application/json` `{"template_id": <id>}` (the `TemplateIdentifier` shape),
matching the ADL2 path at `dispatch/definition.rs:327-333`. Fold the two upload
responders toward one shared helper so they cannot drift again.

### 3.4 OPT GET `ETag` + `406` (G-4/G-5)

- Emit `ETag: W/"<template uid>"` on the ADL1.4 GET (the uid is on the parsed
  OPT; the upload already computes an ETag from the template id — reuse it).
- Add explicit `Accept` negotiation on both GETs: honour the enum in
  `Accept_Template` / `Accept_Template_adl2`, and return `406` for an
  `Accept` outside it, instead of defaulting to XML/text (`dispatch/definition.rs:111-130`,
  `:200-209`). For ADL2, an `application/json` Accept is the `OperationalTemplateV2`
  JSON form — keep the current `text/plain` default and return `406` (not a
  wrong body) until the cADL parser (W-4) can project the JSON.

### 3.5 ADL2 example/version + at_version (G-6/G-7)

No action independent of W-4. Keep the `501`s and the deprecated-`at_version`
drop as recorded PORT-NOTE residue (§4); W-4 lands the example generator and the
`OperationalTemplateV2` JSON projection that would make these executable.

---

## 4. Standing PORT NOTEs (the honest residue after the fixes)

Each remains spec-cited in place:

- **ADL2 `example`/`version` → 501** (G-6): needs a cADL/AOM2 source model the
  tree lacks; `version_get` is `deprecated: true` and ADL2 is OPTIONAL for CNF.
  Lands with WORKLIST W-4.
- **ADL2 upload `at_version` ignored** (G-7): the param is `deprecated: true`;
  dropping it is spec-permitted.
- **ADL2 GET `application/json` form** (part of G-5 residue): the JSON
  `OperationalTemplateV2` projection is deferred to W-4; served as a `406` (not
  a wrong body) until then.
- **Cross-owned, not this document's to close:** `template_overlay` upload →
  500 (04 G-7); ADL2 wire **409** vs SM/master04 "replace" (04 G-12); lexical
  ADL1.4 archetype validity (04 G-1); ADL2 validity subset (04 G-2 / W-4);
  non-AQL formalism rejection (04 G-6, the SM face of G-2).
- **ITS-REST edition:** the vendored OAS is `info.version: latest` (development
  edition) — it carries the ADL2 + `example` endpoints that are dev-edition
  additions over Release-1.0.3; the version-identity labelling is tracked by the
  CNF instrument (D1, `docs/blueprint/07-cnf.md`), not here.
