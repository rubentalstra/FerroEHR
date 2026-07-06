# ATNA Audit Trail — Rust-native design

- **Status:** implementing (pulled forward from Stage 2 by owner decision, 2026-07-06)
- **Stage:** Stage 1 (owner-prioritized; originally Stage 2 — see `PORT_MASTER_PLAN.md` §11)
- **Date:** 2026-07-05 · **Amended:** 2026-07-06 (§8 implementation binding)
- **Owner:** —
- **Reference (prior art, not a port target):** EHRbase Enterprise Features → ATNA
  (`https://docs.ehrbase.org/docs/EHRbase/Enterprise-Features/ATNA`). EHRbase
  implements ATNA via the Java **IPF** (Open eHealth Integration Platform)
  library. **We do not port IPF**; we implement the same behaviour natively in
  Rust (ADR-006/008: prior art, not an oracle).

## 1. What ATNA is

**ATNA** (Audit Trail and Node Authentication) is the IHE profile for security
audit logging in healthcare systems. Its audit-record payload is the
**DICOM Audit Message** (DICOM PS3.15 §A.5, the successor to IETF RFC 3881),
an XML `AuditMessage` document. Records are shipped to an **Audit Record
Repository (ARR)** over **Syslog** — RFC 5424 framing, transported over UDP
(RFC 5426) or TLS (RFC 5425, the IHE-recommended secure transport).

For a CDR this means: every access to a clinical resource (EHR, composition,
contribution, directory, query, admin operation) emits one `AuditMessage`
describing *who* did *what* to *which* resource, with *what outcome*, from
*where*, and *when* — sent to a central repository (commonly Elastic Stack via
Logstash).

## 2. Scope — audited operations

Every operation below emits exactly one audit record on completion (success or
failure). This maps 1:1 onto our generated ITS-REST operation ids
(`openehr-its::rest::generated`), so the hook can be data-driven from a table
keyed by `operation_id`.

| Group | Operations |
|---|---|
| **Definition / stored query** | list stored queries, store a query, get stored query + metadata |
| **EHR** | create EHR, create EHR with id, get EHR by id, get EHR by subject id |
| **EHR_STATUS** | get at time, get by version id, update; versioned: get, revision history, version at time, version by id |
| **Composition** | create, update, delete, get by version id, get at time; versioned: get, revision history, version by id, version at time |
| **Directory** | create, update, delete, get folder at version, get folder at time |
| **Contribution** | create, get by id |
| **Query** | execute ad-hoc AQL, execute stored query |
| **Admin** | update EHR, delete EHR, delete composition, delete directory, delete query |

## 3. The DICOM Audit Message we emit

Target shape (from the EHRbase reference — an EHR-create success record):

```xml
<AuditMessage>
  <EventIdentification EventActionCode="C" EventDateTime="…Z" EventOutcomeIndicator="0">
    <EventID csd-code="110110" codeSystemName="DCM" originalText="Patient Record"/>
    <EventOutcomeDescription>Operation performed successfully</EventOutcomeDescription>
  </EventIdentification>
  <ActiveParticipant UserID="john doe" UserIsRequestor="true"
      NetworkAccessPointID="10.216.24.150" NetworkAccessPointTypeCode="2">
    <RoleIDCode csd-code="110153" codeSystemName="DCM" originalText="Source Role ID"/>
  </ActiveParticipant>
  <ActiveParticipant UserID="ehrbase" UserIsRequestor="false"
      NetworkAccessPointID="10.42.23.77" NetworkAccessPointTypeCode="2">
    <RoleIDCode csd-code="110152" codeSystemName="DCM" originalText="Destination Role ID"/>
  </ActiveParticipant>
  <AuditSourceIdentification AuditEnterpriseSiteID="…" AuditSourceID="ehrbase">
    <AuditSourceTypeCode csd-code="4" codeSystemName="DCM"
        originalText="Application Server Process or Thread"/>
  </AuditSourceIdentification>
  <ParticipantObjectIdentification ParticipantObjectID="…"
      ParticipantObjectTypeCode="1" ParticipantObjectTypeCodeRole="1"
      ParticipantObjectDataLifeCycle="1">
    <ParticipantObjectIDTypeCode csd-code="2" codeSystemName="RFC-3881"
        originalText="Patient Number"/>
  </ParticipantObjectIdentification>
</AuditMessage>
```

