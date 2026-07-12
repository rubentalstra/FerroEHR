# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Maintenance rules: every pull request that changes user-visible behaviour —
the REST surface, AQL, validation, storage/migrations, configuration, CLI,
container/Helm artifacts — adds an entry under **[Unreleased]** in the same
PR (a CI guard enforces this). Cutting a release renames [Unreleased] to the
version + date, adds fresh link references, and tags `vX.Y.Z`; the release
workflow refuses a tag that has no matching section here.

## [Unreleased]

### Added

- SMART App Launch resource-server support (openEHR SMART App Launch
  framework, development edition), config-gated and off by default
  (`EHRBASE_REST_SMART__*`): the `/.well-known/smart-configuration`
  discovery document, the full resource-scope grammar
  (`compartment/resource.permission` with `*`/`**`/`ns::*` patterns), and
  scope + launch-context (`ehrId`→patient) enforcement composed after
  RBAC/ABAC.
- Subject Proxy Service completed (SM `I_SUBJECT_PROXY_SERVICE`): variables
  are now tracked over time (a persisted sample history per variable),
  `currency` freshness is evaluated (fresh samples are served without
  re-querying; data-set registration tightens currency), data-set local
  aliases resolve on reads, `using_app_ids` lifecycle drops empty data
  sets, and frames execute with primary→fallback semantics. New FHIR frame
  executor (config-gated named systems, `EHRBASE_SUBJECT_PROXY__*`) lets
  variables be populated from FHIR R4 servers; manual variables gain a
  notification input channel.
- System API `OPTIONS /` conformance manifest rebuilt: reports the live
  mounted endpoint groups, a single provenance source (the tested
  development-edition ITS-REST identity), and configurable identity fields
  (`EHRBASE_REST_SYSTEM__*`); also mounted at the API base path.
- Item tags via headers (`openehr-item-tag`/`openehr-version-item-tag`):
  accepted on EHR-group and demographic writes and echoed on responses.
- Query API: multi-EHR scoping (`ehr_ids` set), an honest
  `ehr_id_does_not_exist` (404) for a well-formed absent EHR id, a weak
  `ETag` on `RESULT_SET` responses, parameter-substituted
  `meta._executed_aql`, and an optional query execution timeout
  (`EHRBASE_QUERY__TIMEOUT_MS`) mapped to `408`.
- Definition API: template list filtering (`template_id` glob, `concept`,
  `version`) and pagination are honoured; stored-query `query_type` is
  read with an honest unsupported-formalism rejection; ADL1.4 uploads
  return the JSON `TemplateIdentifier` under `Prefer: return=identifier`.
- FLAT/STRUCTURED (Simplified Formats, now STABLE): the `_`-prefixed
  optional RM attribute family (`_uid`, `_link`, `_feeder_audit`,
  `_null_flavour`, `_mapping`, `_normal_range`, participations, work-flow
  ids, …) round-trips in both directions; `|raw` canonical-JSON embedding
  on write; complete quantity/date-time/multimedia leaf attribute tables;
  `|other` open-value-set rules enforced.

### Changed

- Development-edition ITS-REST protocol adopted (the server's tested
  contract identity, now reported consistently as such): `ETag` response
  headers carry the weak `W/"…"` indicator (bare quoted values are still
  accepted on `If-Match`); committal metadata uses the lowercase
  `openehr-version` / `openehr-audit-details` value-form headers (the
  deprecated `openEHR-VERSION.*` dotted spellings remain accepted) and a
  client-supplied `system_id` is merged into the commit audit; `Location`
  is emitted only on resource creation (no longer on reads/deletes);
  `Preference-Applied` echoes the honoured `Prefer`; `405`/`501` render
  the openEHR error body.
- Demographic DELETE follows the published Demographic API: the preceding
  version id rides in the path; a stale id yields `409` (with the latest
  version `ETag`), an already-deleted party `400`.
- Admin `DELETE /admin/ehr/all` follows the published Admin API: `204`
  with no body, and an absent `ehr_id` parameter now means delete ALL
  EHRs.
