# W-3e — `ehrbase-rest`: complete spec-first rewrite (ITS-REST)

Owner directive 2026-07-12: the REST crate *is* the implementation of
ITS-REST — rebuild it exactly the way `ehrbase-sm` was rebuilt (W-3c method):
**per specification document — read the spec, create the folder, author the
files from the spec text; never migrate legacy**. Full compliance is the
goal; intermediate steps need not compile (same big-bang ruling as the SM
rewrite); the `ehrbase` platform impl is fixed afterwards in one pass.

> **Edition ruling (owner, 2026-07-12):** the oracle is the ITS-REST
> **development** edition @ `e8a093e9` — the same commit the vendored OAS and
> the B5 conformance identity already use — because it carries the API specs
> Release-1.0.3 lacks: **System** (STABLE), **Demographic API**
> (DEVELOPMENT), **Admin API** (DEVELOPMENT), **Formats** (STABLE there),
> **SMART** (DEVELOPMENT). `scripts/vendor-spec-docs.sh` re-pinned
> accordingly; the table below reflects the development-edition set. The
> demographic and admin surfaces therefore get spec-governed `api/` modules
> (no longer extensions-by-analogy); SMART is recorded, not implemented.

## The oracle (all vendored)

| Spec | Vendored at | Governs |
|---|---|---|
| **Overview** (STABLE) | `docs/specs/openehr/ITS-REST/specifications/docs/overview/` (`Intro`, `Glossary_and_conventions`, `Resources`, `Requests_and_responses`) | the cross-cutting protocol: content negotiation, representation headers (`ETag`/`Location`/`Last-Modified`/`Prefer`/`If-Match`), committal headers (`openEHR-VERSION.*`, `openEHR-AUDIT_DETAILS.*`), datetime formats, HTTP status discipline, error body, auth-out-of-band, versioned-object addressing |
| **EHR API** (STABLE) | `…/docs/ehr/` + `…/operations/*.yaml` (EHR, EHR_STATUS, VERSIONED_EHR_STATUS, COMPOSITION, VERSIONED_COMPOSITION, DIRECTORY, CONTRIBUTION) | the `/ehr/**` resource surface |
| **Query API** (STABLE) | `…/docs/query/` (`Qualified_query_name`, `Query_types`, `Request`, `Response`) + operations | `/query/**` (ad-hoc + stored, GET + POST forms, RESULT_SET wire) |
| **Definition API** (STABLE) | `…/docs/definition/` + operations | `/definition/**` (templates ADL 1.4/2, stored queries) |
| **SDT** (DEVELOPMENT) | `docs/specs/openehr/ITS-REST/docs/simplified_data_template/master0{0..6}*.adoc` | the simplified (FLAT/structured) wire formats |
| **OAS 3.0** (TRIAL) | `…/specifications/*.openapi.yaml` + `headers/` `operations/` `parameters/` `responses/` `schemas/` | the machine contract — **already generated** into `openehr-its::rest` (`emit-rest`, the codegen pipeline); the crate implements the generated traits, never hand-mirrors the OAS |

Standing architecture (unchanged by this rewrite): the ITS-REST *contract*
(DTOs, server traits, route tables) is generated from the vendored OAS into
`openehr-its`; `ehrbase-rest` is the **protocol adapter** implementing those
traits over the `ehrbase-sm` native API (`Platform`). The rewrite reorganises
and re-authors the adapter itself.

## Target structure (mirrors the spec set)

