# Extensions — the spec-homeless surface (W-3f)

Owner directive 2026-07-12: W-3f is the spec-first redesign of the `ehrbase`
platform crate. Registers 01–11 own everything with an openEHR-spec home; this
document (register 12, "Extensions") is the **inverse mapping** — the modules
that have **no openEHR spec home** and therefore must be quarantined, flagged,
and gated as deliberate local design/extensions, plus the two candidates that
turn out to be spec-governed and must be **reassigned** back to a spec register.

This is a READ-ONLY audit. No code is changed here. Every candidate is verified
against the code (file:line) and against the only three openEHR chapters that
touch integration / deployment / security-of-access concerns before anything is
declared spec-homeless.

**Spec oracle** (read before quarantining anything — these chapters *mention*
integration/eventing/deployment/access, so each candidate is checked against
them first):

- `docs/specs/openehr/BASE/docs/architecture_overview/master14-integration.adoc`
  — "Integrating openEHR with other Systems": the **integration-archetype**
  strategy (`GENERIC_ENTRY` + `FEEDER_AUDIT`, mapping *integration* archetypes →
  *designed* archetypes). This is the closest the specs come to a "connector",
  and it governs the **data-conversion model**, not any transport or resource
  format.
- `docs/specs/openehr/BASE/docs/architecture_overview/master13-deployment.adoc`
  — the 5-tier architecture (persistence / back-end services / virtual EHR /
  application logic / presentation). Informative; prescribes no eventing,
  tenancy, or object-storage mechanism.
- `docs/specs/openehr/BASE/docs/architecture_overview/master07-security.adoc`
  — §Access Control (`EHR_ACCESS` gateway), §Anonymity, §Access logging
  ("openEHR does not specify models of such logs"). Governs the `EHR_ACCESS`
  *object*, not any cache of it.
- Cross-checked: `docs/spec-audit/architecture-overview/CHECKLIST.md` rows
  §3.3 Deployment Environments (139–140), Integration IM (389–394), Security
  overview (695–696: "authn is deployment-level"), FEEDER_AUDIT (492–498).

**The register rule** (owner hard rule 2026-07-11): code, doc comments, and
PORT NOTEs justify behaviour by citing the **openEHR spec file + section**, or
— where the spec is silent — carry the explicit flag *"no openEHR spec governs
this — our own design/extension"*. Never an ADR number. This register makes
that flag load-bearing.

**Fixed constraints** (not redesigned here): the `Platform` trait catalogue in
`app/ehrbase-sm` is FIXED; the DB schema is SETTLED; enterprise/RBAC is Stage 2
— tenancy is only *quarantined as it exists*, never designed further.

---

## 1. What "extension" means for this crate

An extension is a feature the `ehrbase` binary offers that **no openEHR
specification requires or describes**. Three chapters were checked as the last
possible spec home:

- **master14 does not rescue the FHIR connector or the AMQP eventing.** Its
  integration model is archetype-to-archetype data conversion through
  `GENERIC_ENTRY` + `FEEDER_AUDIT` (lines 45–74). It says nothing about FHIR
  resources, message brokers, topic routing, or outbound emission. Our FHIR
  connector maps directly to *designed* templates (mapping-as-data), not via
  `GENERIC_ENTRY` integration archetypes — a different, spec-silent mechanism.
- **master13 is informative deployment guidance**; it prescribes no eventing,
  multi-tenancy, or blob-offload mechanism.
- **master07 governs the `EHR_ACCESS` object and authn-at-deployment**, not a
  cache, a broker, or a tenant registry.

Everything below is therefore spec-homeless **except** two candidates that the
oracle *does* claim (`codes.rs`, `ehr_access_cache.rs`) — those are reassigned.

Each quarantined extension already obeys the two non-negotiables and must keep
them after the move:

1. **Flag header** — a module doc comment carrying *"no openEHR spec governs
   this — our own design/extension"* (all six already do; verified below).
2. **Config gate, off by default** — the feature does nothing unless a
   `figment` config explicitly enables it; with it off the commit/read paths
   are byte-identical to the no-extension behaviour (the zero-drift gate).

---

## 2. Candidates audited (file:line evidence)

