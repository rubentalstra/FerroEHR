# Phase 16 — AQL engine (our typed IR over the node model, ADR-008)

> Re-scoped 2026-07-05 by ADR-008: our own design (no ASL port); EHRbase's
> engine is prior art only.

- Status: done (2026-07-07)
- Consumes: `openehr-query` (AST, done), P10 (node model), P14 (WebTemplate),
  the BMM-generated RM attribute model
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-008

## Objectives

Execute AQL 1.1: semantic/path analysis over a **BMM-generated RM attribute
model** (no reflection, no hand-maintained tables — `openehr-codegen` /
`openehr-lang` emit it), lowering into **our own typed query IR**, SQL
generation via `sea-query` against the P10 node model (nested-set interval
joins for CONTAINS, `jsonb_path_query_first` + jsonpath item methods +
`openehr_magnitude` for typed leaf comparison/ordering, `JSON_TABLE` for
array unnesting, GIN `jsonb_ops` `$.**` equality anchors as pre-filters),
execution via `sqlx`, and `RESULT_SET` (ITS-REST 1.0.3) assembly.

## Preconditions

- [x] P10 (node model), P14 (WebTemplate), RM attribute model generated

## Scope

**In:** the RM attribute-model codegen target; path analysis + typing
(candidate types, multi-valued detection, abstract→concrete expansion); the
IR + AST→IR lowering; IR→SQL incl. versioning semantics (`LATEST_VERSION`
partial-index path, `ALL_VERSIONS` — in scope, ADR-008); parameter binding;
the feature envelope (accept/reject documented per construct); `/query/aql` +
stored queries wired through P11/P12. **Out:** the parser (P07, done); query
caching / perf tuning (P20).

## Tasks

- [x] `emit-rm-model` (or `openehr-lang` API): attribute→types, multiplicity,
      descendant sets, structure classification — generated from BMM
      — new `openehr-codegen` target emits `openehr-rm::model` (static
      `RmClass`/`RmAttribute`/`Container` + `class`/`attribute`/`attributes`/
      `descendants`/`ancestors`/`is_a`/`is_structure_root`) from BASE+RM BMM;
      `is_structure_root` mirrors `ehrbase::storage::codec::STRUCTURE_TYPES`;
      wired into `emit`, the drift script, and `/regen-codegen`.
- [x] Path analysis + typing against the generated model
      — `ehrbase::aql::analyze`: the deterministic path split (structure-hop
      prefix via `model::is_structure_root` + fragment jsonpath suffix),
      abstract-slot expansion via `model::descendants`, multi-valued detection
      (any list/set step), and the `Coercion` decision (Magnitude/Text/Temporal/
      Boolean/Raw) incl. parent-aware temporal typing of `.../value` ISO leaves.
- [x] Query IR + AST→IR lowering (incl. CONTAINS trees, version addressing)
      — `ehrbase::aql::{ir,lower,error,mod}`: typed relational IR (no SQL) +
      full-envelope lowering + `plan()` entry with `$param` validation; 32 unit
      tests (path-split table, per-construct accept/reject, CONTAINS trees,
      version scoping, params). Compiles + clippy-clean + nextest green.
- [x] IR→SQL via sea-query (interval joins, typed extraction, magnitude,
      unnesting; LATEST/ALL_VERSIONS)
      — `ehrbase::aql::sql`: fully typed `sea-query` (no string-built SQL) — FROM
      as cross-join + typed `Expr` conditions; CONTAINS = interval self-joins,
      EHR/VERSION links; anchor descent via correlated scalar subqueries;
      `PgExpr::contains`/`concatenate`, built-in `Func` aggregates,
      `Func::cust` only for `jsonb_path_query_first`/`to_jsonb`/`openehr_magnitude`,
      `BinOper::Custom("#>>")` (the one operator sea-query has no variant for);
      `SqlError` variants added.
- [x] Execute + `RESULT_SET` assembly; QUERY endpoints live
      — `ehrbase::aql::exec` runs one statement via `sqlx` + `sea-query-sqlx`,
      reads scalar cells as canonical JSON, reassembles whole-object cells via the
      P10 codec (subtree re-based); `ehrbase-rest` gained a `QueryService` seam
      (re-joined to `Backend`) + `dispatch/query` for all six QUERY operations;
      `ehrbase::service::{aql_query,api::query}` implement it (parse→plan→SQL→exec,
      REST fetch/offset composed with AQL LIMIT/OFFSET, conflicts → 400).