- FLAT duplicate node-name suffixes default to the specification form
  (`name_1`); the Better-compatible form (`name2`) is available behind the
  `ehrbase-quirks` feature.
- The `ehrbase-rest` and `ehrbase-sm` crates were restructured
  specification-first (one folder per ITS-REST spec / SM chapter, all
  spec-silent surfaces quarantined under `extensions/`) — no route
  changes beyond those listed here.

### Fixed

- Template list endpoints no longer ignore filter and pagination
  parameters.
- The conformance manifest and `/rest/status` no longer misreport the
  implemented ITS-REST edition as `1.0.3`.


### Added

- Multiple folder hierarchies per EHR (`EHR.folders`): beyond the
  `/directory` hierarchy, additional root `FOLDER`s can be committed through
  the CONTRIBUTION endpoint, each versioned independently. The EHR resource
  now carries the `folders` reference list (creation order) and `directory`
  (always its first member); EHR extract import and admin dump/load carry
  the hierarchies too. The `/directory` endpoints behave exactly as before.
- `ehr:` URI support: `DV_EHR_URI` values are parsed against the full
  openEHR `ehr:` grammar (EHR / top-level structure by uid or exact version
  id / interior item paths, absolute and relative forms), and the server can
  resolve local `ehr:` references internally (e.g. LINK targets). openEHR
  path processing now also supports `//` path patterns and 1-based
  positional predicates in stored-structure navigation (AQL is unchanged —
  its grammar defines neither).

- `EHR_ACCESS` access-control is now enforced. The spec-mandated,
  change-controlled `EHR_ACCESS` object of an EHR (RM ehr §EHR_ACCESS Class)
  is the foundational access-decision layer, evaluated after authentication
  and before dispatch on every EHR-scoped route; the enterprise RBAC/ABAC
  layers compose on top of it. Its `settings` use the
  `ehrbase.access_control.v1` scheme (`docs/design/ehr-access-scheme.md`):
  a `default_access` (`open`/`restricted`) with a `user:`/`role:` access
  list gating the EHR, per-Composition privacy-level ceilings on Composition
  reads, and a gate-keeper that guards changes to the settings themselves
  (`403 Forbidden` on a denial). Every existing EHR keeps working — the
  default (no settings) is open.
- Client-supplied CONTRIBUTION `uid`s are honoured on commit when unused
  (`409 Conflict` when already in use; previously silently ignored).
- `Prefer: resolve_refs` is honoured on contribution reads: the
  CONTRIBUTION's `versions` are returned as full `ORIGINAL_VERSION`
  objects instead of `OBJECT_REF`s (ITS-REST representation negotiation).
- AQL single-row functions now execute: `LENGTH`, `SUBSTRING`, `POSITION`,
  the string `CONTAINS`, `CONCAT`/`CONCAT_WS`, `ABS`/`MOD`/`CEIL`/`FLOOR`/
  `ROUND`, and `CURRENT_DATE`/`CURRENT_TIME`/`CURRENT_DATE_TIME`/`NOW`/
  `CURRENT_TIMEZONE` (QUERY master03 §Functions).
- AQL `TERMINOLOGY()` Boolean value expressions
  (`TERMINOLOGY('validate'|'subsumes', …) = true`) and terminology-URI
  `matches` operands (`matches { terminology://… }`) are now evaluated
  through the terminology service (previously typed rejects).
- AQL archetype predicates now honour archetype-specialisation subsumption:
  a query naming a parent archetype (e.g.
  `[openEHR-EHR-OBSERVATION.laboratory.v1]`) also matches data created with
  any specialisation child (e.g. `…laboratory-glucose.v1`), scoped to the
  same RM entity and major version (BASE architecture_overview master10
  §Design-time Relationships; AM master07 §Querying). Non-HRID predicates
  (at/id-codes) keep exact case-folded matching.

### Changed

