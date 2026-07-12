# AQL engine — spec-first audit + redesign (W-3f)

The `ehrbase` platform crate's AQL execution engine (`app/ehrbase/src/aql/`)
audited **spec-first**: the register below is built from the AQL 1.1 language
spec's own construct decomposition (`docs/specs/openehr/QUERY/docs/AQL/`,
principally `master03-syntax.adoc`), and the engine code is then mapped *onto*
each construct — never the reverse. Path semantics are cross-checked against
`docs/specs/openehr/BASE/docs/architecture_overview/master11-paths.adoc`. The
lowering-pipeline internals (typed IR, sea-query SQL, nested-set joins) are
**spec-silent** and audited under that flag, grounded on
`docs/design/aql-engine.md`.

Read-only audit. Verdicts are `conformant` / `divergent` / `missing` /
`typed-reject` (a typed reject is a *legitimate documented envelope boundary*,
recorded as such with its citation — never a silent wrong answer). Prior verdicts
in `docs/blueprint/04-query.md` are cross-referenced; where that chapter is stale
(it predates the B6 single-row-function + semantic-pass work), the **code** wins
and the delta is noted.

Pipeline (all modules in `app/ehrbase/src/aql/`):

```
AQL text ─parse(openehr-query)→ AST ─[terminology pre-pass]→ AST'
  ─analyze+lower→ QueryIr (typed, no SQL) ─sql::build→ one sea-query SELECT
  ─exec→ RESULT_SET (ITS-REST 1.0.3)
```

- `analyze.rs` (583) — path split + typing vs `openehr_rm::model`.
- `lower.rs` (585) — AST → `QueryIr`; the accept/reject envelope.
- `ir.rs` (741) — the typed IR (no SQL strings).
- `sql.rs` (1,483) — IR → one PostgreSQL SELECT (sea-query). **The split target.**
- `exec.rs` (218) — execute + `RESULT_SET` assembly + ABAC scope collection.
- `terminology.rs` (464) — the `TERMINOLOGY()`/`matches`-URI semantic pre-pass.
- `error.rs` (292) — `AqlFeatureError` / `AnalysisError` / `SqlError` / `ExecError`.

---

## 1. AQL construct register (spec → code)

### Query structure & lexical (Q-01…Q-04)

| Construct | Spec § | Code | Verdict |
|---|---|---|---|
| Clause skeleton SELECT/FROM/WHERE/ORDER BY/LIMIT, ordered | master03 §Query structure (906–918) | parser (`openehr-query`); `lower::lower` | conformant |
| Case-insensitive keywords | master03 §Reserved words (17) | lexer; `Bindings` fold case (`analyze.rs:72`) | conformant |
| Reserved words/characters | master03 §Reserved words (19–32) | lexer | conformant (fn-names lex as idents — recorded D-2/F-08-05) |

### Paths, variables, parameters, predicates (Q-05…Q-11)

| Construct | Spec § | Code | Verdict |
|---|---|---|---|
| openEHR path syntax (archetype + RM-attr paths) | master03 §openEHR path syntax (36–63); master11 §Paths | `analyze::analyze_rm_path` path split | conformant |
| Variables — unique, case-insensitive | master03 §Variables (86–95) | `lower::Planner::bind` → `DuplicateVariable`; case-fold | conformant *(uniqueness now enforced — blueprint "F-08-14 open" is stale)* |
| Parameters `$name` anywhere a value appears | master03 §Parameters (97–113) | `ir::Params`, `Bind::Param`; presence check in `plan` | conformant |
| Standard predicate `[lhs op rhs]` | master03 §Standard predicate (147–157) | `analyze::apply_standard`; path-rhs → typed reject in predicate position (`bind_from_operand`) | conformant (path-vs-path only in WHERE, not predicate RHS — documented) |
| Archetype predicate (FROM-only) | master03 §Archetype predicate (159–181) | `analyze::apply_predicate`; `archetype_predicate` (subsumption) | divergent-by-design → **G-05** |
| Node predicate — 5 prose forms | master03 §Node predicate (183–249) | `analyze::apply_node` | conformant for prose forms; OR/regex forms → **G-10** |
| Identified paths — 3 forms | master03 §Identified Paths (251–286) | `analyze::analyze_path` (root pred + anchor + fragment) | conformant |