```
app/ehrbase-rest/src/
├── lib.rs            crate map + spec precedence (wire oracle: ITS-REST + ECC)
├── overview/         the Overview spec (cross-cutting protocol)
│   ├── negotiate.rs  content/representation negotiation (Accept/Prefer)
│   ├── headers.rs    ETag / Location / Last-Modified / If-Match discipline
│   ├── committal.rs  openEHR-VERSION.* / openEHR-AUDIT_DETAILS.* merge
│   ├── datetime.rs   the Overview datetime/format rules
│   └── error.rs      status-code discipline + error body
├── api/
│   ├── ehr/          the EHR API spec (one module per resource:
│   │                 ehr.rs, ehr_status.rs, versioned_ehr_status.rs,
│   │                 composition.rs, versioned_composition.rs,
│   │                 directory.rs, contribution.rs)
│   ├── query/        the Query API spec (adhoc.rs, stored.rs, response.rs)
│   └── definition/   the Definition API spec (template_adl14.rs,
│                     template_adl2.rs, stored_query.rs)
├── sdt/              the SDT spec wire (simplified formats; EhrScape stays
│                     feature-gated prior art)
├── extensions/       EVERYTHING not in ITS-REST, quarantined + flagged:
│   admin, management, /rest/status, item tags (dev-branch), terminology
│   wire, subject_proxy wire (W-3c step 7), events, fhir connector, tenancy
│   middleware, access/ (RBAC/ABAC), openapi/swagger serving
└── router.rs         assembly over the openehr-its generated route tables
```

## Step 0 — the register first (owner ruling 2026-07-12)

Before any rewrite code: the per-spec compliance register at
**`docs/design/its-rest/`** — one audited design/gap document per
specification (overview, system, ehr, query, definition, demographic,
admin, formats, **smart**), same template as `docs/design/sm-platform/`
(verified file:line state, cited G-n gap register, target design, PORT-NOTE
residue). The rewrite then executes the register. **SMART App Launch is an
implementation target** (greenfield: discovery document, registration,
launch flows, scope model on the existing OAuth2/OIDC + Cedar seams), not
merely a record.

## Method (identical to the SM rewrite)

Per spec document, in order — Overview → EHR → Query → Definition → SDT →
extensions sweep → final rewire:

1. Read the spec document in full (+ the operation/header/response YAMLs it
   governs).
2. Create the folder; author the files fresh from the spec text — verbatim
   citations in doc comments (`Requests_and_responses.md §…`,
   `operations/composition_create.yaml`), PORT NOTEs for every deliberate
   deviation, spec defects recorded.
3. Audited-faithful existing logic may be carried into the fresh structure
   (it is spec-true text, not legacy), but every file gets re-grounded in
   its governing spec section and dead citations scrubbed.
4. Wire-oracle precedence: the vendored OAS + CNF/ECC schedule win over
   prose where they conflict; conflicts get recorded.
5. Chapter-register gaps close as we pass: the open ITS-REST rows of
   `docs/design/sm-platform/` chapter files and blueprint map rows 13–15
   (committal-header MUST, `Last-Modified` emission, If-Match hardening,
   `OPTIONS /`, RESULT_SET `ETag`, DIRECTORY `path` semantics,
   version-family XML) — each lands in code or a re-verified cited PORT
   NOTE.
6. After the crate rewrite: fix `ehrbase` (the platform impl) against BOTH
   rewritten crates in one pass, then the full test suite, ECC zero-drift,
   website book + changelog updates.

## The zero-TODO mandate (owner ruling 2026-07-12)

The parallel rewrite coordinates cross-folder seams via `TODO(w3e-integrate)`
/ `TODO(w3c…)` / `TODO(w3e-formats)` markers. **The phase does not close
while ANY of them remains** in `app/ehrbase-rest`, `app/ehrbase-sm`,
`app/ehrbase` or `crates/openehr-flat`: the final step is an inventoried
elimination sweep (grep-driven) in which every marker is implemented — incl.
the in-wave deferrals (SPS FHIR frame executor, the ServiceResponse item-tag
seam, the formats read-emission wire + snapshot regen, provenance adoption
in the System manifest + /status, AuditOpId on extension routers, the query
408 timeout signal, per-API `return=identifier` bodies). Spec-gap
`PORT NOTE`s (cited records of spec silence/defects) are not TODOs and stay.

## Exit criteria

- [ ] Every `src/` module maps 1:1 to a spec document (or sits in
  `extensions/` with a spec-silence flag).
- [ ] The five ITS-REST spec documents fully read and realized; every
  MUST/SHOULD in the Overview protocol implemented or PORT-NOTEd.
- [ ] Blueprint map rows 13–15 tails closed.
- [ ] **Zero `TODO(w3e-*)`/`TODO(w3c…)` markers across the four crates**
  (grep-verified in the close checklist).
- [ ] Workspace green (build + clippy + nextest), ECC zero-drift, docs +
  changelog current.
