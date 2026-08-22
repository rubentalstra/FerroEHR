# Audit trail (IHE ATNA)

FerroEHR keeps a full security audit trail of API access — _who_ did _what_
to _which_ resource, with _what outcome_, from _where_, and _when_ — following
the IHE **ATNA** (Audit Trail and Node Authentication) profile, the standard
openEHR itself points at (the platform Service Model names the System Log
component "IHE ATNA-compliant system log"). It is **on by default**: every
deployment records a queryable audit trail with zero external dependencies.

<!-- toc -->

The trail is orthogonal to openEHR's own `CONTRIBUTION`/`AUDIT_DETAILS`
change-control audit (which the server always writes in the same transaction
as every version change): openEHR audit records what a version says about its
own authorship; ATNA records security surveillance of *access*, including
reads and rejected attempts.

## The record, in both official formats

Every audited operation produces one record, rendered in the two formats the
IHE standards define:

- **FHIR R4 `AuditEvent`** following the IHE **BALP** (Basic Audit Log
  Patterns) content profiles — the modern RESTful-ATNA form and the canonical
  stored form. Patient-centric operations carry the resolved EHR subject as
  the patient entity (`PatientRead`/`PatientCreate`/… profile claims); query
  executions carry the search expression; Bearer-authenticated requests
  record the token's `jti` (and never the token itself) per
  `OAUTHaccessTokenUse.Minimal`.
- **DICOM Audit Message** (DICOM PS3.15 §A.5 XML) — the classic ATNA form,
  shipped over syslog per IHE ITI-20 when the syslog sink is enabled.
  Dedicated DICOM event ids are used throughout: Patient Record (110110),
  Query (110112), Export/Import (110106/110107, the EHR-Extract directions),
  User Authentication (110114, with the Login `EventTypeCode` 110122).

Every server operation is audited — an unrecognised extension operation fails
*closed* to a generic audited class, so nothing is silently unaudited — and
access refusals are always recorded: a `401`, a `403`, and the `400` a
malformed `Authorization` header earns. A refusal is attributed to the caller
**only when one actually authenticated**; an unattributable denial is recorded
as unattributed rather than under a fabricated subject, so no audit record
ever names an identity that did not authenticate.

> [!NOTE]
> **Successful-login records are suppressed by default**
> (`[audit] suppress_login_events = true`). Set it to `false` to record them.
> Even then, a login record marks a genuine authentication *event*, not every
> authenticated request: it is emitted when credentials were actually
> verified — a Basic verified-credential-cache miss. A cache hit continues an
> established session, and a Bearer request authenticated out of band at your
> identity provider, so neither mints a per-request login record. Rejections
> are recorded regardless of this switch.

## Sinks

