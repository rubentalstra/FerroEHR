# docs/ — map of the documentation tree

A pure-Rust, openEHR-spec-conformant CDR (ITS-REST 1.0.3 + AQL 1.1) with
greenfield PG18-native internals. This file says what lives where and which
document is authoritative for what. When two docs disagree, the **newer
decision wins**. *(The former `blueprint/`, `design/`, `enterprise/`, and
`spec-audit/` layers were deleted 2026-07-16/17 — implemented or stale; the
vendored specs are the only doc oracle, and live behaviour is documented in
the code + the user website.)*

## Start here

| Path | What it is | Authoritative for |
|---|---|---|
| root `ROADMAP.md` | The forward product roadmap | where the product goes next |
| `plans/WORKLIST.md` | The single open-items tracker (one row per item) | what's open |
| `plans/current-phase.md` | Live pointer to the active work | what's active right now |
| `PROGRESS.md` | One row per phase + rebuild checkpoints | the historical record of what shipped |
| `endpoint-map.md` | Every endpoint traced route → dispatcher → service → SQL, plus the background loops | the navigation + optimization instrument |

## Decisions

- `ADRs/` — architecture decision records (decision *history* only — code
  cites specs, never ADRs). The load-bearing current set: **ADR-004** (spec
  layer generated from BMM), **ADR-005** (ITS XML/REST generated),
  **ADR-006** (app = idiomatic Rust on the generated crates), **ADR-008**
  (greenfield PG18 storage + AQL; openEHR CNF conformance is the acceptance
  target — *read first for internals*), **ADR-013** (enterprise schema
  baseline), **ADR-014** (contribution-outbox eventing), **ADR-015**
  (multi-tenancy), **ADR-016** (FHIR connectors), **ADR-017** (multimedia
  externalization), **ADR-018** (the three-crate application consolidation:
  `ehrbase` platform library + `ehrbase-rest` adapter + `ehrbase-server`
  binary; zero re-exports). Earlier records are superseded-in-part or
  narrow; each says so in its header.

## The specs (the oracle — do not edit)

- **`specs/openehr/`** — the vendored normative openEHR spec text + the CNF
  Platform Conformance Test Schedule. **The conformance oracle.** Never
  hand-edit; refreshed only by `scripts/vendor-spec-docs.sh`. Use `/spec-lookup`
  / `/spec-audit`. Pins recorded in `VERSIONS.md`.

## Plans

- `plans/` — `current-phase.md` (live pointer), `WORKLIST.md` (the tracker),
  the active `w14-*` register/tracker files, and
  `feature-flat-structured.md`. Completed plan files are pruned once their
  close is recorded in `PROGRESS.md` (all in git history). See
  `plans/README.md`.

## Platform + verification

- `architecture.md` — how the system is built and why, including the SM
  Platform Service Model component map.
- `VERSIONS.md` — the single source of truth for every version pin (language,
  PG, openEHR spec components) except third-party Rust crates (root `Cargo.toml`
  wins for those).
- `postgres-features.md` — the PG 17/18 feature delta this CDR exploits.
- `conformance/` — ECC run artifacts, one directory per SUT (`ehrbase-rs/`,
  `ehrbase-java/`, …): `CONFORMANCE_REPORT.md`, `CONFORMANCE_STATEMENT.md`,
  `CONFORMANCE_CERTIFICATE.md`, `results.json`, badges; the SUT-independent
  `CATALOG.md` at the root. Current (ehrbase-rs): **370 executed · 335
  passed · 0 failed — CORE PASS / STANDARD PASS**.
- `benchmarks/` — `REPORT.md` + `results.json` + `COMPARISON.md` from the
  benchmark harness (`tools/benchmark`), regenerated per measured pair.
