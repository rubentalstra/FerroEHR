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

## [3.14.0] - 2026-07-30

### Fixed

- **AQL VERSION coded-field predicates respect their sub-paths.** Predicates
  on `commit_audit/change_type` and `lifecycle_state` compared every
  sub-path against the stored numeric code, so the rubric form
  (`…/value='creation'`) silently never matched. The three defined
  sub-paths now compare correctly (`defining_code/code_string` against the
  code, `value` against the terminology rubric, `terminology_id/value`
  against `openehr`); other suffixes are clean invalid-query rejections.
- **AQL `SELECT DISTINCT` with `ORDER BY` executes correctly.** Sorting a
  DISTINCT projection by one of its selected columns previously failed with
  a database error surfaced as HTTP 500; it now orders by the output column.
  Sorting a DISTINCT projection by an expression that is not selected is a
  clean invalid-query rejection (the AQL specification defines no semantics
  for it) instead of a 500.
- **AQL date/time functions work in temporal comparisons.** Comparing a
  temporal path against `NOW()`/`CURRENT_DATE_TIME` etc. previously failed
  with a database type error surfaced as HTTP 500; function operands now
  join the comparison in the same coercion space as literals.
- **Comma-fraction ISO 8601 timestamps compare correctly in AQL.** Canonical
  `DV_DATE_TIME` values using the ISO-permitted comma decimal sign
  (`21:22:19,501+00:00`) were silently excluded from temporal comparisons
  (and their promoted index column stored NULL). Both the write-time
  promotion and the query-time casts now normalize the comma form.
- **AQL coded-name node predicates match correctly.** The name term-code
  shortcut (`[at0002, snomed_ct(3.1)::313267000]`, and the
  `terminology::code|informational text|` form) was compared as one raw
  token against `code_string` and could never match. It now decomposes per
  the AQL specification's canonical expansion: `code_string` and
  `terminology_id/value` are compared separately, the informational `|…|`
  tail is ignored, and a bare at-code name operand asserts the archetype's
  `local` terminology.

### Changed

- **`TOP n BACKWARD` is now rejected with rewrite guidance.** The deprecated
  direction variant previously returned the *first* n rows silently. The
  server now refuses it as an invalid query whose message shows the
  recommended rewrite (`ORDER BY <path> DESC LIMIT n`). Plain `TOP n` and
  `TOP n FORWARD` are unchanged.

## [3.13.0] - 2026-07-30

### Added

- **The ISO 8601 date/time/duration types implement their computational
  functions** (BASE `foundation_types/master06-time_types.adoc`
  §Computational Functions + the four `Iso8601_*` class definitions): the
  DEFINITE `add`/`subtract`/`diff` on dates, times and date/times — a
  duration reduced to exact seconds with the `Time_definitions`
  `Average_days_in_year`/`Average_days_in_month` lengths — and the NOMINAL
  `add_nominal`/`subtract_nominal`, which advance the calendar to the same
  day-of-month and clamp it down where the target month is shorter (29 Feb
  `++ P1Y` → 28 Feb, 31 Jan `++ P1M` → 28/29 Feb). Durations gain
  `add`/`subtract`/`multiply`/`divide`/`negative`. Also added across the four
  types: `as_string` (the value in extended format), `is_extended`,
  `is_decimal_sign_comma` and `has_fractional_second`. Arithmetic on a
  partial value, or a result outside the representable 0000–9999 year range,
  is reported as no result rather than an invented one.

- **openEHR path expressions support general comparison predicates** (BASE
  architecture overview §Paths and Locators, "Other Predicates"): path
  predicates of the form `[at0007 and time >= '2005-06-24T09:30:00']` or
  `[value/defining_code/code_string = 'A04']` — a relative attribute path,
  an operator (`=`, `!=`, `<`, `<=`, `>`, `>=`), and a quoted-string or
  numeric literal — now parse and evaluate everywhere RM paths are resolved,
  including `ehr:` URI resolution. Strings compare lexically (ISO 8601
  date/times order temporally), numbers numerically, with XPath existential
  node-set semantics; predicate text outside the grammar is still rejected
  loudly. Previously these spec-defined forms were refused as unsupported.

## [3.12.0] - 2026-07-29

### Changed

- **The conformance verdict model no longer has excused capability states.**
  The published reports and certificates previously carried two
  non-verdict evidence tokens — `unrealized` (every case excused by a
  register citation) and `no_cases` (a claim the catalogue named no case
  for). Both are deleted: the catalogue gates now refuse those shapes before
  any server is assessed, and every capability a party claims is reported as
  exactly one of passed / failed / inconclusive / not-evidenced. A required
  capability without passing evidence now fails its tier with no excuse arm,
  for every assessed party alike; both committed records and the published
  comparison were re-derived under the stricter model (no tier verdict
  changed for either party).
- **An empty TDD batch answers `200` with `[]` instead of `201`.**
  `POST /message/tdd/{ehr_id}/batch` with an empty array creates nothing, and
  `201 Created` reported a creation that did not happen. Batches with members
  are unaffected.
- **`EXTRACT_SPEC.extract_type` now accepts every code the openEHR Reference
  Model names.** `POST /message/export` previously refused
  `openehr-synchronisation` and `openehr-generic` — two of the five extract
  types the RM's EHR Extract chapter lists by example — as out of group.
  Both are accepted now, alongside `openehr-ehr`, `openehr-demographic`,
  `generic-emr` and the catch-all `other`.