Records fan out to independently configured sinks (`[audit]` in
`ferroehr.toml` — see the
[configuration reference](installation/config-audit.md#audit)):

| Sink | Default | What it does |
|---|---|---|
| `[audit.store]` | **on** | The local **Audit Record Repository**: records persist in the dedicated `audit` PostgreSQL schema (append-only and tamper-evident, strictly outside the EHR content), served back via the ITI-81 search below. `retention_days` prunes old records hourly (`0` = keep forever). |
| `[audit.syslog]` | off | The classic ATNA feed: DICOM PS3.15 XML over syslog (RFC 5424; UDP or TLS transport) to an external ARR, per IHE ITI-20. |
| `[audit.fhir_feed]` | off | The RESTful-ATNA feed (ITI-20 **ATX: FHIR Feed**): each FHIR `AuditEvent` is `POST`ed to an external FHIR ARR. With the local store on, delivery is **outbox-driven**: an ARR outage loses nothing, pending records ship on recovery. |

Emission never blocks the request path — a record is handed to a bounded queue
(`queue_capacity`) and written by a background drain.

The local store is the durability anchor. Under `fail_mode = "closed"`, an
operation whose audit record cannot be recorded answers
`503 Service Unavailable` — the deployment demanded an audit trail it cannot
currently deliver, so no un-audited PHI access happens. (`open`, the default,
drops-and-meters instead; every loss path is metered — see the `atna_audit_*`
counters in [Operations](operations.md).) Login and rejection records never
gate a response in either mode.

## Tamper evidence

An audit trail that anything with the application's database password can
quietly rewrite is a log, not an accountability record. The local store is
therefore tamper-**evident**: every record is linked into a SHA-256 hash chain
maintained inside PostgreSQL, so each record commits to its predecessor and to
its own content. The chain is built by the database itself, not by the server,
which means it covers every writer — the per-event insert, the batched drain,
and any statement typed by hand.

Three controls sit on top of it, and they are separate on purpose:

- **The table refuses the ordinary rewrite paths outright.** The only permitted
  change to a stored record is the per-sink forwarding stamp; an `UPDATE` of any
  other column, a `DELETE`, and a `TRUNCATE` are refused by the database.
  Retention pruning goes through the one sanctioned deletion path, which records
  *which* records it removed and what the surviving chain must link back to — so
  reaping does not look like tampering, and tampering does not look like
  reaping.
- **The privileges are narrow.** The runtime role may record an event, stamp it
  forwarded, and read the trail back. It holds nothing that can rewrite a
  record, remove one, or alter the chain's own bookkeeping. That posture is only
  fully in force with `db.migrate = "verify"` — see
  [Operations](operations.md#database-roles-and-least-privilege), because a
  self-migrating server owns the schema and can therefore turn the enforcement
  off.
- **Verification is a query you can run yourself.** It recomputes every digest,
  re-walks every link, and checks both ends of the chain:

  ```sql
  SELECT * FROM audit.verify_audit_chain();
  ```

  An empty result means the trail is intact. Any row names one record — its
  chain position, its id, when it was recorded — and what is wrong with it:
  content modified after it was written, records deleted with no retention
  record for the removal, or records removed from the end of the chain where no
  successor would have noticed. Run it on a schedule and alert on any output.

> [!IMPORTANT]
> This is **detection, not prevention**, and the boundary is worth stating
> plainly. The chain is unkeyed, so a party with unrestricted write access to
> the `audit` schema — the database owner, or a superuser — can delete a record
> and recompute every hash after it. What closes that case is not a bigger hash:
> it is keeping the trail somewhere that party does not control. Enable
> `[audit.syslog]` or `[audit.fhir_feed]` so records leave the box as they are
> written, and give the server an app-role-only DSN so it is not that party.

## Retrieving audit records (ITI-81)

The RESTful-ATNA **ITI-81 Retrieve ATNA Audit Event** transaction is served
at the FHIR façade:

```text
GET /ferroehr/rest/openehr/v1/fhir/r4/AuditEvent
```

It returns a FHIR `searchset` Bundle of the stored `AuditEvent` documents,
newest first, with the full match `total`. Supported search parameters:

| Parameter | Meaning |
|---|---|
| `date` | event-time bound, `ge`/`le`-prefixed RFC 3339 instants, repeatable |
| `patient` | the recorded patient (EHR subject) id |
| `agent` | the authenticated principal |
| `entity` | the touched resource id |
| `outcome` | the outcome indicator: `0`, `4`, `8` or `12` |
| `action` | the action code: `C`, `R`, `U`, `D` or `E` |
| `_count` / `_offset` | paging; page size defaults to 50, capped at 1000 |

Other FHIR search parameters are ignored (lenient search); a malformed value on
a supported one is a `400` carrying a FHIR `OperationOutcome`. The surface is
**admin-only** under RBAC and answers `404` when the local store is disabled.

```bash
# Who accessed patient-42's data this month?
curl -u admin:pw \
  "https://cdr.example.org/ferroehr/rest/openehr/v1/fhir/r4/AuditEvent?patient=patient-42&date=ge2026-07-01T00:00:00Z"
```

This is also the openEHR "record demerging" instrument: when data lands in
the wrong EHR, the patient-filtered audit search shows exactly who read it.

> [!NOTE]
> Audit records are themselves sensitive — they name patients, subjects and
> actions — so this endpoint is a PHI-disclosure surface and is authorized like
> any other.

## Node authentication (ITI-19, mutual TLS)

ATNA's second half is node authentication. `[server.tls]` terminates TLS
natively and can demand a verified client certificate:

```toml
[server.tls]
enabled = true
cert_file = "/etc/ferroehr/server.pem"
key_file = "/etc/ferroehr/server.key"
min_version = "1.3"               # 1.3 (default) | 1.2
client_auth = "required"          # off | optional | required
client_ca_file = "/etc/ferroehr/client-ca.pem"
```

The protocol floor is **TLS 1.3 only** by default, following the OWASP
Transport Layer Security cheat sheet. Setting `min_version = "1.2"` enables
1.2 *alongside* 1.3 — never instead of it — for a client that genuinely cannot
do 1.3, such as an older integration engine or a pinned Java runtime. TLS 1.1
and 1.0 are not selectable at all.

With `client_auth = "required"`, only clients presenting a certificate
chaining to your explicit trust anchor complete the handshake — the IHE
mutually-authenticated-node posture. Deployments terminating TLS at an
ingress keep `[server.tls]` off and enforce mTLS there instead.

> [!WARNING]
> The separate-port management listener (`management.port`) stays **plain HTTP**
> even with `[server.tls]` enabled, and it binds all interfaces. Treat it as an
> internal surface and keep it off any publicly routed port.

Complete the posture with time synchronisation (IHE Consistent Time): run
NTP/chrony on every node so audit timestamps align across systems.

## Configuration summary

Auditing defaults to on with the local store only; see the
[configuration reference](installation/config-audit.md#audit) for every
`[audit]` key and its `FERROEHR__AUDIT__*` environment form. The syslog sink's
own keys are `host` / `port` / `transport` / `tls_ca_file` /
`tls_identity_cert_file` / `tls_identity_key_file` under `[audit.syslog]`; the
switches shared by every sink stay directly under `[audit]`.

> [!NOTE]
> There are no alternative spellings for any of these: an unrecognized
> `FERROEHR_*` variable or an unknown TOML key is a **boot error** naming the
> spelling it should have had, never a silently ignored setting. Validate a
> deployment's configuration up front with `ferroehr config check`.
