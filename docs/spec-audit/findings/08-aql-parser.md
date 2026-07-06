# 08 — AQL 1.1 lexer/parser/AST

## Summary

Audited `crates/openehr-query` (`lexer.rs`, `ast.rs`, `parser.rs`, `tests/corpus.rs`)
against the authoritative openEHR QUERY (AQL 1.1) grammar vendored at
`crates/openehr-query/vendor/grammar/{AqlLexer.g4,AqlParser.g4}` and the normative
syntax text at `docs/specs/openehr/QUERY/docs/AQL/master03-syntax.adoc`. The grammar
(not EHRbase behaviour) is the oracle.

Overall the parser is a faithful, well-structured transcription of the grammar:
clause order, `CONTAINS`/`WHERE` boolean trees, `AND`>`OR` and `NOT`-tightest
precedence, aggregates, `matches`/`LIKE`, node/archetype/standard predicates,
`DISTINCT`/`TOP`/`LIMIT`/`OFFSET`, parameters, and `terminology()` are all present
and correct, and the design correctly resolves the PEG ordering hazards (function
before path, version before class, etc.).

However there are **three real defects that reject or corrupt valid AQL**: (1) the
`TERM_CODE` lexer regex omits the hyphen, so hyphenated terminology ids
(`SNOMED-CT::…`, `ISO_639-1::…` — the latter is literally cited as an example in the
lexer's own doc comment and the grammar) fail to lex; (2) the `versionPredicate`
`standardPredicate` alternative is not implemented, so `VERSION v[<path> op <val>]`
does not parse; (3) integer literals that overflow `i64` are silently coerced to `0`
via `unwrap_or_default()`. Several smaller grammar-coverage gaps and one dead AST
variant follow. All findings below were confirmed by running the lexer/parser.

Severity counts: **critical 0 · major 3 · minor 8 · info 4**.

## Findings

### F-08-01: Hyphenated terminology ids fail to lex (`TERM_CODE` regex omits `-`)
- **Severity:** major
- **Spec:** `AqlLexer.g4` rules `TERM_CODE` / `TERM_CODE_CHAR` (`fragment TERM_CODE_CHAR: NAME_CHAR | '.'`, and `NAME_CHAR: WORD_CHAR | '-'`); `master03-syntax.adoc` §Node predicate (term-code form `terminology_id::code_string|value|`).
- **Code:** `crates/openehr-query/src/lexer.rs:178-182` (`TermCode` regex `[a-zA-Z0-9._]+(\([a-zA-Z0-9._]+\))?::[a-zA-Z0-9._]+(\|[^|\[\]]+\|)?`).
- **Problem:** `TERM_CODE_CHAR` in the grammar includes `-` (via `NAME_CHAR`), but the regex character classes are `[a-zA-Z0-9._]` — no hyphen. So the terminology-id and code segments cannot contain `-`. Confirmed:
  `SNOMED-CT::1234` lexes as `[Identifier("SNOMED"), Minus, TermCode("CT::1234")]`;
  `ISO_639-1::en` lexes as `[Identifier("ISO_639"), Minus, TermCode("1::en")]`.
  In a predicate/matches context this then fails to parse (`SELECT o FROM OBSERVATION o[at0001, SNOMED-CT::1234|x|]` → parse error at `SNOMED`). The lexer's own doc comment (line 168) cites `'ISO_639-1::en'` as a valid `TERM_CODE`, and hyphenated terminology ids (SNOMED-CT, ICD10-AM) are extremely common. There is no standalone `:` token, so the trailing `::` also becomes an unrecoverable lex error in some inputs.
- **Fix:** add `-` to both `TERM_CODE` character classes (and the parenthesised version part): `[a-zA-Z0-9._-]+(\([a-zA-Z0-9._-]+\))?::[a-zA-Z0-9._-]+(\|[^|\[\]]+\|)?`. Add a lexer test for `SNOMED-CT::1234` and `ISO_639-1::en`.
- [ ] fixed

### F-08-02: `VERSION` standard-predicate form not parsed
- **Severity:** major
- **Spec:** `AqlParser.g4` rule `versionPredicate : LATEST_VERSION | ALL_VERSIONS | standardPredicate ;`
- **Code:** `crates/openehr-query/src/parser.rs:341-344` (`version_predicate = select!{ LatestVersion => Latest, AllVersions => All }`).
- **Problem:** only the `LATEST_VERSION` and `ALL_VERSIONS` keyword alternatives are handled; the third alternative, `standardPredicate` (`objectPath COMPARISON pathPredicateOperand`), is missing. So a version predicate such as `VERSION v[commit_audit/time_committed > '2020-01-01']` or `VERSION v[uid/value=$vid]` fails to parse (confirmed: parse error at `commit_audit`). The AST already declares the target variant `VersionPredicate::Standard(Box<StandardPredicate>)` (`ast.rs:329-330`), so this is purely an unwired parser branch. ADR-008 makes `ALL_VERSIONS`/version querying a first-class capability, so this gap matters.
- **Fix:** add the `standard` predicate parser (already built inside `path_parsers`) as a third alternative in `version_predicate`, mapping to `VersionPredicate::Standard`. Wire `path_parsers` output through so the `standard` parser is available at this call site.
- [ ] fixed

### F-08-03: Integer-literal overflow silently coerced to `0` (`unwrap_or_default`)
- **Severity:** major
- **Spec:** `master03-syntax.adoc` §Built-in Types / Integer data ("Integers are represented as numeric literals … `1`, `2`, `365`"); no stated 64-bit bound.
- **Code:** `crates/openehr-query/src/parser.rs:69-72` (`Token::Integer(s) => Primitive::Integer(s.parse().unwrap_or_default())`, likewise `Real`/`SciInteger`/`SciReal`); also `:314` (`TOP`) and `:476` (`LIMIT`/`OFFSET`).
- **Problem:** `s.parse::<i64>().unwrap_or_default()` returns `0` when the literal does not fit `i64`, silently corrupting the query rather than reporting an error. Confirmed: `… WHERE c/x = 99999999999999999999999` parses to `Primitive(Integer(0))`. A comparison against `0` instead of the intended value is a silent, hard-to-diagnose wrong result. (`Real` overflow becomes `inf` via `f64::parse`, also silent.) `Primitive::Integer` is `i64`, which additionally can't represent AQL integers wider than 64 bits at all.
- **Fix:** make lexeme→value conversion fallible and surface a parse error on overflow (or widen to `i128`/store the raw lexeme). At minimum, do not use `unwrap_or_default()` for a value that changes query semantics — return an error instead of `0`.
- [ ] fixed

### F-08-04: AQL line comments (`--` COMMENT channel) not stripped
- **Severity:** minor
- **Spec:** `AqlLexer.g4` rules `COMMENT` (`SYM_DOUBLE_DASH ' ' ~[\r\n]* …` and bare `--` → `channel(COMMENT_CHANNEL)`).
- **Code:** `crates/openehr-query/src/lexer.rs:43` (`#[logos(skip r"[ \t\r\n\f]+")]` — whitespace only) and `:152-154` (`--` → `DoubleDash`).
- **Problem:** the grammar defines `--`-introduced line comments on a hidden channel; they should be skipped. The lexer has no comment rule, so `--` is always emitted as `DoubleDash`, and any real comment breaks the token stream. Confirmed: `SELECT c -- comment\nFROM COMPOSITION c` → parse error at `DoubleDash`. (Note the grammar's `SYM_DOUBLE_DASH?` before EOF is nearly unreachable in ANTLR because `COMMENT` also matches a bare `--\n`/`--<EOF>`; the practical requirement is that `--` starts a comment.)
- **Fix:** add a `logos` skip/callback for `-- …` to end-of-line (and bare `--` at EOL/EOF) matching the `COMMENT` rule; keep `DoubleDash` only for the (rare) inline terminator case if needed, or drop it.
- [ ] fixed

### F-08-05: Reserved function-name words lex as identifiers (over-permissive)
- **Severity:** minor
- **Spec:** `master03-syntax.adoc` §"Reserved words and characters" (line 23) lists `LENGTH, CONTAINS, POSITION, SUBSTRING, CONCAT, CONCAT_WS, ABS, MOD, CEIL, FLOOR, ROUND, CURRENT_DATE, CURRENT_TIME, CURRENT_DATE_TIME, NOW, CURRENT_TIMEZONE, TERMINOLOGY` as reserved; `AqlLexer.g4` tokenizes them (`STRING_FUNCTION_ID`/`NUMERIC_FUNCTION_ID`/`DATE_TIME_FUNCTION_ID` and the individual rules).
- **Code:** `crates/openehr-query/src/lexer.rs:214-215` (everything not a dedicated keyword falls through to `Identifier`); the function-id group is intentionally not tokenized (module note lines 8-14).
- **Problem:** in ANTLR these names lex as `*_FUNCTION_ID` tokens and thus cannot be used as variable/attribute identifiers; here they lex as `Identifier`, so `SELECT length FROM EHR length` parses (confirmed: no error), treating `length` as a class variable — which the grammar/spec forbid. This is a permissiveness divergence (accepts queries the spec rejects). Design trade-off is documented, but it does change the reserved-word envelope.
- **Fix:** either add these as case-insensitive keyword tokens that the `functionCall` `name` parser accepts (matching the grammar), or accept the divergence with a `// PORT NOTE:` explicitly recording that single-row function names are not reserved as identifiers. Prefer the former for conformance.
- [ ] fixed

### F-08-06: `DATE`/`TIME`/`DATETIME` literals collapsed to `Primitive::String` — no typed AST variant
- **Severity:** minor
- **Spec:** `AqlLexer.g4` distinct tokens `DATE`, `TIME`, `DATETIME`; `AqlParser.g4` `primitive : STRING | numericPrimitive | DATE | TIME | DATETIME | BOOLEAN | NULL`; `master03-syntax.adoc` §Dates and Times ("AQL grammar identifies this value as a datetime value").
- **Code:** `crates/openehr-query/src/lexer.rs:15-19` (PORT NOTE: temporals lexed as `String`); `ast.rs:437-450` (`Primitive` has `String/Integer/Real/Boolean/Null` only — no `Date/Time/DateTime`).
- **Problem:** the grammar produces distinct temporal tokens and `primitive` has distinct temporal alternatives; the spec states the *grammar* classifies extended-ISO literals as date/time (vs. basic-format strings). The implementation lexes all quoted values as `String` and the AST has no temporal variant, so the syntactic distinction the grammar makes is not preserved anywhere — even as a deferred-typing placeholder. Downstream (typing from path context per the §Dates and Times NOTE) is still workable, but the parser cannot record "the grammar would have tagged this DATE".
- **Fix:** acceptable as a deferral, but add `Primitive::{Date,Time,DateTime}` (or a `Temporal(raw)` variant) and either lex the ISO forms or re-classify quoted values at parse time, so the semantic pass has the grammar's distinction available. At minimum document that all temporal literals are indistinguishable from strings in this AST.
- [ ] fixed

### F-08-07: Archetype-HRID `-rc`/`-alpha` version suffixes and namespace hyphens not lexed
- **Severity:** minor
- **Spec:** `AqlLexer.g4` `VERSION_ID : DIGIT+ ('.' DIGIT+)* ( ( '-rc' | '-alpha' ) ( '.' DIGIT+ )? )? ;` and `NAMESPACE`/`LABEL` (which permit `-` via `NAME_CHAR`).
- **Code:** `crates/openehr-query/src/lexer.rs:171-175` (`ArchetypeHrid` regex, version tail `\.v[0-9]+(\.[0-9]+)*`, namespace prefix `[a-zA-Z][a-zA-Z0-9_.]*::`).
- **Problem:** the version tail regex stops at digits/dots, so `…v1.0.0-rc.2` or `…v2-alpha` won't lex as a single `ArchetypeHrid` (the `-rc`/`-alpha` becomes `Minus`/`Identifier`). The namespace prefix class omits `-`, so a hyphenated namespace won't be captured. Both are grammar-legal.
- **Fix:** extend the version tail to `(\.[0-9]+)*((-rc|-alpha)(\.[0-9]+)?)?` and add `-` to the namespace class, matching `VERSION_ID` and `LABEL`.
- [ ] fixed

### F-08-08: Recursive unary minus in `numericPrimitive` not accepted
- **Severity:** minor
- **Spec:** `AqlParser.g4` `numericPrimitive : INTEGER | REAL | SCI_INTEGER | SCI_REAL | SYM_MINUS numericPrimitive ;` (minus recurses over `numericPrimitive`).
- **Code:** `crates/openehr-query/src/parser.rs:75-79` (`negative = just(Token::Minus).ignore_then(unsigned)` — a single leading minus over an unsigned literal only).
- **Problem:** the grammar allows `SYM_MINUS numericPrimitive` recursively (e.g. `- - 5`); the implementation accepts exactly one minus. Confirmed: `… = - - 5` → parse error at the second `Minus`. Edge case, but a strict grammar-coverage gap.
- **Fix:** make the minus rule recursive (`just(Minus).repeated().foldr(unsigned, negate)`), or explicitly document the single-minus restriction with a `// PORT NOTE:`.
- [ ] fixed

### F-08-09: String literals not unescaped — AST holds raw escaped text
- **Severity:** minor
- **Spec:** `AqlLexer.g4` `STRING` with `ESCAPE_SEQ` (`\\ ['"?abfnrtv\\]`), `OCTAL_ESC`, `UTF8CHAR` (`\\u HEX{4}`).
- **Code:** `crates/openehr-query/src/parser.rs:58-65` (`unquote` strips only the surrounding quotes) and `ast.rs:438-441` (String doc: "unescaping … deferred; raw slice sans surrounding quotes").
- **Problem:** escape sequences are left verbatim in the AST string, so `'a\nb'` yields the four characters `a \ n b`, and a `\'`-escaped quote inside a single-quoted string is retained as `\'`. This is a documented deferral, but any consumer comparing string values (predicate matching, `LIKE` operands, `terminology()` args) sees un-normalised text.
- **Fix:** unescape per `ESCAPE_SEQ`/`OCTAL_ESC`/`UTF8CHAR` when building `Primitive::String`/name/like/terminology operands (or explicitly define the boundary that unescaping happens in the semantic pass, and ensure that pass exists before the IR consumes strings).
- [ ] fixed

### F-08-10: `PathPredicate::Standard` is unreachable; standard predicates classified as node predicates
- **Severity:** minor
- **Spec:** `AqlParser.g4` `pathPredicate : '[' (standardPredicate | archetypePredicate | nodePredicate) ']' ;` — three distinct classifications; `master03-syntax.adoc` §Predicates ("three types of predicates … standard … archetype … node").
- **Code:** `crates/openehr-query/src/parser.rs:153-180` — `node` includes `standard` as one of its atoms (`node_atom … .or(standard …)`), and the `predicate` definition tries `archetype`, then `node`, then `standard`. Because `node` already matches the bare `objectPath COMPARISON operand` form, the trailing `PathPredicate::Standard` alternative (`:178`) is never reached.
- **Problem:** a plain standard predicate such as `[ehr_id/value='123']` is parsed as `PathPredicate::Node(NodePredicate::Standard(..))` rather than `PathPredicate::Standard(..)`. Information is preserved (same path/op/operand), but the grammar's three-way classification is flattened and the `PathPredicate::Standard` AST variant is dead code. Downstream code that pattern-matches on `PathPredicate::Standard` (expecting the grammar's split) will silently never fire.
- **Fix:** either reorder so a bare standard comparison (no `AND`/`OR`, no code) is classified as `PathPredicate::Standard` (matching the grammar), or remove `PathPredicate::Standard` and document that standard predicates are represented as `Node(NodePredicate::Standard)`. Pick one and make the AST honest.
- [ ] fixed

### F-08-11: `SCI_INTEGER` mapped to `Primitive::Real` (integer-ness lost)
- **Severity:** minor
- **Spec:** `AqlLexer.g4` distinguishes `SCI_INTEGER: INTEGER E_SUFFIX` from `SCI_REAL: REAL E_SUFFIX`; `AqlParser.g4` `numericPrimitive` lists them separately.
- **Code:** `crates/openehr-query/src/parser.rs:71-72` (`Token::SciInteger(s) => Primitive::Real(...)`).
- **Problem:** `1e10` (a `SCI_INTEGER` in the grammar) becomes `Primitive::Real`, losing the integer classification the grammar assigns. Minor because scientific notation denotes a magnitude either way, but it is a strict-fidelity divergence with no `SciInteger` AST variant.
- **Fix:** either add distinct handling (parse `SciInteger` to `Primitive::Integer` when it is integral) or record a `// PORT NOTE:` that scientific literals are always `Real`.
- [ ] fixed

### F-08-12: Grammar/spec mismatches faithfully inherited (informational)
- **Severity:** info
- **Spec:** `master03-syntax.adoc` §Functions vs `AqlLexer.g4`/`AqlParser.g4`.
- **Code:** `crates/openehr-query/src/{lexer.rs,parser.rs}`.
- **Problem:** two spec-vs-grammar tensions where the implementation correctly follows the *grammar* (the oracle), worth recording so they are not "fixed" wrongly later:
  (a) The prose lists `CONTAINS` as a string function, but in the lexer `CONTAINS` is tokenized before `STRING_FUNCTION_ID`, so `contains(...)` can never be a string-function call — the implementation likewise lexes `contains` as the `Contains` keyword, matching ANTLR. (b) `master03-syntax.adoc` TERMINOLOGY Example 5 uses a nested `CONCAT(...)` as a `TERMINOLOGY` argument, but `terminologyFunction : TERMINOLOGY '(' STRING ',' STRING ',' STRING ')'` allows only three `STRING`s; the implementation correctly rejects the nested-function arg. Both are grammar limits, not implementation bugs.
- **Fix:** none required; keep as documented `// PORT NOTE:` if these ever surface as apparent "failures" against spec prose examples.
- [ ] fixed

### F-08-13: UTF BOM not skipped
- **Severity:** info
- **Spec:** `AqlLexer.g4` `UNICODE_BOM: (…) -> skip;`
- **Code:** `crates/openehr-query/src/lexer.rs:42-43` (skips only `[ \t\r\n\f]+`).
- **Problem:** a leading UTF-8/16/32 BOM is not skipped and would cause a lex error on the first token. Very low impact (callers normally strip BOMs), but a strict grammar-coverage omission.
- **Fix:** add a `logos` skip for `\u{FEFF}` (and the byte BOM if inputs are ever non-UTF-8-normalised).
- [ ] fixed

### F-08-14: Semantic-only constraints not enforced (informational, likely out of scope)
- **Severity:** info
- **Spec:** `master03-syntax.adoc` §TOP ("not allowed to use TOP while also using LIMIT"), §LIMIT (`row_count` min 1, `offset` min 0), §Variables ("variable name must be unique").
- **Code:** `crates/openehr-query/src/parser.rs` (no such checks — appropriately, this is the syntactic layer).
- **Problem:** these are semantic rules the grammar does not encode; the parser does not (and arguably should not) enforce them. Recorded only so the downstream semantic/analysis pass (P16) owns them explicitly.
- **Fix:** enforce in the semantic pass, not the parser; track as a P16 checklist item.
- [ ] fixed

## Hygiene notes

- **Corpus test is far thinner than it appears.** `tests/corpus.rs::select_blocks` only captures `----` listing blocks whose text contains `SELECT` (line 54). The spec's operator, function, `LIKE`, `matches`, `NOT`, and `TERMINOLOGY` examples are written as bare `WHERE …` fragments, so they are silently skipped. Live run: **10 SELECT blocks total → 4 parsed, 6 excluded, 0 exercised for WHERE/function syntax.** The suite therefore proves almost nothing about the `WHERE`/function/predicate surface. Recommend either wrapping fragment examples in a minimal `SELECT x FROM EHR e` shell before parsing, or extracting all listing blocks and classifying them.
- **Untested constructs (no unit test and not in the parsed corpus subset):** `VERSION` class expression, `LATEST_VERSION`/`ALL_VERSIONS`, `TOP … FORWARD/BACKWARD`, `MIN/MAX/SUM/AVG`, all string/numeric/date-time functions, `LIKE`, `matches { URI }`, `terminology()`, standard predicate on a class expression (`EHR e[ehr_id/value=$x]`), parameter predicates (`[$archetypeId]`, `[at0002, $name]`), negative/scientific numerics, and multi-column `ORDER BY`. Several of these intersect the defects above (F-08-01/02/03), which is why the gaps went unnoticed.
- **No parse/print round-trip property test.** `.claude/rules/testing.md` prescribes `proptest` "AQL parse/print round-trips"; there is neither a printer nor a proptest. Round-trip testing would have surfaced F-08-09 (escapes) and F-08-03 (integer overflow) quickly. (A printer is also generally useful for the IR pass.)
- **`unwrap_or_default()` on lexeme parsing (parser.rs:69-72, 314, 476)** is the anti-pattern behind F-08-03; audit all four sites — silent `0`/`inf` defaults have no place in value-carrying parse output.
- **Dead code:** `PathPredicate::Standard` (F-08-10) and `VersionPredicate::Standard` (F-08-02) AST variants are declared but never produced by the parser; either wire them or remove them so the AST reflects what is actually built.
- Positive: precedence handling (`AND`>`OR`, `NOT` tightest, `CONTAINS` greedily binding the following sub-expression) and the PEG-ordering choices (function-before-path in `terminal`/`columnExpr`, version-before-class in `classExprOperand`, archetype/node-before-standard in predicates) are correct and match ANTLR's resolution of the ambiguous left-recursive rules. The clause skeleton and EOF enforcement are faithful.
