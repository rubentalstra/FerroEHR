---
paths: ["crates/openehr-query/**", "crates/ehrbase/src/aql/**"]
---

# AQL engine rules

The AQL engine is the crown jewel and the hardest part (P16). It spans two
locations:

- `crates/openehr-query/` — the spec crate: AQL 1.1.0 **lexer + AST + parser**,
  **done** (`logos` + `chumsky`, corpus-validated). Semantic path analysis
  against WebTemplates is done as part of the engine (P16), consuming this AST.
- `crates/ehrbase/src/aql/` — the **execution engine** (P16): AST → an
  abstract-SQL IR (ASL) → PostgreSQL, built in **idiomatic Rust following
  EHRbase's proven approach** (its `aql-engine` Java — `asl`, `pathanalysis`,
  `querywrapper`, `sql`, `featurecheck` — is the read-only behavioural reference,
  not a class-mirror template; ADR-006).

## No ANTLR, ever

The parser is `logos` + `chumsky` (`openehr-query`), reimplemented from the
QUERY 1.1.0 grammar — **no ANTLR runtime is ever a dependency of the server.**
This is done; don't revisit it.

## The pipeline (follow EHRbase's approach, idiomatically)

parse (`openehr-query`, done) → semantic/path analysis vs WebTemplate →
**AST → ASL** (abstract-SQL IR) → ASL rewrite/optimize → **ASL → SQL** (via
`sea-query`: JSONB path extraction, array unnesting, current+history `UNION`,
`JSON_TABLE` where viable) → execute (`sqlx`) → assemble `RESULT_SET` (schema
1.0.3). Keep these as distinct passes/modules — EHRbase's ASL IR is a proven
design worth following (it's how the hard cases stay tractable), and stage
boundaries make behaviour debuggable against the reference. Write idiomatic Rust;
do **not** mirror the Java class layout, and do **not** collapse the IR away.

**ASL is EHRbase's own IR, not an openEHR spec artifact** — follow its shape as
the reference. Use PG 18/17 features (`JSON_TABLE`, skip scan) where they
simplify the emitted SQL; a pure perf tuning of the SQL that isn't needed for
correctness is a `// PERF(port):` for P20 (Optimization).

## Boundary

`openehr-query` produces a parsed, semantically-analysable AST; everything after
(ASL, SQL generation via `sea-query`, execution via `sqlx`) lives in `ehrbase`.
Keep it clean: no SQL in the spec crate, no grammar/parsing in the server crate.
Behaviour is verified by the parity harness (P19), not by class-level diffing.
