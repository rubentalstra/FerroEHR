---
name: query-api-ops-location
description: Where the 6 ITS-REST Query API execution operations are specified (dedicated released prose + decomposed OAS + SM I_QUERY_SERVICE) and the confirmed gaps/defects in that text
metadata:
  type: reference
---

# Query API (6 execution ops) — location map

Complements [[aql-result-set-equivalence-location]] (AQL semantics + RESULT_SET
shape), [[its-rest-wire-contract-location]] (cross-cutting wire rules) and
[[sm-query-service-chapter8-location]] (the SM ch.8 side: class map, diagram-only
multiplicities, the 5 rival name grammars, the unfinished SPECPR-292 rename).

**The Query API is one of the few API groups with DEDICATED released prose** —
unlike the EHR API whose docs dir is an overview-only stub (see
[[ehr-status-ops-location]]). Prose is the oracle; OAS is shapes only.

`docs/specs/openehr/ITS-REST/specifications/`:
- `docs/query/Request.md` — THE parameter contract. §GET vs POST (POST
  recommended, URI-length), §Common Headers and Query Parameters (the
  `ehr_id`/`offset`/`fetch`/`query_parameters` SHOULD-list + "Related request
  headers: `openehr-ehr-id`" + "Related response headers: `ETag`"), §About the
  ehr_id parameter (the MUST NOT for population queries; the AMB-59 duality
  sentence), §Query parameters (the named-binding law: "SHOULD NOT be prefixed
  with `$`", two worked GET examples).
- `docs/query/Qualified_query_name.md` — the FULL name grammar
  (`[{namespace}::]{query-name}`, reverse-domain namespace, `[a-zA-Z0-9_.-]`,
  the `aql` reserved-name NOTE from SPECITS-46) + the SEMVER partial-prefix
  latest-match rule (one sentence, covers both omitted and partial version).
- `docs/query/Query_types.md` — single-EHR / population / stored / ad-hoc taxonomy.
- `docs/query/Response.md` — 14 lines, NO normative content: two
  `<SchemaDefinition>` refs (ResultSet, ResultSetMetadata) + "metadata comprise
  a set of optional (implementation dependent) attributes, useful for debugging".
  All RESULT_SET requiredness therefore comes from the SCHEMAS, not prose.
- `docs/query/Description.md` — STABLE status marker only.
- `query.openapi.yaml` — the 3 paths → 6 operations (GET+POST each).
- `operations/query_execute_{adhoc_query,adhoc_query_body,stored_query,
  stored_query_body,stored_query_version,stored_query_version_body}.yaml`.
- `responses/{200,400,404,404_Query_version,408}_Query.yaml`;
  `headers/ETag_RESULT_SET.yaml` (example `W/"<uuid>"`).
- `parameters/query/{q,ehr_id_Query,offset,fetch,query_parameters}.yaml`,
  `parameters/path/{qualified_query_name,version}.yaml`,
  `parameters/header/{Accept_JSON,ContentType_JSON}.yaml`.
- `schemas/query/{AQL,AdhocQueryExecute,Query,QueryName,QueryVersion,QueryType,
  QueryParameters,Offset,Fetch,StoredQuery,ResultSet*}.yaml`.
- Stored-query DEFINITION side (store/list/get, needed for resolution +
  round-trip): `operations/definition_query_*.yaml`,
  `responses/200_StoredQuery_{get,stored}.yaml`, `200_QueryList.yaml`,
  `409_StoredQuery_version.yaml`, `headers/Location_Query.yaml`.

**SM**: `SM/docs/openehr_platform/master08-query_service.adoc` (§Overview holds
the SM stored-query id form `reverse-domain-name '::' semantic-id [ '/' version ]`)
+ `SM/docs/UML/classes/{i_query_service,stored_query_execute_spec,
adhoc_query_execute_spec,result_set,result_set_column,result_set_row,
result_query_descriptor,query_descriptor}.adoc`.

**AQL binding**: `QUERY/docs/AQL/master03-syntax.adoc` §Reserved words (L15),
§Parameters + §Syntax (L97/L104 — `$name`, letters/digits/underscores, not a
reserved word), §Parameter Resolution (L132 — "no specific guidelines"),
§TOP (deprecated, not combinable with LIMIT), §LIMIT (L1115),
§ORDER BY (L1094). `master04-result_structure.adoc` (raw = Array<Array<Any>>;
annotated result sets delegated to SM/REST).

**CONFIRMED GAPS / DEFECTS (all verified first-hand 2026-07-27):**
- `openehr-ehr-id` has NO parameter file in `parameters/header/` and is declared
  on NO operation — it exists ONLY in Request.md prose (and the overview
  deprecated-header table `openEHR-EHR-id` → `openehr-ehr-id`).
- `schemas/query/Query.yaml` (stored-query POST body) declares
  `required: [offset, fetch, query_parameters]` — all three REQUIRED, and it has
  no `q`. Contradicts the prose defaults (offset default 0, fetch
  implementation-dependent).
- POST ops declare NO `ehr_id` anywhere (not a param, not in either body schema).
- `responses/400_Query.yaml`/`404_Query*`/`408_Query.yaml` declare NO body
  (unlike the generic `responses/400.yaml` which refs `schemas/others/Error.yaml`).
- `Accept_JSON`/`ContentType_JSON` enums = `application/json` only → the Query
  API OAS binds JSON only; ITS-XML has a QUERY RESULT_SET XSD but only as a
  README stub marked "draft" (`ITS-XML/components/QUERY/latest/README.adoc`,
  no XSD file vendored).
- Request.md's headline example AQL ends `ORDER BY temperature DESC FETCH 3` —
  `FETCH` is NOT an AQL construct (grep "FETCH" across `QUERY/` = zero hits);
  the same example's node predicate `items[at0.63 and name/value='Symptoms']`
  is malformed. Released-example defect.
- Glossary_and_conventions.md defines NO query term (`q`, `offset`, `fetch`,
  `qualified_query_name`, query `version` all absent).
- CNF: `master11-func_tc_querying.adoc` is a stub (all "xx"/TBD); the Robot
  suite `CNF/tests/platform/robot/I_QUERY_SERVICE/` implements only
  `POST /query/aql` — all eight GET stored-query keywords in
  `_resources/keywords/aql_query_keywords.robot` (L268-296) are `No Operation`.
- SM↔ITS-REST conflicts (no register entry existed as of 2026-07-27):
  SM RESULT_SET requires columns/id/creation_time (rows 0..1) vs ITS-REST
  requiring only `rows`; SM namespace mandatory + `/version` separator vs REST
  optional namespace + path segment; SM version "3-part semver" vs REST partial
  prefix; SM `query_parameters: Hash<String,String>` vs REST typed JSON values;
  SM `ehr_ids: List<UUID>` vs REST single `ehr_id`; SM
  `rows_to_fetch` "zero or negative means all" / `row_offset` "zero or negative
  means offset of zero" — no REST equivalent, and the SM's own amendment record
  (0.9.5/SPECPR-292, 28 Feb 2019) says those two names were renamed to
  `item_offset`/`items_to_fetch`, which master02+master08 use but the class
  file never adopted; SM error `ehr_id_does_not_exist` has NO REST 404 on the
  ad-hoc ops.