- [x] AQL corpus end-to-end tests + the feature-envelope matrix
      — `tests/service_aql.rs` (testcontainers PG18): CONTAINS chains, magnitude
      WHERE+ORDER BY, DISTINCT, aggregates, LIMIT/OFFSET + fetch/offset (+conflict),
      NOT CONTAINS, `$params`, ehr_id scoping, VERSION uid/time selection,
      whole-COMPOSITION reassembly = `composition_get`, LATEST vs ALL_VERSIONS;
      `tests/service_query.rs` adds HTTP `/query/aql` (200 RESULT_SET; malformed → 400).
      Status matrix in `docs/design/aql-engine.md`.

## Exit criteria

- [x] The AQL conformance corpus executes correctly over real data
      (testcontainers), including version addressing (LATEST vs ALL_VERSIONS)
- [x] Feature envelope documented; every rejection is an explicit typed error
- [x] Compiles + clippy-clean + nextest green

## Decisions made this phase

- **IR shape (`ehrbase::aql::ir`, packages 2+3).** Sources are a flat `Vec<Source>`
  addressed by a dense `SourceId(index)`; `Source ∈ {Ehr, Rm, Version}`. The FROM
  containment is a separate `ContainsTree` (`Operand{source, contained: Option<Contained>}`
  + `And`/`Or`, `Contained{link: Contains|NotContains, tree}`) referencing sources
  by id — so cross-VO joins and anti-joins are structural, decided by the SQL
  package. Data paths are a `LeafPath` split into `anchor: Vec<StructureStep>`
  (node-row hops) + `fragment: Vec<FragmentStep>` (JSONB tail) + candidate
  `TypeSet` + `Coercion` + `multi_valued`. WHERE is a typed `Expr` tree; SELECT is
  `Vec<SelectColumn>`; every identified path is a `PathTarget ∈ {Data, Version, Ehr}`.
  **No SQL strings anywhere in the IR** (design §Typed IR).
- **Deviation from the design sketch — `VersionScope`.** The sketch listed a
  distinct `AtTime(param)` variant; AQL only expresses version-at-time as a
  standard predicate on version metadata, so all version-selection predicates
  lower uniformly to `VersionScope::Predicate(VersionMetaPredicate)` and the
  at-time case is recognised via `VersionScope::is_at_time()` (documented in
  `ir.rs`). `Latest`/`All` are their own variants (ADR-008 `ALL_VERSIONS` from
  day one). VERSION scope propagates down the contained subtree to the VO
  sources.
- **`Coercion` kept to the design's 5 variants.** `Magnitude` covers both
  DV_ORDERED value objects (→ `ext.openehr_magnitude`) and numeric primitives
  (direct cast); the analyzer records the leaf `TypeSet` and the SQL package
  picks the exact extraction. A mixed/unknown candidate set is `Raw` (guarded
  runtime dispatch — never a silent wrong-type compare). `.../value` under a
  temporal DV parent is typed `Temporal` (its `value` is an ISO-8601 String).
- **Path-split determinism.** Relies on the codec invariant that structure-typed
  children are always pruned from a node's fragment, so structure hops are a
  strict prefix. Structure classification uses the generated
  `model::is_structure_root` (lockstep with `codec::STRUCTURE_TYPES`).
- **Source scope rule.** An RM FROM class is accepted iff ≥1 concrete descendant
  is a structure root (addressable in the node store); this cleanly rejects
  demographic/EXTRACT/DV sources as `UnsupportedSourceClass`.
- **Envelope errors.** `AqlError = Feature | Analysis | Sql | Exec`; every
  rejection cites its QUERY spec section. `CurrentDateTimeInOrderBy` is defined
  but structurally unreachable from the current AST (ORDER BY admits only
  `identifiedPath`, so `now()` there is a parse-time error) — kept as a guard.