### Field mapping

| Field | Value |
|---|---|
| `EventActionCode` | `C` create · `R` read · `U` update · `D` delete · `E` execute (query) |
| `EventDateTime` | current time, ISO-8601 UTC (`jiff::Timestamp::now()`) |
| `EventOutcomeIndicator` | `0` success · `4` minor · `8` serious · `12` major failure — derived from the HTTP status of the response |
| `EventID` | EHR ops → `csd-code=110110`, `originalText="Patient Record"`; data ops → `originalText` ∈ {`composition`,`contribution`,`query`,`directory`}; app activity → `csd-code=110100`, `originalText="Application Activity"` |
| `AuditEnterpriseSiteID` | configured enterprise site id (CDR/tenant id) |
| `ActiveParticipant.UserID` (requestor) | Basic → username; OAuth2 → `sub` claim. **Sourced from our `auth::Principal`** in the request extensions |
| `ActiveParticipant` (destination) | this server (`AuditSourceID`), its host/IP |
| `NetworkAccessPointID` | client IP (from `X-Forwarded-For`/peer addr); `NetworkAccessPointTypeCode=2` (IP) |
| `ParticipantObjectID` [EHR/EHR_STATUS] | `TypeCode=1`, `IDTypeCode.originalText=URI`, id = the EHR's patient subject id |
| `ParticipantObjectID` [composition/contribution/directory] | subject id of the parent EHR (`TypeCode=1`) and/or the object URI (`TypeCode=2`) |
| `ParticipantObjectID` [ad-hoc query] | `TypeCode=2`, `IDTypeCode.originalText="Search Criteria"`, id=`UNKNOWN` |
| `ParticipantObjectID` [stored query] | `TypeCode=2`, URI form, or the qualified query name as search criteria |
| missing mandatory element | the configured `value-if-missing` (default `UNKNOWN`) |

## 4. Rust-native architecture

No IPF, no JVM. Three small pieces, all in safe Rust:

1. **`AuditMessage` model + serializer** — plain structs mirroring the DICOM
   schema, serialized to canonical XML with `quick-xml` (the crate already in
   the workspace). This is *not* openEHR ITS-XML; it is the DICOM audit schema,
   so it lives in its own module, not `openehr-its`.
2. **Syslog client** — RFC 5424 message framing with the DICOM
   Audit-Trail-Message-Format profile, over:
   - **UDP** (RFC 5426) via `tokio::net::UdpSocket` — the default the reference
     Elastic/Logstash stack listens for on port 514;
   - **TLS** (RFC 5425) via `tokio-rustls` (the workspace TLS stack) for secure
     transport to the ARR.
   Framing is ~30 lines; there is no mature Rust IHE-ATNA crate, so we own it
   (dual-use audit-logging code, defensible).
3. **Emitter hook** — a `tower`/axum layer plus a service-layer seam:
   - The layer captures request metadata (client IP, matched `operation_id`,
     the `auth::Principal` from request extensions) and the response status.
   - On response, it looks up the operation in a **static audit table**
     (`operation_id → (EventActionCode, EventID, participant-object rule)`),
     resolves the participant object id from the response/handler context (e.g.
     the created EHR's subject id — surfaced by the service layer via a
     response extension), builds the `AuditMessage`, and hands it to a
     **buffered, non-blocking sender** (a bounded `tokio::mpsc` drained by a
     background task) so auditing never blocks or fails a request. Send failures
     are logged and metered, not propagated (fail-open on transport, per common
     ATNA deployment; a fail-closed mode is a config option).

