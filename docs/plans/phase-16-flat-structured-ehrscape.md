# Phase 16 — FLAT / STRUCTURED / Web Template + EhrScape

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): flat, adl (Phases 10, 09)
- Compile required: no (Phase A)

## Objectives

Implement FLAT (simSDT) and STRUCTURED (structSDT) formats in `openehr-flat`
targeting Better's `web-template` semantics with EHRbase quirks accepted
(`|unit` vs SDT's `|units`), the Web Template JSON wire format, and port
EHRbase's EhrScape endpoints (`/rest/ecis/v1/*`) into `openehr-ehrbase-compat`.

## Preconditions

- [ ] Phase 10 done: WebTemplate available as the structural source
- [ ] Phase 11 done: `Validate` trait available for FLAT/STRUCTURED input validation
- [ ] Phase 15 done: service layer available for EhrScape to call into

## Scope

In: FLAT (simSDT) ser/de, STRUCTURED (structSDT) ser/de, Web Template JSON
wire-format serialization, MIME type routing (`application/openehr.wt+json`,
`.wt.flat+json`, `.wt.structured+json`), EhrScape endpoint ports, admin API
completion, WebTemplate export endpoint.
Out: the WebTemplate in-memory model itself (Phase 10 owns that; this phase
only serializes FLAT/STRUCTURED data against it), the Matrix format (not on
the critical path; add later if needed).

## Tasks

- [ ] Implement FLAT (simSDT) serialization: WebTemplate-driven flattening of an RM object graph into dot-path-keyed JSON
- [ ] Implement FLAT (simSDT) deserialization: reconstruct an RM object graph from dot-path-keyed JSON against a WebTemplate
- [ ] Implement STRUCTURED (structSDT) ser/de following the same WebTemplate-driven approach with structSDT's nesting rules
- [ ] Implement the EHRbase `|unit` vs SDT `|units` quirk consistently across both formats, gated by an `ehrbase-quirks` feature
- [ ] Implement Web Template JSON wire-format serialization (the WebTemplate model itself, exported over HTTP)
- [ ] Wire MIME type routing for `application/openehr.wt+json`, `application/openehr.wt.flat+json`, `application/openehr.wt.structured+json` in the REST layer
- [ ] Port EhrScape endpoints (`/rest/ecis/v1/*`) into `openehr-ehrbase-compat`, calling into the Phase 15 service layer
- [ ] Complete the admin API and WebTemplate export endpoint stubbed in Phase 06
- [ ] Write round-trip tests against Better's `web-template-tests` fixtures for both FLAT and STRUCTURED
- [ ] Add PORT STATUS trailers referencing EHRbase's EhrScape/Flat/Structured Java classes as source

## Exit criteria

- [ ] FLAT and STRUCTURED both round-trip a representative composition against its WebTemplate
- [ ] MIME type routing correctly dispatches to the right serializer
- [ ] EhrScape endpoints are callable end-to-end through `openehr-ehrbase-compat`

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. Pull fixtures directly from `web-template-tests` rather than
inventing new ones — EHRbase's own quirks only show up as deltas against that
corpus, and the deltas are exactly what needs a `// PORT NOTE:`.
