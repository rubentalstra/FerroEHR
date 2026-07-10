# Product completeness roadmap — ehrbase-rs vs the enterprise CDR market

- **Date:** 2026-07-10 · **Owner-confirmed postures:** FHIR connectors+façade ·
  contribution-outbox eventing · fully-integrated multi-tenancy (Stage-2
  flagship) · S3 multimedia externalization. UI suite explicitly excluded
  (headless product).
- **Market reference:** vitagroup HIP CDR Suite (EHRbase core + proprietary
  enterprise layer). We compete as a **headless, spec-measured, pure-Rust
  CDR**: no frontend, every capability API-first, every conformance claim
  machine-verified.
- **Spec grounding:** every planned capability below cites the vendored
  openEHR specs (`docs/specs/openehr/`) or carries an explicit
  **spec-silent** flag (meaning: our design fills a seam the spec names but
  does not standardise — recorded per ADR discipline). Research basis:
  B8 spec-grounding pass (SM master02/09/10/12, RM ehr_extract,
  DV_MULTIMEDIA, BASE architecture-overview §The EHR System).

---

## 1. Scorecard

Legend: ✅ have (evidence) · 🔷 planned (spec-grounded design below) ·
◽ deliberately excluded (reason).

### Data storage

| Capability | HIP | ehrbase-rs | Evidence / plan |
|---|---|---|---|
| openEHR REST API | 1.0.1 | ✅ **1.0.3** (generated contract) | ECC 341/315/0; CORE+STANDARD **PASS** (machine-computed Certificate) |
| openEHR AQL | 1.0 | ✅ **1.1** | corpus goldens + TERMINOLOGY() family |
| openEHR RM | 1.1 | ✅ **1.2.0** (BASE 1.3, TERM 3.1, AM 1.4+2.4) | generated from BMM, fidelity gates |
| `ALL_VERSIONS` AQL | — (EHRbase rejects) | ✅ | temporal `vo_version` (ADR-008) |
| Server Admin API | ✓ | ✅ + dump/load + archive | SM I_ADMIN + I_ADMIN_DUMP_LOAD implemented |
| FHIR Server R4 + Search | ✓ | 🔷 **connectors + read façade** (not a second server) | §2.1 |
| Event Trigger | ✓ | 🔷 **contribution outbox + filters** | §2.2 |
| Binary Storage (S3) | ✓ | 🔷 **DV_MULTIMEDIA externalization** | §2.4 |

### Integration

| Capability | HIP | ehrbase-rs | Evidence / plan |
|---|---|---|---|
| Integration engine / custom mappings | ✓ | 🔷 connector framework on the SM seams | §2.1 — Subject-Proxy `DATA_FRAME` + EHR Extract are the spec's own slots |
| FHIR connector in/out | ✓ | 🔷 | §2.1; terminology client already speaks FHIR TS (B4) |
| HL7v2 connector in/out | ✓ | 🔷 (behind the same frame seam; second priority) | `HL7v2_SAMPLE` is a named SM frame type |
| EHR Extract / TDD import | — (not surfaced) | ✅ | SM-5: export+import incl. IMPORTED_VERSION; TDD→COMPOSITION converter |
| Subject Proxy service | — | ✅ (openEHR frame live; FHIR/HL7v2 frames = the connector seam) | SM-6 |

### Scalability & operations

| Capability | HIP | ehrbase-rs | Evidence / plan |
|---|---|---|---|
| PostgreSQL | ✓ | ✅ **PG 18-native** (uuidv7, temporal PK, skip scan) | ADR-008/013 |
| Horizontal scaling (YugabyteDB) | optional | ◽→🔷 *verification item only* | PG-wire-compatible engines are a compatibility test pass (temporal PK/GiST support must be verified), not a port. Not roadmapped until a customer needs it. |
| Multi-tenancy (API + integrated) | ✓✓ | 🔷 **Stage-2 flagship, fully integrated** | §2.3 — schema already RLS-ready (ADR-013) |
| Transaction compensation | ✓✓ | ✅ *semantics covered, different shape* | openEHR-native: CONTRIBUTION atomicity (all-or-nothing commits), indelible versioning + logical delete, admin physical delete; a dedicated "compensation API" is redundant in a version-controlled store — PORT NOTE stance |
| AMQP integration bus | ✓ | 🔷 broker publisher on the outbox | §2.2 |
| Kubernetes configuration | ✓ | 🔷 Helm chart / manifests (deployment artifact) | cheap, high checkbox value; includes the roles/pgaudit/TLS posture from ADR-013 + review doc 02 |