Suggested layout (Stage 2): a dedicated **`ehrbase-audit`** crate (model +
syslog client + table), with the middleware wired in `ehrbase-rest` and the
participant-object resolution provided by the `ehrbase` service layer. Login
events (Basic auth) are audited as "Application Activity" unless suppressed.

## 5. Configuration (Rust-native)

`figment`, `EHRBASE_ATNA_`-prefixed, folded into `RestConfig`/a dedicated
`AuditConfig`. Names mirror EHRbase's behaviour; the env keys are ours.

| Setting | Env | Meaning | Example |
|---|---|---|---|
| `audit.enabled` | `EHRBASE_ATNA_ENABLED` | master switch | `true` |
| `audit.enterprise_site_id` | `EHRBASE_ATNA_ENTERPRISE_SITE_ID` | enterprise/tenant id | `1f332a66-…` |
| `audit.repository_host` | `EHRBASE_ATNA_REPOSITORY_HOST` | ARR host | `localhost` |
| `audit.repository_port` | `EHRBASE_ATNA_REPOSITORY_PORT` | ARR port | `514` |
| `audit.transport` | `EHRBASE_ATNA_TRANSPORT` | `udp` \| `tls` | `udp` |
| `audit.source_id` | `EHRBASE_ATNA_SOURCE_ID` | audit source id | `ehrbase` |
| `audit.value_if_missing` | `EHRBASE_ATNA_VALUE_IF_MISSING` | fill for empty mandatory fields | `UNKNOWN` |
| `audit.suppress_login_events` | `EHRBASE_ATNA_SUPPRESS_LOGIN_EVENTS` | skip auth/login events | `true` |
| `audit.fail_mode` | `EHRBASE_ATNA_FAIL_MODE` | `open` (log & continue) \| `closed` (reject on send failure) | `open` |

## 6. Testing

- **Unit:** build an `AuditMessage` for each operation class and assert the XML
  matches the DICOM mapping (an `insta` snapshot per event action code, with the
  reference EHR-create example above as a golden vector). Redact
  `EventDateTime`.
- **Integration:** a mock UDP/TLS syslog listener (a `tokio` socket in the test)
  asserts the framed RFC 5424 record arrives and parses; verify user id from
  both Basic and OAuth2 principals, outcome codes from success/failure statuses,
  and the participant-object rules per resource type.
- **Manual/e2e:** the reference Elastic Stack (Elasticsearch + Kibana +
  Logstash) via docker-compose (see the EHRbase reference) as an ARR to eyeball
  records end-to-end. Not part of CI.

## 7. Relationship to openEHR AUDIT_DETAILS

ATNA auditing is **orthogonal** to openEHR's own `AUDIT_DETAILS`/`CONTRIBUTION`
provenance (which we already write in the same transaction as every change,
per ADR-008). openEHR audit = *what the record says about its own authorship*;
ATNA audit = *security surveillance of API access*. Both coexist: a composition
create writes a `CONTRIBUTION` + `AUDIT_DETAILS` row (openEHR) **and** emits an
ATNA `AuditMessage` (security). This document covers only the latter.

## 8. Implementation binding (2026-07-06 — the decisions that govern the build)

Sections 1–7 are the behavioural spec; this section pins the design to the
codebase as it stands (post spec-audit: typed `ServiceResponse`/`ResourceMeta`
envelope, `auth::Principal` request extension, generated `ROUTES` dispatch
tables, promoted `ehr.subject_id`).

### 8.1 Crate layout

New workspace crate **`ehrbase-audit`** (application layer, hand-written,
`thiserror`), no dependency on any `ehrbase-*` crate (pure leaf: `quick-xml`,
`tokio`, `tokio-rustls`, `jiff`, `serde` for config):

