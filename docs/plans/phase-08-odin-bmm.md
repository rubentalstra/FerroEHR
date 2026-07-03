# Phase 08 — ODIN + BMM parsers

> **⚠️ ADR-004 (2026-07-03):** this phase's openEHR spec layer is now GENERATED from the BMM meta-model by `openehr-codegen`, not hand-transcribed. Read `docs/ADRs/ADR-004-spec-driven-codegen.md` and the "Code generation" section of `CLAUDE.md`. Tasks/notes below describe the superseded hand-transcription approach.

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): LANG 1.0.0 (BMM schema v2.3) / Layer 6a
- Compile required: no (Phase A)

## Objectives

Implement a native Rust ODIN parser (`openehr-odin`) and BMM object model +
P_BMM parser (`openehr-bmm`), both consumed by the ADL/AOM layer in Phase 09.
Neither crate has a Java counterpart — EHRbase pulled these from `archie`.

## Preconditions

- [ ] Phase 00 done: `openehr-odin` and `openehr-bmm` crate skeletons exist

## Scope

In: ODIN lexer/parser (`logos` + `chumsky`) against the canonical
`specifications-BASE/computable/grammar/odin.g4`, the BMM object model
(schema v2.3), P_BMM parsing.
Out: ADL/AOM proper (Phase 09), any BMM instance beyond what P_BMM needs to
represent the RM's own BMM instance for validation.

## Tasks

- [ ] Reimplement the ODIN grammar (`odin.g4`) as a `logos` lexer for `openehr-odin`
- [ ] Implement the ODIN parser (`chumsky`) producing an ODIN value AST (primitive, list, object, keyed object)
- [ ] Write property-based round-trip tests (parse -> print -> parse -> equal) for the ODIN parser
- [ ] Define the BMM object model (schema v2.3): package, class, property, generic parameter definitions
- [ ] Implement the P_BMM parser (parsed BMM instance, pre-flattening) on top of the ODIN parser
- [ ] Write a golden-vector test parsing the RM's own BMM schema instance (if available) or a representative fragment
- [ ] Wire `miette`/`ariadne` diagnostics for parse errors in both crates
- [ ] Add PORT STATUS trailers noting these are spec transcriptions with no Java source

## Exit criteria

- [ ] `openehr-odin` parses a representative ODIN document and round-trips it
- [ ] `openehr-bmm` parses a representative BMM schema fragment into the object model
- [ ] Parse errors produce readable diagnostics via `miette` or `ariadne`

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. `chumsky` 0.10 is the pinned parser combinator (1.0 is still
alpha); confirm its error-recovery ergonomics against a deliberately malformed
ODIN fragment before committing the whole grammar to one combinator style.
