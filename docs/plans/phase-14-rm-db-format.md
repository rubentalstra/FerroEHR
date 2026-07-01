# Phase 14 — rm-db-format: RM <-> row-per-locatable bridge

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): RM (Phase 03), persistence (Phase 07)
- Compile required: no (Phase A)

## Objectives

Port EHRbase's `rm-db-format` module: the bridge that decomposes an in-memory
RM object graph into row-per-locatable storage (leaf-attribute JSONB in
`ehr.comp_data`/`_history`) on write, and reassembles the RM object graph from
those rows on read.

## Preconditions

- [ ] Phase 03 done: RM object graph types exist to decompose/reassemble
- [ ] Phase 07 done: `ehr.comp_data`/`_history` schema exists

## Scope

In: RM-to-row decomposition (write path), row-to-RM reassembly (read path),
leaf-attribute JSONB encoding, `comp_version` linkage.
Out: AQL-time row access (Phase 13 reads the same tables via generated SQL,
independently), versioning/trigger orchestration (Phase 15 owns the service-
layer transaction wrapping).

## Tasks

- [ ] Port the RM-to-row decomposition walker: traverse a `COMPOSITION` (or other LOCATABLE root) and emit one row per locatable node
- [ ] Implement leaf-attribute JSONB encoding matching EHRbase's `ehr.comp_data` column shape
- [ ] Port the row-to-RM reassembly walker: given a set of rows for one composition, rebuild the RM object graph including containment (FOLDER/CLUSTER/ITEM_TREE nesting)
- [ ] Implement `comp_version` linkage: associate decomposed rows with their owning version
- [ ] Implement `_history` row handling: write to history on update, read from current or history depending on query context
- [ ] Write a round-trip test: decompose a real composition (from Phase 09's vendored archetypes/Phase 03 fixtures) into rows, reassemble, and assert structural equality
- [ ] Write a test proving history rows are produced correctly across a two-version update sequence
- [ ] Add PORT STATUS trailers referencing EHRbase's `rm-db-format` Java classes as source

## Exit criteria

- [ ] A representative composition round-trips through decomposition and reassembly with structural equality
- [ ] History-row generation is correct across at least a two-version update
- [ ] `comp_version` linkage correctly associates rows with their version

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. This phase and Phase 13 both read `ehr.comp_data`/`_history`;
keep the JSONB leaf-encoding format identical between the two so Phase 13's
generated SQL can extract values this phase's writer actually produces —
cross-check column shape against Phase 07's `sea-query` definitions before
diverging from EHRbase's Java encoding.
