# Admin & messaging APIs

Two groups of routes sit beside the clinical API for operators rather than for
clinical clients: the **admin API**, which physically deletes, reports on, and
moves repository content, and the **messaging API**, which exports and imports
whole records as openEHR EHR Extracts and accepts documents in the
template-data (TDD) form. This page is the operator's reference for both —
every path, what gates it, what it accepts, and every status code it answers.

<!-- toc -->

## Where these routes live, and what gates them

Both groups are mounted under the API base path, so every path below is
relative to it. With the default `server.base_path`, `{base}` reads
`/ferroehr/rest/openehr/v1`, and `{base}/admin/dump` is
`/ferroehr/rest/openehr/v1/admin/dump`.

The two groups are gated differently, and the difference matters
operationally:

| Group | Switch | Authorization class | While switched off |
|---|---|---|---|
| `{base}/admin/…` | `admin.enabled` (`FERROEHR__ADMIN__ENABLED`), default `false` | admin — the configured `authz.rbac.admin_role` (`ADMIN` by default) | every route answers **405 Method Not Allowed** with an **empty** `Allow` header: the resource exists but currently serves no method |
| `{base}/message/…` | none — always mounted | ordinary clinical, exactly like the composition API | n/a |

Two consequences worth planning around:

- **The messaging routes are not admin-gated.** They read and write the same
  clinical content the composition API does, so they carry the same ordinary
  authentication and the same coarse clinical class. A principal holding the
  configured read-only role (`authz.rbac.readonly_role`, `READONLY` by
  default) is refused **403** on every messaging *import*, before the body is
  read; the exports stay available to it.
- **RBAC has to be on for the admin role to mean anything.** With
  `authz.rbac.enabled = false` — the posture the Compose quickstart ships —
  authentication is the only gate, so *any* authenticated caller reaches every
  enabled admin route. Turn RBAC on before enabling the admin API anywhere
  that is not a laptop.

While the admin API is enabled, the server also advertises `/admin` in the
`OPTIONS` conformance manifest it serves at the API base-path root; with the
API off, that entry is absent.

> [!WARNING]
> `DELETE {base}/admin/ehr/all` with no parameter empties the repository. There
> is no confirmation step and no undo. Keep `admin.enabled` off unless a
> workflow needs it, and gate the admin role tightly.

## Physical deletion

Normal openEHR deletes are *logical* — the version history is retained. The
admin API is the exception: **physical, irreversible** removal, for legal
erasure requests and test-data cleanup.

| Route | Success | Refusals |
|---|---|---|
| `DELETE {base}/admin/ehr/{ehr_id}` | **204** — the EHR and every resource it owns (compositions, `EHR_STATUS`, item tags, contributions, and all their historical versions) are physically gone | **400** malformed id (rejected before any deletion), **404** no EHR with that id |
| `DELETE {base}/admin/ehr/all` | **204** — bulk delete | **400** any id in the list is not a well-formed UUID; the whole request is refused before anything is deleted |
| `DELETE {base}/admin/template/{template_id}` | **204** | **404** unknown template, **409** a committed version still references it |
| `DELETE {base}/admin/query/{qualified_query_name}/{version}` | **204** | **404** unknown name, or a known name with no such version |
| `GET {base}/admin/config` | **200** — the effective configuration as a redacted JSON tree | — |

Every route additionally answers **401** unauthenticated, **403** for an
authenticated caller outside the admin class, and **405** while the group is
switched off.

Details that decide behaviour:

- **The bulk delete's parameter is optional, and its absence means
  everything.** `DELETE {base}/admin/ehr/all` with no `ehr_id` deletes every
  EHR on the server. To delete a subset, pass `?ehr_id=<uuid>` — repeatable
  (`?ehr_id=a&ehr_id=b`) or comma-separated (`?ehr_id=a,b`); blank entries are
  dropped. An id that names no EHR deletes nothing and is not an error, so the
  bulk route has no `404`.
- **Deleting a template never orphans clinical data.** The **409** is a
  deliberate guard: while any stored composition was committed against the
  template, the delete is refused and the message says how many committed
  versions still hold the reference. Delete those compositions first.
- **A stored-query delete removes exactly one `(name, version)` row.** The
  query's other versions survive.
- **The config view is redacted structurally, not by key name.** Passwords and
  password hashes, HMAC and signing-key secrets and S3 secret keys render as
  `***`; connection URLs (database, AMQP) keep their host and path and mask
  the embedded credentials (`postgres://***@host:5432/db`). Non-secret
  identifiers — usernames, roles, an OIDC issuer — stay visible. Redaction is
  a property of the configuration's secret types, so no secret value can reach
  this response.

