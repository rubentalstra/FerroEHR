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

- [x] OPT 1.4 XML parser → typed model; persist to `template_store`. Delivered as
      a **codegen** path (no hand-writing spec types): a new `openehr-codegen`
      `emit-opt` target generates `openehr-its::opt14` (typed `OperationalTemplate`
      + constraint model + `ToXml`/`FromXml`) from the OPT XSD closure
      (`Template.xsd`→OpenehrProfile→Archetype→Resource→BaseTypes); RM leaves
      resolve to `openehr_rm`/`openehr_base`. **All 91 vendored `.opt` templates
      parse** (`openehr-its/tests/opt14_corpus.rs`). Wired into the drift gate.
- [ ] Runtime ODIN + ADL 1.4 / AOM 1.4 text parsers — **deferred** (not needed
      for OPT-1.4 *XML* ingestion; a text-parser subsystem for a later phase).
- [x] DEFINITION `adl1.4` endpoints (upload/list/get) via the service layer
      (`ehrbase::service::template` on `template_store`, mirroring `stored_query`);
      GET returns the stored OPT XML as `application/xml`. `adl2` stays 501.
- [x] Tests: e2e on PG 18 (`ehrbase/tests/service_template.rs`) — upload → list →
      get (byte-identical) → idempotent re-upload; 404 + 422 paths.

## Exit criteria

- [x] Reference OPT 1.4 XML templates upload, store, and retrieve
- [x] Parsed model is complete enough to drive P14/P15 (typed `opt14`
      `OperationalTemplate` + full C_* constraint tree; nested `C_ARCHETYPE_ROOT`
      handled). Scope notes: the differential/presentation envelope
      (`T_CONSTRAINT`/`T_VIEW`) + `EXPR_LEAF.item` are opaque `serde_json::Value`
      (not part of the operational definition).
- [x] Compiles + clippy-clean; deterministic regen (drift gate green)

## Decisions made this phase

- **AOM2/2.4 pivot considered, then re-grounded on CNF.** The modern AOM2 model
  (`am24`) is already BMM-generated and JSON-ready, but the ADL2 template *upload*
  wire is ADL2 **text** (a ~7–10 wk ODIN+cADL+EL parser) with **no** shipping
  corpus, and the openEHR **CNF platform conformance schedule REQUIRES OPT 1.4 XML**
  (via `adl1.4`; it gates the COMPOSITION/AQL suites) while **ADL2 is OPTIONAL and
  untested**. So OPT 1.4 XML is the CNF-required, corpus-validated Stage-1 target;
  it feeds the modern generated model (interop, not a legacy foundation).
- **Codegen, not hand-writing** (hard rule): the OPT types are generated from the
  official ITS-XML `Template.xsd` closure via `emit-opt` (self-contained: the XSD
  subtype graph gives nested `C_ARCHETYPE_ROOT` a variant by construction, which
  the BMM `am14` enums could not). `am14`/`am24` BMM crates stay the canonical
  models; `opt14` is a scoped legacy-OPT-XML wire adapter beside them.
- **ADL2 / OPT 2.0** deferred to a later optional modern-capability phase (the
  ADL2 text parser → `am24`, the differentiator EHRbase lacks).
