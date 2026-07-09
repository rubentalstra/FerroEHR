# Phase 17 — EhrScape + admin compatibility surface

> **Rescoped (2026-07-06):** WebTemplate + FLAT (simSDT) + STRUCTURED (structSDT)
> conversion moved into **P14** (built there, full Better parity). P17 now covers
> only the **EhrScape (`/rest/ecis/v1/*`) + admin** compatibility endpoints in
> `ehrbase-rest` (feature-gated `ehrscape` module — the `ehrbase-compat` crate was deleted 2026-07-09, ADR-010), reusing the `openehr-flat` converters P14 delivered.
>
> **App-crate reality (ADR-011, 2026-07-09):** the application is three crates —
> `app/{ehrbase, ehrbase-rest, ehrbase-sm}` (`ehrbase-audit`/`ehrbase-signing`
> dissolved into `ehrbase::{system_log, signing}`; authz → `ehrbase-rest::access`).
> EhrScape is the `ehrbase-rest::ehrscape` feature module.

- Status: not-started (Stage-1 app build, step 9 of 13)
- Consumes: `openehr-flat` (WebTemplate/FLAT/STRUCTURED, P14), P15 (validation), P12 (service)
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-006; serialization rule (Better semantics + `ehrbase-quirks` flag)

## Objectives

The EhrScape compatibility surface: the EhrScape (`/rest/ecis/v1/*`) + admin
endpoints in `ehrbase-rest`'s feature-gated `ehrscape` module, reusing the `openehr-flat` FLAT/STRUCTURED/
WebTemplate conversion built in P14. Target Better's `web-template` semantics;
`|unit` is singular (no `|units` divergence — see the serialization rule);
genuine Better extras (`|unit_system`/`|unit_display_name`) live behind the
`ehrbase-quirks` feature flag. WebTemplate/FLAT/STRUCTURED are a compat layer,
not CNF-conformance-gated.

## Preconditions

- [ ] P14 (WebTemplate), P15 (validation), P12 (service layer)

## Scope

**In:** FLAT ↔ RM and STRUCTURED ↔ RM conversion driven by the WebTemplate
(`openehr-flat`); MIME types (`application/openehr.wt.flat+json`,
`…wt.structured+json`); EhrScape endpoints + admin API (`ehrbase-rest::ehrscape`, feature-gated);
`ehrbase-quirks` flag. **Out:** anything AQL (P16); the canonical JSON/XML
formats (done, `openehr-its`).

## Tasks

- [ ] FLAT (simSDT) ↔ RM via WebTemplate (`openehr-flat`)
- [ ] STRUCTURED (structSDT) ↔ RM
- [ ] EhrScape + admin endpoints (`ehrbase-rest::ehrscape`, feature-gated), MIME negotiation
- [ ] `ehrbase-quirks` flag; tests vs Better `web-template-tests`

## Exit criteria

- [ ] FLAT and STRUCTURED round-trip against Better's vectors
- [ ] EhrScape create/get composition (flat) works end to end
- [ ] Compiles + clippy-clean

## Decisions made this phase

- Better `web-template` semantics are the oracle; EHRbase-specific quirks are
  opt-in via the feature flag, never in the default path.

## Notes — EhrScape wire quirks (folded from docs/design/ehrscape/, 2026-07-09)

`PUT /rest/ecis/v1/ehr/{ehrId}/status` — with the EHR id as a path variable,
the request body is the EhrScape EHR_STATUS shape, e.g.:

```json
{
    "subjectId": "TEST",
    "subjectNamespace": "TEST",
    "queryable": true,
    "modifiable": true,
    "otherDetails": null,
    "otherDetailsTemplateId": null
}
```

Quirk to preserve: the `Content-Type` header is overridden to express the
body's type, due to a problematic definition in the EhrScape standard.
