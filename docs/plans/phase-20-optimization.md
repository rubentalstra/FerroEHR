# Phase 20 — Optimization

> **App-crate reality (ADR-011, 2026-07-09):** three app crates —
> `app/{ehrbase, ehrbase-rest, ehrbase-sm}` + `tools/{conformance, benchmark}`
> + `crates/openehr-*`.

- Status: not-started (Stage-1 app build, step 12 of 13)
- Consumes: the parity-passing server (P19)
- Compile required: perf
- Decisions: `docs/postgres-features.md` (PG 18), ADR-006/008

## Objectives

Make the parity-passing server fast, without regressing parity. PostgreSQL 18
tuning (async I/O), a pipelined hot-read path for AQL where `sqlx` bottlenecks,
and `JSON_TABLE` codegen in the AQL SQL generator.

## Preconditions

- [ ] P19 parity holds (never optimize before parity)

## Scope

**In:** PG 18 AIO / index tuning; isolate hot AQL reads onto
`deadpool-postgres` + `tokio-postgres` for pipelining if measured to help;
`JSON_TABLE` (PG 17+) codegen in the ASL→SQL path; `criterion`/`divan` benches.
**Out:** any behavioural change (parity must stay green).

## Tasks

- [ ] Benchmark the hot paths (`criterion`/`divan`); find bottlenecks
- [ ] PG 18 AIO + index tuning
- [ ] Optional pipelined read path (`deadpool`/`tokio-postgres`) if it helps
- [ ] `JSON_TABLE` codegen in the AQL generator

## Exit criteria

- [ ] Measured improvement on hot paths; parity (P19) still green
- [ ] Benches committed; no correctness regressions

## Decisions made this phase

- Optimization never trades away parity; every change re-runs the P19 gate.