- `PUT …/composition/{uid_based_id}` rejects a body whose
  `COMPOSITION.uid` does not identify the versioned object addressed by
  the path (`400`).
- AQL semantic analysis is stricter per QUERY master03: duplicate FROM
  variable names reject, variable references are case-insensitive,
  `LIMIT 0`/negative `OFFSET` reject, `SUM`/`AVG` over non-numeric paths
  reject, scalar-function arity is validated, and `LIKE` `\*`/`\?`
  escapes now match the literal characters.
- OPT 1.4 template upload enforces the AOM 1.4 constraint-model invariants
  (attribute existence bounds, single-attribute occurrences, archetype-id
  well-formedness and root-type match, slot identifier validity,
  internal-reference target paths, constraint-reference definedness,
  boolean satisfiability, assumed-value validity, temporal and duration
  constraint-pattern validity, duplicate code-list codes) — invalid
  templates are rejected with `400` carrying the AOM rule code.
- ADL2 artefact upload (`I_DEFINITION_ADL2`) now validates sources against
  the registration-decidable AOM2 catalogue (mandatory sections, header
  versions, root type/node-id rules, specialisation depth, terminology
  language consistency, code definedness, value-set validity, term-binding
  keys) instead of a header-only probe — invalid sources are rejected with
  `422` carrying the AOM2 rule code.

### Added

- **Version-tree branching and merge provenance** (RM common master06
  §Version tree / §Distributed versioning / §Version Merging). Branch
  version ids (`trunk.branch.version`) are now first-class on every
  surface: modifying a version that was imported from another system forks
  a branch with the local `creating_system_id` (the spec's mandated rule
  for local modifications of copied versions) while the imported trunk
  version stays the container current; branch tips are continued,
  superseded, read, exported, and re-imported like any version; the
  container current / `LATEST_VERSION` (including in AQL) is the latest
  *trunk* version. `ORIGINAL_VERSION.preceding_version_uid` is now stored
  at commit (previously synthesized) and `other_input_version_uids` (merge
  provenance) is accepted on the CONTRIBUTION wire, preserved on import,
  and served on read. The `vo_version` storage carries the version tree in
  explicit columns with per-lineage temporal non-overlap constraints and
  the spec's global version-identity uniqueness tuple.

### Changed

- **Stricter spec-mandated validation** on the commit path: a client
  `AUDIT_DETAILS` with an empty `system_id`, a committer
  `PARTY_IDENTIFIED`/`PARTY_RELATED` with no identity, an empty committer
  name, or a `PARTY_RELATED.relationship` outside the openEHR
  `subject_relationship` group is now rejected with 422 (previously
  accepted, or surfaced as a 500 DB error); a non-root RM node carrying
  `archetype_details` violates `LOCATABLE.Archetyped_valid` and is
  rejected; EHR-Extract `versions[]` members with a `_type` other than
  `ORIGINAL_VERSION` are rejected on import.
- AQL `VERSION` `uid` values are now built from each version's stored
  `creating_system_id` and version-tree id, not the server's live
  `system_id` configuration.

- The `ehrbase-rs-postgres` image now pre-creates the layered group roles
  (`ehrbase_migrator`, `ehrbase_app`, `ehrbase_reader`), so Compose/dev
  deployments get the same least-privilege grant topology as hardened
  deployments instead of `roles absent` startup notices. Existing data
  volumes keep working; recreate the volume (or create the roles once by
  hand) to pick the grants up.
- Public documentation website at <https://rubentalstra.github.io/ehrbase-rs/>:
  a product landing page, a versioned user guide (frozen per release, `dev`
  tracking `develop`), and an offline OpenAPI endpoint reference covering all
  seven openEHR API groups. Built from `website/` and deployed by CI, with
  link-check and OpenAPI-drift gates.

## [3.0.0] - 2026-07-11

First public release of **EHRbase-rs** — a pure-Rust openEHR Clinical Data
Repository. Version numbering starts at 3.0.0: this project began as a fork
of EHRbase (Java, 2.x line) and is released as its next-generation successor;
inherited upstream tags/releases were removed from the fork. Published as a
**pre-release**: the platform is feature-complete and conformance-verified,
but has not yet run in production.

