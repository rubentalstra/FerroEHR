# Query Service (`I_QUERY_SERVICE`) — SM interface audit + design

Audit of our realization of the openEHR SM **Query Service** against the
vendored spec. Scope: the **SM interface** parity — ad-hoc + stored query
execution, the execute-spec parameter semantics, the `RESULT_SET` shape, and
paging/scope semantics. **Out of scope:** the AQL language internals (parser,
IR, SQL lowering, terminology family) — those are governed by
`docs/design/aql-engine.md` and are only referenced here, never re-planned.

**Spec oracle** (read before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master08-query_service.adoc`
  (the Query Service chapter: model of querying, stored-query identifier form,
  `RESULT_SET` overview, paging parameters)
- `docs/specs/openehr/SM/docs/UML/classes/` — `i_query_service.adoc`,
  `stored_query_execute_spec.adoc`, `adhoc_query_execute_spec.adoc`,
  `result_set.adoc`, `result_set_column.adoc`, `result_set_row.adoc`,
  `result_query_descriptor.adoc` (+ its `query_descriptor.adoc` ancestor)
- Adjacent: the ITS-REST QUERY binding
  (`docs/specs/openehr/ITS-REST/specifications/docs/query/Request.md`) — the
  wire realization of the abstract SM `EXECUTE_SPEC` types.

**Current implementation** (verified 2026-07-12):

- SM trait: `app/ehrbase-sm/src/services/query.rs` (52 lines) — `QueryService`
  with `query_execute_adhoc` + `query_execute_stored`, both defaulting to
  `NotImplemented` (`query.rs:26`, `query.rs:40`).
- Execute-spec types: `app/ehrbase-sm/src/types.rs` — `AqlQueryRequest`
  (`types.rs:325`) + `QueryOutcome` (`types.rs:348`) realize the two SM
  `EXECUTE_SPEC` classes plus the paging/scope parameters.
- Impl: `app/ehrbase/src/service/api/query.rs` (46 lines) — `impl QueryService
  for EhrbaseService`; both methods delegate to `execute_aql`
  (`query.rs:20`, `query.rs:43`); stored resolution via `get_stored_query`
  (`query.rs:31`).
- Engine glue + `RESULT_SET` assembly:
  `app/ehrbase/src/service/aql_query.rs` (`execute_aql` at `aql_query.rs:38`;
  `result_set_json` at `aql_query.rs:211`); the AQL engine proper is
  `app/ehrbase/src/aql/`.
- Stored-query store: `app/ehrbase/src/service/stored_query.rs`
  (`get_stored_query` at `stored_query.rs:100`).
- Wire (REST dispatch): `app/ehrbase-rest/src/dispatch/query.rs` (196 lines) —
  six operations covering ad-hoc + stored (latest/versioned) × (params/body).
- ECC: AqlBasic + QueryProvisioning areas exercise this surface (blueprint §2
  records these green at B6 close).

---

## 1. Faithful realizations (evidence)

The SM interface is realized closely; recording the matches, not only the
gaps:

- **Both interface functions present with SM-faithful signatures.**
  `execute_stored_query` → `query_execute_stored(qualified_query_name,
  version, request)` and `execute_ad_hoc_query` → `query_execute_adhoc(aql,
  request)` (`app/ehrbase-sm/src/services/query.rs:26,40`). The `exec_spec`
  attributes (`ADHOC_QUERY_EXECUTE_SPEC.source`,
  `STORED_QUERY_EXECUTE_SPEC.qualified_query_name` + `version` +
  `query_parameters`) map onto the trait arguments plus `AqlQueryRequest`
  (`types.rs:318-341`).
- **`query_parameters` (`Hash<String,String>`) realized** as the request
  `parameters` map, bound into the engine `Params` (`aql_query.rs:152-172`).
  The SM types them `String→String`; we accept JSON scalars and degrade
  complex values to text — a tolerant superset.
- **Stored-query identifier form honoured.** `reverse-domain::semantic-id`
  split (`stored_query.rs:105`) and the optional 3-part SEMVER version, with
  "latest when not supplied" (`stored_query.rs:128-135`), match
  master08 §Overview and `stored_query_execute_spec.adoc`.
- **Paging parameters `row_offset`/`rows_to_fetch`** realized as
  `offset`/`fetch` with the SM's "zero/negative means all/zero" spirit
  composed against AQL `LIMIT`/`OFFSET`/`TOP` and collisions rejected `400`
  (`aql_query.rs:70-76,177-197`) — the "handle large result sets efficiently"
  requirement of master08 §Overview.
- **Population-query gate.** With no explicit EHR scope, the query is
  restricted to EHRs whose `EHR_STATUS.is_queryable = True`
  (`app/ehrbase/src/aql/sql.rs:479-500`), exactly as the `ehr_ids` parameter
  doc mandates (`i_query_service.adoc` `_ehr_ids_` note).
- **Stored-query not-found → 404.** `get_stored_query` returns
  `ServiceError::NotFound` (`stored_query.rs:139`), mapped to
  `VersionedObjectDoesNotExist` / 404
  (`app/ehrbase/src/service/mod.rs:339`).
- **`RESULT_SET` is assembled** with columns (`{name,path}`), rows, and query
  metadata (`aql_query.rs:211-235`) in the ITS-REST 1.0.3 shape the wire
  designates; content-negotiated JSON/XML at the edge (`dispatch/query.rs:144`).

---

## 2. Gap register (what is not spec-true today)

Every gap cites the governing spec text. This is a **high-compliance** surface;
the gaps are localized, not structural.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **Multi-EHR scoping (`ehr_ids: List<UUID>`) is not realized — only a single EHR.** Both interface functions take `ehr_ids: List<UUID>[0..1]` ("Specific set of EHRs on which to execute the query"). Our request carries only `ehr_id: Option<String>` (`types.rs:326`), sourced from one `ehr_id` query param / `openEHR-EHR-id` header (`dispatch/query.rs:179-185`) and lowered to a single-UUID scope (`aql_query.rs:200-207`, `SqlCtx.ehr_id`). A caller cannot scope to a *set* of EHRs. | `i_query_service.adoc` (`execute_stored_query`/`execute_ad_hoc_query` `_ehr_ids_` param, `List<UUID>`) | Single-EHR only. Note: the ITS-REST wire itself binds only a single `ehr_id`, so this is partly an ITS-REST/SM binding gap — but the SM interface is not fully realized. |
| G-2 | **`ehr_id_does_not_exist` is never raised.** It is the *only* declared error for both interface functions. `parse_ehr_id` validates UUID **syntax** (400 on malformed — `aql_query.rs:200-207`) but never checks EHR existence; a well-formed but non-existent `ehr_id` silently yields an empty result set instead of the declared error. | `i_query_service.adoc` (`.Errors` = `ehr_id_does_not_exist`, both functions) | No existence check on the query scope. |
| G-3 | **`RESULT_SET.id` not emitted.** SM `RESULT_SET.id [1..1]` ("Unique identifier of this result set"). `result_set_json` emits `meta`/`q`/`columns`/`rows`/`name` (`aql_query.rs:220-234`) but no result-set id. | `result_set.adoc` (`id [1..1]`) | Absent from the assembled document. Note: the ITS-REST 1.0.3 `ResultSet` schema does not carry `id` either — an SM/ITS-REST divergence, recorded not silently dropped. |
| G-4 | **`RESULT_QUERY_DESCRIPTOR.executed` is the raw AQL, not the parameter-substituted text.** SM defines `executed` as "Executed query text with **all parameters substituted**". We put the verbatim input AQL in `meta._executed_aql` (`aql_query.rs:225`, `name`/`q` passthrough). | `result_query_descriptor.adoc` (`executed`) | Parameters are bound at SQL-lowering, not surfaced in the descriptor; the returned "executed" text still shows `$params`. |
| G-5 | **`RESULT_SET_COLUMN.archetype_id` never populated.** SM column definition carries an optional `archetype_id [0..1]`. Our `ColumnMeta` has only `name` + `path` (`app/ehrbase/src/aql/exec.rs:21-25`), and `result_set_json` emits only those (`aql_query.rs:214-219`). | `result_set_column.adoc` (`archetype_id [0..1]`) | Column meta omits `archetype_id`. Optional in the SM; the spec's own NOTE questions whether it is needed vs. inside the path. |
| G-6 | **`formalism` is neither parsed nor rejected — the "rejected typed" claim is not realized.** `ADHOC_QUERY_EXECUTE_SPEC.formalism {default "aql"}` ("any other string value" permitted). The doc comment at `types.rs:321-323` states other formalisms are "rejected typed", but no code reads a formalism field (the ITS-REST `AdhocQueryExecute` wire struct has no `formalism`; dispatch reads only `q`/`offset`/`fetch`/`query_parameters` — `dispatch/query.rs:75-83`). | `adhoc_query_execute_spec.adoc` (`formalism`); the doc-comment claim | A supplied formalism cannot reach the server, so it is silently ignored rather than rejected — the doc claim is aspirational, not enforced. |

---

## 3. Target design

The interface is close to spec-true; the design is corrective, not a rebuild.

### 3.1 Multi-EHR scope (G-1)

- **`AqlQueryRequest.ehr_id: Option<String>` → `ehr_ids: Vec<String>`** (or
  add a parallel `ehr_ids` while keeping `ehr_id` as the single-element
  convenience), threaded to `SqlCtx` as a set. The population gate
  (`sql.rs:apply_population_gate`) already restricts EHR roots via an
  `IN (subquery)`; scoping to an explicit set becomes `ehr_id IN ($1,$2,…)`
  instead of `= $1`, reusing the same join machinery.
- **Wire**: the ITS-REST QUERY binding surfaces a single `ehr_id`; an
  explicit multi-EHR list is an **extension** on the request body
  (`ehr_ids: [uuid,…]`), documented as out of CORE/STANDARD scope. *No
  ITS-REST endpoint parameter governs a multi-EHR list — the SM interface
  does; the extension realizes the SM `List<UUID>` faithfully while the
  single `ehr_id` stays the conformant wire path.*

### 3.2 `ehr_id_does_not_exist` (G-2)

- Before execution, when a scope is present, verify each id exists in the
  `ehr` table (a single `SELECT … WHERE id = ANY($1)` existence probe) and
  raise the SM-declared error — mapped to the ITS-REST QUERY error body
  (`404`/`400` per `docs/specs/openehr/ITS-REST/.../query/`). Distinguish
  "malformed UUID" (400) from "well-formed, absent" (the declared
  `ehr_id_does_not_exist`). Cheap: one probe, only on scoped queries.

### 3.3 `RESULT_SET` completion (G-3/G-4/G-5)

- **`id`**: emit a `uuidv7()`-derived result-set id into the assembled
  document (SM `RESULT_SET.id [1..1]`), even though the ITS-REST schema does
  not require it — additive, harmless to conformant clients. Record the
  SM/ITS-REST divergence as a PORT NOTE.
- **`executed`**: after parameter binding, render the substituted text (or, if
  substitution happens only in SQL, reconstruct the AQL with `$params`
  replaced by their bound literals) and place it where the abstract
  `RESULT_QUERY_DESCRIPTOR.executed` maps on the wire; until then, keep the raw
  AQL but rename the field to reflect it is the *source*, not the *executed*
  text, so the descriptor is not mislabelled.
- **`archetype_id`**: when the AQL path analysis knows the archetype bound to a
  SELECT column, populate `ColumnMeta.archetype_id` and emit it; otherwise
  omit (it is `[0..1]`). Driven by the engine's path analysis
  (`docs/design/aql-engine.md`), not re-planned here.

### 3.4 `formalism` honesty (G-6)

- Either (a) parse a `formalism` field where the wire/body carries one and
  reject non-`aql` with a typed 400 (making the doc claim true), or (b) fix the
  `types.rs` doc comment to state plainly that the ITS-REST binding carries no
  formalism, so only AQL is ever received. Prefer (a) if the extension body is
  added anyway; (b) is the minimal correction.

### 3.5 Verification

- Unit/integration: multi-EHR scope (set membership); non-existent-EHR error
  vs. malformed-UUID 400; `RESULT_SET` shape (`id` present, `executed`
  substituted, `archetype_id` where known); formalism rejection if (a) chosen.
- ECC: the AqlBasic / QueryProvisioning areas gain cases for the
  non-existent-EHR error and multi-EHR scope; zero-drift gate holds.

---

## 4. Standing PORT NOTEs (the honest residue)

- **`RESULT_SET.id` and `RESULT_QUERY_DESCRIPTOR` divergence from ITS-REST:**
  the abstract SM `RESULT_SET` is richer than the vendored ITS-REST 1.0.3
  `ResultSet` schema (which has `meta`/`q`/`columns`/`rows`). The wire is the
  conformant surface; SM-only attributes (`id`, `creation_time` as a first-
  class field vs. `meta._created`, the full `RESULT_QUERY_DESCRIPTOR`) are
  emitted additively where they do not break the wire schema. Cite
  `result_set.adoc` + the ITS-REST query schema, never an ADR.
- **Multi-EHR `ehr_ids` list** is realized as an extension body field — the
  ITS-REST endpoint parameters bind only a single `ehr_id`; the SM
  `List<UUID>` is honoured through the extension. *No ITS-REST parameter
  governs the multi-EHR case — our own extension over the SM interface.*
- **`formalism`**: the SM permits "any other string value"; we support only
  AQL. A non-AQL formalism is a typed rejection (once G-6 is closed) — the
  supported-formalism set is a documented subset, re-checked if a second
  query formalism is ever adopted.
- **Partial-SEMVER stored-query resolution** (`{major}` / `{major}.{minor}`
  → highest match, `stored_query.rs:107-119`) is a **superset** of the SM's
  "3-part semver, else latest" — a deliberate enhancement, not a gap; note it
  so it is not mistaken for drift.