### Operators (Q-12…Q-18)

| Construct | Spec § | Code | Verdict |
|---|---|---|---|
| Comparison `= > >= < <= !=` | master03 §Comparison operators (293–308) | `sql::binoper`; `Coercion` per leaf | conformant |
| DV_ORDERED ordered-magnitude comparison | master03 §ORDER BY (1109) + Q-33 | `coerce_value` → `ext.openehr_magnitude` for `DV_*` | conformant (full-precision); **G-14** for mixed/`Raw` sets |
| LIKE (`?`/`*`, whole-string, escapes) | master03 §LIKE (310–332) | `sql::aql_like_to_sql` | conformant |
| matches value list (OR semantics) | master03 §matches item 1 (338–365) | `Expr::Matches` → `is_in` | conformant |
| matches URI | master03 §matches item 2 (367–402) | `terminology::expand_uri` pre-pass; `lower.rs` `MatchesUri` guard | conformant *if pre-pass wired* → **G-02** |
| matches TERMINOLOGY() (+ mixed list) | master03 §matches item 3 (404–416), §TERMINOLOGY (748–767) | `terminology::expand_matches` (expand merged into list) | conformant *if pre-pass wired* → **G-02** |
| AND/OR/NOT in WHERE | master03 §Logical operators (418–458) | `IrExpr::And/Or/Not`; `sql::where_expr` | conformant |
| EXISTS (+ NOT EXISTS) | master03 §EXISTS (473–488) | `IrExpr::Exists` → `is_not_null` on the extraction subquery | conformant |

### Functions (Q-19…Q-24)

| Construct | Spec § | Code | Verdict |
|---|---|---|---|
| Aggregates COUNT/MIN/MAX/SUM/AVG, COUNT(DISTINCT/\*) | master03 §Aggregate functions (503–569) | `AggFunc`; `aggregate_expr` | conformant for COUNT/SUM/AVG; **MIN/MAX forced-Magnitude → G-15** |
| String fns LENGTH/CONTAINS/POSITION/SUBSTRING/CONCAT/CONCAT_WS | master03 §String functions (571–619) | `sql::scalar_fn_expr` | conformant *(blueprint "MISSING" is stale — executed since B6)*; string `CONTAINS()` un-lexable → **G-10**/D-2 |
| Numeric fns ABS/MOD/CEIL/FLOOR/ROUND | master03 §Numeric functions (621–662) | `sql::scalar_fn_expr` | conformant |
| Date/time fns CURRENT_DATE/TIME/DATE_TIME/NOW/CURRENT_TIMEZONE | master03 §Date and time functions (664–695) | `sql::scalar_fn_expr` (`to_char(now(),…)`) | conformant *(CURRENT_TIMEZONE now whitelisted — stale in blueprint)* |
| Function composition (nested fn args) | master03 §Functions (492–501) | `lower::lower_function`; recursive `lower_terminal` | conformant |

### FROM / containment / versioning (Q-25…Q-29)

| Construct | Spec § | Code | Verdict |
|---|---|---|---|
| Class expressions (RM class + var + predicate) | master03 §Class expressions (775–808) | `lower::lower_operand`; `model::class` | conformant; off-scope classes (demographic) → typed-reject `UnsupportedSourceClass` |
| CONTAINS chains (any depth) | master03 §Containment (958–966) | nested-set interval self-joins (`sql::emit_rm`) | conformant |
| Boolean containment: **AND** and **OR** | master03 §Containment (968–979) | AND → `ContainsTree::And`; **OR → `sql.rs:286` `Unsupported`** | **OR divergent/missing → G-01** |
| NOT CONTAINS (exclusion) | master03 §Containment (981–987), §NOT (460–471) | `emit_not_contains` (`NOT EXISTS` anti-join) | conformant for simple form; compound/nested/VERSION forms typed-reject → **G-08** |
| VERSION sources: LATEST_VERSION / ALL_VERSIONS / std predicate | grammar `versionClassExpr`/`versionPredicate`; master02 (24) — **no master03 prose (D-3)** | `VersionScope::{Latest,All,Predicate}`; `push_scope` | conformant to grammar; our own semantics PORT-NOTEd → **G-07**; branch ids → **G-06** |

