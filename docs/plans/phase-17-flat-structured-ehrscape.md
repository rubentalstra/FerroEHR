# Phase 17 — FLAT / STRUCTURED / Web Template + EhrScape

- Status: not-started (Stage-1 app build, step 9 of 13)
- Consumes: `openehr-rm`, P14 (WebTemplate), P15 (validation), P12 (service)
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-006; serialization rule (Better semantics + `ehrbase-quirks` flag)

## Objectives

The vendor formats and the EhrScape compatibility surface: FLAT (simSDT),
STRUCTURED (structSDT), and Web Template JSON conversion in `openehr-flat`, plus
the EhrScape (`/rest/ecis/v1/*`) + admin endpoints in `ehrbase-compat`. Target
Better's `web-template` semantics; EHRbase quirks (`|unit` vs `|units`) live
behind the `ehrbase-quirks` feature flag.

## Preconditions

- [ ] P14 (WebTemplate), P15 (validation), P12 (service layer)

## Scope

**In:** FLAT ↔ RM and STRUCTURED ↔ RM conversion driven by the WebTemplate
(`openehr-flat`); MIME types (`application/openehr.wt.flat+json`,
`…wt.structured+json`); EhrScape endpoints + admin API (`ehrbase-compat`);
`ehrbase-quirks` flag. **Out:** anything AQL (P16); the canonical JSON/XML
formats (done, `openehr-its`).

## Tasks

- [ ] FLAT (simSDT) ↔ RM via WebTemplate (`openehr-flat`)
- [ ] STRUCTURED (structSDT) ↔ RM
- [ ] EhrScape + admin endpoints (`ehrbase-compat`), MIME negotiation
- [ ] `ehrbase-quirks` flag; tests vs Better `web-template-tests`

## Exit criteria

- [ ] FLAT and STRUCTURED round-trip against Better's vectors
- [ ] EhrScape create/get composition (flat) works end to end
- [ ] Compiles + clippy-clean

## Decisions made this phase

- Better `web-template` semantics are the oracle; EHRbase-specific quirks are
  opt-in via the feature flag, never in the default path.
