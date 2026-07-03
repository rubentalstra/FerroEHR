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
(EHRbase v2 recommends PG 16; we target PG 18). PG 18 brings asynchronous I/O,
native `uuidv7()`, B-tree skip scan, temporal `PRIMARY KEY`/`UNIQUE`/`FOREIGN
KEY`, `RETURNING OLD/NEW`, self-join elimination, and OR-to-array planning,
plus `JSON_TABLE()` inherited from PG 17. Managed providers may lag on
`io_uring`; still target PG 18 and let the AIO benefit follow once available.

## openEHR specification matrix

There is no umbrella "openEHR Release" version; each component versions
independently. Pin each one exactly:

| Component | Version | Status | Notes |
|---|---|---|---|
| BASE (Foundation Types + Base Types) | 1.2.0 | STABLE | transcribe first |
| RM (Reference Model) | 1.1.0 | STABLE | literal transcription, ~108 classes |
| AM (Archetype Model) | 2.3.0 | STABLE (OPT 2 in dev) | ADL 1.4, ADL 2, AOM 1.4, AOM 2, OPT 1.4 all stable |
| QUERY (AQL) | 1.1.0 | STABLE | |
| LANG (BMM / ODIN / EL) | 1.0.0 | STABLE (mixed) | BMM schema v2.3 |
| TERM (Terminology) | 3.0.0 | STABLE | XML file internally v3.1.0 |
| ITS-XML (XSDs) | 1.0.2 | STABLE (2.0.0 TRIAL) | TARGET 1.0.2, namespace `http://schemas.openehr.org/v1` (settled 2026-07-03): it is the latest *stable* ITS-XML (2.0.0 is TRIAL/in-development) and is exactly what stock EHRbase emits (`archie`), so it is what a 1:1 port requires. RM *model* stays 1.1.0; only the XML wire format is v1. Both bundles vendored at `crates/openehr-serde/schemas/xml/` (v1 = target; v2 = Stage-3 reference). See phase-05 DECISION note. |
| ITS-REST (REST API) | 1.0.3 | STABLE | ADMIN API is dev-branch only |
| ITS-JSON (JSON Schemas) | development | DEVELOPMENT | pinned commit `5acae056248e917a4b4c56f7e712f4fcfeb616a6` (master, 2021-10-31); `components/openehr_rm_1.1.0_all.json` vendored at `crates/openehr-serde/schemas/` |
| ITS-BMM (BMM instances) | per-RM | STABLE per-schema | |

See `PORT_MASTER_PLAN.md` Section 7 for the class-by-class transcription scope
and Section 3 for the full narrative behind each pin.

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
