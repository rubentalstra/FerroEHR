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

- **`ehrbase-admin-ui` — the admin console**, a new standalone web
  application (its own binary and OCI image,
  `ghcr.io/rubentalstra/ehrbase-rs-admin-ui`) that manages any
  ITS-REST-1.0.3 CDR strictly over its REST API. Pure Rust end to end
  (Leptos SSR + WASM, zero hand-written JavaScript). Feature set:
  dual Basic + OIDC login (credentials held server-side in the BFF),
  a dashboard (count tiles, query-group tiles, a commit-activity trend
  chart), a Template Manager (list/filter/upload OPTs with the CDR's
  validation diagnostics verbatim; per-template path-catalog tree, raw-OPT
  view, and format-switchable generated example), an EHR browser (finder,
  status/directory/compositions/contributions, and a composition viewer
  with canonical JSON/XML + FLAT/STRUCTURED toggle, version history, and
  audit details), a **point-and-click Query Builder** that assembles the
  real AQL AST (typed per-datatype criteria from the template's
  constrained value sets, nested AND/OR/NOT groups, projection columns,
  live AQL preview) and runs it via the Query API, a raw AQL editor with
  BFF-side grammar validation and parameter bindings, stored-query
  management with console-local query groups, and a system panel (CDR
  status, SMART discovery, the served OpenAPI rendered natively).
  Configured by one `ehrbase-admin-ui.toml` (+ `EHRBASE_ADMIN__*` env);
  ships in the quickstart compose as the `ehrbase-admin-ui` service on
  port 3000. Verified by a Rust-native browser E2E journey suite
  (merge-gating in CI, screenshots published as artifacts).

## [3.1.1] - 2026-07-17

### Fixed

- The release pipeline attaches the per-architecture server binary tarballs
  again: since the crate consolidation the binary is produced by the
  `ehrbase-server` package (the executable is still named `ehrbase`), but
  the release asset build still compiled the `ehrbase` platform library and
  failed — v3.1.0 published without binary assets. Container images were
  not affected. Use v3.1.1 for downloadable binaries.

## [3.1.0] - 2026-07-17

### Changed

- The FLAT and STRUCTURED (Simplified Formats) layer was rewritten against
  the official openEHR ITS-REST Simplified Formats specification: exact
  node-id generation, per-type attribute suffixes, the full `ctx/`
  vocabulary with its documented defaults, `|raw` embedding, and the
  `|other` open-value-set rules (invalid combinations are now rejected with
  `422` instead of being silently ignored). Unknown field identifiers in a
  simplified payload are now rejected rather than dropped.
- Format selection is done exclusively via the `Accept` and `Content-Type`
  headers on every endpoint that supports the simplified media types
  (`application/openehr.wt.flat+json`, `…wt.structured+json`, and
  `application/openehr.wt+json` for template rendering), with proper
  RFC 9110 q-value negotiation, `406`/`415` answers naming the supported
  formats, and simplified support on CONTRIBUTION payloads
  (`versions[].data`) with the envelope staying canonical.
- Committing a composition in a simplified format now requires the
  `openehr-template-id` request header (`422` without it, previously `400`);
  the undocumented `template_id` query parameter is no longer read.
- Content negotiation is strict everywhere: an `Accept` header that none of
  an endpoint's supported formats can satisfy is answered with `406`
  (previously some JSON-only endpoints leniently returned JSON), and the
  server's own generated OpenAPI now advertises the simplified media types
  on the composition, contribution, and template endpoints.
- Release builds now abort on integer arithmetic overflow instead of
  silently wrapping (`overflow-checks` enabled in the release profile) — a
  corrupted-value class of fault becomes a crash-and-restart instead of
  wrong clinical data.

### Removed

- The `ehrbase-quirks` cargo feature and its vendor-specific behaviours
  (alternate duplicate-id spelling, the non-standard `|unit_system` /
  `|unit_display_name` quantity suffixes) — the specification-defined
  behaviour is now the only behaviour.

