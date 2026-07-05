---
paths: ["crates/openehr-query/**", "crates/ehrbase/src/aql/**"]
---

# AQL engine rules

The AQL engine is the crown jewel and the hardest part (P16). It spans two
locations:

- `crates/openehr-query/` — the spec crate: AQL 1.1.0 **lexer + AST + parser**,
  **done** (`logos` + `chumsky`, corpus-validated). Semantic path analysis
  against WebTemplates is done as part of the engine (P16), consuming this AST.
- `crates/ehrbase/src/aql/` — the **execution engine** (P16): AST → **our own
  typed query IR** → PostgreSQL, designed fresh per ADR-008 over the P10 node
  model, with path analysis driven by the **BMM-generated RM attribute model**
  (no reflection, no hand tables). EHRbase's engine is prior art only.

## No ANTLR, ever

The parser is `logos` + `chumsky` (`openehr-query`), reimplemented from the
QUERY 1.1.0 grammar — **no ANTLR runtime is ever a dependency of the server.**
This is done; don't revisit it.

## The pipeline (our design, ADR-008)

parse (`openehr-query`, done) → path analysis + typing against the
**BMM-generated RM attribute model** (+ WebTemplate where template context is
needed) → **AST → typed query IR** (our own Rust enums) → **IR → SQL** (via
`sea-query`: nested-set interval joins on the node table for CONTAINS,
`jsonb_path_query_first` + jsonpath item methods + `openehr_magnitude` for
typed leaf comparison/ordering, `JSON_TABLE` for array unnesting, GIN
`jsonb_ops` `$.**` equality anchors as pre-filters) → execute (`sqlx`) →
assemble `RESULT_SET` (schema 1.0.3). Keep the IR a distinct pass — that is
what keeps the hard cases tractable — and do **not** collapse it away.

Versioning semantics: `LATEST_VERSION` = the current partial index;
`ALL_VERSIONS` = the temporal table unfiltered (supported — ADR-008). A pure
perf tuning of the SQL that isn't needed for correctness is a
`// PERF(port):` for P20 (Optimization).

## Boundary

`openehr-query` produces a parsed, semantically-analysable AST; everything after
(the IR, SQL generation via `sea-query`, execution via `sqlx`) lives in `ehrbase`.
Keep it clean: no SQL in the spec crate, no grammar/parsing in the server crate.
Behaviour is verified by the AQL corpus + the CNF conformance suite (P19).