### Added

#### openEHR platform
- openEHR REST API (ITS-REST 1.0.3): EHR, EHR_STATUS, COMPOSITION,
  DIRECTORY/FOLDER, CONTRIBUTION, QUERY, DEFINITION (ADL 1.4 + ADL2), admin
  and management surfaces, with canonical JSON **and** XML content
  negotiation. The wire contract is generated from the official openEHR
  OpenAPI/BMM/XSD models with a CI drift gate.
- AQL 1.1 query engine: typed path analysis over a spec-generated Reference
  Model compiled to PostgreSQL SQL; `LATEST_VERSION` **and** `ALL_VERSIONS`;
  terminology-backed `TERMINOLOGY()` expansion; stored parameterised queries.
- Full change-control semantics: contribution-atomic commits, indelible
  temporal version history (PostgreSQL 18 `WITHOUT OVERLAPS`), logical
  delete, attestations, per-version digital signatures (RFC 8785),
  point-in-time reads.
- Templates and validation: OPT 1.4 ingestion with artefact validity
  checking (AOM2 codes), WebTemplate / FLAT / STRUCTURED simplified formats,
  deep archetype-constraint validation on every commit.
- EHR Extract and messaging (SM I_EHR_EXTRACT/I_MESSAGE/I_TDD): whole-EHR
  export/import preserving distributed version identity, EHR cloning, TDD
  import.
- Demographics: versioned party store (PERSON, ORGANISATION, GROUP, AGENT,
  ROLE) with relationships.
- Terminology: the bundled openEHR terminology plus pluggable external FHIR
  terminology servers (validate / expand / subsume).
- Conformance instrument: the ECC runner executes the full catalogue (341
  cases, JSON + XML) against the composed server and computes profile
  verdicts — **CORE: PASS · STANDARD: PASS · OPTIONS: OBTAINED**, generating
  the Conformance Statement + Certificate.

#### Integration
- Change events: transactional outbox publishing every contribution commit
  to AMQP/RabbitMQ — at-least-once, per-EHR ordered, PHI-free envelopes,
  server-side filterable subscriptions (off by default).
- FHIR R4 connectors: mapping-driven inbound ingestion (validated
  compositions with FEEDER_AUDIT provenance), a read façade over AQL, and
  event-driven outbound resource emission (off by default).
- S3 multimedia externalization: threshold-based content-addressed offload
  of DV_MULTIMEDIA to any S3-compatible store with sha-256 integrity
  verification; SeaweedFS supported out of the box (off by default).

#### Security & operations
- Authentication: HTTP Basic (argon2) and OAuth2/OIDC bearer (Keycloak,
  Active Directory, any standards-compliant IdP).
- Authorization: RBAC plus ABAC via the embedded Cedar policy engine or a
  remote PDP.
- Multi-tenancy: each tenant an isolated logical openEHR system with its own
  `system_id`, enforced by PostgreSQL row-level security (off by default —
  single-tenant mode is unchanged).
- IHE ATNA system log: DICOM audit messages over (TLS) syslog with
  build-time operation coverage.
- Observability: structured logs, OpenTelemetry traces, Prometheus metrics,
  health probes; identified data never enters telemetry.
- Layered database roles (migrator / writer / reader) with a hardened
  PostgreSQL baseline.

#### Deployment
- Docker Compose stack (server + PostgreSQL 18) with an optional Grafana
  LGTM observability overlay.
- Distroless, non-root, shell-less multi-arch container images (amd64 +
  arm64) on GHCR.
- Helm chart with security-hardened defaults (non-root, read-only rootfs,
  seccomp, default-deny NetworkPolicy) and golden-render validation.

[unreleased]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.0...HEAD
[3.0.0]: https://github.com/rubentalstra/ehrbase-rs/releases/tag/v3.0.0
