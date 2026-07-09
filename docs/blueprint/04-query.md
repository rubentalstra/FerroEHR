# Blueprint 4 — QUERY (AQL 1.1)

Scope: the full AQL 1.1 feature envelope a compliant CDR must support, from the
vendored spec at `docs/specs/openehr/QUERY/docs/` (AQL syntax spec
`AQL/master02-overview.adoc`, `AQL/master03-syntax.adoc`,
`AQL/master04-result_structure.adoc`, `AQL/master06-writing_AQL.adoc`,
`AQL/master07-grammar.adoc`) plus the normative ANTLR4 grammar it includes
(vendored at `crates/openehr-query/vendor/grammar/{AqlLexer.g4,AqlParser.g4}`).
The REST wire for executing AQL (ad-hoc + stored queries, RESULT_SET JSON) is
ITS-REST territory (Blueprint chapter for ITS-REST); this chapter covers the
*language* a compliant engine must accept and evaluate. Amendment record
(`AQL/master00-amendment_record.adoc`) confirms Release 1.1.0 added `NULL`,
`DISTINCT`, built-in functions, term-code node predicates, and the ANTLR4
grammar rewrite — all in scope.

---

## Normative requirements (what a compliant CDR MUST do)

### Query structure and lexical rules

- **Q-01 — Clause skeleton.** An AQL statement consists of `SELECT` (mandatory),
  `FROM` (mandatory), `WHERE`, `ORDER BY`, `LIMIT` (each optional), **in that
  order**. "An AQL statement must at least contain the `SELECT` and `FROM`
  clauses." — `AQL/master03-syntax.adoc` §Query structure/Overview (lines
  906–918); grammar rule `selectQuery` (`AqlParser.g4:22-24`).
- **Q-02 — Case-insensitive keywords.** "Keywords in AQL are not
  case-sensitive" (`SELECT`/`select`/`SeLeCt` identical). —
  `AQL/master03-syntax.adoc` §Reserved words and characters (line 17).
- **Q-03 — Reserved words and characters.** The reserved set: clause keywords
  (`SELECT`, `AS`, `FROM`, `CONTAINS`, `WHERE`, `ORDER BY`, `LIMIT`, `OFFSET`,
  `DISTINCT`), operators (`AND`, `OR`, `NOT`, `LIKE`, `matches`, `exists`,
  `<`, `>`, `=`, `!`), function names (`COUNT`, `MIN`, `MAX`, `SUM`, `AVG`,
  `LENGTH`, `CONTAINS`, `POSITION`, `SUBSTRING`, `CONCAT`, `CONCAT_WS`, `ABS`,
  `MOD`, `CEIL`, `FLOOR`, `ROUND`, `CURRENT_DATE`, `CURRENT_TIME`,
  `CURRENT_DATE_TIME`, `NOW`, `CURRENT_TIMEZONE`, `TERMINOLOGY`), literals
  (`true`, `false`, `NULL`), and the delimiter characters (`"` `'` `|` `[]`
  `{}` `()` `$` and the path characters `/` `.`). —
  `AQL/master03-syntax.adoc` §Reserved words and characters (lines 19–32).
- **Q-04 — Comments.** `--`-introduced line comments are lexed to a hidden
  channel (i.e. skipped); a leading Unicode BOM is skipped. — `AqlLexer.g4`
  rules `COMMENT`, `UNICODE_BOM`.

### Paths, variables, parameters, predicates

- **Q-05 — openEHR path syntax.** Both path kinds must be supported inside
  identified paths: *archetype paths* (nodes within an archetype, e.g.
  `/data[at0002]/events[at0003]/data[at0001]/items[at0004]`) and *RM class
  attribute paths* (e.g. `/category`, `/context/start_time`, `/uid/value`),
  with predicate expressions in `[]` at any step. —
  `AQL/master03-syntax.adoc` §openEHR path syntax (lines 36–63); grammar
  `objectPath`/`pathPart` (`AqlParser.g4:142-147`).
- **Q-06 — Variables.** Defined only in `FROM`; must be unique within a
  statement; not case-sensitive; optional on classes not referenced elsewhere;
  formed of an initial letter followed by alphanumerics/underscores, not
  clashing with reserved words. — `AQL/master03-syntax.adoc` §Variables/Syntax
  (lines 86–95).
