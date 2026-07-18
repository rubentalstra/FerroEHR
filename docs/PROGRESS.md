# Progress

One row per phase; see `docs/plans/current-phase.md` for the live pointer and
the per-phase files for detail. Status values: `not-started`, `in-progress`,
`blocked`, `done`. Phase files were **renumbered (2026-07-04) into one clean
`00→20, 99` sequence** so number = order.

> **Three pivots shape this record:**
> - The openEHR **spec layer is generated** from BMM, not hand-transcribed
>   (`openehr-base/rm/am/term/lang`).
> - The **ITS layer is generated** (canonical XML `ToXml`/`FromXml`
>   + the ITS-REST contract, in `openehr-its`); JSON validation + fidelity gates green.
> - The **application** is a *modern idiomatic Rust service
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

| TESTPERF | **Testing-framework full redesign + rewrite** (owner 2026-07-18: suites kept growing — `extensions_openapi::every_documented_path_routes` past 300 s; clarified in-session as a FULL redesign+rewrite). Profiled root cause: 357 DB-backed tests each booting their own `postgres:18` testcontainer + full 3-schema migration (nextest = process per test, nothing shared; the helper duplicated inline in ~29 files). The rewrite: the new **`tools/testkit`** crate — one shared PG18 server (`EHRBASE_TEST_PG_URL` in CI = the workflow's own service with ALTER SYSTEM non-durable tuning; locally a reusable named `ehrbase-testkit-pg18` container, fsync/synchronous_commit/full_page_writes off + max_connections 200), one migrated **template database per migration fingerprint** (`ehrbase::db::migration_fingerprint()`; advisory-locked build stamped as the database comment — readable via `shobj_description` without blocking clones), one `CREATE DATABASE … TEMPLATE` clone per test (WAL_LOG, ~100 ms) with capped pools and an opportunistic sweep of stale clones/templates/`ehrbase_tk_*` roles; `empty_db()` for the pristine-DDL storage spike. All 50 DB test files rewritten (REST via the slimmed `tests/common` seam; every inline `Pg` harness deleted; AMQP/S3 suites keep brokers; `tenant_isolation` roles per-clone — roles are cluster-global on a shared server). nextest 60 s SLOW flagging + a `ci` profile (fail-fast off); CI runs zero docker-in-docker test containers. **Measured (same command/machine): full-workspace nextest 791 s → 329 s wall (test phase 170 s), 2.4×; the 300 s offender → 5.0 s; 1958/1958 passed, zero tests over 60 s; coverage unchanged (no test weakened).** Gates: workspace clippy zero, fmt, machete, doc tests; ECC **386/351/0** zero-drift vs the recorded baseline (fresh artifacts committed — the previously committed results.json was an all-errored transport-dud) | 2026-07-18 | #127 |
| ADMIN-UI | **The admin console** (`app/ehrbase-admin-ui` + `ghcr.io/rubentalstra/ehrbase-rs-admin-ui`) — a standalone pure-Rust (Leptos 0.8 SSR + WASM, zero authored JS) web console consuming the CDR strictly over ITS-REST: dual Basic + OIDC login (BFF-held credentials; code+PKCE against Keycloak), dashboard (count tiles, group tiles, SVG commit trend), Template Manager (upload with verbatim CDR diagnostics; per-template WT path-catalog tree + raw OPT + format-switchable example), EHR browser (finder, status/directory/compositions/contributions, composition viewer with JSON/XML/FLAT/STRUCTURED toggle + revision history + audit), the **point-and-click Query Builder** (per-datatype criteria from the template's constrained value sets, n-ary AND/OR/NOT groups, projection/count shapes, live AQL preview emitted through the real AST — `openehr-query` gained the corpus-round-tripped canonical printer), raw AQL editor, stored queries + console-local groups, system panel (status, SMART discovery, natively-rendered served OpenAPI). Verified end to end by the new Rust-native browser E2E battery (`scripts/ui-e2e.sh`, `thirtyfour` + chromedriver against the composed postgres+CDR+Keycloak stack): **7/7 journeys + the docs-screenshot pass green**, every journey gated on a zero-SEVERE browser console; the battery found and fixed 7 real defects pre-merge (SameSite=Strict killing the OIDC callback, the client router swallowing the OIDC anchor, the leptos-0.8 SSR'd-ErrorBoundary hydration mismatch — converted crate-wide to in-suspense resolution, controlled inputs wiping credentials typed before hydration, the ITS-REST fetch-vs-LIMIT rule 400ing every builder run, a missing tracing subscriber, thaw's crates.io beta failing plain-cargo codegen — re-pinned to main). Ships as the third OCI image + quickstart compose service, with the website-book Admin console chapter embedding 9 harness-captured screenshots, merge-gating `ui-e2e` + `ui-screenshot-guard` CI jobs; UX inspired by Cabolabs EHRServer (credited in the README; reimplemented fresh, no code copied). Gates: clippy zero workspace-wide (--all-features --all-targets + wasm32), 1883/1883 nextest, ECC **386/351/0 zero-drift** | 2026-07-18 | (this PR) |
| V3.2.0 | **Release v3.2.0 cut** (minor: the ATNA audit-trail full redesign + the admin-console completeness wave). En route to a fully green develop (owner mandate — no more red merges): the three merged-red CI breakers fixed (the unused thirtyfour import in e2e_audit under -D warnings; two clippy pedantic findings in pages/audit.rs under --all-features; the missing directory-create book image failing the docs build), `ehr_detail.rs` split into per-tab modules (owner: file too long; pure relocation, zero re-exports), the icon-only chrome sweep (owner hard rule: no emoji — icondata everywhere), the every-view screenshot set (per-tab EHR folders; directory captured both before and after it exists, built live from a named folder template; the audit raw-record view), and the UI-3 / DIR / TESTPERF worklist rows registered. PRs #124 (stabilize, CI fully green) + #125 (the release cut, CI fully green); tag v3.2.0 (prerelease pending owner sign-off) | 2026-07-18 | #124/#125 |
| ATNA | **The ATNA audit-trail full redesign** (`I_SYSTEM_LOG` rewrite; owner directive 2026-07-18 + six owner rulings). Research verdict first (IHE ITI TF-1 §9 / ITI-20 / the RESTful ATNA supplement / IHE BALP v1.1.4, read first-hand): DICOM PS3.15 §A.5 **is** the mandated ATNA message format (kept), HL7v2 has no audit role, the official modern layer is FHIR R4 `AuditEvent`; openEHR itself endorses in-system access logs but rules them out of the EHR content (BASE master07 §Access logging). Landed in six phases on one branch: **(A)** DICOM corrections — the ITI-20-mandated syslog MSGID `IHE+RFC-3881` (was `IHE+DICOM`), dedicated EventIDs (110112 Query, 110106/110107 Export/Import, 110114 User Authentication + Login EventTypeCode 110122 for logins and 401/403s), the operation as an `EventTypeCode` on every record, token `jti` capture, 1xx/3xx→success outcomes; **(B)** the FHIR R4 `AuditEvent` renderer per the BALP patterns (Patient\*/plain profiles, direction-correct source/destination roles, base64 query entity, `OAUTHaccessTokenUse.Minimal` token agent, honest `meta.profile` claims), JSON goldens + round-trip; **(C)** the local Audit Record Repository — a new `audit` PG schema (third migrator; append-only; promoted search columns + the BALP document as the canonical jsonb payload; per-sink outbox stamps; deliberately not RLS-scoped with the operator-surface rationale; tenant carried informationally via a TenantContext response republish); **(D)** sink fan-out — store-first drain with bounded retries and a store-health flag (fail-closed = 503 until recovery: no un-audited PHI access), syslog in-drain, the ITI-20 **ATX:FHIR Feed** outbox worker (ARR outage loses nothing — proven by a wiremock+testcontainers battery), the `[atna]`→`[audit]` config redesign with **audit ON by default** (local store only), per-sink metrics, hourly retention reaper; **(E)** the **ITI-81** retrieval — `GET /fhir/r4/AuditEvent` searchset Bundle (date ge/le, patient, agent, entity, outcome, action, `_count`/`_offset`), admin-only under RBAC, 404 when the store is off, endpoint-map + native OpenAPI; **(F)** **ITI-19** node authentication — `[server.tls]` native TLS (BCP 195 floor) with `client_auth = off|optional|required` mutual-TLS against an explicit trust anchor, e2e-tested on a real listener, plus the dedicated book audit chapter and the rewritten configuration reference. The admin-console audit-log browser (phase G) landed on the design kit: the /audit screen over ITI-81 with URL-driven filters, pagination, raw-record view, first-class disabled/empty states, admin-only gating proven by e2e journeys, and both book screenshots (populated + empty) captured by the composed battery (13/13 journeys green incl. the new audit pair). Gates: workspace clippy zero, fmt clean, suites green (audit_store/audit_feed/audit_iti81/server_tls/audit_e2e batteries added), Helm goldens re-rendered; **ECC deferred by owner directive 2026-07-18** — one zero-drift run covers this rewrite together with the UI-2 admin-console overhaul when the two streams converge | 2026-07-18 | (this PR) |
| W-21 | Release **v3.1.0 + v3.1.1** (the Simplified Formats rewrite + governance overhauls; minor bump per owner). En route, the post-#108 develop CI failure was diagnosed as a REAL multi-tenancy defect — sqlx `before_acquire` never runs for freshly opened connections (docs.rs `PoolOptions::before_acquire`), so a pool-growth acquire ran as the reserved default tenant — fixed in both pool hooks with a deterministic regression test (#110, verified failing on the unfixed code). v3.1.0 published notes-only (its asset build still compiled the `ehrbase` library — the containers.yml crate-consolidation defect, second instance); all five workflows + scripts/deploy audited (release.yml was the only remaining hit), fixed, and v3.1.1 tagged with both arch tarballs attached, `prerelease=true` (pre-release until owner sign-off) | 2026-07-17 | #109/#110/#111 |
| FLAT | Simplified Formats spec-exact greenfield rewrite — `openehr-flat` re-authored from the STABLE ITS-REST `simplified_formats` chapters (one internal sim tree, FLAT/STRUCTURED as pure codecs, typed FlatKey model, master05 table-driven codecs, master06 ctx vocabulary, template-driven walkers with level re-materialisation, in-context WT children per the master04 example, non-conforming stored values via `|raw`); REST negotiation rebuilt (one WireFormat core, RFC 9110 q-values, full endpoint matrix incl. CONTRIBUTION inner payloads, `openehr-template-id` enforcement, strict 406/415 incl. all four retired media types, native utoipa OpenAPI advertising the formats); the `ehrbase-quirks` flag and every vendor-oracle framing deleted; 75 spec-example vectors + corpus round-trips 37/37 stable; new ECC-SF area (16 cases, all passing) — ECC **386/351/0** CORE+STANDARD PASS, OPTIONS OBTAINED (up from 370/335/0, zero drift); resolves issue #95 | 2026-07-17 | #108 |
| W-3f | Platform-crate redesign — the `ehrbase` crate rebuilt spec-first: 12 design registers (spec-onto-code, `docs/design/platform/`), big-bang rewrite into versioning/ (signing dissolved per RM common master06 §Digital Signature) + storage/ (node/version/ehr/tag repos — the semantics/SQL seam) + service/<10 SM chapters> + aql/sql/ split + validation/ + templates/ + extensions/ quarantine; CommitEnv hooks close the CONTRIBUTION-path guard gap; OR-CONTAINS implemented (the blueprint's B6 claim was false); lifecycle state machine, case-insensitive identifiers (+migration 0007), Extract audit events; all 127 register G-rows closed; 1440/1440 nextest, ehrbase clippy-zero, ECC 341/315/0 held exactly (CORE+STANDARD PASS) with the instrument's bare-ETag parsing fixed and the E.2 directory guard restored | 2026-07-13 | (this PR) |
| A1 | Full spec audit — 24-chapter register (1,126 requirements) verified + fixed, zero deferrals: version-tree branching/merge provenance, the AOM 1.4 + ADL2 artefact validators, the AQL single-row function set + TERMINOLOGY boolean/URI forms, RM invariant completion (DV_TEXT family, identifiers, lists, tables), terminology constants + strict subsumption, protocol tail (resolve_refs, body-uid cross-check, supplied contribution uid, ADL2 wire 409); spec-only citation rule enforced on every touched file | 2026-07-12 | (this PR) |
| W-11 | Benchmark rewritten as the hospital-day stress instrument: ward-simulated clinical workload on official CKM templates (byte-identical across SUTs), 16-class CO-corrected latency histograms, CPU/RSS/storage/cold-start sampling, saturation-knee ladder (generator-bound + died-under-load detection), SVG-charted reports + cross-SUT COMPARISON; committed hour-profile pair (rs: 47/188 MB, 0.9% CPU, p99 wins every headline class; upstream: 515/606 MB, 1.7% CPU; knee: upstream 643 vs ours 161 req/s — published, P20 fuel); found + fixed en route: 2 example-generator defects (structural stubs at0017, openEHR rubric labels — both spec-cited, ECC-TPL-017 added), the W-12 overload OOM (shed layer, 503 + Retry-After), 6 instrument defects | 2026-07-14 | (this PR) |
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
