# Phase 10 — WebTemplate builder

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): ADL (Phase 09)
- Compile required: no (Phase A)

## Objectives

Port EHRbase's OptVisitor equivalent: build a WebTemplate from a flattened
OPT, matching Better's `web-template` semantics plus EHRbase's known quirks
(`|unit` vs SDT's `|units`). The WebTemplate is the structure the AQL path
analyzer (Phase 12) and composition validator (Phase 11) both query against.

## Preconditions

- [ ] Phase 09 done: OPT 1.4 XML parses into AOM 1.4

## Scope

In: WebTemplate object model, the OPT-to-WebTemplate builder (OptVisitor
equivalent), path-to-node indexing, EHRbase quirk compatibility (`|unit`).
Out: FLAT/STRUCTURED serialization of WebTemplate-shaped data (Phase 16),
Web Template JSON wire format serialization (also Phase 16 — this phase
builds the in-memory model only).

## Tasks

- [ ] Define the WebTemplate object model (node, input, annotation) matching Better's `web-template` semantics
- [ ] Port the OptVisitor equivalent: walk a flattened OPT and emit a WebTemplate tree
- [ ] Implement path-to-node indexing so a WebTemplate node can be looked up by AQL/archetype path in O(log n) or better
- [ ] Implement the EHRbase `|unit` quirk (vs SDT's `|units`) as an explicit compatibility shim, documented with a `// PORT NOTE:`
- [ ] Reference Better's `web-template-tests` corpus and select representative fixtures to port as Rust tests
- [ ] Write a test building a WebTemplate from a real OPT (from Phase 09's vendored archetypes) and asserting node count/paths
- [ ] Add PORT STATUS trailers referencing the EHRbase OptVisitor Java class as source

## Exit criteria

- [ ] WebTemplate builds successfully from at least one real flattened OPT
- [ ] Path-to-node lookup works for a representative set of archetype paths
- [ ] The `|unit` EHRbase quirk is implemented and tested against a quantity-bearing archetype

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. This phase receives EHRbase's OptVisitor-equivalent Java (part
of `service`, landed in `openehr-server` during Phase 00's `git mv`); port it
faithfully first, then cross-check node shapes against a `web-template-tests`
fixture before calling it done.