### Security

| Capability | HIP | ehrbase-rs | Evidence / plan |
|---|---|---|---|
| Basic Auth | ✓ (HIP: dropped) | ✅ | argon2 store |
| OAuth2 / OIDC + external IdPs (AD, Keycloak) | ✓ | ✅ | openidconnect/jsonwebtoken; JWKS/issuer discovery |
| ABAC **API** (PDP integration) | ✓ | ✅ **remote-PDP mode** | `authz_remote_pdp` e2e tests |
| ABAC **fully integrated** | ✓ (XACML) | ✅ **Cedar policy engine** (XACML-equivalent, modern policy language) | `access` module; `abac_e2e`, `authz_cedar_engine` tests; docs/enterprise/access-control.md |
| RBAC | — | ✅ | `rbac_e2e` |
| ATNA logging (API + integrated) | ✓✓ | ✅ both | `system_log` module: DICOM-over-syslog (TLS), evidenced; docs/enterprise/atna-audit.md |
| DB-layer hardening | — | ✅ role architecture, pgaudit posture, PITR guidance | ADR-013 + schema-review 02 |

### Admin & dev tools (headless posture)

| Capability | HIP | ehrbase-rs | Substitute |
|---|---|---|---|
| Admin dashboard / Template UI / Explorer / AQL editor / User+Tenant UIs / Forms / Patient viewer | ✓ | ◽ **excluded — no frontend, by design** | The API-first substitutes: full management surface (`/management`, health, Prometheus/OTLP), OpenAPI + Swagger UI served from the generated contract, template/query/admin REST APIs, dump/load + conformance CLI, machine-readable Conformance Statement/Certificate artifacts. Third parties (or a future separate product) can build UIs on these — the CDR itself stays headless. |
| Conformance instrument | — | ✅ **ECC runner + Certificate generation** (unique differentiator) | `tools/conformance`; nobody else ships machine-verified CORE/STANDARD PASS artifacts |

### Services (SLA/consulting): business offerings, out of scope for the codebase.

---

## 2. The four planned capabilities (owner-confirmed, spec-grounded)

### 2.1 FHIR connectors + read façade (not a second server)

**Spec basis:** Subject Proxy `DATA_FRAME`/`HL7_FHIR_SAMPLE` (SM master10
§Bindings — FHIR named as a frame source, "currently not standardised");
EHR Extract as the sanctioned coarse-grained interop serialization that
bridges non-openEHR formats via `GENERIC_ENTRY` (RM ehr_extract
§Requirements); TERM's official FHIR CodeSystem/ValueSet renderings; our
B4 FHIR terminology client. **Spec-silent:** any FHIR REST mapping or
conversion algorithm — our design, PORT-NOTEd.

**Design:** a `connector` layer (new crate or `ehrbase-rest` feature) with:
(a) **inbound**: FHIR R4 resources (Observation, Condition, Patient, …) →
template-driven mapping to COMPOSITIONs (reusing the TDD/WebTemplate
machinery pattern) committed through the normal validated path; (b)
**outbound**: contribution-outbox-driven (see 2.2) FHIR resource emission +
on-demand read façade (`GET /fhir/r4/...` mapping to AQL under the hood);
(c) mapping definitions as data (versioned mapping artefacts), not code —
the "custom mappings" parity. HL7v2 follows the same frame seam later.

### 2.2 Eventing: contribution outbox → broker (AMQP first)

**Spec basis:** SM master02 §General Assumptions sanctions Kafka/AMQP as
protocol adapters over the native API; §Interface Calls requires adapters
to preserve pre/post-conditions transactionally. **Spec-silent:** any
event/subscription semantics — our design.

**Design:** a transactional **outbox table** written in the same
transaction as every CONTRIBUTION (we already have the atomic commit
point); a publisher daemon drains it to a pluggable broker backend
(`lapin`/AMQP first; Kafka-ready trait) with at-least-once delivery +
ordering per EHR; **event filters** (the "Event Trigger" parity) as
server-side subscriptions: predicate on kind/template/archetype/change_type
(optionally an AQL-shaped condition later). Event payload = the
contribution envelope (ids + audit + version refs), never PHI-by-default;
full-fetch is a callback through the authenticated API.

