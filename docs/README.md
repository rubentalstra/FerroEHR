# docs/ — map of the documentation tree

A pure-Rust, openEHR-spec-conformant CDR (ITS-REST 1.0.3 + AQL 1.1) with
greenfield PG18-native internals. This file says what lives where and which
document is authoritative for what. When two docs disagree, the **newer
decision wins** (ADR-011 > ADR-010 > … ; the blueprint > the historical plan).

## Start here (the roadmap)

| Path | What it is | Authoritative for |
|---|---|---|
| **`blueprint/00-THE-BLUEPRINT.md`** | The single driving document: mission, the consolidated gap ledger (§2 — proven foundations + ECC breakdown + spec-area map), the numbered build order (B1–B6), standing rules | the trajectory, priorities, **and the whole spec-gap surface** ("what's the state, what's next") |
| `blueprint/01-rm.md … 07-cnf.md` | Per-component chapters: spec-cited requirements → verified state → ordered remaining work | the per-component compliance detail |
| `plans/current-phase.md` | Live pointer to the active work | what's active right now |
| `PROGRESS.md` | One row per phase + rebuild checkpoints | the historical record of what shipped |

## Decisions

- `ADRs/` — architecture decision records. The load-bearing current set:
  **ADR-004** (spec layer generated from BMM), **ADR-005** (ITS XML/REST
  generated), **ADR-006** (app = idiomatic Rust on the generated crates),
  **ADR-008** (greenfield PG18 storage + AQL; openEHR CNF conformance is the
  acceptance target — *read first for internals*), **ADR-010** (SM-aligned
  service architecture), **ADR-011** (app-crate redesign — the current
  three-crate app layout + protocol-free SM native API). Later records:
  **ADR-012** (closed archetype validation), **ADR-013** (enterprise schema
  baseline + PG18 operational-practices appendix), **ADR-014** (contribution-outbox
  eventing), **ADR-015** (multi-tenancy), **ADR-016** (FHIR connectors),
  **ADR-017** (multimedia externalization). ADR-001/002/003/007/009 are
  superseded-in-part or narrow; each says so in its header.

## The specs (the oracle — do not edit)

- **`specs/openehr/`** — the vendored normative openEHR spec text + the CNF
  Platform Conformance Test Schedule. **The conformance oracle.** Never
  hand-edit; refreshed only by `scripts/vendor-spec-docs.sh`. Use `/spec-lookup`
  / `/spec-audit`. Pins recorded in `VERSIONS.md`.

## Plans

- `plans/` — `current-phase.md` (live pointer) + the future phase files:
  `phase-20-optimization.md`, `phase-99-cutover.md`, the retained
  `phase-17..19` (task detail; scope absorbed into the blueprint arc), and
  `sm-phase-04-terminology-admin.md` (last SM phase file, header still
  `in-progress` though its work shipped via B3/B4). Completed phase files were
  pruned (2026-07-09 for 00–16/SM/S2; 2026-07-11 for B1–B8/E1–E5) — all in git
  history + `PROGRESS.md`. See `plans/README.md`.

## Design

The SM Platform Service Model design set was retired 2026-07-11 (SM-1..SM-6
shipped); the SM component map now lives in `architecture.md` and the vendored
SM spec at `specs/openehr/SM/`. The remaining design docs:

- `design/aql-engine.md` — the AQL engine feature envelope + IR→SQL design.
- `design/observability.md` — telemetry/metrics/health stack.
- `design/version-signing.md` — VERSION.signature (`ehrbase::signing` module).
- `design/container-images.md` — the GHCR images + compose quickstart.
- `design/benchmarking.md` — the benchmark harness (`tools/benchmark`).
- `design/conformance-framework.md` — the ECC (own conformance framework) design.
- `design/terminology-server-integration.md` — which self-hostable terminology
  server to run in Docker (Snowstorm / HAPI FHIR) and how to point the CDR + the
  conformance runner at it by URL. Built at B4.

## Enterprise (live behaviour docs)

- `enterprise/atna-audit.md` — the IHE ATNA audit trail (`ehrbase::system_log`
  module; op-id classification + middleware in `ehrbase-rest`).
- `enterprise/access-control.md` — RBAC/ABAC (`ehrbase-rest::access` module).
- `enterprise/deployment.md` — the Kubernetes Helm chart + operational posture
  (ADR-013 roles + Appendix §3/§5/§6). Built at E5.
- `enterprise/product-roadmap.md` — the market scorecard + enterprise-capability
  roadmap (B8).

## Spec compliance detail

- The whole-codebase spec-gap surface is the **blueprint §2**
  (`blueprint/00-THE-BLUEPRINT.md`) — the consolidated view that superseded the
  standalone per-finding spec-audit ledger (retired 2026-07-11); the
  per-component compliance detail is `blueprint/01-rm.md … 07-cnf.md`.
- `terminology-validation.md` — the external FHIR-R4 terminology-*client* design
  (how the CDR validates coded values against a terminology server); pairs with
  `design/terminology-server-integration.md` (which server to run). Built at B4.

## Platform + verification

- `VERSIONS.md` — the single source of truth for every version pin (language,
  PG, openEHR spec components) except third-party Rust crates (root `Cargo.toml`
  wins for those).
- `postgres-features.md` — the PG 17/18 feature delta this CDR exploits, mapped
  to phases.
- `conformance/` — ECC run artifacts, one directory per SUT (`ehrbase-rs/`,
  `ehrbase-java/`, …): `CONFORMANCE_REPORT.md`, `CONFORMANCE_STATEMENT.md`,
  `CONFORMANCE_CERTIFICATE.md`, `results.json`, badges; the SUT-independent
  `CATALOG.md` at the root. Current (ehrbase-rs, W-10 re-derived baseline):
  **369 executed · 334 passed · 0 failed · 35 adjudicated skips — CORE PASS /
  STANDARD PASS** (ratcheted by ECC-TPL-017, the example round-trip case).
- `benchmarks/` — `REPORT.md` + `results.json` from the benchmark harness.
