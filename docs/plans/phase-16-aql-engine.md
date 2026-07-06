# Phase 16 — AQL engine (our typed IR over the node model, ADR-008)

> Re-scoped 2026-07-05 by ADR-008: our own design (no ASL port); EHRbase's
> engine is prior art only.

- Status: not-started (Stage-1 app build, step 8 of 13)
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

- [ ] P10 (node model), P14 (WebTemplate), RM attribute model generated

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
- [ ] IR→SQL via sea-query (interval joins, typed extraction, magnitude,
      unnesting; LATEST/ALL_VERSIONS)
- [ ] Execute + `RESULT_SET` assembly; QUERY endpoints live
- [ ] AQL corpus end-to-end tests + the feature-envelope matrix

## Exit criteria

- [ ] The AQL conformance corpus executes correctly over real data
      (testcontainers), including version addressing
- [ ] Feature envelope documented; every rejection is an explicit typed error
- [ ] Compiles + clippy-clean + nextest green

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
- **Envelope errors.** `AqlError = Feature(AqlFeatureError) | Analysis(AnalysisError)`;
  no Sql/Exec variants yet (next package). Every rejection cites its QUERY spec
  section. `CurrentDateTimeInOrderBy` is defined but structurally unreachable
  from the current AST (ORDER BY admits only `identifiedPath`, so `now()` there
  is a parse-time error) — kept as a guard.
