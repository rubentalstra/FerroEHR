# Progress

One row per phase; see `docs/plans/current-phase.md` for the live pointer and
the per-phase files for detail. Status values: `not-started`, `in-progress`,
`blocked`, `done`. Phase files were **renumbered (2026-07-04) into one clean
`00→20, 99` sequence** so number = order.

> **Three pivots shape this (read the ADRs):**
> - **ADR-004** — the openEHR **spec layer is generated** from BMM, not
>   hand-transcribed (`openehr-base/rm/am/term/lang`).
> - **ADR-005** — the **ITS layer is generated** (canonical XML `ToXml`/`FromXml`
>   + the ITS-REST contract, in `openehr-its`); JSON validation + fidelity gates green.
> - **ADR-006** — the **EHRbase application** is a *modern idiomatic Rust service
>   on top of the generated `openehr-*` crates* (not a 1:1 Java-structure port),
>   with Basic + OAuth2/OIDC auth in Stage 1 (RBAC in Stage 2). App phases build
>   as **compiling, tested increments**.

## Foundation — DONE (phases 00–08: spec + serialization + REST contract)

| Phase | Title | Status | Note |
|---|---|---|---|
| 00 | Scaffolding | done | Workspace green; Java relocated; harness live; `openehr-*`/`ehrbase-*` split. |
| 01 | Foundation + Identification (BASE) | done | Generated (ADR-004) → `openehr-base`. |
| 02 | Terminology (TERM) | done | `openehr-term` — TERM classes generated; bundle/assets hand-written. |
| 03 | Reference Model (RM) | done | Generated (ADR-004) → `openehr-rm` (the domain model consumed by `ehrbase-*`). |
| 04 | Canonical JSON (ITS-JSON) | done | `#[derive(OpenEhrType)]` + `openehr-its::json` + validation/round-trip gates. |
| 05 | Canonical XML (ITS-XML) | done | Generated (ADR-005) `emit-xml` → `ToXml`/`FromXml`; round-trip + EHRbase-XML gates. |
| 06 | ODIN + BMM reader | done | For codegen (`openehr-codegen`/`openehr-lang`); runtime ADL/ODIN parser → P13. |
| 07 | AQL parser + AST | done | `openehr-query` (logos + chumsky), corpus-validated. Path analysis → P16. |
| 08 | ITS-REST contract | done | Generated (ADR-005) `emit-rest` → DTOs + server traits + routes in `openehr-its` (96 ops). |

## Pivot (2026-07-05, ADR-008)

Greenfield internals: our own PG18-native storage + AQL design; the
compatibility target is **openEHR spec conformance** (CNF schedule), not
EHRbase parity; the in-tree EHRbase Java reference was removed. ADR-006/007
partially superseded. Phase files 10/16/19 re-scoped.

## Stage 1 application build — the remaining work (phases 09–20, 99, in order)

| # | Phase | Title | Status | Consumes / crates |
|---|---|---|---|---|
| 1 | 09 | Persistence foundation | **done (2026-07-05)** | `ehrbase::db` (settings/pool/migrate/iden); squashed `0001_baseline.sql` per schema + schema-equality gate vs the legacy Flyway chain (ADR-007); `sea-query-sqlx` replaces `sea-query-binder`; testcontainers PG18, 8/8 tests |
| 2 | 10 | Storage foundation (greenfield node model, ADR-008) | **not-started (NEXT)** | `openehr-rm`/`openehr-its`, P09 infra, PG18 |
| 3 | 11 | REST server foundation + **auth** | not-started | `axum`/`tower-http`, `openehr-its` traits, `oauth2`/`jsonwebtoken`/`argon2`, `utoipa` |
| 4 | 12 | Service layer (versioning, contributions, audit) | not-started | P09/P10/P11, `sqlx` tx |
| 5 | 13 | Template ingestion (OPT 1.4 XML, ADL/AOM) | not-started | `openehr-am`, `openehr-lang` |
| 6 | 14 | WebTemplate builder | not-started | P13, `moka` |
| 7 | 15 | Composition validation | not-started | P14, `openehr-term` |
| 8 | 16 | AQL engine (AST→ASL→SQL) | not-started | `openehr-query`, P09/P10/P14, `sea-query` |
| 9 | 17 | FLAT/STRUCTURED + EhrScape | not-started | `openehr-flat`, `ehrbase-compat`, P14 |
| 10 | 18 | Workspace integration | not-started | binary wiring; delete ported-out Java |
| 11 | 19 | openEHR conformance (CNF schedule, ADR-008) | not-started | `specifications-CNF` runners, corpus suites |
| 12 | 20 | Optimization | not-started | PG18 AIO, pipelining, `JSON_TABLE` |
| 13 | 99 | Cutover | not-started | delete residual Java/Maven; tag release |

**Stage 2** (after P19 parity holds): RBAC/attribute authz, plugin system,
multi-tenancy — see `PORT_MASTER_PLAN §11`.
