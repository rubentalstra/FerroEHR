# ITS-REST Query API — spec audit + design

Audit of our realization of the **ITS-REST Query API** (development edition,
`STABLE`) against the vendored spec. Scope: the **wire** contract — the six
query-execution operations (ad-hoc + stored, latest/versioned, GET + POST),
the request parameter/body encoding, the `RESULT_SET` response document and
its metadata, the paging (`offset`/`fetch`) and scope (`ehr_id`) semantics,
the response headers (`ETag`, `Content-Type`) and the declared status codes
(200 / 400 / 404 / 408).

**Out of scope** (governed elsewhere, referenced not re-planned here):

- The **SM `I_QUERY_SERVICE` interface** parity (multi-EHR `List<UUID>`,
  `ehr_id_does_not_exist`, `RESULT_SET.id`, `RESULT_QUERY_DESCRIPTOR.executed`,
  `RESULT_SET_COLUMN.archetype_id`, `formalism`) — that is
  `docs/design/sm-platform/08-query.md`; its G-rows are cited here where the
  wire realizes (or fails to realize) an SM attribute, never re-derived.
- The **AQL language internals** (parser, IR, SQL lowering, terminology
  family) — `docs/design/aql-engine.md`.
- The **stored-query definition endpoint** (store/list/get) — that is the
  DEFINITION API (`definition_query_*.yaml`), a separate audit.

**Spec oracle** (read before any change):

- `docs/specs/openehr/ITS-REST/specifications/docs/query/` — `Description.md`
  (purpose; AQL is the query language), `Qualified_query_name.md` (stored-query
  identifier `[{namespace}::]{query-name}`, reserved name `aql`, SEMVER +
  partial-prefix resolution), `Query_types.md` (single-EHR / population /
  stored / ad-hoc), `Request.md` (GET-vs-POST, `ehr_id`/`offset`/`fetch`/
  `query_parameters`, the `openehr-ehr-id` header, the `ETag` response header),
  `Response.md` (`RESULT_SET` + `ResultSetMetadata`).
- `specifications/operations/query_execute_{adhoc_query,stored_query,
  stored_query_version}{,_body}.yaml` — the six operations, their parameter
  lists, and their declared responses (`200_Query`, `400_Query`, `404_Query`,
  `404_Query_version`, `408_Query`).
- `specifications/schemas/query/` — `AdhocQueryExecute` (`q` required),
  `Query` (stored POST body: `offset`+`fetch`+`query_parameters` all
  required), `ResultSet` (`rows` required; `meta`/`name`/`q`/`columns`
  optional), `ResultSetColumn` (`name` required, `path`), `ResultSetRow`
  (array of `ANY`), `ResultSetMetadata` (`_href`/`_type`/`_schema_version`/
  `_created`/`_generator`/`_executed_aql`, `additionalProperties: true`),
  `Fetch`/`Offset`/`QueryName`/`QueryVersion`/`QueryParameters`.
- `specifications/parameters/query/{q,ehr_id_Query,offset,fetch,
  query_parameters}.yaml`, `parameters/path/{qualified_query_name,version}.yaml`,
  `responses/200_Query.yaml` (declares `ETag` + `Content-Type` headers),
  `headers/ETag_RESULT_SET.yaml`.
- `computable/OAS/query-codegen.openapi.yaml` — the assembled bundle; paths
  `/query/aql`, `/query/{qualified_query_name}`, `/query/{qualified_query_name}/{version}`.

**Current implementation** (verified 2026-07-12):

- Wire dispatch: `app/ehrbase-rest/src/dispatch/query.rs` (196 lines) — all six
  operations, normalizing params/body → `AqlQueryRequest`, calling the
  `QueryService` seam, rendering the `RESULT_SET`
  (`dispatch/query.rs:61-145`).
- Generated contract: `crates/openehr-its/src/rest/generated/query.rs` — the
  six operations at the spec paths (`query.rs:167-206`), route table
  (`query.rs:217`).
- SM seam (just rewritten, multi-scope): `app/ehrbase-sm/src/services/query/`
  — `request.rs` (`AqlQueryRequest.ehr_ids: Vec<String>` at `request.rs:28`,
  `single_ehr_id()` at `request.rs:52`), `service.rs` (`QueryService` trait).
- Service glue + `RESULT_SET` assembly: `app/ehrbase/src/service/aql_query.rs`
  (`execute_aql` at `aql_query.rs:38`, `result_set_json` at `aql_query.rs:211`).
