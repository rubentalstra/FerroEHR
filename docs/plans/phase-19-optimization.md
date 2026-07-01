# Phase 19 — Optimization

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): the entire workspace (Phases 00-18)
- Compile required: perf

## Objectives

Tune the parity-complete system for performance: PostgreSQL 18 asynchronous
I/O configuration, pipelining for hot AQL read paths (isolating onto
`tokio-postgres` + `deadpool-postgres` if `sqlx`'s lack of query pipelining
bottlenecks them), and `JSON_TABLE()` codegen improvements in the AQL SQL
generator.

## Preconditions

- [ ] Phase 18 done: >=99% behavioral parity holds

## Scope

In: PG 18 AIO tuning, hot-path query pipelining, `JSON_TABLE()` codegen
improvements, benchmark harness (`criterion`/`divan`).
Out: any change that would alter observable REST-surface behavior (that would
regress Phase 18's parity number and needs to go back through the parity
harness).

## Tasks

- [ ] Benchmark the current AQL hot-read path with `criterion`/`divan` to establish a baseline
- [ ] Tune PostgreSQL 18 AIO configuration (`io_method`, `io_workers` or equivalent) for the workload profile
- [ ] Identify AQL read paths bottlenecked by `sqlx`'s lack of query pipelining
- [ ] Where bottlenecked, isolate that read path onto `tokio-postgres` + `deadpool-postgres` per Section 8's contingency
- [ ] Expand `JSON_TABLE()` usage in the ASL -> SQL generator (Phase 13) wherever PG 17+ support makes it a net win over the JSONB-extraction baseline
- [ ] Re-run the Phase 18 parity harness after every optimization to confirm no behavioral regression
- [ ] Re-run the Phase 19 benchmark suite after each change and record before/after numbers
- [ ] Document tuning decisions and benchmark deltas

## Exit criteria

- [ ] Benchmark suite shows a measured improvement over the Phase 18 baseline on the hot AQL read path
- [ ] Parity harness still reports >=99% after all optimizations
- [ ] Any `tokio-postgres`/`deadpool-postgres` isolation, if adopted, is documented with the benchmark data that justified it

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. Optimize only after parity holds, and re-verify parity after
every optimization — a faster query that returns a subtly different result
is a regression, not an improvement.