### Fixed

- A tenant-resolution failure (tenant registry unreachable) now fails the
  request with `503` instead of silently serving it under the default
  tenant; unknown tenant keys keep the documented unscoped behaviour and
  are negative-cached.
- Audits for authenticated writes that carry no committal headers are now
  attributed to the authenticated user (Basic username / token subject, with
  the mechanism recorded as the identifier type) instead of the generic
  system identity.
- Multi-tenant deployments now actually run on the tenant-scoped connection
  pool: with `tenancy.enabled = true` every database connection carries the
  request's tenant for the row-level-security policies. Previously the
  binary always built the plain pool, so all requests fell through to the
  default tenant regardless of configuration.
- Multi-tenancy: a connection freshly opened by the pool while serving a
  request (pool growth under load) could miss the tenant stamp and run as
  the reserved default tenant — reads returning nothing and writes landing
  outside the caller's tenant. The tenant-scoped pool now stamps
  `ehrbase.tenant_id` both when a connection is opened and on every
  checkout, so every connection carries the caller's tenant. Deployments
  with `tenancy.enabled = true` should upgrade.
- The demographic APIs (party and relationship writes) now honour the
  `openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*` committal headers exactly
  as the EHR APIs do — a caller-supplied committer, description, and
  system id are merged into the stored version's audit.
- Direct COMPOSITION create/update/delete now honour the ITS-REST committal
  headers (`openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*`): a
  caller-supplied committer, audit description, change type, lifecycle
  state, signature, and attestations are merged into the stored version
  exactly as on the CONTRIBUTION path (previously the direct paths discarded
  them and always committed server defaults).
- The template store no longer double-reads the OPT XML when generating an
  example for a cold template, and template upload is a single atomic
  statement (the duplicate-check race window is gone).
- The event-outbox publisher declares its AMQP topology only on connect or
  subscription change (previously every poll cycle re-declared each queue),
  and the FHIR outbound emitter parks a persistently failing row after a
  bounded retry budget instead of blocking the stream forever.
- A FLAT/STRUCTURED composition body that parses as JSON but does not conform
  to its target template now returns `422 Unprocessable Entity` instead of
  `500 Internal Server Error` — such an input is client data, not a server
  fault. Output conversion of stored compositions remains a `500` on failure.
- Panicking request handlers and audit fail-closed (`503`) responses now
  carry the standard openEHR `{ error, message }` JSON error body (the audit
  `503` also carries `Retry-After`), instead of a plain-text body.
- A malformed `If-Match` header on a state-changing request is now rejected
  with `400 Bad Request` instead of being silently ignored — an unparseable
  precondition previously ran as if no `If-Match` was sent, opening a
  lost-update window. `If-Match: *` and valid version ids are unaffected.
- Database constraint and serialization/deadlock failures now surface as
  `409 Conflict`, and connection-pool exhaustion under load as `503 Service
  Unavailable` with `Retry-After`, instead of collapsing every database error
  to `500 Internal Server Error`.
- Stored-query and template metadata list/read endpoints no longer silently
  blank a field when a database column fails to decode; a decode failure now
  surfaces as `500` with a real error instead of an empty value.

### Changed

- The application is consolidated to two library crates plus a thin binary
  (`ehrbase` — the platform, `ehrbase-rest` — the ITS-REST adapter,
  `ehrbase-server` — the binary): the `ehrbase-sm` trait catalog is gone,
  the REST adapter calls the concrete platform service directly, and the
  full configuration tree (`[server]`, `[auth]`, `[authz]`, `[smart]`,
  `[management]`, `[tenancy]`, `[admin]`) is defined in the platform crate.
  The served wire, the `ehrbase.toml` schema, and the container entrypoint
  (`ehrbase`) are unchanged.
- Bundle-backed terminology lookups and template/query validity checks are
  now synchronous in-process calls (no behaviour change on the wire).