- **IR→SQL mapping (package 4, `ehrbase::aql::sql`).**
  - **Fully typed `sea-query`** — the query STRUCTURE and every column/value are
    typed nodes (`Expr::col`, `Expr::val` auto-parameterized, `SelectStatement`
    from/where/order/limit/distinct, `Expr::from(SelectStatement)` scalar
    subqueries, `Expr::not_exists`). PostgreSQL specifics use the official typed
    surface: `PgExpr::contains` (`@>`), `PgExpr::concatenate` (`||`), built-in
    `Func::{count,count_distinct,min,max,sum,avg}`. `Func::cust` is used only for
    functions sea-query doesn't model (`jsonb_path_query_first`, `to_jsonb`,
    `upper_inf`, our `openehr_magnitude`); `BinOper::Custom("#>>")` only for the
    jsonb-scalar-as-text operator (no typed variant exists). **No string-built
    SQL** (sqlx-conventions rule). `ext` functions resolve unqualified via the
    pool `search_path`.
  - **FROM = cross join + typed WHERE** (planner folds to joins). A VO-root RM
    source (or a top-level content source) opens a `node`+`vo_version`(+`audit`)
    alias group; content contained within a group shares its `vo_version` and
    interval-joins (`num BETWEEN a.num AND a.num_cap`, same `(vo_id,sys_version)`).
    EHR→VO links on `ehr_id`; a VERSION source shares the contained VO's
    `vo_version` alias. `NOT CONTAINS` = correlated `Expr::not_exists`. `OR` in
    CONTAINS is an explicit `SqlError::Unsupported` (out of the initial SQL
    envelope; not in the acceptance set).
  - **Identified-path extraction = the design's split.** Empty anchor → read the
    source node's `data` inline; non-empty anchor → a **correlated scalar
    subquery** walking the anchor chain (interval containment + `rm_type IN` +
    archetype/name/std filters per step) and extracting the fragment with
    `jsonb_path_query_first`. Subqueries yield the value or NULL, so missing paths
    compare false and never multiply rows (keeps OR/NOT/EXISTS correct). Coercion:
    `Magnitude`→`openehr_magnitude` for DV objects else `#>>`+`::numeric`;
    `Text/Raw`→`#>>`; `Boolean`→`::boolean`; `Temporal`→`#>>`+`::timestamptz`.
  - **Whole-object select** projects the anchor node's `(vo_id, sys_version, num,
    num_cap)`; `exec` reassembles the node subtree through the P10 codec
    (re-based so the anchor becomes the fragment root). `PERF(port)` one query per
    whole-object cell — single-query jsonb aggregation is P20.
  - **REST fetch/offset vs AQL LIMIT/OFFSET** — composed in `service::aql_query`:
    a REST `fetch` with an AQL `LIMIT`/`TOP`, or a REST `offset` with an AQL
    `OFFSET`, is rejected `400` (ITS-REST query Request: `fetch` "cannot be
    combined with AQL-`TOP`"); otherwise the AQL clause wins, else the REST param.
  - **`RESULT_SET`** (ITS-REST 1.0.3): `{meta:{_type,_schema_version:"1.0.3",
    _created,_executed_aql}, q, columns:[{name,path?}], rows:[[…]]}`; column name
    = `AS` alias else `#{index}`. Planner/SQL rejections → `400`, DB/assembly
    failures → `500`.

## Feature-envelope status (construct → state)

| Construct | State |
|---|---|
| SELECT paths / literals / `AS` alias | tested |
| SELECT DISTINCT | tested |
| Aggregates COUNT/COUNT(*)/COUNT(DISTINCT)/MIN/MAX/SUM/AVG | tested (COUNT/MIN/MAX) |
| FROM EHR / COMPOSITION / OBSERVATION (RM structure classes) | tested |
| CONTAINS chains (2–3 deep, EHR→COMPOSITION→OBSERVATION) | tested |
| NOT CONTAINS (anti-join) | tested |
| AND in CONTAINS | accepted (typed) |
| OR in CONTAINS | rejected (`SqlError::Unsupported`) |
| VERSION LATEST_VERSION / ALL_VERSIONS | tested |
| VERSION at-time predicate (`commit_audit/time_committed`) | accepted |
| VERSION metadata select (uid, commit_audit/time_committed, …) | tested (uid, time) |
| WHERE comparison on typed leaves (magnitude) | tested |
| WHERE EXISTS / LIKE / MATCHES / AND/OR/NOT | accepted (unit-tested lowering) |
| ORDER BY typed leaf (magnitude) | tested |
| LIMIT/OFFSET (AQL) + REST fetch/offset (+ conflict → 400) | tested |
| `$parameters` (typed binds) | tested |
| ehr_id REST scope | tested |
| archetype / at-code / name node predicates | tested |
| terminology()/demographic/branch-version/scalar-function-in-SQL | rejected (typed) |
