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
| 9 | 17 | FLAT/STRUCTURED + EhrScape | absorbed into the B-arc — final review | `openehr-flat`, EhrScape adapter in `ehrbase-rest` |
| 10 | 18 | Workspace integration | absorbed into the B-arc — final review | binary wiring; no residual Java remains (ADR-008) |
| 11 | 19 | openEHR conformance (CNF schedule, ADR-008) | **met** | ECC 341 executed · 315 passed · 0 failed — CORE PASS / STANDARD PASS (see the B-arc below) |
| 12 | 20 | Optimization | remaining | PG18 AIO, pipelining, `JSON_TABLE` |
| 13 | 99 | Cutover | remaining | final docs; tag first release |

The linear P17–P20 roadmap was overtaken by the blueprint build order (B1–B8)
below, which drove the ECC conformance instrument to full green. P17/P18 work
landed inside that arc; **P19 conformance is met**; P20 optimization
cutover remain, alongside the documentation-website initiative.

## SM track (ADR-010/011) — DONE

One native trait per SM Platform Service Model interface in `ehrbase-sm`; the
SM component map now lives in `docs/architecture.md`. SM-1..SM-4 landed in the
pre-blueprint sequence and SM-5/SM-6 shipped inside the B3 wave.

| Phase | Title | Status |
|---|---|---|
| SM-1 | `app/*`+`tools/*` layout, `ehrbase-sm` native API (trait per SM interface, call-status table, UPDATE_VERSION envelope), ATTESTATION, `is_queryable` gate, contribution list/count, EHR_SUMMARY | **done (2026-07-09, PR #31)** |
| SM-2 | Definitions completion: ADL 1.4 archetype store + OPT completion, `DefinitionAdl2Service` + `adl2_artefact` store + wire adl2 upload/list/get, stored-query calls | **done (2026-07-09, PR #32)** |
| SM-3 | PARTY_RELATIONSHIP (versioning + wire) + EHR Index service + storage-semantics audit wave (all 7 findings fixed) | **done (2026-07-09, PR #33)** |
| SM-4 | Terminology surface + Admin completion; carries the ADR-011 app-crate redesign (protocol-free `ehrbase-sm` literal SM catalog, `Platform` generics, leaf crates dissolved into modules) | **done (2026-07-09, PR #34)** |
| SM-5 / SM-6 | Message (EHR Extract + TDD) / Subject Proxy | **done** — shipped in the B3 SM-services wave (PR #38) |

## Blueprint build arc (B1–B8) — DONE

The roadmap moved to `docs/blueprint/00-THE-BLUEPRINT.md` (§2 = the consolidated
spec-gap surface; it superseded the standalone spec-audit ledger). Every phase
below closed with an ECC run at zero drift; the baseline ratcheted only upward.

| Phase | Title | Close | PR |
|---|---|---|---|
| B1 | ADR-011 rebuild convergence; ECC re-baselined 211/318, zero drift | 2026-07-09 | #36 |
| B2 | ArchetypeValidation depth (81→0 failing cases); ECC 293/319 | 2026-07-10 | #37 |
| B3 | SM services wave 3 — Admin dump/load → SM-5 (Message/EHR Extract/TDD) → SM-6 (Subject Proxy) | 2026-07-10 | #38 |
| B4 | Terminology-server integration + TS conformance harness; ECC 298/338 | 2026-07-10 | #39 |
| B5 | Conformance-instrument corrections (ch 7 D1–D5); instrument made honest | 2026-07-10 | #40 |
| B6 | Full conformance — **ECC 341 executed · 315 passed · 0 failed; CORE PASS / STANDARD PASS** | 2026-07-10 | #41 |
| B7 | Enterprise-grade schema baseline (ADR-013: squashed baseline, four-role model, spec fixes) | 2026-07-10 | #42 |
| B8 | Product-completeness roadmap — market scorecard + four spec-grounded enterprise capabilities | 2026-07-10 | #43 |

## Enterprise capabilities (E1–E5) — DONE

Spec-grounded enterprise features from the B8 roadmap; each closed with ECC
341/315/0 zero drift.

| Phase | Title | Close | PR |
|---|---|---|---|
| E1 | Eventing — contribution outbox + AMQP publisher + event subscriptions (ADR-014) | 2026-07-11 | #44 |
| E2 | Multi-tenancy — RLS `FORCE` isolation, tenant-scoped requests, admin CRUD (ADR-015) | 2026-07-11 | #45 |
| E3 | FHIR connectors — inbound mapping + outbound emitter + read façade (ADR-016) | 2026-07-11 | #46 |
| E4 | S3 multimedia — DV_MULTIMEDIA externalization, verified re-inline, GC, blob dump/load (ADR-017) | 2026-07-11 | #47 |
| E5 | Kubernetes deployment artifacts — hardened Helm chart, ops doc, golden-render validation | 2026-07-11 | #48 |

## Post-arc phases

| Phase | What | Closed | PR(s) |
|---|---|---|---|
| — | Release reset: v3.0.0 (product SemVer; spec crates carry spec versions), Keep-a-Changelog + guard, changelog-driven release workflow, inherited fork tags/branches removed | 2026-07-11 | #53, #55 |
| — | Docs cleanup: 45 historical files pruned, references repointed | 2026-07-11 | #51 |
| W1 | Public documentation website — mdBook on Pages: landing, versioned book (dev · latest · v3.0.0 via the `docs-dist` archive), offline OpenAPI reference (7 API groups), link + OAS-drift gates (both negative-tested), same-PR docs discipline | 2026-07-11 | #52, #54, #56, #57, #60, #62, #63 |

| W-3f | Platform-crate redesign — the `ehrbase` crate rebuilt spec-first: 12 design registers (spec-onto-code, `docs/design/platform/`), big-bang rewrite into versioning/ (signing dissolved per RM common master06 §Digital Signature) + storage/ (node/version/ehr/tag repos — the semantics/SQL seam) + service/<10 SM chapters> + aql/sql/ split + validation/ + templates/ + extensions/ quarantine; CommitEnv hooks close the CONTRIBUTION-path guard gap; OR-CONTAINS implemented (the blueprint's B6 claim was false); lifecycle state machine, case-insensitive identifiers (+migration 0007), Extract audit events; all 127 register G-rows closed; 1440/1440 nextest, ehrbase clippy-zero, ECC 341/315/0 held exactly (CORE+STANDARD PASS) with the instrument's bare-ETag parsing fixed and the E.2 directory guard restored | 2026-07-13 | (this PR) |
| A1 | Full spec audit — 24-chapter register (1,126 requirements) verified + fixed, zero deferrals: version-tree branching/merge provenance, the AOM 1.4 + ADL2 artefact validators, the AQL single-row function set + TERMINOLOGY boolean/URI forms, RM invariant completion (DV_TEXT family, identifiers, lists, tables), terminology constants + strict subsumption, protocol tail (resolve_refs, body-uid cross-check, supplied contribution uid, ADL2 wire 409); spec-only citation rule enforced on every touched file | 2026-07-12 | (this PR) |
| W-10 | Conformance framework redesigned + rewritten from the CNF component up (owner directive: the incrementally-grown instrument was not trusted): 14 spec-derivation registers, spine-first case authoring (every expectation cites the schedule/ITS-REST text), multi-SUT from day one (ehrbase-rs compose · upstream EHRbase Java · BYO endpoint by URL), spec-edition ladder with explicit edition findings, one spec-grade wire-parsing client layer, per-SUT artefact dirs with Statement + Certificate for any SUT (framework self-assessment), fairness register for foreign runs, `conformance compare` matrix; the D1 triage fact-checked 23 failures against the vendored specs — 12 instrument defects fixed (incl. both adjudication registers silently resolving relative to the CWD) and 7 real server defects fixed in separate spec-cited commits (contribution status mapping, concrete VERSIONED_* wire types, demographic full-OVID If-Match, weak ETags, FOLDER-contribution 400); re-derived baseline **368/333/0/35 — CORE + STANDARD PASS**; upstream EHRbase 2.34.0 recorded as comparison DATA | 2026-07-13 | (this PR) |

## Remaining

- **X1 comparison** — honest EHRbase (Java) vs EHRbase-rs comparison page:
  ECC run against upstream, benchmark overhaul, measured numbers only
  (plan drafted, awaiting owner review — `docs/plans/x1-comparison.md`).
- **P20 optimization** — PG18 AIO tuning, hot-read pipelining, `JSON_TABLE` codegen.
- ~~P99 cutover~~ — removed 2026-07-12 (owner ruling): the release machinery
  already shipped with v3.0.0; releases are cut from the changelog whenever
  ready, no dedicated phase needed.

**Stage 2** capabilities (RBAC/attribute authz via the `ehrbase-rest::access`
module, multi-tenancy, plugin system) largely landed early through the E-arc;
remaining enterprise archaeology is tracked in the blueprint.
