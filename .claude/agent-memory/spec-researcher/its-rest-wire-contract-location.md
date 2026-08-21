---
name: its-rest-wire-contract-location
description: Where ITS-REST 1.1.0 wire-level conformance facts live (status codes, headers, media types, error body, RESULT_SET) — decomposed OAS, not prose tables
metadata:
  type: reference
---

# ITS-REST 1.1.0 wire-contract location map

Root: `docs/specs/openehr/ITS-REST/` (Release-1.1.0 @ 24058992d).

**Normative prose (Markdown, not adoc)** lives in
`specifications/docs/`:
- `overview/Requests_and_responses.md` — THE single normative HTTP-semantics
  doc: HTTP methods table, `Prefer` (minimal/identifier/representation +
  `Preference-Applied`, default=minimal), `If-Match`/`ETag` (W/ weak prefix
  MANDATORY in 1.1.0; 412 flow), `Location` (the prose scopes it to 201 and
  deprecates it on GET/DELETE, but the release itself also declares it on
  `200_StoredQuery_stored` — the #1565 instantiated contradiction),
  `openehr-version`/`openehr-audit-details`/`openehr-template-id`/
  `openehr-uri`/`openehr-item-tag` custom headers (lowercased in 1.1.0; old
  MixedCase deprecated), and **the ONE normative HTTP status-code table**
  (200/201/204/400/401/403/404/405/406/408/409/412/415/422/500/501). Error
  body example here = `{message, code, errors[DV_CODED_TEXT]}`.
- `overview/Resources.md` — content negotiation, canonical XML/JSON + Simplified
  Formats media types, datetime (extended ISO 8601), identifier forms.
- `query/{Request,Response,Query_types,Qualified_query_name}.md` — GET-vs-POST,
  ehr_id/offset/fetch/query_parameters, RESULT_SET, stored/adhoc/population.

## Path/operation census — quote these, never estimate (counted 2026-08-21)
`specifications/*.openapi.yaml` = EXACTLY SEVEN groups: overview, system, ehr,
demographic, query, definition, admin (and `specifications/docs/` has exactly
those seven sub-dirs). Simplified Formats + SMART are adoc sub-specs under
`ITS-REST/docs/`, with NO OAS. Path counts (`grep -c "^  '/"`):
**ehr 23 · demographic 27 · definition 9 (13 operations) · admin 2**.
`definition.openapi.yaml`'s nine are all under `/definition/template/{adl1.4,adl2}/**`
or `/definition/query/**` — no archetype path, and **no DELETE operation exists
anywhere in the whole vendored OAS for definitions** (`ls operations | grep delete`
= admin/party/composition/directory/tags only). The assembled
`computable/OAS/definition-validation.openapi.yaml` has the same nine.
**`overview/Resources.md` L3-L9 is the resource DEFINITION** ("a resource is an
instance object of a specific openEHR class") and its examples list
"definitions: TEMPLATE, **ARCHETYPE**, QUERY" — so the release NAMES archetype a
resource while publishing no archetype endpoint. Resources.md enumerates NO
endpoints, so it is NOT a citation for "path X does not exist" — cite the
`*.openapi.yaml` path list for that.

**Machine-readable contract (the real per-operation source of truth)** is a
DECOMPOSED OAS under `specifications/`:
- `operations/<operationId>.yaml` — per-op params + `responses:` status set.
- `responses/<code>_<name>.yaml` — per-response body+headers (e.g.
  `412_COMPOSITION.yaml` carries ETag; `409_EHR.yaml` vs `409_EHR_with_id.yaml`).
- `parameters/header/*.yaml` — media-type ENUMS: `Accept_LOCATABLE`/
  `ContentType_LOCATABLE` = json|xml|wt.flat+json|wt.structured+json;
  `Accept_Template` adds `application/openehr.wt+json`; `Accept_canonical` =
  json|xml only. `If-Match.yaml` required=true, `Prefer.yaml` enum+default.
- `schemas/query/ResultSet.yaml` (+ ResultSetColumn/Row/Metadata) — RESULT_SET
  shape: required `rows`; optional meta/name/q/columns; row = array of ANY.
- `schemas/others/Error.yaml` — the SCHEMA error body = `{message (req),
  validationErrors[string] (req)}` — DIVERGES from the overview prose example
  (`{message, code, errors[]}`). Flag this divergence.
- `computable/OAS/*.openapi.yaml` — assembled bundles (codegen/html/validation
  variants per API group).

**Key wire facts:** NO per-API status-code prose tables exist — per-API codes
are ONLY in each operation's `responses:` map. ETag = `W/"<version_uid>"`
(weak). If-Match required only when preceding_version_uid not in path; missing
when expected → SHOULD 400; mismatch → 412 + ETag. Simplified FLAT/STRUCTURED
are content-negotiated on the SAME EHR/composition routes (no separate
endpoints); path syntax rules in `docs/simplified_formats/master04` (see
[[flat-structured-format-location]]). Template upload dup → 409; template GET
XML vs `application/openehr.wt+json` web-template.

**Versioned-update ops (composition_update / ehr_status_update):** documented
status set for `operations/composition_update.yaml` = {200,204,400,404,412,422};
`operations/ehr_status_update.yaml` = {200,204,400,404,412} — **NO 422 branch
listed for ehr_status update** (OAS silence; overview general table still defines
422). **Neither update op lists 409** — 409 is NOT a documented branch for
versioned PUT updates (409 appears only on create/duplicate ops: 409_EHR,
409_*_with_uid_based_id, 409_template_already_exists). Branch conditions (verbatim
in `responses/*.yaml` descriptions): 400 = "could not be parsed or is invalid
(malformed URL, missing required header/param, syntactically invalid
header/param/content)"; 422 = "content type and syntax correct, could be converted
to a resource, but semantic validation errors, such as the underlying template is
not known or is not validating"; 412 = "If-Match doesn't match the latest version"
(+ ETag latest version_uid); 404 = unknown ehr_id or uid_based_id. Missing-If-Match-
when-expected → SHOULD 400 (overview §If-Match). If-Match `required:true` in
`parameters/header/If-Match.yaml`.

**ITEM_TAG surface IS in RELEASED 1.1.0** (STABLE EHR API): routes in
`ehr.openapi.yaml` L95-113 — `/ehr/{ehr_id}/tags` (get),
`.../composition/{uid_based_id}/tags` (get/put) + `/tags/{key}` (delete),
`.../ehr_status/{uid_based_id}/tags` (get/put) + `/tags/{key}` (delete); ops
`{composition,ehr_status,ehr}_tags_*.yaml`; also the `openehr-item-tag`/
`openehr-version-item-tag` request/response headers (overview
`Requests_and_responses.md` §openehr-item-tag). RM grounding =
`RM/docs/common/master07-tags.adoc` (Tags Package, ITEM_TAG class). Demographic
tags exist too but Demographic API is DEVELOPMENT-status.