- Every versioned write now commits through the single folded
  audit+contribution+version statement even with digest signing enabled
  (the commit instant is read up front with the placement, so the signature
  is computed before any insert); version-tree placement is one read instead
  of three, and contribution commits batch their target pre-reads. Fewer
  round trips per write, identical wire behaviour and stored semantics.
- The OpenAPI documents (the composed `openapi.json` and the twelve Swagger
  spec-selector family documents) and the SMART `.well-known/smart-configuration`
  discovery document are now built once at server startup instead of being
  regenerated on every request. No change to the document content.

### Added

- External terminology providers cache their FHIR operation results
  (`$validate-code`/`$expand`/`$subsumes`/`$lookup`) for a configurable TTL
  (`[terminology.external.providers.<name>] cache_ttl_secs`, default 300 s,
  `0` disables; `cache_capacity`, default 10000) — a validation burst over
  the same codes costs one remote round trip per window instead of one per
  code.
- A new `atna_audit_serialize_failed_total` metric counts ATNA audit records
  dropped because the message failed to serialize, so audit loss is always
  metered.

## [3.0.3] - 2026-07-16

### Changed

- The served OpenAPI documents now categorize operations the way the
  official ITS-REST reference documents do: standard-group operations are
  tagged by resource (EHR, EHR_STATUS, COMPOSITION, DIRECTORY, CONTRIBUTION,
  ITEM_TAG; PERSON, AGENT, GROUP, ORGANISATION, ROLE, VERSIONED_PARTY;
  ADL 1.4, ADL 2, Query) instead of one flat tag per API group, and the
  Swagger UI spec selector offers one document per API family — the five
  standardised openEHR groups and the seven server-extension families —
  plus the complete composed surface, all filtered from the server's own
  generated document.

### Fixed

- Duplicate-template-id fixture resolution in the validation corpus test is
  now deterministic (sorted path order) instead of OS-dependent `read_dir`
  order, fixing a Linux-only CI failure.

## [3.0.2] - 2026-07-15

### Changed

- The benchmark instrument measures both comparison stacks under a fairer,
  more deterministic protocol: the databases get a 1 GB `/dev/shm` floor
  (Docker's 64 MB default starved PostgreSQL's parallel workers mid-run),
  maintenance debt is settled with `VACUUM ANALYZE` after seeding and
  between ladder rungs (autovacuum no longer lands inside measured
  windows), the ladder drains in-flight backlog between rungs, and the
  measured cold start no longer includes building the ehrbase-rs container
  image. Ladder output prints latencies in magnitude-appropriate units
  (µs/ms/s), and the generated comparison page reports clinical events per
  minute beside request rates.
- **Configuration is now one `ehrbase.toml`.** The whole server is configured
  by a single TOML file (sections `[server]`, `[db]`, `[log]`, `[telemetry]`,
  `[auth]`, `[authz]`, `[admin]`, `[tenancy]`, `[smart]`, `[management]`,
  `[signing]`, `[query]`, `[events]`, `[fhir]`, `[terminology]`,
  `[multimedia]`, `[atna]`, `[subject_proxy]`), discovered from `--config`,
  `EHRBASE_CONFIG`, `./ehrbase.toml`, or `/etc/ehrbase/ehrbase.toml`. Every
  `EHRBASE_*` environment variable is now a mechanical per-key override:
  `EHRBASE` + the TOML path, upper-cased, with `__` between every segment
  including after the prefix
  (e.g. `EHRBASE__DB__MAX_CONNECTIONS`, `EHRBASE__AUTH__OIDC__ISSUER`). This
  replaces the previous ~14 independent per-subsystem loaders and their
  several env-name grammars. **Old spellings are not aliased** (greenfield —
  nothing is deployed to migrate): a pre-redesign variable fails at boot with
  the exact uniform replacement suggested (e.g. `EHRBASE_DB_MAX_CONNECTIONS`
  → "did you mean `EHRBASE__DB__MAX_CONNECTIONS`?"). `DATABASE_URL` and
  `RUST_LOG` remain permanent conventional aliases. New `ehrbase config
  default` prints an annotated template and `ehrbase config check` validates a
  config (and prints the effective, secret-redacted result) without a
  database. The compose stack, Helm chart, and docs all move to the new file +
  spellings; the PostgreSQL-init container variables `EHRBASE_DB_USER` /
  `_PASSWORD` / `_NAME` were renamed `PG_INIT_USER` / `_PASSWORD` / `_DB` so
  they no longer collide with the server's reserved `EHRBASE_` namespace.

