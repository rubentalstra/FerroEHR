# System Log (`I_SYSTEM_LOG`) — spec-first audit of the platform module (W-3f)

Owner directive 2026-07-12 (W-3f, spec-first): map the vendored openEHR SM +
BASE spec **onto** the `ehrbase::system_log` module, never the code onto the
spec. This document builds its register from the spec skeleton first, then maps
each existing file onto it. **Conclusion up front: this area is a re-ground, not
a teardown.** The module already realizes the one normative openEHR line
(IHE-ATNA) faithfully, every file is well under the ~700-line ceiling, and the
divergences are small (citation hygiene + one integration seam). No file is a
delete/quarantine candidate.

**Spec oracle** (read before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master02-overview.adoc` — the
  platform component table (the single normative System-Log line) and the
  out-of-band-auth statement.
- `docs/specs/openehr/SM/docs/UML/classes/i_system_log.adoc` — the
  `I_SYSTEM_LOG` interface (verified **empty** — see SL-2).
- `docs/specs/openehr/BASE/docs/architecture_overview/master07-security.adoc`
  — the security chapter's audit-logging concerns (§Audit trailing, §Access
  logging, §Record demerging, §Non-repudiation, §Integrity/Versioning).
- **Externally-cited standards** the ATNA line pulls in: **DICOM PS3.15 §A.5**
  (Audit Message schema), **IETF RFC 3881** (audit participant-object codes),
  **RFC 5424** (syslog format), **RFC 5425** (syslog-over-TLS), **RFC 5426**
  (syslog-over-UDP). openEHR normatively names none of these element-by-element
  — they are inherited *through* the "IHE ATNA-compliant" mandate and are cited
  as external standards, not as openEHR spec text.

**The trait surface is FIXED** (not in W-3f scope): `ehrbase_sm::SystemLog`
(`app/ehrbase-sm/src/services/system_log/{mod,service}.rs`) — `emit` +
`audit_enabled` + `suppress_login_events`, over the transport-agnostic
`AuditEvent` / `EventActionCode` / `EventOutcome` / `ObjectClass` /
`EmitOutcome` model. This audit governs only the platform-crate **rendering**
(`app/ehrbase/src/system_log/`) and classifies the sibling `src/telemetry/`.

**Current implementation** (verified 2026-07-12):

| File | Lines | Role |
|------|-------|------|
| `app/ehrbase/src/system_log/mod.rs` | 88 | module map + `impl SystemLog for EhrbaseService` |
| `app/ehrbase/src/system_log/message.rs` | 466 | DICOM `AuditMessage` model + `quick-xml` serializer |
| `app/ehrbase/src/system_log/codes.rs` | 235 | DCM / RFC-3881 code constants + `AtnaAction`/`AtnaOutcome`/`AtnaObject` renderings |
| `app/ehrbase/src/system_log/syslog.rs` | 358 | RFC 5424 assembly + RFC 5425 TLS / RFC 5426 UDP transports |
| `app/ehrbase/src/system_log/sender.rs` | 248 | bounded-mpsc sender + background drain + fail modes |
| `app/ehrbase/src/system_log/config.rs` | 185 | `figment` `AuditConfig` (`EHRBASE_ATNA_*`) |

Wire seam (in `ehrbase-rest`, out of this crate's scope but named as a seam):
`app/ehrbase-rest/src/system_log/{classify.rs (330), middleware.rs (356)}` —
op-id → `AuditEvent` classification + the tower layer that emits through the
`Platform` supertrait. Behavioural design record:
`docs/enterprise/atna-audit.md` (a repo design doc — see G-8 on how it is cited).

---

## 1. Spec-derived requirement register (built from the spec, then mapped)

Each row is a requirement the spec (openEHR) or the standards it inherits
(external) place on the System Log, followed by the code that satisfies it.

| # | Requirement | Citation | Code | Verdict |
|---|-------------|----------|------|---------|
| **SL-1** | The platform MUST provide an **IHE ATNA-compliant system log** component. This is the *entire* normative openEHR statement, verbatim: `\|System Log \| IHE ATNA-compliant system log.` | SM `master02-overview.adoc` (platform component table) | whole module | **conformant** — DICOM audit message shipped over ATNA-profile syslog |
| **SL-2** | `I_SYSTEM_LOG` defines **no operations** — the interface body is empty (`h\|*Description* 2+a\|` with no content, no calls). The concrete call contract is therefore undefined by the spec and is a design decision. | SM `i_system_log.adoc` (verified empty stub) | `ehrbase_sm::SystemLog` (FIXED); `mod.rs:70` impl | **conformant** — a recorded spec gap; the `emit`/policy contract is our design (PORT NOTE, keep) |
| **SL-3** | Authentication/authorization are **out of band** ("assumed to have been dealt with before any particular call … OAuth, RFC 7235 … role-based access control"). The system log records *what happened*, it does not enforce authz; auth **failures** are nonetheless audit-worthy events. | SM `master02-overview.adoc` §(interface-call semantics), line 109 | `ObjectClass::ApplicationActivity`, `suppress_login_events` (`config.rs:60`); failures always audited (rest middleware) | **conformant** |
| **SL-4** | **Access logging** — "read accesses by application users to EHR data **should** be logged … openEHR does not specify models of such logs." The ATNA log is exactly this unspecified model. | BASE `master07-security.adoc` §Access logging | `EventActionCode::Read` audited; `data_life_cycle`=access-use `6` (`codes.rs:152`) | **conformant** — read ops emit records |
| **SL-5** | **Record demerging** relies on per-EHR access logs to determine who accessed mis-filed data. | BASE `master07-security.adoc` §Record demerging | Patient-Number participant object carries the resolved `ehr.subject_id` (`message.rs:211`, `sender.rs:156`) | **conformant** — subject-scoped records support demerge queries |
| **SL-6** | **Non-repudiation** — "logging of communication of Extracts … can be used to guarantee non-repudiation of information passed between systems." | BASE `master07-security.adoc` §Non-repudiation | no `ObjectClass::Extract`; EHR-Extract (SM-5) export/import not classified | **missing** — see G-7 (integration seam, not a teardown) |
| **SL-7** | **Write-access audit** — "every write access of any kind … is logged with the user identification, time, reason." **Placement note:** this is the RM *change-control* audit (`AUDIT_DETAILS` on every VERSION/CONTRIBUTION), **not** the ATNA system log. | BASE `master07-security.adoc` §Integrity/Versioning, line 198 | versioning path + `audit` table (`0001_baseline.sql`), not `system_log` | **conformant** — correctly placed **outside** this module (scope boundary, G-6) |
| **SL-8** | Audit records use the **DICOM Audit Message schema**: `EventIdentification`, `ActiveParticipant(s)`, `AuditSourceIdentification`, `ParticipantObjectIdentification(s)`. | DICOM PS3.15 §A.5 (external) | `message.rs:53` model + `to_xml` (`message.rs:130`) | **conformant** |
| **SL-9** | `EventActionCode` (C/R/U/D/E), `EventOutcomeIndicator` (0/4/8/12), `EventID` codes. | DICOM PS3.15 §A.5.1 (external) | `codes.rs` `AtnaAction`/`AtnaOutcome`/`AtnaObject` | **conformant** (code-choice PORT NOTEs G-3) |
| **SL-10** | Participant-object type/role/id-type/data-life-cycle codes. | IETF RFC 3881 §5.3/§5.5 (external) | `codes.rs:60–95`, `message.rs:build_objects` | **conformant** |
| **SL-11** | Records framed as **RFC 5424** syslog messages (PRI, VERSION, HEADER, UTF-8 BOM + MSG). | RFC 5424 (external) | `syslog.rs:assemble_syslog` | **conformant** (PRI/MSGID PORT NOTEs G-4/G-5) |
| **SL-12** | **TLS** transport (RFC 5425 octet-counting) — the IHE-recommended secure transport; realizes the **Node Authentication** half of ATNA via mutual-TLS trust anchors. | RFC 5425 (external); ATNA node-auth | `syslog.rs:TlsTransport`, `tls_client_config` (`syslog.rs:257`) | **conformant** |
| **SL-13** | **UDP** transport (RFC 5426, one datagram per message). | RFC 5426 (external) | `syslog.rs:UdpTransport` | **conformant** |
| **SL-14** | Missing mandatory fields carry a fill value (never absent). | DICOM PS3.15 §A.5 (mandatory-field rule) | `value_if_missing` / `nonempty` (`message.rs:266`) | **conformant** |

`src/telemetry/` (config/indicators/layers/mod/prometheus/samplers) maps to
**no** SL row: it is `tracing`/`metrics`/OTLP/Prometheus observability
infrastructure. **No openEHR spec governs this — our own design.** It is
operational telemetry (spans, gauges, Prometheus scrape), categorically
distinct from the ATNA *audit* trail (a security/medico-legal record). Correct
that it is a sibling module and **stays outside `system_log`** — merging them
would conflate observability with the ATNA compliance surface. Placement:
already-correct.

---

## 2. G-row register

| # | Item | Citation / flag | Severity | Disposition |
|---|------|-----------------|----------|-------------|
| **G-1** | `I_SYSTEM_LOG` is an empty stub; the `emit`/`audit_enabled`/`suppress_login_events` contract is our design. | SM `i_system_log.adoc` (empty) | low | **PORT NOTE (keep)** — already recorded in `ehrbase-sm` service.rs and `mod.rs`; re-verified accurate |
| **G-2** | The DICOM/RFC codes, schema, transports are correctly implemented against the external standards. | DICOM PS3.15 §A.5; RFC 3881/5424/5425/5426 | — | **already-correct** |
| **G-3** | EventID code choices where the DICOM table is silent: query→`110110`+`"query"` (not `110112`), template→`110100`+`"template"`, demographic→`110110`+`"demographic"`. | DICOM PS3.15 §A.5.1 (silent on these) | low | **PORT NOTE (keep, re-verify)** — 3 notes in `codes.rs:27,191,197`; sound and cited |
| **G-4** | Syslog PRI severity = 5 (Notice) is an IHE convention, not fixed by RFC 5424. | RFC 5424 §6.2.1 (silent) | low | **PORT NOTE (keep)** — `syslog.rs:24` |
| **G-5** | `MSGID = "IHE+DICOM"` is the IHE ATNA application-defined value. | RFC 5424 §6.2.7 (app-defined) | low | **PORT NOTE (keep)** — `syslog.rs:38` |
| **G-6** | Write-audit (`master07` §Versioning) is the RM `AUDIT_DETAILS` change-control record, **not** the ATNA system log — correctly implemented in the versioning path, not here. | BASE `master07-security.adoc` §Integrity | low | **already-correct** — document as a scope boundary in `mod.rs` (no code change) |
| **G-7** | **Non-repudiation of EHR-Extract communication** (`master07` §Non-repudiation) has no coverage: `ObjectClass` has no `Extract` variant and SM-5 export/import is not classified/emitted. | BASE `master07-security.adoc` §Non-repudiation | **med** | **fix-in-rewrite / TODO(w3f-integrate)** — add an `Extract` object class (SM trait is FIXED; enum extension is a coordinated change) and classify SM-5 message ops; until then, PORT NOTE the gap |
| **G-8** | Code comments justify behaviour by citing the **repo design doc** `docs/enterprise/atna-audit.md §N` (e.g. `message.rs:7` "golden vector … §3"; `codes.rs:29`; the §3 field-mapping references). The owner hard rule requires **spec** citations in code; an internal doc is neither the openEHR spec nor an ADR, but it is not the oracle. | owner rule (spec-adherence.md); `message.rs:7`, `codes.rs:29,199` | **med** | **fix-in-rewrite** — re-anchor each justification to the external standard section (DICOM PS3.15 §A.5.x / RFC 3881 §5.x), demoting `atna-audit.md` to a "see also" design pointer |
| **G-9** | `src/telemetry/` is spec-silent observability infra, correctly separate from the audit trail. | flag: no openEHR spec governs this — our own design | low | **already-correct** — keep as sibling module; do not fold into `system_log` |
| **G-10** | No `system_log` DB table: records are fire-and-forget to the ARR over syslog (the ATNA model). Schema is settled; nothing to change. | ATNA (ARR is the record store) | — | **already-correct** |

Counts: **already-correct 4** (G-2, G-6, G-9, G-10) · **PORT NOTE keep/re-verify
4** (G-1, G-3, G-4, G-5) · **fix-in-rewrite 2** (G-7 also an integrate seam,
G-8) · **quarantine/delete 0**.

---

## 3. Target design — re-grounded `app/ehrbase/src/system_log/`

The module keeps its current shape (it is close to spec-true; this is a
re-ground). File layout is unchanged and every file stays well under ~700
lines (largest is `message.rs` at 466):

```
app/ehrbase/src/system_log/
├── mod.rs      # module map + impl SystemLog for EhrbaseService
│               #   + a documented SCOPE BOUNDARY note (G-6): write-audit is
│               #     RM AUDIT_DETAILS in the versioning path, not here
├── message.rs  # DICOM AuditMessage (PS3.15 §A.5) + quick-xml serializer
│               #   G-8: citations re-anchored to PS3.15 §A.5.x, not atna-audit.md
├── codes.rs    # DCM / RFC-3881 constants + AtnaAction/Outcome/Object
│               #   G-3/G-8: code-choice PORT NOTEs cite PS3.15 §A.5.1 directly
│               #   G-7: add ObjectClass::Extract rendering once the SM enum grows
├── syslog.rs   # RFC 5424 assembly + RFC 5425 TLS / RFC 5426 UDP
│               #   G-4/G-5: PRI/MSGID PORT NOTEs (kept, cite RFC 5424 §6.2.x)
├── sender.rs   # bounded-mpsc sender + background drain + fail-open/closed
└── config.rs   # figment AuditConfig (EHRBASE_ATNA_*)
```

Design decisions (all re-grounds, no structural change):

1. **Keep the file split** — one file per ATNA concern (schema / codes /
   transport / sender / config), each independently testable, all sub-700.
2. **G-8 citation re-anchoring** — the only pervasive edit: replace every
   `docs/enterprise/atna-audit.md §N` *justification* with the external
   standard section it derives from (DICOM PS3.15 §A.5.x, RFC 3881 §5.x, RFC
   5424 §6.2.x). `atna-audit.md` survives only as a non-normative "see also".
3. **G-6 scope-boundary doc** — a one-paragraph `mod.rs` note stating the ATNA
   system log is the *read/operation* audit; the *write/change-control* audit
   is `AUDIT_DETAILS` in versioning (BASE `master07` §Integrity). No behaviour
   change; prevents future double-implementation.
4. **G-7 EHR-Extract audit** — the substantive open item. Coordinated with the
   FIXED `ehrbase_sm` model: add `ObjectClass::Extract` (+ its DICOM `EventID`
   rendering here) and classify SM-5 export/import so Extract communication is
   audited (non-repudiation, `master07`). Marked `TODO(w3f-integrate)` because
   it spans the SM crate and the rest classification table.
5. **Telemetry stays separate** (G-9) — no move.

---

## 4. Seams (`TODO(w3f-integrate)` candidates)

- **Every service chapter emits events.** The platform builds an `AuditEvent`
  and calls `SystemLog::emit`; today only the **rest middleware**
  (`ehrbase-rest/src/system_log/middleware.rs`, via `classify.rs`) drives this
  for generated ITS-REST ops. Native-API / SM-5/SM-6 operations reached
  outside the REST surface (EHR-Extract export/import, TDD import, Subject
  Proxy) have **no** emission path — `TODO(w3f-integrate)`: a service-layer
  emission seam so those chapters audit too (this is where G-7 lands).
- **Rest middleware classification** (`ehrbase-rest`, FIXED seam this phase):
  the op-id → `AuditEvent` table + coverage guard (empty-`UNAUDITED`-allowlist
  assertion) stay; when `ObjectClass::Extract` is added, `classify.rs` gains
  the extract op-ids. `TODO(w3f-integrate)`.
- **Subject resolver** (`sender.rs:SubjectResolver`) — the binary injects the
  `ehr.subject_id` lookup so this crate stays DB-free; unchanged, verified.

---

## 5. Standing PORT-NOTE residue after the re-ground (the honest tail)

- **Kept, re-verified:** the empty-`I_SYSTEM_LOG`-stub → our-contract note
  (G-1); the query/template/demographic EventID code choices (G-3); syslog
  severity=5 / PRI=85 (G-4); `MSGID=IHE+DICOM` (G-5). Each now cites the
  external standard section directly (G-8), with `atna-audit.md` as see-also.
- **New:** EHR-Extract-communication non-repudiation is PORT-NOTEd as an
  integration gap until G-7 lands (SM enum + SM-5 classification).
- **Dropped:** none — no PORT NOTE in this module was found stale or wrong.
- **Scope boundary (not a PORT NOTE, a doc note):** write/change-control audit
  is `AUDIT_DETAILS` (versioning), not the ATNA system log (G-6); telemetry is
  spec-silent observability, not audit (G-9).
