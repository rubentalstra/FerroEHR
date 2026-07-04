# Phase 14 — WebTemplate builder

- Status: not-started (Stage-1 app build, step 6 of 13)
- Consumes: `openehr-am` (OPT model, P13), `openehr-term`
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-006

## Objectives

Build the **WebTemplate** (the flattened, UI/validation-oriented view of an OPT)
from an ingested operational template — EHRbase's `OptVisitor` equivalent. The
WebTemplate drives composition validation (P15), the FLAT/STRUCTURED formats
(P17), and AQL semantic path analysis (P16). Cache built WebTemplates with
`moka`.

## Preconditions

- [ ] P13 (OPT ingested into `openehr-am`)

## Scope

**In:** OPT → WebTemplate node tree (paths, rm types, cardinalities, terminology
bindings, inputs); `moka` cache keyed by template id; WebTemplate JSON export
(`application/openehr.wt+json`); Better `web-template` semantics as the oracle.
**Out:** validation logic (P15); FLAT/STRUCTURED conversion (P17).

## Tasks

- [ ] WebTemplate model + OPT→WebTemplate builder
- [ ] `moka` cache
- [ ] WebTemplate JSON export endpoint
- [ ] Tests vs Better `web-template-tests` vectors

## Exit criteria

- [ ] Reference OPTs produce WebTemplates matching Better's vectors (insta)
- [ ] Cache hit path verified
- [ ] Compiles + clippy-clean

## Decisions made this phase

- Better `web-template` semantics are the oracle; EHRbase quirks behind the
  `ehrbase-quirks` flag on `openehr-flat` (P17).
