# A1 Spec Audit — Chapter `its-rest-general` — Requirements (Phase 1: Extract)

- **Chapter:** its-rest-general (ITS-REST 1.0.3 shared/general protocol layer)
- **Date:** 2026-07-11
- **Spec files read** (all under `docs/specs/openehr/ITS-REST/specifications/`):
  - `overview.openapi.yaml` (root API; `OPTIONS /` mounted here)
  - `docs/overview/Requests_and_responses.md` (HTTP methods, auth, headers, status codes, content negotiation) — the primary normative prose
  - `docs/overview/Resources.md` (resource identification, data representation, XML/JSON/alt formats, datetime)
  - `docs/overview/Glossary_and_conventions.md` (identifier lexical forms)
  - `operations/options.yaml` + `responses/200_options.yaml` + `schemas/others/Options.yaml`
  - `parameters/header/{If-Match,Prefer,Accept_template}.yaml`
  - `headers/{ETag,ETag_VERSION,Allow,Location_COMPOSITION,...}.yaml`
  - shared `responses/{400,406,409,412_COMPOSITION,...}.yaml`

**Correction note:** the task named `ITS-REST/specifications/headers/`, `parameters/`, `responses/` — all exist as directories. The prose "docs" live under `docs/overview/*.md` (not a flat `docs/`); read there. No listed file was missing.

**Chapter-note reminder (B6 tail):** committal-header merge, `Last-Modified`, `If-Match` hardening, and `OPTIONS /` must land on **every** change-controlled/versioned resource, not just COMPOSITION — the requirements below are phrased resource-generally for exactly that reason.

---

