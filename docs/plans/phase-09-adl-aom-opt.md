# Phase 09 — ADL 1.4 + AOM 1.4 + OPT 1.4 XML

> **⚠️ ADR-004 (2026-07-03):** this phase's openEHR spec layer is now GENERATED from the BMM meta-model by `openehr-codegen`, not hand-transcribed. Read `docs/ADRs/ADR-004-spec-driven-codegen.md` and the "Code generation" section of `CLAUDE.md`. Tasks/notes below describe the superseded hand-transcription approach.

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): AM 2.3.0, ODIN, BMM / Layer 6b
- Compile required: no (Phase A)

## Objectives

Implement the ADL 1.4 parser, AOM 1.4 object model, and OPT 1.4 XML parsing —
the EHRbase-compatible slice, since EHRbase v2.31.0's template ingestion is
still ADL 1.4 / OPT 1.4 XML. Build ADL 2 / AOM 2 in parallel behind the `adl2`
feature flag, deferrable if time-constrained.

## Preconditions

- [ ] Phase 08 done: ODIN and BMM parsers available
- [ ] Phase 03 done: RM types available for AOM constraint targets
- [ ] Phase 05 done: XML infrastructure (`quick-xml` patterns) available for OPT 1.4 XML

## Scope

In: ADL 1.4 lexer/parser, AOM 1.4 object model (archetype, c_object, c_attribute,
c_complex_object, etc.), OPT 1.4 XML parsing against AM 1.4 OPT XSDs.
Out: OPT 2 flattener (AM 2.3.0 marks it still in dev — defer), ADL 2/AOM 2
full implementation (build behind `adl2` feature but do not block this phase's
exit criteria on it).

## Tasks

- [ ] Reimplement the ADL 1.4 grammar (`reader_adl14/src/main/antlr/*.g4`) as a `logos` + `chumsky` parser in `openehr-adl`
- [ ] Define the AOM 1.4 object model: `ARCHETYPE`, `C_OBJECT`, `C_ATTRIBUTE`, `C_COMPLEX_OBJECT`, `C_PRIMITIVE_OBJECT`, `ARCHETYPE_SLOT`, `C_ARCHETYPE_ROOT`
- [ ] Implement ADL 1.4 archetype identifier and ontology-section parsing (term definitions, constraint bindings)
- [ ] Vendor AM 1.4 OPT XSDs from `specifications-ITS-XML/components/AM/Release-1.4/`
- [ ] Implement OPT 1.4 XML parsing into the AOM 1.4 object model using `quick-xml`
- [ ] Write a round-trip test: parse a representative `.adl` archetype file into AOM 1.4, then into a flattened OPT-equivalent structure
- [ ] Behind the `adl2` feature flag: stub the ADL 2 grammar and AOM 2 object model entry points (implementation can lag)
- [ ] Add PORT STATUS trailers; update `docs/ROSETTA.md` with ADL grammar -> Rust AST mappings

## Exit criteria

- [ ] ADL 1.4 parser successfully parses at least one real openEHR reference archetype
- [ ] AOM 1.4 object model round-trips a parsed archetype into OPT 1.4 XML matching the vendored XSD
- [ ] `adl2` feature flag exists and compiles (even if its implementation is a stub)

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. Fetch a handful of real reference archetypes (openEHR CKM or
EHRbase's own `test-data`) before starting the grammar work, so the parser is
validated against real input from the first test, not synthetic fragments.