### Removed

- The nine per-subsystem `EHRBASE_*_CONFIG` file pointers
  (`EHRBASE_REST_CONFIG`, `EHRBASE_AUTHZ_CONFIG`, `EHRBASE_ATNA_CONFIG`,
  `EHRBASE_SIGNING_CONFIG`, `EHRBASE_EVENTS_CONFIG`,
  `EHRBASE_FHIR_OUTBOUND_CONFIG`, `EHRBASE_MULTIMEDIA_CONFIG`,
  `EHRBASE_VALIDATION_CONFIG`, `EHRBASE_MANAGEMENT_CONFIG`,
  `EHRBASE_SUBJECT_PROXY_CONFIG`): merge each file's contents into the single
  `ehrbase.toml` under its `[section]`.
- `EHRBASE_REST_AUTH__ADMIN_SCOPE`: subsumed by `authz.rbac.admin_role`.

### Fixed

- Unknown or misspelled configuration is now rejected at boot with a
  did-you-mean suggestion (and the `file:line` for a file key) — previously a
  typo'd TOML key or `EHRBASE_*` variable was silently ignored, so a
  not-applied security setting could pass unnoticed.
- The documented `EHRBASE__SUBJECT_PROXY__SYSTEMS__<name>__BASE_URL` env form
  now actually binds — the old loader stripped the prefix such that this
  spelling was dead, so subject-proxy systems could only be set via a file.
- Unparseable `[query]` values (`query.plan_cache_capacity`, `query.timeout_ms`)
  now error at boot instead of silently falling back to defaults.
- The Swagger UI works again and now documents the **complete server
  surface** from one natively generated OpenAPI document. `…/rest/swagger-ui`
  previously entered an infinite redirect loop (the UI's trailing-slash
  redirect fought the server's path normalization) and its OpenAPI document
  was an empty stub. The UI now loads directly (documentation URL corrected to
  `/ehrbase/rest/swagger-ui`), and its spec selector has a single entry,
  `ehrbase-rest`, generated by the server itself (`utoipa-axum`, one
  `#[utoipa::path]` handler per operation, so route and documentation cannot
  drift): every ITS-REST API group (EHR, COMPOSITION, CONTRIBUTION, DIRECTORY,
  DEMOGRAPHIC, DEFINITION, QUERY, ADMIN) plus the server's own extensions
  (terminology, PARTY_RELATIONSHIP, event-subscription, multi-tenancy, FHIR
  connector) and its operational endpoints (status/health, management, SMART
  discovery, the OpenAPI endpoints). No vendored OpenAPI is served. The
  document also declares the server's **configured** authentication scheme so
  the "Authorize" dialog and per-endpoint padlocks match the running server:
  HTTP Bearer (JWT) when OIDC is configured, otherwise HTTP Basic, and none
  when authentication is disabled — never both at once.

## [3.0.1] - 2026-07-14

### Added

- The server now prints an ASCII-art startup banner to stdout before the
  structured startup logs: the `EHRbase-rs` wordmark, the running version, the
  maintainer credit (Ruben Talstra), the project URL, and the load-bearing
  spec/platform pins (openEHR RM 1.2.0 · ITS-REST 1.0.3 · AQL 1.1 ·
  PostgreSQL 18). The banner is suppressed under JSON logging
  (`EHRBASE_LOG_FORMAT=json`) so machine log consumers see only structured
  lines.
- AQL queries are now planned once and cached: a repeated ad-hoc or stored
  query text reuses its lowered plan instead of re-parsing and re-analysing on
  every execution, while per-request parameter values, `fetch`/`offset`
  paging, and EHR scope still bind independently. Queries that resolve
  terminology (`matches TERMINOLOGY(…)`) are never cached, so their expansion
  is always current. New configuration knob
  `EHRBASE_QUERY__PLAN_CACHE_CAPACITY` (default `256`; `0` disables the cache)
  bounds how many distinct plans are held, and a new `aql_plan_cache_events_total`
  metric (`event` = `hit`/`miss`) reports cache activity.

### Fixed

- The composition validator no longer falsely rejects templates that use the
  same archetype more than once under one container, differentiated by name:
  each instance is now routed to the sibling constraint whose name it
  satisfies, instead of being checked against the first same-archetype
  sibling's overlay. Cross-contaminated content (a child from one overlay
  placed in the other-named instance) is still rejected.