- **Conformance: a product is no longer excused from 186 test cases for
  declaring an older REST release** (#635). Every conformance case may declare
  the openEHR release its behaviour needs, and systems declaring an earlier
  release are skipped for it. That declaration had been copied onto 343 cases
  as authoring boilerplate, which quietly wrote off most of the EHR,
  COMPOSITION, DIRECTORY, CONTRIBUTION, QUERY and template surface for any
  product declaring ITS-REST 1.0.3 — behaviour those products do implement and
  should be judged on. Each case was re-derived against the released
  amendment record, and the requirement is kept only where the released text
  actually dates it: ITEM_TAGs, the Demographic API, admin EHR deletion, the
  Simplified Formats media types, `Prefer: return=identifier`, the
  audit-details `system_id`, the reserved `aql` query name, the template
  `/example` sub-resource, and SMART on openEHR. Everything else is now judged
  for every product, with the two genuinely release-dated header rules (the
  weak `W/` ETag form and the read/delete `Location` restriction) still
  applied only to the release that introduced them. No test was removed or
  weakened; the comparison against other openEHR products now covers what they
  really implement.

- **The conformance statement no longer claims nine capabilities it cannot
  demonstrate** (#623). ADL 1.4 and ADL 2 archetype provisioning, the admin
  Activity Report, EHR dump/load, EHR and demographic archiving, EHR Extract,
  TDS, and the MESSAGE API were all being claimed while every one of their
  test cases was excused: openEHR's released REST API publishes no endpoints
  for them, and EHRbase-rs exposes none of its own either — the underlying
  service methods exist, but nothing reaches them over HTTP, so a conformance
  runner has nothing to drive. Claiming a capability is the obligation to
  prove it, so those claims are withdrawn until the routes exist. Nothing was
  removed from the product; what changed is that the published statement now
  only claims what can be demonstrated.

- **Helm chart and operator-facing comments cite durable references** (#322).
  The chart's `values.yaml`, `Chart.yaml` and post-install NOTES pointed at
  internal design documents that no longer exist (the deleted design and
  enterprise doc trees) and at retired decision-record numbers. Each is now
  either the official upstream documentation it was standing in for (the
  PostgreSQL docs for the unprivileged app role and the `lock_timeout`
  migration wrapper, the Kubernetes Pod Security Standards for the container
  security posture), an explicit "our own extension, no openEHR spec governs it"
  flag on the optional integrations, or the rationale written out inline —
  so an operator reading the chart is never sent to a dead path. No default
  value, template, or rendered manifest changed.

- **The published per-chapter outcome bars are now a two-level chart with no
  `Other` bucket** (#613). The single bar per schedule chapter hid the EHR
  chapter's hundreds of cases behind one rectangle and swept the System API
  and anything unrecognised into an `Other` row. The chart now renders a
  chapter header carrying the chapter's total above one scaled bar per
  **band** — the surface a case actually exercises (EHR resource /
  EHR_STATUS / COMPOSITION / DIRECTORY / CONTRIBUTION / item tags / revision
  history, ADL 1.4 vs ADL 2 vs stored queries, ad-hoc vs stored query
  execution, parties vs relationships vs versioned party, and so on) — with
  the exact passed / FAILED / errored / cited-N-A counts printed beside every
  row, so a small band never loses its numbers to a short bar. Cited-N/A
  segments carry a hatch texture so "not executed, with a citation" can read
  as neither a pass nor a failure. The taxonomy is **total**: every case id
  maps to a named band and an unmapped id fails the render naming the id,
  rather than landing in a silent bucket. Both published SUTs render the same
  bands — a band with no case shows as an explicit `no cases` row — so the
  comparison page reads band-for-band.

- **The conformance pipeline now exercises BOTH claimed version-signing modes
  in every run, in the one committed record** (#609). openEHR defines a
  version signature at two depths of one mechanism — a plain digest (an
  integrity check) and an openPGP RFC 4880 signature (which additionally
  authenticates the author) — and a running server does one or the other. The
  product claims both, so `scripts/conformance.sh` now brings up a **second
  deployment of the same built image** in the openPGP posture alongside the
  standard stack (its own compose project, host port 8081), and the party's
  `ixit.json` declares it as an extra instance carrying its own signing block.
  The openPGP signature cases address that instance; one run, one
  `results.json`, and the Signing capability's evidence covers both modes.
  Consequently the `CONF_SIGNING_MODE=pgp` environment switch and the separate
  `ixit.pgp.json` party file are **removed** — there is nothing left to select,
  both modes always run. A conformance target that declares no such instance
  (upstream EHRbase) has the openPGP cases recorded not-applicable with that
  citation instead of failed, which is also now true for any case addressing an
  instance a party does not declare: it is excused at selection time rather
  than surfacing as an inconclusive row.

- **The conformance suite proves EHR-scoped querying against two EHRs, not
  one** (#604). The four cases that check a query is confined to the EHR named
  in the `openehr-ehr-id` request header used to run against a server holding
  a single EHR, so a server that ignored the header returned the same rows and
  passed. Each now creates a second EHR with its own content first: an
  unscoped answer carries the extra row and fails the case. The behaviour
  being checked is unchanged; the check can no longer be satisfied by
  accident.

- **The conformance suite now names a malformed request and invalid content
  differently everywhere** (#605). Fifteen more conformance cases used to
  report a rejected request as a content-validation failure when what the
  request actually broke was its own syntax — an unparseable template upload,
  a path segment that is not an identifier, a `version_at_time` outside the
  ISO 8601 form the specification mandates, or a tag list sent as something
  other than a list. Those now report as malformed requests. Nothing changes
  on the wire (all fifteen answered `400 Bad Request` before and after) and
  no server passes or fails differently; the published conformance report and
  case records simply name one rejection law one way, so a reader can tell the
  two families apart.

- **Two behaviours the conformance suite used to treat as optional are now
  required of every server** (#556). openEHR publishes its REST
  specification as normative prose *and* as OpenAPI files, and the prose is
  silent on more than it looks. Where the prose says nothing, those OpenAPI
  files are now read as part of the specification rather than set aside — so
  behaviours previously recorded as "the specification does not say" turn out
  to be specified after all, and the suite stops excusing them. Two change
  how a server is judged. Uploading an operational template under a
  `template_id` that already exists must answer `409 Conflict`; it was
  previously a declared choice between refusing and silently replacing the
  stored template, and a server could opt out of the refusal. Updating a
  COMPOSITION whose request body carries a `uid` naming a different version
  container than the URL must be rejected; the mismatch was previously
  reported without affecting the verdict. Both are now gating conformance
  cases. The published conformance artifacts and the ambiguity register
  record the specification sentence behind each.

- **The published Conformance Statement now declares the non-openEHR surface
  this server serves** (#527). A new "Additional non-openEHR surface" section
  lists every extension route family — health, status, the OpenAPI/Swagger
  meta routes, management, terminology, event subscriptions, multi-tenancy,
  the FHIR R4 connector and its mapping store, the ITI-81 audit read,
  `PARTY_RELATIONSHIP`, the bare stored-query list, the admin
  template/query/config routes and SMART discovery — with the routes it
  serves and the configuration that enables it. The section states plainly
  that none of it is part of any conformance claim: no openEHR specification
  governs these routes, no conformance case exercises them, and no verdict
  depends on them. A reader of the statement no longer has to discover the
  extension surface on the wire.

- **Canonical-XML support is now declared per resource family in the
  Conformance Statement, instead of being assumed for every resource**
  (#572). The openEHR release publishes an XML document element for only
  eight names — `composition`, `version`, `items`, `template`, `extract`,
  `extract_request`, `versioned_object`, `archetype` — while its REST API
  addresses `application/xml` to the whole resource surface. For a resource
  with no published document (EHR, EHR_STATUS, the directory FOLDER, the
  demographic party types, CONTRIBUTION) the specification therefore neither
  requires a server to serve XML nor forbids it, so the suite no longer
  asserts either answer: the statement declares, per family, whether this
  server offers canonical XML there, and the conformance run judges the
  matching branch — the XML read, or the `406 Not Acceptable` refusal the
  specification designates for an `Accept` a service cannot fulfil. This
  server declares XML support for EHR, EHR_STATUS, directory and the party
  families and declares it unsupported for CONTRIBUTION reads, exactly as it
  behaves. The full per-resource classification, its citations and the
  upstream report asking openEHR to reconcile the two inventories are
  recorded in the conformance ambiguity register as `AMB-167` / `UPR-127`.


- **Stored top-level objects now carry their copied `uid` at commit time**
  (#439). The full three-part `OBJECT_VERSION_ID` is stamped into the
  canonical body before it is decomposed, signed, and stored, so the
  contained object served inside an ORIGINAL_VERSION envelope, the bare
  resource reads, AQL projections, and EHR Extract exports all carry the
  identical uid value (ITS-REST overview *Resources* §Identifier types).
  Previously the uid was injected only on some read paths; clients now see
  one consistent shape everywhere. Imported (EHR Extract) content is
  exempt — its bodies are preserved verbatim.
- **`EHR.ehr_status` references the version container by its
  `HIER_OBJECT_ID`** (#426). The served EHR body's `ehr_status` OBJECT_REF
  (typed `VERSIONED_EHR_STATUS` per the RM invariant) previously carried an
  `OBJECT_VERSION_ID` naming one version — inconsistent with the sibling
  `ehr_access` ref and with the RM's container semantics (`OBJECT_REF.id` is
  the id of the referenced object; the referenced object is the
  VERSIONED_EHR_STATUS, whose uid is a `HIER_OBJECT_ID`). Both refs now
  carry the container id. Clients that read the current EHR_STATUS version
  uid from the EHR body must fetch `GET /ehr/{ehr_id}/ehr_status` and use
  its own `uid` instead.

### Fixed

- **ADL 1.4 archetypes with anonymous archetype slots are accepted.** The
  ADL 1.4 specification writes archetype slots without a node id in its own
  examples (`allow_archetype OBSERVATION occurrences matches {0..1} …`), and
  published CKM archetypes use that form — but the parser demanded
  `[atNNNN]` and refused such sources with a syntax error, so
  `POST /definition/archetype/adl1.4` answered `422` for spec-valid
  archetypes. Both the anonymous and the identified slot forms now parse;
  ADL 2 sources still require the node id, as ADL 2 defines.
- **An empty TDD batch aimed at a non-existent EHR is now refused.**
  `POST /message/tdd/{ehr_id}/batch` verified the target EHR once per
  document, so a batch with no documents answered success without checking
  the EHR at all. The target is now verified for every batch, empty ones
  included, and an unknown EHR answers `404`.

- **An activity-report request whose time interval runs backwards is now
  refused instead of answered.** `GET /admin/report/*` accepts an optional
  `time_interval=<lower>/<upper>`; a pair bounded on both sides with the lower
  bound *after* the upper one is not an interval at all (the openEHR BASE
  `Interval` invariant requires `lower <= upper`), and the server used to run
  it anyway and hand back the empty count such a range selects — a
  truthful-looking answer for a window nobody asked for. It is now `400`.
  Equal bounds remain a legitimate single-instant interval.

- **A corrupt dump archive no longer reports as an internal server fault.**
  `POST /admin/load` against a location whose manifest or segment is mangled
  or truncated used to surface as an unexpected-exception `500`, while a
  location holding no archive at all reported the openEHR service model's
  `file_not_writable`. Both are the same fact — the location does not hold a
  readable archive — and both now report `file_not_writable`, with nothing
  loaded either way.

- **An export requesting a format this server does not implement now answers
  `501 Not Implemented` instead of `400`.** `openehr_canonical_xml` and the
  `7z` compression format are valid values in the openEHR service model that
  this server does not build; reporting them as malformed requests was wrong,
  and the response now says the functionality is unsupported.

- **The template list collapses to the latest version of each template when
  the `version` parameter is absent** (#614). `GET
  /definition/template/adl1.4` (and the ADL2 twin) used to return every
  stored version regardless; the released openEHR REST API says an absent
  `version` returns "only the latest version". Pass `version=*` to list every
  stored version — the admin console's template inventory does exactly that,
  so its view is unchanged.
- **Conformance runner: a requirement the openEHR specification dates to a
  release is now judged only against the servers that claim that release**
  (#627, #628). Two rules the ITS-REST overview introduces at Release 1.1.0 —
  that an `ETag` carrying a resource identifier must be weak (`W/"…"`), and
  that `Location` is no longer returned on reads and deletes — used to be
  enforced or waived by accident, depending on whether a case happened to
  carry a version floor for some unrelated reason. Each rule now carries the
  floor itself, so a target declaring an earlier ITS-REST release is still
  driven for the operation and still judged on everything else, while a target
  declaring 1.1.0 or later faces the rule everywhere it applies; a test
  derives the affected set from the committed catalogue so no future binding
  can escape it. Separately, the query resultSet's `ETag` is no longer
  required to be PRESENT: the specification names the header without any
  requirement keyword and its only strength anywhere is a SHOULD, so a server
  that omits it is not failed — while a server that emits it must still emit
  it in the weak form. Conformance verdicts for this product are unchanged (it
  declares ITS-REST 1.1.0 and returns the header).

- **Conformance runner: one operation is now sent one way** (#629). The runner
  built request headers in two places — once for a case's own steps and once
  for the preconditions a case needs — and the two disagreed, so the same
  operation went on the wire differently depending on which path reached it
  (a template upload was refused as a case and accepted as a precondition).
  There is now a single header-construction path; a binding declares the
  `Accept` it intends; and a refusal of that `Accept` is recorded as a named
  outcome instead of vanishing into an unmapped status. The published
  conformance report also gains a per-capability table showing how many cases
  passed, failed, and came back inconclusive, so a divergence can no longer
  hide behind an inconclusive exchange. A binding may now also declare the
  release its wire first appeared in, and cases driving it are recorded
  not-applicable — with that citation — for targets that declare an earlier
  one.

- **Conformance runner: a create asked for a minimal response may answer
  either `201 Created` with an empty body or `204 No Content`** (#630). The
  specification says an empty-bodied response SHOULD use `204`, and its
  machine-readable artifacts declare `201` for creates; both are therefore
  conformant, and the suite now judges both instead of leaving one of them an
  inconclusive result. Updates are unchanged (`204` only, where both sources
  agree).

- **Web Template: a template that narrows a party to `PARTY_RELATED` now
  describes its party fields** (#600). When an operational template pins a
  party slot — a subject, a composer, a participation performer — to
  `PARTY_RELATED`, the generated Web Template used to describe that node as an
  empty container: none of the `|name`, `|id`, `|id_scheme` and
  `|id_namespace` fields the Simplified Formats specification gives every
  party appeared, so a form builder reading the Web Template could not offer
  them even though the server has always accepted and returned them. The four
  fields are now described, alongside the `relationship` sub-path the narrowing
  adds. The same held wherever a party node also constrained an attribute; the
  fields survive that too. Nothing changes for stored data or for the FLAT and
  STRUCTURED wire.

- **FLAT/STRUCTURED: the specification's other spelling of a related party's
  relationship is accepted on input** (#589). The Simplified Formats mapping
  table for a `PARTY_RELATED` writes the relationship sub-path
  `…/_relationship`, while every example block in the same section — and the
  participation-performer table — writes `…/relationship|code`. Only the
  example form was accepted, so a producer that followed the table row had
  its composition rejected with an unknown-path error. Both spellings are now
  read, and either one makes the party a `PARTY_RELATED`. What the server
  *emits* is unchanged: always the example spelling, so stored data and
  round-trips look exactly as before.

- **Three FLAT/STRUCTURED mapping gaps: an entry's subject, null-flavoured
  elements, and the event-context paths** (#532, #533, #534). An entry's
  `subject` now travels over the wire in both directions: a composition whose
  OBSERVATION, EVALUATION, INSTRUCTION, ACTION or ADMIN_ENTRY names someone
  other than the record subject emits `…/subject|name`, `…|id`,
  `…|id_scheme`, `…|id_namespace` plus the `/_identifier:i` and
  `/relationship` sub-paths, and the same keys are accepted on input —
  previously the subject was dropped on the way out and rejected as an
  unknown suffix on the way in, so the information was lost in both
  directions. A "self" subject carrying an external reference is marked
  `|_type: PARTY_SELF` so it comes back as itself rather than as an
  identified party. Second, an element that records *why* a value is missing
  (a null flavour, which the reference model makes mutually exclusive with
  the value) now keeps `/_null_flavour` and `/_null_reason` through a full
  round trip; the flattener reached elements only through their value, so a
  null-flavoured element vanished entirely. Third, the event-context fields
  the specification also spells as paths — `…/context/start_time` and
  `…/context/setting` — are honoured on input instead of being silently
  discarded in favour of the `ctx/` defaults, as are an entry's
  `…/language` and `…/encoding`; a bare `…/context/setting|code` resolves
  against the openEHR *setting* value set exactly as `ctx/setting` does.
  Paths the specification does not define are still rejected with a clear
  error rather than ignored.

- **The SMART discovery endpoint is fully described in the published API
  reference** (#535): the `application/json` requirement, the required
  `org.openehr.rest` service with its absolute `baseUrl`, the
  capability-honesty rules, the public pre-auth posture, and a worked
  document example — previously a one-line declaration.

- **The FLAT/STRUCTURED mapping of `INSTRUCTION_DETAILS` and
  `INTERVAL_EVENT.sample_count`** (#521). An ACTION's instruction details now
  travel over the wire exactly as the Simplified Formats specification maps
  them — three suffixes on the `_instruction_details` field itself:
  `|path`, `|composition_uid` and `|activity_id`. Previously the server
  emitted a nested `_instruction_details/instruction_id` field with generic
  object-reference suffixes, so `|composition_uid` was never produced, the
  instruction path sat one level too deep, and two suffixes the
  specification does not define were emitted; clients that sent the
  specified form had the details silently dropped. Both directions are now
  symmetric, so a composition round-trips through FLAT and STRUCTURED
  without losing the reference. Separately, an interval event's
  `|sample_count` (the count of samples the interval summarises) is now
  both emitted and accepted; it was previously ignored in both directions.

- **The template documentation no longer advertises a build flag that does
  not exist** (#521). The templates-and-validation page described an
  `ehrbase-quirks` build that renumbers duplicate node names and accepts two
  vendor-only `DV_QUANTITY` suffixes; that feature was removed long ago.
  There is one behaviour — the one the specification prescribes.

- **The published API reference is now honest about the non-openEHR surface,
  and follows a non-default base path** (#526). Every operation this server
  serves outside the standardised openEHR ITS-REST resource set — the
  management, terminology, event-subscription, multi-tenancy and FHIR R4
  groups, plus the IHE ITI-81 audit retrieval — now states in its own
  published description that no openEHR specification governs it (the flag
  previously lived only in source-module comments the document never
  carried), and each disabled-group `404` now says that an unauthenticated
  caller is answered `401` first, which is what the server actually does. The
  `/status`, `openapi.json`, Swagger-UI and System-`OPTIONS` declarations now
  follow a configured `server.base_path` instead of always printing the
  default deployment's paths. The twelve per-family
  `ehrbase-{family}.openapi.json` documents are documented, the ITI-81 and
  admin-extension operations now appear in a family document (previously in
  none), and the document itself declares a `servers` block, a link to the
  implemented ITS-REST release, descriptions for every tag it uses, and the
  implemented ITS-REST contract version as `x-openehr-its-rest` (distinct
  from `info.version`, which is the product version).

- **SMART App Launch conformance** (#519). The discovery document's
  `services.*.baseUrl` values are now absolute URLs built from the new
  required `smart.public_base_url` origin (the specification requires
  absolute URLs); the `openehr-permission-v1` capability is advertised only
  in fail-closed mode (`require_smart_scopes = true`) so advisory
  deployments no longer over-claim fine-grained enforcement; operators can
  advertise the HL7 base capabilities via `smart.endpoints.capabilities`;
  enabling SMART now boot-validates the origin plus the
  authorization/token endpoints; the published OpenAPI's discovery path
  follows a configured `platform_base_url`; and — the substantive gap —
  the **template and AQL scope families now enforce**: in fail-closed mode
  a token without a matching `template-…`/`aql-…` scope is denied `403` on
  the template and query routes (previously only the composition family
  was gated).

- **The published API reference now describes the admin endpoints in full,
  and the disabled-admin answer is documented correctly** (#513). The five
  admin operations — the released `DELETE /admin/ehr/{ehr_id}` and
  `DELETE /admin/ehr/all`, plus the template-delete, stored-query-version-
  delete and effective-config extensions — gained the branches they actually
  answer (`400` for a malformed EHR id, `401`/`403` from the admin role gate,
  `404`, the template `409`), the mandatory empty `Allow` header on every
  disabled-group `405`, and worked request/response examples. They now carry
  the released operation text verbatim, including the permanent-physical-
  delete cascade and its data-protection (GDPR) sentence, the
  development/testing note on the bulk route, and the fact that this server
  deletes synchronously (`204` only — the specification's optional
  asynchronous `202` is never returned). The bulk route documents both
  accepted query forms (`?ehr_id=a&ehr_id=b` and `?ehr_id=a,b`) and that an
  absent or empty list deletes every EHR; the three extension routes are
  flagged plainly as our own, governed by no openEHR operation. Reference
  documentation and configuration docs that claimed a disabled admin API
  answers `404` were corrected: it answers `405 Method Not Allowed` with an
  empty `Allow` header.

- **Demographic ITEM_TAG collections honour the released dual-form
  addressing** (#509). A version-addressed `uid_based_id` on the demographic
  tag routes now reads, replaces, and deletes that VERSION's own distinct
  tag collection (previously every form reached the container's set); the
  tags GET and DELETE now answer `404` for a nonexistent, wrong-kind, or
  cross-space target (previously an empty `200` list); both
  `openehr-item-tag` and `openehr-version-item-tag` request headers are
  accepted on party create AND update, each landing on its own target's
  collection with its own response-header echo; a tag's `target` is now the
  bare RM `UID_BASED_ID` (an `OBJECT_VERSION_ID` for version targets) and
  its `owner_id` follows the released examples' `local`/`SYSTEM` shape; and
  the PARTY_RELATIONSHIP extension's stale-delete `409` now echoes the
  latest `version_uid` in `ETag` like the party delete it mirrors.

- **The published API reference now describes the demographic item-tag
  endpoints in full** (#510). The sixteen `ITEM_TAG` operations — the
  person / agent / group / organisation / role `tags` read, replace and
  delete-by-key, plus the space-wide `GET /demographic/tags` — gained the
  status branches they actually answer (`400`, `404`, `406`, `415`, `422`),
  the `Prefer` / `Content-Type` / `Accept` request headers, the
  `Preference-Applied` echo on both replace branches, and worked ITEM_TAG
  examples. They now state plainly that a version-addressed `uid_based_id`
  and a container-addressed one name two DISTINCT tag collections, that an
  empty list on the replace clears every tag, that deleting by key alone
  removes every tag under that key whatever its `target_path`, and that a
  tag collection is never change-controlled — so no `ETag`, `Last-Modified`
  or `Location` is offered anywhere on the family. The space-wide list is
  documented for what it is: the one tag route with no scoping parameter at
  all, no paging, and `200` (an empty array when nothing matches) or `400`
  as its only outcomes.

- **Demographic header echoes** (#388). The stale-version party DELETE's
  `409 Conflict` now returns the latest `version_uid` in `ETag` (the released
  response requires it); and the demographic CONTRIBUTION read now carries
  the weak `ETag` (the contribution uid) and `Last-Modified` from the
  committal instant, matching its EHR sibling.

- **The published API reference now describes the demographic endpoints in
  full** (#505). The 26 person / agent / group / organisation / role,
  versioned-party and demographic-contribution operations in the served
  OpenAPI document gained the response headers they actually send (`ETag`,
  `Location`, `Last-Modified`, `Preference-Applied` and the two
  `openehr-*-item-tag` headers), the status branches they actually answer
  (`204`, `400`, `406`, `409`, `412`, `415`, `422`), the committal
  (`openehr-version` / `openehr-audit-details`), `Prefer`, `If-Match`,
  `Accept` and `Content-Type` request headers, worked PERSON /
  VERSIONED_PARTY / CONTRIBUTION examples, and a spec citation on every
  branch. Reads and deletes no longer suggest a `Location` header they never
  send, and the party routes now state plainly that Simplified (FLAT /
  STRUCTURED) media types are refused because a demographic party is not
  templated. The eight `PARTY_RELATIONSHIP` operations are labelled for what
  they are — an extension of this server, with no openEHR REST operation
  behind them — and the group carries a note that the openEHR Demographic
  API is itself a `DEVELOPMENT`-state specification.

- **Stored-query stores answer honest `Location`s and validate the version
  segment** (#498). The version-less `PUT /definition/query/{name}` now
  always names the version it actually wrote (`…/1.0.0`) in `Location` —
  previously, when a higher version already existed, the header pointed at
  that untouched neighbour. The versioned
  `PUT /definition/query/{name}/{version}` now requires an exact numeric
  `major.minor.patch` and rejects prefix, pre-release, or malformed version
  segments with `400 Bad Request` — previously any string was stored
  verbatim, and a single non-numeric version (e.g. `1.0.0-rc.1`) broke every
  later stored-query list and retrieval on the server. Both store forms also
  now refuse a payload declaring a media type other than their single
  `text/plain` body type with `415 Unsupported Media Type` (an absent
  `Content-Type` remains accepted).

- **The published API reference now describes the stored-query endpoints in
  full** (#499). The four stored-query operations in the served OpenAPI
  document gained the `Location` response header both stores actually send,
  the bodyless shape of their `200`, the status branches they actually
  answer (`400` everywhere, `406` on the reads, `409` on the versioned
  store, `404` on the version read), the qualified-name and `version`
  grammars (including the reserved `aql` name and the read-side
  prefix-resolution rule), request and response examples, and a spec
  citation on every branch. The bare "list every stored query" route is now
  labelled for what it is — an extension of this server, not a released
  openEHR operation.

- **Template rejection statuses are coherent across both upload routes**
  (#493). An ADL2 source with grammar-level syntax errors now answers
  `400 Bad Request` (the released "syntactically invalid … content" branch)
  instead of `422`; AOM2 validation-phase failures on a parsed source keep
  answering `422` with the rule codes in `validationErrors`. On the ADL 1.4
  side, an AOM2 artefact-validity violation on a successfully parsed OPT now
  answers `422` with the rule code in `validationErrors` (previously `400`) —
  syntax gates `400`, semantics gate `422`, on both routes.

- **Template-upload rejection statuses follow the released split** (#489).
  An ADL 1.4 OPT upload whose body is not well-formed XML now answers
  `400 Bad Request` (the released "syntactically invalid … content" branch)
  instead of `422`; well-formed XML that is not a valid OPT stays `422`.
  The ADL2 template upload now refuses a payload declaring a media type
  other than its single `text/plain` body type with `415 Unsupported Media
  Type` (an absent `Content-Type` remains accepted), mirroring the ADL 1.4
  guard.

- **The published API reference now describes the template endpoints in
  full** (#490). The nine ADL 1.4 / ADL 2 template operations in the served
  OpenAPI document gained the response headers they actually send (`ETag`,
  `Location`, `Preference-Applied`), the status branches they actually
  answer (`400`, `406`, `415`, `422`), request/response examples, and a
  spec citation on every branch; "Get template at version" is now marked
  deprecated, as the openEHR REST specification marks it.

- **Stored-query POST bodies accept `{}`; the query POSTs accept the URL
  parameter forms** (#481). The three body members of the stored-execute
  body are optional (the docs text gives `offset` a default and makes
  `fetch` implementation-default — the stalled required-list loses), so a
  parameterless stored query executes with an empty body; and all three
  POSTs now accept `offset`/`fetch`/named `$parameters` from the URL (the
  docs-text SHOULD-list draws no GET/POST distinction), with a body-vs-URL
  disagreement rejected 400.

- **Tag GET/DELETE verify the addressed target; empty `target_path`
  normalizes to absent** (#474). `GET`/`DELETE` on the per-target tag
  routes now answer 404 for a nonexistent, foreign-EHR, or wrong-kind
  `uid_based_id` (the released trigger: "when the `uid_based_id` does not
  exist"; previously the GET answered `200 []` and the DELETE was not
  kind-checked), and a `target_path: ""` on the tag PUT is normalized to
  the absent path so `""` and absent are one `(key, target_path)`
  identity. The EHR-wide tag listing likewise answers 404 for an unknown
  `ehr_id` (previously `200 []`).

- **Contribution change-type mismatch statuses follow the released
  assignment** (#467). A non-creation `change_type` committed as the FIRST
  version of a versioned object (the released `400_CONTRIBUTION` trigger:
  "the modification type does not match the operation - i.e. first version
  of a MODIFICATION") now answers 400; a `249|creation|` member carrying a
  `preceding_version_uid` — the unassigned mirror case — moves to 422.
  Previously the two were inverted.

- **The CONTRIBUTION GET serves `ETag` + `Last-Modified`** (#463).
  `GET …/contribution/{contribution_uid}` now carries the contribution-uid
  weak `ETag` (the same identity the 201 already carries) and a
  `Last-Modified` derived from the contribution audit's commit instant
  (ITS-REST overview *Requests and responses* §"ETag and Last-Modified").

- **Directory by-version reads verify the full addressed identity; the
  directory DELETE 204 carries the deleted version's identity** (#456).
  `GET …/directory/{version_uid}` now answers 404 when the addressed
  `creating_system_id` does not match the stored identity (ITS-REST
  overview *Resources* §Identifier types), and
  `DELETE …/directory` answers 204 with the NEW `523|deleted|` version's
  weak `ETag` + `Last-Modified` (previously header-less), matching the
  composition DELETE.

- **COMPOSITION update body-uid mismatch is 422, not 400** (#451). A PUT
  whose body `COMPOSITION.uid` names a different versioned object than the
  request path is now rejected 422 Unprocessable Entity — the body is
  well-formed and the contradiction is semantic (ITS-REST *Requests and
  responses* §HTTP status codes, the 422 row; no released sentence assigns
  the rejection — register-documented).

- **Versioned-composition version-by-id reads are container-scoped** (#449).
  `GET …/versioned_composition/{versioned_object_uid}/version/{version_uid}`
  previously ignored the container segment and served any version the
  `version_uid` named; a `version_uid` whose `object_id` does not match the
  path's container now answers 404 (ITS-REST overview *Resources*
  §Identifier types; RM `Owner_id_valid`).


- **EHR creation mints RM-valid EHR_STATUS and EHR_ACCESS objects; the RM
  archetype-root invariants are enforced on client bodies** (#423). The
  bootstrap defaults carried an archetype-HRID `archetype_node_id` with no
  `archetype_details` — violating RM `Is_archetype_root` (unconditional on
  both classes) with `Archetyped_valid` ("is_archetype_root xor
  archetype_details = Void"). Both defaults now carry the `ARCHETYPED` block
  (archetype_id = the node id, rm_version 1.2.0), and a client-supplied
  EHR_STATUS/EHR_ACCESS violating `Archetyped_valid` (a root without
  `archetype_details`, or a mismatching `archetype_id`) or `Links_valid`
  (an explicit empty `links` list) is rejected with `422`. Clients that
  previously committed root objects without `archetype_details` must now
  supply it.

- **Imported and archive-loaded EHRs are now complete, first-class EHRs**
  (#425). An EHR-Extract clone (`import_ehr`) created no EHR_ACCESS, so a
  source extract that carried none produced an EHR permanently violating the
  RM invariant `Ehr_access_valid` (`EHR.ehr_access` is 1..1) whose served
  `GET /ehr/{ehr_id}` body simply omitted the mandatory reference; the clone
  now commits the same default EHR_ACCESS the create path uses (RM ehr
  master04 §EHR Creation — a root EHR object, an EHR Status object and an EHR
  Access object), in the import's own transaction. Neither the import nor the
  admin archive load promoted the EHR's subject, so imported/loaded EHRs were
  invisible to `GET /ehr?subject_id=…&subject_namespace=…` and exempt from the
  one-EHR-per-subject `409`; both paths now derive the subject from the landed
  EHR_STATUS. Consequences: importing or loading an EHR whose subject this
  repository already holds is now refused — `409` for an import (naming the
  subject and the EHR that holds it), and, for the archive load, a per-record
  `DUMP_LOAD_FAIL_REPORT` entry that skips just that EHR exactly like a
  duplicate EHR id, leaving the rest of the archive to load.


- **Both EHR creates now accept and merge the committal request headers**
  (#422). ITS-REST `docs/overview/Requests_and_responses.md` §"openehr-version
  and openehr-audit-details" makes it a MUST that a service accept
  `openehr-version` / `openehr-audit-details` on the direct `PUT`/`POST`/
  `DELETE` commits of change-controlled resources and merge "whatever is
  provided … with the default VERSION and VERSION.audit_details attributes on
  commit runtime". Creating an EHR commits its EHR_STATUS and EHR_ACCESS in a
  contribution (RM ehr master04 §EHR Creation), but `POST /ehr` and
  `PUT /ehr/{ehr_id}` ignored both headers — while the served OpenAPI already
  claimed they were merged. They now are: the supplied `description`,
  `committer`, and `system_id` land on the creating contribution and on both
  committed versions' `commit_audit`, and `openehr-version:
  lifecycle_state.code_string` sets the new EHR_STATUS version's lifecycle
  state. `change_type` is constrained to `249|creation|` (a create commits a
  first version): restating `249` is accepted, another group code is `400`,
  and a token outside the `audit_change_type` group is `422`. The OpenAPI
  document now also lists both headers as documented parameters on the two
  create operations.

- **Served OpenAPI: the four EHR-resource operations now document the whole
  wire** (#427). `GET /ehr`, `POST /ehr`, `GET /ehr/{ehr_id}` and
  `PUT /ehr/{ehr_id}` under-described what the server actually does. The two
  creates now declare their `415` (an unprocessable request `Content-Type`,
  including a Simplified Format, which is defined only for templated
  COMPOSITION content) and `406` (an `Accept` that canonical JSON/XML cannot
  satisfy) branches, both a MUST in the REST spec's format sections; the two
  reads declare `406` as well, and `GET /ehr/{ehr_id}` declares the `400` it
  returns for a malformed (non-UUID) `ehr_id`. Every success response now
  carries a header block: `ETag`/`Location`/`Last-Modified`/
  `Preference-Applied` on the `201`s, `ETag` on the reads — where the absence
  of `Location` and `Last-Modified` on a read is now stated explicitly rather
  than left unsaid. The `Prefer`-conditional `201` body is documented as a
  named example pair (`representation` — the full RM `EHR`; `identifier` —
  the single-`uid` object), the `Prefer` header enumerates its three tokens
  and its default, and the request body, the read bodies, and the subject
  query parameters carry real served-shape examples. Corrected false claim:
  `PUT /ehr/{ehr_id}` described `ehr_id` as any `HIER_OBJECT_ID` with "a UUID
  strongly recommended", while the server accepts UUIDs only — which is what
  the abstract service model types the argument as, and every UUID is a valid
  `HIER_OBJECT_ID` root.

- **Served OpenAPI: the System API's `OPTIONS` operation is now documented**
  (#418). The Options-and-Conformance endpoint (`OPTIONS` on the API base
  path) was served but absent from the generated OpenAPI document and
  Swagger UI, because its route mounts outside the documenting router (above
  the CORS layer, deliberately). A documented twin now carries the full
  operation description — the `Options` manifest schema with field
  documentation and example, the `Allow`/`Content-Type` response headers,
  and the `406` negotiation branch.

- **`version_at_time` now accepts a datetime without a timezone, interpreting
  it in the server's local timezone** (#401). ITS-REST
  `docs/overview/Resources.md` §"Datetime format" requires the extended
  ISO 8601 form for datetime query parameters and states
  that "Timezone SHOULD be only supplied when needed, otherwise the local
  timezone is assumed" — so `?version_at_time=2016-06-23T13:42:16` is a valid
  request. It was answered `400 Bad Request`, because both at-time parsers
  (the EHR group's and the DEMOGRAPHIC group's duplicate) required an offset.
  A single shared decoder now backs every at-time read — EHR_STATUS,
  COMPOSITION, DIRECTORY, the `versioned_*` version reads, the demographic
  party/relationship reads, and the contribution `time_range` — and resolves
  an offset-less value against the server's system timezone (`TZ`, else the
  platform's local-time configuration); a value falling inside a
  daylight-saving fold or gap resolves to the earlier and the later instant
  respectively. Genuinely malformed input is unchanged: the basic ISO 8601
  format (`20160623T134216Z`), a date without a time (`2016-06-23` — the
  parameter is specified as "a given time", and reading it as midnight would
  silently serve a version the caller never asked for), a timezone-less value
  carrying an `[Area/Location]` annotation, and anything unparseable are all
  still `400`.
- **Every `405 Method Not Allowed` now carries an `Allow` header, and the
  `408`/`413` transport refusals use the openEHR error body** (#400). ITS-REST
  `docs/overview/Requests_and_responses.md` §"HTTP Methods" answers a method a
  resource does not serve with `405`, over RFC 9110 — the authority that
  section names — whose §15.5.6 requires that "the origin server MUST generate
  an Allow header field in a 405 response containing a list of the target
  resource's currently supported methods". The router's `405` already carried
  it; the `405` returned when the **admin API is disabled** did not, because it
  comes from a matched handler and so never reached the router's allow-header
  machinery. It now sends the empty field value RFC 9110 §10.2.1 defines for
  exactly this case ("An empty Allow field value indicates that the resource
  allows no methods, which might occur in a 405 response if the resource has
  been temporarily disabled by configuration"). Separately, a request that
  times out (`408`) or declares a body over the 16 MiB limit (`413`) was
  answered by the middleware with an empty or `text/plain` body; both now
  render the same `{ "error", "message" }` JSON every other error path emits.
  Finally, the deviation behind all of this is now recorded rather than
  silent: the same spec section *also* SHOULDs `501 Not Implemented` for an
  unrecognized method, and we answer `405` there too — the two SHOULDs overlap
  for any method outside the tabulated subset, `405` is a predefined
  non-conflicting code in the spec's own status table, and a blanket `501`
  fallback would misreport unknown **paths** that are owed `404`. `501` is
  still returned for a recognized but unimplemented operation. Adjudicated in
  the conformance ambiguity register as `AMB-60`, with the wire-surface
  boundary registered alongside it.
- **The `openehr-ehr-id` request header now scopes `GET` query execution too,
  and a scope named twice must agree** (#399). ITS-REST
  `docs/query/Request.md` §"About the `ehr_id` parameter" lets clients supply
  the single-EHR scope "as a query parameter `ehr_id` or alternatively as a
  request header named `openehr-ehr-id`", and §"Common Headers and Query
  Parameters" applies that to "all query execution requests". Only the `POST`
  forms honoured the header: `GET /query/aql`, `GET /query/{name}` and
  `GET /query/{name}/{version}` read the scope from the query string alone, so
  a header-scoped `GET` silently ran as a **population query** across every
  EHR. All six execution operations now resolve the scope through one seam.
  The released text never says what a request carrying *both* forms means, so
  the handling is adjudicated and registered (ambiguity register `AMB-59`):
  both forms naming the **same** EHR execute normally, and both forms naming
  **different** EHRs are rejected `400 Bad Request` rather than silently
  picking one — a request that names two EHRs cannot be answered correctly
  (`docs/overview/Requests_and_responses.md` §"HTTP status codes", row `400`).
  An empty header value carries no identifier and neither scopes nor
  conflicts. The deprecated `openEHR-EHR-id` spelling keeps working (HTTP
  field names are case-insensitive). The header is now also declared on every
  query operation in the served OpenAPI.
- **`Prefer` / representation polish: `return=identifier` is structurally
  never `204`, item-tag echoes are per-target, and `Preference-Applied` is
  emitted from one seam** (#398). Three divergences from ITS-REST overview
  `Requests_and_responses.md`:
  - `Prefer: return=identifier` could fall through to the empty (possibly
    `204 No Content`) minimal response while still claiming
    `Preference-Applied: return=identifier`. The identifier branch now
    carries the identifier it renders, so it is unreachable without one —
    §"Prefer only identifier": "the status will be `201 Created` or `200 OK`,
    never `204 No Content`". A write that genuinely produces no identifier
    applies, and declares, the default `return=minimal` instead of claiming
    an unapplied preference.
  - The `openehr-item-tag` and `openehr-version-item-tag` **response** echoes
    on a change-controlled write merged both targets' tags into one list and
    repeated it under both header names. Each header now confirms only its
    own target's stored list — §"openehr-item-tag and
    openehr-version-item-tag": "`openehr-item-tag` applies to
    *VERSIONED_OBJECT* targets" while "`openehr-version-item-tag` applies to a
    specific target *VERSION*", each confirming "the actual list of
    `ITEM_TAGs` stored". A header the request did not send is not echoed.
    (The demographic surface still emits both headers from one set, because
    its tags are stored against the `VERSIONED_OBJECT` only, so the two
    targets coincide there.)
  - `Preference-Applied` was emitted only by the canonical RM / JSON write
    helpers. It is now declared by every write path through the same seam —
    the demographic party, relationship and CONTRIBUTION writes, both ADL 1.4
    and ADL 2 template uploads, the `ITEM_TAG` collection writes, and the
    Simplified-Formats (FLAT/STRUCTURED) COMPOSITION commit — always naming
    the preference the response actually applied, including the applied
    default `return=minimal` when no `Prefer` was sent. Demographic party
    writes additionally honour `Prefer: return=identifier` (`{uid}` body),
    which they previously ignored.
- **ADL 1.4 template negotiation: the response type mirrors `Accept`, and a
  non-XML OPT upload is `415`** (#397). Two divergences from ITS-REST overview
  `Resources.md`:
  - `GET /definition/template/adl1.4/{template_id}` with
    `Accept: application/json` was answered `Content-Type:
    application/openehr.wt+json` — a type the client never accepted. It now
    returns the same Web Template document under `Content-Type:
    application/json` (§JSON Format: "Proper header `Content-Type:
    application/json` MUST be present in the response of the service unless
    the response has no content body"). `Accept:
    application/openehr.wt+json` keeps the Web Template media type and
    `Accept: application/xml` the canonical OPT, both unchanged. (The
    released source is internally inconsistent here — the operation
    description names only XML + `wt+json` while its `Accept`/`Content-Type`
    enumerations include `application/json` with no schema — so serving the
    Web Template body is the recorded fixed handling, not a `406`.)
  - `POST /definition/template/adl1.4` accepted any `Content-Type` and failed
    a JSON payload with `400` from the OPT parser. A request declaring a
    non-XML payload type is now refused `415 Unsupported Media Type` before
    parsing (§XML Format: "If the service cannot process the request payload
    as XML format, it MUST respond with HTTP status code `415 Unsupported
    Media Type`"). `application/xml` and `text/xml` upload as before, and an
    absent `Content-Type` still reads as the operation's single body type
    (the header is a client MAY).
- **`Last-Modified` and `ETag` completion on the EHR and DEFINITION surfaces**
  (#396). ITS-REST overview `Requests_and_responses.md` §"`ETag` and
  Last-Modified" requires both headers on "VERSION, VERSIONED_OBJECT, or other
  resources that have versioning or unique state identifiers", with
  `Last-Modified` "derived from `VERSION.commit_audit.time_committed.value`".
  Only the `ETag` half shipped previously:
  - `Last-Modified` (IMF-fixdate) is now emitted on every VERSION read
    (`…/versioned_composition/{uid}/version[/{version_uid}]`,
    `…/versioned_ehr_status/version[/{version_uid}]`), on all COMPOSITION and
    `EHR_STATUS` reads and writes (including the delete `204` and the
    FLAT/STRUCTURED representations, whose version identity is
    serialization-independent), and on the EHR create `201`. The value is the
    served version's commit instant — read off the VERSION envelope where the
    body carries one, and off the version row / commit result for the bare
    COMPOSITION and `EHR_STATUS` representations, which have no
    `commit_audit` of their own.
  - `GET /ehr/{ehr_id}` and `GET /ehr?subject_id=…` now carry the weak
    `ETag` built from `EHR.ehr_id.value` — the source the spec section itself
    names. (No `Last-Modified`: the RM `EHR` root is not a VERSION, and
    `time_created` is not a last-modification instant.)
  - The ADL2 template responses (`POST /definition/template/adl2`,
    `GET …/adl2/{template_id}`, `GET …/adl2/{template_id}/{version}`) now
    carry the weak `ETag` their ADL 1.4 siblings already emitted. The value is
    the **resolved** `ARCHETYPE_HRID`, so addressing a template by a partial
    id or major-version prefix still yields an `ETag` that changes when the
    served artefact does.
  `CONTRIBUTION` creation still omits `Last-Modified` (the commit instant is
  not carried out of the version-set commit yet) and is marked `TODO` in the
  service layer.
- **Committal request headers: client `change_type` honoured, DELETE accepts
  the headers, deprecated `openEHR-AUDIT_DETAILS` spelling restored** (#395).
  Three divergences from ITS-REST overview `Requests_and_responses.md`
  §"openehr-version and openehr-audit-details" + §"Deprecated headers":
  - A client-supplied `AUDIT_DETAILS.change_type` (e.g.
    `change_type.code_string="250"` for an amendment) is now merged into the
    commit instead of being silently replaced by the operation default — the
    spec lists `change_type` first among the client-suppliable attributes and
    requires "whatever is provided it MUST be merged". The value is validated
    against the openEHR `audit_change_type` group (out-of-group → `422`,
    `AUDIT_DETAILS.Change_type_valid`) and against the operation (a
    contradicting code such as `249|creation|` on an update is rejected; the
    exact status is spec-unassigned — see ambiguity AMB-54 — and returns
    `400`). Applies to the direct COMPOSITION/EHR_STATUS/DIRECTORY commits
    and the demographic party/relationship commits alike.
  - `DELETE /composition/{id}` and `DELETE /directory` now accept
    `openehr-version`/`openehr-audit-details` and merge the supplied
    description/committer/system_id into the `523|deleted|` commit audit —
    the spec requires the headers accepted on PUT, POST **and** DELETE.
  - The bare deprecated header name `openEHR-AUDIT_DETAILS` (the exact
    spelling in the spec's deprecation table, which is a different HTTP
    header name than `openehr-audit-details`) is accepted again alongside
    the 1.0.3 dotted forms and the current name; the current name still wins
    on conflict.
  The `audit_change_type` constant set now mirrors the complete TERM group
  (all nine codes), locked by a test against the terminology bundle. Five new
  CNF catalogue cases pin the merge family end-to-end.
- **Demographic API: response-header discipline and `If-Match` handling** (#394).
  Three MUST/SHOULD-level divergences from the ITS-REST overview
  (`Requests_and_responses.md`) are corrected on the `/demographic` surface:
  - `Location` is no longer emitted on reads, deletes, or `409`/`412` error
    responses. The header now rides create/update writes only, per §Location
    ("MUST NOT be used to indicate an alternate representation of an existing
    resource"; "MUST ONLY be used for resource creation … or redirect
    responses") and §"Deprecated headers", which deprecates it on `GET` and
    `DELETE`. Those responses keep the weak `ETag` (and `Last-Modified` where
    known), so a client reading the version identity is unaffected; a client
    that was following the `Location` of a `GET`/`DELETE` must use the request
    URL it already has.
  - `If-Match` now accepts the **weak** `W/"…"` form the server itself emits as
    the `ETag`, alongside the bare-quoted and unquoted forms — previously
    echoing the server's own `ETag` back was rejected as a malformed
    precondition (`400`). The full `OBJECT_VERSION_ID` is compared
    case-insensitively (BASE composite-identifier semantics), so a case-variant
    `creating_system_id` no longer raises a spurious `412`. A syntactically
    invalid `If-Match` remains a `400` and is never silently ignored.
  - The `versioned_party` / `versioned_party_relationship` reads (the container,
    its revision history, and the version reads) now carry the weak `ETag` and,
    where the served body exposes the commit instant, `Last-Modified` — both
    SHOULD-present on `VERSION`/`VERSIONED_OBJECT` responses.

### Added

- **Several terminology servers can now serve one instance at the same time.**
  Every entry under `[terminology.external.providers]` is started up, not just
  `default`, and a new `[terminology.external.routes]` map sends each
  terminology to the server that serves it — the key is a terminology id
  (`SNOMED-CT`) or a system URI (`http://snomed.info/sct`), matched
  case-insensitively, and the value names a provider. A terminology with no
  route goes to the provider named `default` (or to the sole configured one).
  Routing applies to the whole terminology surface: the `/terminology/*`
  extension API, AQL `TERMINOLOGY(…)`, and composition validation. So SNOMED CT
  can live on one server while LOINC or ICD live on others — the deployment
  reality openEHR's terminology chapter describes. Configuring a route to a
  provider that does not exist is now a startup error instead of a silent
  fallback.
- **Terminology servers can require OAuth2.** A provider's `oauth2_client` key
  now does something: it names an entry under
  `[terminology.external.oauth2_clients]` (token endpoint, client id, client
  secret or `client_secret_file`, optional scopes, `refresh_leeway_secs`, and
  `client_secret_basic` / `client_secret_post`), and the CDR obtains a
  client-credentials access token and sends it as a bearer credential on every
  request to that server. The token is cached and renewed shortly before it
  expires, so a validation burst costs one token request per token lifetime. A
  refused grant fails the call with a clear error — a request is never sent
  unauthenticated as a fallback.
- **Terminology servers can require a client certificate (mutual TLS).** A
  provider takes three new keys — `client_cert_path`, `client_key_path` and
  `ca_bundle_path` — so the CDR presents a client certificate to that
  terminology server and verifies the server against that server's own trust
  anchors. The identity is per provider because a client certificate is issued
  by the peer's PKI: a deployment enrolled with a national SNOMED CT service, a
  commercial value-set server and an in-house server holds three different
  certificates, and repeating the same paths covers the case where one identity
  really does serve them all. `ca_bundle_path` *replaces* the default trust
  anchors for that provider, so a privately-issued terminology server is pinned
  to its own CA instead of also accepting the whole public web PKI. There is no
  option to skip verification — server-certificate and hostname verification
  stay on for every provider; the bundle changes which anchors are trusted,
  never whether the server is checked. Anything broken (one half of an
  identity, an unreadable PEM, a key file holding no key, a CA bundle holding
  no certificate) fails at startup, never at the first validated code.
- **Composition commits can now check archetype value-set bindings against a
  live terminology server.** When a template binds an `ac` code to an external
  terminology query, and `[terminology.external]` is enabled, committing a
  COMPOSITION resolves that query and requires the coded value to be a member
  of the value set it returns. A non-member is a `422` naming the path, the
  code and the bound query. If the value set cannot be resolved at all (server
  down, error response, no server configured for that terminology), the
  existing `fail_on_error` switch decides: `false` (the default) accepts the
  commit and logs a warning, `true` rejects it. With `[terminology.external]`
  disabled — the shipped default — nothing is resolved, no request is made, and
  commit behaviour is exactly as before.
- **An opt-in `terminology` Compose profile runs a real FHIR R4 terminology
  server beside the CDR.** `docker compose --profile terminology -f
  docker-compose.yml -f docker/sut-terminology.yml up` starts a digest-pinned
  HAPI FHIR JPA server (host port 8090, `EHRBASE_TERMINOLOGY_PORT`) plus a
  one-shot container that seeds it — over the server's own FHIR API — with two
  synthetic test code systems and their value sets, one SNOMED-CT-shaped and
  one LOINC-shaped, and verifies `$validate-code` and `$expand` before exiting.
  The overlay switches on the `[terminology.external]` providers now shipped
  (disabled) in `docker/ehrbase.dev.toml`, so the plain quickstart is
  unchanged. No licensed terminology content is distributed: the fixtures live
  under the reserved `example.test` domain, and the SNOMED-CT-shaped and
  LOINC-shaped codes are invented for the test corpus.
- **The conformance record now covers the terminology-routed surface.**
  `scripts/conformance.sh` composes the terminology profile for every
  `ehrbase-rs` run, and the catalogue gained eight cases: AQL `TERMINOLOGY()`
  resolved through the routed server (the Boolean `validate` form answering
  true and false, and the `expand` filter over committed data), the
  two-simultaneous-servers routing proof, and commit-time archetype
  constraint-binding validation (a member code accepted, a non-member refused,
  and the unresolvable value set under each declared posture). A party's
  `ixit.json` gains a `terminology` block declaring its terminology servers,
  the namespaces each answers for, and its fail-open/fail-closed posture; a
  party that declares none has those cases recorded not-applicable with that
  citation instead of failed.
- **`POST /admin/dump` now serves the `openehr_canonical_xml` logical format,
  which used to answer `501`.** Both openEHR export formats are available:
  the default `openehr_canonical_json` keeps each version's content inline in
  the archive's segment files, while `openehr_canonical_xml` writes each
  version to its own `versions/<version_uid>.xml` entry — a complete
  `ORIGINAL_VERSION` document under the openEHR-published `<version>` root,
  readable by any tool that speaks canonical openEHR XML. The archive's own
  bookkeeping (`manifest.json`, the segment files) stays JSON in both formats,
  because openEHR publishes no XML document form for it. `POST /admin/load`
  is unchanged for callers: it still takes only a location and now reads the
  logical format out of the archive's manifest, exactly as it already detected
  the container. Both formats round-trip in all three containers (loose,
  `archive.zip`, `archive.7z`) and reproduce every record byte-for-byte. A
  single unreadable `versions/*.xml` entry is reported against the one EHR it
  belongs to and skipped, while the rest of the archive loads.

- **The definition and messaging extension routes now document their refusal
  branches.** Every ADL 1.4 / ADL 2 archetype route, every `/message` route
  and every `PARTY_RELATIONSHIP` route declares `401` (no valid principal)
  and — on the writes — `403` (a principal holding the configured read-only
  role) in the served OpenAPI, so a client can see the whole answer set of an
  endpoint before it calls it. The TDD batch additionally documents its `413`
  boundary: the batch has no cardinality limit of its own, only the
  server-wide request-body limit.

- **Conformance: the admin extension batteries now test what must be
  REFUSED, not only what must work.** A server that accepts what the contract
  forbids is as non-conformant as one that refuses what it must accept, and
  the activity-report, EHR/demographic archive and dump/load batteries proved
  only their happy paths. They now also drive the unauthenticated (`401`) and
  non-administrative (`403`) probes on every route of each family, every
  argument-type refusal (a service outside the enumeration, the three ways a
  time interval can be malformed, a malformed id in an archive list, an
  unknown export format, a non-positive segment size), the empty-selection and
  repeat-archive boundaries, and the zip / 7z / loose container-detection round
  trips on load — with the duplicate-report body now asserted rather than
  assumed. The branches that cannot be driven from a client (the
  admin-disabled `405`, which needs a differently configured deployment, and
  the corrupt-archive `5xx`, which needs bytes placed on the server's own file
  system) are recorded as explicit boundaries and covered by in-process tests
  instead of being silently absent.
- **The documentation site renders mathematics.** Formulas on the
  performance pages (the open-loop arrival schedule, the population-anchored
  write-rate derivation) are now typeset with KaTeX, pre-rendered to static
  HTML at build time — pages stay self-contained with no client-side script
  and no CDN request; the KaTeX stylesheet and fonts are served by the site
  itself.
- **Conformance: ADL 1.4 archetype provisioning is now tested rather than
  excused.** openEHR's released REST API defines no ADL 1.4 archetype
  resource, so the capability used to be reported as "excused — unrealized on
  this technology profile" even though this server serves archetype routes of
  its own design. Six conformance cases now execute against
  `/definition/archetype/adl1.4` — upload with source read-back, an
  unparseable-source refusal, listing, and the get/delete branches including
  their not-found halves. Because openEHR gives the capability no wire, the
  published certificate marks the row `extension` and it no longer gates the
  CORE profile — a conscious, register-recorded departure from the
  conformance profiles book, which requires a capability the release gives no
  wire for.
- **The admin dump/load archive now supports 7z compression.** `POST
  /admin/dump` accepts `compression_format: "7z"` alongside `zip` and the
  uncompressed form, packing the same archive entries into one `archive.7z`;
  `POST /admin/load` detects and reads all three container forms without
  being told which one it was given. (The `openehr_canonical_xml` logical
  format remains a declared `501` boundary — the archive's XML form is a
  design of its own, tracked separately.)
- **Repository dump/load and the whole messaging surface are now HTTP
  routes** — the last service capabilities that had no wire. Under the
  existing `EHRBASE__ADMIN__ENABLED` gate and `ADMIN` role,
  `POST /admin/dump` writes an archive of every EHR to a location on the
  server's file system and `POST /admin/load` populates the repository from
  one; both answer `200` with the per-entity report the openEHR service model
  defines (empty when everything succeeded), and a load into a non-empty
  repository reports each already-present EHR rather than failing. Under the
  ordinary clinical authentication — these are not admin routes — the new
  `/message` group serves EHR Extract export (`GET /message/export/{ehr_id}`,
  `POST /message/export` by specification) and import
  (`POST /message/import` for a whole-EHR clone,
  `POST /message/import/{ehr_id}` to add content to an existing one), plus
  Template Data Document import (`POST /message/tdd/{ehr_id}` and its
  all-or-nothing `/batch` sibling). Like the admin extensions, all of these
  are ehrbase-rs extensions: the openEHR service model defines the operations,
  the released REST API surfaces no endpoint for them, and no openEHR
  conformance claim rests on the URLs — see the book's Operations page and the
  served OpenAPI document, which flags every one of them.

- **The dump archive can be written as a ZIP.** `POST /admin/dump` accepts the
  service model's `zip` compression format, packing the manifest, segments and
  multimedia blobs into a single `archive.zip` instead of loose files. Load
  takes no format argument and detects the container, so an archive always
  reads back the way it was written.

- **The admin API gained an activity report and archiving, and the definition
  API gained archetype provisioning** — service capabilities that had no HTTP
  route until now. Under the existing `EHRBASE__ADMIN__ENABLED` gate and
  `ADMIN` role: `GET /admin/report/contribution[/count]`,
  `GET /admin/report/versioned_composition/count` and
  `GET /admin/report/composition_version/count` report CONTRIBUTION and
  COMPOSITION-version activity per service over an optional ISO 8601 time
  interval, and `POST /admin/archive/ehrs` / `POST /admin/archive/parties`
  mark a named set of EHRs or demographic parties archived (a read-neutral,
  idempotent, all-or-nothing marker — never a delete). Alongside the released
  template routes, the definition API now serves the ADL 1.4 archetype store
  (`POST`/`GET /definition/archetype/adl1.4`,
  `GET`/`DELETE /definition/archetype/adl1.4/{archetype_id}`) and the ADL 2
  archetype/artefact views (`GET /definition/archetype/adl2[/count]`,
  `GET /definition/artefact/adl2[/count]`,
  `DELETE /definition/artefact/adl2/{artefact_id}`). All of these are
  ehrbase-rs extensions: the openEHR service model defines the operations, the
  released REST API surfaces no endpoint for them, and no openEHR conformance
  claim rests on the URLs — see the book's Operations page and the served
  OpenAPI document, which flags every one of them.

- **The measured hospital-simulation workload now exercises every claimed
  capability** (#625). The performance run used to touch about a third of the
  capabilities the conformance statement claims, and the rest were listed as
  "not yet exercised" catalogue gaps. Sixteen new operations joined the
  measured workload — demographic registration (person create/read/amend plus
  relationship churn), template example and ADL 2 definition polls, advanced
  and terminology-backed AQL reads, Simplified-FLAT commit and read-back,
  version-provenance (signature) reads, the System API options probe, the
  SMART service-discovery fetch, and the two access-control refusals — so the
  published Workload Coverage table now answers "yes" for every claimed
  capability except eleven that carry a per-capability, register-linked
  reason: either the operation would destroy the measured population
  mid-run (physical deletion and the released admin delete API), or openEHR
  defines no wire and this product serves no route for it, leaving the load
  instrument nothing to send. No row is left undecided, and a future journey
  that lands one of those capabilities is forced to delete its exclusion.
- Measured runs can now drive a SMART-secured deployment: the load client
  mints the scope-limited access token its ixit principal declares (once per
  token lifetime, never per request), so a deployment running the SMART
  resource-server posture — the standard EHRbase-rs conformance posture — is
  measurable at all. Boundary probes address the read-only and
  unauthenticated principals a deployment declares; a deployment that
  declares none simply runs the workload without those journeys.
- **Conformance: the last two untested capabilities now carry real executed
  batteries** (#624). "Demographic archetype validation" and "Bulk EHR load"
  were the two capabilities the conformance report listed with *no cases* —
  named in openEHR's conformance profiles book, but never actually exercised.
  Both are now tested against the released REST wire. Demographic archetype
  validation gets eight isolated cases over the party-commit endpoints: a
  committed PERSON/ROLE is refused when it is not archetype-rooted, when its
  root archetype identifier contradicts its own archetype details, when an
  optional list (contacts, roles, capabilities) is present but empty, and when
  an identity's value is missing or carries the wrong openEHR type — plus an
  accept case proving a fully archetyped party, contacts, addresses and
  languages included, is stored and read back intact. Bulk EHR load is
  verified as what it actually is on released wire — a population loaded
  through the ordinary EHR and composition endpoints — with one case covering
  breadth (eight EHRs, one composition each, all identities distinct and every
  document read back unchanged) and one covering depth (four commits into a
  single EHR, each independently addressable, with an AQL query over that EHR
  returning exactly the loaded set). Both capabilities are now claimed in the
  published conformance statement, and their case-count floors are recorded so
  the coverage can only grow.

- **Conformance: the PARTY_RELATIONSHIP capability is now tested rather than
  excused** (#623). openEHR's released REST API defines no PARTY_RELATIONSHIP
  resource, so the six relationship operations used to be reported as
  "excused — unrealized on this technology profile" even though EHRbase-rs
  serves them. They are now driven for real: fifteen conformance cases execute
  against the `/demographic/party_relationship` routes this product serves of
  its own design, covering create, read, read-at-time, read-at-version, update
  and delete plus their refusal branches. The certificate marks the row
  `extension`, which is a promise as much as a label — no openEHR profile
  result may rest on a route openEHR does not specify, and the runner now
  fails validation if that line is ever crossed (a new `realization-scope`
  gate, with the binding's route required to appear in the published
  extension-surface declaration). Such cases are also skipped, with a cited
  reason, for any system under test whose conformance statement does not claim
  the capability — a route openEHR does not specify is an offer only the party
  making it answers for, so the published comparison against other products
  never charges them for routes they never offered.

- **Conformance runner: a certification claim can no longer be hollow** (#622).
  `cnf-runner validate` now reads the committed party statements beside the
  artifact root and relates every claim to the catalogue, so three new gates
  fail before any system under test is even composed. `claim-completeness`
  rejects a claimed capability with no verdict-bearing case at all, and
  requires a capability whose every case is excused (because the openEHR
  release publishes no wire for it) to name the register entry that
  adjudicated that — an excuse that outlives the missing wire is a finding
  too. `capability-depth` gives every capability a `min_cases` floor so one
  token case can never certify it; floors only ever ratchet up.
  `workload-coverage` requires every claimed capability the measured
  hospital-simulation workload does not exercise to carry an adjudicated
  exclusion, which the conformance certificate now prints with its reason in
  place of the previous bare "NO — catalogue gap" cell. The certificate's
  Profile Report also gains a **Realization** column saying whether a
  capability was verified over released ITS-REST wire or over routes this
  product serves of its own design (the latter can never gate an openEHR
  profile tier).

- **Conformance runner: the SMART on openEHR boundary is now executed, not
  declared** (#538). Three behaviours that were previously carried as
  statement-level claims are real conformance cases: the
  `/.well-known/smart-configuration` discovery document (served from the
  Platform base URL as `application/json`, advertising the required
  `org.openehr.rest` service at an absolute base URL), the resource-scope
  grammar that lets a granted scope reach exactly the operation it names, and
  the 403 refusal of a request the granted scopes do not permit. Because SMART
  is off by default, they run in their own **lane**:
  `CONF_SMART_MODE=1 bash scripts/conformance.sh` boots the server with the
  SMART resource-server posture enabled (`docker/sut-smart.yml`), drives the
  SMART group, and writes to `docs/conformance/<sut>-smart/`; the default lane
  is untouched and remains the published baseline. To exercise scopes at all
  the runner now mints its own short-lived access tokens against a **committed
  test issuer** (`tools/cnf-runner/party/smart/` — public test key material for
  the harness, never usable for anything else), because a CDR validates tokens
  and never issues them and the conformance stack runs no Authorization
  Server. A conformance target that does not run the SMART role simply does not
  declare the lane in its `ixit.json`, and these cases are recorded
  not-applicable with that citation rather than failed.

- **Conformance runner: two more wire behaviours are now measured, not
  excused** (#539, #569). The bulk admin delete's subset selector is exercised
  in the repeated `?ehr_id=a&ehr_id=b` form the openEHR path template asks
  for, proving every named EHR is deleted rather than only the first; and the
  rule that a server stamps its OWN configured system identifier into a
  commit audit when the client supplies none is now checked against that
  identifier, not merely against "some non-blank value". The identifier is a
  deployment fact no openEHR operation exposes, so a conformance target
  declares it in its `ixit.json` (`"system_id": "…"`); a target that declares
  none has those cases recorded not-applicable with that citation instead of
  being checked against a guess. Both behaviours were previously carried as
  cited coverage gaps.

- **Conformance coverage: calling a resource with the wrong HTTP method is
  now measured** (#596). The openEHR REST specification says a method the
  specification recognizes but the addressed resource does not serve should be
  answered `405 Method Not Allowed`, and the HTTP standard it defers to
  requires that answer to carry an `Allow` field listing the methods the
  resource does support. The conformance suite now proves both on a real
  resource — a `DELETE` to the EHR collection, which the specification serves
  only under `POST` and `GET` — instead of recording the behaviour as an
  untestable gap. The `Allow` check asserts that both specified methods are
  listed while tolerating any order and any additional methods a server
  chooses to support.

- **Canonical XML: choose the openEHR schema namespace per request** (#196).
  openEHR publishes its XML schemas in two lineages that differ only by the
  namespace a document declares — `http://schemas.openehr.org/v1` (the stable
  release) and `http://schemas.openehr.org/v2` (the newer, trial release). You
  can now pick one with a `version` parameter on the XML media type:
  `Accept: application/xml; version=2` returns the v2 namespace, and
  `Content-Type: application/xml; version=2` declares a v2 request payload. A
  v2 response is labelled `Content-Type: application/xml; version=2`. Nothing
  changes for existing clients: omitting the parameter (or sending
  `version=1`) serves the v1 namespace under a plain
  `Content-Type: application/xml`, exactly as before, and request payloads in
  either namespace have always been accepted. Asking for a namespace the
  server does not serve is `406 Not Acceptable` on `Accept` and `415
  Unsupported Media Type` on `Content-Type`. Operational-template XML
  (`…/definition/template/adl1.4/{template_id}`) is always v1 and ignores the
  parameter. The parameter is an EHRbase-rs extension — the openEHR REST
  specification predates the two lineages and defines no way to select one.

- **Conformance coverage: the ITEM_TAG routes are now measured** (#288). All
  twenty-three released tag operations — the EHR-wide and demographic-wide
  listings, the COMPOSITION and EHR_STATUS families, and the five demographic
  party families — are enumerated by the conformance instrument for the first
  time; they have no openEHR service-model interface, so they were previously
  invisible to its coverage derivation. Thirty-two new cases turn the five tag
  laws into executed wire assertions: tag identity is the (key, target_path)
  pair, a container's tag collection and a version's tag collection are
  disjoint on read, write and delete alike, `ITEM_TAG.target` is served as the
  bare openEHR identifier, every typed tag route answers 404 for a uid of
  another kind (within the EHR space, within the demographic space and across
  the two), and the `openehr-item-tag` / `openehr-version-item-tag` request
  headers on a commit land in their own separate collections. Tag support is
  reported under a new **ItemTags** capability at the OPTIONS tier, matching
  the specification's own statement that a server need not support ITEM_TAGs.

- **Conformance coverage: the COMPOSITION, CONTRIBUTION and PARTY resources
  are now exercised in canonical XML and in the Simplified Formats, not only
  in canonical JSON** (#288). Eighteen new CNF cases drive
  `Accept: application/xml` reads and `Content-Type: application/xml` commits
  across composition create/update/latest/at-time/at-version, the
  VERSIONED_COMPOSITION container, the composition and contribution existence
  probes, and the whole PERSON create/update/read family, plus FLAT and
  STRUCTURED reads of a composition at latest and at time and FLAT/STRUCTURED
  composition updates. Each row asserts the negotiated response media type
  the specification makes a MUST, and the XML commits are compared against the
  canonical-JSON twin of the same resource, so a format-specific data loss
  shows up as a failure rather than a silent difference. One branch stays
  deliberately unexercised and is now recorded with its full derivation: the
  openEHR release declares `application/xml` for the CONTRIBUTION *commit*
  but publishes no XML form of the commit envelope, which is reported
  upstream rather than invented locally.


- **Served OpenAPI: complete documentation for the six Query operations**
  (#482). The two ad-hoc and four stored AQL executions now document what
  the wire actually does. Every `200` declares the weak RESULT_SET `ETag`
  (an identifier of the result set — ours is a deterministic content digest,
  since the released `ResultSet` schema carries no id field) and carries a
  canonical RESULT_SET example: `columns[]` with the `#N` unaliased-column
  convention, rows whose cells are JSON primitives *and* canonical
  `_type`-tagged RM objects, and the optional `meta` (`_type`,
  `_schema_version`, `_created` in extended ISO 8601, and `_executed_aql` =
  the parameter-SUBSTITUTED text, with `q` keeping the query as submitted).
  The parameters now carry the released semantics: the named-`$parameter`
  binding law and its un-prefixed rule, the `ehr_id` duality (query
  parameter or `openehr-ehr-id` header, deprecated MixedCase spelling
  accepted, a conflict 400), `offset`'s default of 0 and `fetch`'s
  implementation-defined default with the one released prohibition
  (`fetch` cannot be combined with AQL `TOP`), the qualified-query-name
  grammar including the reserved `aql`, and the version exact/prefix
  matching law. Also declared: `415` on the three POSTs, request-body
  examples, and the `Prefer`-scope reason no query response carries
  `Location` or `Preference-Applied`. Where the released text is silent the
  declarations say so explicitly — the reserved protocol keys that never
  bind as AQL parameters, REST paging composing over AQL `LIMIT`/`OFFSET`,
  the URL-vs-body precedence on the POSTs, and the `ehr_id`-scope 404.
  Document only — no wire change.

- **Served OpenAPI: complete documentation for the seven EHR ITEM_TAG
  operations** (#475). The EHR-wide read, the two per-target reads, the two
  collection replaces and the two key-scoped deletes now document what the
  wire actually does. The dual-form `uid_based_id` is spelled out with the
  released version/container sentence and the disjointness it implies (a tag
  has exactly one `target`, so container tags and version tags are separate
  collections and neither read sees the other). The `PUT` bodies are
  described as what they are — a bare JSON array of UPDATE_ITEM_TAG (`key`
  required, `value`/`target_path` optional, `target`/`owner_id`
  server-assigned from the route and ignored if sent), with `[]` quoted as
  the clear-all form, (`key`, `target_path`) as the identity, last-wins on a
  duplicate pair, an empty `target_path` normalizing to absent, and the
  200/204 `Prefer` split (204 by default, 200 carrying the full RESULTING
  list, `return=identifier` resolving to minimal because an ITEM_TAG has no
  uid). The deletes document their SET semantics (every tag under the key on
  the addressed collection) and the released third 404 trigger that makes
  them deliberately non-idempotent. Every operation now declares the target
  guard's 404s (unknown, foreign-EHR, wrong-kind or missing-version target),
  the JSON-only reality (406 for an XML `Accept`, 415 for an XML
  `Content-Type` — no ITEM_TAG type exists in the canonical XML ITS), the
  RM-invariant 422 family on the writes, the `ehr_tags_get` filter semantics
  (AND-combined, exact, case-sensitive, scalar, unbounded), and real ITEM_TAG
  examples including a VERSION-targeted tag. Also recorded: no tag route
  serves `ETag`/`Last-Modified` or accepts `If-Match` — a tag has neither a
  version nor a uid — and the released-text defects met on the way (the
  aggregate read's COMPOSITION-typed response schema, the `_updated`
  responses' copy-pasted "retrieved" wording, `tag_key` vs the `key` path
  parameter, and the "(logically) deleted" wording on a non-versioned
  resource). Document only — no wire change.

- **Served OpenAPI: complete documentation for the three CONTRIBUTION
  operations** (#464). The native change-set commit now declares the whole
  `NewContribution` envelope — `versions[]` of UPDATE_VERSION
  (`preceding_version_uid`, `signature`, `lifecycle_state`, `attestations`,
  `data`, `commit_audit`) plus the change-set `audit`, the accepted `_type`
  spellings (`UPDATE_AUDIT` / `AUDIT_DETAILS` / omitted), the server-set
  `time_committed`, the honoured-if-unused client `uid`, and the
  committer/`system_id` copy-down — with a canonical two-member example (a
  COMPOSITION creation plus an EHR_STATUS modification) and the SPECITS-84
  rule quoted: the envelope stays canonical JSON, only each
  `versions[i].data` takes the FLAT/STRUCTURED form. Every branch is
  documented: `201` with the weak `ETag` carrying the *contribution* uid (not
  a version uid), `Location`, `Preference-Applied` and the `Prefer`-conditional
  bodies (the representation lists the minted version OBJECT_REFs, the
  identifier body the contribution uid, minimal an empty `201`); `400` with
  the released first-version-of-a-MODIFICATION trigger; `404`; `406`; `409`
  (client uid in use — released — plus the non-modifiable EHR, duplicate
  singletons and an EHR_STATUS delete member, flagged as ours); `412` for a
  stale member `preceding_version_uid`; `415`; and the full `422` family
  (empty `versions`, out-of-group change types, data on a delete/attestation
  member, missing data, template and RM-invariant failures). The by-uid `GET`
  documents the plain-UUID `contribution_uid`, `Prefer: return=representation,
  resolve_refs` (members resolved to full ORIGINAL_VERSIONs, which is also
  what makes a simplified `Accept` meaningful), its `200` headers and
  canonical example (members as OBJECT_REFs, full AUDIT_DETAILS with optional
  `description`), and `400`/`404`/`406`. The contribution-list route is
  prominently flagged as our own extension with no openEHR spec behind it,
  and its `offset`/`fetch` clamping (0 / 20, capped at 100 — never a `400`)
  and row shape are now described accurately. Document only — no wire change.

- **Served OpenAPI: complete documentation for the five DIRECTORY
  operations** (#457). Every response now declares its headers (weak `ETag`,
  `Last-Modified`, `Location`, `Preference-Applied`, item-tag echoes) and the
  reads and writes carry canonical FOLDER examples (nested `folders`, `items`
  as OBJECT_REFs); the writes document the `If-Match` precondition — carried
  in the header because these routes have no version segment, so a stale
  value is `412`, never `409` — plus `Prefer`, the `openehr-version` /
  `openehr-audit-details` committal headers and the item-tag headers, and the
  canonical-JSON/XML-only request bodies (a Simplified-Format `Content-Type`
  is `415`, an unfulfillable simplified `Accept` `406`: a FOLDER is not
  templated). The `version_at_time` and `path` query parameters are described
  with the released sentence plus our register-documented resolution rules
  (root-implicit, leading-slash tolerant, folders-only, first-match; a future
  time serves the latest version, a time before the first commit is `404`),
  and every branch the wire serves is documented — the deleted-directory
  `204` on both reads, the `DELETE`'s `204` carrying the new deleted version's
  identity, the `404`s (including an EHR with no directory), the `412`s with
  the latest-uid `ETag`, `400`/`406`/`415`/`422`, and the `409`s that are our
  own design (creating a directory when one already exists, and a
  non-modifiable EHR), each flagged as such. Document only — no wire change.

- **Served OpenAPI: complete documentation for the eight COMPOSITION and
  VERSIONED_COMPOSITION operations** (#450). Every response now declares its
  headers (weak `ETag`, `Last-Modified`, `Location`, `Preference-Applied`,
  item-tag echoes) and a canonical example; the commits document the
  `openehr-version` / `openehr-audit-details` / `openehr-template-id` request
  headers and the four negotiable media types (canonical JSON/XML plus
  `application/openehr.wt.flat+json` and
  `application/openehr.wt.structured+json`); and every branch the wire
  actually serves is described — the `GET`'s deleted-version `204` for all
  addressing forms, the `DELETE` quartet (`204` carrying the NEW deleted
  version's identity, `400` already-deleted, `404`, `409` not-latest with the
  latest-uid `ETag`), `412`/`415`/`406`/`422`, and the `409`s that are our own
  design (duplicate live persistent COMPOSITION per template, and a
  non-modifiable EHR), each flagged as such. Document only — no wire change.

- **Served OpenAPI: complete documentation for the seven EHR_STATUS and
  VERSIONED_EHR_STATUS operations** (#443). Every response now declares its
  headers (weak `ETag`, `Last-Modified`, `Location`, `Preference-Applied`,
  item-tag echoes), canonical examples, and the 406/415 negotiation
  branches; the EHR_STATUS update documents the `openehr-version` /
  `openehr-audit-details` committal headers and the
  `Prefer: return=identifier` response shape. Document only — no wire
  change.

- **`Last-Modified` on VERSIONED_OBJECT container and revision-history
  reads** (#442). `GET …/versioned_ehr_status`,
  `GET …/versioned_composition/{uid}`, and both `…/revision_history` reads
  now carry `Last-Modified` derived from the newest held version's commit
  instant, alongside the existing container-uid weak `ETag` (ITS-REST
  overview *Requests and responses* §"ETag and Last-Modified": both headers
  SHOULD accompany a VERSIONED_OBJECT response).


- **`[server] system_id` — the deployment's own openEHR system identifier is
  now configurable** (#424, `EHRBASE__SERVER__SYSTEM_ID`, default unchanged at
  `ehrbase-rs.local`). The value is stamped into `EHR.system_id` at EHR
  creation (RM *EHR Information Model* §EHR Identifier Allocation: the
  identifier "that would normally be used for locally created EHRs"), into
  `AUDIT_DETAILS.system_id` whenever the client supplies none through
  `openehr-audit-details` (the REST API requires the server to "set it to its
  own configured system identifier"), and into every minted
  `OBJECT_VERSION_ID.creating_system_id`. Previously it was a hard-coded
  constant that no configuration could change. Choose it before the first EHR
  is created and keep it stable — the value is stored per EHR and per version,
  so a later change affects only newly authored data and never rewrites
  existing identifiers. It is distinct from `[server.identity]`, which is only
  the `OPTIONS` manifest's display identity. An empty value, or one containing
  the `OBJECT_VERSION_ID` separator `::`, is refused at boot.

### Removed

- **The bare-root `OPTIONS /` alias of the System API endpoint** (#420). The
  System API defines exactly one location for the Options-and-Conformance
  operation — the API base-path root (`OPTIONS {base_path}`, e.g.
  `/ehrbase/rest/openehr/v1`); the extra bare-root mount was our own
  duplication and answered identically. Clients probing `OPTIONS /` must use
  the base path.

## [3.11.0] - 2026-07-26

### Added

- **Admin console: EHR_STATUS editing and a status version history** (#306). The
  EHR detail screen's **Status** tab is no longer read-only: an **Edit status**
  card toggles `is_queryable` and `is_modifiable` and edits `other_details`
  (canonical-JSON `ITEM_STRUCTURE`; blank removes it), committing a new
  `EHR_STATUS` version conditionally on the version the screen loaded. Every
  other attribute — the subject included — is sent back exactly as the CDR served
  it, so an edit can never drop what the form does not show; a non-object
  `other_details` is refused before anything is sent, and a rejected document
  keeps the CDR's own diagnostic on screen beside the form. If another client
  committed a new status meanwhile, the write is refused rather than overwriting
  it, and the console says so with what to do next. A new **Status history** tab
  adds the versioned view: the `VERSIONED_EHR_STATUS` container plus the selected
  version's envelope facts, the revision history newest-first, a date-and-time
  lookup that resolves the version extant at that instant, and any version's
  document opened by its own `OBJECT_VERSION_ID`. A non-queryable EHR's warning
  now points at the toggle that fixes it.
- **Admin console: SMART scope previewer + effective identity** (#299). The user
  menu's "View scopes" drawer no longer prints a raw list of scope strings. It
  now states **who you are and what decides what you may do** — the
  authenticated principal and the policy source behind it (a Basic session
  replays its CDR account and carries no SMART scopes; an OIDC session's roles
  and permissions come from the same access token whose scopes are listed) — and
  renders every scope as its **parsed grant**: the compartment it delegates to
  (`patient`/`user`/`system`), the resource family and id pattern it reaches, the
  create/read/update/delete/search operations it permits, and a *broad access*
  marker on a bare `*`. Launch contexts and identity claims are labelled as such,
  and an unrecognised scope stays visible verbatim instead of vanishing. A new
  **previewer** field takes any scope string — or a whole space-separated claim —
  and renders the same reading, with an actionable explanation when a
  resource-shaped scope is malformed (a bad compartment, a missing or invalid
  `.<permission>` tail, an unknown resource). The drawer also states plainly that
  scopes **narrow** access and never grant it: the CDR remains the enforcer. The
  reading comes from the same scope grammar the CDR's own SMART gate enforces
  with, so the console's explanation cannot drift from the server's behaviour.

- **Admin console: grouped multi-series result charts** (#296). The results pane
  (both the point-and-click builder and the raw AQL editor) now charts **every**
  numeric result column instead of only the first one: one line per column, named
  by the column's own alias, with a legend whose entries switch a series on and
  off — the last visible series stays on, so the chart never empties itself. When
  a column holds ISO-8601 date/times it is offered as the **X axis** and used by
  default, giving a real time scale in which the points sit at their true
  distance apart whatever order the rows arrived in; the row order remains
  available as the fallback axis. A single numeric column still draws as one
  plain line with no legend. The **Table | Chart** toggle is now offered for
  every non-empty result set, and a result set with nothing to chart (no numeric
  column, or a single row) explains that in the chart pane instead of showing a
  blank box.
- **Admin console: EHR-detail and System-panel completions** (#315). The EHR
  detail screen now opens with a **summary header** read from the EHR resource
  itself (id, creating system, creation time, current EHR-status reference), so
  an unknown or mistyped EHR id is reported once at the top of the screen
  instead of once per tab. The **Create EHR** card takes an optional **EHR id**:
  supply a UUID to create that exact EHR (a non-UUID is refused before anything
  is sent, and an id already in use comes back as the CDR's own conflict with
  what to do next), or leave it blank as before. The composition viewer gains
  **Delete composition** — the openEHR *logical* delete of the latest version
  behind a confirmation dialog, which returns to the EHR's composition list on
  success and, if the version moved on meanwhile, says so instead of deleting
  the wrong one — and a **Versioned object** card reading the versioned
  composition and the selected version directly (lifecycle state, preceding
  version, contribution, signature, whether the version still carries content).
  The contributions tab opens with a **contribution activity** timeline of
  writes per day. On **System**, a **conformance manifest** card shows what the
  CDR advertises about itself through the openEHR System API (product, vendor,
  claimed conformance profile, and the API groups it actually mounts), and the
  served-OpenAPI card gains a **per-family document selector** whose choice
  lives in the URL (`/system?openapi=query`), so a family document is
  shareable and survives a reload.

- **Admin console: run a stored query with its parameters, at the version form
  you choose** (#295). A stored-query row now offers **Run**, which opens a
  runner screen for that query: it shows the stored AQL, prompts one field per
  `$parameter` the query declares, and executes it on the CDR as a real stored
  query (`POST /query/{name}[/{version}]` carrying `query_parameters`) rather
  than re-sending the text as an ad-hoc query. The results land in the same
  results pane as everywhere else, with paging — except when the query sets its
  own `LIMIT`/`TOP`, which the screen says instead of fighting. All three openEHR
  version-resolution forms are selectable and labelled with the exact request
  they will send: **latest** (no version), a **version prefix** like `1` or `1.2`
  (the CDR picks the latest match), or an **exact** `1.2.0`. A parameter value
  that reads as JSON is sent as that type (`38.5` as a number, `true` as a
  boolean); anything else is sent as text, and quoting forces text (`"0123"`).
  A field left blank is not sent at all.
- **Admin console: open a stored query in the query builder** (#295). Stored
  queries and the raw editor now offer **Open in builder** beside *Open in
  editor*: a query that fits the point-and-click builder's model is loaded back
  into it — template, conditions, output shape, ordering and limit — with the
  next version proposed for saving, so a stored query can be revised visually
  instead of by editing text. The load is never lossy: the builder only accepts a
  query it can reproduce **byte for byte**, and anything else (a parameterised
  query, a hand-written shape the builder has no controls for) opens with a
  notice naming exactly what it could not express and a link to work on it in the
  raw AQL editor.
- **Admin console: the stored-query and template tables are paged** (#298). Both
  listings now carry the console's shared pagination footer — which rows are on
  screen out of how many (`26–50 of 137 templates`), previous/next, and a
  rows-per-page choice (25/50/100). The page and the window size live in the
  address bar (`?page=`/`?size=`), so a page is shareable and survives a reload,
  the browser's back/forward walk the pages, and the controls work before the
  console's WebAssembly bundle has loaded. The templates filter still narrows the
  rows client-side; the footer counts what the filter left. Deleting the last row
  of the last page lands on rows rather than on a blank table, and a hand-typed
  window size is clamped to a sane range.
- **Admin console: a real document viewer** (#297). Every pane that shows a
  wire document — the composition viewer, the EHR status tab, the directory raw
  mode, a contribution, a template's OPT and example tabs, a stored query — now
  offers three views of it plus a **Copy** button. **Highlighted** (the default)
  shows the byte-exact document with JSON/XML syntax highlighting from a
  pure-Rust tokenizer (no JavaScript, no new dependency; a very large document
  is shown unstyled instead of tokenized), **Raw** shows the same text
  unstyled, and **Rendered** shows a template-free clinical reading of a
  canonical openEHR JSON document: RM section headings with their type and
  archetype node id, and one label/value row per `ELEMENT` — quantities with
  their units, coded text with its terminology code, a null-flavoured leaf
  saying so. The rendered view needs no operational template, so a composition
  whose template was since removed still reads normally; it folds away the
  bookkeeping (language, territory, category, uid) that the raw views keep in
  full, and is read-only — nothing is stored anywhere.

- **Admin console: stored-query versions are reachable** (#336). Both save
  surfaces (the point-and-click builder and the raw AQL editor) now carry an
  optional **Version** field beside the namespace and name, and state under the
  fields exactly which store a click will perform: leaving it empty stores at the
  server-assigned version and replaces what is there, while a
  `major.minor.patch` version stores a new **immutable** version and is refused
  with the CDR's own message if that pair already exists. **Open in editor** now
  keeps the version it loaded and proposes the next minor one, so editing a
  stored query publishes a new version instead of colliding with the one it came
  from, and a partial pattern (`1`, `1.0`) is refused in the save field with an
  explanation — that form selects the latest matching version when *reading* a
  query, and is not something to file a definition under.

- **Admin console: an Operations panel** (`/operations`) over the CDR's
  operational surfaces — dependency health, build and specification provenance,
  the metric registry, and runtime log control. The health card reads the public
  readiness probe (`GET /health/readiness`) and renders the aggregate plus one
  row per indicator, explaining on screen how that differs from the topbar
  status pill; the build card reports the CDR's version, git commit, `rustc`,
  PostgreSQL target and openEHR specification pins; the metrics card shows four
  headline tiles plus a browser over the whole registry, with the selected
  metric in the URL so a view is shareable; and the log card changes the live
  log filter (and resets it to the boot value) behind a confirmation dialog that
  names the consequence, re-reading the CDR's answer so the panel shows what the
  server confirmed. The screen appears in the sidebar **only when the CDR serves
  its management surface** (the console probes `GET /management/info`); a
  deployment with it switched off sees no Operations entry at all, and an
  individual endpoint left off renders as a stated absence rather than an error.
  The redacted effective configuration is deliberately not duplicated here — the
  CDR serves the same snapshot on both its management surface and its admin API,
  so the panel links to the one viewer on the System screen.
- **Admin console setting `cdr.management_base_url`**
  (`EHRBASE_ADMIN__CDR__MANAGEMENT_BASE_URL`): the CDR's management surface
  including its base path, for deployments that serve it on a separate internal
  listener (`management.port`) or under a renamed `management.base_path`.
  Unset, the console derives `{cdr.base_url}/management`.
- **The compose quickstart enables the CDR's management surface**
  (`docker/ehrbase.dev.toml`): `info`/`metrics` at `private`, `prometheus`
  `public`, `env`/`loggers` `admin_only` — so the console's Operations panel
  works out of the box on the dev stack. The surface remains off by default on
  the bare binary and in the Helm chart.

- **Admin console: delete templates, stored queries and EHRs** when the CDR's
  admin API is enabled. The Template Manager list rows and the template detail
  screen can delete an operational template; the stored-query rows can delete a
  query version from the CDR store (labelled "Delete from CDR", clearly
  separate from the console-local "Remove group"); and the EHR detail screen can
  physically delete an EHR, returning to the EHR list on success. Every action
  confirms in a modal dialog naming the exact object (the query-group removal
  now does too), and every failure names the object and the next action — a
  template still referenced by a committed version, or a session without the
  ADMIN role, is refused by the CDR and reported as such. The console first
  asks the CDR which API groups it serves (the openEHR System API conformance
  manifest, `OPTIONS` on the API base path) and renders **no** delete
  affordance at all when the admin group is not among them.
- **Public health probes (`/health/liveness`, `/health/readiness`)**: the
  server now always serves a complete health family on its main HTTP port,
  unauthenticated and independent of every configuration switch —
  `GET /health` (unchanged: constant `200 OK`, plain-text `OK`),
  `GET /health/liveness` (an identical alias under the
  orchestrator-conventional path), and `GET /health/readiness` (the
  indicator-backed probe: database ping, migrations applied and the in-memory
  component flags, `200` when the aggregate is UP/DEGRADED, `503` when a
  required component is DOWN, with the full per-indicator JSON body). They are
  mounted outside the API's authentication and overload-shedding layers, so
  they answer without credentials and are never shed on a saturated server.
  This family is now the only health surface (see **Removed**).

### Changed

- **`ETag`/`Last-Modified` on every versioned read** (#368). The
  VERSIONED_COMPOSITION and VERSIONED_EHR_STATUS container reads, the
  VERSION-by-id reads, and both revision-history reads now carry the
  versioning headers (container/version uid as the `ETag`; the commit
  instant as `Last-Modified` where the body carries one) — previously only
  the at-time variants did.
- **Unqualified stored-query names are one identity everywhere** (#366). A
  query stored without a namespace (`PUT /definition/query/my_bp/1.0.0`) now
  lands under the openEHR-assumed `misc` namespace — the same identity the
  by-name GET, the listing, the SM calls, and the admin delete address — so a
  bare-named query is no longer invisible to the admin delete (and vice
  versa). Descriptors return the canonical `misc::`-qualified name; a
  bare-name listing pattern also matches its `misc::` composition.
- **Query GETs bind the spec's named parameters** (#364). AQL `$parameter`
  binds on `GET /query/aql` and `GET /query/{name}[/{version}]` now arrive as
  ordinary named query-string parameters (`?temperature_from=36&…`), exactly
  as the REST API documents them — values are typed JSON-first with string
  fallback, a `$` prefix is tolerated, and the previous JSON-object
  `query_parameters=` form remains accepted (a named parameter wins a
  collision).

- **Version identity is the full three-part `version_uid`, compared
  case-insensitively** (#367). Deleting a composition (and reading a version
  by id) with a fabricated `creating_system_id` is now refused (409 / 404) —
  previously only the version number was compared, so a made-up system id
  could delete the latest version. Conversely, a `version_uid` or `If-Match`
  differing only in case is accepted as the same identifier, per the openEHR
  composite-identifier case rule.
- **Item tags follow the spec's identity and target model** (#365). Two tags
  sharing a key on different `target_path`s now coexist (the ITEM_TAG identity
  is the key + target_path pair, per the ITS-REST item-tag prose) instead of
  silently collapsing; a version-addressed tag (`…/composition/{version_uid}/tags`)
  now tags THAT VERSION, disjoint from the container's tags, instead of being
  folded onto the container; the tag's `target` is returned in the RM shape (a
  bare `HIER_OBJECT_ID` or `OBJECT_VERSION_ID`, replacing the former OBJECT_REF
  wrapper — the released RM wins over the stalled OAS schema); tag routes now
  404 when the addressed object is of the other kind; and the
  `openehr-item-tag` / `openehr-version-item-tag` commit headers write to their
  own distinct collections. Deleting by key removes every path under that key
  in the addressed collection (the wire has no path selector).

- **Admin console: accessibility and empty-state polish** (#302). Table header
  cells are now announced as column headers by screen readers, and every
  icon-only control in the query builder (the catalog's expand/collapse
  chevrons, the remove buttons on conditions, groups, columns and sort rules)
  plus the unlabelled column-alias, sort-path and sort-direction controls state
  what they do. Data regions that used to come back as a line of grey text —
  template usage, the served OpenAPI list, the commit-activity chart, an EHR's
  compositions, the directory version history, a version's audit, the query
  builder's conditions and result rows, and a template filter that matches
  nothing — now render the console's standard empty state: an icon, what is
  empty, and what to do about it. The user menu's popover matches the rest of
  the console's panels instead of the widget kit's stock chrome, and the modal
  backdrop is a theme token, so it dims correctly in dark mode.
- **Contribution list shows the change type's display rubric** (#304). The
  EHR contribution-list extension (`GET /ehr/{ehr_id}/contribution`) now
  carries `change_type_rubric` beside the raw `change_type` group code —
  resolved from the openEHR `audit_change_type` terminology group by the CDR
  itself, so clients never maintain a local code table. The admin console's
  contributions tab displays the rubric (code on hover). The SM-catalog
  `delete_opt` service path now also refuses with the same friendly
  409-and-reference-count as the admin template delete while committed
  versions still reference the template, instead of relying on the raw
  foreign-key error.
- **openEHR BASE spec pin refreshed** (#341). The vendored BASE 1.3.0 spec
  text and BMM codegen input now track upstream `specifications-BASE` master
  `e4879576` (24 commits: the SPECBASE-48 RESOURCE_DESCRIPTION invariants,
  the SPECAM-82 CODE_PHRASE package move into base_types, SPECPR-426/386/460
  corrections). No wire or validation behaviour changes: every
  behaviour-relevant item was verified already satisfied by the
  implementation; the regenerated crates differ only in documentation text
  and the CODE_PHRASE module location.
- **Readiness moved from `/management/health/readiness` to the public
  `/health/readiness`** (and liveness to `/health/liveness`). Database-backed
  readiness no longer hides behind `management.enabled` + a probe switch —
  nothing has to be enabled for an orchestrator to probe the server. Point
  existing probes at the new paths.
- **Helm probes use the public paths on the main HTTP port**: the chart's
  `httpGet` liveness/startup probes hit `/health/liveness` and readiness hits
  `/health/readiness` on the `http` port, with no prerequisite — the previous
  render-time failure demanding `config.management.enabled=true` +
  `config.management.probes_enabled=true` is gone. Prometheus scrape
  annotations still point at the management surface (and its separate port when
  configured), unchanged.
- **Admin console: the System screen's activity tile now links to the audit
  browser** (#301) instead of stating that the CDR exposes no audit read
  surface — it does (the IHE ITI-81 retrieval the `/audit` screen has been
  browsing all along). The tile carries a one-line description of the trail and
  an **Open audit browser** button.
- **Admin console: every write now reports its failure as prominently as its
  success** (#301). Uploading a template, creating an EHR, committing or
  updating a composition, saving a stored query or query group, and creating,
  saving, restoring or deleting a directory all raise a failure notification
  naming the object, what the CDR objected to (its diagnostic verbatim), and
  the next action to take — a stale version to reload, a role to sign in with,
  an unreachable CDR to check. Where the diagnostic is worth reading line by
  line (template validation, a rejected composition body) it also stays on
  screen beside the form as before. Previously a failed write showed a quiet
  inline message that was easy to miss after a run of green success toasts.
- **Admin console: query groups are now derived from the stored-query
  namespace** instead of being named sets kept in a console-local file. A
  stored query is identified by a qualified name — `namespace::name`, the
  namespace optional and, per the openEHR REST specification, a reverse domain
  name whose purpose is separation of stored queries by team or organisation —
  so the console groups by exactly that: the **Queries** screen's right-hand
  panel and the **Dashboard**'s cohort tiles are both derived live from
  `GET /definition/query`, and queries saved without a namespace collect under
  *unqualified*. The grouping consequently lives in the CDR: it is visible to
  every openEHR client, survives a console restart, needs no backup, and is
  identical across console replicas. The group create/edit/remove controls are
  gone — a query joins a group by being saved under that namespace — and both
  save surfaces (the query builder and the raw AQL editor) now offer a
  first-class **Namespace** field beside the query name, showing the exact
  qualified name the save will write. Existing local groups are not migrated:
  re-save a query under the namespace you want it grouped by.
- **Admin console: the EHR Directory tab creates an empty root folder**, then
  the structured tree editor builds the hierarchy. The console no longer ships
  or stores named folder shapes to start from.

### Removed

- **`GET /ehrbase/rest/status/health`** is removed. It was a third name for the
  constant liveness answer already served at `/health` and `/health/liveness`,
  with no consumer anywhere in the product (no probe, no client, no
  documentation pointed at it). Point any caller at `/health` (load balancers,
  container `HEALTHCHECK`) or `/health/liveness` (orchestrator probes);
  `GET /ehrbase/rest/status` — the product status document, a different contract
  — is unchanged.
- **`/management/health`, `/management/health/liveness`, and
  `/management/health/readiness`** are removed; the management surface is now
  ops introspection only (info, prometheus, metrics, env, loggers). The
  aggregate component view is the body of the public `/health/readiness`.
- **The `management.probes_enabled` and `management.endpoints.health`
  configuration keys** are removed. Configuration is strict, so a config file
  or `EHRBASE__MANAGEMENT__PROBES_ENABLED` / `…__ENDPOINTS__HEALTH` environment
  variable still setting them **fails at boot** with an unknown-key error —
  delete the keys; the probes are always on.
- **Admin console: folder templates are removed.** The named FOLDER-tree shapes
  the Directory tab could start from (and their `admin-ui-folder-templates.json`
  store) are gone; create the empty root and build the hierarchy in the tree
  editor, which commits it as ordinary directory versions the CDR owns.
- **Admin console: both console-local JSON stores are removed** —
  `admin-ui-groups.json` (query groups) and `admin-ui-folder-templates.json`
  (folder templates). The console now keeps **no local domain state at all**:
  it has no database and writes no files, so every fact it shows lives in the
  CDR and reads the same for every client and every replica. Delete the files;
  nothing reads them.
- **The admin console's `groups_file` configuration key**
  (`EHRBASE_ADMIN__GROUPS_FILE`) is removed. Console configuration is strict,
  so a config file or environment variable still setting it **fails at boot**
  with an unknown-key error — delete it.

### Fixed

- **The observability Compose overlay boots again** (#321). Every server
  variable in `docker-compose.observability.yml` was written in a
  single-underscore form (`EHRBASE_MANAGEMENT_*`, `EHRBASE_OTEL_*`,
  `EHRBASE_LOG_FORMAT`) that the strict boot-time sweep of the reserved
  `EHRBASE_` namespace rejects, so `docker compose -f docker-compose.yml -f
  docker-compose.observability.yml up` failed at startup with unknown-variable
  errors instead of starting the server. The overlay now uses the documented
  `EHRBASE__…` grammar (`EHRBASE__TELEMETRY__OTLP_ENDPOINT`,
  `EHRBASE__LOG__FORMAT`, `EHRBASE__MANAGEMENT__ENABLED`,
  `EHRBASE__MANAGEMENT__PORT`, `EHRBASE__MANAGEMENT__ENDPOINTS__{INFO,METRICS,PROMETHEUS}`),
  with unchanged intent: OTLP traces to the bundled collector, JSON logs, and
  the management surface public on internal port 9464 for the bundled Prometheus
  to scrape. A test now runs the real sweep over every variable the shipped
  Compose files set on the server service, so this class of drift fails in the
  test suite rather than at `docker compose up`.
- **The `compositions_committed_total` metric now counts** (#332). The counter
  was declared and scraped but never incremented, so dashboards over it
  rendered a permanently empty series. Every commit route that lands a
  COMPOSITION version — create, update, delete, and a CONTRIBUTION commit —
  now increments it once per committed version, labelled `change_type` with the
  openEHR `audit_change_type` code recorded on that version's audit
  (`249`/`251`/`523`/…). The increment happens after the transaction commits, so
  a rolled-back write is never counted. In the same audit of the metric
  registry, five metrics that were emitted but not registered
  (`version_signature_invalid_total`, `authz_cedar_decisions_total`,
  `authz_remote_pdp_calls_total`, `atna_audit_rejected_total`,
  `atna_audit_reaped_total`) now carry their `# HELP`/`# TYPE` descriptions in
  the `/management/prometheus` exposition.
- **Admin console: find-an-EHR-by-id works without JavaScript** (#301). The
  finder is now a plain `GET` form: submitting it before (or without) the
  browser app loading redirects to the EHR's detail screen server-side, and
  `/ehrs?find=<ehr_id>` is a shareable shortcut to any EHR. With the app loaded
  the lookup is unchanged — one client-side navigation, no page reload.
- **Admin console: template links and query-string values are now
  percent-encoded via the standard codec** (#293); template ids containing
  reserved characters no longer produce broken links. The console's
  hand-rolled percent encoder is gone — every internal link (the template
  detail link and its tab links, the stored-query "Open in editor" link, and
  the query builder's "Open in raw editor" link) builds its path segment and
  query-string values with the `urlencoding` crate.
- **Admin console: deleting a document reference from a directory folder no
  longer risks row state attaching to the wrong sibling** (#292). The item
  rows of the directory tree editor are now identified by a stable per-item
  identity instead of their position in the folder, so removing one reference
  leaves every remaining row bound to its own reference.

## [3.10.0] - 2026-07-25

### Added

- **CNF total wire-surface coverage gate (#271)**: a new `surface-coverage`
  machine gate in the CNF runner (`cnf-runner validate`) fails on any
  spec-defined wire behaviour with no covering case and no adjudicated
  exception — enforcing breadth, not just pass rate (`.claude/rules/testing.md`
  §CNF coverage). It measures three axes against the RELEASED spec sources only
  (the SM platform interfaces + the ITS-REST docs text, never the vendored
  OAS): (1) every SM operation of the platform interfaces has an `its-rest`
  binding or a cited boundary; (2) every realized binding's declared outcome
  and format branch is exercised by a case or excepted; (3) the cross-cutting
  wire behaviours (conditional headers `ETag`/`Location`/`Last-Modified`/
  `Prefer`/`If-Match`, JSON+XML negotiation, the 406/415 families, the error-body
  and deprecated-media families) map to covering cases or exceptions. The
  authored, spec-cited exception ledger is a new committed artifact
  (`tools/cnf-runner/artifacts/vocab/wire_surface.yaml`, with a published JSON
  Schema); `cnf-runner validate --specs …` refreshes a deterministic coverage
  report at `docs/conformance/coverage-report.md`. Coverage only ratchets up.
- **CNF catalogue content deepening (coded-text value dimension, deferred
  grounds, spec-authored corpus) (#278)**: the coded-text content cases
  (`CONT-DV_CODED_TEXT-validate_local_codes` / `-validate_ext_term`) gain an
  acceptance-direction `value` dimension — value = the bound rubric, value ≠
  rubric (an arbitrary label), and value = the raw code are all **accepted**
  (no RM invariant requires `value` to equal the coded rubric — the "must be
  the rubric" text is `dv_coded_text.adoc` Description prose, registered as
  AMB-55), while an **empty** value is rejected (the sole value invariant, RM
  `dv_text.adoc` §Invariants `Valid_value: not value.is_empty`); the
  synthesized OPTs now bind component-ontology rubrics for their local
  constraint codes. New functional coverage: a **template-example round-trip**
  (the generated example commits back cleanly) and a **deprecated-media Accept
  → 406** response-side case (the ICS-conditional companion to the existing
  request-side 415, under AMB-39). Registered as spec-silent boundaries:
  **Accept q-value negotiation strictness** (AMB-56 — ITS-REST defines only
  "unfulfillable Accept → 406", nothing about q-value weighting) and a
  **simplified-inner-data CONTRIBUTION surface** (AMB-57 — ITS-REST 1.1.0
  commits CONTRIBUTIONs canonical-only). All remaining corpus-manifest
  "structural placeholder" markers are replaced with spec-authored fixtures or
  cited boundaries.

- **Version-signature conformance breadth (CNF)**: a distinct-signature-per-
  version case (the signature is computed over the version's canonical form,
  which includes `uid` — two versions can never share a signature; RM common
  master06 §Digital Signature), backed by a new `distinct_from` fact on the
  runner's signature assertion and a `signature` capture on the
  version-envelope read binding. DIRECTORY (FOLDER) version-signature cases
  land SM-anchored as N/A-with-citation on ITS-REST 1.1.0 (no
  `versioned_directory` resource — AMB-24), activating automatically if a
  later ITS release adds the endpoint. The runner's binding-completeness gate
  now mirrors the interpreter's variant-based binding selection.

- **ADL2/OPT2 templates are full FLAT/STRUCTURED peers of OPT 1.4 (#269)**: a
  FLAT (`application/openehr.wt.flat+json`) or STRUCTURED
  (`application/openehr.wt.structured+json`) composition **commit** keyed to an
  ADL2-registered template now resolves and is validated against that template's
  archetype constraints, exactly as an ADL 1.4 commit is. Two behaviours were
  brought to parity: the am24 (OPT2) Web-Template builder now populates the
  archetype-conformance constraints (existence, cardinality, closed-attribute
  sibling sets, archetype slots, structural stubs) that composition validation
  reads — so an ADL2-template instance is archetype-constraint-checked, not only
  RM- and terminology-checked — and the runtime template resolver falls back to
  the ADL2/OPT2 store when a template id is not an ADL 1.4 template (previously a
  commit against an ADL2-registered template returned **422 "operational template
  not known"**).
- **Citation metadata (`CITATION.cff`)**: the repository is now citable in
  research papers — GitHub renders a "Cite this repository" button (APA +
  BibTeX) from the new CFF 1.2.0 file (author with ORCID, Apache-2.0,
  abstract, keywords, release version/date). A `citation-guard` CI job
  schema-validates the file and enforces that its `version` matches the
  workspace version; the release procedure bumps `version`/`date-released`
  on every cut.

### Changed

- **AQL engine: post-streaming optimization rungs** (measured, one change per
  rung): the streaming shape's dead root LATERAL is elided when the root is
  unreferenced (one fewer `pk_node` probe per version row; a bare
  `uid/value` projection now runs with zero node probes), and the
  `archetype` predicate column is case-folded at write (BASE base_types
  master05 §Composite Identifiers and Case) so archetype equality is plain
  indexed equality — `LOWER()` disappears from every containment hop.
  Measured on the seeded 10k bench: ward statement execution −11.3%,
  buffer reads −10.7%, planning −10.3%; stress knee re-measured at the
  committed 512 arrivals/s. The aql-probe instrument now attributes
  planner time per statement (`pg_stat_statements.track_planning`).


- **Docker / Compose deployment rework (#282)**: a from-the-ground-up rebuild
  of the container surface for smaller images, faster builds, and a
  production-grade posture on every build.
  - **One Dockerfile per image, two targets, zero drift**: `docker/Dockerfile`
    and `docker/admin-ui/Dockerfile` now each expose `runtime-from-source`
    (what `docker compose build` uses) and `runtime-prebuilt` (what CI uses),
    both sharing a single runtime stage — so the compose-built and published
    images can no longer diverge. The separate `*.runtime` Dockerfiles are
    removed.
  - **Faster rebuilds**: dependency compilation is split into its own
    `cargo-chef` layer, so editing application code no longer recompiles
    dependencies, and CI now reuses that layer across runs via an exported
    build cache.
  - **Debian 13 + digest pinning**: builder and runtime moved to Debian 13
    ("trixie"); the runtime is `distroless/cc-debian13` (non-root user 65532).
    Every base image is now pinned by immutable digest, and the bundled
    versions are refreshed — PostgreSQL 18.4, Keycloak 26.7.0, SeaweedFS 4.40,
    Grafana otel-lgtm 0.29.2.
  - **Compose**: the optional services are now opt-in behind profiles
    (`--profile s3` for SeaweedFS, `--profile keycloak` for Keycloak); every
    service declares memory/CPU limits mirroring the Helm chart; Keycloak has a
    real healthcheck; and there is no hard-coded project name, so the dev,
    conformance, and E2E stacks no longer collide.
  - **Build provenance** no longer reads `.git` from the build context (which
    is now excluded, shrinking the context and stabilising the cache): the
    `/management/info` commit SHA flows through the standard `REVISION` build
    argument (the same value as the `org.opencontainers.image.revision` label)
    and degrades to `unknown` when unset — never a failed build.
- **Simplified Formats folded into `openehr-its` (#268)**: the FLAT /
  STRUCTURED / Web-Template implementation moved from the standalone
  `openehr-flat` crate into `openehr-its` as the `openehr_its::flat` module,
  mirroring the openEHR ITS component decomposition (Simplified Formats is a
  STABLE ITS-REST 1.1.0 sub-specification, alongside canonical JSON, XML, and
  the REST contract this crate already houses). Pure packaging refactor — no
  change to the FLAT/STRUCTURED/Web-Template wire behaviour.

### Fixed

- **ADL2 filler-root naming in the projected WebTemplate**: a
  `use_archetype`-filled archetype root resolved its display rubric in the
  component (filled archetype) terminology first, so the template-side slot id
  could false-positively match an unrelated internal id of the constituent
  (e.g. a filled OBSERVATION surfacing as "history"). The slot rubric now
  resolves in the introducing template's own terminology first (ADL2 obliges
  the introducing artefact to define its node ids), with the component scope
  as last resort — FLAT paths over filled ADL2 templates carry the
  template-declared names.
- **CNF runner: `openehr-template-id` for in-flow-provisioned templates**: the
  simplified-format commit header now resolves from the committed data set's
  own manifest-declared `template_id` (falling back to the case's provisioned
  template list), so cases that upload their template inside the flow (the
  ADL2 FLAT pair) drive the commit correctly instead of omitting the header.

## [3.9.0] - 2026-07-24

### Added

- **Content structural conformance cases from the official schedule**: the
  master15 COMPOSITION content×context tables and the master16 ENTRY-family
  tables (OBSERVATION, HISTORY, EVENT, ITEM_STRUCTURE) are now encoded under
  their verbatim official ids, replacing the ad-hoc structural cases that
  had been authored on the false claim that those chapters were empty;
  derivable catalogue extensions beyond the official cells survive as
  flagged addition cases.

- **Dual POC measured records on the v3.8.0 build, both directions
  published**: ehrbase-rs earns class POC (normative hour at 2.03/s
  offered, worst p99 108 ms, 0 errors / 7,320 requests); upstream
  EHRbase 2.34.0 on the identical instrument, corpus, and resource floor
  does not (ward-dashboard AQL p99 10.9 s vs the 1 s ceiling, 2.4%
  errors). Comparison page and all measurement visuals derive from the
  committed runner artifacts.

### Changed

- **Version-signature read verification is now `strict` by default (#273)**:
  with signing enabled and `signing.verify_on_read` unset, the server now
  recomputes the signature of every version it served and returns a `500`
  integrity fault on a mismatch, instead of the previous silent-pass (`off`)
  default that signed every version and then never checked it. Set
  `signing.verify_on_read` explicitly to `warn` (log + meter, still serve) or
  `off` (never check) to opt out. **Client-supplied signatures** (an author's
  own signature, or one carried by an imported version) are tracked as such and
  are always stored verbatim and never re-verified, so strict-by-default never
  rejects a legitimately-stored foreign signature. Our-own-design integrity
  hardening — no openEHR spec governs server-side verify-on-read timing (RM
  common master06 §Digital Signature).

- **CNF catalogue audited case-by-case against the official spec text
  (#231)**: every case in every chapter re-verified across grounds,
  expectations, citations, fixtures, captures, and register linkage, with
  the findings applied directly to the catalogue and register (the durable
  record is the register + closed issues + git history).
  Highlights: spec-overreaching rejection rows removed (AQL TERMINOLOGY
  operation strictness; the mixed-precision interval rows now report-only
  under the SPECPR-380 openness); the SEC-BASIC proposal citations corrected;
  stale stub-era template ids fixed; the delete-latest-version OPT case
  realigned to the official version-less ground; the wrong-template update
  ground rebased onto a fixture that is valid against its own template; the
  physical-EHR-delete binding accepts the OAS-enumerated async 202; eight
  new ambiguity-register entries pin previously prose-only adjudications;
  and every phantom REQUIREMENTS.md pointer now carries its real anchor.

### Fixed

- **Conformance-runner commit provisioning fails loud**: a `requires.commit`
  key resolving to a plain composition fixture was silently skipped, leaving
  the case's committed-state precondition unestablished; a single object now
  commits as a one-item set and any other shape is a provisioning error.

- **The measured-window driver accepts the spec-legal `204 No Content`
  minimal-return form** on create-family writes (ITS-REST: with
  `Prefer: return=minimal` a service SHOULD use 204 when no body is
  returned) — previously every upstream journey commit was falsely
  counted an error; and the upstream comparison stack's database now
  gets the same `shm_size` floor as the ehrbase-rs stack (Docker's 64 MB
  default starved its PostgreSQL during maintenance settling).

## [3.8.0] - 2026-07-24

### Added

- **CNF catalogue: stored-query name-grammar cases** — three new
  `definition_query` cases pin the ITS-REST `Qualified_query_name` grammar:
  a plain unqualified name and a namespace-less dotted name (the dot is part
  of the query-name character set, not a namespace separator) both store and
  read back, and the reserved query-name `aql` is rejected case-insensitively.
- **`cnf-runner stress-compare`** — the cross-SUT stress overlay: both
  systems' latency-throughput curves on one canvas, rendered
  deterministically from the two committed `stress.json` reports (driven
  by `scripts/render-comparison.sh`); both directions on equal footing.
- **Measured runs record resource telemetry**: each measurement in
  `results.json` now carries an optional, schema-published `resources`
  block — per-container (server and database separately) CPU, resident
  memory, block-device and network I/O sampled every 10 s across the
  whole window (run-clock offsets, warmup/measured/drain phase stamps),
  plus the database volume's on-disk size at four anchors (empty → scale
  seed → ward seed → after the window) with the derived bytes per
  committed composition. Sampling is enabled by the new optional
  `containers` block in the ixit (compose container names); without it a
  run records no resources and the report says so — telemetry never
  influences a class verdict. Two new rendered assets (the resource
  time-series and the disk-growth chart) join the perf-assets family and
  the book's Performance chapter, drift-guarded in CI like every
  published number.
- **`cnf-runner aql-probe`** — the seeded-corpus AQL optimization probe:
  fires the measurement machinery's own AQL vocabulary against a freshly
  seeded server, records wire-latency percentiles per probe, and
  attributes the database-side cost per statement (`pg_stat_statements`
  through the ixit `containers` capability, degrading honestly without
  it). Report schema published (`aql-probe.schema.json`); exploration
  evidence only — never a conformance record.
- **Stress steps carry resource telemetry** — every load-ladder rung
  records the same per-container CPU/memory/I/O series as the measured
  class runs over its own warmup+hold window, so a breached rung shows
  where it saturated; the stress progress stream now logs each rung's
  verdict live (stable/BREACHED with the sustained rate, resource peaks,
  and named breaches) plus a ladder recap, and measured class runs log
  their verdict evidence at window end.
- A **diurnal day-curve** arrival option for the extended 8/12-hour
  measured holds (ITU-T E.500 busy-hour semantics: the class floor is the
  busy-hour rate).
- The conformance certificate gains a **Workload Coverage** section:
  claimed capabilities vs the set the measured hospital simulation
  actually exercised, with untouched claimed capabilities listed
  explicitly as journey-catalogue gaps.
- `scripts/generate-ckm-examples.sh` — regenerates the committed CKM
  example payload skeletons from a running SUT's example endpoint;
  `scripts/vendor-ckm-templates.sh` now vendors the runner's journey
  template pack.
- **Conformance visuals**: the capability-matrix heat grid (one cell per
  claimed capability, grouped by profile tier, evidence encoded as a
  CVD-safe color AND a glyph) and per-chapter outcome bars, rendered
  deterministically from the committed verdicts/results by the new
  `cnf-runner conformance-assets` subcommand
  (`scripts/render-conformance-assets.sh`, CI regenerate-and-diff
  guarded) and embedded on the book's conformance and comparison pages
  (both SUTs) and the landing page.

### Changed

- **`--skip-seed` and the sidecar corpus index are retired** (CLI flags on
  `perf`/`stress`, the `CONF_PERF_SKIP_SEED` pipeline variable): every
  measurement instrument now always seeds a freshly composed, empty
  server and the stack is torn down afterwards — seed reuse bred
  stale-state confusion.
- **Measurement instruments settle database maintenance
  deterministically** (`vacuumdb --analyze` through the DB container)
  after seeding and before every measured window and stress rung —
  a stale-statistics plan after the million-row seed cost a measured ~9×
  on the ward-worklist query; settling moves that debt outside every
  measured window, identically for every SUT.
- The CNF measured-performance workload is now a full **hospital
  simulation**: the class cases (`PERF-hospital_sim-*`, renamed from
  `PERF-mixed_load-*`) schedule clinical journeys — ADT
  admission/discharge, vitals rounds, the medication loop, medicines
  reconciliation, asynchronous laboratory/imaging order-to-result
  pipelines, specialist/registry reporting, public-health notifications,
  chart review, ward dashboards with a registered stored query, versioned
  corrections, contribution audit review, workflow tagging, logical
  deletion, and template polling — expanding into 22 measured operation
  kinds instead of 4, each with its own HDR-V2 record. The
  population-anchored envelope is unchanged and now validator-enforced
  (the expanded write share must reconcile to the derivation's 10:1..50:1
  read:write band); journey payloads commit against 15 COMPOSITION-rooted
  openEHR CKM templates vendored with provenance.

### Removed

- **The transitional benchmark lab** (`tools/benchmark`,
  `scripts/benchmark.sh`, `docker/benchmark/`, the manual benchmark
  workflow, and the committed `docs/benchmarks/**` artifacts): all
  measurement is native to the CNF runner — measured class runs, the
  stress ladder, the AQL probe, and the cross-SUT stress overlay — and the
  comparison page now derives its performance side from the committed
  `docs/conformance/<sut>/stress.json` reports (upstream shown as "not
  measured yet" until its report lands, never a one-sided claim).
- The completed ECC→CNF cutover comparison lane: the generated
  `docs/conformance/cnf-comparison.md`, the `cnf-runner compare-ecc`
  subcommand, the drift gate, and the preserved ECC catalogue/map (all in
  git history; the five deferred grounds are re-registered on the
  catalogue-deepening tracker). The `docs/conformance/CATALOG.md` pointer
  stub is gone with it, and the CNF 2.0 design record moved to
  `docs/conformance/cnf-design.md` as a permanent reference document.

### Fixed

- **Storing a query under the reserved name `aql` is now rejected** with
  400, case-insensitively and whether or not a namespace is supplied
  (ITS-REST `Qualified_query_name` §NOTE — the name would collide with the
  ad-hoc `/query/aql` route). A three-part `ns::aql::name` name keeps
  working: its middle segment is the formalism, not the query-name.
- **A coded value whose text is not the template-bound rubric is now
  rejected at commit** (422 naming the path, the committed value, and
  the bound rubric): RM `DV_CODED_TEXT` — "value must be the rubric from
  a controlled terminology" — enforced wherever the template itself is
  authoritative for the rubric (archetype-local at-codes and explicitly
  bound external term definitions, any bound language); `openehr`-
  terminology codes stay unchecked (the terminology ships official
  translations the template cannot enumerate), and a bound code with no
  rubric stays accepted. The once-accepted code-as-value instance is a
  pinned rejection.
- **Coded-text example values now carry the template-bound rubric**: the
  Web Template builder resolved display labels only for local at-codes,
  so an external code's rubric (OPT `term_definitions` keyed
  `TERMINOLOGY::code`, e.g. SNOMED-CT bindings) was lost and generated
  examples emitted the raw code as `DV_CODED_TEXT.value` — spec-invalid
  instance data (RM: "value must be the rubric from a controlled
  terminology"). The qualified key now resolves; the covid19 example
  regenerates with rubrics; every pack example commits clean on strict
  validators.
- **Child-assembled `DV_INTERVAL` values now carry the mandatory boundary
  flags**: an interval built from `lower`/`upper` sub-path children (the
  FLAT builder's container path — template examples included) previously
  omitted `lower_unbounded`/`upper_unbounded`/`lower_included`/
  `upper_included`, making every half-open interval spec-invalid (BASE
  `Interval`: the flags are mandatory and `Limits_consistent` is
  unevaluable against an absent bound); the flags now derive from bound
  presence, an explicit datum flag wins, and the committed CCTA example
  is regenerated. Strict validators (upstream EHRbase) rejected the old
  instances with 422.
- **Population AQL with `LIMIT` now streams instead of materializing the
  corpus**: a LIMIT-bearing, unordered, non-DISTINCT, non-aggregate
  population query lowers to a streaming FROM shape (the current-version
  spine with `LATERAL` node probes), so PostgreSQL stops at the LIMIT
  instead of building an archetype-anchor bitmap over every matching node
  first — measured on a million-composition corpus, the cross-EHR ward
  worklist drops from ~113 ms to ~2 ms per execution (~40× fewer buffer
  reads); ordered/aggregate/EHR-scoped queries keep the previous plan
  shape, and result semantics are unchanged. A version-field projection
  of `uid`/`contribution_id`/`lifecycle_state` no longer joins the audit
  table it never reads.
- **AQL cross-EHR queries with `LIMIT` no longer collapse under corpus
  scale**: predicates on multi-valued (anchored) paths now lower as
  existential semi-joins (`EXISTS` — the predicate holds when ANY matched
  node satisfies it; deterministic where the previous first-match pick was
  plan-dependent), the archetype anchor index leads with the RM type so
  the whole `CONTAINS`-class + archetype boundary is one index probe, and
  queries that never touch audit fields no longer join the audit table.
  The measured ward-dashboard profile (p99 5.8 s at class-POC scale) drops
  to milliseconds-per-request territory.
- The template **example generator no longer collapses `DV_INTERVAL`
  wrappers** onto a single constrained bound: interval-valued elements keep
  their interval identity (bounds as `/lower`/`/upper` sub-paths per the
  Simplified Formats mapping), fixing generated examples the platform's own
  validation rejected (the CKM CCTA report OPT); the CNF journey catalogue
  re-commits the CCTA imaging report.

## [3.7.0] - 2026-07-22

### Added

- The conformance pipeline assesses **upstream EHRbase (Java)** as a second
  system under test: `CONF_SUT=ehrbase-java scripts/conformance.sh` composes
  the official `ehrbase/ehrbase:2.34.0` + `ehrbase-v2-postgres` images on
  fresh volumes (`docker/sut-ehrbase-java.yml`, readiness probed externally
  — the official image carries no in-container health tooling) and runs the
  same committed catalogue with upstream's own committed party set
  (`tools/cnf-runner/party/ehrbase-java/`). The public comparison
  (`docs/conformance/COMPARISON.md` + the website comparison page) is fully
  generated from the two committed results/verdicts sets — profile verdicts,
  the 39-capability evidence matrix, and failure tables in both directions.
- The conformance runner performs ISO/IEC 9646-style ICS-driven test
  selection: `cnf-runner run --statement` excuses option-gated cases whose
  register branch the party statement does not declare as N/A with citation
  (previously they ran and recorded spurious failures the verdict pipeline
  then excused).
- Conformance badges carry measured amounts: per-tier badges read e.g.
  `PASS 10/10 capabilities`, the overall badge `CORE+STANDARD PASS ·
  323/323 cases` — derived from `verdicts.json` + the capability matrix,
  never hand-typed.


- Read-only role support in RBAC: a principal carrying the configured
  `authz.rbac.readonly_role` (default `READONLY`) is refused with `403` on
  every write operation — creating an EHR, committing a composition,
  uploading a template, and any update/delete — even when it also holds
  granting roles such as `ADMIN`. Reads and AQL queries stay permitted, so a
  `READONLY` account is an authenticated, view-only principal. The dev compose
  stack ships an `ehrbase-readonly` account (password `ehrbase`) for
  evaluation.
- CNF 2.0 reference runner, third increment — the executor and both verdict
  machineries: the data-driven flow interpreter under the five interpreter
  laws (per-row re-provisioning, step-mismatch row abort, errored-vs-failed
  classification, fixed temporal resolution, aggregates-after-last-row) with
  the live HTTP driver realized purely from the operation bindings, the
  reference resolver (corpus/recipes/rows/captures with normative sentinel
  semantics), the normative RESULT_SET equivalence comparator, content-case
  execution via the synthesized generate→commit→expect flow, the party
  artifacts (statement/results/ixit with schema validation and mandatory
  N/A citations), the pure verdict pipeline + deterministic
  report/statement/certificate renderers, the runner-verification pack
  (committed transcript + player: adjudicated verdicts reproduced, broken
  runners rejected), and the performance machinery (class cases with the
  published population-anchored floors, re-checkable HDR V2 measurement
  records, the earned/not-earned pure verdict). Nine published JSON-Schema
  families, drift-guarded. Live-SUT runs (the earned-class measurement and
  pack part 2) execute against a composed SUT via the new `run`/`verdicts`
  CLI once cutover lands.
- CNF 2.0 reference runner, second increment: the complete CNF 2.0 catalogue
  authored from the framework — 347 cases across every schedule chapter
  (EHR, EHR_STATUS, COMPOSITION, CONTRIBUTION, DIRECTORY, ADL 1.4 + ADL2
  definitions, stored queries, demographic, admin, messaging, AQL, content
  data-type and structural validation, simplified formats, Security
  SEC-BASIC + Signing) with 84 per-operation ITS-REST bindings (every
  status/header mapping cited to its OAS source; wire gaps are typed
  `unrealized` declarations, not silent absences), the ambiguity register
  grown to 38 adjudicated entries, and the ECC↔CNF comparison gate CLEAN:
  all 394 active rows of the old harness's catalogue adjudicated
  (350 covered, 5 deferred to the simplified-formats deepening, 18 dropped
  with justification, 9 out of scope, 12 ADL2 rows covered) in the committed
  map with the generated report at `docs/conformance/cnf-comparison.md`
  (drift-guarded). Old-harness retirement follows the owner's report review
  with the executor/emission workstreams so an acceptance instrument runs
  continuously.

- CNF 2.0 reference runner (`tools/cnf-runner`), first increment: the typed
  schedule-artifact model (case cores, per-ITS operation bindings, outcome +
  selector vocabularies, the capability→family→tier matrix, corpus manifest,
  ambiguity register — every closed vocabulary a Rust enum/newtype), a
  published JSON-Schema set for all seven artifact families (committed under
  `tools/cnf-runner/schemas/`, drift-guarded, vendorable by any runner), a
  full cross-artifact validator (id uniqueness, SM-operation and spec-ref
  resolution against the vendored specs, binding completeness, corpus
  integrity, reference/sentinel and decision-table grammars, capability-tier
  consistency), the `cnf-runner` CLI (`emit-schemas`, `validate`), and the
  eight pilot case encodings as the first schedule artifacts. The existing
  ECC (`tools/conformance`) is unchanged and remains the acceptance
  instrument until the comparison gate.
- Performance conformance, measured end to end: a `cnf-runner perf` run plays
  an open-loop offered-load schedule against a composed server at a
  population-anchored volumetric class (proof-of-concept, small, large,
  regional), records re-checkable HDR histograms into the conformance
  results, and earns — never declares — a class verdict recomputed by the
  verdict pipeline. `CONF_PERF_CLASS=<class> scripts/conformance.sh` runs it
  as a pipeline stage; the earned classes flow into the verdicts, report,
  certificate, and a performance badge. Published SVG assets (the class
  ladder and per-class latency charts) plus a generated summary are rendered
  from the committed measurement records by `scripts/render-perf-assets.sh`
  and guarded against drift in CI, and a new **Performance** chapter on the
  documentation website explains the class ladder, the floors' derivation
  from official activity statistics, how a coordinated-omission-free run
  works, and how to reproduce it.
- The sustained-window ladder: `cnf-runner perf --hours 1|2|4|6|8|12`
  (pipeline: `CONF_PERF_HOURS`) extends a class run's measured window beyond
  the normative hour — a longer hold of the same offered load is a stricter
  demonstration and persists like any measured run. There is deliberately no
  shortened run.
- A step-load **stress instrument**, distinct from conformance:
  `cnf-runner stress` climbs short intense load steps (geometric doubling,
  ~two-minute holds, bisection refinement) to the **maximum sustainable
  throughput** inside a latency budget, over the same seeded corpus and
  workload mix as the class runs. The report (`stress.json`,
  schema-published, environment-bound, per-step re-checkable histograms)
  earns no class and never touches the conformance results; the class floors
  appear as context only. A latency-throughput curve SVG renders from the
  committed report through the same drift-guarded asset pipeline, and the
  documentation's Performance chapter tells the two-instrument story.

### Changed

- The conformance acceptance instrument is now the CNF 2.0 reference runner
  (`tools/cnf-runner`) end to end: `scripts/conformance.sh` composes the SUT
  on fresh volumes, executes the committed machine-readable catalogue,
  computes verdicts through the pure pipeline, and writes
  results/verdicts/report/statement/certificate + badges per SUT. The ECC
  harness (`tools/conformance`) is retired — its final inventory is
  preserved at `tools/cnf-runner/comparison/ecc-catalog.tsv` and the
  reviewed cutover record is `docs/conformance/cnf-comparison.md`; the
  previous ehrbase-java comparison artifacts are frozen as historical data.
  Committed per-SUT party sets (ixit + statement) live under
  `tools/cnf-runner/party/`.
- Verdict semantics: a REQUIRED capability whose every selected case is
  excluded by a schedule-registered ambiguity (an unrealized wire on the
  technology profile, e.g. ADL 1.4 archetype provisioning under ITS-REST
  1.1.0 — AMB-41) is now recorded as an explicit `unrealized` scope
  exclusion on the certificate instead of silently failing the tier; the
  API-presence capabilities (EHR/DEFINITION/QUERY API) are evidenced by
  chapter exemplar cases.
- The benchmark harness converged onto the conformance runner's corpus,
  recipes, and ixit topology, so both instruments seed identical clinical
  documents through the public write path. The performance numbers in the
  README and on the website are no longer hand-typed: they derive from
  committed run artifacts (the benchmark comparison charts and the CNF
  measurement records), and the site stale-numbers guard now also rejects a
  hand-typed rate, latency, or footprint in the sources.


- OPT-1.4 → ADL2 conversion fidelity: `DV_ORDINAL`/`DV_QUANTITY` constraints
  now convert to real AOM2 attribute tuples (`[value, symbol]`,
  `[units, magnitude(, precision)]`) instead of loose unconstrained nodes;
  slot include/exclude assertions are carried (both retained 1.4 slots and
  the filled-slot `include` naming the embedded archetype); OPT
  `default_value`s are carried and serialized as the ADL2 `_default`
  pseudo-attribute; temporal constraints keep both the ISO8601 pattern and
  the range plus assumed values; `referenceSetUri` becomes an ac-code term
  binding; `CONSTRAINT_REF` resolves against the merged 1.4
  `constraint_definitions`/`constraint_bindings`; and everything a
  decomposed root cannot express (out-of-scope bindings, tuple assumed
  values, `DV_STATE` machines, unconvertible assertions) is reported in the
  converted archetype's `RESOURCE_DESCRIPTION.conversion_details`. The
  whole vendored OPT corpus now converts, validates and re-parses as the
  standing test gate.

### Fixed

- OPT 1.4→2 decomposition now emits phase-1-clean ADL2 sources for every
  template in the corpus: a `-`-specialised embedded root (whose
  differential lineage a flattened OPT cannot resolve) is emitted as an
  unspecialised depth-0 archetype with every dotted code renumbered into
  the flat code space, and 1.4 node codes legitimately reused across
  sibling subtrees re-mint archetype-wide-unique ADL2 ids — terminology
  definitions and bindings follow in both cases, and every remap is
  recorded in the converted archetype's `conversion_details` provenance.

- The ATNA Audit Record Repository no longer loses records under a sustained
  write load: the audit drain now takes queued events in batches and
  persists each batch in one multi-row `INSERT` (the previous per-event
  round trips saturated far below write-path rates, filling the bounded
  queue and fail-open dropping the tail). Drop warnings are rate-limited to
  one per interval carrying the count since the previous warning instead of
  one log line per dropped record (the exact count stays on the
  `atna_audit_dropped_total` metric), and the default
  `audit.queue_capacity` rises from `1024` to `8192` for burst headroom.

- Composition validation closes eight archetype-constraint enforcement gaps
  the CNF content chapter exposed: `C_STRING` list/pattern constraints on
  `DV_IDENTIFIER.issuer`/`assigner`/`type` (only `id` was checked);
  `DV_MULTIMEDIA.size` against `C_INTEGER` list and range constraints
  (previously unvalidated); `C_ATTRIBUTE` existence `1..1` on
  `OBSERVATION.state`/`protocol`, `HISTORY.summary`, and `EVENT.state` now
  rejects the absent attribute; `DV_SCALE` value/symbol value-set
  constraints (generic `C_REAL` list + `C_CODE_PHRASE` code list — AOM 1.4
  has no `C_DV_SCALE`) are enforced, including on `DV_INTERVAL` bounds;
  `timezone_validity` on `C_TIME`/`C_DATE_TIME` (mandatory and prohibited)
  is honoured; half-open (one-side-unbounded) temporal range constraints
  reject out-of-range values; a `DV_PROPORTION` of kind fraction or
  integer-fraction with a non-zero `precision` is rejected
  (`Fraction_validity`); and a partial `DV_TIME` such as `10` is no longer
  over-rejected against `HH:??:??`/`HH:XX:XX` patterns (optional and
  not-allowed fields both admit an absent field).
- A `DV_TIME`/`DV_DATE_TIME` literal carrying a fraction on the hours or
  minutes component (e.g. `10.5`, `10:05.5`) is now rejected: openEHR
  supports fractional seconds only (BASE time types §ISO 8601 semantics not
  included).
- A `DV_URI` whose value has no URI scheme (e.g. `xyz`, `www.example.org`)
  is now rejected on commit per the CNF content schedule's RFC-3986 rule;
  plain-text URI content after the scheme remains accepted per the RM's
  plain-text allowance.
- A COMPOSITION create (`201`) or update (`200`) whose response is negotiated
  as a Simplified Format (`Accept: application/openehr.wt.flat+json` or
  `…wt.structured+json`) now returns the `ETag` and `Location` headers, matching
  the canonical (`application/json`/`application/xml`) response. Previously a
  FLAT/STRUCTURED commit body omitted both version-id headers, so clients could
  not read the new version uid or resource URL from a simplified-format commit.
- Composition validation now rejects a `DV_DURATION` whose value carries a
  decimal fraction on any component other than seconds (e.g. `P1Y3M4DT2.5H` or
  `PT2H14.5M`). openEHR permits a fraction only on the seconds component
  (BASE time types: "in openEHR, only fractional seconds are supported"), so
  such a value now fails its RM `Value_valid` invariant with `422` instead of
  being accepted.
- Composition validation now enforces a `DV_QUANTITY` constraint that fixes a
  measurement `property` (with no enumerated unit list): the committed `units`
  must be a unit of that physical property (per the openEHR measurement
  property↔unit table). A quantity constrained to `length` committed with a
  mass unit such as `mg` is now rejected with `422` instead of being accepted.
- Composition validation now rejects a coded value whose terminology is
  foreign to a `C_CODE_PHRASE` constraint that explicitly binds the
  archetype-`local` terminology with a closed code list. Committing a
  `DV_CODED_TEXT` whose `defining_code` uses, e.g., SNOMED-CT against a
  `local`-scoped closed list now yields `422` instead of being accepted.
- The AQL `ehr_id` execution scope now also binds bare `FROM EHR e` sources:
  a scoped query without a CONTAINS chain previously ran over the whole
  population instead of the single EHR context the `ehr_id` parameter selects
  (ITS-REST query `Request.md` §Common Headers and Query Parameters).
- A CONTRIBUTION delete member targeting the EHR_STATUS is now refused with
  `409 Conflict`: `EHR.ehr_status` is mandatory (RM ehr, EHR class, 1..1), so
  deleting the only status would leave the EHR violating its own invariant.
- FLAT/STRUCTURED commits: spec-listed direct RM-attribute paths that an
  operational template leaves unconstrained are no longer rejected as unknown
  paths. `ACTION/ism_transition` (`current_state`/`transition`/`careflow_step`
  + `_reason:i`) and `ACTION/time`, plus `INSTRUCTION/narrative`,
  `OBSERVATION/history_origin`, `ACTIVITY/timing` + `action_archetype_id`, and
  `INTERVAL_EVENT/width` + `math_function`, are now built from their datum
  parts per the ITS-REST Simplified-Formats `master05-rm_mapping.adoc` per-type
  tables, and emitted symmetrically on the reverse (RM → FLAT) direction so
  round-trips stay lossless. Previously a client-supplied `ism_transition` was
  rejected with "unknown simplified path" and the ACTION state fell back to the
  synthesized `initial` default.
- AQL paging: the REST `fetch`/`offset` parameters now page over the result
  set the AQL `LIMIT`/`OFFSET` clauses define instead of being rejected with
  `400` when combined. Per ITS-REST query `Request.md`, only pairing `fetch`
  with the deprecated AQL `TOP` modifier is prohibited — that rejection
  remains. Negative `fetch`/`offset` values are now rejected explicitly.


- Spec version identity is now derived from the `openehr-*` crate versions
  instead of hand-typed literals, fixing the stale values those literals had
  drifted to: the startup banner advertised `ITS-REST 1.0.3` (now `1.1.0`),
  and the AQL `RESULT_SET` `meta._schema_version` was still emitted as
  `1.0.3` (now `1.1.0`, the implemented ITS-REST release). Every `openehr-*`
  spec crate exposes a `SPEC_VERSION` constant (= its crate version; the AM
  crate also exposes per-generation `am14`/`am24` constants from the BMM
  schemas), and the shared provenance constants behind the banner,
  `/status`, `OPTIONS /` (System Options), and `/management/info` read
  those, so a future pin bump propagates everywhere at compile time. The
  served `restapi_specs_version`/`openehr_rest_api_version` identity is now
  the plain version string `1.1.0` (matching the System API OAS example)
  instead of the tag-styled `Release-1.1.0`.
- SM call-status fidelity: service-layer "does not exist" failures now carry
  their granular `CALL_STATUS_TYPE` (`ehr_id_does_not_exist`,
  `composition_does_not_exist`, `template_does_not_exist`,
  `object_version_does_not_exist`, …) end-to-end instead of resurfacing as
  the generic `versioned_object_does_not_exist` after crossing the service
  boundary. HTTP status codes are unchanged (every does-not-exist status was
  and remains `404`); some `404` body messages are now the precise
  construction-site text.

## [3.5.0] - 2026-07-21

### Changed

- Conformance: zero skipped outcomes. The former 35 skips are eliminated —
  11 cases now execute against the documented ehrbase-rs extension surfaces
  (contribution listing, admin template deletion, bare stored-query
  listing), 6 more execute via new composed-stack wiring (an OpenPGP-signing
  sibling instance and a hermetic FHIR terminology fixture with fault
  injection) and loaded-database AQL golden support, and 18 native-API-only
  service operations are now first-class not-applicable verdicts carrying
  their SM citation and native-test evidence.

### Added

- ADL 2 archetype validation now enforces VETDF (external term-binding
  validity): a term bound to an external terminology (SNOMED CT, LOINC, …)
  that the configured terminology service reports as absent is rejected
  `422` with the `VETDF` rule code. Bindings the service cannot verify (no
  external provider configured, an unknown terminology, or a transport
  fault) are not raised, per the spec's "subject to tool accessibility"
  carve-out; archetype-internal (`local`/`openehr`) bindings are unaffected
  (covered by VTTBK/VTCBK key validity).
- ISO 8601 temporal ordering on the openEHR BASE time types
  (`Iso8601_date`/`_time`/`_date_time`/`_duration`): comparison with honest
  incomparability (partial-date range semantics, UTC normalization for
  zoned values, duration ordering via the spec's own `to_seconds`
  reduction with the `Time_definitions` average constants). ADL 2
  archetype validation now enforces assumed-value interval containment for
  temporal constraint types (previously undecidable and skipped); an
  incomparable pair never raises a violation.

## [3.4.0] - 2026-07-20

### Changed

- The implemented openEHR REST API is **ITS-REST Release-1.1.0** (published
  upstream 19-Jul-2026). The server was already built against the
  pre-release text of this release — the regenerated REST contract is
  byte-identical at the release tag — so wire behaviour is unchanged; the
  advertised API identity moves from 1.0.3/development to 1.1.0 everywhere
  (documentation, OpenAPI metadata, conformance artifacts), and the
  `openehr-its` spec crate is now versioned 1.1.0. Conformance reports
  state the tested edition as `release-1.1.0` (formerly `development`;
  the old label remains accepted as a CLI/config alias).

## [3.3.0] - 2026-07-20

### Added
- **ADL2 templates are now compiled and validated by the full ADL2 engine.**
  `POST /definition/template/adl2` runs the complete `openehr-adl` pipeline —
  parse, then the AOM2 validity catalogue (phase 1 basic integrity, reference-
  model conformance, and specialisation conformance against an already-loaded
  parent) — in place of the former source-subset probe. An invalid artefact is
  a **422** whose `Error.validationErrors` list the offending rule-code
  mnemonics (S-codes for an unparseable source, V-codes for a validation-phase
  failure). `GET /definition/template/adl2/{template_id}` now serves the
  `application/json` `OperationalTemplateV2` projection alongside the
  `text/plain` source, and resolves a partial `template_id` to the latest
  matching version; the previously `501` `…/{template_id}/{version}` (versioned
  get, marked deprecated in the spec) is implemented, and template list rows now
  carry `concept` and `archetype_id`. `GET …/{template_id}/example` now generates
  an example COMPOSITION from the compiled operational template (an ADL2 →
  Web Template front end feeding the shared example generator), served across the
  four `Accept_LOCATABLE` representations (canonical JSON/XML, `openehr.wt.flat`,
  `openehr.wt.structured`) with `type` (`input`/`output`) + `detail_level`
  (`required`/`medium`/`complete`) query parameters, and `400`/`404`/`406` exactly
  as the ADL 1.4 example endpoint. An `Accept` naming only `application/xml` on
  the plain template GET is a `406` (the operation declares no XML response body).
- **ADL 1.4 archetypes are now validated by the ADL 1.4 engine, and can be
  migrated to ADL 2.** An ADL 1.4 source archetype (the `I_DEFINITION_ADL14`
  archetype surface) is now parsed and validated **as ADL 1.4** by the
  `openehr-adl` engine — the subset of the phase-1 catalogue that corresponds to
  the ADL 1.4 / AOM 1.4 standalone validity rules (VARID, VARDT, VARCN, VATID,
  VDSEV/VDSIV, …), replacing the former structural probe. An invalid source is a
  **422** naming the offending rule-code mnemonic. A new service capability
  migrates a stored ADL 1.4 archetype to ADL 2 source (`adl14_convert_to_adl2`);
  no openEHR spec governs 1.4 → 2 conversion (our own design/extension) and the
  ITS-REST contract declares no conversion operation, so it is a library
  capability with no REST endpoint. The ADL 1.4 operational-template (OPT) REST
  surface (`/definition/template/adl1.4`) is unchanged.
- **RM terminology-backed invariant validation.** Composition (and any RM
  value) validation now enforces the openEHR terminology-service and code-set
  RM class invariants at the wire-boundary dispatcher, unified into a single
  hook (`openehr-its`) that every validation consumer inherits. The 30 wired
  invariants (each audited clean against the whole corpus before enforcement):
  `COMPOSITION` category/language/territory, `EVENT_CONTEXT` setting,
  `ELEMENT` null-flavour, `ISM_TRANSITION` current-state/transition,
  `PARTICIPATION` + `EXTRACT_PARTICIPATION` function/mode, `INTERVAL_EVENT`
  math-function, `TERM_MAPPING` purpose, `AUDIT_DETAILS` change-type,
  `ATTESTATION` reason, `PARTY_RELATED` relationship, `VERSION`
  lifecycle-state, `ENTRY`/`DV_TEXT` language + encoding, `DV_MULTIMEDIA`
  media-type/charset/language/compression/integrity algorithms, `DV_PARSABLE`
  charset/language, `DV_ORDERED` normal-status, and the `AUTHORED_RESOURCE` /
  `RESOURCE_DESCRIPTION_ITEM` / `TRANSLATION_DETAILS` original-language. An
  out-of-vocabulary openEHR code is a `422` naming the violated RM invariant;
  HTTP status codes are unchanged.

- Admin console: the Directory tab is now a complete directory experience —
  a structured folder-tree editor (add/rename/remove sub-folders, attach and
  remove composition item references with a picker), version history with
  read-only views and one-click restore, a `version_at_time` time-travel
  control, a sub-folder `path` query, and directory deletion with
  confirmation — on top of the existing create-from-template flow (raw JSON
  editing stays available as an advanced mode).

### Changed
- **RM validation invariant messages now carry the spec's (BMM) invariant
  names.** Three class-invariant violation messages were reconciled from their
  inherited archie spellings to the openEHR BMM invariant names, so a `422`
  validation payload reporting one of them changes text: `Accuracy_valid` →
  `Accuracy_validity` (DV_AMOUNT and its descendants — DV_QUANTITY, DV_COUNT,
  DV_DURATION, DV_PROPORTION), `Is_archetypeRoot` → `Is_archetype_root` (the
  ENTRY subtypes — OBSERVATION, EVALUATION, INSTRUCTION, ACTION, ADMIN_ENTRY),
  and `Location_validity` → `location_valid` (EVENT_CONTEXT). The check logic
  and HTTP status codes are unchanged; only the invariant name inside the
  `Invariant <name> failed on type <TYPE>` message differs.

- **Canonical-JSON codec cutover.** The openEHR spec types are now
  (de)serialized to/from canonical JSON entirely by a native emitted
  `ToJson`/`FromJson` codec in `openehr-its` — the spec types (`openehr-base`,
  `openehr-rm`, `openehr-am`, `openehr-term`, `openehr-lang`) no longer carry a
  serde derive, and the `openehr-derive` proc-macro crate is removed. The wire
  bytes are unchanged (proven by the R0 determinism manifest + the byte-hazard
  gates); the only externally visible difference is the **error-message shape on
  a malformed JSON request body** — the codec's parser reports `expected … at
  line N column M` / `missing field … on …` diagnostics instead of the previous
  serde phrasing (the HTTP status codes are unchanged: still `400`/`422`). A
  present-but-`null` array field is now rejected as a type error (was silently
  treated as an empty array), matching the strict tolerance contract.

- The served OpenAPI document now describes the COMPLETE wire for every
  operation (162 declarations across all API groups): every path/query
  parameter, request headers (`Prefer` incl. `return=identifier`, required
  `If-Match` forms, the committal headers), every reachable status code
  with its exact trigger, and the load-bearing response headers (weak
  `ETag`, `Location`, `Last-Modified`) — audited operation-by-operation
  against the vendored ITS-REST specification (both the operation
  definitions and the normative overview rules). A structural completeness
  test now gates the document.
- A disabled Admin API now answers `405 Method Not Allowed` (the status the
  ITS-REST specification declares for a disabled admin operation) instead
  of `404`.
- COMPOSITION and EHR_STATUS tag updates now honour the `Prefer` header as
  the specification defines: the default (`return=minimal`) returns
  `204 No Content`; `return=representation` returns `200` with the stored
  tag list. Previously the stored list was always returned with `200`.
- Demographic responses now carry `Last-Modified` (from the version's
  commit time) alongside the weak `ETag`; PARTY_RELATIONSHIP create/update
  honour `Prefer: return=identifier`.

### Fixed
- **Template example generation now produces fully-valid compositions.**
  `GET /definition/template/adl1.4/{template_id}/example` populated only a
  skeleton for many templates (issue #94) and could emit out-of-range or
  wrongly-typed values. The generator now synthesizes spec-valid values for
  every constrained field — quantities inside their magnitude ranges (with
  dimensionless empty units preserved), proportions satisfying their kind's
  invariants inside the archetype's numerator/denominator ranges, durations
  inside their declared range, coded text from closed value lists, URIs and
  parsables honouring their pattern constraints, and the archetype-constrained
  container/event types (`ITEM_LIST`/`ITEM_SINGLE`/`INTERVAL_EVENT`) instead
  of abstract defaults — and every generated example at the committable detail
  levels (`required`, `medium`) passes the server's own full composition
  validation. Generation is byte-deterministic.
- **Archetype-conformance validation no longer demands `archetype_node_id` on
  reference-model types that cannot carry one.** `EVENT_CONTEXT` (and any
  other non-`LOCATABLE` type) inherits `PATHABLE`, which the RM gives no
  `archetype_node_id`; a template archetyping `/context[at…]` therefore could
  never be satisfied by canonical data and such compositions were wrongly
  rejected on commit. Non-`LOCATABLE` nodes now match structurally by their
  attribute position (per the RM inheritance graph); `LOCATABLE` nodes keep
  strict node-id matching.

- Admin console: text typed into the EHR finder and create-EHR fields before
  the app finished loading is no longer silently wiped (the inputs are now
  hydration-safe, like the login form); success toasts no longer intercept
  clicks on buttons beneath them in the e2e battery.
- `GET /ehr/{ehr_id}/directory/{version_uid}` now honours the `path` query
  parameter (slash-separated FOLDER names selecting a sub-folder subtree),
  as the ITS-REST `directory_get_by_version_id` operation specifies; an
  unresolved path returns 404. Previously the parameter was accepted but
  ignored and the full tree was always returned.
- The served OpenAPI now documents the full DIRECTORY wire contract
  (`version_at_time`/`path` parameters, `Prefer` including
  `return=identifier`, `If-Match`, and the complete status ladders
  including 204/400/409/412).

## [3.2.0] - 2026-07-18

### Added
- **`GET {base}/admin/config` — the redacted effective configuration** (an
  ehrbase-rs extension; the openEHR admin API defines only EHR deletes).
  Returns the merged effective configuration (file + `EHRBASE_*` env +
  `--set` overrides) as a JSON tree with every secret-bearing value redacted
  structurally by its secret type — passwords, password hashes, HMAC/signing
  secrets, and S3 secret keys render as `***`, and connection URLs (database,
  AMQP) mask their embedded credentials while keeping host and path; non-secret
  identifiers (usernames, roles, OIDC issuer) stay visible. Shares the admin
  gate and authorization of the admin deletes (`EHRBASE__ADMIN__ENABLED=true`,
  `ADMIN` role); disabled admin API answers `404`.
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
  port 3000. The sign-in page is served fully rendered and works with
  JavaScript disabled (the login form posts and redirects natively), and
  offers exactly the methods that can work: the console's configured login
  modes intersected with the authentication schemes the CDR advertises in
  its `WWW-Authenticate` challenge. The console received a full design
  system (semantic design tokens with lockstep light/dark theming, a teal
  brand shared by the widget kit, iconified navigation, breadcrumbed page
  headers, named table headers, empty states, and toast feedback on every
  mutation) and the complete working feature set: query result **export**
  (CSV/JSON, a plain form download that works without WebAssembly),
  **EHR creation** (empty or subject-bound) and **find-by-subject-id**,
  **composition commit** (canonical JSON/XML/FLAT with verbatim CDR
  validation diagnostics) and **edit-as-new-version** (`If-Match`
  concurrency), stored-query **open-in-editor**, shareable URL-driven tab
  state on the detail screens, a template identity card (version,
  languages, UID, archetype id), an **EHRs (cohort)** query shape
  (`SELECT DISTINCT` over the criteria tree), a **Table | Chart** toggle
  on numeric result columns, a version **timeline strip** with a
  `version_at_time` picker on the composition viewer, and a
  **contributions table** on the EHR detail screen. The Directory tab can
  now **create and edit the EHR folder directory** (spec-standard
  POST/PUT with `If-Match`), starting from console-local **folder
  templates** (two built-ins included); the System panel gained
  **repository usage** (per-template composition counts) and a read-only
  **runtime configuration** view backed by the CDR's new redacted
  `GET /admin/config` endpoint (secrets redacted structurally by their
  types — never by key matching). The E2E harness gained an image mode
  (`UI_E2E_IMAGE=1`) that runs the identical journey battery against the
  composed OCI image — including a genuinely end-to-end OIDC journey: the
  quickstart Keycloak now pins one canonical issuer and the dev CDR config
  trusts it via standard OIDC discovery, so a bearer-authenticated console
  session queries the CDR for real. Verified by a Rust-native browser E2E
  journey suite (merge-gating in CI, screenshots published as artifacts),
  including journeys over seeded clinical data and a JavaScript-disabled
  login journey.
- **`GET /ehr/{ehr_id}/contribution` — a paged contribution list** (an
  ehrbase-rs extension; the openEHR REST API defines only the by-uid read).
  Returns the EHR's contributions newest-first as
  `{ "rows": [ { uid, time_committed, committer, change_type } ], "total" }`,
  paginated with `offset` (default 0) and `fetch` (default 20, capped at
  100); **404** for an unknown EHR. Authenticated like the other EHR reads.
- **`DELETE /admin/template/{template_id}` and
  `DELETE /admin/query/{qualified_query_name}/{version}`** — admin deletes for
  operational templates and stored-query versions (ehrbase-rs extensions; the
  openEHR admin API defines only EHR deletes). Same admin gate and
  authorization as the EHR deletes: **204** on success, **404** for an unknown
  id. The template delete additionally returns **409** when a committed
  version still references the template, so a physical delete never orphans
  clinical data.

- **ATNA audit — richer DICOM records**: every audit record now carries the
  concrete operation as a DICOM `EventTypeCode` (login/logout as DCM
  110122/110123; REST operations as their ITS-REST operation id under the
  `openEHR-ITS-REST` code system), and Bearer-authenticated requests record
  the token's `jti` as the minimal token identity (token contents are never
  logged).
- **ATNA audit — FHIR R4 `AuditEvent` rendering (IHE BALP)**: every audit
  record also renders as a FHIR R4 `AuditEvent` conforming to the IHE Basic
  Audit Log Patterns (Patient\*/plain Create/Read/Update/Delete/Query
  profiles, `OAUTHaccessTokenUse.Minimal` token agent, profile claims only
  when genuinely satisfied) — the modern half of the dual ATNA format.
- **ATNA audit — local Audit Record Repository, on by default**: audit
  records are persisted in a new PostgreSQL `audit` schema (append-only;
  strictly outside the EHR content; per-sink delivery stamps; configurable
  `retention_days` with an hourly reaper). Every deployment now gets a
  queryable audit trail out of the box with nothing leaving the node.
- **ATNA audit — RESTful ATNA forwarding (ITI-20 ATX:FHIR Feed)**: opt-in
  `[audit.fhir_feed]` sink POSTs each FHIR `AuditEvent` to an external Audit
  Record Repository; with the local store on, delivery is outbox-driven — an
  ARR outage loses nothing and pending records ship on recovery.
- **ATNA audit — per-sink metrics** (`atna_audit_sent_total{sink=…}`,
  `…send_failed_total{sink=…}`, `atna_audit_rejected_total`,
  `atna_audit_reaped_total`).
- **ITI-81 Retrieve ATNA Audit Event** (`GET /fhir/r4/AuditEvent`): the
  official RESTful-ATNA retrieval — a FHIR search over the local Audit
  Record Repository returning a `searchset` Bundle of the stored `AuditEvent`
  documents. Filters: `date` (`ge`/`le`), `patient`, `agent`, `entity`,
  `outcome`, `action`, plus `_count`/`_offset` paging. Admin-only under
  RBAC; `404` when the local store is disabled.
- **Native TLS + mutual-TLS client authentication** (`[server.tls]`): the
  main listener can terminate TLS itself (TLS 1.2+ floor per IETF BCP 195)
  and demand a verified client certificate
  (`client_auth = "off" | "optional" | "required"`) against an explicit CA —
  the IHE ATNA ITI-19 node-authentication posture. The management listener
  stays plain HTTP.
- A dedicated **Audit trail (IHE ATNA)** book chapter covering the dual
  formats, the sinks, the ITI-81 retrieval, fail-mode semantics, and mTLS.
- **Admin console — the Audit log screen** (`/audit`): browse the CDR's
  ATNA security audit trail through the standard ITI-81 retrieval, with
  URL-driven filters (event-time window, patient, principal, outcome,
  action), pagination, and a per-row view of the full stored FHIR
  `AuditEvent`. Admin-only under RBAC; a disabled local audit store and a
  no-matches filter each render their own first-class state.

### Changed
- The ITS-REST template list (`GET /definition/template/adl1.4`) now reports
  the optional `version` field of each `TemplateMetadata`, derived from the
  template id's version axis (the spec documents the value as "taken from
  `template_id`"); it is omitted when the id carries no version.
- **Audit configuration redesigned: `[atna]` is now `[audit]`**, on by
  default with only the local store active, and sink-structured:
  `[audit.store]` (local repository), `[audit.syslog]` (classic
  DICOM-over-syslog feed; keys `host`/`port`/`transport`/`tls_ca_file`/
  `tls_identity_cert_file`/`tls_identity_key_file` replace the old
  `repository_host`/`repository_port`/`tls_*_path`), `[audit.fhir_feed]`
  (RESTful ATNA). `resolve_subject` now defaults to `true`. A configuration
  still using `[atna]` fails at boot with did-you-mean guidance (strict
  loader; no silent aliasing).
- **Fail-closed auditing got stronger**: with `fail_mode = "closed"` and the
  local store enabled, a store that stops accepting writes makes every
  subsequent auditable operation answer `503 Service Unavailable` until a
  write succeeds again — no un-audited PHI access.

### Fixed
- **ATNA audit — IHE/DICOM conformance corrections** (IHE ITI TF-2 ITI-20 /
  DICOM PS3.15 §A.5.1): the syslog `MSGID` is now the mandated
  `IHE+RFC-3881` (was `IHE+DICOM`); AQL query execution uses the dedicated
  DICOM EventID 110112 "Query" (was 110110); EHR-Extract communication uses
  the direction-coded EventIDs 110106 "Export" / 110107 "Import";
  authentication events (genuine logins and rejected 401/403 attempts) use
  EventID 110114 "User Authentication" with `EventTypeCode` 110122 "Login"
  (were generic Application Activity); and 1xx/3xx responses (e.g. `304 Not
  Modified`) are now recorded as success instead of minor failure.
- **Admin console — icon-only chrome and small polish**: every emoji and
  typographic glyph in the UI is replaced by a proper SVG icon (folder tree,
  status capability badges, remove buttons, disclosure carets, upload
  trigger, pagination arrows); the Audit log screen highlights its own
  navigation entry; and the documentation screenshots now cover every EHR
  detail tab — including the directory tab both before (create from a folder
  template) and after the directory exists — plus the audit raw-record view.

## [3.1.1] - 2026-07-17

### Fixed
- The release pipeline attaches the per-architecture server binary tarballs
  again: since the crate consolidation the binary is produced by the
  `ehrbase-server` package (the executable is still named `ehrbase`), but
  the release asset build still compiled the `ehrbase` platform library and
  failed — v3.1.0 published without binary assets. Container images were
  not affected. Use v3.1.1 for downloadable binaries.

## [3.1.0] - 2026-07-17

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
  `ehrbase.access_control.v1` scheme:
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


[unreleased]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.14.0...HEAD
[3.14.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.13.0...v3.14.0
[3.13.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.12.0...v3.13.0
[3.12.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.11.0...v3.12.0
[3.11.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.10.0...v3.11.0
[3.10.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.9.0...v3.10.0
[3.9.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.8.0...v3.9.0
[3.8.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.7.0...v3.8.0
[3.7.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.6.0...v3.7.0
[3.6.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.5.0...v3.6.0
[3.5.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.4.0...v3.5.0
[3.4.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.3.0...v3.4.0
[3.3.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.2.0...v3.3.0
[3.2.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.1.1...v3.2.0
[3.1.1]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.1.0...v3.1.1
[3.1.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.3...v3.1.0
[3.0.3]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.2...v3.0.3
[3.0.2]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.1...v3.0.2
[3.0.1]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.0...v3.0.1
[3.0.0]: https://github.com/rubentalstra/ehrbase-rs/releases/tag/v3.0.0
