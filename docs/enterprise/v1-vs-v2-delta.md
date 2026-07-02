# EHRbase v1 vs v2 delta — enterprise feature archaeology

Output of the Phase 0 archaeology task (PORT_MASTER_PLAN.md Section 11.1).
Record only. Nothing in this file is to be built or restored during Stage 1;
confirmed items become Stage 2 phase files (`docs/plans/s2-phase-NN-*.md`).

## Anchors

| Ref | Commit | Date | Role |
|---|---|---|---|
| `v0.32.0` = branch `reference/v1` | `89acbdd97` | 2024-01-19 | last pre-v2 release (the "v1" line) |
| `v2.0.0` | `f5a09784e` | 2024-04-08 | first release after the cut |
| `v2.33.0` | `ce5546144` | 2026-06-16 | current import baseline (branch `develop`) |

All findings below were taken from committed refs only (`git ls-tree`, `git show`,
`git grep <ref>`, `git diff <ref> <ref>`), never from the working tree, which is
mid-reorganization.

**Provenance caveat that shapes everything here:** v2 was developed in a closed
repository and landed in the public repo as a single squashed commit,
`0eaa92b42` "CDR-1375 Open-Source new ehrbase version" (2024-04-08, PR #1255
`feature/new_ehrbase`). Only 21 commits exist between `v0.32.0` and `v2.0.0`,
almost all dependency bumps. There are therefore no public per-feature removal
commits; the v2.0.0 CHANGELOG entry says only that the release "contains a
complete overhaul of the data structure and the Archetype Query Language (AQL)
engine". Where a reason is not stated in CHANGELOG.md/UPDATING.md, this file
says "reason not stated".

Overall magnitude: `git diff --shortstat v0.32.0 v2.33.0` = 1293 files changed,
+73,714 / −104,840 lines. Java file count fell from 744 to 428.

## Summary table