> [!NOTE]
> The template delete, the stored-query delete and the config view are FerroEHR
> extensions: the openEHR admin API defines only the two EHR deletes. They
> share the same switch and the same authorization as those deletes.

## The activity report

Four **read-only** counters over the repository's change history, behind the
same switch and role as the deletes.

Every route takes two query parameters:

| Parameter | Required | Meaning |
|---|---|---|
| `a_service` | yes | the service whose versioned content to report on — one of `Admin`, `Definitions`, `Ehr`, `Ehr_index`, `Demographic`, `Message`, `Query`, `System_log`, matched case-insensitively |
| `time_interval` | no | `<lower>/<upper>`, two ISO 8601 date-times matched **inclusively** against each commit time |

Either bound may be left empty for an open interval
(`?time_interval=2026-01-01T00:00:00Z/`); an absent parameter reports over all
time. A service that holds no versioned content reports an empty list or `0`
rather than failing.

| Route | **200** body |
|---|---|
| `GET {base}/admin/report/contribution` | the matching CONTRIBUTION ids as a JSON array, ordered by commit time then id |
| `GET {base}/admin/report/contribution/count` | how many there are, as a bare JSON number |
| `GET {base}/admin/report/versioned_composition/count` | how many distinct COMPOSITION version containers had a version committed in the interval |
| `GET {base}/admin/report/composition_version/count` | how many individual COMPOSITION versions were committed in the interval |

**400** covers an absent `a_service`, one that names no known service, a
`time_interval` that is not `<lower>/<upper>`, a bound that is not a valid ISO
8601 date-time, and an interval bounded on **both** sides whose lower bound is
after its upper bound — that is not an interval, and answering it with the
empty result it would select would hand back a truthful-looking count for a
window nobody asked for. Equal bounds are a legitimate single-instant interval
and are reported normally.

## Archiving

Four routes that move a selected set of records to the server's cold storage
tier and back, behind the same switch and role. Archiving is **not** a delete:
the archived records stay fully readable through the normal API, with their
whole revision history intact.

| Route | Body | Success |
|---|---|---|
| `POST {base}/admin/archive/ehrs` | `{"ehr_ids": ["…"]}` | **204** — every named EHR and all its versioned content is archived |
| `POST {base}/admin/archive/parties` | `{"party_ids": ["…"]}` | **204** — every named demographic party is archived |
| `POST {base}/admin/archive/ehrs/restore` | `{"ehr_ids": ["…"]}` | **204** — every named EHR's archived content is back in the primary tier |
| `POST {base}/admin/archive/parties/restore` | `{"party_ids": ["…"]}` | **204** — every named party's archived content is back in the primary tier |

All four are **all-or-nothing and idempotent**: a body of the wrong shape or a
malformed id is **400** and an id that names nothing is **404**, in both cases
before anything is moved; re-archiving an already-archived record, or restoring
one that is not archived, changes nothing. An empty list succeeds and moves
nothing. A body sent without `Content-Type: application/json` is **415**. A
party that is currently archived is still found by its restore call — the
existence check spans both tiers — so an archived party is never reported
missing.

A party's `PARTY_RELATIONSHIP`s are **not** carried along — each is an
independently addressable versioned object, archived in its own right.

What "cold storage tier" means here: the archived rows are physically moved out
of the primary tables into a separate schema in the same database, so the
tables and indexes that serve everyday traffic shrink by exactly what was
archived — no extra tablespace, volume, or external service to operate. Reads
of unarchived records never touch the cold tier; a read that addresses an
archived record is served from it. Writing to an archived record brings it
back to the primary tier first, so a versioned object is never split across
tiers; a physical delete clears both tiers; and `{base}/admin/dump` still
exports archived content.

Two consequences to plan for:

- **AQL queries see the primary tier only**, so an archived record stops
  appearing in query results — that is exactly what shedding the query tables'
  rows and indexes buys you. Everything addressed by id (an EHR, a
  composition, a folder, a party, a version, a revision history) keeps working
  as before. Restoring puts the record back in query results.
- **There are three ways back, and only one of them is deliberate.** The
  `…/restore` routes above reverse a whole set on request; a write to an
  archived record thaws just that record as a side effect, whichever route the
  write came in on; and a physical delete removes it from both tiers. Query
  visibility is the effect to plan around, and the restore routes are how you
  get it back.

