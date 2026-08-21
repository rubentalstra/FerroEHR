---
name: el-grammar-landmarks
description: Where the EL/BEL grammar + expression object model live (vendored .g4 set at crates/openehr-lang/vendor/grammar/v1_1) and the confirmed grammar-vs-published-schema gaps
metadata:
  type: reference
---

# EL / BEL syntax + model — where to look

## The grammars ARE vendored (docs only `include::` a remote repo)
- `LANG/docs/EL/masterAppA-syntax.adoc` is 17 lines: it `include::`s
  `{openehr_openehr_antlr_include}/ElParser.g4` + `ElLexer.g4` (remote), so the
  spec text itself carries NO productions. The files are vendored at
  **`crates/openehr-lang/vendor/grammar/v1_1/`** (ElParser.g4, ElLexer.g4,
  Cadl2Lexer/Parser, SymbolsLexer, GeneralIdsLexer, base_lexer, base_expressions,
  odin, odin_values) — provenance + the v1_0 ODIN-only set are described in
  `crates/openehr-lang/vendor/grammar/PROVENANCE.md`. BEL's appendix points at
  `base_expressions.g4` instead.
- Token homes: `SymbolsLexer.g4` L21 `SYM_NE : '/=' | '!=' | '≠'`, L22 `SYM_EQ`;
  `GeneralIdsLexer.g4` L13/14 UC_ID/LC_ID; `ElLexer.g4` L47-52 SYM_THEN/SYM_AND/
  SYM_OR/SYM_IMPLIES.

## Confirmed gaps (re-verified first-hand 2026-08-21)
- `ElParser.g4` L239 `elFunctionCall: LC_ID ( '(' elExprList ')' )?` with L241
  `elExprList: elExpression ( ',' elExpression )*` → **no zero-argument
  parenthesised call**; no `'(' ')'` alternative anywhere. Published BMM schemas
  nevertheless write `is_null()`, `signature().result`, `is_callable()`.
- `SYM_AND` = `'and' | 'AND' | '∧'` only, and **SYM_THEN is defined but used in
  NO ElParser production** → `and then` / `or else` are inexpressible.
- `attached()` is defined in `LANG/docs/EL/master04-terminal_entities.adoc`
  §Attached() Predicate **L278-286** (negated as `not attached (ref)`).
  **`unattached` appears NOWHERE** in the vendored specs or grammars, and the EL
  chapters never use the word "Void".
- Quantifiers: ElParser L90/92 `elForAllExpr`/`elThereExistsExpr` use `'¦'` (broken
  bar), while `LANG/docs/BEL/master03-language.adoc` §Container Operators L303-328
  prints `|`.
- BEL model gap: `LANG/docs/UML/classes/org.openehr.lang.beom.expr_for_all.adoc`
  declares only `condition: ASSERTION` + `operand: EXPR_VALUE_REF` (inheriting
  precedence_overridden/operator/symbol from `…beom.expr_operator.adoc`) — **no
  attribute carries the bound variable** of master03's `for_all v : c | …`; there
  is no EXPR_THERE_EXISTS class at all. Class pages are pulled in by
  `LANG/docs/BEL/master04-expression_object_model.adoc` §Core Package L43-62.
- Eiffel-flavoured forms present in the published BMM schemas (grep the vendor
  BMM tree): `and then`, `/= Void`, `Result :=` (am 2.2.0/2.3.0),
  `arity in |1..2|` (lang 1.0.0 + bmm3), `.for_all (v: T | …)`,
  `for_all p in creators` (bmm3). **Typographic quotes are `“` ESCAPES in
  the .json files** — grep the odin/yaml serialisations (or the escape) or you
  will wrongly conclude they are absent.

Related: [[bmm-schema-validity-landmarks]], [[lexical-layer-per-language-location]].