| Module | Files / lines | What it does (verified) | Flag present? |
|---|---|---|---|
| `src/events/` | `amqp.rs` 130, `config.rs` 176, `mod.rs` 214, `publisher.rs` 356 | Contribution-outbox eventing: a background drainer reads the transactional `event_outbox` (written by `service::vobject` in the commit tx) and publishes PHI-free envelopes to an AMQP broker (`events/mod.rs:1-24`, `EventPublisher` seam `mod.rs:54-70`, `AmqpPublisher` lapin impl). Off by default (`EventsConfig::enabled`, `config.rs:38,72`). | yes (spec-silent by nature) |
| `src/fhir_outbound/` | `config.rs` 162, `mod.rs` 31, `publisher.rs` 333 | FHIR **outbound** emitter: a second drainer walks committed outbox rows via its own cursor, reverse-maps matching COMPOSITIONs to FHIR JSON, and publishes clinical (PHI) resources to a separate exchange. Off by default (`fhir_outbound/config.rs:47,75`; `mod.rs:1-31`). Reuses the `events` broker seam. | yes |
| `src/service/fhir/` | `mapping.rs` 991, `mod.rs` 590 | FHIR **connector**: the `fhir_mapping` mapping-as-data store (CRUD) + inbound ingest (`FhirConnectorAdapter::fhir_ingest`) that builds a COMPOSITION from a mapping, stamps `FEEDER_AUDIT` provenance, and commits through the validated create path (`service/fhir/mod.rs:1-19`). Self-flagged "our own extension — no openEHR spec governs this; E3". | yes |
| `src/multimedia/` | `config.rs` 136, `mod.rs` 139, `offload.rs` 418, `store.rs` 198 | `DV_MULTIMEDIA` externalization to S3-compatible object storage: on commit, inline `data` over a threshold is written to a content-addressed blob store (SHA-256 key) and the JSON rewritten to a `uri`; re-inlined on read. Off by default (`multimedia/mod.rs:1-17,72-80`). Self-flagged "Server-side blob storage is spec-silent — this module fills it". | yes |
| `src/service/tenant.rs` | 237 | Multi-tenancy registry: CRUD over the `tenant` table + claim/header → `TenantContext` resolution; `TenantAdapter` on `EhrbaseService`. Self-flagged "the tenancy model is spec-silent" (`tenant.rs:1-16`). | yes |
| `src/service/event_subscription.rs` | 215 | Event-filter subscription store (`kind`/`change_type`/`template_id`/`archetype` predicate → AMQP topic binding). `EventSubscriptionAdapter` extension, "spec-silent" (`event_subscription.rs:1-11`). Part of the `events` extension. | yes |
| `src/service/ehr_access_cache.rs` | 68 | **REASSIGN.** A `moka` per-EHR cache of parsed `EHR_ACCESS` scheme settings, consulted on every EHR-scoped request. The *cache* is spec-silent, but the object it caches — `EHR_ACCESS` — is RM (`ehr_access_cache.rs:1-14`). | n/a — reassigned |
| `src/service/codes.rs` | 168 | **REASSIGN.** Change-control terminology codes: `audit_change_type` and `version_lifecycle_state` group membership + rubric resolution (`codes.rs:1-11,53-105`). Fully spec-governed — `AUDIT_DETAILS.Change_type_valid`, `ORIGINAL_VERSION.Lifecycle_state_valid`, TERM bundle. | n/a — reassigned |

---

## 3. G-row register

Disposition vocabulary: **quarantine** (move to `extensions/`, keep the
spec-silent flag) · **reassign-NN** (spec-governed — belongs to register NN) ·
**delete** (dead/duplicative) · **PORT NOTE** (residue action).

| id | Module | Spec citation / spec-silent flag | Severity | Disposition |
|---|---|---|---|---|
| G-12-01 | `events/` (AMQP publisher, 876 L) | spec-silent — master14 governs archetype data-conversion, not brokers; master13 informative | med | **quarantine** → `extensions/events/` |
| G-12-02 | `service/event_subscription.rs` (215 L) | spec-silent — subscription predicates are our own model | low | **quarantine** → `extensions/events/subscription.rs` |
| G-12-03 | `fhir_outbound/` (526 L) | spec-silent — no openEHR outbound/FHIR transport | med | **quarantine** → `extensions/fhir/outbound.rs` (+ config) |
| G-12-04 | `service/fhir/` connector (1581 L) | spec-silent mapping; `FEEDER_AUDIT` stamping reuses RM common `FEEDER_AUDIT_DETAILS` (register 01) | high | **quarantine** → `extensions/fhir/`; **split** `mapping.rs` (991 > 700) |
| G-12-05 | `multimedia/` (891 L) | spec-silent blob storage; the `DV_MULTIMEDIA` shape it rewrites is RM 1.2.0 data types (register 01) | med | **quarantine** → `extensions/multimedia/` |
| G-12-06 | `service/tenant.rs` (237 L) | spec-silent; Stage-2-adjacent (multi-tenancy) — quarantine only, do not extend | low | **quarantine** → `extensions/tenancy/` |
| G-12-07 | `service/ehr_access_cache.rs` (68 L) | `EHR_ACCESS` is RM — master07 §Access Control; RM `ehr/ehr_access` | low | **reassign-01** (RM EHR) — stays service-internal perf infra, not an extension |
| G-12-08 | `service/codes.rs` (168 L) | RM `AUDIT_DETAILS.Change_type_valid`, `ORIGINAL_VERSION.Lifecycle_state_valid`; TERM bundle groups | low | **reassign-01/02** (RM change-control + TERM) — core spec support, not an extension |

