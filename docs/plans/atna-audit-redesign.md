# ATNA audit trail — full redesign (I_SYSTEM_LOG rewrite)

*Plan file (lifecycle: DELETED in the PR that implements it; durable record =
`docs/PROGRESS.md` + `CHANGELOG.md` + the living reference docs). Authored
2026-07-18 from first-hand standards research + a full inventory of the
current implementation. Worklist row: **ATNA**.*

---

## 1. The mandate and the standards stack (research findings)

The only normative openEHR statement for the System Log component is one
line — "System Log | IHE ATNA-compliant system log"
(`docs/specs/openehr/SM/docs/openehr_platform/master02-overview.adoc:38`);
the `I_SYSTEM_LOG` interface (`SM/docs/UML/classes/i_system_log.adoc`) is an
empty stub. Everything else comes from the external IHE/DICOM/HL7 standards
that "IHE ATNA-compliant" pulls in.

**Finding 1 — DICOM is not the wrong standard; DICOM *is* the ATNA
standard.** IHE ITI TF-1 §9 (ATNA) and the ITI-20 Record Audit Event
transaction are normative:

> "Events are encoded in accordance with DICOM PS3.15 Annex A.5." — ITI TF-1 §9
> "The MSG field of the SYSLOG-MSG shall be present and shall be an XML
> structure using UTF-8 minimal length encoding following the DICOM
> PS3.15 A.5 format." — ITI TF-2 ITI-20

The classic ATNA feed is therefore DICOM PS3.15 §A.5 XML over syslog
(RFC 5424 message; RFC 5425 TLS or RFC 5426 UDP transport). Dropping it
would break conformance with every classic Audit Record Repository (ARR).

**Finding 2 — the official modern layer is HL7 FHIR AuditEvent, not
HL7v2.** HL7v2 has no audit-message role in any IHE profile. The RESTful
ATNA supplement ("Add RESTful Query and Feed to ATNA") extends ITI-20 with
the **ATX: FHIR Feed** option (HTTP `POST [base]/AuditEvent` of FHIR R4
AuditEvent resources) and adds **ITI-81 Retrieve ATNA Audit Event** (a FHIR
search on `AuditEvent` returning a Bundle). The **IHE BALP** profile (Basic
Audit Log Patterns, v1.1.4, FHIR 4.0.1) defines the normative AuditEvent
*content* patterns:

