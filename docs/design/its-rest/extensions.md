# Extensions — the rethink (W-3e)

Owner directive 2026-07-12: the `app/ehrbase-rest/src/extensions/` surface
must **not** be a 1:1 carry-over of the pre-rewrite grab-bag — every module
is re-audited against the openEHR specs and either (a) **promoted** to the
spec folder that actually governs it, (b) **kept** as a justified,
explicitly-flagged spec-silent design, or (c) **redesigned** where the
carry-over encodes a stale premise. Nothing keeps the "extension" label that
a spec in the vendored oracle in fact governs.

## Classification register

| Module (today) | Audit verdict | Disposition |
|---|---|---|
| `audit.rs` + `audit_table.rs` (ATNA middleware + op classification) | **Spec-governed** — realizes the SM System Log component ("IHE ATNA-compliant system log", SM `master02-overview.adoc` §openEHR Platform Model) over the `SystemLog` trait (`ehrbase-sm::services::system_log`). Calling it an "extension" hides a normative capability (Logging is a STANDARD-profile capability in the CNF schedule). | **Promote** → `src/system_log/` (its own spec-aligned module: `middleware.rs` = audit.rs, `classify.rs` = audit_table.rs), module doc citing SM master02 + the CNF Logging capability. |
| item-tag plumbing (inside `access`-adjacent dispatch + the old tag routes) | **Spec-governed** — the development-edition EHR API defines the tag operations (`composition_tags_get` etc. in `ehr.openapi.yaml`) and the Overview defines the `openehr-item-tag`/`openehr-version-item-tag` headers. | **Already promoted** by the rewrite: tag ops live in `api/ehr/{composition,ehr,ehr_status}.rs`, header helpers in `overview/params.rs`. The `ItemTagAdapter` seam stays in `ehrbase-sm::extensions` only until the SM publishes an ITEM_TAG service interface (none exists — RM `common.item_tag` governs the *type*; record as spec gap). |
| `access/authn/` (Basic + OAuth2/OIDC + JWT) | **Spec-adjacent** — ITS-REST places authn out of band (`overview/Requests_and_responses.md` §Authentication and authorization: scheme not mandated, but `401`/`403`/`407` + `WWW-Authenticate` discipline IS normative), and SMART master06 §Authentication now *builds on* this seam (Bearer/JWT validation is the resource-server duty). | **Keep** under `extensions/access/authn/` but re-document: the 401/403 discipline cites the Overview §; the JWT/Bearer layer cites SMART master06 (the CDR-side duties); the deprecated-grant rejections live in `smart/config.rs`. |
| `access/authz/` (Cedar RBAC/ABAC) + `abac.rs` (PEP) | **Spec-silent by design** — authorization is out of band (same Overview §; SM master02 §Functional Style: "Authentication and authorisation is assumed to have been dealt with before any particular call"). SMART scope enforcement (master08) now AND-composes onto this PEP. | **Keep**, flagged; the PEP gains the SMART `enforce::evaluate` call site (the smart worker's integrate note). Rename `abac.rs` → `access/pep.rs` so the policy-enforcement point lives with the access stack instead of floating loose. |
| `access/ehr_access.rs` | **RM-governed slot, our scheme** — `EHR_ACCESS.settings` is RM (`ehr.adoc`: "All access decisions to data in the EHR must be made in accordance with the policies and rules in this object"); openEHR publishes no concrete `ACCESS_CONTROL_SETTINGS` scheme (W-9 decision). | **Keep** with the existing dual citation (RM slot + our scheme id `ehrbase.access_control.v1`). |
| `management/` (health/metrics/info/env/loggers) | **Spec-silent** — pure ops surface; no openEHR spec governs it. | **Keep**, flagged. `info.rs`'s spec-version provenance must stop triple-duplicating what `api/system` now owns (the System-manifest worker's G-7): management/info reads the same single provenance source the manifest uses. |
| `openapi.rs` (Swagger/OAS serving) | **Spec-adjacent** — serves the *vendored* OAS (the authoritative contract); not an API the spec defines. | **Keep**, flagged; assert it serves the development-edition bundles (same identity as the codegen input). |
| `terminology.rs` (the `/terminology` wire) | **SM-governed service, spec-silent wire** — `I_TERMINOLOGY_SERVICE` is SM master12; no ITS-REST terminology API exists in the development edition either (checked: no `terminology` group in the OAS set). | **Keep** as the extension wire with the master12 citation for the *operations'* semantics; wire shape stays flagged our-own. |
| `event_subscription.rs`, `fhir.rs` | **Spec-silent** — eventing + FHIR connector are our enterprise features (E1/E3); nothing in SM/ITS-REST governs them. | **Keep**, flagged (already are). |
| `tenant_routes.rs` + `access/tenant.rs` | **Spec-silent** — multi-tenancy (E2); zero spec mentions (verified for the SM crate move). | **Keep**, flagged. |
| *(new)* subject-proxy wire (W-3c step 7) | **SM-governed service, spec-silent wire** — like terminology: `I_SUBJECT_PROXY_SERVICE` is SM master10; master10 §Specifying a Data-set explicitly anticipates a REST ingestion surface but ITS-REST defines none. | **Add** under `extensions/subject_proxy.rs` when W-3c step 7 executes, citing master10 for operation semantics. |

## Target structure after the rethink

```
app/ehrbase-rest/src/
├── system_log/        PROMOTED — SM master02 System Log (ATNA middleware +
│                      op classification; was extensions/audit*.rs)
└── extensions/        only the genuinely spec-silent residue:
    ├── access/        authn (Overview §Auth discipline + SMART master06
    │                  citations) · authz Cedar · pep.rs (was abac.rs) ·
    │                  ehr_access (RM slot, our scheme) · tenant
    ├── management/    ops surface (single provenance source shared with
    │                  api/system)
    ├── openapi.rs     serves the vendored development-edition OAS
    ├── terminology.rs SM master12 semantics, our wire
    ├── event_subscription.rs · fhir.rs · tenant_routes.rs   (E1/E2/E3)
    └── subject_proxy.rs (W-3c step 7)
```

Execution rides the W-3e reconcile pass (the moves + re-documentation are
mechanical; the SMART PEP call-site and the provenance unification are the
two behavioural touches). Every kept module's top doc carries the explicit
"no openEHR spec governs this — our own design/extension" flag or its
precise spec citation — never the old bare "extension" shrug.
