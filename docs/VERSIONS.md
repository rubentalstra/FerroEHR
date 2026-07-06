# Pinned Version Matrix

The single source of truth for every version pin in this project. If this file
and a config file (`Cargo.toml`, `rust-toolchain.toml`, CI workflows) disagree,
fix the drift — do not let either silently win. For third-party Rust crate
versions specifically, the root `Cargo.toml` `[workspace.dependencies]` table
is authoritative (see "Rust dependency pins" below); this file records the
platform, language, database, and openEHR specification pins.

## Language and runtime

| Item | Pin |
|---|---|
| Rust toolchain | stable 1.96 (1.96.1) |
| MSRV | 1.96 |
| Edition | 2024 |
| Cargo resolver | v3 |

Rationale: the deliverable is a binary/service, not a published library, so
MSRV can track current stable rather than trailing it. A crate later split out
for standalone publication may relax to 1.94/1.95 at that time.

## Database

| Item | Pin |
|---|---|
| PostgreSQL | 18, target 18.4 or newer (CI runs `postgres:18.4`) |
| Required extensions | `uuid-ossp`, `pgcrypto`, `pg_trgm` |

PostgreSQL is the only component in the stack with a meaningful version delta
(EHRbase Java targets PG 15/16; we target PG 18). PG 18 brings asynchronous I/O,
native `uuidv7()`, B-tree skip scan, temporal `PRIMARY KEY`/`UNIQUE`/`FOREIGN
KEY`, `RETURNING OLD/NEW`, virtual generated columns, self-join elimination, and
OR-to-`ANY` planning, plus `JSON_TABLE()` + the SQL/JSON functions inherited
from PG 17. **See `docs/postgres-features.md`** for the full PG 17+18 feature
delta over PG 16 and where each app phase (P09/P12/P16/P11/P20) uses it —
"the best possible system" means exploiting these. Minor releases (18.1–18.4,
17.x) are bugfix/security only (no new features); pin the latest patch (18.4,
2026-05-14). Managed providers may lag on `io_uring`; still target PG 18 and let
the AIO benefit follow.

## openEHR specification matrix

There is no umbrella "openEHR Release" version; each component versions
independently. **The spec crates are generated from the vendored BMM
meta-model (ADR-004), so the BMM version pins below are load-bearing** — they
are the actual codegen input, not just documentation. Pins bumped to *latest*
on 2026-07-03 (the versions available as clean `*.bmm.json`).

| Component | Version | Status | Notes |
|---|---|---|---|
| BASE (Foundation Types + Base Types) | **1.3.0** | STABLE | generated → `openehr-base` (foundation + base types; `openehr-foundation` folded in) |
| RM (Reference Model) | **1.2.0** | STABLE | generated → `openehr-rm` |
| AM (Archetype Model) | **1.4.0 + 2.4.0** | STABLE | generated → `openehr-am`, both versions as `am14` (ADL 1.4) + `am24` (ADL 2) |
| QUERY (AQL) | 1.1.0 | STABLE | `openehr-query`; grammar-driven (AqlLexer/Parser `.g4`), not BMM |
| LANG (BMM / ODIN / EL) | 1.0.0 | STABLE (mixed) | `openehr-lang` — the ODIN + BMM reader that feeds codegen |
| TERM (Terminology) | 3.1.0 | STABLE | `openehr-term` — **hand-written** (BMM has only interface classes; bundle/assets/logic are not derivable) |
| ITS-XML (XSDs) | 1.0.2 target (2.0.0 TRIAL) | STABLE | canonical XML in `openehr-its` (hand-written, `quick-xml`); namespace `http://schemas.openehr.org/v1`; both bundles vendored at `crates/openehr-its/schemas/xml/`. |
| ITS-REST (REST API) | 1.0.3 | STABLE | ADMIN API is dev-branch only |
| ITS-JSON (JSON Schemas) | development | DEVELOPMENT | validation oracle for the fidelity gate; pinned commit `5acae056248e917a4b4c56f7e712f4fcfeb616a6`; `openehr_rm_1.1.0_all.json` vendored at `crates/openehr-its/schemas/` |
| ITS-BMM (BMM meta-model, JSON) | per-component (see above) | STABLE per-schema | **the codegen input**; vendored `*.bmm.json` at `crates/openehr-codegen/vendor/bmm/` with provenance |

**Spec text vendored in-repo:** the normative documentation for every
component above — plus SM (platform service model / SDT) and **CNF (the
conformance guide + Platform Conformance Test Schedule + Robot suite)** — is
vendored at `docs/specs/openehr/` by `scripts/vendor-spec-docs.sh`, pinned to
these same versions (exact commits in each component's `PROVENANCE.md` and in
the script). It is the read/conformance oracle; codegen still consumes only
`crates/openehr-codegen/vendor/**` and `crates/openehr-its/schemas/**`.

**Parity note:** these are the *latest* spec versions; stock EHRbase/`archie`
emits an RM 1.1.0-era wire format. Track this divergence as a Stage-1 REST
parity consideration — the fidelity gate (EHRbase canonical-JSON corpus
round-trip) is where it will surface.

See `docs/ADRs/ADR-004-spec-driven-codegen.md` for how generation works and
`PORT_MASTER_PLAN.md` Section 7 for the component scope.

## EHRbase reference point

This is a fork, and the fork's import point differs from the point the master
plan was authored against. Both are recorded so provenance is unambiguous:

| Item | Value |
|---|---|
| Plan authored against | EHRbase v2.31.0 |
| Tree actually imported at (Phase 0) | EHRbase v2.33.0 |
| `reference/v1` git ref | v0.32.0 (last pre-v2 tag) |

`PORT_MASTER_PLAN.md` Section 3 was written against v2.31.0. By the time this
fork's Phase 0 reorganization ran, upstream had advanced to v2.33.0, and that
is the tree actually `git mv`'d into the Cargo workspace. Treat v2.33.0 as the
operative behavioural baseline for parity testing; the plan's narrative
references to v2.31.0 describe the reasoning at authoring time, not a
different import.

`reference/v1` is a read-only git reference pinned at v0.32.0, the last tag
before the v1→v2 architectural break. It is consulted only during the Stage 2
enterprise-feature archaeology and restoration work (see `PORT_MASTER_PLAN.md`
Section 11) and is never merged into the working tree during Stage 1.

## Rust dependency pins

The authoritative, fully-pinned third-party Rust dependency set lives in the
root `Cargo.toml` under `[workspace.dependencies]`. `CLAUDE.md` carries a
categorized narrative summary for orientation, and `PORT_MASTER_PLAN.md`
Section 8 carries the original design-time table. On any discrepancy between
this file, `CLAUDE.md`, the master plan, and the manifest, **the manifest
wins**. Add a dependency to a crate with `dep.workspace = true`; do not
hand-pin a version at the crate level.
