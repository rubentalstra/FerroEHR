# docs/ — map of the documentation tree

A pure-Rust, openEHR-spec-conformant CDR (ITS-REST 1.0.3 + AQL 1.1) with
greenfield PG18-native internals. This file says what lives where and which
document is authoritative for what. When two docs disagree, the **newer
decision wins**. *(The former `ADRs/`, `blueprint/`, `enterprise/`, and
`spec-audit/` layers were deleted 2026-07-16/17 — implemented or stale; the
vendored specs are the only doc oracle, and live behaviour is documented in
the code + the user website. `design/` holds ONLY designs not yet
implemented — currently the admin-console design — and each file there is
deleted in the PR that implements it.)*

## Start here

| Path | What it is | Authoritative for |
|---|---|---|
| root `ROADMAP.md` | The forward product roadmap | where the product goes next |
| `plans/WORKLIST.md` | The single open-items tracker (one row per item) | what's open |
| `plans/current-phase.md` | Live pointer to the active work | what's active right now |
| `PROGRESS.md` | One row per phase + rebuild checkpoints | the historical record of what shipped |
| `endpoint-map.md` | Every endpoint traced route → dispatcher → service → SQL, plus the background loops | the navigation + optimization instrument |

## Decisions

The former `ADRs/` layer has been **deleted** (owner ruling 2026-07-17 — the
ADRs caused more confusion than value: they were superseded piecemeal and left
stale claims behind). Architectural decisions now live inline, in the durable
record: the living reference docs (`architecture.md`, this tree, `VERSIONS.md`),
`PROGRESS.md`, `CHANGELOG.md`, and git history. **No document reads, writes, or
cites an ADR; the only citable references are the vendored specs and official
external documentation.** (The current design in brief: the spec + ITS layer is
generated from the vendored machine-readable specs by `openehr-codegen`; the
application is idiomatic Rust of our own design on those crates, with its own
PG18-native storage and typed AQL engine, three app crates with zero
re-exports, an SM-aligned service layer, and enterprise capabilities —
eventing, multi-tenancy, FHIR connectors, multimedia externalization;
acceptance is the openEHR conformance suite. See `architecture.md`.)

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