- RESTful activity: `IHE.BasicAudit.{Create,Read,Update,Delete,Query}` and
  the patient-centric variants `IHE.BasicAudit.Patient{Create,Read,Update,
  Delete,Query}` ("Privacy centric... well-formed indication of the Patient
  when they are the subject of the activity").
- Security tokens: `IHE.BasicAudit.OAUTHaccessTokenUse.{Minimal,Opaque,
  Comprehensive}`, `IHE.BasicAudit.SAMLaccessTokenUse.{Minimal,
  Comprehensive}`.
- CapabilityStatements: `IHE.BALP.AuditCreator`, `IHE.BALP.AuditConsumer`,
  `IHE.BALP.ATNA.AuditRecordRepository`.

**Finding 3 — what openEHR itself says about audit storage** (BASE
`architecture_overview/master07-security.adoc`):

- §Integrity (write audit): "every write access of any kind to any part of
  an openEHR record is logged with the user identification, time, reason" —
  this is the RM change-control `AUDIT_DETAILS`/CONTRIBUTION record,
  **already implemented in the versioning path and out of scope here**.
- §Access logging (read audit, line 158): "read accesses by application
  users to EHR data **should be logged in the EHR system**. Currently
  openEHR does not specify models of such logs... There are some proponents
  of the argument that even read-access logs should be made part of the
  content of the EHR proper; **currently openEHR does not support this
  approach**."
- §Record demerging (line 160): "the access logs for that EHR should be
  used to determine who has accessed that data" — access logs must be
  *queryable*.

Consequences: openEHR endorses keeping an access log **in the system**,
defines **no model** for it (ATNA fills that gap), and **prohibits** making
it part of the EHR content. So a local audit store is spec-consistent as
long as it lives entirely outside the EHR data (never in `node` /
`vo_version`, never versioned, never reachable through AQL or the openEHR
REST resources). Beyond the quotes above, no openEHR spec governs the audit
mechanics — IHE ATNA/BALP govern the format and transport; the storage
design below is our own design/extension and is flagged as such.

## 2. Owner rulings (2026-07-18)

1. **Format: dual.** Keep the spec-mandated DICOM PS3.15 §A.5 + syslog
   path AND add FHIR R4 AuditEvent per BALP + RESTful ATNA. Full ATNA
   compliance, classic and modern.
2. **Storage: both, configurable** (per the openEHR findings above): an
   optional **local PG-backed Audit Record Repository** (queryable via
   ITI-81 and the admin console) and optional **forwarding** to an external
   ARR (syslog and/or FHIR feed). Local storage is strictly outside the EHR
   content.
3. **ITI-19 in scope:** server-side mutual-TLS (client-certificate)
   support lands with this rewrite; Consistent Time (ITI-1/NTP) stays
   deployment guidance in the book.
4. **Default posture: audit ON with only the local store enabled** —
   compliance out of the box, nothing leaves the node; forwarding opt-in.
5. **Fail-closed is absolute:** with `fail_mode = "closed"`, a failed
   local-store write 503s the clinical request — no un-audited PHI access.
6. **Demographic (PARTY) operations audit on the non-patient BALP
   profiles** (PARTY id recorded as the entity); revisit if a
   PARTY↔EHR-subject link becomes resolvable.
7. **PurposeOfUse is deferred** until the authz layer carries a purpose
   claim (Stage-2 RBAC/ABAC); the record model keeps a clean seam for it.

## 3. Current state and gap analysis

The current subsystem (`app/ehrbase/src/system_log/` + `ehrbase-rest::
system_log`) already implements: the transport-agnostic event model, a
DICOM PS3.15 §A.5 XML serializer (insta-goldened), RFC 5424 assembly
(PRI 85) + RFC 5425 TLS (octet-counting, reconnect, private trust anchor,
optional client cert) + RFC 5426 UDP, a bounded-mpsc sender with
fail-open/fail-closed modes (fail-closed → 503 + `Retry-After`), total
classification coverage of every generated ITS-REST operation
(fail-closed default for unknown ops), background subject resolution, five
metrics, and E2E tests including a real TLS round-trip. That core survives;
the rewrite reshapes it and closes these gaps:

| # | Gap | Fix in this redesign |
|---|-----|----------------------|
| G1 | No FHIR AuditEvent representation at all | BALP renderer + ATX:FHIR Feed sink + local store (§5.3–5.5) |
| G2 | No local, queryable audit record (openEHR "record demerging" needs it) | PG-backed ARR + ITI-81 + admin-console browser (§5.5–5.6) |
| G3 | `MSGID` is `IHE+DICOM`; ITI-20 mandates `IHE+RFC-3881` | fix constant (`syslog.rs`) |
| G4 | Query/Export/Import collapse onto DCM 110110 "Patient Record" | dedicated EventIDs: 110112 Query, 110106 Export, 110107 Import |
| G5 | No `EventTypeCode`; no `PurposeOfUse` | add EventTypeCode; PurposeOfUse deferred until an authz purpose signal exists (owner ruling 7) |
| G6 | `MajorFailure` (12) never emitted | outcome mapping finalized against DICOM PS3.15 outcome definitions during implementation |
| G7 | No security-token audit content | BALP OAUTHaccessTokenUse.Minimal agent on FHIR records when the request carried a bearer token |
| G8 | Record lost on transport failure (metered, no durability) | local store as durable primary + outbox-style forwarder with `backon` retries (§5.4) |
| G9 | No ITI-19 node authentication beyond client-TLS to the ARR | server mTLS support (§5.7) |
| G10 | TLS key paths are plain strings, not the config `Secret`/`*_file` types | align with the config tree's secret handling |
| G11 | No dedicated book page (folded into security.md) | dedicated audit chapter (§7) |

Non-goals (unchanged behaviour): the RM change-control write audit
(`AUDIT_DETAILS`, versioning path) is a separate, already-implemented
concern; `extensions/fhir/feeder_audit.rs` (RM FEEDER_AUDIT provenance) is
unrelated; audit events never enter the EHR content or AQL surface.

## 4. Architecture overview

```
request ──► ehrbase-rest::system_log middleware (classify + build event)
                        │  EhrbaseService::emit (non-blocking)
                        ▼
             ehrbase::system_log core  ──  one canonical AuditRecord
                        │ bounded queue + background drain (existing pattern)
                        ▼
             ┌─────────────────────────────────────────────┐
             │ renderers                                   │
             │   • DICOM PS3.15 §A.5 XML (classic)         │
             │   • FHIR R4 AuditEvent per BALP (modern)    │
             └───────────────┬─────────────────────────────┘
                             ▼ fan-out to independently-enabled sinks
     ┌───────────────┬────────────────┬─────────────────────────────┐
     │ syslog UDP    │ syslog TLS     │ FHIR Feed (ITI-20 ATX:FHIR) │
     │ (RFC 5426)    │ (RFC 5425)     │ POST {arr}/AuditEvent       │
     └───────────────┴────────────────┴─────────────────────────────┘
                             │
                             ▼ (optional, durable, first in line)
              local ARR store (PG schema `audit`, outside EHR data)
                    ├── ITI-81: GET /fhir/r4/AuditEvent (FHIR search)
                    └── admin console: audit-log browser screen
```

## 5. Design

### 5.1 Canonical event core (`ehrbase::system_log::event` — reshaped)

One internal `AuditRecord` remains the single source both renderers consume.
Extensions over today's `AuditEvent`:

- `event_id`: a proper enum (PatientRecord 110110, Query 110112, Export
  110106, Import 110107, ApplicationActivity 110100, UserAuthentication
  110114 for login events) replacing the per-class 110110 reuse (G4).
- `event_type`: optional EventTypeCode carrying the concrete operation
  (rendered as DCM/IHE transaction codes where defined, else our own
  `originalText`-only code — flagged: no external code governs openEHR REST
  operations) (G5).
- outcome: keep the 0/4/8/12 ladder; the HTTP-status → outcome mapping is
  finalized against the DICOM PS3.15 outcome definitions with a unit table
  (G6).
- identity: split `user_id` into authenticated principal + optional bearer
  token identity (JWT `jti`/issuer for the BALP token-use agent, G7);
  never log token contents or clinical payloads (PHI rule in
  `.claude/rules/reliability.md` stands).
- patient identity: keep the background `SubjectResolver` enrichment;
  the resolved subject feeds both the DICOM Patient-Number participant and
  the BALP `entity[patient]` / Patient* profile selection.

Classification stays in `ehrbase-rest::system_log::classify` with its
total-coverage + fail-closed-default tests — that discipline is a keeper.

### 5.2 DICOM renderer (classic path — corrected, kept)

`message.rs` survives with: MSGID `IHE+RFC-3881` (G3), dedicated EventIDs
(G4), EventTypeCode (G5), the finalized outcome mapping (G6). Golden
snapshots re-cut; syslog assembly/transport (RFC 5424/5425/5426) unchanged
apart from the MSGID constant and the `Secret`-typed key config (G10).

### 5.3 FHIR renderer (modern path — new, `ehrbase::system_log::fhir`)

Renders the same `AuditRecord` to a FHIR R4 (4.0.1) `AuditEvent` JSON
document conforming to the BALP patterns. Mapping:

| Our record | BALP profile |
|---|---|
| C/R/U/D on a patient-centric class (Ehr, EhrStatus, Composition, Contribution, Directory, Extract) with subject resolved | `IHE.BasicAudit.Patient{Create,Read,Update,Delete}` |
| C/R/U/D, no patient subject (Template, stored-query definition, Demographic\*) | `IHE.BasicAudit.{Create,Read,Update,Delete}` |
| AQL execution scoped to one EHR | `IHE.BasicAudit.PatientQuery` |
| AQL execution, population scope | `IHE.BasicAudit.Query` |
| bearer-token request | + `IHE.BasicAudit.OAUTHaccessTokenUse.Minimal` agent |
| login/authentication events | UserAuthentication (110114) basic pattern |

\* Demographic PARTY resources are about persons but are not EHR patient
subjects; they audit on the non-patient profiles (owner ruling 6).

Hand-written serde structs for exactly the AuditEvent slices BALP needs
(no FHIR-crate dependency; consistent with the existing FHIR-connector
approach). Golden-vector tests against the official BALP examples.

### 5.4 Sinks and delivery (reshaped `sender.rs`)

Fan-out to independently enabled sinks behind the existing bounded-queue +
background-drain + fail-mode machinery:

- **syslog-udp / syslog-tls** — existing transports, DICOM payload.
- **fhir-feed** — ITI-20 ATX:FHIR Feed: `POST {arr_base}/AuditEvent`
  (`reqwest`, rustls), `backon` retry with jittered backoff, then the
  existing metered-drop semantics (or fail-closed per config).
- **store** — the local ARR (§5.5). When the store is enabled it is
  written first and is the durability anchor: forwarding sinks read their
  work from an outbox column set (per-sink sent/pending/failed markers) so
  a down ARR never loses records (G8) — no openEHR spec governs this; our
  own design, standard transactional-outbox pattern.

Fail-mode semantics (open/closed → 503 + `Retry-After`) and the metric set
carry over, extended per-sink (`atna_sink_sent_total{sink=...}` etc.).

### 5.5 Local Audit Record Repository (new — our own design/extension)

- **Schema:** new PG schema `audit` with its own sqlx migrator set (same
  pattern as `ext`/`ehr`), created via
  `sqlx migrate add --source app/ehrbase/migrations/audit --sequential`.
  One table `audit.audit_event`: `id uuidv7 PK`, `recorded_at timestamptz`,
  promoted search columns (`action`, `outcome`, `event_id`, `principal`,
  `patient_id`, `resource_type`, `resource_id`, `client_ip`, plus tenant
  scoping consistent with the RLS design), `fhir jsonb` (the rendered BALP
  AuditEvent — the canonical stored form), per-sink outbox columns.
  Strictly outside the EHR schema: no FK into `ehr`/`node`/`vo_version`, no
  AQL visibility, per BASE master07 §Access logging (logs are not EHR
  content).
- **Retention:** `audit.store.retention_days` (0 = keep forever) applied by
  a periodic reaper; deletion is the *only* mutation (append-only
  otherwise).
- **ITI-81:** `GET /fhir/r4/AuditEvent` on the existing FHIR façade —
  FHIR search returning a `searchset` Bundle; initial parameter set per
  ITI-81: `date` (ge/le), `agent`, `patient`, `entity`, `outcome`,
  `action`, `_count`/paging. Admin-scoped authz (same `access` policy layer
  as the other admin surfaces); served through our native utoipa OpenAPI
  like every other route.
- **Admin console:** a new audit-log browser screen in `ehrbase-admin-ui`
  consuming ITI-81 strictly over REST (filter form + paged table + record
  detail view). Registered as a dependency in the UI-2 design-overhaul
  plan.

### 5.6 Configuration (redesigned `[audit]` tree)

`[atna]` is replaced by a sink-structured `[audit]` tree (breaking config
change → minor version bump, changelog + book migration table):

```toml
[audit]
enabled = true                     # master switch
source_id = "ehrbase"
enterprise_site_id = "..."
value_if_missing = "UNKNOWN"
suppress_login_events = true
fail_mode = "open"                 # open | closed
queue_capacity = 1024
resolve_subject = true

[audit.store]                      # local ARR (the durability anchor)
enabled = true
retention_days = 0                 # 0 = keep forever

[audit.syslog]                     # classic ATNA feed (DICOM PS3.15 XML)
enabled = false
host = "arr.example.org"
port = 6514
transport = "tls"                  # udp | tls
tls_ca_file = "..."                # Secret-typed like the rest of the tree
tls_identity_cert_file = "..."
tls_identity_key_file = "..."

[audit.fhir_feed]                  # RESTful ATNA (ITI-20 ATX:FHIR Feed)
enabled = false
base_url = "https://arr.example.org/fhir"
# auth options aligned with the existing outbound FHIR connector config
```

Defaults (owner ruling 4): `audit.enabled = true` **with only the local
store on** — compliance out of the box, zero external dependency, nothing
leaves the node; forwarding stays opt-in. With `fail_mode = "closed"` a
failed store write 503s the request (owner ruling 5).

### 5.7 ITI-19 node authentication (server mTLS)

`ehrbase-server` gains optional TLS client-certificate verification:
`[server.tls]` grows `client_auth = "off" | "optional" | "required"` +
`client_ca_file`. Implemented with rustls' client-cert verifier via the
existing `axum-server` rustls integration; cipher/protocol floor per
BCP 195 (TLS 1.2 minimum, prefer 1.3). Applies to the CDR listener;
the admin console keeps consuming REST as an ordinary (m)TLS client.
Consistent Time (ITI-1) is documented as NTP deployment guidance in the
book, not code.

## 6. Work plan

Bounded phases, each a compiling/tested increment; ECC zero-drift at close.

- **A — core reshape:** `AuditRecord` + event-id/event-type/outcome model,
  classification updates, DICOM renderer corrections (G3–G6), re-cut
  goldens.
- **B — FHIR renderer:** BALP structs + mapping + token-use agent, golden
  vectors from the official BALP examples.
- **C — store:** `audit` schema + migrator + writer sink + retention
  reaper + testcontainers coverage.
- **D — forwarding:** sink fan-out rework, FHIR-feed sink (reqwest +
  backon + wiremock tests), outbox-driven delivery when the store is on,
  per-sink metrics.
- **E — ITI-81 + console:** FHIR-façade AuditEvent search + authz +
  OpenAPI, admin-ui browser screen (ui-gates + ui-e2e journey).
- **F — ITI-19 + docs + close:** server mTLS, config migration, book
  chapter (audit page + configuration + operations + security cross-links),
  changelog, `docs/endpoint-map.md`, full gates + ECC, delete this file.

## 7. Verification & documentation obligations

- Gates per phase: crate-scoped clippy/nextest while iterating, full
  workspace gates + `scripts/conformance.sh` zero-drift before merge;
  ui-gates + `scripts/ui-e2e.sh` for the console screen.
- Golden vectors: re-cut DICOM snapshots; BALP official examples as FHIR
  goldens; RFC 5424 header asserts updated for MSGID.
- Book: new dedicated audit-trail chapter (architecture, both formats, all
  sinks, ITI-81 usage, mTLS setup, NTP guidance, config migration table
  `[atna]` → `[audit]`); `installation/configuration.md` + `security.md` +
  `operations.md` updated in the same PRs.
- Changelog: `### Changed` (config redesign, MSGID/EventID corrections) +
  `### Added` (FHIR AuditEvent, local ARR, ITI-81, admin screen, mTLS).

## 8. Resolved questions

All four design questions were put to the owner on 2026-07-18 and resolved
as rulings 4–7 in §2 (defaults, fail-closed semantics, demographic
profiles, PurposeOfUse deferral). No open questions remain; implementation
can start at phase A.