- **Q-07 — Parameters.** `$name` parameters (letters/digits/underscores, no
  reserved words) substitutable **anywhere a criterion value appears**: inside
  predicates (`[$archetypeId]`, `[at0003, $nameValue]`,
  `[ehr_id/value=$ehrUid]`) and in `WHERE` criteria; substituted values follow
  literal quoting rules (strings/dates quoted, numbers/booleans not). —
  `AQL/master03-syntax.adoc` §Parameters/Syntax (lines 97–113).
- **Q-08 — Standard predicate.** `[lhs op rhs]` where lhs is an openEHR path,
  op ∈ `>, >=, =, <, <=, !=`, rhs is a value, parameter, or another openEHR
  path. — `AQL/master03-syntax.adoc` §Standard predicate (lines 147–157).
- **Q-09 — Archetype predicate.** Bare archetype-id predicate
  (`[openEHR-EHR-COMPOSITION.encounter.v1]`), usable **only in the FROM
  clause**; semantically equivalent to a standard predicate on
  `archetype_node_id` (the spec sanctions canonical-form rewriting). —
  `AQL/master03-syntax.adoc` §Archetype predicate (lines 159–181).
- **Q-10 — Node predicate, all five forms.** (a) `[at0002]`; (b)
  `[at0002 and name/value=…]`; (c) comma shortcut `[at0002, 'name'|$param]`;
  (d) term-code name shortcut `[at0002, terminology::code|value|]` with the
  specified expansion to `name/defining_code/code_string` +
  `terminology_id/value` (local terminology for at-codes); (e) general
  criterion `[at0002 and value/defining_code/terminology_id/value=$tid]`. —
  `AQL/master03-syntax.adoc` §Node predicate (lines 183–249). The grammar
  additionally admits `ARCHETYPE_HRID` heads, `PARAMETER`-only, boolean
  `AND`/`OR` of node predicates, and `objectPath MATCHES CONTAINED_REGEX`
  (`AqlParser.g4:118-126`).
- **Q-11 — Identified paths, three forms.** `var/path`, `var[predicate]`,
  `var[predicate]/path`; usable in every clause except `FROM`. —
  `AQL/master03-syntax.adoc` §Identified Paths/Syntax (lines 251–286).

### Operators

- **Q-12 — Comparison operators.** `=`, `>`, `>=`, `<`, `<=`, `!=`, `LIKE`,
  `matches` on identified paths. — `AQL/master03-syntax.adoc` §Comparison
  operators (lines 293–308).
- **Q-13 — LIKE semantics.** `?` matches exactly one character, `*` matches
  zero or more; the pattern matches the **entire** string; without wildcards
  `LIKE` behaves as `=`; literal `?`/`*` escaped with backslash; applies to
  strings and string-represented dates/times. — `AQL/master03-syntax.adoc`
  §LIKE (lines 310–332).
- **Q-14 — matches with value list.** `matches {v1, v2, …}` — cADL-style list
  of string/date-time/integer/real items (strings and date/times quoted), OR
  semantics across items. — `AQL/master03-syntax.adoc` §matches item 1 (lines
  338–365).
- **Q-15 — matches with URI.** `matches { terminology://… }` — a terminology
  URI (RFC 3986; scheme=terminology, authority=service, path=function,
  query=arguments), openEHR EHR URI, or other URI as the operand. —
  `AQL/master03-syntax.adoc` §matches item 2 (lines 367–402).