| ID | Requirement | Citation | Category | Risk |
|----|-------------|----------|----------|------|
| its-rest-general-R1 | Services MUST accept the `openEHR-VERSION` and `openEHR-AUDIT_DETAILS` custom request headers on commit operations (they carry committal metadata; none are individually mandatory). | Requests_and_responses.md §"openEHR-VERSION and openEHR-AUDIT_DETAILS" L56 | header | high |
| its-rest-general-R2 | Whatever `openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*` values are provided MUST be merged with the default VERSION and VERSION.audit_details attributes at commit runtime (not dropped, not replacing wholesale). | Requests_and_responses.md L67 | behaviour | high |
| its-rest-general-R3 | For all change-controlled resources (COMPOSITION, EHR_STATUS, FOLDER, etc.) `PUT`, `POST` and `DELETE` directly on the resource MUST be allowed, and MUST internally be executed using the native CONTRIBUTION-wrapped VERSION mechanism. | Requests_and_responses.md L49-54 | behaviour | high |
| its-rest-general-R4 | `openEHR-VERSION.lifecycle_state` accepts terminology `code_string` values 532 (complete), 553 (incomplete), 523 (deleted); implementations must map these codes to the correct lifecycle state on commit. | Requests_and_responses.md L73-75 | mandatory-attr | medium |
| its-rest-general-R5 | `openEHR-AUDIT_DETAILS.change_type` accepts `code_string` values 249 creation, 250 amendment, 251 modification, 252 synthesis, 523 deleted, 666 attestation, 253 unknown; these must map to the correct AUDIT_DETAILS.change_type coded value. | Requests_and_responses.md L76-82 | mandatory-attr | medium |
| its-rest-general-R6 | When a service receives `If-Match` and the condition evaluates to `false`, it MUST NOT perform the requested method and MUST respond `412 Precondition Failed`. | Requests_and_responses.md §"If-Match" L88-89; responses/412_COMPOSITION.yaml | rejection-duty | high |
| its-rest-general-R7 | On a `412` from an `If-Match` mismatch the service SHOULD return the latest `version_uid` in both the `Location` and `ETag` response headers. | Requests_and_responses.md L89; responses/412_COMPOSITION.yaml | header | medium |
| its-rest-general-R8 | `If-Match` value is always a `version_uid` enclosed in double quotes; the operation is performed only if the resource's existing latest `version_uid` (the `preceding_version_uid`) matches the header value. | parameters/header/If-Match.yaml (description, `required: true`) | validity-fn | high |
| its-rest-general-R9 | The `openEHR-TEMPLATE_ID` request header MUST be used when committing a COMPOSITION via `PUT`/`POST` using a simplified data format that carries no `LOCATABLE.archetype_details.template_id`. | Requests_and_responses.md §"openEHR-TEMPLATE_ID" L100-102 | header | medium |
| its-rest-general-R10 | Services MUST return the `Location` response header whenever a create or update operation was performed. | Requests_and_responses.md §"Location" L111-112 | header | high |
| its-rest-general-R11 | On `Prefer: return=minimal` (or no `Prefer`, since minimal is the default), a `Location` header giving the direct resource URL MUST be part of the response; if there is no payload the service SHOULD use `204 No Content`. | Requests_and_responses.md §"Representation details negotiation" L245-248, L255 | behaviour | high |
| its-rest-general-R12 | The default representation policy when no `Prefer` header is present MUST be `return=minimal`. | Requests_and_responses.md L255; parameters/header/Prefer.yaml (`default: return=minimal`) | behaviour | medium |
| its-rest-general-R13 | On `Prefer: return=representation` the payload SHOULD include a full representation of the (modified) resource; a `Location` header MAY be present. | Requests_and_responses.md L250-253 | behaviour | low |
| its-rest-general-R14 | The `Prefer` header value is constrained to the enum `return=representation` / `return=minimal`; other values are not defined by the contract. | parameters/header/Prefer.yaml (`enum`) | validity-fn | low |
| its-rest-general-R15 | To indicate request/operation status, one of the specification's HTTP status codes MUST be used (200,201,204,400,401,403,404,405,406,408,409,412,415,422,500,501). | Requests_and_responses.md §"HTTP status codes" table + L196 | status-code | medium |
| its-rest-general-R16 | `400 Bad Request` MUST be used when the request URL or body could not be parsed or has invalid content, and only when no more specific 4xx applies. | Requests_and_responses.md L200-201; responses/400.yaml | status-code | medium |
| its-rest-general-R17 | If an authentication/authorization framework is present, the service MUST use `WWW-Authenticate` and/or `Proxy-Authenticate` response headers and return `401`, `403`, or `407` whenever applicable. | Requests_and_responses.md §"Authentication and authorization" L35-38 | behaviour | medium |
| its-rest-general-R18 | Services MUST support at least one of the openEHR XML or JSON formats for resource representation. | Resources.md §"Data representation" L61 | behaviour | medium |
| its-rest-general-R19 | When a request payload is XML, if the service cannot process XML (format not supported) it MUST respond `415 Unsupported Media Type`. | Resources.md §"XML Format" L69-70 | rejection-duty | high |
| its-rest-general-R20 | When the client sends `Accept: application/xml` and the service cannot fulfil it, the service MUST respond `406 Not Acceptable`. | Resources.md L72-73; responses/406.yaml | rejection-duty | high |
| its-rest-general-R21 | For an XML response with a body, `Content-Type: application/xml` MUST be present (unless the response is `204` with no content body). | Resources.md L74-75 | header | medium |
| its-rest-general-R22 | XML request payloads and results MUST be valid against the published ITS-XML XSDs. | Resources.md L67 | serialization | medium |
| its-rest-general-R23 | When a request payload is JSON, if the service cannot process JSON it MUST respond `415 Unsupported Media Type`. | Resources.md §"JSON Format" L118-119 | rejection-duty | high |
| its-rest-general-R24 | When the client sends `Accept: application/json` and the service cannot fulfil it, the service MUST respond `406 Not Acceptable`. | Resources.md L121-122 | rejection-duty | high |
| its-rest-general-R25 | For a JSON response with a body, `Content-Type: application/json` MUST be present (unless `204` with no content body). | Resources.md L123-124 | header | medium |
| its-rest-general-R26 | The `_type` metadata attribute value MUST be the uppercase RM class name; `_type` MUST be present whenever polymorphism applies or the declared RM type is abstract (dynamic type differs from static type). | Resources.md §"JSON Format" L99-105 | serialization | high |
| its-rest-general-R27 | JSON attribute names MUST be lowercase snake_case as in the equivalent RM type; metadata (non-RM) attributes MUST be prefixed with `_`. | Resources.md L83, L99 | serialization | medium |
| its-rest-general-R28 | RM attributes (even required ones) that are Null / empty list / empty array SHOULD be absent when serialized as JSON. | Resources.md L114 | serialization | low |
| its-rest-general-R29 | Alternative simplified data formats MUST use their exact media types: `application/openehr.wt.flat+json` (simSDT flat), `application/openehr.wt.structured+json` (structured), `text/plain` (ADL2 templates / AQL), `application/openehr.nc.flat+json` (ncSDT), `application/openehr.tds2+xml` (TDS/TDD). | Resources.md §"Alternative data formats" L138-164 | serialization | medium |
| its-rest-general-R30 | When a request payload is in a simplified format the service does not support, it MUST respond `415 Unsupported Media Type`. | Resources.md L170-171 | rejection-duty | high |
| its-rest-general-R31 | When the client's `Accept` for a simplified format cannot be fulfilled, the service MUST respond `406 Not Acceptable`. | Resources.md L173-174 | rejection-duty | high |
| its-rest-general-R32 | For any simplified-format response with a body, the appropriate `Content-Type` MUST be present (unless `204` with no content body). | Resources.md L175-176 | header | medium |
| its-rest-general-R33 | HTTP query parameters and path segments that are dates/datetimes/times MUST use the extended ISO 8601 format (`YYYY-MM-DDThh:mm:ss.sss[Z|±hh:mm]`). | Resources.md §"Datetime format" L185-186 | validity-fn | medium |
| its-rest-general-R34 | Date/datetime/time values inside a request body (e.g. DV_DATE_TIME in COMPOSITION content) MUST be preserved exactly as sent and passed to the backend unchanged; retrieval SHOULD return them in the original format, avoiding any format change. | Resources.md L189-191 | behaviour | medium |
| its-rest-general-R35 | `ETag` and `Last-Modified` SHOULD be present in all responses targeting a VERSION, VERSIONED_OBJECT, or other resource with a similar unique identifier. | Requests_and_responses.md §"ETag and Last-Modified" L167-168 | header | medium |
| its-rest-general-R36 | The `Last-Modified` response header value MUST be the datetime taken from `VERSION.commit_audit.time_committed.value` (HTTP-date format). | Requests_and_responses.md L159-165 | header | medium |
| its-rest-general-R37 | The `ETag` value uniquely identifies the resource state and changes as soon as the resource changes; recommended value is the resource's unique identifier (e.g. VERSION.uid.value / EHR.ehr_id.value). | Requests_and_responses.md L148-157; headers/ETag.yaml, headers/ETag_VERSION.yaml | header | low |
| its-rest-general-R38 | The `OPTIONS /` endpoint MUST exist at the API root; the service SHOULD respond with appropriate HTTP codes, headers, and potentially a conformance-manifest payload. | overview.openapi.yaml (paths `/` → options); operations/options.yaml | behaviour | medium |
| its-rest-general-R39 | The `OPTIONS` `200` response includes an `Allow` header listing the supported methods (e.g. `GET, POST, PUT, DELETE, OPTIONS`). | responses/200_options.yaml; headers/Allow.yaml | header | low |
| its-rest-general-R40 | The `OPTIONS` conformance payload (`Options` schema) exposes `solution`, `solution_version`, `vendor`, `restapi_specs_version`, `conformance_profile`, and `endpoints`. | schemas/others/Options.yaml | serialization | low |
| its-rest-general-R41 | A `version_uid` MUST have the lexical form `object_id :: creating_system_id :: version_tree_id`, where `object_id` equals the containing `versioned_object_uid`. | Glossary_and_conventions.md L16; Resources.md §"Resource identification" L28-29 | serialization | medium |
| its-rest-general-R42 | Once a resource is persisted, its assigned identifier MUST never change for that resource instance. | Resources.md §"Resource identification" L18 | behaviour | low |
| its-rest-general-R43 | An implicit URI (versioned_object_uid, no version) references the same resource as the explicit version_uid URI only while the latest version is unchanged; once a new version is committed the implicit URI resolves to the new latest, not the old one. | Resources.md L41-53 | behaviour | medium |
| its-rest-general-R44 | In case of errors (status `400`–`500`) the service MAY return an error-detail body (message/code/errors list) only when `Prefer: return=representation` is present. | Requests_and_responses.md L203-236 | behaviour | low |
| its-rest-general-R45 | `409 Conflict` MUST be returned when a resource with the same identifier(s) already exists. | Requests_and_responses.md status table L189; responses/409.yaml | status-code | medium |
