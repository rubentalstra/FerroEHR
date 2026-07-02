# Phase 03 — RM transcription

- Status: done
- Started: 2026-07-02   Completed: 2026-07-02   Owner: Ruben
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

- [x] Transcribe rm.data_types Basic (`DV_BOOLEAN`, `DV_STATE`, `DV_IDENTIFIER`) and Text (`DV_TEXT`, `DV_CODED_TEXT`, `CODE_PHRASE`, `TERM_MAPPING`, `DV_PARAGRAPH`) — CODE_PHRASE has no Inherit row (standalone leaf); DvText triple for DV_PARAGRAPH substitutability; MatchKind value-enum
- [x] Transcribe rm.data_types Quantity subtree (`DV_ORDERED` through `DV_PROPORTION`, `DV_INTERVAL<T>`, `REFERENCE_RANGE<T>`, `PROPORTION_KIND`), encoding the `DV_COUNT.magnitude` covariant redefinition — F-bounded DvOrderedData; i64-vs-OrderedNumeric bound conflict flagged for P17
- [x] Transcribe rm.data_types Date_time, Time_specification, Encapsulated, and URI subtrees, boxing `DV_MULTIMEDIA.thumbnail` for recursion — Option<Box<DvMultimedia>>; dual-inheritance decision rule recorded (embed foundation parent only when RM functions delegate to it)
- [x] Transcribe rm.data_structures (`DATA_STRUCTURE`, `ITEM_STRUCTURE` family, `ITEM`/`CLUSTER`/`ELEMENT`, `HISTORY<T>`/`EVENT<T>` family), boxing `CLUSTER` and `ITEM_TREE` recursion and encoding `ITEM_STRUCTURE.as_hierarchy()` covariance — recursion via Vec<Item> (heap indirection, documented); function-level covariance = widened trait method + narrowed inherent override
- [x] Transcribe rm.common: `PATHABLE`/`LOCATABLE`/`ARCHETYPED`/`LINK`/`FEEDER_AUDIT*` using `Weak`/index for `PATHABLE.parent()`, never an owning back-reference — parent() -> Option<Weak<dyn PathableApi>>; LocatableData is the crate-wide embedding template
- [x] Transcribe rm.common: `PARTY_PROXY` family, `PARTICIPATION`, `AUDIT_DETAILS`, `ATTESTATION`, `REVISION_HISTORY*`, `VERSIONED_OBJECT<T>`/`VERSION<T>`/`ORIGINAL_VERSION<T>`/`IMPORTED_VERSION<T>`, `CONTRIBUTION`, `FOLDER` (boxed recursion) — Version<T> closed enum; FOLDER recursion via Vec (documented); ITEM_TAG from the tags chapter included
- [x] Transcribe rm.ehr: `EHR`, `EHR_STATUS`, `EHR_ACCESS`, `COMPOSITION`, `EVENT_CONTEXT`, `CONTENT_ITEM`/`SECTION`/`ENTRY` family, `OBSERVATION`/`EVALUATION`/`INSTRUCTION`/`ACTION`, `ACTIVITY`, `INSTRUCTION_DETAILS`, `ISM_TRANSITION` — noting `EVENT_CONTEXT`, `INSTRUCTION_DETAILS`, `ISM_TRANSITION` inherit `PATHABLE` not `LOCATABLE` — hazard applied; EHR itself has NO Inherit row (flagged); VERSIONED_X as newtype wrappers
- [x] Transcribe rm.demographic: `PARTY`/`ROLE`/`ACTOR`/`PERSON`/`ORGANISATION`/`GROUP`/`AGENT`, `PARTY_RELATIONSHIP`, `PARTY_IDENTITY`, `CONTACT`, `ADDRESS`, `CAPABILITY`, plus the versioned binding — two-level nested enums mirror the Inherit chain; X_VERSIONED_PARTY omitted (not in the chapter's include list)
- [x] Transcribe rm.integration `GENERIC_ENTRY`; confirm `rm.ehr_extract` stays out of scope (feature-gate stub only, no content) — GenericEntry participates in the ContentItem enum; ehr_extract untouched
- [x] Encode closed-enum decisions for `DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`, `VERSION<T>` per Section 7.2 — all five verified by grep; PATHABLE.parent() is the one documented open-trait-object exception
- [x] Add PORT STATUS trailers to every transcribed file; update `docs/ROSETTA.md` with each spec-class -> Rust-type mapping — 107/107 trailers verified; ~30 new ROSETTA rows

## Exit criteria

- [x] Every class in Section 7.1's rm.* inventory (minus `rm.ehr_extract`) has a corresponding Rust type in `openehr-rm` — 107 files across data_types(34)/data_structures(13)/common(23)/ehr(20)/demographic(13)/integration(1)/support(3)
- [x] All five structural hazards from Section 7.2 are applied consistently across the crate (checked by grep for `Weak<`, `Box<`, enum definitions) — 23 Weak, 15 Box, 14 closed enums incl. the required five
- [x] `docs/ROSETTA.md` has one row per transcribed RM class — package-level + notable-class rows appended (~30)

## Decisions made this phase

- PATHABLE.parent() -> Option<Weak<dyn PathableApi>>: the single deliberate
  open-trait-object exception to ADR-001 §4 (implementor set spans every RM
  package); Weak-not-owning per the settled hazard. LocatableData +
  LocatableApi: PathableApi is the embedding template everywhere; the
  Weak<dyn> upcast seam is flagged for the first concrete impl at P17.
- F-bounded generics for self-referential abstract state:
  DvOrderedData<T: DvOrderedApi> because each leaf's normal_range narrows
  to Self per its own (redefined) table rows — extends ADR-001 §3/§5.
- Dual RM+foundation inheritance decision rule: embed the foundation parent
  only when the RM class's functions delegate to its methods (DV_DURATION's
  magnitude() uses Iso8601_duration::to_seconds()); a value-string-only
  mixin becomes `value: String` directly (DV_DATE/TIME/DATE_TIME).