- **Q-16 — matches with TERMINOLOGY() results.** The right-hand operand may be
  a `TERMINOLOGY(…)` call, or a curly-brace list **mixing explicit codes with
  TERMINOLOGY() results** ("the AQL interpreter is responsible for generating
  a valid list of codes during semantic analysis"). —
  `AQL/master03-syntax.adoc` §matches item 3 (lines 404–416) + §TERMINOLOGY
  (lines 748–767).
- **Q-17 — Logical operators.** `AND`, `OR` (binary, over boolean
  expressions), `NOT` (unary negation) in `WHERE`, with parentheses for
  grouping. — `AQL/master03-syntax.adoc` §Logical operators (lines 418–458),
  §WHERE/Syntax (lines 993–1006).
- **Q-18 — EXISTS.** Unary operator over an identified path, `WHERE`-only:
  true iff data exists at the path; combinable with `NOT`. —
  `AQL/master03-syntax.adoc` §EXISTS (lines 473–488).

### Functions

- **Q-19 — Aggregate functions.** `COUNT([DISTINCT] expr | *)` (returns 0 on
  no rows, Integer), `MIN(expr)`, `MAX(expr)` (String/Date/Time/Integer/Real
  input, NULL on no rows, return type = input type), `SUM(expr)`, `AVG(expr)`
  (Integer/Real input, NULL on no rows). NULL inputs ignored unless stated. —
  `AQL/master03-syntax.adoc` §Aggregate functions (lines 503–569); grammar
  `aggregateFunctionCall` (`AqlParser.g4:186-189`).
- **Q-20 — String functions.** `LENGTH(s)→Integer`,
  `CONTAINS(s, sub)→Boolean`, `POSITION(sub, s)→Integer` (1-based, 0 if
  absent), `SUBSTRING(s, pos[, len])→String` (1-based; len optional →
  to end), `CONCAT(e1, e2, …)`, `CONCAT_WS(sep, e1, e2, …)`. —
  `AQL/master03-syntax.adoc` §String functions (lines 571–619).
- **Q-21 — Numeric functions.** `ABS(x)`, `MOD(x, y)`, `CEIL(x)`, `FLOOR(x)`,
  `ROUND(x[, decimal=0])`, Real/Integer args, result type derived from
  argument. — `AQL/master03-syntax.adoc` §Numeric functions (lines 621–662).
- **Q-22 — Date/time functions.** `CURRENT_DATE()` ('YYYY-MM-DD'),
  `CURRENT_TIME()` ('hh:mm:ss'), `CURRENT_DATE_TIME()`/`NOW()`
  ('YYYY-MM-DDThh:mm:ss.sss±hh:mm'), `CURRENT_TIMEZONE()` ('±hh:mm'), all
  zero-argument, String results. — `AQL/master03-syntax.adoc` §Date and time
  functions (lines 664–695).
- **Q-23 — TERMINOLOGY function.** `TERMINOLOGY(operation, service_api,
  params_uri)` (all String args, `Any` result) invoking an external
  terminology server (expand/validate/lookup/map/subsumes, FHIR TS et al.);
  usable as a `matches` operand, inside a `matches {…}` list, and as a Boolean
  value expression (`TERMINOLOGY(…) = true`). — `AQL/master03-syntax.adoc`
  §TERMINOLOGY (lines 699–769); grammar `terminologyFunction`
  (`AqlParser.g4:191-193`).
- **Q-24 — Function composition.** Function args are expressions over
  literals, parameters, variables, identified paths **or other functions**;
  functions are usable primarily in `SELECT` and `WHERE`. The listed set is
  the *core*: "Various other functions may exist however in various AQL
  implementations." — `AQL/master03-syntax.adoc` §Functions (lines 492–501).

### FROM / containment / versioning

- **Q-25 — Class expressions.** RM-class name (mandatory) + optional variable
  + optional standard-or-archetype predicate; RM classes are whatever the
  bound Reference Model defines (for openEHR: `EHR`, `COMPOSITION`,
  `OBSERVATION`, …); AQL itself is model-neutral. —
  `AQL/master03-syntax.adoc` §Class expressions (lines 775–808), §FROM (lines
  920–932).
- **Q-26 — CONTAINS.** Containment constraint between two class expressions
  (parent CONTAINS child) matching data-hierarchy relationships; chains of
  arbitrary depth. — `AQL/master03-syntax.adoc` §Containment (lines 958–966);
  `AQL/master02-overview.adoc` feature 3 (line 21).
- **Q-27 — Boolean containment trees.** "Logical operators `AND` and `OR` and
  parentheses `()` are used when multiple containment constraints are
  required" — both `AND` and `OR` between CONTAINS branches are normative. —
  `AQL/master03-syntax.adoc` §Containment (lines 968–979); grammar
  `containsExpr` (`AqlParser.g4:73-78`).
- **Q-28 — NOT CONTAINS.** `NOT` combined with `CONTAINS` expresses an
  exclusion constraint (absence of any containment relationship). —
  `AQL/master03-syntax.adoc` §Containment (lines 981–987) and §NOT (lines
  460–471).
- **Q-29 — VERSION sources & version predicates.** The grammar admits
  `VERSION var [LATEST_VERSION | ALL_VERSIONS | standardPredicate]` as a FROM
  class-expression operand — i.e. querying version objects, latest-only, all
  versions, or versions filtered by a standard predicate (e.g.
  `VERSION v[commit_audit/time_committed > '…']`). — `AqlParser.g4:89-92`
  (`versionClassExpr`), `:128-132` (`versionPredicate`);
  `AqlLexer.g4:52-54`; `AQL/master02-overview.adoc` feature 6: "supports
  time-based conditions to query historical versions of data" (line 24).
  (Prose semantics are absent from master03 — see Spec defects below.)

### SELECT / ORDER BY / LIMIT

- **Q-30 — SELECT column expressions.** Identified paths, functions,
  **literals**, or bare variable names (returning the full RM object of the
  variable's type); multiple columns comma-separated; each column may carry an
  `AS` alias conforming to variable-name syntax. — `AQL/master03-syntax.adoc`
  §SELECT/Syntax (lines 1008–1053), §Name alias (lines 1089–1092); grammar
  `selectExpr`/`columnExpr` (`AqlParser.g4:46-48,66-71`).
- **Q-31 — DISTINCT.** `SELECT DISTINCT` removes duplicate rows (a row is
  duplicate if every corresponding column value matches). —
  `AQL/master03-syntax.adoc` §DISTINCT (lines 1055–1068).
- **Q-32 — TOP (deprecated).** `TOP n [FORWARD|BACKWARD]` must still parse
  (deprecated in favour of `LIMIT` + `ORDER BY`); **combining `TOP` and
  `LIMIT` in one query is not allowed**. — `AQL/master03-syntax.adoc` §TOP
  (lines 1070–1087) + NOTE line 34; grammar `top` (`AqlParser.g4:195-198`).
- **Q-33 — ORDER BY.** One or more sorting expressions (identified path +
  optional `DESC|DESCENDING|ASC|ASCENDING`, default ascending); multi-key
  tie-breaking left to right; ordering uses equal/less-than/greater-than over
  primitives and `Ordered` types. Without `ORDER BY`, default ordering is
  **undefined** by the spec. — `AQL/master03-syntax.adoc` §ORDER BY (lines
  1094–1113).
- **Q-34 — LIMIT/OFFSET.** `LIMIT row_count [OFFSET offset]`; `row_count`
  minimum 1, `offset` minimum 0 (default 0); applies **after** DISTINCT
  de-duplication; pagination semantics (offset of first row is 0). —
  `AQL/master03-syntax.adoc` §LIMIT (lines 1115–1153).

### Literals, types, results

- **Q-35 — Literals & built-in types.** Integer (no separators, no hex), Real
  (decimal point), Boolean (`true`/`false` case-insensitive), String
  (single- or double-quoted, no line breaks), `NULL`, and ISO 8601
  dates/times/datetimes as quoted literals — **extended-format temporal
  literals are grammatically classified as date/time values** (basic-format
  ones remain strings); the underlying temporal type is inferred from context
  (path metadata / ISO format) so engines process them as temporal quantities.
  — `AQL/master03-syntax.adoc` §Literals (lines 855–861), §Built-in Types
  (lines 863–904 incl. NOTE line 885); grammar `primitive`
  (`AqlParser.g4:165-171`); negative and scientific numerics
  (`numericPrimitive`, `AqlParser.g4:173-179`).
- **Q-36 — Identified expressions.** WHERE criteria are unary (`NOT`,
  `EXISTS`) or binary: lhs = identified path **or function over one**, op =
  comparison operator, rhs = literal/function value, parameter, LIKE/matches
  pattern, **or another identified path** (path-vs-path comparison). —
  `AQL/master03-syntax.adoc` §Identified expression (lines 810–853); grammar
  `identifiedExpr` (`AqlParser.g4:80-87`).
- **Q-37 — Result structure.** The raw result is a 2-dimensional table,
  `Array<Array<Any>>`, with `NULL` for missing/unknown items; annotated result
  structures (column metadata) are delegated to the service layer — for
  openEHR, the RESULT_SET of the Abstract Platform Query Service / REST Query
  API. — `AQL/master04-result_structure.adoc` (whole file).
- **Q-38 — Returned granularity.** Results may be objects of any granularity,
  from top-level RM objects to primitive data items. —
  `AQL/master02-overview.adoc` feature 2 (line 20).

---

## Current implementation state (verified, not assumed)

Verified against: `crates/openehr-query/` (lexer/parser/AST — 49/49 tests pass,
run 2026-07-09), `app/ehrbase/src/aql/` (`analyze`/`ir`/`lower`/`sql`/`exec`,
32 unit tests + 4 e2e in `app/ehrbase/tests/service_aql.rs`),
`docs/design/aql-engine.md` (the documented feature envelope),
`docs/spec-audit/SPEC_AUDIT.md` + `findings/08-aql-parser.md` (area 08: 0
critical, 3 major, 8 minor, 3 info — majors and minors fixed except F-08-14),
the blueprint §2 gap surface (§2.2: AqlBasic 5 ECC failing; ADR-011 rebuild in
flight), and `tools/conformance/src/suites/{query,query_golden}.rs`.

**Layer split:** the *parser* (Q-01…Q-11 syntax, Q-25…Q-36 grammar shapes) is
essentially complete; the *engine* executes a large core envelope with every
reject a typed `AqlFeatureError`/`SqlError::Unsupported` (never a silent wrong
answer), but several normative constructs are currently in the rejected set.

| Req | State | Evidence / what is missing |
|---|---|---|
| Q-01 clause skeleton | **DONE** | `AqlParser`-faithful `selectQuery` in `crates/openehr-query/src/parser.rs`; audit 08: "clause skeleton and EOF enforcement are faithful" |
| Q-02 case-insensitive keywords | **DONE** | logos case-insensitive keyword patterns, `lexer.rs`; audit 08 positive notes |
| Q-03 reserved words | **PARTIAL (documented)** | Function names (`length`, `abs`, `now`, …) deliberately lex as identifiers — PORT NOTE at `lexer.rs:24-32` (F-08-05 resolution): accepts queries the spec's reserved list rejects; grammar's `functionCall` also admits bare `IDENTIFIER`, so divergence is defensible and recorded |
| Q-04 comments/BOM | **DONE** | F-08-04 + F-08-13 fixed (`--` comments skipped, BOM skipped) |
| Q-05 path syntax | **DONE** | `objectPath`/`pathPart` with nested predicates, `parser.rs`; corpus test `official_aql_corpus_parses` green |
| Q-06 variables | **PARTIAL** | Parsed correctly; **uniqueness not enforced anywhere** (F-08-14 still open — semantic-pass responsibility, unimplemented in `analyze.rs`; no duplicate-variable check found) |
| Q-07 parameters | **DONE** | Parser: `PARAMETER` in predicates/terminals; engine: `ir::Params` typed binds, missing-param → typed error; e2e-tested (`docs/design/aql-engine.md` §Status "$parameters … tested") |
| Q-08 standard predicate | **DONE** | Parser + engine (`EhrPredicate`, node constraints); path-rhs supported in parse; engine compares paths (`service_aql.rs`) |
| Q-09 archetype predicate | **DONE** | Parsed; lowered to `Source` archetype constraint on the promoted `node.archetype` column (`sql.rs`) |
| Q-10 node predicate, 5 forms | **DONE (parse) / PARTIAL (exec)** | All prose forms parse (incl. term-code after F-08-01 hyphen fix); grammar-extra `MATCHES CONTAINED_REGEX` parses but is a typed reject at analysis (`analyze.rs:468` `AqlFeatureError::RegexNodePredicate`) |
| Q-11 identified paths | **DONE** | All three forms, parser + `analyze` path-split (node-row anchor + fragment jsonpath) |
| Q-12 comparison operators | **DONE** | Full set lowered to `Expr::Cmp` with typed `Coercion` (magnitude/text/temporal); e2e-tested |
| Q-13 LIKE | **DONE** | `aql_like_to_sql` (`sql.rs:1146`) translates `?`/`*` + backslash escapes to SQL `LIKE`; literal + parameter patterns (`lower.rs:342-347`) |
| Q-14 matches value list | **DONE** | `MatchesOperand::ValueList` → `Expr::Matches` → SQL `IN`-style (`sql.rs:791`); parameters in lists supported |
| Q-15 matches URI | **MISSING (typed reject)** | `lower.rs:360` → `AqlFeatureError::MatchesUri`; needs terminology-service integration |
| Q-16 matches TERMINOLOGY() | **MISSING (typed reject)** | `lower.rs:357-358` (`MatchesTerminology`), `:453` (in value lists); documented reject in `docs/design/aql-engine.md` §Feature envelope |
| Q-17 AND/OR/NOT in WHERE | **DONE** | `whereExpr` boolean tree parsed with correct precedence (audit 08 positive); `IrExpr::And/Or/Not` rendered (`sql.rs:768`); tested |
| Q-18 EXISTS | **DONE** | `identifiedExpr` EXISTS parsed; `Expr::Exists` lowered/rendered; envelope status "tested / accepted" |
| Q-19 aggregates | **DONE** | `aggregateFunctionCall` parsed; `AggFunc` COUNT/COUNT(*)/COUNT(DISTINCT)/MIN/MAX/SUM/AVG in IR→SQL; COUNT/MIN/MAX e2e-tested, SUM/AVG accepted (design §Status) |
| Q-20 string functions | **MISSING (typed reject at SQL)** | LENGTH/SUBSTRING/POSITION/CONCAT/CONCAT_WS parse and lower to `ScalarFn` (`ir.rs:597-629`, `lower.rs:430-446`) but `sql.rs:694` ("scalar function in SELECT") and `:812` ("scalar function operand") reject them — **no built-in single-row function is executable**. String-`CONTAINS()` is additionally absent from the whitelist (also un-lexable as a call — grammar tokenizes `CONTAINS` as keyword; see Spec defects D-2) |
| Q-21 numeric functions | **MISSING (typed reject at SQL)** | Same as Q-20: ABS/MOD/CEIL/FLOOR/ROUND whitelisted in IR, rejected at SQL rendering |
| Q-22 date/time functions | **MISSING** | CurrentDate/Time/DateTime/Now in the IR whitelist but rejected at SQL; **`CURRENT_TIMEZONE` is not in the whitelist at all** (`lower.rs:430-446` has no `current_timezone` arm → `UnsupportedFunction`) |
| Q-23 TERMINOLOGY function | **MISSING (typed reject)** | `lower.rs:395` `AqlFeatureError::TerminologyFunction`; built at B4 with the terminology-server integration (design §Feature envelope) |
| Q-24 function composition | **PARTIAL** | Parser: `functionCall` args are `terminal`s incl. nested functions — done; engine: blocked on Q-20/21/22 execution |
| Q-25 class expressions | **DONE** | `classExpression` parsed; `Source::VersionedObject`/`Ehr` with rm_type/archetype/name constraints via `openehr_rm::model` (BMM-generated) |
| Q-26 CONTAINS chains | **DONE** | Nested-set interval self-joins on the node store; 2–3-deep chains e2e-tested on PG18 (`service_aql.rs`) |
| Q-27 AND/**OR** containment | **PARTIAL** | AND-CONTAINS accepted; **OR-CONTAINS parses + lowers but SQL generation rejects it** (`sql.rs:275-276` `Unsupported("OR in the CONTAINS/FROM tree")`) — a normative gap (master03 line 968 makes `OR` in containment first-class) |
| Q-28 NOT CONTAINS | **DONE** | Anti-join (`NOT EXISTS`); e2e-tested (design §Status "tested") |
| Q-29 VERSION sources | **DONE** | `versionClassExpr` + all three `versionPredicate` alternatives parse (F-08-02 fixed); engine: LATEST_VERSION (partial index) and ALL_VERSIONS (temporal table unfiltered) e2e-tested; standard-predicate (at-time) accepted (`ir.rs:202`); VERSION metadata paths (uid, commit_audit/time_committed) tested — beyond-EHRbase capability (ADR-008) |
| Q-30 SELECT column exprs | **PARTIAL** | Paths/literals/aliases/whole-object variables done + tested (whole-object cells reassembled through the P10 node codec); **function column expressions rejected** (`sql.rs:694`) pending Q-20/21/22 |
| Q-31 DISTINCT | **DONE** | `QueryIr.distinct` → native SQL DISTINCT; tested |
| Q-32 TOP | **DONE** | Parsed (incl. FORWARD/BACKWARD); mapped to LIMIT; `TOP`+`LIMIT` → typed reject `TopWithLimit` (`lower.rs:420-427`) |
| Q-33 ORDER BY | **DONE** | Multi-key, ASC/DESC + long forms, default ASC; DV_ORDERED keys via `ext.openehr_magnitude` (IMMUTABLE, indexable); tested. Now-family functions in ORDER BY are a documented typed reject (`ir.rs:634` `is_temporal_now`) |
| Q-34 LIMIT/OFFSET | **PARTIAL** | Parse + SQL + REST fetch/offset composition (+conflict 400) tested; **`row_count >= 1` / `offset >= 0` bounds not validated** (`lower_limit`, `lower.rs:420-427`, passes values through; F-08-14 open) |
| Q-35 literals & types | **DONE** | All primitives incl. NULL (`analyze.rs:521` `TypedLit::Null`), negative/scientific numerics; temporal literals get typed AST variants + context inference in `Coercion::Temporal` (F-08-03 overflow and F-08-06 temporal-variant findings fixed) |
| Q-36 identified expressions | **PARTIAL** | Binary/unary forms incl. path-vs-path comparison parse and execute; function-lhs (`functionCall COMPARISON terminal`) parses but hits the scalar-function SQL reject |
| Q-37 result structure | **DONE** | `exec.rs` builds ITS-REST 1.0.3 RESULT_SET (columns from aliases/paths, rows as canonical JSON, NULL for missing); e2e via `/query/*` endpoints (`service_query.rs`); all six QUERY endpoints live (P16 close, `docs/plans/current-phase.md`) |
| Q-38 result granularity | **DONE** | Primitive leaves through whole COMPOSITION objects (node-codec reassembly); tested |

**Conformance signal:** ECC run 2026-07-08 — `AqlBasic` 5/… failing
("AQL feature envelope edges") + `QueryProvisioning` 4 failing (stored-query
wire edges), out of 106 total failures dominated by non-AQL
ArchetypeValidation (blueprint §2.2). ECC is suspended during the
ADR-011 rebuild and re-converges at B1/P19.

---

## Remaining work (ordered, concrete)

1. **OR in the CONTAINS tree (Q-27).** The one *core-syntax* normative reject.
   Lower `ContainsTree::Or` to SQL — union of interval-join branches or a
   disjunctive `EXISTS` pair per branch over the same anchor
   (`app/ehrbase/src/aql/sql.rs:275`); add e2e proof
   (`OBSERVATION a OR OBSERVATION b` under one COMPOSITION) and an
   envelope-test flip.
2. **Execute the single-row function set (Q-20/21/22, unblocks Q-24/Q-30/Q-36
   function forms).** Render `ScalarFn` in `sql.rs` SELECT columns, WHERE
   operands, and (non-now-family) ORDER BY: string fns map ~1:1 to PG
   (`length`, `substr`, `position`, `concat`, `concat_ws` — mind 1-based and
   whole-string semantics per master03 §String functions), numeric to
   `abs/mod/ceil/floor/round`, date/time to bind-time constants formatted per
   master03 §Date and time functions. Add `CURRENT_TIMEZONE` to the
   `scalar_fn` whitelist (`lower.rs:430`). Decide + PORT-NOTE the string
   `CONTAINS()` call (grammar makes it un-lexable — see D-2).
3. **Semantic-pass constraints (Q-06, Q-34; F-08-14 — the only open area-08
   finding).** In `analyze.rs`: reject duplicate variable names; validate
   `LIMIT >= 1`, `OFFSET >= 0` (typed errors → ITS-REST 400/422 per the wire
   mapping).
4. **Close the 5 `AqlBasic` + 4 `QueryProvisioning` ECC failures** once the
   ADR-011 rebuild is green — triage each against Q-requirements above;
   expected overlap with items 1–3 (`tools/conformance/src/suites/query.rs`).
5. **matches URI + TERMINOLOGY() (Q-15/16/23).** Requires the terminology
   service integration (SM track / `ehrbase-sm` terminology seam): implement
   `TERMINOLOGY('expand'|'validate'|…, service_api, params_uri)` against a
   FHIR-TS client (`reqwest`, wiremock-tested), expansion merged into
   `matches` value lists at semantic-analysis time per master03 lines
   756–759. Staged: `expand` in `matches` first; `validate`-as-boolean next;
   URI-operand form last. Keep the typed rejects until each lands.
6. **Node-predicate regex (`MATCHES CONTAINED_REGEX`, grammar-only).** Decide:
   implement via PG `~` on the extracted text, or keep the typed reject with
   a PORT NOTE citing that the construct exists only in `AqlParser.g4:123`
   with no prose semantics (D-3).
7. **Parse/print round-trip proptest + AQL printer** (audit 08 hygiene note;
   `.claude/rules/testing.md` prescribes it) — also gives the engine a
   canonical-form printer for stored-query normalization.
8. **P19 re-convergence:** full ECC QUERY suite + golden result sets
   (`suites/query_golden.rs`) green; re-audit this chapter's table.

---

## Spec defects/TBDs encountered (verbatim, cited)

- **D-1 — Placeholder TBD in the preface.**
  `docs/specs/openehr/QUERY/docs/AQL/master01-preface.adoc:12-13`:
  `[.tbd]` / `*TBD*: (example To Be Determined paragraph)` — an empty
  boilerplate TBD block published in the Release-1.1 preface (same in
  `AQL_examples/master01-preface.adoc:12-13`).
- **D-2 — `CONTAINS` string function is un-lexable under the normative
  grammar.** Prose lists it as a built-in: "CONTAINS() | Validates if a
  string contains other string" (`AQL/master03-syntax.adoc:583`) and
  "`CONTAINS(expression, substring)`" (line 596), but `AqlLexer.g4` tokenizes
  `CONTAINS` as the containment keyword before any function-id group, so
  `contains(x, y)` can never parse as a function call (recorded as spec-audit
  F-08-12(a); implementation follows the grammar).
- **D-3 — VERSION semantics exist only in the grammar.**
  `AqlParser.g4:91` (`VERSION variable=IDENTIFIER? … versionPredicate`) and
  `:128-132` (`LATEST_VERSION | ALL_VERSIONS | standardPredicate`) define
  version-source queries, but `master03-syntax.adoc` contains **no section**
  describing their semantics; the only prose is `master02-overview.adoc:24`:
  "supports time-based conditions to query historical versions of data."
  Result-shape, predicate targets (`commit_audit/…`?), and interaction with
  CONTAINS are implementation-defined. Ours is PORT-NOTEd in
  `app/ehrbase/src/aql/ir.rs` (VersionScope; trunk-only branching).
  Likewise `objectPath MATCHES CONTAINED_REGEX` (`AqlParser.g4:123`,
  `AqlLexer.g4:125`) has no prose semantics anywhere in master03.
- **D-4 — TERMINOLOGY example contradicts the grammar.**
  `AQL_examples/master03-syntax-function.adoc:70` uses a nested call as the
  third argument — `TERMINOLOGY('subsumes', 'hl7.org/fhir/r4',
  CONCAT('system=…&codeA=235856003&codeB=', e/value/defining_code/code_string))
  = true` — but the grammar rule is `terminologyFunction : TERMINOLOGY '('
  STRING ',' STRING ',' STRING ')'` (`AqlParser.g4:191-193`), which admits
  only three string literals (spec-audit F-08-12(b); implementation follows
  the grammar and rejects the nested form).
- **D-5 — Result-set normativity is delegated.**
  `AQL/master04-result_structure.adoc:5`: "Such annotated results are not
  formally defined by this specification, and are considered an artefact of
  the relevant API or service definition." — column metadata/RESULT_SET
  conformance is therefore tested against ITS-REST + SM, not QUERY.
- **D-6 — Default ordering explicitly undefined.**
  `AQL/master03-syntax.adoc:1098`: "If no `ORDER BY` clause is specified,
  then the query result doesn't have any default ordering criteria defined by
  this specification. … In terms of compliance to this specification, default
  ordering in results is undefined." (Freedom, not defect — recorded so no
  test ever pins unordered results.)
- **D-7 — `exists` casing inconsistency.** The reserved-word list gives
  lowercase `exists` among operators (`master03-syntax.adoc:22`) while every
  usage and the lexer rule are `EXISTS`; keywords being case-insensitive
  (line 17) makes this cosmetic.