## Dump and load

Two routes that move the whole repository to and from an archive on the
**server's** file system, behind the same switch and role. Both answer **200**
with a JSON array of per-entity failure reports — an **empty** array means
everything succeeded.

### `POST {base}/admin/dump`

Writes an archive of every EHR **and of every standalone demographic
container** — the parties and party relationships that live outside any EHR.
The body is `{"file_sys_loc": "…"}` plus the optional export settings:

| Field | Values | Default |
|---|---|---|
| `logical_format` | `openehr_canonical_json` or `openehr_canonical_xml` | canonical JSON |
| `compression_format` | `zip` or `7z` — omit for loose files | uncompressed |
| `segment_split_size` | segment size in kb (a positive integer) | `1024` |

`logical_format` chooses how the clinical **content** is serialized, not how
the archive is packaged:

- `openehr_canonical_json` (the default) keeps each version's content inline
  in the segment files, exactly as this server stores it.
- `openehr_canonical_xml` writes each version to its own
  `versions/<version_uid>.xml` entry instead — a complete `ORIGINAL_VERSION`
  document under the openEHR-published `<version>` root, ready to hand to any
  tool that reads canonical openEHR XML.

The archive is a directory holding a `manifest.json`, one or more
`segment-NNNN.json` files, and a `blobs/` subdirectory for any externalized
multimedia. When the repository holds standalone demographic containers, the
archive additionally carries a `demographic-commons.json` (their shared
audits and contributions) and one or more `demographic-NNNN.json` segments —
one record per party or relationship, in the same version-record shape the
EHR segments use. With `compression_format` set, those same entries are
packed into a single `archive.zip` or `archive.7z` inside the location
instead. The
archive's own bookkeeping — the manifest and the segment skeleton — stays JSON
in both logical formats, because openEHR publishes no XML document form for
it.

### `POST {base}/admin/load`

Populates the repository from an archive. It takes the location and nothing
else: the container (loose files, a single `archive.zip`, or a single
`archive.7z`) is detected from what the location holds, and the logical format
is read from the archive's own manifest — so a load never has to be told how
the dump was written. Archives written before the demographic wave existed
simply carry no demographic entries and load unchanged.

The repository being loaded into **need not be empty**. An EHR whose id is
already present is reported and skipped rather than failing the load, so the
response array names each one — and a standalone demographic container that
already exists is reported the same way, under its own kind (`PERSON`,
`ORGANISATION`, `GROUP`, `AGENT`, `ROLE`, or `PARTY_RELATIONSHIP`) as the
`entity_type`:

```json
[ { "entity_type": "EHR",
    "entity_id": "7d44b88c-4199-4bad-97dc-d78268e01398",
    "dump_status": false,
    "error": "an EHR with this id already exists" } ]
```

Both directions are lossless: a dump and a load reproduce every record,
whichever format and container you choose.

### Refusals

| Status | When |
|---|---|
| **400** | a missing or blank `file_sys_loc`; a format value outside the lists above; a non-positive `segment_split_size`; an `encoding` field (the openEHR service model declares that enumeration with no members, so no value a client could send names one); on load, an archive carrying externalized multimedia this server has no store for |
| **415** | the request `Content-Type` is not `application/json` |
| **500** | a location that holds **no** archive, and one holding an archive that is corrupt — a mangled or truncated container, manifest, or segment — are the same fact and answer the same way: the service model's single `file_not_writable` error for these operations. Nothing is loaded either way. On dump, the same status covers a location, segment, payload entry or manifest that could not be created or written |

A single unreadable `versions/*.xml` entry is *not* in that family: it belongs
to one EHR, so that EHR is reported in the response array and skipped whole
while the rest of the archive loads.

> [!NOTE]
> The activity report, the archive routes and the dump/load pair are FerroEHR
> extensions: the openEHR *service model* defines these operations, but the
> released REST API surfaces no endpoint for them, so their URLs are our own.
> The two `…/restore` routes go one step further — the service model declares
> the archive calls and no un-archive counterpart, so both the operation and its
> URL are ours. They gate no openEHR conformance claim; see
> [Conformance](conformance.md).

## EHR Extract and TDD import

