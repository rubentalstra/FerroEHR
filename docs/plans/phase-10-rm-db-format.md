# Phase 10 — rm-db-format (RM ↔ JSONB row-per-locatable)

- Status: not-started (Stage-1 app build, step 2 of 13)
- Consumes: `openehr-rm`, P09 (schema + `sea-query` tables)
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-006 (follow EHRbase's `rm-db-format` approach, idiomatic Rust)

## Objectives

The bridge between an in-memory RM object graph (`openehr-rm` types) and the
decomposed **row-per-locatable** storage EHRbase uses: decompose a `COMPOSITION`
(and `EHR_STATUS`, `FOLDER`) into `comp_data` rows with leaf-attribute JSONB, and
reassemble the RM object graph from those rows. This is bespoke openEHR-server
logic (no crate provides it), written idiomatically **following EHRbase's
rm-db-format algorithm** (`crates/ehrbase/src/rm_db_format/` Java is the
reference).

## Preconditions

- [ ] P09 done (tables + pool)

## Scope

**In:** decompose/recompose between `openehr-rm` graphs and the `comp_data`/
`_history` row model; leaf JSONB encoding (reuse `openehr-its::json` canonical
encoding — do not invent a second JSON shape); entity/path indexing needed for
reassembly and AQL. **Out:** transaction/versioning orchestration (P12); the
AQL SQL generator (P16, which reads this row layout).

## Tasks

- [ ] Decomposition walker: RM graph → `comp_data` rows (path, entity, leaf JSONB)
- [ ] Reassembly: rows → RM graph (`openehr-rm` types) via canonical JSON
- [ ] Round-trip property test (RM → rows → RM equal) over the corpus
- [ ] Confidence check against EHRbase's decomposition for representative comps

## Exit criteria

- [ ] Corpus compositions decompose and reassemble losslessly (proptest/insta)
- [ ] Row shape matches what the AQL engine (P16) will query
- [ ] Compiles + clippy-clean

## Decisions made this phase

- Leaf JSONB uses the canonical `openehr-its` encoding (single source of truth).