| Capability | v1 evidence (at v0.32.0) | v2 status (at v2.33.0) | Stage 2 restoration |
|---|---|---|---|
| ABAC (attribute-based access control) | `application/src/main/java/org/ehrbase/application/abac/*` (4 classes), `@PreAuthorize("checkAbac...")` on 7 controllers, `abac:` block in application.yml | Removed entirely (no `abac` package, no config block) | In scope — highest priority |
| Multi-tenancy | `org.ehrbase.api.tenant.*`, `org.ehrbase.tenant.*`, `TenantService`, `TenantAspect`, `@TenantAware`, migrations V73–V84 incl. Postgres RLS | Removed; v2.0.0 schema kept a vestigial `tenant` table, dropped again by `V5_1..V5_4__remove_multi_tenancy.sql` (shipped inside v2.0.0). Zero `tenant` references in v2.33 Java | In scope — decide model at S2 |
| Security integration (authn: Basic/OAuth2/OIDC) | `application/.../config/security/*` (6 classes), `security:` config block, `doc/security/` Keycloak guide | Retained, relocated to `configuration/.../config/security/*`; Keycloak how-to docs removed from repo | Retained in v2 — port during Stage 1 (P6/P15), not a Stage 2 item |
| Plugin system (PF4J SPI) | `plugin/` module: extension points, plugin aspects, KV store, plugin security | Retained but slimmed (−208/+102 lines); plugin aspects deprecated in 2.5.0 (#1344); KV store moved to `api` as `KeyValuePair(Repository)` | Retained in v2 — Stage 2 ADR for the Rust replacement (per master plan) |
| EhrScape API | `rest-ehr-scape` with Ehr, Query, Composition, Template controllers | Ehr + Query controllers deleted at v2.0.0; whole API deprecated since 2.0.0 and disabled by default since 2.10.0 (`ehrbase.rest.ehrscape.enabled=false`) | Partially in scope — port the surviving slice; restoring Ehr/Query EhrScape endpoints is optional, low priority |
| Attestation persistence | `ehr.attestation`, `attestation_ref`, `attested_view` tables; `AttestationAccess` DAO wired into Composition/Ehr/Status access | Removed with the schema rewrite; only a passing mention in `AbstractVersionedObjectRepository` | Out of scope — was DB scaffolding, never exposed over REST |
| v1 normalized DB schema (jOOQ) | 88 Flyway migrations in `base/src/main/resources/db/migration`, ~39 `ehr.*` tables | Replaced wholesale by row-per-locatable schema (`jooq-pg/src/main/resources/db/migration/ehr`, fresh V1) | Not restorable by design — the port targets the v2 schema |
| Misc repo-level assets | `base` module, `doc/` tree, `terminology.xml`, `validation` stub, `.circleci` | Removed/dissolved | Out of scope |

## 1. ABAC — attribute-based access control (removed)

What it did (v0.32.0):

- `application/src/main/java/org/ehrbase/application/abac/AbacConfig.java` —
  `@ConditionalOnProperty(name = "abac.enabled")`, `@ConfigurationProperties(prefix = "abac")`.
  Defines `AbacType { EHR, EHR_STATUS, COMPOSITION, CONTRIBUTION, QUERY }` and
  `PolicyParameter { ORGANIZATION, PATIENT, TEMPLATE }`, plus an Apache
  HttpClient that POSTs a policy-evaluation request to an external policy
  decision point (`abac.server`, e.g. `http://localhost:3001/rest/v1/policy/execute/name/`).
- `CustomMethodSecurityExpressionRoot.java` / `CustomMethodSecurityExpressionHandler.java` /
  `MethodSecurityConfig.java` (same package) — register the custom SpEL
  functions `checkAbacPre` / `checkAbacPost` used from `@PreAuthorize` /
  `@PostAuthorize`.
- Enforcement points: `git grep checkAbac v0.32.0` hits 7 controllers in
  `rest-openehr/src/main/java/org/ehrbase/rest/openehr/`:
  `OpenehrCompositionController` (8), `OpenehrQueryController` (4),
  `OpenehrVersionedCompositionController` (4), `OpenehrVersionedEhrStatusController` (4),
  `OpenehrEhrStatusController` (3), `OpenehrContributionController` (2),
  `OpenehrEhrController` (2).
- Configuration (v0.32.0 `application/src/main/resources/application.yml`):
  `abac.enabled` (default false), `abac.server`, `abac.organizationClaim`
  (`organization_id`), `abac.patientClaim` (`patient_id`), and per-resource
  policies `abac.policy.{ehr,ehrstatus,composition,contribution,query}` with
  `name` + `parameters` drawn from organization/patient/template. Claims were
  read from the OAuth2 JWT; the patient parameter fell back to the EHR subject.

v2 status: gone at v2.0.0 already (no `abac` hits at v2.0.0 or v2.33.0 outside
the tenant migration). Role checks that remain in v2 are the coarse
`hasRole('ADMIN')`-style USER/ADMIN split of the base security config.

Why removed: reason not stated in CHANGELOG.md or UPDATING.md; removal happened
inside the closed-development squash `0eaa92b42`.

Stage 2: in scope, highest priority (plan Section 11.2). Rust candidates per
the pinned stack: `casbin` or `cedar-policy` behind a tower layer, replicating
the pre/post-check semantics and the external-PDP option.

## 2. Multi-tenancy (removed)

What it did (v0.32.0):

- API surface: `api/src/main/java/org/ehrbase/api/annotations/TenantAware.java`,
  `api/.../aspect/TenantAspect.java`, `api/.../service/TenantService.java`
  (CRUD: `create/update/findBy/deleteTenant/getAll/hasTenant`, plus
  `getCurrentSysTenant(): Short` and `getCurrentTenantIdentifier()`), and
  `api/.../tenant/` (`Tenant`, `TenantAuthentication`,
  `TenantIdExtractionStrategy` with priority/accept/extract,
  `ExtractionStrategyAware`, `ThreadLocalSupplier`).
- Implementation: `service/src/main/java/org/ehrbase/tenant/`
  (`DefaultTenantAspect`, `DefaultTenantAuthentication`, `TokenSupport`,
  `extraction/AuthenticatedExtractionStrategy`, `extraction/DefaultExtractionStrategy`)
  and `service/.../service/TenantServiceImp.java`; DAO in
  `service/.../dao/access/jooq/TenantAccess.java`, `interfaces/I_TenantAccess.java`,
  `support/TenantSupport.java`.
- Tenant selection: extracted from the JWT claim `"tnt"`
  (`DefaultTenantAuthentication.TENANT_CLAIM = "tnt"`); without a token the
  default strategy synthesizes `TenantAuthentication.DEFAULT_TENANT_ID =
  "1f332a66-0e57-11ed-861d-0242ac120002"`. No `X-Tenant-*` header handling
  exists at v0.32.0.
- Database: `base/src/main/resources/db/migration/V73__add_tenantid_column.sql`
  (creates `ehr.tenant` and adds tenant columns),
  `V74__add_rls_for_tenant.sql` (enables Postgres ROW LEVEL SECURITY on ~25
  `ehr.*` tables), `V75/V75_1` (tenant id in indexes), `V78` (properties column
  on tenant), `V83__change_sys_tenant_to_short.sql` (`sys_tenant` becomes a
  `Short` surrogate key), `V84__tenant_delete_cascade.sql`.
- No REST controller for tenant management exists in the v0.32.0 tree — the
  open-source part is service/DAO only (see "Uncertain" below).

v2 status: the Java stack is completely gone (zero `tenant` matches in v2.33.0
`*.java`). The v2.0.0 baseline schema (`jooq-pg/.../ehr/V1__ehr.sql` +
`V2__tenant.sql`) still created a `tenant` table and inserted `default_tenant`
with exactly the v1 default UUID `1f332a66-0e57-11ed-861d-0242ac120002` — a
compatibility vestige for the pre-release v2 line — and then
`V5_1..V5_4__remove_multi_tenancy.sql`, shipped in the same v2.0.0 release,
drop every `sys_tenant` foreign key/column and `DROP TABLE tenant`.

Why removed: reason not stated. The explicit migration name
(`remove_multi_tenancy`) confirms the removal was deliberate.

Stage 2: in scope (plan Section 11.2). The v1 model to restore is: tenant
registry table, per-row `sys_tenant` discriminator, JWT-claim-driven tenant
context, optional Postgres RLS enforcement. Decide at S2 whether to replicate
RLS or filter in the query layer.

## 3. Security integration — authn (retained, relocated; docs removed)

- v0.32.0: `application/src/main/java/org/ehrbase/application/config/security/`
  — `SecurityConfiguration`, `BasicAuthSecurityConfiguration`,
  `OAuth2SecurityConfiguration`, `NoOpSecurityConfiguration`, `SecurityFilter`,
  `SecurityProperties`. Config keys `security.authType` (BASIC/OAUTH/NONE),
  `authUser/authPassword`, `authAdminUser/authAdminPassword`,
  `oauth2UserRole/oauth2AdminRole`, plus
  `spring.security.oauth2.resourceserver.jwt.issuer-uri` (Keycloak).
- v2.33.0: same set survives as `configuration/src/main/java/org/ehrbase/configuration/config/security/`
  (`SecurityConfig`, `SecurityConfigBasicAuth`, `SecurityConfigOAuth2`,
  `SecurityConfigNoOp`, `SecurityFilter`, `SecurityProperties`) with the same
  config keys. Not a removed capability.
- Actually removed: the operator documentation `doc/security/README.md` plus
  ~16 Keycloak/Postman setup screenshots (the whole `doc/` tree —
  `conformance_testing`, `release`, `security` — left the repo at v2.0.0).
- What v1 never had in OSS: ATNA audit logging (`git grep -i atna` at v0.32.0:
  no hits). Do not list ATNA as a regression.

Stage 2: nothing to restore beyond ABAC/tenancy above; authn is ported in
Stage 1 with `jsonwebtoken`/`openidconnect`/`argon2` per the pinned stack.

## 4. Plugin system — PF4J SPI (retained, slimmed)

- Both refs have the `plugin` module with `EhrBasePlugin`,
  `EhrBasePluginManagerInterface`, `WebMvcEhrBasePlugin`,
  `NonWebMvcEhrBasePlugin`, the five extension points
  (`Composition/Ehr/Query/Template ExtensionPoint`, `ExtensionPointHelper`),
  `rest/RestEndpointSupport`, and `security/{AuthorizationInfo,
  PluginSecurityConfiguration}`. `plugin-manager.*` config keys are unchanged.
- Delta v0.32.0 → v2.33.0 (`git diff --stat -- plugin`): 23 files,
  +102/−208. `repository/KeyValueEntry.java` and `KeyValueEntryRepository.java`
  were replaced by `api/src/main/java/org/ehrbase/api/repository/KeyValuePair.java`
  + `KeyValuePairRepository.java` (moved, not removed);
  `registration/ExternalBeanRegistration.java` was added;
  `PluginSecurityConfiguration` shrank.
- The service-side plugin aspects
  (`service/src/main/java/org/ehrbase/plugin/{Abstract,Composition,Ehr,Query,Template}PluginAspect.java`)
  exist at both refs but were deprecated in 2.5.0 ("Deprecate plugin aspects",
  PR #1344, CHANGELOG 2.5.0); the `ehr.plugin` KV table still exists in the v2
  schema.

Stage 2: the plugin system itself survived; the Stage 2 item is the ADR for a
Rust replacement mechanism (trait registry / cdylib / WASM), as already noted
in the master plan. No v1-vs-v2 recovery needed.

## 5. EhrScape API (scope reduced, deprecated, off by default)

- v0.32.0 `rest-ehr-scape` controllers: `CompositionController`,
  `EhrController`, `QueryController`, `TemplateController` (+ response types
  `EhrResponseData`, `QueryResponseData`, `TemplatesResponseData`,
  `TemplateExampleResponseData`).
- v2.33.0: only `CompositionController` and `TemplateController` remain.
  `EhrController.java` and `QueryController.java` were deleted in the v2.0.0
  squash (`0eaa92b42`), i.e. the `/rest/ecis/v1/ehr` and `/rest/ecis/v1/query`
  surfaces are gone.
- Stated reason/timeline (UPDATING.md at v2.33.0, section 2.10.0): "Starting
  from version 2.0.0 the ehrscape API was deprecated. With the release of
  version 2.10.0, the API is now disabled by default", toggle
  `ehrbase.rest.ehrscape.enabled` (default `false`; introduced by PR #1415).

Stage 2: port the surviving slice in Stage 1 (P16). Restoring the removed
Ehr/Query EhrScape endpoints is optional and low priority; the deprecation is
upstream policy, not a technical loss.

## 6. Attestation persistence scaffolding (removed)

- v0.32.0: tables `ehr.attestation`, `ehr.attestation_ref`,
  `ehr.attested_view` (all under RLS per V74); DAO
  `service/.../dao/access/jooq/AttestationAccess.java` +
  `interfaces/I_AttestationAccess.java`, referenced from
  `CompositionAccess`, `EhrAccess`, `StatusAccess`, `AdminApiUtils`.
- v2.33.0: no attestation tables in the v2 schema; the only source mention is
  in `service/.../repository/AbstractVersionedObjectRepository.java`.
- No REST endpoint existed at either ref (ITS-REST has no attestation
  operations). Why removed: reason not stated; presumably dropped with the
  schema rewrite as dead scaffolding.

Stage 2: out of scope. Revisit only if openEHR attestation support becomes a
product requirement.

## 7. Database schema rewrite (context for all of the above)

Not an enterprise feature, but the mechanism by which several features fell:

- v1: `base/src/main/resources/db/migration` — 88 Flyway files (V1..V84+),
  jOOQ classes in module `jooq-pq`. Normalized, node-per-table model:
  `ehr.entry` (composition content as one JSONB per entry), `composition`,
  `event_context`, `participation`, `party_identified`, `identifier`,
  `status`, `folder*` (5 tables), `object_ref*`, `containment` (AQL
  containment index), `concept`/`language`/`territory` (openEHR terminology
  loaded into the DB), `template_store`, `terminology_provider`,
  `access` (EHR_ACCESS), `compo_xref`, `attestation*`, `audit_trail`,
  `session_log`, `heading`/`template_heading_xref`. AQL had an optional
  jsquery path (`server.aqlConfig.useJsQuery`) requiring the `jsquery`
  extension.
- v2: fresh baseline in `jooq-pg/src/main/resources/db/migration/ehr`
  (`V1__ehr.sql` onward; module renamed `jooq-pg`): row-per-locatable
  `comp_data`/`comp_version` (+`_history`), `ehr_status_data`,
  `ehr_folder_data`, `contribution`, `audit_details`, `users`,
  `template_store`, `stored_query`, `plugin`; `system` dropped later
  (`V11__drop_system.sql`). `concept/language/territory`, `containment`,
  `access`, `compo_xref`, `audit_trail`, `session_log`,
  `terminology_provider`, `heading*` have no v2 equivalents.
- `audit_trail` and `session_log` have no Java references in v1
  `service`/`application` main sources — they were EtherCIS-era leftovers, not
  working features. Do not count them as regressions.
- Data migration v1→v2 is explicitly external: UPDATING.md 2.0.0 points to
  https://github.com/ehrbase/migration-tool.

Stage 2: none of this is restorable or desirable to restore; the Rust port
targets the v2 schema (plan P7/P13/P14).

## 8. Other observations (mostly not removals)

- `base` Maven module: dissolved by v2.0.0. It held the migrations,
  `db-setup/*.sql` (including `add_restricted_user.sql`,
  `migrate_to_cloud_db_setup.sql`) and the bundled `terminology.xml` +
  `Terminology.xsd`. v2.33.0 has no `terminology.xml` in-repo; terminology
  assets come via the openEHR SDK dependency (see Uncertain).
- `validation` directory at v0.32.0 was already vestigial (single orphaned
  test resource, no pom); its disappearance is not a feature loss.
- Added in v2 (reverse delta, for completeness): `aql-engine`,
  `rm-db-format`, `configuration`, `db_scripts` modules; `cli` module
  (`CliDataBaseCommand` etc.); `AdminQueryController` (stored-query admin);
  experimental Item Tags; Matrix format; `ext` schema with aggregate
  functions.
- Survived intact (false-positive guard): admin API (`/rest/admin`, both refs,
  `admin-api.active` default false), `/rest/status` (`StatusController`),
  management/actuator endpoints, external FHIR terminology validation
  (`validation.external-terminology.*` + `FhirTerminologyValidation`, moved
  in-repo just before the cut by PR #1242), JavaMelody toggle, Redis cache
  option, template overwrite switch (renamed `system.allow-template-overwrite`
  → `ehrbase.template.allow-overwrite`, PR #1440).
- CI moved from CircleCI (`.circleci`, v1) to GitHub Actions only (v2).

## Uncertain / needs manual confirmation

- How v1 tenants were administered: no tenant REST controller exists at
  v0.32.0, only `TenantService`/DAO. Tenant CRUD may have been exposed by a
  closed-source plugin or a commercial layer. Confirm against EHRbase v1 docs
  or upstream maintainers before scoping the Stage 2 tenancy API.
- Whether any commercial/enterprise connectors existed outside this repo
  (plan 11.2 "commercial connectors"): nothing in the OSS tree at either ref
  evidences them; cannot be confirmed from git alone.
- The external ABAC policy server product v1 targeted (config example points
  at `localhost:3001/rest/v1/policy/execute/name/`): its identity/protocol is
  not documented in-repo; verify the expected request/response contract before
  reimplementing `checkAbacPre/Post` semantics.
- Exact terminology asset path in v2: `terminology.xml` is absent from the
  v2.33.0 repo; assumed to ship inside the openEHR SDK jar. Verify which SDK
  artifact provides it before P2 pins the TERM bundle behavior.
- v1 `heading`/`template_heading_xref` and `compo_xref` tables: no clear v1
  Java usage was verified in this pass; treated as dormant schema, but a
  deeper grep of `jooq-pq` consumers would confirm.
- Task brief cited `develop` at `bd4cced35` as the import point; tag
  `v2.33.0` resolves to `ce5546144` here. All v2 claims in this file are
  against the tag.
