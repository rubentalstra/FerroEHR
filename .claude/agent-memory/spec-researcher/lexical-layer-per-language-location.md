---
name: lexical-layer-per-language-location
description: Where the per-language LEXICAL rules (keywords/case/reservedness, boolean, number, date-time, code tokens) live for ODIN vs ADL2-cADL vs BEL vs EL, plus the normative-.g4 elevation and the confirmed lexical defects
metadata:
  type: reference
---

# Lexical layer: ODIN / ADL2-cADL / BEL / EL — navigation

## Owning sections (docs text)

| topic | ODIN | ADL2 (cADL) | BEL | EL |
|---|---|---|---|---|
| keywords | `LANG/docs/odin/master03-basics.adoc` §Keywords (L47-49, "no keywords of its own") | `AM/docs/ADL2/master04.2-cadl_basics.adoc` §Keywords (L3-24, list only) + `master07.04-adl_basics.adoc` §Keywords (L3-19, the SECTION keywords + "can safely appear as identifiers") | `LANG/docs/BEL/master03-language.adoc` §Operators (L253-287) | `LANG/docs/EL/master05-expressions.adoc` §Primitive Operators (L33-63, UPPERCASE) |
| case rule | boolean only (master07 L73) | **SILENT** (only .g4 has it) | **SILENT** | `EL/master03-basics.adoc` §Syntax style L7 "Upper- and lower-case are not formally distinguished" |
| literals | `odin/master07-leaf_data.adoc` (whole chapter) | `ADL2/master04.5-cadl_primitive_types.adoc` (delegates to ODIN for int/real/string lists/intervals) | `BEL/master03-language.adoc` §Literals L23-101 | `EL/master04-terminal_entities.adoc` §Literals L11-77 |
| escapes/encoding | `odin/master03-basics.adoc` §File Encoding + §Special Character Sequences | `ADL2/master03-file_encoding.adoc` (verbatim twin) | — | — |
| grammar appendix | `odin/masterAppB-syntax_spec.adoc` (include-only) | `ADL2/masterAppB-syntax_spec.adoc` (include-only) | `BEL/masterAppA-syntax.adoc` (include-only) | `EL/masterAppA-syntax.adoc` → ElParser.g4/ElLexer.g4 **NOT VENDORED** |

## The .g4 elevation (important)

`ADL2/masterAppB-syntax_spec.adoc` L4: "**The normative specification of the ADL2 syntax is expressed in Antlr4 as a series of component grammars, shown below.**" — the vendored `.g4` files ARE that appendix's content, so for ADL2 they are normative-by-reference, not merely reference material. Vendored at `crates/openehr-adl/vendor/grammar/` + `crates/openehr-lang/vendor/grammar/`.

Import graph (determines which lexer sees which tokens):
`odin.g4 → odin_values.g4 → base_lexer.g4` (**no adl_keywords**) ·
`cadl2_primitives.g4 → adl_keywords + odin_values` ·
`base_expressions.g4 (=BEL) → cadl2_primitives + odin_values` (**so BEL DOES see the ADL keyword set**) ·
`cadl2.g4 → cadl2_primitives, odin, base_expressions` · `adl2.g4 → cadl2`.
Consequence: standalone ODIN reserves nothing; ODIN *inside an ADL2 file* is lexed by the merged single ANTLR lexer that includes `adl_keywords`.
`adl_keywords.g4` is NOT one of the six files masterAppB includes → keyword casing has no docs-text home at all for ADL2.

## Load-bearing lexical facts

- `base_lexer.g4` L94-95 `SYM_TRUE/SYM_FALSE` case-insensitive; L178-180 `INTEGER: DIGIT+ E_SUFFIX?`, `REAL: DIGIT+ '.' DIGIT+ E_SUFFIX?` (sign is a PARSER rule in `odin_values.g4` L59/L68, never part of the token; only `ISO8601_DURATION` L91 carries a lexer-level `-`).
- `base_lexer.g4` L35-47 puts DATE/TIME/DATE_TIME/DURATION_CONSTRAINT_PATTERN in the SHARED base lexer (so they tokenize in ODIN text too) while only `cadl2_primitives.g4` c_date/c_time/c_date_time/c_duration consume them.
- `adl_keywords.g4` makes every ADL section keyword require a leading `'\n'` (L16-23) — that is the mechanism behind master07.04's "can safely appear as identifiers".

## Confirmed released-text defects (lexical)

1. `master04.5` L412 "`yyyy-??-XX` could be transformed into `1995-??-XX`" — unlexable: YEAR_PATTERN is only `yyyy|YYYY|yyy|YYY`, and ISO8601_DATE has no `??`/`XX` mix.
2. `BEL/master03` L42-43 + `ADL2/master07.11-adl_rules.adoc` L137-138/148-149: `[snomed_ct::389086002|Hypoxia|]`, `[at0016|never used|]` — TERM_CODE_REF forbids `|`; no production exists.
3. `EL/master03` L18-20 makes `|` a comment-leader while `EL/master04` L63-71 makes `|` the interval delimiter and `master05` L137 the quantifier separator.
4. `EL/master05` L89/L97 make `--` the `subtract_nominal` operator while `master03` L18 makes `--` the comment leader.
5. `odin/master07` L213 `[at0200], ...` — bracket-code list item with no ODIN production (TERM_CODE_REF needs `::`).
6. `odin/master04` L8 `@schema = URI` — no `@` token/production in odin.g4.
7. `odin/master06` L50 `<["tourism_db_13"]/hotels[...]>` — ADL_PATH segments must start with ALPHA_LC_ID.
8. `ADL2/master07.05` L14 allows `adl_version=N.M` but VERSION_ID requires 3 parts.
9. `EL/master04` L246 `PY2003`, L310 `'P38Y'` (quoted duration).
10. `%` percentage literal: defined only in `EL/master04` L22-23; one stray ODIN example (`master05` L138 `weighting = <76%>`); no ODIN/BEL production.
11. `master.adoc` includes `master07.11-adl_rules.adoc` — the `…rulesNEW.adoc` file (with `check`/`defined()`/`symbols` section) is **NOT included** and has no grammar. (Corrects an earlier memory note that said rulesNEW is current.)
12. ADL 1.4 by contrast DOES carry an in-docs lexical spec: `ADL1.4/master05-cadl.adoc` L1325-1355 (case-insensitive keyword char-classes, `V_LOCAL_CODE : a[ct][0-9.]+`) and `ADL1.4/master08-adl.adoc` L658-745.
