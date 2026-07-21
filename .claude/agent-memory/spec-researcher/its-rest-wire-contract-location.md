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
  MANDATORY in 1.1.0; 412 flow), `Location` (201 only; GET/DELETE use
  deprecated), `openehr-version`/`openehr-audit-details`/`openehr-template-id`/
  `openehr-uri`/`openehr-item-tag` custom headers (lowercased in 1.1.0; old
  MixedCase deprecated), and **the ONE normative HTTP status-code table**
  (200/201/204/400/401/403/404/405/406/408/409/412/415/422/500/501). Error
  body example here = `{message, code, errors[DV_CODED_TEXT]}`.
- `overview/Resources.md` — content negotiation, canonical XML/JSON + Simplified
  Formats media types, datetime (extended ISO 8601), identifier forms.
- `query/{Request,Response,Query_types,Qualified_query_name}.md` — GET-vs-POST,
  ehr_id/offset/fetch/query_parameters, RESULT_SET, stored/adhoc/population.

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