**Counts by disposition:** quarantine **6** (G-12-01..06) · reassign **2**
(G-12-07 → register 01; G-12-08 → registers 01/02) · delete **0**.

Every candidate is live, wired in `main.rs`, and config-gated — nothing is dead
or duplicative, so there is no delete-candidate.

---

## 4. Target design — `app/ehrbase/src/extensions/`

Consolidate the six quarantined modules under one `extensions/` tree so the
spec-homeless surface is visible in one place (today it is scattered: three
top-level modules `events`/`fhir_outbound`/`multimedia` plus three buried under
`service/`). Every file ≤ ~700 lines. `codes.rs` and `ehr_access_cache.rs` stay
where they are (§5).

```
app/ehrbase/src/extensions/
  mod.rs            -- flag header; re-exports; the config-gate wiring contract (§6)
  events/           -- G-12-01, G-12-02  (from src/events/ + service/event_subscription.rs)
    mod.rs            -- EventPublisher seam, routing/binding-key helpers
    config.rs         -- figment EventsConfig (enabled=false default)
    amqp.rs           -- lapin AmqpPublisher
    publisher.rs      -- outbox drainer + retention pruner + EventsHandle (356 L, ok)
    subscription.rs   -- event_subscription CRUD + EventSubscriptionAdapter (215 L, ok)
  fhir/             -- G-12-03, G-12-04  (from service/fhir/ + fhir_outbound/)
    mod.rs            -- mapping store CRUD + FhirConnectorAdapter::fhir_ingest
    mapping.rs        -- pure FHIR->COMPOSITION transform  (SPLIT: ≤700; see below)
    feeder_audit.rs   -- the FEEDER_AUDIT builder split out of mapping.rs (RM-typed)
    outbound.rs       -- outbound drainer + FhirOutboundHandle (from fhir_outbound/publisher.rs, 333 L)
    config.rs         -- FhirOutboundConfig (+ the connector's own gate)
  multimedia/       -- G-12-05  (from src/multimedia/, unchanged layout)
    mod.rs            -- MultimediaEngine (offload/expand/referenced_keys)
    config.rs         -- MultimediaConfig (enabled=false default)
    offload.rs        -- plan/apply offload+expand (418 L, ok)
    store.rs          -- content-addressed object_store BlobStore (198 L, ok)
  tenancy/          -- G-12-06  (from service/tenant.rs) — quarantine only, Stage-2-adjacent
    mod.rs            -- tenant registry CRUD + TenantAdapter (237 L, ok)
```

**Required file split (G-12-04):** `service/fhir/mapping.rs` is 991 lines — over
the ≤700 target. Split the `FEEDER_AUDIT` builder (`feeder_audit`,
`inject_feeder_audit`, `mapping.rs:394-424` — RM-typed, register-01 code) into
`fhir/feeder_audit.rs`, leaving the resource→COMPOSITION field mapping in
`fhir/mapping.rs`. `service/fhir/mod.rs` (590 L) is under the limit; it becomes
`extensions/fhir/mod.rs` as-is.

Each submodule keeps a **flag header** (module doc comment) restating *"no
openEHR spec governs this — our own design/extension"* and pointing at its
design doc (`docs/design/ehr-access-scheme.md` etc. where one exists).

---

## 5. The two reassignments (NOT extensions)

- **G-12-07 `ehr_access_cache.rs` → register 01 (RM EHR).** `EHR_ACCESS` is the
  spec's access-control gateway (master07 §Access Control lines 249–264; RM
  `ehr/ehr_access`). The cache is a spec-silent *performance mechanic* for a
  spec-governed object — exactly analogous to `openehr_flat::cache::WebTemplateCache`,
  which lives with templates, not in an "extensions" bucket. It stays
  service-internal (`pub(super)`), keeps its spec-silent flag on the *caching*,
  and is owned by the EHR/register-01 redesign — do **not** move it to
  `extensions/`.
