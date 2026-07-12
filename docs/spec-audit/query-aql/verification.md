# A1 Spec Audit — Verify + Fix — chapter `query-aql`

- **Chapter:** QUERY — AQL 1.1 (master03 syntax + master04 result structure)
- **Date:** 2026-07-11
- **Scope:** all 50 requirements `query-aql-R1 … R50`
- **Result (defer-nothing pass):** 8 defect families fixed — the largest:
  the **single-row function set was represented in the IR but rejected at
  SQL generation** (every LENGTH/SUBSTRING/… query errored), and the
  **TERMINOLOGY() Boolean and URI-operand forms were typed rejects**. Also:
  duplicate variables silently accepted, variable case-sensitivity, `LIMIT
  0` accepted, LIKE escapes mistranslated, SUM/AVG silently NULL-coerced
  non-numeric input, no function arity checking. Zero deferrals.

## Verdict table (condensed)

| ids | classification | evidence / fix |
|---|---|---|
| R1 | verified | keyword case folding (lexer test `keywords_are_case_insensitive`) |
| R2, R3 | verified | grammar structure (mandatory SELECT+FROM, fixed clause order — chumsky sequence) |
| R4 | fixed | duplicate FROM variables now reject (`AnalysisError::DuplicateVariable`; bindings previously silently overwrote); test `duplicate_variable_rejected` |
| R5 | fixed (case) + verified (reserved) | variable names now case-fold for binding AND reference resolution (`Bindings` keys folded); reserved words cannot lex as identifiers; test `variable_names_fold_case` |
| R6, R7 | verified | parameter lexing; typed parameter substitution (`ParamValue` binds, not text splicing) |
| R8–R12 | verified | standard/archetype/node predicates incl. the name-value and term-code shortcuts (parser + `resolve_node_predicate`) |
| R13, R14 | verified | identified-path forms; the exact comparison-operator set (`CompOp`) |
| R15 | verified | LIKE is whole-string (SQL LIKE semantics; `*`→`%`, `?`→`_`) |
| R16 | fixed | `\*`/`\?` now translate to the LITERAL characters (previously emitted a stray backslash + live wildcard); test `like_escapes_are_literal` |
| R17, R18 | verified | matches RHS forms enforced by grammar; value-list OR semantics (`is_in`) |
| R19 | fixed | the terminology-URI operand (`matches { terminology://… }`) is now resolved at semantic analysis through the terminology seam into an explicit value list (was a typed reject); test `uri_operand_expands_to_a_value_list` |
| R20, R21 | verified | boolean operators lower to SQL AND/OR/NOT (semantic negation, not syntactic) |
| R22 | verified | EXISTS is grammar-bound to WHERE |
| R23, R24 | verified | NOT CONTAINS executed as anti-join; OR/AND containment trees (B6) |
| R25, R26 | verified | class-expression arity (grammar); FROM classes resolved against the generated RM model (`UnknownClass`) |
| R27, R28 | verified | SQL aggregate NULL semantics; `COUNT(*)`/`COUNT(DISTINCT)`, Integer return, 0 on empty (service_aql acceptance set) |
| R29 | verified | MIN/MAX NULL-on-empty (SQL); input set per leaf typing |
| R30 | fixed | SUM/AVG over a textual/temporal/boolean leaf is now a typed reject (`AggregateInputType`; was a silent magnitude-NULL coercion); test `sum_over_textual_leaf_rejected` |
| R31–R37 | fixed | the ENTIRE single-row function set now renders to SQL and executes (was `Unsupported` at SQL build): LENGTH, SUBSTRING (1-based, optional length), POSITION (1-based, 0-absent — `strpos` arg order), string CONTAINS (new `ScalarFn::StrContains` + parser acceptance of the keyword in function position), CONCAT/CONCAT_WS, ABS/MOD/CEIL/FLOOR (Integer returns), ROUND (decimal defaults 0), CURRENT_DATE/TIME/DATE_TIME/NOW/**CURRENT_TIMEZONE** (new variant) in the exact spec formats; arity validated at lowering (`FunctionArity`); live-verified on PG18 (`scalar_functions_execute`: every value asserted, incl. 1-based positions and Integer ceils) |
| R38 | verified | TERMINOLOGY arity is grammar-fixed (three string args) |
| R39 | fixed | all three usage forms now work: (a) direct matches RHS + (b) embedded-in-braces merge (B4) and (c) the **Boolean value expression** — `TERMINOLOGY('validate'\|'subsumes', api, uri) = true` evaluated once at semantic analysis (constant args) through the new `boolean_operation` seam (FHIR provider / bundle routing; `lookup`/`map` typed-reject with no boolean semantics); AST gains `IdentifiedExpr::Resolved`, IR gains `Expr::Const`; tests `boolean_validate_form_resolves_to_a_constant`, `boolean_form_honours_the_operator_and_literal` |
| R40, R41 | verified | numeric literal lexing (no hex/grouping; real needs the point); case-insensitive booleans; single-line strings |
| R42 | verified-policy | temporal typing is leaf-driven (extended-ISO values compare temporally via the leaf coercion; basic-format strings compare as text) — the spec's context-inference realized through the typed IR |
| R43–R45 | verified | ORDER BY default ASC; ordered-magnitude comparison via `openehr_magnitude` (leaf coercion); no default order assumed |
| R46 | fixed | `LIMIT` < 1 / `OFFSET` < 0 now reject (`PagingBounds`); test `limit_zero_rejected` |
| R47 | verified | TOP+LIMIT conflict rejected (`TopWithLimit`) |
| R48, R49 | verified | DISTINCT-then-paging (SQL semantics); column-expression forms + AS aliases |
| R50 | verified | RESULT_SET rows are `Array<Array<Any>>` with NULL for missing leaves (exec assembly) |

## Fixes applied

- `aql/sql.rs` — `scalar_fn_expr` (the full function catalogue → PG SQL);
  LIKE escape handling; `Expr::Const` rendering.
- `aql/lower.rs` — `bind()` duplicate rejection; paging bounds; function
  arity; SUM/AVG input typing; `current_timezone`/`contains` whitelist.
- `aql/analyze.rs` — case-folded variable bindings.
- `aql/terminology.rs` — the pre-pass now resolves all three TERMINOLOGY
  usage forms + terminology-URI operands (`Request`/`Resolution` model);
  trait gains `boolean_operation` + `expand_uri`.
- `service/api/terminology.rs` — seam implementations (FHIR + bundle
  routing, `params_uri` query-arg parsing via the `urlencoding` crate).
- `openehr-query` — parser accepts `CONTAINS` as a function name in
  function position; AST `IdentifiedExpr::Resolved` (semantic-analysis
  product, documented).
- `aql/error.rs` — five new typed error variants; stale stage-(b)/(c)
  PORT NOTEs replaced.
- Owner ruling (2026-07-11): all percent/URL encoding+decoding now goes
  through the `urlencoding` crate (2.1.3) — both hand-rolled codecs
  (`ehrbase-rest/src/params.rs`, the new terminology query-arg parser)
  replaced; rule recorded in CLAUDE.md.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