- SQL context: `app/ehrbase/src/aql/sql.rs` (`SqlCtx` at `sql.rs:65`, paging at
  `sql.rs:975`, population gate at `sql.rs:488`); exec/column meta:
  `app/ehrbase/src/aql/exec.rs` (`ColumnMeta` at `exec.rs:21`).
- Response rendering: `app/ehrbase-rest/src/overview/negotiate.rs`
  (`respond` at `negotiate.rs:283`, `json_response` sets `Content-Type` at
  `negotiate.rs`).
- Global request timeout → 408: `app/ehrbase-rest/src/router.rs:112`.
- ECC: AqlBasic + QueryProvisioning areas exercise this surface (blueprint §2,
  green at B6 close).

---

## 1. Faithful realizations (evidence)

This is a **high-compliance** surface; the matches, not only the gaps:

- **All six operations mounted at the spec paths.** `GET`/`POST /query/aql`
  (ad-hoc), `GET`/`POST /query/{qualified_query_name}` (stored latest),
  `GET`/`POST /query/{qualified_query_name}/{version}` (stored versioned) —
  the generated route table (`generated/query.rs:217-...`) matches
  `query-codegen.openapi.yaml` lines 223/272/332. Dispatch handles each
  `operationId` (`dispatch/query.rs:61-137`).
- **Reserved name `aql` handled structurally.** `/query/aql` is a static
  route that takes precedence over the `/query/{qualified_query_name}`
  parameterised route in axum, so a stored query can never shadow the ad-hoc
  endpoint — the effect the reserved-name rule requires
  (`Qualified_query_name.md`: "must not be `aql`").
