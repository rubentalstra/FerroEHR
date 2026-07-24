# Catalogue audit — DEFINITION_QUERY chapter

Issue #231 · audited 2026-07-24 · 10 cases · verdicts: 10 ok / 0 defects / 0 ambiguities

Chapter context: CNF `master05-func_tc_definition_query.adoc` is a stub chapter —
every official case table carries xx/TBD cells (verified: Description/Pre/Post/Flow
all `xx` for has_query-xxx, valid_query-valid/-invalid/-bad_formalism,
list_queries-*). AMB-38 records the stub-chapter handling (SM-derived flows,
disclosure guard on every case); all seven SM-derived cases carry that guard.
The three name-grammar cases (PR #263) are explicit catalogue additions beyond
the official cells, each flagged as such in its authoring comment.

| case | verdict | evidence | resolution |
|---|---|---|---|
| has_query-xxx | ok | SM `i_definition_query.adoc`: has_query "Return True if the query identified by a_query_name is registered" — flow (store, then check true) realizes it; AMB-19 covers the missing dedicated existence endpoint (realized via list-by-name GET) | none |
| list_queries-empty | ok | SM list_queries "List all registered queries" (0..1 optional op); wire `definition-codegen.openapi.yaml` GET `/definition/query/{qualified_query_name}` treats the name as a PATTERN and defines only a 200 response → empty result is 200 + empty list, so `expect: ok` + `returns: []` is derivable; `requires: server: exclusive` correctly guards the global-state ground | none |
| list_queries-non_empty | ok | Same SM/wire ground; store-then-list flow; message_exemplar assertion consistent with the stub-chapter draft posture | none |
| list_queries-select_items | ok | SM list_queries(item_offset 0..1, items_to_fetch 0..1) verified in `i_definition_query.adoc`; the wire GET carries only `qualified_query_name` + `Accept` (verified in the vendored OAS — no offset/fetch parameters), so pagination is not wire-realizable; AMB-20 records exactly this with `fixed_handling` | none |
| store_query-unqualified_name | ok | ITS-REST `Qualified_query_name.md`: "`[{namespace}::]{query-name}` … The `namespace` is optional", `my_compositions` in the valid-examples list — namespace-less store must succeed; flow stores + has_query-verifies | none |
| store_query-dotted_name | ok | Same file: "The `query-name` may include any combination of characters, matched by the pattern `[a-zA-Z0-9_.-]`" — the dot is inside the query-name charset and the separator is `::`, so `cnf.ward_dashboard-probe` is spec-valid namespace-less; `requires: server: empty` prevents any cross-case name collision (dim 4) | none |
| store_query-reserved_name | ok | Same file §NOTE: "The `query-name` value must not be `aql` (case-insensitive), as that is a reserved name" — both `aql` and `AQL` steps expect `validation_failed`; case-insensitivity is verbatim spec text | none |
| valid_query-valid | ok | SM valid_query(a_query_text, a_type) "True if the provided query text is a valid instance of the formalism"; store_query carries `Pre_valid_query: is_valid_query(a_query_text)` — realization via store is sound (AMB-18 `fixed_handling`); valid AQL text + `query_type: AQL` → stored | none |
| valid_query-invalid | ok | Same ground: syntactically invalid AQL fails `Pre_valid_query` → rejected; AMB-18 carried | none |
| valid_query-bad_formalism | ok | SM master04 §Query Formalism: `a_type` names the formalism ("AQL"/"aql"/"AQL::1" equivalent, case-insensitive) — an unsupported formalism (`XPATH`) cannot satisfy the validity check → rejected; AMB-18 carried | none |

Checks common to the chapter:
- **Ground (dim 1):** master05 stub status verified in the vendored text; AMB-38 guard on every SM-derived case; the three grammar additions carry explicit authoring comments.
- **Expectations (dim 2):** every expect/assert recomputed from SM operation signatures + the Qualified_query_name grammar + the OAS response set; no observed-behaviour echoes found. The SM master04 §Registered Queries naming schemes (`<ns>::<name>`, `<ns>::<formalism>::<name>`, `misc` default namespace) are consistent with the grammar cases.
- **Citations (dim 3):** every `spec_refs` target opened and confirmed to say what the case claims.
- **Fixtures (dim 4):** no corpus fixtures used; `server: empty`/`exclusive` requirements prevent shared-SUT collisions (incl. the dotted probe name also used by the perf pack).
- **Captures (dim 5):** no cross-step captures beyond stored-name reuse within a case; names are consistent per case.
- **Ambiguity tags (dim 6):** AMB-18/19/20/38 all read and each covers precisely the divergence its case carries.
