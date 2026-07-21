# EHRbase-rs — product roadmap

*Last revised: 2026-07-20. This file is **direction and themes only** — it
carries no item-level state and no quotable numbers, because both go stale.
The live work tracker is GitHub Issues (`gh issue list --state open`; pinned
issues = current focus; milestones = releases — root `CLAUDE.md` §Issue
workflow). The build record is the closed issues + PR descriptions +
`CHANGELOG.md` + git history. Every measured claim lives in the generated
artifacts: conformance in `docs/conformance/`, benchmarks in
`docs/benchmarks/` — always cite those, never this file.*

## What the product is

A pure-Rust, headless, API-first openEHR CDR on PostgreSQL 18: ITS-REST
1.1.0 at the API, AQL 1.1 as the query language, RM 1.2.0 as the domain
model — the spec layer generated from the official machine-readable specs,
the application our own design on top (`docs/architecture.md`). Conformance
is machine-verified per release by the built-in ECC runner; performance is
measured against upstream EHRbase (Java) on identical workloads with both
directions always published. Shipped surface: the full platform (EHR /
COMPOSITION / DIRECTORY / CONTRIBUTION, versioning, templates + validation,
WebTemplate + FLAT/STRUCTURED, ADL 1.4 + 2.4, EHR Extract + TDD,
demographics, terminology, admin), enterprise capabilities (change events,
FHIR R4 connectors, S3 multimedia, RBAC/ABAC, multi-tenancy, ATNA audit,
Helm/distroless, observability), and the Leptos admin console as a third
container image.

## Direction (durable themes)

- **Conformance depth** — the ECC baseline only ratchets upward; skips go
  to zero (executed or cited-N/A); spec updates are watched continuously,
  never just at pin bumps; every spec-facing behaviour traces to the
  vendored spec text.
- **Honest publication** — the website derives every conformance and
  benchmark claim from committed runner artifacts (hand-typed numbers are
  a CI failure, not a style issue); the public comparison against upstream
  publishes wins and losses alike.
- **Performance** — profile-first, one change per ladder, re-measured per
  release; the PG18-native headroom (AIO tuning, pipelined hot reads,
  `JSON_TABLE` codegen, speculative indexes) is spent only where a profile
  demands it.
- **The admin console** — a real design system and 100% feature
  completeness over strictly ITS-REST; the CDR stays headless, the console
  stays a client, never a bypass.
- **Interop** — FHIR first-class today; HL7v2 connectors behind the same
  integration-frame seam as the named next posture; FLAT/STRUCTURED
  verified table-by-table against the SDF spec.
- **Operational maturity** — HA/scale-out guidance, PITR/backup drills,
  upgrade rehearsals, cache-tier re-evaluation for multi-instance
  deployments.
- **Stage-2/3 enterprise archaeology** — remaining `reference/v1`
  capabilities land only with spec grounding or an explicit
  our-own-design flag.

## Standing rules (apply to everything above)

Vendored specs are the oracle; ECC zero-drift gates every phase; the
conformance baseline only ratchets upward; comparisons publish both
directions; a claim without a committed measurement or citation does not
ship.
