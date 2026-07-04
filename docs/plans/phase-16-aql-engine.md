# Phase 16 — AQL engine (AST → ASL → SQL) — the crown jewel

- Status: not-started (Stage-1 app build, step 8 of 13)
- Consumes: `openehr-query` (AST, done), P09 (tables), P10 (rm-db-format row
  layout), P14 (WebTemplate, for path analysis)
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-006 (follow EHRbase's ASL approach, idiomatic Rust)

## Objectives

Execute AQL: take the parsed AST (`openehr-query`), analyse its paths against
WebTemplates, lower it through an **Abstract SQL Layer (ASL)** intermediate
representation, generate PostgreSQL against the row-per-locatable JSONB schema
(`sea-query`: JSONB path extraction, array unnesting, current + `_history`
`UNION`, `JSON_TABLE` where viable), execute, and assemble the ITS-REST
`RESULT_SET`. This is the highest-value bespoke subsystem; there is no crate for
it. We follow **EHRbase's proven pipeline** (`crates/ehrbase/src/aql/` —
`pathanalysis`, `asl`, `querywrapper`, `sql`, `featurecheck` — is the reference)
in idiomatic Rust.

## Preconditions

- [ ] P09 (schema/tables), P10 (row layout the SQL targets), P14 (WebTemplate)

## Scope

**In:** semantic path analysis (AQL paths ↔ WebTemplate/RM); the ASL IR + AST→ASL
lowering; ASL rewrite/optimize; ASL→SQL via `sea-query` (JSONB extraction,
current+history UNION, `JSON_TABLE`); parameter binding; execute via `sqlx`;
`RESULT_SET` assembly (schema 1.0.3); the QUERY endpoints (`/query/aql` ad-hoc +
stored) wired via P11/P12; feature-check/reject unsupported AQL. **Out:** the
front-end parser (P07, done); query result caching / perf tuning (P20).

## Tasks

- [ ] Semantic path analysis against WebTemplates
- [ ] ASL IR + AST→ASL lowering (+ rewrite/optimize)
- [ ] ASL→SQL generator (`sea-query`, JSONB, current+history UNION, JSON_TABLE)
- [ ] Execute (`sqlx`) + assemble `RESULT_SET`
- [ ] Wire `/query/aql` (ad-hoc + stored) endpoints
- [ ] Tests: AQL example corpus end-to-end + parity spot-checks vs EHRbase

## Exit criteria

- [ ] Representative AQL (the CDR conformance queries) execute end to end and
      return correct `RESULT_SET`s over real data
- [ ] current + `_history` semantics correct; JSONB extraction correct
- [ ] Compiles + clippy-clean

## Decisions made this phase

- ASL IR mirrors EHRbase's *approach* (not its Java classes); optional hot-read
  pipelining (`deadpool`/`tokio-postgres`) is a P20 concern, not here.
