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
| 1 | 09 | Persistence foundation | **done (2026-07-05)** | `ehrbase::db` (settings/pool/migrate/iden); the ADR-007 legacy-Flyway baseline + equality gate was subsequently replaced by the greenfield `0001_schema.sql` per schema (ADR-008); `sea-query-sqlx` replaces `sea-query-binder`; testcontainers PG18 |
| 2 | 10 | Storage foundation (greenfield node model, ADR-008) | **done (2026-07-05)** | Spike-validated schema (node per-version + temporal vo_version + ext magnitude fns); lossless node codec (`ehrbase::storage`); 15/15 tests |
| 3 | 11 | REST server foundation + **auth** | **done (2026-07-05)** | `ehrbase-rest` axum app: all 5 generated `*Api` traits mounted (~96 ops) via a generic `ROUTES` dispatcher + type-directed `*Params` deserializer; full `tower-http` stack; JSON/XML negotiation (`openehr-its`); Basic (argon2) + OAuth2/OIDC bearer (jsonwebtoken/JWKS, resource-server) as one middleware (401/403); `figment` config; Swagger UI + status/health/info; `ehrbase` binary boots. 48 crate + 107/107 workspace tests, clippy-clean |
| 4 | 12 | Service layer (versioning, contributions, audit) | **done (2026-07-05, PR #16)** | `ehrbase::service` (DI `Backend` seam): EHR/EHR_STATUS/COMPOSITION/DIRECTORY/CONTRIBUTION CRUD on the `node`/`vo_version` store, temporal versioning + time-travel, `contribution_create` (atomic multi-version), revision history, `ehr_get_by_subject`, stored-query CRUD, item-tag CRUD, committer-from-principal, typed XML responses (`respond_rm`); e2e on PG 18 |
| 5 | 13 | Template ingestion (OPT 1.4 XML) | **done (2026-07-05, PR #17)** | Codegen `emit-opt` → `openehr-its::opt14` (typed OPT + C_* tree + XML); all 91 vendored `.opt` parse; DEFINITION `adl1.4` upload/list/get on `template_store`; `adl2` = 501 |
| 6 | 14 | WebTemplate + FLAT/STRUCTURED (SDT surface) | **done (2026-07-05, PR #18)** | `openehr-flat`: WebTemplate builder + FLAT + STRUCTURED (Better parity), `moka`-cached; SDT endpoints |
| 7 | 15 | Composition validation | **done (2026-07-06, PR #19)** | RM invariants + terminology + WebTemplate walk → ITS-REST 422 |
| 8 | 16 | AQL engine (typed IR → SQL, ADR-008) | **done (2026-07-07)** | `emit-rm-model` BMM attribute model; typed IR (`ehrbase::aql::plan`); typed sea-query SQL (nested-set CONTAINS, magnitude, LATEST+**ALL_VERSIONS**); RESULT_SET 1.0.3; `/query/*` live; corpus e2e PG18. Same branch: ATNA audit trail, GHCR images+CI, full observability stack |
| 9 | 17 | FLAT/STRUCTURED + EhrScape | not-started — **current phase** | `openehr-flat`, `ehrbase-compat`, P14 |
| 10 | 18 | Workspace integration | not-started | binary wiring; delete ported-out Java |
| 11 | 19 | openEHR conformance (CNF schedule, ADR-008) | not-started | `specifications-CNF` runners, corpus suites |
| 12 | 20 | Optimization | not-started | PG18 AIO, pipelining, `JSON_TABLE` |
| 13 | 99 | Cutover | not-started | delete residual Java/Maven; tag release |

**Stage 2** (after P19 conformance holds): RBAC/attribute authz, plugin system,
multi-tenancy — see `PORT_MASTER_PLAN §11`.

## Spec audit (2026-07-06)

Full-codebase audit against the vendored openEHR specs: **14 areas, ~197
findings** tracked in `docs/spec-audit/SPEC_AUDIT.md` (per-finding checkboxes in
`docs/spec-audit/findings/`). Fixes land in waves on `claude/spec-audit-full`;
Wave 1 (critical wire/CNF divergences) underway.
