# ITS-REST — per-specification compliance design register

Owner directive 2026-07-12: before the complete `ehrbase-rest` rewrite
(W-3e), audit the whole ITS-REST **development edition** (`e8a093e9` — the
same commit as the vendored OAS and the B5 conformance identity)
specification-by-specification, one design/gap document each — the same
method that drove the `ehrbase-sm` rewrite
([`../sm-platform/`](../sm-platform/README.md)). **SMART is in scope as an
implementation target**, not just a record.

**Total redesign mandate (owner directive 2026-07-12):** the register is not
only a gap list — the `ehrbase-rest` crate gets a **complete redesign with a
proper per-spec folder structure**, exactly like the `ehrbase-sm` rewrite,
because the current layout (flat `dispatch/` grab-bag, 1,000-line files,
protocol logic scattered across the root) is unmaintainable. Target
structure (authored fresh from the spec text, never migrated):

```
app/ehrbase-rest/src/
├── lib.rs           crate map + wire-oracle precedence
├── overview/        the Overview spec (negotiate, committal, headers,
│                    version_id, params, error — one file per protocol area)
├── api/
│   ├── system/      the System API (OPTIONS / conformance manifest)
│   ├── ehr/         the EHR API (one module per resource: ehr, ehr_status,
│   │                versioned_ehr_status, composition, versioned_composition,
│   │                directory, contribution)
│   ├── query/       the Query API (adhoc, stored, response)
│   ├── definition/  the Definition API (template_adl14, template_adl2,
│   │                stored_query)
│   ├── demographic/ the Demographic API (spec-governed now, not analogy)
│   └── admin/       the Admin API (physical EHR delete ops)
├── formats/         the Formats spec wire (flat/structured media types)
├── smart/           SMART App Launch (discovery, registration, launch,
│                    scopes — NEW implementation)
├── extensions/      EVERYTHING spec-silent, quarantined + flagged (access/
│                    authn+authz, management, EhrScape, events, fhir,
│                    tenancy middleware, item-tag extras, openapi serving)
└── router.rs        assembly over the openehr-its generated route tables
```

Every file cites its governing spec section; spec-silent code moves to
`extensions/` with the explicit flag; oversized files are broken down along
the spec's own resource boundaries.

**Oracle:** `docs/specs/openehr/ITS-REST/` — prose under
`specifications/docs/<api>/` and `docs/{simplified_formats,
simplified_data_template,smart_app_launch}/`, the machine contract under
`specifications/{operations,headers,parameters,responses,schemas}/` and
`computable/OAS/` (generated into `openehr-its::rest` — the contract layer
the crate implements). Template: verified current state (file:line), cited
`G-n` gap register, target design, honest PORT-NOTE residue.

| Spec (maturity) | Document | Implementation seam today |
|---|---|---|
| Overview (STABLE) | [overview.md](overview.md) | `app/ehrbase-rest/src/overview/` (negotiate, committal, error, version_id, params, status) |
| System (STABLE) | [system.md](system.md) | `/rest/status`, `OPTIONS /`, management surface |
| EHR API (STABLE) | [ehr.md](ehr.md) | `app/ehrbase-rest/src/dispatch/ehr.rs` |
| Query API (STABLE) | [query.md](query.md) | `app/ehrbase-rest/src/dispatch/query.rs` |
| Definition API (STABLE) | [definition.md](definition.md) | `app/ehrbase-rest/src/dispatch/definition*.rs` |
| Demographic API (DEVELOPMENT) | [demographic.md](demographic.md) | `app/ehrbase-rest/src/dispatch/demographic.rs` |
| Admin API (DEVELOPMENT) | [admin.md](admin.md) | admin routes + `management/` |
| Formats — simplified (STABLE) | [formats.md](formats.md) | `crates/openehr-flat` + the SDT media types in `overview/negotiate.rs` |
| SMART App Launch (DEVELOPMENT) | [smart.md](smart.md) | **greenfield** — integrates with `app/ehrbase-rest/src/access/` (OAuth2/OIDC) |

Open gaps execute through `docs/plans/rest-crate-rewrite.md` (W-3e) — a gap
lives in code or a re-verified cited PORT NOTE, never only in this folder.