- **`ehr_id` accepted as either the query parameter or the `openEHR-EHR-id`
  header.** `ehr_id_from_request` reads the `ehr_id` query param first, else the
  header (`dispatch/query.rs:179-185`) — exactly `Request.md` §About the ehr_id
  parameter ("MAY supply it as a query parameter … or alternatively as a
  request header").
- **GET and POST body forms both realized.** POST decodes `AdhocQueryExecute`
  (`q`+paging+params, `dispatch/query.rs:74-83`) and `Query` (paging+params;
  `ehr_id` from query/header, `dispatch/query.rs:149-162`), matching
  `AdhocQueryExecute.yaml` / `Query.yaml`.
- **`query_parameters` without `$` prefix, typed superset.** Bound into the
  engine `Params` (`aql_query.rs:152-172`); the spec types them
  `String→String` (`QueryParameters.yaml`) — we accept JSON scalars and degrade
  complex values to text, a tolerant widening (`Request.md` §Query parameters:
  "SHOULD NOT be prefixed with `$`").
- **`fetch`/`offset` vs AQL `LIMIT`/`OFFSET`/`TOP` collision → 400.**
  `compose_paging` rejects a REST `fetch` combined with an AQL `LIMIT`/`TOP`
  and a REST `offset` combined with an AQL `OFFSET` (`aql_query.rs:182-197`) —
  `Request.md` ("`fetch` … cannot be combined with AQL-top").
- **Negative paging treated as "no bound".** `build_paging` applies `LIMIT`
  only when `limit >= 0` and `OFFSET` only when `offset > 0`
  (`sql.rs:975-986`), the spirit of the SM "zero/negative = all/zero" rule.
- **Population gate.** With no explicit `ehr_id`, roots are restricted to EHRs
  whose `EHR_STATUS.is_queryable = True` (`sql.rs:488-500`) — `Query_types.md`
  §Population queries + the SM `ehr_ids` doc.
- **`RESULT_SET` shape.** `result_set_json` assembles `meta`
  (`_type`/`_schema_version`/`_created`/`_executed_aql`) + `q` + `columns[]`
  (`{name, path}` or `{name}`) + `rows[][]` (`aql_query.rs:211-235`), matching
  `ResultSet.yaml` (`rows` required; the rest optional) + `ResultSetColumn.yaml`
  (`name` required, `path` optional) + `ResultSetRow.yaml` (array of `ANY`).
- **Stored-query not-found → 404; parse/feature-reject → 400.** `NotFound`
  maps to 404 (`08-query.md` §1); AQL parse errors, unsupported features, and
  path/typing errors all map to `SmError::precondition` → 400
  (`aql_query.rs:45-67, 240-247`), matching `400_Query.yaml` / `404_Query.yaml`.
- **`Content-Type: application/json` emitted** on the 200 (`json_response`,
  `negotiate.rs`) — `200_Query.yaml` `Content-Type` header.
- **A 408 is emittable.** The global `TimeoutLayer::with_status_code(
  REQUEST_TIMEOUT, …)` (`router.rs:112-114`) returns 408 for any over-long
  request, covering `408_Query.yaml` at the transport level (see G-2 for the
  query-specific caveat).

---

## 2. Gap register (what is not spec-true today)

Every gap cites the governing spec file/operation and the code file:line. The
gaps are localized wire-level items, not structural.

| # | Gap | Spec citation | Today (file:line) |
|---|-----|---------------|-------------------|
| G-1 | **`ETag` response header never emitted.** `200_Query` declares an `ETag` header ("an identifier of the RESULT_SET", `W/"…"` weak form); `Request.md` lists it under "Related response headers". The `respond` helper writes only `Content-Type` and the body; no `ETag` is added (unlike the EHR/COMPOSITION groups, which set `ETag` on their reads). No `RESULT_SET` id exists to derive it from either (see G-6). | `responses/200_Query.yaml` (`ETag` header); `headers/ETag_RESULT_SET.yaml`; `Request.md` §Common Headers | `dispatch/query.rs:144` calls `negotiate::respond` → `negotiate.rs:283-296`, which emits no `ETag`. `grep ETag app/ehrbase-rest/src/dispatch/query.rs` = none. |
| G-2 | **No query-level execution timeout → 408 is only a blunt global cutoff.** `408_Query` is "returned when there is a **query execution** timeout (maximum query execution time reached, therefore the server **aborted the execution of the query**)". We have only a whole-request `TimeoutLayer` (`router.rs:112`); the query engine sets no `statement_timeout` and cannot abort a slow query specifically, so a heavy query either completes or trips the coarse global timeout (empty body, not query-scoped). `map_exec_error` has no 408 path — a DB error becomes 500 (`exception`). | `responses/408_Query.yaml`; `operations/query_execute_*` `'408'` | `aql_query.rs:251-260` maps DB errors → `SmError::exception` (500), never 408. No per-query `statement_timeout` in `aql/exec.rs` / `aql/sql.rs`. |
| G-3 | **Multi-EHR scope is half-plumbed — a live mid-refactor drift.** The SM seam was rewritten to `ehr_ids: Vec<String>` (`request.rs:28`) and dispatch now collects the single wire `ehr_id` into that vec (`dispatch/query.rs:66-71,156`), but the service glue still reads a **single** `request.ehr_id` field that no longer exists on the struct (`aql_query.rs:77`), and `SqlCtx.ehr_id` is a single `Option<Uuid>` (`sql.rs:69`) lowered to `ehr_id = $1` / `IN (population subquery)` (`sql.rs:460-474`). So (a) the `ehrbase` crate does not currently reconcile with the rewritten seam, and (b) even once it does, a set of EHRs cannot reach the SQL. This is the wire/service realization of `08-query.md` **G-1**. The ITS-REST endpoint itself binds only one `ehr_id`, so full `List<UUID>` scoping is an extension; single-EHR is the conformant wire path. | `parameters/query/ehr_id_Query.yaml` (single `ehr_id`); `Request.md` §About the ehr_id parameter; cross-ref `08-query.md` G-1 (`i_query_service.adoc` `ehr_ids: List<UUID>`) | `aql_query.rs:77` (`request.ehr_id.as_deref()` — stale field); `sql.rs:65-69` (`SqlCtx.ehr_id: Option<Uuid>`); `sql.rs:460-474` single-id filter. |
| G-4 | **`ehr_id_does_not_exist` never raised — a non-existent EHR yields an empty result set.** `Request.md` says `ehr_id` "MUST NOT be supplied for population queries", and the SM declares `ehr_id_does_not_exist` as the sole error for both functions. `parse_ehr_id` validates only UUID **syntax** (malformed → 400); a well-formed but absent id is passed straight to the population/scope filter, returning `{ "rows": [] }`. This is `08-query.md` **G-2** at the wire. | `Request.md` §About the ehr_id parameter; cross-ref `08-query.md` G-2 (`i_query_service.adoc` `.Errors`) | `aql_query.rs:200-207` — syntax-only; no existence probe before `aql::execute` (`aql_query.rs:93`). |
| G-5 | **`meta._executed_aql` is the raw input AQL, not the parameter-substituted text.** `ResultSetMetadata._executed_aql`: "the actual AQL query that was executed by the server, **after replacing the query parameters**". We copy the verbatim request AQL (with `$params` still present) into both `q` and `_executed_aql`. This is `08-query.md` **G-4** (`RESULT_QUERY_DESCRIPTOR.executed`) surfaced on the wire. | `schemas/query/ResultSetMetadata.yaml` (`_executed_aql`); cross-ref `08-query.md` G-4 | `aql_query.rs:220-227` — `"_executed_aql": aql` (the raw `&str`); params are bound only at SQL lowering (`build_params`, `aql_query.rs:152`). |
| G-6 | **`meta._href` never emitted (GET endpoints).** `ResultSetMetadata._href`: "URL of the executed query (**only for GET endpoint**)". The assembled `meta` omits `_href` for all forms. Low value (debugging metadata, optional) but declared. | `schemas/query/ResultSetMetadata.yaml` (`_href`) | `aql_query.rs:220-227` — no `_href`; `result_set_json` takes no request-URL context. |
| G-7 | **`meta._generator` not emitted.** Optional metadata ("identifier of the application that generated the result"). Harmless omission; noted for completeness so it is not mistaken for drift. `additionalProperties: true` means our subset is schema-valid. | `schemas/query/ResultSetMetadata.yaml` (`_generator`) | `aql_query.rs:220-227` — absent. |
| G-8 | **`fetch = 0` returns zero rows, not "all".** `build_paging` applies `LIMIT` whenever `limit >= 0`, so `fetch=0` → `LIMIT 0` → empty result. ITS-REST leaves the zero/negative semantics implementation-defined ("default depends on the implementation"), but the SM `rows_to_fetch` it realizes says "zero or negative value means 'all'". A minor divergence at the `0` boundary (negative is already treated as unbounded). | `schemas/query/Fetch.yaml`; cross-ref SM `stored_query_execute_spec.adoc` `rows_to_fetch` | `sql.rs:976-980` — `limit >= 0` applies `LIMIT limit` (0 included). |

**Explicitly not ITS-REST gaps** (SM-richer-than-wire; recorded so they are not
re-filed here): `RESULT_SET.id` (`08-query.md` G-3) and
`RESULT_SET_COLUMN.archetype_id` (`08-query.md` G-5) are **absent from the
vendored `ResultSet`/`ResultSetColumn` schemas** — emitting them is an additive
SM courtesy, not an ITS-REST requirement. Canonical **XML** for the query
response is likewise not required: the query operations declare only
`Accept_JSON` and `200_Query` carries only `application/json`, so our JSON-only
answer (XML `Accept` → 406, `negotiate.rs:290`) is conformant.

---

## 3. Target design

Corrective, not a rebuild. Ordered by value.

### 3.1 Reconcile the multi-EHR refactor + close the wire scope (G-3)

- **Finish the seam migration in `ehrbase`**: replace `request.ehr_id`
  (`aql_query.rs:77`) with `request.ehr_ids` handling — parse each id, and widen
  `SqlCtx.ehr_id: Option<Uuid>` to `ehr_ids: Vec<Uuid>` (`sql.rs:65-69`). The
  scope filter (`sql.rs:460-474`) becomes `node.ehr_id = ANY($1)` when the set is
  non-empty, reusing the same root-join machinery; the population gate
  (`sql.rs:488`) still fires only for the empty-set (unscoped) case. Until this
  lands the `ehrbase` crate will not build against the rewritten seam — this is
  the immediate blocker.
- **Wire**: the conformant path stays a single `ehr_id` (query param or
  `openEHR-EHR-id` header). A multi-EHR list is an **extension** body field
  (`ehr_ids: [uuid,…]`) documented out of CORE/STANDARD scope — *no ITS-REST
  parameter governs a multi-EHR list; the SM `List<UUID>` is honoured through
  the extension while the single `ehr_id` remains the spec surface.*

### 3.2 `ehr_id_does_not_exist` (G-4)

- When a scope is present, probe existence (`SELECT 1 FROM ehr WHERE id = ANY($1)`)
  before executing, and raise a distinct error mapped per the query error table:
  **malformed UUID → 400** (`400_Query`), **well-formed but absent → 404**
  (the closest ITS-REST code; there is no dedicated 422 for the query scope).
  One cheap probe, only on scoped queries. Record the SM-error → ITS-REST-code
  choice as a PORT NOTE (the ITS-REST query responses do not enumerate an
  ehr-scope error).

### 3.3 Query-execution timeout → 408 (G-2)

- Set a per-query `statement_timeout` (config `EHRBASE_AQL__QUERY_TIMEOUT`, our
  own design — no openEHR spec governs the value) on the execution connection;
  map a Postgres `57014` (query canceled / statement timeout) SQLSTATE to a new
  `ApiError::RequestTimeout` → 408 with the ITS-REST body, distinct from the
  generic 500 path (`aql_query.rs:251`). Keep the global `TimeoutLayer` as the
  outer backstop. *No `RequestTimeout` variant exists on `ApiError`
  (`runtime.rs:24`) — add it (status 408).*

### 3.4 `RESULT_SET` metadata completion (G-1, G-5, G-6, G-7)

- **`_executed_aql`**: after `$param` binding, render the substituted text
  (reconstruct the AQL with each `$name` replaced by its bound literal) and put
  it in `_executed_aql`, keeping the raw source in `q` (`ResultSet.q` = "the
  given AQL"). Driven from `build_params` + the parsed AST, not re-planned here.
- **`ETag`** (G-1): derive a weak `ETag` from a stable hash of the assembled
  `RESULT_SET` (or a `uuidv7()` result-set id if `RESULT_SET.id` is added per
  `08-query.md` G-3) and set it in `dispatch/query.rs` alongside the negotiated
  body — a small `respond_with_etag` variant of `negotiate::respond`.
- **`_href`** (G-6): thread the request URI into `result_set_json` and emit
  `_href` for the GET operations only (omit on POST, per the schema note).
- **`_generator`** (G-7): emit a static build identifier (cheap, optional).

### 3.5 `fetch = 0` (G-8)

- Decide and document: either treat `fetch = 0` as unbounded (align with the SM
  `rows_to_fetch` "zero = all") or keep `LIMIT 0` as a documented ITS-REST
  reading. Prefer aligning with the SM to avoid a surprising empty result;
  a one-line change at `sql.rs:976` (`limit > 0`).

### 3.6 Verification

- Unit/integration: `ETag` present on 200; `_executed_aql` shows substituted
  literals; `_href` on GET only; non-existent-EHR (404) vs malformed-UUID (400)
  vs population (empty ok); `statement_timeout` → 408; multi-EHR set membership;
  `fetch=0` semantics.
- ECC: the AqlBasic / QueryProvisioning areas gain cases for the ETag header,
  the ehr-scope error, and (if adopted) the timeout path; zero-drift gate holds.

---

## 4. Standing PORT NOTEs (the honest residue)

- **Multi-EHR `ehr_ids` list is an extension** — the ITS-REST query endpoint
  parameters bind only a single `ehr_id`; the SM `List<UUID>` is realized as an
  extension body field. *No ITS-REST parameter governs the multi-EHR case —
  our own extension over the SM interface* (cite `ehr_id_Query.yaml` +
  `i_query_service.adoc`, never an ADR).
- **`ehr_id_does_not_exist` → HTTP code** — the ITS-REST query responses
  enumerate 400/404/408 only; the SM scope error is mapped to 404 (absent EHR)
  / 400 (malformed id). Documented mapping, not a spec-declared query code.
- **`statement_timeout` value** is our own design — *no openEHR spec governs the
  maximum query execution time*; only that a 408 is returned when it is reached
  (`408_Query.yaml`).
- **SM-richer-than-wire attributes** (`RESULT_SET.id`,
  `RESULT_SET_COLUMN.archetype_id`, full `RESULT_QUERY_DESCRIPTOR`) are emitted
  additively where they do not break the vendored `ResultSet` schema, or left to
  `08-query.md`; the wire schema is the conformant surface.
- **JSON-only query response** — the query operations declare only
  `application/json`; an XML `Accept` yields 406. Conformant with the vendored
  OAS (unlike the resource APIs, "XML on every endpoint" does not extend to the
  query result).
- **Partial-SEMVER stored-query resolution** (`{major}` / `{major}.{minor}` →
  highest match) is a deliberate superset of the spec's SEMVER rule
  (`Qualified_query_name.md`, `parameters/path/version.yaml`) — an enhancement,
  noted so it is not mistaken for drift.
