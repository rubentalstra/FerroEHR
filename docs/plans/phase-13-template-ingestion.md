# Phase 13 — Template ingestion (ADL / AOM / OPT 1.4 XML)

- Status: not-started (Stage-1 app build, step 5 of 13)
- Consumes: `openehr-am` (generated AOM types), `openehr-lang` (ODIN)
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-006

## Objectives

Ingest operational templates so the server can validate compositions and build
WebTemplates. EHRbase's template ingestion is **OPT 1.4 XML** first, so that is
the priority: parse OPT 1.4 XML into the generated `openehr-am` (`am14`) types
and persist to `template_store`. ADL 1.4 / AOM 1.4 text parsing (and ADL2 behind
a flag) follow.

## Preconditions

- [ ] `openehr-am` generated (done); `openehr-lang` ODIN reader (done)
- [ ] P09 (`template_store` table)

## Scope

**In:** OPT 1.4 **XML** parser (via `openehr-its`/`quick-xml`) → `openehr-am`
`am14` OPT model → `template_store`; the runtime ODIN + ADL 1.4 / AOM 1.4 text
parsers (`openehr-am` parser modules consuming `openehr-lang` ODIN); template
upload/list/get endpoints (DEFINITION API, wired via P11/P12).
**Out:** WebTemplate construction (P14); validation (P15); ADL2/AOM2 (feature-
gated, not on the parity path).

## Tasks

- [ ] OPT 1.4 XML parser → `openehr-am` model; persist to `template_store`
- [ ] Runtime ODIN + ADL 1.4 / AOM 1.4 text parsers
- [ ] DEFINITION endpoints (`/definition/template/adl1.4`) via the service layer
- [ ] Tests: ingest reference OPTs (openEHR/CKM samples) round-trip

## Exit criteria

- [ ] Reference OPT 1.4 XML templates upload, store, and retrieve
- [ ] Parsed model is complete enough to drive P14/P15
- [ ] Compiles + clippy-clean

## Decisions made this phase

- OPT 1.4 XML is the Stage-1 ingestion target (EHRbase parity); ADL2 deferred.
