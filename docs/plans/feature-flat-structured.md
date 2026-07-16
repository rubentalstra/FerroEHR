# Feature — FLAT + STRUCTURED interop depth (+ EhrScape compatibility)

*Renamed from the old "phase 17" file (owner, 2026-07-16): this is a feature
track, not a build phase.* Two halves:

1. **SIM-B/SDF transformation-rule audit** — verify the FLAT (simSDT) /
   STRUCTURED (structSDT) converters in `openehr-flat` against the SM
   simplified-format spec tables (`docs/specs/openehr/SM/docs/simplified_im_b/`
   §Transformation Rules + the `ctx/` vocabulary), accept SDF-normative leaf
   encodings, and document any deliberate quantity-encoding divergence.
   Interop quality — not conformance-gated.
2. **EhrScape (`/rest/ecis/v1/*`) + admin compatibility endpoints** in
   `ehrbase-rest` (feature-gated `ehrscape` module), reusing the existing
   `openehr-flat` converters.

> App-crate reality (2026-07-16): the application is `app/{ehrbase,
> ehrbase-rest, ehrbase-server}`; the REST adapter calls the concrete
> `EhrbaseService` directly. EhrScape lands as the `ehrbase-rest::ehrscape`
> feature module.

- Status: not-started
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