- Template example generation (`GET …/example`) at `detail_level=medium` and
  `complete` no longer produces an empty composition for templates whose
  content is entirely optional: `medium` now returns a fully-populated
  single-instance committable example (honouring temporal patterns,
  C_DURATION field patterns, media-type code lists, and container
  cardinality bounds), and `complete` additionally demonstrates a second
  occurrence of repeating nodes. `required` (the default) is unchanged.
- AQL `SELECT c/uid/value` (and `c/uid`) on a COMPOSITION — or any
  versioned-object root — now returns the server-assigned
  `OBJECT_VERSION_ID`, version-correct under `LATEST_VERSION` and
  `ALL_VERSIONS`. It previously returned `null` because the uid was
  injected only on REST reads, never into stored data. (QUERY master03
  lists `COMPOSITION.uid.value` as a normative identified path.)
- Composition commits against an already-seen template no longer re-read the
  stored OPT from the database on every commit — the built WebTemplate cache
  is now consulted first (measured: 10,206 redundant reads in a 120 s load
  window, the #2 database statement by total time). Deleting a template now
  also evicts it from that cache, so a commit racing a delete gets the
  correct `422` ("template not known") instead of a foreign-key `500`.

### Changed

- Basic-auth verification no longer re-runs the Argon2 password hash on
  every request: verified credentials are cached (as a SHA-256 digest,
  never plaintext) for `EHRBASE_REST_AUTH__VERIFIED_CACHE_TTL_SECONDS`
  (default 60 s; `0` disables), and cache misses hash on a background
  thread. At load this removes roughly a full CPU core of per-request
  hashing.
- Composition create/update responses are built from the commit result
  instead of re-reading the just-written document from the database — one
  connection acquisition and two queries fewer per write; when version
  signing is disabled the server also no longer rebuilds the full document
  it would only have signed. Response bodies and headers are unchanged.
- Storage: the version table's two GiST exclusion constraints and two
  speculative JSONB indexes on the node table (a GIN over every fragment and
  a magnitude expression index — no query the engine generates could use
  either) were removed; version-validity non-overlap is unchanged and held
  by construction (one open row per lineage via unique indexes, atomic
  close-then-insert writes, and an overlap audit on archive load). This
  removes the dominant per-commit index-maintenance and lock-contention
  costs on the write path.
- Connection-pool defaults changed: `EHRBASE_DB_MAX_CONNECTIONS` 10 → 20,
  `EHRBASE_DB_MIN_CONNECTIONS` 0 → 2, and the per-checkout liveness ping is
  disabled (a broken connection is detected by its first statement).
  `TCP_NODELAY` is now set on accepted sockets, removing Nagle-induced
  latency on small responses.
- Composition commits make fewer database round trips: the audit and
  contribution rows are written in one statement, and the create-path EHR
  existence + modifiability gates are one read instead of two. Error
  behaviour is unchanged (a missing EHR is still `404` before a
  non-modifiable `409`).
- The transactional event outbox is no longer written on every commit when no
  eventing consumer is configured. The per-commit `event_outbox` row (and its
  envelope serialization) is now written only when the AMQP publisher
  (`EHRBASE_EVENTS_ENABLED`) or the FHIR outbound emitter
  (`EHRBASE_FHIR_OUTBOUND_ENABLED`) is enabled. Consequence: the outbox
  records commits made while a consumer is enabled (at-least-once, even with
  zero bound subscribers — the gate is the boot-time config, not the current
  subscriber set); commits made while every consumer was off are not
  back-filled if eventing is later enabled.
- IHE ATNA login ("Application Activity") records now mark genuine
  authentication events rather than every authenticated request. A login
  record is emitted only when the request actually verified credentials (a
  Basic verified-credential cache miss); a cache hit continues an established
  session and a Bearer request authenticated out of band at the OIDC provider,
  so neither mints a per-request login record. Rejections (401/403) are still
  always audited, and login records remain off by default
  (`EHRBASE_ATNA_SUPPRESS_LOGIN_EVENTS`, default `true`).
- Per-EHR `EHR_ACCESS` access-settings are cached as default-open at EHR
  creation, so the access gate's first check on a freshly created EHR no
  longer costs a database lookup (a hospital-day workload creates EHRs
  constantly). Importing an `EHR_ACCESS` version into an existing EHR now
  evicts that cache entry, so the access decision reflects the imported
  policy immediately.
- Composition validation is substantially faster with identical outcomes:
  the RM-invariant pass validates each node directly against the
  spec-generated Reference Model instead of deserializing every node into
  its typed struct (falling back to the typed path for anything it cannot
  vouch for), the archetype-constraint walk reuses constraint paths parsed
  once per cached WebTemplate instead of re-parsing them on every node
  visit, and validation error messages are byte-for-byte unchanged
  (equivalence is pinned by tests across the full corpus). Measured
  end-to-end: a fully populated International Patient Summary validates in
  well under half its previous time.

### Added

- Storage migration `0008`: a promoted `context_start timestamptz` column on
  COMPOSITION root node rows (backfilled from stored data, partially
  indexed), plus the fail-safe `ext.openehr_timestamp` conversion function.
  The AQL engine reads the indexed column for
  `ORDER BY`/`WHERE` on `c/context/start_time/value` — the measured
  patient-dashboard hot path — instead of re-extracting JSONB per candidate
  row; results are unchanged, including NULL placement and the verbatim
  projected value.
- Overload backpressure: the REST server now caps the number of API requests
  it handles concurrently and sheds the excess immediately with
  `503 Service Unavailable` + `Retry-After: 1` instead of queueing every
  request until it runs out of memory. Under sustained offered load beyond
  database capacity the server now degrades with clean errors rather than
  being killed. The cap is configurable via `EHRBASE_REST_MAX_IN_FLIGHT`
  (concurrent requests, not per second; default 256, raise for
  high-throughput deployments; `0` disables shedding). The `/status`, health,
  and discovery
  endpoints are never limited, so operators can always probe an overloaded
  server. (No openEHR spec governs overload behaviour; the `503` follows
  RFC 9110 §15.6.4.)
- Conformance framework (`tools/conformance`) redesigned and rewritten from
  the openEHR CNF component up (W-10). It now assesses **any** openEHR CDR:
  point it at a deployed server (`scripts/conformance.sh` with
  `CONF_SUT=byo CONF_BASE_URL=…`, or the CLI's `--sut byo --base-url …`) and
  receive the full spec-cited artefact set — `results.json`, a conformance
  report, a Conformance Statement, a Conformance **Certificate** (a
  machine-computed framework assessment, explicitly not an official openEHR
  certification), and badges, written per SUT. Upstream EHRbase (Java) is a
  built-in target (`CONF_SUT=ehrbase-java`) with a committed fairness
  register; a cross-SUT comparison matrix can be rendered from two or more
  runs (`conformance compare`). Assertions carry a **spec-edition ladder**:
  the runner tries the newest edition form first (weak `W/"…"` ETags,
  RM 1.2.0 wire) and steps down to Release-1.0.3-era forms, reporting the
  satisfied edition level per case instead of failing a CDR on edition
  deltas; ehrbase-rs CI runs stay pinned to the development edition so the
  ladder can never mask a regression.

- AQL: `OR`-combined `CONTAINS` expressions now execute (previously rejected
  as unsupported), including nested `AND`/`OR`/`NOT` containment trees, and
  `NOT CONTAINS` accepts compound operands.
- ATNA auditing: EHR-Extract export and import operations now emit audit
  events (object class `Extract`) when auditing is enabled.
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

- Version lifecycle states are now enforced as a state machine (RM common
  §Version Lifecycle): a commit whose `lifecycle_state` is not a legal
  transition from the preceding version's state (for example
  `incomplete` → `inactive` without completing first) is rejected `422`.
- Template identifiers now compare case-insensitively (case-preserving):
  lookups accept any casing and uploading a case-variant duplicate is a
  `409` conflict, backed by a unique index (new migration).
- AQL `MIN`/`MAX` aggregate over non-numeric leaves (text, dates, times)
  now compares type-appropriately instead of forcing a numeric cast, and
  mixed-type leaf comparison dispatches numerically for numbers.
- Contribution commits now verify the target EHR exists (`404` otherwise)
  and honour the `EHR_STATUS.is_modifiable = false` write guard and
  versioned-composition invariants on every path, including
  CONTRIBUTION-wrapped commits. Re-creating an existing directory (a folder
  hierarchy with the same root archetype and name) via a CONTRIBUTION is a
  `409` conflict; a hierarchy with a distinct root remains a new
  `EHR.folders` member.
- EHR-index errors now carry the precise SM error names
  (`ehr_id_does_not_exist`, `subject_id_does_not_exist`) instead of a
  generic not-found.
- Contribution retrieval now lists versions affected by `attestation`-only
  items alongside committed versions for demographic contributions,
  matching the EHR-scoped behaviour.
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

### Fixed

- Template example generation (`GET /definition/template/adl1.4/{id}/example`)
  now honours the template's structural constraints: a missing mandatory
  ENTRY structure (e.g. `ACTION.description`) is synthesized with the
  template's constrained node (its RM type, `archetype_node_id`, and name)
  instead of a blind `at0001` placeholder, so generated examples validate
  and commit against the same template. Surfaced by the official openEHR
  CKM **International Patient Summary** template; probed by the new
  conformance case ECC-TPL-017 (example → commit round-trip).
- Template list endpoints no longer ignore filter and pagination
  parameters.
- The conformance manifest and `/rest/status` no longer misreport the
  implemented ITS-REST edition as `1.0.3`.
- Contribution commits: a creation version against an already-existing
  object, and a modification/deletion/attestation whose
  `preceding_version_uid` names an object the server does not hold, now
  return `400` (the contract's modification-type-mismatch scope) instead of
  `422`/`404` — on `POST /ehr/{ehr_id}/contribution`, `404` is reserved for
  an unknown `ehr_id`.
- Versioned-object reads (`GET …/versioned_composition`,
  `…/versioned_ehr_status`, versioned directory) now emit the concrete RM
  class (`VERSIONED_COMPOSITION` / `VERSIONED_EHR_STATUS` /
  `VERSIONED_FOLDER`) in `_type`, not the abstract `VERSIONED_OBJECT`.
- Demographic API: `If-Match` preconditions now verify the full
  `OBJECT_VERSION_ID` (previously only the version-tree number, which
  accepted phantom versions); relationship delete now honours the same
  `If-Match` preconditions as party delete; demographic `ETag`s are emitted
  in the weak form (`W/"…"`).

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

[unreleased]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.1.1...HEAD
[3.1.1]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.1.0...v3.1.1
[3.1.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.3...v3.1.0
[3.0.3]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.2...v3.0.3
[3.0.2]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.1...v3.0.2
[3.0.1]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.0...v3.0.1
[3.0.0]: https://github.com/rubentalstra/ehrbase-rs/releases/tag/v3.0.0
