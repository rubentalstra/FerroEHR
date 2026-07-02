# Phase 12 — AQL parser + AST + semantic path analysis

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): QUERY 1.1.0, ADL / Layer 6c
- Compile required: no (Phase A)

## Objectives

Implement the AQL 1.1.0 lexer, parser, AST, and semantic path analyzer in
`openehr-aql`, resolving archetype paths against WebTemplates (Phase 10).
This is part of the AQL engine, one of the two VERY HARD areas on the
port-difficulty map (Section 6) and on the critical path.

## Preconditions

- [ ] Phase 10 done: WebTemplate available for path resolution
- [ ] Phase 09 done: ADL path syntax available for cross-reference

## Scope

In: AQL lexer (`logos`) and parser (`chumsky`) against the canonical
`AqlLexer.g4`/`AqlParser.g4`, the AQL AST (SELECT/FROM/WHERE/ORDER BY/
CONTAINS/etc.), semantic path analysis resolving archetype paths against a
WebTemplate.
Out: AST -> ASL translation and SQL generation (Phase 13 — a separate,
even-harder step), stored query persistence (that is `openehr-server`
service-layer concern, Phase 15).

## Tasks

- [ ] Reimplement the AQL grammar (`AqlLexer.g4`, `AqlParser.g4` from `specifications-QUERY/docs/AQL/grammar/`) as a `logos` lexer
- [ ] Implement the AQL parser (`chumsky`) producing a full AST: SELECT, FROM, WHERE, ORDER BY, CONTAINS, TOP/LIMIT/OFFSET
- [ ] Implement AQL predicate expression parsing (comparison operators, boolean combinators, parameters)
- [ ] Implement archetype-path parsing within AQL (the path syntax embedded in SELECT/WHERE clauses)
- [ ] Implement semantic path analysis: resolve each AQL path against a WebTemplate, producing a typed, node-linked path
- [ ] Implement error reporting for unresolvable paths (path not present in the WebTemplate) with `miette`/`ariadne` diagnostics
- [ ] Write parser round-trip tests (parse -> print -> parse -> equal) via `proptest`
- [ ] Write semantic analysis tests resolving representative AQL queries against a Phase 10 WebTemplate fixture
- [ ] Add PORT STATUS trailers; update `docs/ROSETTA.md` with AQL grammar -> Rust AST mappings

## Exit criteria

- [ ] AQL parser successfully parses a representative set of real AQL queries (from EHRbase's own test suite or openEHR conformance corpora)
- [ ] Semantic path analysis correctly resolves paths against a WebTemplate fixture and reports errors for invalid paths
- [ ] Parser round-trip property tests pass

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. Pull AQL test queries from EHRbase's own test suite before
writing the grammar, so the parser is validated against real-world query
shapes (including edge cases like `CONTAINS` nesting) from day one.
