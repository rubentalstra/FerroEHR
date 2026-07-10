# ADR-008: Greenfield PG18-native storage + AQL architecture; openEHR conformance replaces EHRbase parity

- **Status:** accepted. *(2026-07-10: the conformance runner (`scripts/conformance.sh`) is built and green — 341 executed · 315 passed · 0 failed, CORE/STANDARD PASS; the "full at P19" references below are the original roadmap, since executed as B1–B6.)*
- **Date:** 2026-07-05
- **Supersedes (in part):** ADR-006 §3/§4 ("follow EHRbase's algorithm as the
  reference", "the real EHRbase v2 schema is reused verbatim") and the
  EHRbase-diff parity harness as the acceptance instrument (P19). ADR-004/005
  (generated spec + ITS layers) are unaffected and remain the foundation.
  ADR-007's *shipped schema content* is replaced; its infrastructure
  (sqlx migrators, testcontainers gates, baseline+equality-test method)
  is retained.

## Context

During P10 the project owner pivoted: the goal is the **best possible modern
Rust CDR**, not a faithful re-creation of EHRbase's internals. Two explicit
decisions were taken:

1. **Full greenfield internals, including our own storage design.**
2. **The compatibility target is the openEHR specifications** (ITS-REST
   1.0.3 contract — already generated into `openehr-its` — and the AQL 1.1
   spec), verified by the official openEHR **Conformance (CNF)** framework
   and its Platform Conformance Test Schedule. EHRbase is demoted to
   reference/inspiration; bug-for-bug parity with it is no longer a goal.

Two research passes ground the design (2026-07-05, docs-verified):

- **The AQL requirements contract** (from our `openehr-query` AST + the AQL
  spec + EHRbase's feature envelope as a market reference): CONTAINS is
  descendant-search by RM type/archetype id correlated per versioned object
  and across multiple bound variables; identified paths need typed leaf
  extraction with openEHR *ordered-magnitude* comparison semantics for
  `DV_ORDERED`; ORDER BY/aggregates over those leaves; strict versioning
  (`LATEST_VERSION`/`ALL_VERSIONS`, audit/contribution); array-valued
  attributes with stable order; and the planner needs a static RM
  attribute/type model to validate and type paths.
- **PostgreSQL 18 capability assessment** (official docs):
  - **TOAST physics:** `jsonb` has no partial detoast — extracting one small
    leaf from a large stored composition detoasts/decompresses the whole
    value, on every access. One-big-JSONB-per-composition is therefore
    structurally wrong for AQL workloads (many rows × few leaves).
  - **GIN:** `$.**` recursive descent *is* indexable, but only under the
    default `jsonb_ops` class and only with an `== constant` anchor; GIN
    serves **no range or ordering operators at all**.
  - Range/ordering over JSON leaves needs **btree expression indexes over
    immutable extractions** (or STORED generated columns — PG18's new
    VIRTUAL generated columns are **not indexable**).
  - `JSON_TABLE`/`JSON_VALUE`/jsonpath item methods (`.decimal()`,
    `.datetime()`, …) are runtime tools (no index participation) — good for
    unnesting and typed coercion of *small* values.
  - PG18 gives us `uuidv7()`, temporal `PRIMARY KEY … WITHOUT OVERLAPS`,
    `RETURNING OLD/NEW`, and btree skip scan.

Conclusion from the physics: **decomposed node storage** (a row per RM
structure node with a small JSONB fragment and a nested-set index) is the
correct architecture on PostgreSQL — independently of EHRbase having chosen
it too. Everything above that substrate is designed fresh.

## Decision

### 1. Mission and acceptance

The product is an **openEHR-spec-conformant CDR**: the generated ITS-REST
1.0.3 contract is the API; AQL 1.1 is the query language; acceptance is the
**openEHR CNF conformance test schedule** (plus our own corpus-driven
integration suites). The EHRbase-diff parity harness and its
`USE_REFERENCE_EHRBASE` negative gate are retired. Where the spec
underdetermines behaviour, we decide idiomatically and document with
`// PORT NOTE:`-style records (ADR-003 discipline continues), consulting
EHRbase and other CDRs as *prior art*, not as an oracle.

### 2. Storage: one canonical node table + one temporal version table

Designed fresh for PG 18; schema authored via `sqlx migrate add` (replacing
the EHRbase baseline from ADR-007 — nothing is deployed, so `0001` is
re-authored rather than appended to).

- **`node` — one unified table for all versioned-object content**
  (COMPOSITION, EHR_STATUS, FOLDER trees; one design instead of EHRbase's
  three parallel table families). Row per RM structure node:
  - `vo_id uuid` (uuidv7), `num int`, `num_cap int`, `parent_num int`,
    `citem_num int` — the **nested-set interval index**: `CONTAINS` is
    `d.num BETWEEN a.num AND a.num_cap` within a `vo_id`, a classic
    integer-range join (Celko nested sets), not a JSON walk.
  - Promoted columns for the hot predicates: `rm_type text` (full RM type
    name — **no two-letter alias compaction**), `archetype text` (concept),
    `name text`, `path text COLLATE "C"` (byte-ordered materialized path;
    same collation trick, our own readable encoding), `ehr_id uuid`.
  - `data jsonb` — the node's **canonical openEHR JSON fragment, verbatim**
    (children pruned). No key aliasing, no synthetic injected fields: what
    is stored is exactly the `openehr-its` canonical encoding, so
    storage↔API needs zero translation, jsonpath filters use real attribute
    names, and debugging reads like the spec. Fragments are small (that is
    the point of decomposition), so the alias compaction EHRbase inherited
    is dropped as legacy micro-optimization; lz4 TOAST compression covers
    the residue and we benchmark at the storage spike.
  - Composite btree `(vo_id, num)` PK + supporting indexes; PG18 skip scan
    reduces the index count.
- **`vo_version` — one temporal table instead of current+`_history` pairs.**
  `(vo_id, sys_version)` rows with `sys_period tstzrange` and a temporal
  `PRIMARY KEY (vo_id, sys_period WITHOUT OVERLAPS)`; the current version is
  `upper_inf(sys_period)` (partial index). `LATEST_VERSION` = the partial
  index; **`ALL_VERSIONS` = the same table with no filter — supported,
  which EHRbase never managed** (it rejects `ALL_VERSIONS`). Version writes
  use `RETURNING OLD/NEW` for one-statement audit capture. `kind` column
  discriminates COMPOSITION/EHR_STATUS/FOLDER. Fallback if first-release
  temporal constraints disappoint: plain unique `(vo_id, sys_version)` +
  application invariant (the spike validates this before commitment).
- **`ehr`, `contribution`, `audit`, `template_store`, `stored_query`,
  `item_tag`** — supporting tables, our own naming/design, every write
  emitting contribution + audit rows in the same transaction (an openEHR
  requirement, not an EHRbase-ism).
- **`ext` helper functions, ours:** a small set of `IMMUTABLE` SQL/plpgsql
  functions (e.g. `openehr_magnitude(jsonb) → numeric` implementing the
  spec's DV_ORDERED magnitude semantics, incl. nominal duration lengths).
  Being immutable they work in **btree expression indexes** for measured
  hot paths — replacing EHRbase's stored synthetic `_magnitude` field and
  its jsonb aggregate zoo with query-time typed extraction (jsonpath item
  methods + our functions) that keeps stored data canonical.

### 3. AQL engine: our own typed IR over a BMM-generated RM model

Parser/AST is done (`openehr-query`). The engine is designed as:

- **Path analysis against a generated RM attribute model.** The planner
  needs "which types can attribute X hold, is it multi-valued, which
  concrete types descend from ENTRY" — EHRbase gets this from Java runtime
  reflection (archie). We **generate it from the BMM** (extending
  `openehr-codegen` with an `emit-rm-model` target or exposing it from
  `openehr-lang`): a compile-time, spec-pinned static model. Strictly
  better: no reflection, no hand-maintained tables, regenerates on spec
  bump.
- **A typed query IR of our own design** (Rust enums, not a port of
  EHRbase's ASL class zoo), lowered to SQL via `sea-query`: interval joins
  on the node table for CONTAINS chains, `jsonb_path_query_first` + item
  methods for leaf extraction, `openehr_magnitude` for DV_ORDERED
  comparison/ORDER BY, `JSON_TABLE` for array unnesting where it wins,
  GIN `jsonb_ops` + `$.**` equality anchors where a document-level
  pre-filter helps.
- **Feature envelope:** start from the AQL spec; where the spec is wider
  than practical, our accept/reject set must be a superset of EHRbase's
  (documented per feature) — e.g. `ALL_VERSIONS` is in scope from the
  start; `NOT CONTAINS` follows the spec grammar with an explicit decision
  recorded when implemented or deferred.

### 4. What is retained unchanged

The generated openEHR foundation (ADR-004/005: spec crates, canonical
JSON/XML, ITS-REST contract, AQL parser), the P09 infrastructure (pool,
settings, two-schema sqlx migrators, testcontainers harness — pointed at the
new schema), the serving stack (axum + generated traits), and all workspace
discipline (clippy/tests-per-phase, official CLIs, no hand-rolling what
crates provide).

### 5. The EHRbase Java reference is removed from the tree

With behaviour-parity retired, the 400+ in-tree `.java` files stop being a
build reference and become dead weight (and a temptation to port). They are
**deleted** (git history and the upstream repo preserve them; the
`protect_java` hook and related CLAUDE.md rules are updated accordingly).
The `reference/v1` git ref stays for the Stage-2 enterprise archaeology.
The archived `claude/phase-10-rm-db-format` branch keeps the abandoned port.

## Consequences

- **Roadmap rewrite:** P10 becomes *Storage foundation* (spike + schema +
  node codec: canonical decompose/reassemble against the new tables);
  P16's AQL phase is re-scoped to the IR + BMM-model design above; P19
  becomes *openEHR conformance* (CNF schedule + corpus suites); P99 cleanup
  shrinks (Java already gone). `docs/plans/*`, `PROGRESS.md`,
  `current-phase.md`, `architecture.md`, `CLAUDE.md`, and the `.claude`
  rules/hooks are updated in this change.
- **A storage spike precedes commitment** (first task of the new P10):
  decompose the 48-composition corpus into the candidate schema in a
  testcontainer, run representative CONTAINS/extract/order queries, and
  validate the temporal-constraint versioning model — benchmarks decide the
  open micro-choices (alias-free fragment size, index set, temporal PK vs
  fallback).
- **Honestly harder:** we own every algorithm now. EHRbase's engine is
  battle-tested against years of AQL corner cases; ours will be validated
  by the conformance suite + corpus + property tests instead. Divergences
  are found by tests, not by diffing a reference server. The CNF schedule
  becomes load-bearing and must be wired early (P12-era smoke conformance,
  full at P19).
- **Better, concretely:** canonical (readable, translation-free) stored
  data; one node table + one temporal version table instead of three table
  families × current/history pairs; `ALL_VERSIONS` supported; a generated
  (not reflected) RM model; magnitude as indexed-on-demand function instead
  of synthetic stored fields; PG18-native keys/constraints/audit capture.

## Alternatives considered

- **Single JSONB document per composition** (+ GIN, JSON_TABLE, expression
  indexes). Rejected on verified physics: whole-document detoast per leaf
  access; GIN cannot serve range/ordering; arbitrary-path expression
  indexes cannot be pre-created; multi-variable CONTAINS binding degrades
  to unindexed runtime tree walks.
- **Keep reusing the EHRbase v2 schema** (ADR-006/007 path). Rejected by
  the owner: carries legacy constraints (alias opacity, three table
  families, current/history duplication, no `ALL_VERSIONS`) into a
  greenfield product whose compatibility target no longer requires them.
- **Hybrid: EHRbase schema + our engine on top.** Rejected: pays the legacy
  costs without the compatibility payoff.
- **Non-relational/graph stores.** Out of scope: PostgreSQL is a pinned
  platform decision (VERSIONS.md) with the right JSONB + relational blend.