### SELECT / ORDER BY / LIMIT (Q-30…Q-34)

| Construct | Spec § | Code | Verdict |
|---|---|---|---|
| SELECT columns: paths / functions / literals / bare var (whole object) | master03 §SELECT (1008–1053) | `emit_select_column`; whole-object → 4 locator cols + codec | conformant |
| DISTINCT | master03 §DISTINCT (1055–1068) | `QueryIr.distinct` → SQL `DISTINCT` | conformant |
| TOP (deprecated); TOP+LIMIT illegal | master03 §TOP (1070–1087) | `lower_limit` maps TOP→LIMIT; `TopWithLimit` reject | conformant |
| ORDER BY (multi-key, ASC/DESC, Ordered types) | master03 §ORDER BY (1094–1113) | `build_order_by`; `order_coercion` (magnitude for DV_ORDERED) | conformant |
| LIMIT/OFFSET (row_count≥1, offset≥0, after DISTINCT) | master03 §LIMIT (1115–1153) | `lower_limit` bounds check (`PagingBounds`); `build_paging` | conformant *(bounds now enforced — stale in blueprint)* |

### Literals / types / results (Q-35…Q-38)

| Construct | Spec § | Code | Verdict |
|---|---|---|---|
| Literals incl. NULL, ISO-8601 temporal typed by context | master03 §Literals + §Built-in Types (855–904) | `TypedLit`; `lower::retype_temporal`; `analyze` temporal `value`-leaf coercion | conformant; partial-precision temporal comparison → **G-04** |
| Identified expressions (unary/binary, path-vs-path, fn-lhs) | master03 §Identified expression (810–853) | `lower::lower_identified` | conformant |
| Result structure `Array<Array<Any>>`, NULL for missing | master04 (whole; normativity delegated, D-5) | `exec::QueryResult` → ITS-REST 1.0.3 RESULT_SET | conformant |
| Result granularity (primitive → top-level object) | master02 feature 2 | whole-object codec reassembly (`exec::reassemble_subtree`) | conformant (per-cell query = **G-11** PERF) |

---

## 2. Own-design internals (spec-silent — "no openEHR spec governs this")

openEHR defines the *language*, not its execution. These are our design
(`docs/design/aql-engine.md`), audited for internal soundness only:

- **The path split** (`analyze.rs`): every identified path splits deterministically
  into structure-node hops (own `node` rows, resolved via `model::is_structure_root`,
  mirrored from `storage::codec::STRUCTURE_TYPES`) + a residual `data`-JSONB
  fragment jsonpath. Sound: the codec prunes all structure children, so no
  structure root can appear below the first non-structure step.
- **Typed IR** (`ir.rs`): relational-algebra-flavoured Rust enums, no SQL strings —
  the discipline the rules mandate. `Coercion` (Magnitude/Text/Temporal/Boolean/Raw)
  is the single typing decision carried from analysis to SQL.
- **SQL lowering** (`sql.rs`): one SELECT; FROM containment = cross-join + typed
  conditions the planner folds to joins; CONTAINS = nested-set interval self-join
  (`num BETWEEN a.num AND a.num_cap` within `(vo_id, sys_version)`); leaf extraction
  = `jsonb_path_query_first` (inline for empty anchor, correlated scalar subquery for
  anchor chains); DV_ORDERED via `ext.openehr_magnitude`. Fully typed sea-query —
  the only string escapes are the sanctioned `Func::cust` set + `#>> '{}'`.
