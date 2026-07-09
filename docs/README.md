# docs/ — map of the documentation tree

A pure-Rust, openEHR-spec-conformant CDR (ITS-REST 1.0.3 + AQL 1.1) with
greenfield PG18-native internals. This file says what lives where and which
document is authoritative for what. When two docs disagree, the **newer
decision wins** (ADR-011 > ADR-010 > … ; the blueprint > the historical plan).

## Start here (the roadmap)

| Path | What it is | Authoritative for |
|---|---|---|
| **`blueprint/00-THE-BLUEPRINT.md`** | Where the project is going and why | the trajectory / priorities |
| `plans/current-phase.md` | Live pointer to the active work | what's active right now |
| `GAP_REGISTER.md` | Consolidated spec-gap surface (proven vs known-missing), prioritized | "what's the state, what's next" |
| `PROGRESS.md` | One row per phase + rebuild checkpoints | the historical record of what shipped |

## Decisions

- `ADRs/` — architecture decision records. The load-bearing current set:
  **ADR-004** (spec layer generated from BMM), **ADR-005** (ITS XML/REST
  generated), **ADR-006** (app = idiomatic Rust on the generated crates),
  **ADR-008** (greenfield PG18 storage + AQL; openEHR CNF conformance is the
  acceptance target — *read first for internals*), **ADR-010** (SM-aligned
  service architecture), **ADR-011** (app-crate redesign — the current
  three-crate app layout + protocol-free SM native API). ADR-001/002/003/007/009
  are superseded-in-part or narrow; each says so in its header.

## The specs (the oracle — do not edit)

- **`specs/openehr/`** — the vendored normative openEHR spec text + the CNF
  Platform Conformance Test Schedule. **The conformance oracle.** Never
  hand-edit; refreshed only by `scripts/vendor-spec-docs.sh`. Use `/spec-lookup`
  / `/spec-audit`. Pins recorded in `VERSIONS.md`.

## Plans

- `plans/` — `current-phase.md` (live pointer) + the active/future phase files:
  `phase-17..20`, `phase-99` (remaining Stage-1 P-phases) and
  `sm-phase-04-terminology-admin.md` (active SM phase, carrying the ADR-011
  rebuild). Completed phase files were pruned 2026-07-09 (in git history +
  `PROGRESS.md`). See `plans/README.md`.

## Design

- `design/sm-platform/` — the SM Platform Service Model design set (spec
  digests 01–06 + gap analysis 07 + target architecture 08 + roadmap 09 +
  message integration 10). Load-bearing spec extractions; packaging reconciled
  to ADR-011 in its `README.md` banner.
- `design/aql-engine.md` — the AQL engine feature envelope + IR→SQL design.
- `design/observability.md` — telemetry/metrics/health stack.
- `design/version-signing.md` — VERSION.signature (`ehrbase::signing` module).
- `design/container-images.md` — the GHCR images + compose quickstart.
- `design/benchmarking.md` — the benchmark harness (`tools/benchmark`).
- `design/conformance-framework.md` + `design/ecc-coverage-review.md` — the ECC
  (own conformance framework) design + coverage review.

## Enterprise (live behaviour docs)

- `enterprise/atna-audit.md` — the IHE ATNA audit trail (`ehrbase::system_log`
  module; op-id classification + middleware in `ehrbase-rest`).
- `enterprise/access-control.md` — RBAC/ABAC (`ehrbase-rest::access` module).
- `enterprise/v1-vs-v2-delta.md` — the EHRbase v1→v2 archaeology (Stage-2 input).

## Spec compliance detail

- `spec-audit/SPEC_AUDIT.md` + `spec-audit/findings/*.md` — the 2026-07-06
  whole-codebase audit; the **per-finding** detail record (82 open / 109 fixed),
  each with spec citations + checkboxes. The consolidated view is
  `GAP_REGISTER.md`.
- `terminology-validation.md` — terminology binding + validation notes.

## Platform + verification

- `VERSIONS.md` — the single source of truth for every version pin (language,
  PG, openEHR spec components) except third-party Rust crates (root `Cargo.toml`
  wins for those).
- `postgres-features.md` — the PG 17/18 feature delta this CDR exploits, mapped
  to phases.
- `conformance/` — ECC run artifacts: `CONFORMANCE_REPORT.md`, `CATALOG.md`,
  `COVERAGE_GAPS.md`, `results.json`, badges (ECC is suspended during the
  ADR-011 rebuild and re-converges at P19).
- `benchmarks/` — `REPORT.md` + `results.json` from the benchmark harness.
