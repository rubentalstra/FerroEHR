---
paths: ["crates/openehr-aql/**", "crates/openehr-server/src/aql/**"]
---

# AQL engine rules

The AQL engine is the crown jewel and the hardest part of the port
(PORT_MASTER_PLAN.md Section 6, difficulty map: AQL planner/SQL generator are
VERY HARD). It splits across two locations:

- `crates/openehr-aql/` — the spec crate: AQL 1.1.0 lexer, parser, AST, and
  semantic/path analysis against Web Templates. No Java receives this; it is
  written from the QUERY 1.1.0 specification.
- `crates/openehr-server/src/aql/` — receives EHRbase's `aql-engine` Maven
  module Java in place: the ASL (Abstract SQL Layer, EHRbase's own IR), the
  ASL rewrite/optimize passes, and the ASL→SQL translator.

## No ANTLR, ever

EHRbase's grammar is ANTLR4. We reimplement it natively — **no ANTLR
runtime is a dependency of the running server, ever.** Use `logos` 0.15 for
the lexer and `chumsky` 0.10 (or `winnow` 0.7) for the parser. Grammar
sources to transcribe from (not bind to):

- Canonical AQL grammar: `specifications-QUERY/docs/AQL/grammar/AqlLexer.g4`
  and `AqlParser.g4`.
- Reference ANTLR grammars for cross-checking token/rule names:
  `openEHR/openEHR-antlr4` → `reader_aql/src/main/antlr/*.g4`.

Port the grammar's token and rule structure faithfully into the lexer/parser
combinator shape; do not invent a different grammar even where chumsky makes
a different factoring tempting.

## Pipeline stays structurally faithful

The Java pipeline is: parse → semantic/path analysis against Web Templates →
**AST → ASL** → ASL rewrite/optimize → **ASL → SQL** (JSONB path extraction,
array unnesting, current+history UNION) → execute → assemble RESULT_SET
(schema 1.0.3). Port each stage as its own module/pass mirroring the Java
class boundaries in `aql-engine`'s `AqlSqlLayer`, ASL model classes, and
optimizer passes. Do not collapse stages together even if a end-to-end
rewrite would be shorter — a later phase can only debug this by comparing
stage-by-stage against the Java source.

**ASL is EHRbase's own intermediate representation, not an openEHR spec
artifact.** Port it as-is from the Java; do not redesign the IR even though
it is bespoke. If PG 18/17 features (`JSON_TABLE`, skip scan) let a later SQL
generation pass simplify the emitted SQL, note that as `// PERF(port):`
during Stage 1 and apply it only after parity (P19 Optimization), unless the
phase task explicitly calls for using `JSON_TABLE` where viable (P13).

## Boundary with openehr-server

`openehr-aql` produces a parsed, semantically-analyzed AST. Everything after
that (ASL construction, SQL generation, execution against `sqlx`/
`sea-query`) lives in `openehr-server`. Keep this boundary; do not let SQL
concerns leak into the spec crate, and do not let grammar/parsing concerns
leak into the server crate.

Every file here still needs the PORT STATUS trailer and annotation
vocabulary from `rust-style.md`.