- **Scope gates** (`sql.rs`): the SM `I_QUERY_SERVICE` population gate
  (`is_queryable`, cited to `SM/docs/UML/classes/i_query_service.adoc`) and the
  multi-`ehr_id` scope are spec-grounded; the ABAC `subject_scope` gate cites the
  internal `docs/enterprise/access-control.md` (Stage-2 enterprise, our own
  extension — flagged, **G-13**).

---

## 3. G-row register

| id | citation / flag | severity | disposition |
|---|---|---|---|
| **G-01** | OR-CONTAINS **normative** — master03 §Containment (968–979). `sql.rs:286` returns `SqlError::Unsupported("OR in the CONTAINS/FROM tree")`; `lower`/IR already build `ContainsTree::Or`, so only SQL lowering is missing. (Blueprint says B6 closed it; code contradicts — the reject stands.) | HIGH | **fix-in-rewrite** |
| **G-02** | matches URI + `TERMINOLOGY()` in matches — master03 §matches (367–416), §TERMINOLOGY. Handled by `terminology::expand_matches` **before** planning; `lower.rs` `MatchesUri`/`MatchesTerminology`/`TerminologyFunction` are defensive guards for a bypassed pre-pass. Must verify the query service calls `expand_matches` on every ad-hoc + stored path. | HIGH | **re-verify** (already-correct if wired; the seam is §5) |
| **G-03** | Dead reject `AqlFeatureError::CurrentDateTimeInOrderBy` + `ScalarFn::is_temporal_now` (`ir.rs:652`) — never returned: ORDER BY's AST admits only identified paths, not function calls, so a now-family fn can't reach ORDER BY. | LOW | **delete** (or convert to a real guard if fn-ORDER BY is added) |
| **G-04** | Partial-precision temporal comparison — master03 §Built-in Types/Dates and Times. `coerce_value` casts the ISO-8601 leaf text to `timestamptz` (`sql.rs:1214`); partial values (`2019`, `12:00`) are a real gap. Existing PORT NOTE `sql.rs:1211`. | MED | **PORT NOTE** (keep; widen only if ECC demands) |
| **G-05** | Archetype-predicate **subsumption** vs literal string equality — master03 §Archetype predicate says `archetype_node_id = '<literal>'`; we implement BASE `architecture_overview` master10 §Design-time Relationships + AM master07 §Querying subsumption instead (correct: a parent query must match specialisation children). PORT NOTE `sql.rs:1307`; ADL2 template-lineage deferred. | MED | **PORT NOTE** (keep; re-verify at the ADL2 phase) |
| **G-06** | Branch (non-trunk) version addressing rejected — trunk-only. `analyze::is_branch_version_id` → `BranchVersionAddressing`. D-3 (no version prose in master03). | LOW | **PORT NOTE** (keep until version branching lands, SM-5 tail) |
| **G-07** | VERSION-source semantics are grammar-only (D-3, `AqlParser.g4`; no master03 section). Our `VersionScope` + metadata field set (`ir.rs`) is a designed reading. | INFO | **PORT NOTE** (keep) |
| **G-08** | NOT CONTAINS restricted to a single simple content operand — compound (AND/OR) operand, further-nested CONTAINS, and VERSION NOT CONTAINS are `Unsupported` (`sql.rs:298,421,427`). master03 §Containment allows richer exclusion trees. | MED | **fix-in-rewrite** (generalise the anti-join) *or* PORT NOTE the boundary |
| **G-09** | EHR `system_id` / whole-EHR standard predicate rejected (`sql.rs:632`); only `ehr_id`/`time_created` predicates lower. Narrow scope boundary. | LOW | **PORT NOTE** (or add the two fields in the rewrite) |
| **G-10** | Node-predicate `OR` (`OrNodePredicate`) and `MATCHES CONTAINED_REGEX` (`RegexNodePredicate`) rejected (`analyze.rs`). Regex has no prose semantics (D-3); node-`OR` is a grammar extra beyond the 5 prose forms. | LOW | **PORT NOTE** (keep regex; decide node-`OR` in rewrite) |
| **G-11** | Whole-object cells reassembled by a per-cell follow-up query (`exec.rs:160`, `reassemble_subtree`). PERF only. | LOW | **PERF(port) → P20** (single-query jsonb aggregation) |
| **G-12** | `Coercion::Raw` (mixed/unknown leaf type set) documented in `ir.rs:405` as "guarded runtime dispatch (numeric for numbers, text otherwise)", but `coerce_value`/`coerce_rhs` treat `Raw` as **plain text** (`sql.rs:1195,1215`). Numbers in a mixed leaf compare/sort lexically → wrong ordering. Doc and impl disagree. | MED | **fix-in-rewrite** (implement the dispatch, or correct the doc + IR comment) |
| **G-13** | ABAC `subject_scope` gate (`sql.rs`, `SqlCtx.subject_scope`) cites the internal `docs/enterprise/access-control.md` — a Stage-2 enterprise concern present in Stage-1 code. Own-design extension; flag, don't cite an ADR. | INFO | **PORT NOTE** (flag as our own extension; the population/`ehr_id` gates are spec-grounded and stay) |