### 2.3 Fully-integrated multi-tenancy (Stage-2 flagship)

**Spec basis:** BASE architecture-overview §The EHR System — one deployment
"may house multiple logical EHR systems in a multi-tenant fashion"; the
spec boundary is `system_id` (on EHR, audits, and every
OBJECT_VERSION_ID's creating_system_id). **Spec-silent:** any tenant API or
isolation model — deployment design left to the implementer.

**Design:** tenant = one or more logical openEHR *systems*: a `tenant`
table; `tenant_id` on the scoping roots (`ehr`, the definition stores,
stored queries, sp_* config); **PG RLS with FORCE** keyed on a session
tenant context (the B7 schema was shaped for this: discriminator columns +
role architecture already in place); tenant-scoped tokens (claim → session
context); per-tenant `system_id` so version identity remains spec-correct
per tenant; per-tenant ECC verification in the conformance suite. This is
the HIP "fully integrated" tier, with engine-enforced isolation rather
than app-only filtering.

### 2.4 Binary storage: DV_MULTIMEDIA externalization (S3-compatible)

**Spec basis:** RM DV_MULTIMEDIA models external storage first-class:
`uri` vs inline `data` with invariant `is_inline or is_external`;
`integrity_check`/`integrity_check_algorithm` (+ their invariants) and
mandatory unencoded `size` exist to validate externally-held content;
RM ehr_extract even sanctions non-resolvable external URIs.
**Spec-silent:** server-side blob storage/fetch — our design.

**Design:** payloads above a threshold are content-addressed (sha-256 →
`integrity_check`, algorithm coded per the openEHR code set) into an
S3-compatible object store (`object_store` crate: S3/GCS/Azure-compatible);
**dev/test/compose runs use SeaweedFS** (github.com/seaweedfs/seaweedfs — 
open-source S3-compatible store, owner decision 2026-07-10): a `seaweedfs`
service in docker-compose + a testcontainers fixture for integration tests,
so the externalization path is exercised in CI without a cloud dependency;
production points the same client at any S3 endpoint. The
stored canonical JSON carries `uri` (+ integrity fields + `size`), the
wire can serve either form (inline on demand via transparent fetch,
honouring the invariant both directions). Version indelibility extends to
blobs: content-addressing makes them naturally immutable + deduplicated;
admin physical delete cascades to unreferenced blobs.

---

## 3. Sequencing (the enterprise stage, after P20/P99)

1. **E1 — Eventing** (outbox + AMQP + filters): smallest, unlocks the
   outbound half of every connector; the schema change is one outbox table.
2. **E2 — Multi-tenancy** (flagship): tenant model + RLS + tenant tokens +
   per-tenant ECC.
3. **E3 — FHIR connectors + façade** (inbound mapping first, then the read
   façade on AQL; HL7v2 after, same seam).
4. **E4 — S3 multimedia** (independent; can interleave).
5. **E5 — K8s deployment artifacts** (Helm chart encoding the ADR-013
   security posture; can ship any time).
Each lands behind the standing gates (workspace green + full ECC zero
drift) with its own ADR where a design choice is spec-silent.

## 4. What we deliberately do NOT build

- **The UI suite** (dashboards, template/explorer/AQL-editor/user-management
  UIs, forms, patient viewer): headless product, API-first substitutes
  listed in §1. A UI is a separate product decision, never a CDR-core one.
- **A full FHIR server**: connectors + façade only (owner decision) — the
  openEHR CDR is the system of record, FHIR is a boundary language.
- **A proprietary "transaction compensation" API**: openEHR's
  contribution-atomic, indelible, logically-deleted version model already
  carries the semantics; we document the mapping instead of cloning the
  checkbox.
- **SLA/consulting line items**: business offerings, not code.

## 5. Our differentiators (the other column of the comparison)

Newer spec pins than the market (REST 1.0.3 / AQL 1.1 / RM 1.2); **measured**
conformance (ECC 341/315/0, CORE+STANDARD PASS, machine-generated
Statement/Certificate — competitors show checkmarks, we ship artifacts);
`ALL_VERSIONS`; EHR Extract + TDD import + Subject Proxy (SM services others
don't surface); ABAC via Cedar with a remote-PDP mode; ATNA both tiers;
enterprise-grade PG18-native schema with documented security posture
(ADR-013); single static binary, pure Rust, no JVM.
