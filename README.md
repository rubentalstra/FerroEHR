<div align="center">

# EHRbase-rs

**A pure-Rust openEHR Clinical Data Repository — spec-compliant, measured, and built for production.**

[![CI](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/ci.yml)
[![ECC conformance](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Frubentalstra%2Fehrbase-rs%2Fdevelop%2Fdocs%2Fconformance%2Fbadge.json)](docs/conformance/CONFORMANCE_REPORT.md)
[![CORE profile](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Frubentalstra%2Fehrbase-rs%2Fdevelop%2Fdocs%2Fconformance%2Fbadge-core.json)](docs/conformance/CONFORMANCE_CERTIFICATE.md)
[![STANDARD profile](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Frubentalstra%2Fehrbase-rs%2Fdevelop%2Fdocs%2Fconformance%2Fbadge-standard.json)](docs/conformance/CONFORMANCE_CERTIFICATE.md)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

[Quick start](#quick-start) · [Features](#features) · [Conformance](#conformance-measured-not-asserted) · [Deployment](#deployment) · [Documentation](#documentation)

</div>

---

EHRbase-rs is a headless, API-first Clinical Data Repository implementing
the [openEHR](https://openehr.org) specifications. It ships as a single
static binary on PostgreSQL 18 — no JVM, no runtime dependencies — and every
compliance claim it makes is **machine-verified**: each release runs the
full conformance catalogue against the live server and generates its own
Conformance Statement and Certificate.

## Why EHRbase-rs

- **Compliance you can verify, not just read.** The built-in conformance
  runner executes the complete catalogue (341 cases, JSON and XML) and
  computes the openEHR profile verdicts — currently **CORE: PASS ·
  STANDARD: PASS · OPTIONS: OBTAINED**, zero failing cases.
- **The latest openEHR specifications**, generated from the official
  machine-readable models: REST API 1.0.3, AQL 1.1, RM 1.2.0, Archetype
  Model 1.4 + 2.4, Terminology 3.1. A specification update is a
  regeneration, not a rewrite.
- **One static Rust binary.** Predictable memory, fast cold starts, a
  minimal distroless container image, no garbage-collection pauses in the
  write path.
- **PostgreSQL 18-native clinical storage.** Temporal versioning with
  database-enforced non-overlap, time-ordered UUIDv7 keys, and canonical
  openEHR JSON stored verbatim — what you store is exactly what the API
  serves.

## Features

### openEHR platform

- **REST API (ITS-REST 1.0.3)** — EHR, EHR_STATUS, COMPOSITION, DIRECTORY,
  CONTRIBUTION, query, template, and admin resources, with canonical JSON
  *and* XML on the wire
- **AQL 1.1 engine** — including `ALL_VERSIONS`, terminology-backed
  `TERMINOLOGY()` expansion inside `matches`, and stored parameterised
  queries
- **Full versioning semantics** — contribution-atomic commits, indelible
  version history, logical delete, attestations, digital signatures,
  point-in-time reads
- **Templates & validation** — OPT 1.4 ingestion with artefact validity
  checking, WebTemplate, FLAT and STRUCTURED formats, deep
  archetype-constraint validation
- **EHR Extract & messaging** — whole-EHR export/import with preserved
  distributed version identity, EHR cloning across systems, TDD import
- **Demographics** — a versioned party store (person, organisation, group,
  agent, role) with relationships
- **Terminology** — the bundled openEHR terminology plus pluggable external
  FHIR terminology servers (validate, expand, subsume)

### Integration

- **Change events** — a transactional outbox publishes every commit to
  AMQP/RabbitMQ with per-EHR ordering, filterable server-side
  subscriptions, and PHI-free payloads by default
- **FHIR R4 connectors** — bidirectional and mapping-driven: ingest FHIR
  resources as validated compositions with full provenance, expose
  committed data through a FHIR read façade, emit FHIR resources on change
- **Binary & object storage** — large multimedia is content-addressed into
  any S3-compatible store with cryptographic integrity verification;
  SeaweedFS works out of the box for self-hosted setups

### Security & operations

- **Authentication** — Basic and OAuth2/OIDC (Keycloak, Active Directory,
  any standards-compliant identity provider)
- **Authorization** — role-based access control plus attribute-based
  policies, via the embedded policy engine or an external policy decision
  point
- **Multi-tenancy** — fully integrated: each tenant is an isolated logical
  openEHR system, enforced by PostgreSQL row-level security
- **ATNA audit logging** — IHE ATNA-compliant system log (DICOM audit
  messages over TLS syslog), alongside the openEHR contribution audit trail
- **Hardened by default** — layered database roles, TLS everywhere, and
  built-in observability: Prometheus metrics, OpenTelemetry traces, health
  probes

### Deployment

- **Docker Compose** for development and evaluation
- **Helm chart** with security-hardened defaults (non-root, read-only
  filesystem, network policies) for Kubernetes
- **Operations guide** covering database roles, backup/PITR, and upgrades

## Quick start

Run the full stack (server + PostgreSQL 18) with Docker:

```bash
docker compose up -d
```

The API is served at `http://localhost:8080/ehrbase/rest/openehr/v1`
(default development credentials `ehrbase` / `ehrbase`), with interactive
OpenAPI documentation at `http://localhost:8080/ehrbase/swagger-ui`.

Create your first EHR:

```bash
curl -u ehrbase:ehrbase -X POST \
  http://localhost:8080/ehrbase/rest/openehr/v1/ehr \
  -H 'Prefer: return=representation'
```

## Conformance, measured not asserted

```bash
scripts/conformance.sh
```

One command builds the current sources into a container, runs the complete
openEHR conformance catalogue against it in both wire formats, and writes
the [report](docs/conformance/CONFORMANCE_REPORT.md), the
[Conformance Statement](docs/conformance/CONFORMANCE_STATEMENT.md), and the
[Certificate](docs/conformance/CONFORMANCE_CERTIFICATE.md). Profile
verdicts are computed by the runner — never hand-asserted.

## Deployment

```bash
helm install ehrbase-rs deploy/helm/ehrbase-rs \
  --set database.existingSecret=my-db-secret
```

See the [deployment guide](docs/enterprise/deployment.md) for the
production checklist: database role separation, TLS, backup and
point-in-time recovery, and audit logging.

## Documentation

| | |
|---|---|
| [Architecture](docs/architecture.md) | How the system is built, and why |
| [Conformance report](docs/conformance/CONFORMANCE_REPORT.md) | The latest measured results, per test case |
| [Deployment guide](docs/enterprise/deployment.md) | Production operations |
| [Product roadmap](docs/enterprise/product-roadmap.md) | The capability matrix and what's next |
| [Developer documentation](docs/README.md) | Contributing, design decisions, specifications |

## Built with

Rust (edition 2024) · PostgreSQL 18 · axum · sqlx — and the openEHR
specifications themselves, vendored and code-generated into the server's
type system.

## License

Apache License 2.0 — see [LICENSE](LICENSE).