Counts by disposition: **fix-in-rewrite 3** (G-01, G-08, G-12) · **re-verify 1**
(G-02) · **delete 1** (G-03) · **PORT NOTE 7** (G-04, G-05, G-06, G-07, G-09,
G-10, G-13) · **PERF→P20 1** (G-11). No `quarantine`. Severity: HIGH 2, MED 4,
LOW 5, INFO 2. The two spec-normative defects a compliant CDR must close are
**G-01 (OR-CONTAINS)** and **G-12 (Raw numeric comparison)**; **G-02** is a wiring
verification, not new code.

---

## 4. Target design — re-grounded `app/ehrbase/src/aql/`

The pipeline stages stay (`analyze → ir → lower → sql → exec`) and their module
boundaries are already spec-clean. The **only structural change** is splitting
`sql.rs` (1,483) along the AQL construct decomposition so no file exceeds ~700
lines; the split follows the spec's own clause structure so each file maps to a
recognisable part of the language.

```
app/ehrbase/src/aql/
├── mod.rs          plan() entry + param collection (unchanged)
├── analyze.rs      path split + typing vs openehr_rm::model (unchanged)
├── lower.rs        AST → QueryIr; the accept/reject envelope (unchanged)
├── ir.rs           the typed IR (unchanged; fix G-12 doc)
├── error.rs        error families (drop the dead G-03 variant)
├── terminology.rs  TERMINOLOGY()/matches-URI semantic pre-pass (unchanged)
├── exec.rs         execute + RESULT_SET + ABAC scope (G-11 PERF marker stays)
└── sql/            ← sql.rs split
    ├── mod.rs      SqlCtx, PreparedQuery, ColumnSpec, CellKind, build(),
    │               build_scope(), Builder struct + new()            (~260)
    ├── from.rs     FROM/containment: walk, contained_edge, emit_rm,
    │               emit_not_contains (+ G-01 OR, + G-08 general anti-join),
    │               VoGroup, scope gates (ehr_scope, population_gate,
    │               queryable_ehr_subquery, ensure_ehr_status_root,
    │               ensure_audit, push_scope, push_ehr_predicate)    (~470)
    ├── select.rs   build_select, emit_select_column, emit_whole_object,
    │               whole_object_alias, aggregate_expr (+ G-15 MIN/MAX
    │               coercion fix)                                    (~240)
    ├── predicate.rs build_where, where_expr, scalar_fn_expr,
    │               operand_value, {archetype,name,std}_cond, matches/like (~360)
    ├── value.rs    value_expr, data_leaf_expr (the path split),
    │               coerce_value/coerce_rhs (+ G-12 Raw dispatch),
    │               version_field_expr, ehr_field_expr, fragment_jsonpath (~300)
    └── expr.rs     typed sea-query building blocks: col, call, to_jsonb,
                    jsonb_path, as_text, cast, binoper, literal_value,
                    archetype_predicate (G-05), aql_like_to_sql, type_cond,
                    order_coercion, leaf_path_string, test helpers      (~320)
```