- **G-12-08 `codes.rs` → registers 01 + 02 (RM change-control + TERM).** This is
  core spec support: numeric group-code membership and rubric resolution for
  `audit_change_type` / `version_lifecycle_state` against the `openehr-term`
  bundle (findings F-06-02/04/06, F-11-01 cited in-file). It is consumed by the
  versioning/contribution write paths and belongs with them, never in
  `extensions/`.

---

## 6. Wiring rule (main.rs / lib.rs gating)

The wiring contract, verified against `main.rs:110-240`, is preserved by the
move:

- `lib.rs` declares a single `pub mod extensions;` (replacing the three
  scattered `pub mod events; pub mod fhir_outbound; pub mod multimedia;` at
  `lib.rs:11-13`).
- **Each extension is off by default and gated in `main.rs`** by loading its
  `figment` config and only then attaching/spawning:
  - `events` — `EventsConfig::load()`; spawn the drainer only if
    `events_config.enabled` (`main.rs:115-122`); a down broker never fails boot
    (the outbox buffers).
  - `multimedia` — `MultimediaEngine::from_config()` returns `None` when
    disabled; attached via `service.with_multimedia(...)` only when `Some`
    (`main.rs:208-219`).
  - `fhir` outbound — `FhirOutboundConfig::load()`; spawn only if `enabled`
    (`main.rs:229-239`). The connector's inbound/mapping routes are separately
    config-gated in `ehrbase-rest`.
  - `tenancy` — resolution middleware active only when configured; the
    `tenant` table is not RLS-scoped (`tenant.rs:1-16`).
- The rule for the redesign: **no extension may alter a commit/read result when
  its gate is off** (the zero-drift invariant already asserted by the
  multimedia and events module docs).

---

## 7. Seams to the service chapters — `TODO(w3f-integrate)` candidates

Moving these modules out of `service/` exposes seams that are currently
`pub(super)` service internals. Each is a `TODO(w3f-integrate)` for the
service-chapter redesigns to publish a stable in-crate seam:

1. **FHIR ingest → validated commit** (G-12-04): `fhir_ingest` reuses the
   `pub(super)` `EhrbaseService::create_composition` and the moka-cached
   `web_template_for` (`service/fhir/mod.rs:13-19` PORT NOTE). Relocating to
   `extensions/fhir/` requires a crate-internal validated-commit seam — the
   original rationale for burying FHIR under `service/` must be replaced by an
   explicit seam, not proximity.
2. **Eventing / FHIR-outbound → the `event_outbox`** (G-12-01/03): both
   drainers consume rows written by `service::vobject` in the CONTRIBUTION
   commit transaction, and outbound reverse-maps via the versioned read seam
   (`fhir_outbound/mod.rs:1-24`). The outbox write remains a
   versioning/contribution-chapter concern; the drain side is the extension.
3. **Multimedia → commit/read path** (G-12-05): `offload` runs on the commit
   path and `expand` on the read path (`?expand_multimedia=true`) inside
   `vobject`. The `with_multimedia` engine handle is the seam
   (`main.rs:210-218`).
4. **`ehr_access_cache` invalidation** (G-12-07): invalidated on every
   `EHR_ACCESS` commit (`ehr_access_cache.rs:57-61`) — a service-internal seam
   owned by register 01, not relocated.

---

## 8. PORT-NOTE residue

| PORT NOTE | Location | Action |
|---|---|---|
| AMQP topic-key sanitisation (dots in `template_id`) + per-version message routing | `events/mod.rs:80-90` | **keep** — valid spec-silent design |
| `event_subscription.archetype` absent from routing key | referenced `events/mod.rs:99-103`, `event_subscription.rs` | **keep** |
| "crate layout: FHIR lives inside `service` because ingest reuses `pub(super)` seams" | `service/fhir/mod.rs:13-19` | **re-verify / rewrite** — under W-3f the module moves to `extensions/fhir/`, so this rationale is superseded; replace with a `TODO(w3f-integrate)` naming the commit seam (§7 item 1) |
| DV_MULTIMEDIA spec-basis note ("server-side blob storage is spec-silent") | `multimedia/mod.rs:1-17` | **keep** — correct flag |
| tenancy "spec-silent" flag | `tenant.rs:1-16` | **keep**; add Stage-2 marker (no further design) |

No PORT NOTE is dropped; the only rewrite is the FHIR crate-layout note, which
becomes an integration TODO once the module relocates.