Six routes under `{base}/message` that move whole records between systems and
accept documents in the template-data form. Unlike the admin extensions above,
these are **not** admin-gated — see
[the gate table](#where-these-routes-live-and-what-gates-them).

### EHR Extract

- `GET {base}/message/export/{ehr_id}` — export one whole EHR. **200** with a
  JSON array holding one `EXTRACT` that carries every versioned object of the
  EHR, latest versions only. **400** if `ehr_id` is not a well-formed
  identifier, which is refused before any lookup; **404** if the EHR does not
  exist; **406** if `Accept` cannot be satisfied (the extract list is JSON
  only).
- `POST {base}/message/export` with an `EXTRACT_SPEC` body — export by
  specification. **200** with one `EXTRACT` per manifest entity, in manifest
  order. The manifest must name at least one entity, and each entity must
  name its record by `ehr_id` or `subject_id`, otherwise **400**; an
  identifier that names nothing is **404**. `extract_type` must be one of the
  extract-content-type codes the openEHR Reference Model names —
  `openehr-ehr`, `openehr-demographic`, `openehr-synchronisation`,
  `openehr-generic`, `generic-emr` — or the catch-all `other`; anything else
  is **400**, as is a selection this service does not support (search
  criteria, an unsupported commit-time interval). This route is classified as
  a **read**: it selects over held versions and commits nothing, exactly like
  the ad-hoc AQL `POST`.
- `POST {base}/message/import` with an `EXTRACT` body — clone a whole EHR. Add
  `?ehr_id=<uuid>` to fix the identifier the clone lands under; leave it off
  and the source identifier the extract carries is re-used. **201** with
  `{"uid": "<ehr_id>"}`, so a caller that supplied no id still learns what was
  created. The extract must carry an `EHR_STATUS` (**400**), and the target
  must not already exist — **409**, which also covers an imported `EHR_STATUS`
  naming a subject another EHR already holds.
- `POST {base}/message/import/{ehr_id}` with an `EXTRACT` body — add the
  extract's content to an **existing** EHR as new versions. **204**. **404**
  if the EHR does not exist; **409** if the EHR already holds an `EHR_STATUS`
  or `EHR_ACCESS` under a different object id, or the imported status names
  another EHR's subject; **422** if a version in the extract is semantically
  invalid (template, RM-invariant or terminology validation).

All four accept either `application/json` or `application/xml` bodies; any
other `Content-Type` is **415**.

### TDD import

- `POST {base}/message/tdd/{ehr_id}` with an `application/xml` body — import
  one Template Data Document. It is converted against the operational template
  its root names and committed through the ordinary validated composition
  path, so **201** with `{"uid": "<version_uid>"}`. The template must already
  be uploaded through the definition API (**404** otherwise, as for an unknown
  EHR); a root that is not in the template-data namespace, carries no template
  id, or does not conform to the template is **400**; a document that is not
  well-formed XML — or a produced composition that fails validation at
  commit — is **422**. A body sent as anything but `application/xml` is
  **415**.
- `POST {base}/message/tdd/{ehr_id}/batch` with a JSON array of TDD documents
  — import several at once. **201** with the created version ids in input
  order. The batch is **all-or-nothing**: every document is converted before
  any is committed, so one bad document rejects the whole batch and commits
  nothing. An **empty** array is a fulfilled no-op — **200** with `[]`, since
  nothing was created — but the target EHR is checked for every batch, the
  empty one included, so an unknown one is **404** whatever the batch holds.
  The batch has no limit on how many documents it may carry; the only bound is
  the server-wide request-body limit, which answers **413** when exceeded.

> [!NOTE]
> The whole `{base}/message` group is a FerroEHR extension: the openEHR service
> model defines a Message component, but the released REST API publishes no
> message, extract, or TDD endpoint at all. These URLs are our own and gate no
> openEHR conformance claim; see [Conformance](conformance.md). For the
> workflow-level view — what an extract is good for and how to drive an
> import — see [EHR Extract & messaging](beyond-core/messaging.md).

## Other routes under `{base}/admin`

Three further families share the `/admin` path prefix but **not** the
`admin.enabled` switch — each has its own, and each answers **404** (not
`405`) while its own switch is off, because the group is simply not serving:

| Routes | Own switch | Documented in |
|---|---|---|
| `{base}/admin/tenant…` | `tenancy.enabled` | [Security & multi-tenancy](security.md) |
| `{base}/admin/event_subscription…` | `events.admin_api` | [Change events (AMQP)](beyond-core/amqp.md) |
| `{base}/admin/fhir_mapping…` | `fhir.api_enabled` | [FHIR connectors](beyond-core/fhir.md) |

They are still admin-class routes for authorization, so the same **401** /
**403** split applies.

For everything else an operator needs — migrations, health probes, the
management surface, observability and upgrades — see
[Operations](operations.md).
