# Phase 13 — AQL engine: AST -> ASL -> SQL

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): AQL (Phase 12), persistence (Phase 07)
- Compile required: no (Phase A)

## Objectives

Port EHRbase's AST -> ASL (Abstract SQL Layer, EHRbase's own IR) planner and
the ASL -> SQL generator, producing JSONB path extraction, array unnesting,
and current+history UNION queries, using `JSON_TABLE()` where PostgreSQL 17+
support makes it viable. This is the other VERY HARD area on the
port-difficulty map and the crown jewel of the whole port (Section 6).

## Preconditions

- [ ] Phase 12 done: AQL AST with resolved paths available
- [ ] Phase 07 done: persistence schema (`ehr.comp_data`/`_history` etc.) available

## Scope

In: the ASL intermediate representation, AST -> ASL planning/rewrite/optimize,
ASL -> SQL generation via `sea-query`, RESULT_SET assembly (schema 1.0.3).
Out: the AQL grammar/AST itself (Phase 12 owns that), rm-db-format decomposition
details beyond what the SQL generator needs to know about column shape
(Phase 14 owns the write side).

## Tasks

- [ ] Define the ASL intermediate representation (EHRbase's own IR) as Rust types: relation references, joins, predicates, projections
- [ ] Port the AST -> ASL planner: translate a resolved AQL AST into an ASL plan
- [ ] Port ASL rewrite/optimization passes (predicate pushdown, join elimination, self-join elimination leveraging PG 18)
- [ ] Port the ASL -> SQL generator for JSONB path extraction against `ehr.comp_data`/`_history`
- [ ] Port array unnesting logic for multi-valued RM structures within the generated SQL
- [ ] Port current+history UNION query generation for versioned-object queries
- [ ] Evaluate and where viable adopt `JSON_TABLE()` (PG 17+) for JSONB-to-relational projection in the generated SQL
- [ ] Port RESULT_SET assembly matching the openEHR RESULT_SET schema 1.0.3
- [ ] Write integration tests executing generated SQL against a `testcontainers` PostgreSQL 18 with seeded composition data
- [ ] Add PORT STATUS trailers referencing EHRbase's `AqlSqlLayer`/ASL Java classes as source

## Exit criteria

- [ ] A representative AQL query executes end-to-end (AST -> ASL -> SQL -> execute -> RESULT_SET) against seeded test data
- [ ] Current+history UNION queries return correct results for a versioned composition
- [ ] At least one query path uses `JSON_TABLE()` where PG 17+ makes it viable

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. This is the hardest single phase in the plan. Budget for it
accordingly: port the ASL IR and planner first (Phase A, no compile needed),
and don't attempt to validate against real SQL execution until the ASL -> SQL
generator is far enough along to produce syntactically valid output.