- Recursion through Vec needs no extra Box (CLUSTER.items, SECTION.items,
  FOLDER.folders); bare 0..1 self-reference does (DV_MULTIMEDIA.thumbnail:
  Option<Box<..>>).
- Function-level covariance (as_hierarchy()) = widened trait method +
  narrowed inherent override per concrete type.
- Closed VALUE domains become enums too (TERM_MAPPING.match -> MatchKind;
  PROPORTION_KIND repr(i32) with spec discriminants).
- Known deferred conflicts, all flagged in-file for P17: DV_COUNT.magnitude
  i64 vs OrderedNumeric bound; EXTERNAL_ENVIRONMENT_ACCESS trait/struct MI
  mismatch with P2's concrete TerminologyService; CODE_PHRASE three-way
  TerminologyCode reconciliation; Weak<dyn> upcast.
- Spec-text defects flagged, never silently resolved: EHR has no Inherit
  row; ITEM_TABLE's six table ambiguities (low confidence); REVISION_HISTORY
  ordering contradiction (resolved toward most-recent-last per both function
  postconditions); DV_STATE table-vs-prose; less_than inverted
  postconditions in the date_time cluster.

## Handoff for next session

P3 complete: 107 RM classes transcribed by nine parallel rm-transcriber
runs, integrated, sweep-verified (rustfmt clean, trailers 107/107, 328
TODO(port), 104 PORT NOTE, hazards grep-verified), committed in four package
commits. One agent stalled at the very end (ehr composition cluster) but its
14 files were complete and verified; only a trailer count needed fixing.
Next is P4 (docs/plans/phase-04-serialization-json.md): canonical JSON in
openehr-serde — serde derives finally land there; the TYPE_NAME consts
across base/rm become #[serde(rename)] attributes, and insta golden vectors
against ITS-JSON (pin a commit) become the acceptance instrument.
