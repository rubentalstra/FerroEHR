# docs/ — map of the documentation tree

A pure-Rust, openEHR-spec-conformant CDR (ITS-REST 1.1.0 + AQL 1.1) with
greenfield PG18-native internals. This file says what lives where and which
document is authoritative for what. When two docs disagree, the **newer
decision wins**. *(The former `ADRs/`, `blueprint/`, `enterprise/`, and
`spec-audit/` layers were deleted 2026-07-16/17 — implemented or stale; the
vendored specs are the only doc oracle, and live behaviour is documented in
the code + the user website. The former `design/` folder is gone too —
2026-07-20, with the admin-console scope consolidation onto tracker issue
#152: unbuilt design content lives in the governing plan file or the issue,
never in a parallel design layer.)*

## Start here

| Path | What it is | Authoritative for |
|---|---|---|
| root `ROADMAP.md` | The forward product roadmap | where the product goes next |
| GitHub Issues (`gh issue list --state open`) | The open-items tracker (root `CLAUDE.md` §Issue workflow) | what's open + what's active (pinned = current focus) |
| Closed issues + PR descriptions + `CHANGELOG.md` | The build record (the former `PROGRESS.md` is retired — its content lives in git history) | the historical record of what shipped |
| `endpoint-map.md` | Every endpoint traced route → dispatcher → service → SQL, plus the background loops | the navigation + optimization instrument |

## Decisions

The former `ADRs/` layer has been **deleted** (owner ruling 2026-07-17 — the
ADRs caused more confusion than value: they were superseded piecemeal and left
stale claims behind). Architectural decisions now live inline, in the durable
record: the living reference docs (`architecture.md`, this tree, `VERSIONS.md`),
the closed issues + PR descriptions, `CHANGELOG.md`, and git history. **No document reads, writes, or
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

- `plans/` — one deep working plan per open tracker issue that needs one,
  plus `WORKLIST.md` (the retired tracker's pointer stub). Completed plan
  files are pruned in the PR that lands them, with the close recorded in
  the PR description + issue handoff comment (all in git history). See
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
