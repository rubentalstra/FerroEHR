# Phase 03 — RM transcription

- Status: in-progress
- Started: 2026-07-02   Owner: Ruben
- Consumes (spec/layer): RM 1.1.0 / Layer 4
- Compile required: no (Phase A)

## Objectives

Literally transcribe the ~108-class openEHR Reference Model into `openehr-rm`,
in dependency order: data_types, then data_structures, then common, then ehr,
then demographic, then integration. This is the single largest transcription
phase and sits on the critical path (Section 6).

## Preconditions

- [x] Phase 01 done: BASE identification types available
- [x] Phase 02 done: terminology service available for `CODE_PHRASE`-bearing types

## Scope

In: all of Section 7.1's rm.data_types (27), rm.data_structures (12),
rm.common (~22), rm.ehr (20), rm.demographic (14), rm.integration (1)
(`GENERIC_ENTRY`).
Out: `rm.ehr_extract` (experimental, deferred indefinitely — feature-gate as
`ehr-extract` if ever built), canonical JSON/XML serialization (Phases 04-05),
composition validation (Phase 11).

## Tasks

- [ ] Transcribe rm.data_types Basic (`DV_BOOLEAN`, `DV_STATE`, `DV_IDENTIFIER`) and Text (`DV_TEXT`, `DV_CODED_TEXT`, `CODE_PHRASE`, `TERM_MAPPING`, `DV_PARAGRAPH`)
- [ ] Transcribe rm.data_types Quantity subtree (`DV_ORDERED` through `DV_PROPORTION`, `DV_INTERVAL<T>`, `REFERENCE_RANGE<T>`, `PROPORTION_KIND`), encoding the `DV_COUNT.magnitude` covariant redefinition
- [ ] Transcribe rm.data_types Date_time, Time_specification, Encapsulated, and URI subtrees, boxing `DV_MULTIMEDIA.thumbnail` for recursion
- [ ] Transcribe rm.data_structures (`DATA_STRUCTURE`, `ITEM_STRUCTURE` family, `ITEM`/`CLUSTER`/`ELEMENT`, `HISTORY<T>`/`EVENT<T>` family), boxing `CLUSTER` and `ITEM_TREE` recursion and encoding `ITEM_STRUCTURE.as_hierarchy()` covariance
- [ ] Transcribe rm.common: `PATHABLE`/`LOCATABLE`/`ARCHETYPED`/`LINK`/`FEEDER_AUDIT*` using `Weak`/index for `PATHABLE.parent()`, never an owning back-reference
- [ ] Transcribe rm.common: `PARTY_PROXY` family, `PARTICIPATION`, `AUDIT_DETAILS`, `ATTESTATION`, `REVISION_HISTORY*`, `VERSIONED_OBJECT<T>`/`VERSION<T>`/`ORIGINAL_VERSION<T>`/`IMPORTED_VERSION<T>`, `CONTRIBUTION`, `FOLDER` (boxed recursion)
- [ ] Transcribe rm.ehr: `EHR`, `EHR_STATUS`, `EHR_ACCESS`, `COMPOSITION`, `EVENT_CONTEXT`, `CONTENT_ITEM`/`SECTION`/`ENTRY` family, `OBSERVATION`/`EVALUATION`/`INSTRUCTION`/`ACTION`, `ACTIVITY`, `INSTRUCTION_DETAILS`, `ISM_TRANSITION` — noting `EVENT_CONTEXT`, `INSTRUCTION_DETAILS`, `ISM_TRANSITION` inherit `PATHABLE` not `LOCATABLE`
- [ ] Transcribe rm.demographic: `PARTY`/`ROLE`/`ACTOR`/`PERSON`/`ORGANISATION`/`GROUP`/`AGENT`, `PARTY_RELATIONSHIP`, `PARTY_IDENTITY`, `CONTACT`, `ADDRESS`, `CAPABILITY`, plus the versioned binding
- [ ] Transcribe rm.integration `GENERIC_ENTRY`; confirm `rm.ehr_extract` stays out of scope (feature-gate stub only, no content)
- [ ] Encode closed-enum decisions for `DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`, `VERSION<T>` per Section 7.2
- [ ] Add PORT STATUS trailers to every transcribed file; update `docs/ROSETTA.md` with each spec-class -> Rust-type mapping

## Exit criteria

- [ ] Every class in Section 7.1's rm.* inventory (minus `rm.ehr_extract`) has a corresponding Rust type in `openehr-rm`
- [ ] All five structural hazards from Section 7.2 are applied consistently across the crate (checked by grep for `Weak<`, `Box<`, enum definitions)
- [ ] `docs/ROSETTA.md` has one row per transcribed RM class

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. This is the largest single transcription phase; consider
delegating per-subtree work to the `rm-transcriber` subagent to avoid burning
context on repetitive class-by-class transcription, keeping this session free
for the structural decisions (enum boundaries, `PATHABLE.parent()` wiring).
