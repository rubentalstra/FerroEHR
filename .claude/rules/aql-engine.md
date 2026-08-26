---
paths: ["crates/openehr-query/**", "app/ferroehr/src/aql/**"]
---

# AQL engine rules

The AQL engine is the crown jewel — **built and extended**; this rule
governs maintenance and extension. It spans two locations:

- `crates/openehr-query/` — the spec crate: AQL 1.1.0 **lexer + AST + parser**
  (`logos` + `chumsky`, corpus-validated). Semantic path analysis consumes
  this AST inside the engine.
- `app/ferroehr/src/aql/` — the **execution engine**: AST → **our own typed
  query IR** → PostgreSQL, designed fresh over the node model,
  with path analysis driven by the **BMM-generated RM attribute model** (no
  reflection, no hand tables). The AQL terminology family
  (`TERMINOLOGY('expand')` merged into `matches` at semantic analysis; later
  stages typed rejects) is implemented. EHRbase's engine is prior art only.
- **Every unsupported construct is a typed, citable reject** — never a
  silent wrong answer, never a generic 500.

## No ANTLR, ever

The parser is `logos` + `chumsky` (`openehr-query`), reimplemented from the
QUERY 1.1.0 grammar — **no ANTLR runtime is ever a dependency of the server.**
This is done; don't revisit it.

## The pipeline (our design)

parse (`openehr-query`, done) → path analysis + typing against the
**BMM-generated RM attribute model** (+ WebTemplate where template context is
needed) → **AST → typed query IR** (our own Rust enums) → **IR → SQL** (via
`sea-query`: nested-set interval joins on the node table for CONTAINS,
`jsonb_path_query_first` + jsonpath item methods + `openehr_magnitude` for
typed leaf comparison/ordering, `JSON_TABLE` for array unnesting, GIN
`jsonb_ops` `$.**` equality anchors as pre-filters) → execute (`sqlx`) →
assemble `RESULT_SET` (schema 1.1.0). Keep the IR a distinct pass — that is
what keeps the hard cases tractable — and do **not** collapse it away.

Versioning semantics: `LATEST_VERSION` = the current partial index;
`ALL_VERSIONS` = the temporal table unfiltered (supported). A pure
perf tuning of the SQL that isn't needed for correctness is a
`// TODO(perf):` for later optimization work.

## Spec sources (the oracle)

AQL semantics (grammar, operators, functions, `RESULT_SET`) are answered from
the vendored spec text at `docs/specs/openehr/QUERY/docs/AQL/` — never from
EHRbase behaviour. Conformance expectations derive from the CNF schedule
(`docs/specs/openehr/CNF/docs/platform_test_schedule/master05-func_tc_definition_query.adoc`
+ `master11-func_tc_querying.adoc`; the upstream Robot suites are reference
material only) and are **verified by the CNF QUERY chapter/AqlBasic capability**
(Veredictum) — the accept/reject envelope, status codes, and result
shapes. Use `/spec-lookup` and cite the section (spec-adherence.md).

## Boundary

`openehr-query` produces a parsed, semantically-analysable AST; everything after
(the IR, SQL generation via `sea-query`, execution via `sqlx`) lives in `ferroehr`.
Keep it clean: no SQL in the spec crate, no grammar/parsing in the server crate.
Behaviour is verified by the AQL corpus + the CNF pipeline — every engine
change ends with a `scripts/conformance.sh` run showing zero drift vs the
committed baseline.