Every file ≤ ~470 lines. `Builder` stays one struct (its `impl` blocks fan across
`from/select/predicate/value` via `impl Builder` in each submodule — idiomatic,
no field churn). No behaviour change from the split itself; the G-fixes ride the
files they land in.

Rewrite work items, in dependency order:
1. **G-01** OR-CONTAINS in `from.rs`: lower `ContainsTree::Or` to a disjunctive
   `EXISTS` pair (or `UNION` of interval-join branches) over the shared anchor;
   flip the envelope test + add an e2e proof.
2. **G-12** `Raw` dispatch in `value.rs`: `CASE jsonb_typeof(...)` → numeric vs
   text comparison, or reduce the analyzer's `Raw` surface by resolving more
   candidate sets (preferred — fewer runtime branches).
3. **G-15** MIN/MAX in `select.rs`: use the argument leaf's own `Coercion`, not a
   forced `Magnitude`, so `MIN/MAX` over String/Date/Time leaves (master03 §MIN/MAX)
   are correct.
4. **G-08** generalise the anti-join; **G-03** delete the dead reject; **G-09/G-10**
   decide fix-vs-PORT-NOTE per row.

---

## 5. Seams — `TODO(w3f-integrate)` candidates

The engine is deliberately storage- and service-coupled at three seams; the
redesign marks each so the integration owner can wire them explicitly:

- **Node-table contract** (`storage::{NodeRow, reassemble}`, `db::iden::{Node,
  VoVersion,Ehr,Audit}`): the SQL builder hard-codes the node/vo_version/ehr/audit
  column vocabulary and the nested-set/`sys_period`/`branch_number` semantics.
  `is_structure_root` must stay in lockstep with `codec::STRUCTURE_TYPES` (the
  `emit-rm-model` generator enforces it). → `TODO(w3f-integrate)`: assert the
  column set against `0001_baseline.sql` at the seam.
- **Query-service execution seam** (`exec::{execute, collect_scope}` + `SqlCtx`):
  the service must (a) run the `terminology::expand_matches` pre-pass **before**
  `plan` (this is the G-02 verification — the guards in `lower.rs` prove nothing
  is silently mis-handled, but the happy path depends on the call), (b) pass the
  resolved `ehr_ids`/`subject_scope`/paging into `SqlCtx`, and (c) map
  `AqlFeatureError`/`AnalysisError` → 400/422 and `ExecError` → 500 at the REST
  boundary. → `TODO(w3f-integrate)`: single documented call order in the service.
- **Terminology service** (`terminology::TerminologyExpander` trait): the
  `TERMINOLOGY()` family (expand/validate/subsumes/URI) resolves through the SM
  `I_TERMINOLOGY_SERVICE` provider (in-process `openehr-term` bundle default,
  remote FHIR TS when configured). → `TODO(w3f-integrate)`: bind the concrete
  expander in the service, not the engine.

---

## 6. PORT-NOTE residue (keep / re-verify / drop)

| PORT NOTE | location | decision |
|---|---|---|
| Temporal comparison casts ISO-8601 → `timestamptz`; partial precision a gap | `sql.rs:1211` (→ `value.rs`) | **keep** (real gap, G-04) |
| Archetype-predicate subsumption instead of literal equality; ADL2 lineage deferred | `sql.rs:1307` (→ `expr.rs`) | **keep**, **re-verify at ADL2** (G-05) |
| Per-cell whole-object reassembly query | `exec.rs:160` | **keep** as `PERF(port)` → P20 (G-11) |
| Branch version addressing trunk-only | `error.rs:147` | **keep** until version branching (G-06) |
| VERSION scope / trunk-only branching designed reading (D-3) | `ir.rs` (VersionScope) | **keep** (G-07) |
| `Coercion::Raw` "numeric-or-text dispatch" comment | `ir.rs:405` | **re-verify → correct** — the impl does text-only (G-12); the note must match the fixed behaviour |

No PORT NOTE is dropped outright; G-12's note is corrected to match the fix, and
G-03's dead reject variant is deleted (it carries no PORT NOTE).