```
crates/ehrbase-audit/src/
├── message.rs    # DICOM AuditMessage model (PS3.15 §A.5) + quick-xml serializer
├── codes.rs      # DCM / RFC-3881 code constants (EventID, RoleID, TypeCodes)
├── event.rs      # AuditEvent — the transport-agnostic input the server hands us
├── table.rs      # operation_id → (action, EventID, participant-object rule)
├── syslog.rs     # RFC 5424 SYSLOG-MSG assembly (+ RFC 5426 UDP / RFC 5425 TLS framing)
├── sender.rs     # bounded mpsc + background drain task; fail-open/closed; metrics
└── config.rs     # AuditConfig (figment-compatible serde struct)
```

`ehrbase-rest` gains one tower layer (`audit.rs`) + wiring; `ehrbase` (binary)
boots the sender from config. Dependency arrows stay downward:
`ehrbase-rest → ehrbase-audit` and `ehrbase → ehrbase-audit`.

### 8.2 The emitter path (request → AuditMessage)

1. The **dispatcher** already knows the generated operation id for every
   request — it inserts an `AuditOpId(&'static str)` extension (single line in
   the generic dispatch path; no per-handler code).
2. The **audit layer** (outermost, after auth) captures: client IP
   (`X-Forwarded-For` first hop, else peer addr), the `Principal` extension
   (Basic username / OAuth `sub`; absent → `UNKNOWN`), request time.
3. On response it reads: HTTP status → `EventOutcomeIndicator`
   (2xx→0, 4xx→4, 5xx→8; 403/401→4), the `AuditOpId`, and an optional
   **`AuditObject` response extension** — populated by the dispatch layer from
   the `ResourceMeta` it already holds (ehr_id, version uid → object URIs).
   No handler-specific audit code: the envelope already carries the metadata.
4. It builds a transport-agnostic `AuditEvent` and `try_send`s it to the
   bounded channel. **Zero DB work and zero blocking on the request path.**
5. The **drain task** (owns the socket) optionally enriches: when
   `audit.resolve_subject = true` and the event has an `ehr_id`, one indexed
   lookup of the promoted `ehr.subject_id` fills the Patient-Number
   participant object (this runs in the background task, never in-request;
   enrichment failure → `value_if_missing`). It then renders the
   `AuditMessage` XML, frames it (RFC 5424; octet-counting for TLS per
   RFC 5425), and sends.

### 8.3 The audit table

`table.rs` is **data-driven and total-coverage-guarded**: a static table keyed
by the generated operation ids (§2 mapping), plus a unit test that walks every
operation id in `openehr-its::rest::generated::ROUTES` and asserts each is
either mapped or in the explicit `UNAUDITED` allowlist (status/health/swagger
and other non-clinical surface). New generated operations then fail the build
until classified — the same guard pattern as the codegen drift checks.

### 8.4 Failure + lifecycle semantics

- Channel full or transport down: `fail_mode=open` (default) → drop + `tracing`
  warn + a `metrics` counter (`atna_audit_dropped_total`); `fail_mode=closed`
  → the audit layer returns **503** for auditable operations (spec-honest:
  the deployment demanded auditing it cannot deliver).
- Graceful shutdown: the sender drains the channel (bounded flush timeout)
  before the server exits — wired into the existing axum-server shutdown path.
- TLS: `tokio-rustls` client; ARR CA configurable (`audit.tls_ca_path`),
  client-cert mutual TLS (`audit.tls_identity_*`) supported — IHE expects
  mutually-authenticated nodes.

### 8.5 Test plan (binding)

Per §6, plus: the total-coverage table guard (§8.3); an in-process UDP
listener e2e over the real axum app (testcontainers not required) asserting
one framed record per audited request with correct action/outcome/user/object
for: EHR create (C), composition get (R), composition update (U), composition
delete (D), AQL execute (E), a 401 (outcome 4, principal UNKNOWN), and a
suppressed login event; TLS framing round-trip against an in-process rustls
listener with a test CA.
