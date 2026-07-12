# Beyond the core

The core of EHRbase-rs is the openEHR platform: EHRs, compositions,
contributions, templates, versioning, and AQL. Around that core the server
ships a set of optional capabilities for integrating with the wider systems
landscape — messaging, demographics, external terminology, change events,
FHIR, and large-object storage. This chapter set describes each one from the
operator's and integrator's point of view: what it does, how to turn it on, and
how to consume it.

> [!IMPORTANT]
> **Every capability in this section is off by default.** The bare server starts
> with all of them disabled, and its behaviour is byte-identical to a
> single-tenant, integration-free openEHR CDR until you explicitly enable one.
> Enabling any of them is a deliberate, auditable configuration decision — and
> some of them carry PHI, which each chapter calls out.

## The capability set

- **[EHR Extract & messaging](messaging.md)** — export and import whole EHRs,
  clone an EHR into another system while preserving its distributed version
  identity, and import Template Data Documents (TDDs) as compositions.
- **[Demographics](demographics.md)** — a versioned party store (persons,
  organisations, groups, agents, roles) with relationships, served over a REST
  surface that mirrors the EHR APIs.
- **[Terminology servers](terminology.md)** — the bundled openEHR terminology
  for local codes, plus pluggable external FHIR R4 terminology servers for
  validating and expanding coded values against external value sets.
- **[Subject Proxy](subject-proxy.md)** — read facts about a subject
  ("date of birth", "latest blood pressure") through named variables backed
  by data frames — AQL against the CDR, reads from configured external FHIR
  servers, or manual feeds — with a per-variable sample history and
  currency-based freshness. A service capability (no REST endpoints yet).
- **[Change events (AMQP)](amqp.md)** — a transactional outbox that publishes
  a PHI-free, at-least-once, per-EHR-ordered event for every commit to
  AMQP/RabbitMQ, so downstream systems can respond to changes.
- **[FHIR connectors](fhir.md)** — mapping-driven inbound ingestion of FHIR R4
  resources, a read façade that returns openEHR data as FHIR, and event-driven
  outbound emission of mapped FHIR resources.
- **[S3 multimedia](s3-multimedia.md)** — threshold-based, content-addressed
  offload of large `DV_MULTIMEDIA` blobs to any S3-compatible object store, with
  integrity verification and expand-on-read.

Security, multi-tenancy, and the audit trail are covered separately in
[Security & multi-tenancy](../security.md); running the server in production —
including the observability and health surfaces the integrations feed — is
covered in [Operations](../operations.md).
