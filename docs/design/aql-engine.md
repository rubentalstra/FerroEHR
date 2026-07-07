# AQL engine design (P16, ADR-008)

The typed AQL 1.1 execution pipeline over the greenfield node store. Our own
design (ADR-008 §3): EHRbase's ASL is prior art, not a template. The vendored
QUERY 1.1 spec (`docs/specs/openehr/QUERY/`) is the semantic authority; the
feature envelope below documents every accept/reject.

## Pipeline

```
AQL text ──openehr-query──▶ AST
        ──analyze──▶ TypedQuery      (path typing vs openehr_rm::model)
        ──lower────▶ QueryIr         (typed relational IR, Rust enums)
        ──sql──────▶ sea_query stmt  (one SELECT over node/vo_version/ehr)
        ──exec─────▶ RESULT_SET      (ITS-REST 1.0.3)
```

Modules (all in `crates/ehrbase/src/aql/`): `analyze`, `ir`, `lower`, `sql`,
`exec`, `error`. The RM attribute model is **generated** (`emit-rm-model` →
`openehr_rm::model`): attribute→declared type + container + mandatory,
ancestors/descendants, abstract flag, `is_structure_root` (matches the node
codec's decompose rule). No reflection, no hand tables.

## Core insight: the path split

The node store decomposes structure (LOCATABLE-rooted) nodes into rows; leaf
content lives verbatim in each row's `data` JSONB fragment. Every identified
path therefore splits deterministically at analysis time:

```
c/content[openEHR-EHR-OBSERVATION.bp.v1]/data[at0001]/events[at0006]/data/items[at0004]/value/magnitude
└──────────────── node-row descent (structure hops) ────────────────┘└─ fragment jsonpath ─┘
```

- **Structure hops** (each step whose resolved type is a structure root) walk
  node rows: resolved with nested-set containment + the promoted columns
  (`archetype`, `name`, `rm_type`) and/or the byte-ordered `path` column.
- **Fragment suffix** (first non-structure step onward) compiles to one SQL/JSON
  path: `jsonb_path_query_first(n.data, '$.value.magnitude')`.

`analyze` computes, per path: the deepest node-row anchor, the fragment
jsonpath, the candidate leaf type set (from the model, abstract slots expanded
to concrete descendants), and multiplicity (any `List`/`Set` step ⇒
multi-valued). Ambiguous leaf types get runtime-typed extraction (below).

## Typed IR (shape)

Rust enums/structs, relational-algebra flavored — no stringly SQL until `sql`:

- `Source::VersionedObject { var, kind: Option<VoKind>, rm_type: TypeSet, archetype: Option<ArchetypeId>, name: Option<NameConstraint> }`
  — one per CONTAINS class operand; `Source::Ehr { var, predicates }` for the
  EHR operand; `Source::Version { var, scope }` wrapping a VO source.
- `ContainsTree` — the FROM containment tree: `Node { source, children: Vec<(Link, ContainsTree)> }`,
  `Link::Contains | Link::NotContains`, boolean `And/Or` over subtrees.
  Lowered to nested-set interval self-joins (`d.num BETWEEN a.num AND a.num_cap`,
  same `(vo_id, sys_version)`); `NOT CONTAINS` = anti-join (`NOT EXISTS`);
  cross-VO containment (EHR contains COMPOSITION x, y) = join via `ehr_id`.
- `VersionScope::Latest | All | AtTime(param) | Predicate(...)` — `Latest`
  rides the `upper_inf(sys_period)` partial index; `All` is the same table
  unfiltered (in scope from day one, ADR-008).
- `Expr` — typed predicate/projection tree: `Leaf(PathBinding)`,
  `Literal(TypedLit)`, `Param(name)`, `Cmp { op, coercion }`, `Exists`,
  `Like`, `Matches`, `And/Or/Not`, `Agg { func, distinct }`,
  `Func(WhiteListed)`.
- `Coercion::Magnitude | Text | Temporal | Boolean | Raw` — decided by the
  analyzer from the candidate leaf types: DV_ORDERED comparisons/ORDER BY go
  through `ext.openehr_magnitude(jsonb)` (the IMMUTABLE helper, indexable);
  strings via `jsonb_path_query_first(...) #>> '{}'`; temporals via jsonpath
  item methods (`.datetime()`); mixed/unknown candidate sets fall back to a
  guarded runtime dispatch (magnitude for numbers, text otherwise) — never a
  silent wrong-type comparison.
- `QueryIr { sources, contains, where_, select, order_by, distinct, limit, offset }`.

## SQL mapping (sea-query, PG18)

| IR construct | SQL |
|---|---|
| VO source (latest) | `node n JOIN vo_version v USING (vo_id, sys_version)` + `upper_inf(v.sys_period)` |
| VO source (ALL_VERSIONS) | same join, no period filter |
| rm_type / archetype / name constraint | promoted columns (`n.rm_type = ANY(...)`, `n.archetype = ...`, `n.name = ...`) |
| CONTAINS edge | self-join `d` on `d.vo_id = a.vo_id AND d.sys_version = a.sys_version AND d.num BETWEEN a.num AND a.num_cap` |
| NOT CONTAINS | `NOT EXISTS (…interval join…)` |
| EHR var | `ehr e JOIN … ON n.ehr_id = e.id` |
| leaf extraction | `jsonb_path_query_first(anchor.data, $jsonpath)` |
| ordered comparison / ORDER BY | `ext.openehr_magnitude(extract)` |
| whole-object select | project `(vo_id, sys_version, num, num_cap)`; reassemble via `storage::codec` post-fetch |
| VERSION paths (commit_audit, uid…) | join `vo_version`/`audit`/`contribution` columns |
| DISTINCT / LIMIT / OFFSET / aggregates | native SQL |
| ehr_id REST parameter | equality on `n.ehr_id` (indexed) |

One statement per query. Multi-valued path predicates use `EXISTS` over
`jsonb_path_exists`/`JSON_TABLE` unnesting; a GIN `$.**` equality anchor is
added as a pre-filter only when the predicate is a constant equality
(planner-chosen, `jsonb_ops`).

## RESULT_SET

ITS-REST 1.0.3 shape: `columns` from SELECT aliases/paths, `rows` as canonical
JSON values. Whole-object cells reassemble the node subtree through the P10
codec (correct first; `PERF(port):` single-query JSONB aggregation is P20).
`fetch`/`offset` REST params compose with AQL LIMIT/OFFSET per the spec's
precedence rules (reject conflicting combinations explicitly).

## Feature envelope (initial; every reject = typed `AqlFeatureError`)

**Accepted:** SELECT (paths, literals, aliases, DISTINCT, aggregates
COUNT/MIN/MAX/SUM/AVG, COUNT(DISTINCT)); TOP (mapped to LIMIT, reject when
combined with LIMIT); FROM EHR/VERSIONED_OBJECT/VERSION(LATEST_VERSION |
ALL_VERSIONS)/all RM classes with archetype+name predicates; CONTAINS trees
with AND/OR/NOT (incl. NOT CONTAINS as anti-join); WHERE comparisons on typed
leaves, EXISTS, LIKE (AQL wildcards → SQL), MATCHES value lists, IN-range via
`>=`/`<=`; ORDER BY typed leaves; LIMIT/OFFSET; `$parameters` (typed binds);
`ehr_id`/`offset`/`fetch` REST params.
**Rejected (explicit, typed):** `terminology()` server-side expansion (needs a
terminology service — later phase), MATCHES against URI/terminology operands,
demographic sources, `current_date_time()`-family in ORDER BY (accepted in
WHERE as bind-time constants), branch version addressing (trunk-only, per the
storage PORT NOTE).

The envelope is asserted by `tests/aql_envelope.rs` — one accepted + one
rejected proof per construct — and must remain a superset of EHRbase's
documented envelope (ADR-008 §3).

## Status (P16 — construct → accepted / rejected / tested)

Pipeline: `analyze` + `ir` + `lower` (32 unit tests) → `sql` (fully typed
sea-query) → `exec` (RESULT_SET) → the `QueryService` seam + `/query/*`
endpoints. e2e in `crates/ehrbase/tests/service_aql.rs` (+ HTTP in
`service_query.rs`), PG18 testcontainers.

| Construct | State |
|---|---|
| SELECT paths / literals / `AS` alias / DISTINCT | tested |
| Aggregates COUNT / COUNT(*) / COUNT(DISTINCT) / MIN/MAX/SUM/AVG | tested (COUNT/MIN/MAX) |
| FROM EHR / RM structure classes; CONTAINS chains (2–3 deep) | tested |
| NOT CONTAINS (anti-join) · AND-CONTAINS | tested · accepted |
| OR-CONTAINS | rejected (`SqlError::Unsupported`, SQL envelope) |
| VERSION LATEST_VERSION / ALL_VERSIONS / at-time | tested / tested / accepted |
| VERSION metadata select (uid, commit_audit/time_committed, …) | tested (uid, time) |
| WHERE compare (magnitude) / EXISTS / LIKE / MATCHES / AND-OR-NOT | tested / accepted |
| ORDER BY typed leaf · LIMIT/OFFSET · REST fetch/offset (+conflict 400) | tested |
| `$parameters` · ehr_id scope · archetype/at-code/name predicates | tested |
| terminology() · demographic · branch-version · scalar-fn-in-SQL | rejected (typed) |

SQL notes: FROM = typed cross-join + `Expr` conditions (planner folds to joins);
CONTAINS = nested-set interval self-joins; anchor descent = correlated scalar
subqueries with promoted-column filters; `PgExpr::contains`/`concatenate` +
built-in `Func` aggregates; `Func::cust` only for
`jsonb_path_query_first`/`to_jsonb`/`upper_inf`/`openehr_magnitude`;
`BinOper::Custom("#>>")` only for jsonb-scalar-as-text. Whole-object cells
reassemble via the P10 codec (`PERF(port)` single-query aggregation → P20).

## Testing

- Unit: analyzer path-split table tests; IR lowering per construct.
- e2e (testcontainers PG18): the AQL corpus + CNF QUERY suite cases
  (`docs/specs/openehr/CNF/`) over seeded compositions, incl. LATEST_VERSION vs
  ALL_VERSIONS over a mutated composition, NOT CONTAINS, magnitude ordering,
  parameter binding.
- Property: parse→lower→SQL never panics on corpus-fuzzed inputs.
