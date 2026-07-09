# openEHR-RS — Master Port Plan

**A 1:1, pure-Rust, spec-faithful port of EHRbase (Java/Spring Boot → Rust), executed Bun-style with Claude Code.**

Status: bootstrap document. Version: 1.0. Owner: Ruben (R.D. Talstra).
This file is the single source of truth. Claude Code reads this first and scaffolds the rest of the repo from Sections 12–16.

> **⚠️ CURRENT ROADMAP (2026-07-09): `docs/blueprint/00-THE-BLUEPRINT.md`.**
> This bootstrap plan is historical (see the amendments below — ADR-004/005/006/008
> then ADR-010/011). The live trajectory, priorities, and app-crate reality now
> live in `docs/blueprint/` (`00-THE-BLUEPRINT.md` §2 is the consolidated gap
> surface), with `docs/plans/current-phase.md` (active pointer) under it. Read the
> blueprint first; read this only for the original architecture/difficulty
> reasoning.

---

> ## ⚠️ AMENDMENT (2026-07-03, ADR-004): the openEHR spec layer is GENERATED, not hand-transcribed
>
> This plan originally called for **literal, class-by-class hand-transcription**
> of the openEHR Reference Model, Base Types, AM, etc. (Sections 4, 7, 10, 14).
> **That approach is superseded.** openEHR publishes a machine-readable
> meta-model (BMM); we now **generate** the spec crates from it deterministically
> with `openehr-codegen`. See `docs/ADRs/ADR-004-spec-driven-codegen.md` and the
> "Code generation" section of `CLAUDE.md`.
>
> Read the rest of this document with these substitutions:
> - **"transcribe the RM/BASE/AM classes by hand"** → generate them from the
>   vendored `*.bmm.json`; never hand-write or hand-edit a `// @generated` file.
>   Change the emitter and regenerate. Spec *behaviour* (invariants, functions)
>   lives in hand-written sibling `*_impl.rs` files.
> - **Crate names (Section 5/9):** `openehr-*` = spec (generated) / `ehrbase-*` =
>   the EHRbase application (ported from Java). `openehr-foundation` folded into
>   `openehr-base`; `openehr-terminology`→`openehr-term`, `openehr-odin`+`openehr-bmm`→
>   `openehr-lang`, `openehr-adl`→`openehr-am`, `openehr-aql`→`openehr-query`,
>   `openehr-rest`→`ehrbase-rest`, `openehr-ehrbase-compat`→`ehrbase-compat`,
>   `openehr-server`→`ehrbase` (binary). New crates: `openehr-codegen`, `openehr-derive`.
> - **Version pins (Section 3):** bumped to latest — RM 1.2.0, BASE 1.3.0,
>   TERM 3.1.0, AM 1.4.0 + 2.4.0 (see `docs/VERSIONS.md`).
> - **`openehr-term`** stays hand-written (BMM has only its interface classes).
> - ADR-001/002 (hand-authoring conventions) are superseded *as conventions* by
>   ADR-004; ADR-003 (spec-gap policies) still governs the `*_impl.rs` behaviour.
>
> The narrative below is preserved for its architecture, difficulty map, phase
> sequencing, and Rust-stack decisions, which remain valid. Only the *method* for
> producing the spec layer changed: generate, don't hand-transcribe.

---

> ## ⚠️ AMENDMENT (2026-07-05, ADR-008): greenfield internals; openEHR conformance replaces EHRbase parity
>
> The project pivoted: the application's **storage, versioning, and AQL engine
> are our own PG18-native designs** (not ports of EHRbase's), and the
> compatibility target is **openEHR spec conformance** (the CNF Platform
> Conformance Test Schedule), not behavior-parity with EHRbase. The in-tree
> EHRbase Java reference was removed (git history preserves it). Read
> `docs/ADRs/ADR-008-greenfield-pg18-storage.md`; the phase files remain the
> authoritative roadmap. References below to "parity harness",
> "behavior-compatible with EHRbase", and the in-tree Java are historical.

> ## ⚠️ AMENDMENT (2026-07-04, ADR-005 + ADR-006): ITS is generated; the app is a modern idiomatic service
>
> Two further changes since ADR-004 make **the phase files (`docs/plans/`) and
> `docs/PROGRESS.md` the authoritative roadmap — read them, and ADR-005/ADR-006,
> before this §10 table.**
>
> - **ADR-005** — the **ITS layer is generated** too: canonical **XML**
>   (`ToXml`/`FromXml`) and the **ITS-REST contract** (DTOs + `#[async_trait]`
>   server traits + routes) are emitted into `openehr-its` (`emit-xml`/`emit-rest`);
>   canonical JSON validation + all fidelity gates are green. So the JSON + XML
>   serialization and the REST *contract* are **done**, and the AQL parser is
>   **done** (`openehr-query`).
> - **ADR-006** — the **EHRbase application** is built as a **modern idiomatic
>   Rust service on top of the generated `openehr-*` crates**, *not* a literal
>   1:1 Java-structure port. This **supersedes principles 1 & 3** (literal
>   transcription) *for the application layer*, and **retires the "early phases
>   need not compile" gate (§4.1)** for it — app phases are built as compiling,
>   tested increments. Stack: `axum`/`tower-http`, `sqlx`+`sea-query` (not
>   sea-orm), `oauth2`/`openidconnect`/`jsonwebtoken`/`argon2` for **Basic +
>   OAuth2/OIDC auth** (Stage 1; RBAC Stage 2), `utoipa`, `moka`. Bespoke server
>   logic (AQL engine, versioning, validation, RM↔JSONB) follows EHRbase's
>   *algorithm* as the reference; the **parity harness (§15)** is the acceptance
>   instrument.
>
> **The §10 table below is superseded by the rewritten phase files, which were
> renumbered (2026-07-04) into one clean `00→20, 99` sequence** (foundation
> `00–08`, application build `09–20`, cutover `99`). The real Stage-1 build order
> is **P09 → P10 → P11 → P12 → P13 → P14 → P15 → P16 → P17 → P18 → P19 → P20 →
> P99** (see `docs/plans/current-phase.md` and `docs/PROGRESS.md`). The
> "compile?" column no longer applies to the app phases.

---

## 0. How Claude Code should use this document

1. Read this entire file before creating anything.
2. Execute the bootstrap checklist in Section 16, in order.
3. Generate the `.claude/` harness (Section 12), the `docs/` tree (Section 13), and the phase files (Section 10) from the templates given here.
4. Never begin porting code until the harness, `PORTING.md`, `ROSETTA.md`, and Phase 0 are in place.
5. Treat Sections 4 and 14 as binding rules, not suggestions.

Do not attribute behaviour to this document in commit messages or code comments. Write the code and the annotations; let them speak for themselves.

---

## 1. Mission and non-negotiable principles

**Mission.** Produce a faithful, pure-Rust reimplementation of EHRbase that behaves identically at the openEHR REST API surface, backed by a natively-transcribed openEHR stack (Reference Model, ADL/AOM, AQL, serialization, terminology) written entirely in Rust with no Java, no `archie`, no JVM openEHR SDK, and no ANTLR runtime.

**Principles, in priority order:**

1. **1:1 faithful port first.** Correctness and behavioural equivalence with EHRbase come before any improvement. Improvements and new features are separate, later stages.
2. **Pure Rust native.** We write the RM, ADL, AOM, AQL, serialization, and terminology ourselves. No foreign-language runtime or binding is a dependency of the running server.
3. **Literal RM transcription.** The openEHR Reference Model is transcribed class-by-class from the official published specifications, mirroring the spec hierarchy, attribute names, types, cardinalities, and invariants. Not an idiomatic reinterpretation.
4. **Fork-based.** All work happens in a fork of `ehrbase/ehrbase`. The Java code stays in-tree as a read-only reference during the port.
5. **Bun-style phased execution.** File-by-file faithful translation. Early phases do not need to compile. A later phase makes it compile. A final phase reaches test parity.
6. **Recover then improve.** Enterprise features EHRbase removed between its v1 line and v2 (RBAC and others) are catalogued now and restored after the 1:1 port, then the codebase is improved. Not during the port.

**Sequencing (the whole project in one line):**

> Stage 1: faithful 1:1 Rust-native port → Stage 2: restore enterprise features (RBAC, etc.) → Stage 3: improve the codebase.

---

## 2. Provenance: the three research passes behind this plan

This plan synthesizes three completed research passes. Commit the full dossiers to `docs/research/` so the reasoning travels with the repo.

- **Pass 1 — Port methodology and platform.** The Bun Zig→Rust methodology as the template; EHRbase's full architecture and port-difficulty map; the PostgreSQL 16-vs-18 decision; current Rust tooling; and the Claude Code scaffolding pattern. This is the pass the project is fundamentally about. Save as `docs/research/01-port-methodology-and-platform.md`.
- **Pass 2 — openEHR specification surface.** The per-component release matrix, the class-by-class RM inventory, the AOM/ADL/AQL grammar sources, the ITS XML/JSON schema sources, and the transcription sequencing. Save as `docs/research/02-openehr-spec-surface.md`.

Sections 3–15 below are the actionable distillation of both.

---

## 3. Pinned target versions

Pin these exactly. Record them in `rust-toolchain.toml`, `Cargo.toml`, CI, and `docs/VERSIONS.md`.

### Language and runtime
- **Rust stable 1.96** (1.96.1 is current as of 2026-07-01). Pin via `rust-toolchain.toml`.
- **MSRV: 1.96.** Acceptable because the deliverable is a binary/service, not a crates.io library. Any crate later split out for publication may drop to 1.94/1.95 at that time.
- **Edition 2024** (stabilized in Rust 1.85, the correct default for a new project).

### Database
- **PostgreSQL 18.4 or newer.** This resolves the "old v16 vs v18" question: PostgreSQL is the only component in the stack with a meaningful 16→18 delta. EHRbase v2 recommends PG 16; we target PG 18 for asynchronous I/O, native `uuidv7()`, B-tree skip scan, temporal PRIMARY KEY/UNIQUE/FOREIGN KEY, RETURNING OLD/NEW, self-join elimination, OR-to-array planning, and (from PG 17) `JSON_TABLE()`. Required extensions: `uuid-ossp`, `pgcrypto`, `pg_trgm`.
- Note: managed providers may lag on `io_uring`; still target PG 18 and let the AIO benefit follow.

### openEHR specification matrix (transcription targets)
There is **no umbrella "openEHR Release 1.1.0"**; components version independently. Pin each:

| Component | Version | Status | Notes |
|---|---|---|---|
| BASE (Foundation + Base Types) | 1.2.0 | STABLE | transcribe first |
| RM (Reference Model) | 1.1.0 | STABLE | literal transcription, ~108 classes |
| AM (Archetype Model) | 2.3.0 | STABLE (OPT 2 in dev) | ADL 1.4, ADL 2, AOM 1.4, AOM 2, OPT 1.4 all stable |
| QUERY (AQL) | 1.1.0 | STABLE | |
| LANG (BMM/ODIN/EL) | 1.0.0 | STABLE (mixed) | BMM schema v2.3 |
| TERM (Terminology) | 3.0.0 | STABLE | XML file internally v3.1.0 |
| ITS-XML (XSDs) | 2.0.0 | TRIAL (1.0.2 stable) | support both for round-trip |
| ITS-REST (REST API) | 1.0.3 | STABLE | ADMIN API is dev-branch only |
| ITS-JSON (JSON Schemas) | development | DEVELOPMENT | no numbered release; pin a git commit hash |
| ITS-BMM (BMM instances) | per-RM | STABLE per-schema | |

**EHRbase reference point:** EHRbase v2.31.0 (released 2026-04-28), JDK 25, PG 15+/16. Its template ingestion is still ADL 1.4 / OPT 1.4 XML, so implement OPT 1.4 XML before ADL 2.

---

## 4. The Bun-style porting methodology (binding)

We copy the method Bun used for its Zig→Rust rewrite (merged May 2026, ~1M lines, Claude Code-executed). The core mechanic is a lookup-table porting guide plus a three-phase gate plus mandatory annotations.

### 4.1 The three-phase gate

**Phase A — faithful capture. The Rust does NOT need to compile.**
Draft a `.rs` next to (or corresponding to) every Java source, mirroring class names, method names, field order, and control flow. Capturing intent is the goal, not compilation. This is the answer to the recurring question: **yes, early phases do not need to compile.** This removes the single most common failure mode (getting stuck on type errors instead of translating logic). Two standing exceptions are allowed without special justification: Java constructors that throw become `Result<Self, E>`, and `AutoCloseable`/`close()` becomes `impl Drop`. Any reshape forced by the borrow checker is allowed only when marked `// PORT NOTE: reshaped for borrowck`.

**Phase B — make it compile, crate-by-crate.**
Wire `Cargo.toml`, fix imports, resolve `todo!()`s, and drive `cargo check` errors to zero. Process the workspace leaf crates first (a crate that depends on nothing internal), then their dependents. Compiler-error count is the burn-down metric.

**Phase C/D — cleanup and parity.**
Deduplicate, remove scaffolding, then run the parity harness until green. Behavioural equivalence at the REST surface is the acceptance bar.

### 4.2 Mandatory annotation vocabulary

Every ported file carries these where relevant, so the whole port is grep-able:
- `// TODO(port):` unfinished translation.
- `// PERF(port):` a place to optimize after parity.
- `// PORT NOTE:` a deliberate structural deviation (e.g. borrowck reshape).
- `// SAFETY:` justification for any `unsafe` (we expect almost none; this is a web service, not a runtime).

### 4.3 Mandatory PORT STATUS trailer

Every ported `.rs` ends with:

```rust
// ─────────────────────────────────────────────
// PORT STATUS
//   source: <java file this replaces, e.g. crates/openehr-server/src/aql/AqlSqlLayer.java>
//   source_loc: <line count of the Java file>
//   confidence: high | medium | low
//   todos: <count of TODO(port) in this file>
//   note: <one line for Phase B triage>
// ─────────────────────────────────────────────
```

### 4.4 Branch discipline

All machine-generated branches are namespaced `claude/*` (e.g. `claude/phase-01-rm-data-types`). This is a hard rule enforced by a hook.

### 4.5 Test discipline (non-negotiable)

- **Never** silently weaken, skip, or delete an existing test to make the port pass.
- **Never** edit a test to route around a runtime bug it exposes.
- Use a **negative-test gate**: a parity test is only valid if it still fails against a stock Java EHRbase without our fix. Model this as a `USE_REFERENCE_EHRBASE=1` mode in the parity harness.

### 4.6 Externalized cross-file context (the Bun `LIFETIMES.tsv` trick)

Before Phase 1, pre-compute a per-field ownership classification so per-file agents look up the answer instead of holding the whole codebase in context. Store as `docs/LIFETIMES.tsv` with columns: `file · type · field · java_type · class · rust_type · evidence`, where `class ∈ {OWNED, SHARED, BORROW_PARAM, STATIC, BACKREF, INTRUSIVE, ARENA, UNKNOWN}`.

---

## 5. Repository strategy (fork, single in-place workspace)

We do **not** keep a Java tree and a Rust tree side by side, and we do **not** move Java into a `legacy/` folder. We fork EHRbase, stand up one proper Rust workspace at the repo root, relocate the existing Java into the crate it belongs to, and write each `.rs` beside the `.java` it replaces (Bun-style: same directory, corresponding name). There is exactly one coherent structure.

1. **Fork `ehrbase/ehrbase`.** All work happens in the fork. Do not open PRs upstream during the port.
2. **Phase 0 reorganization (`git mv`).** Convert the Maven reactor into a single Cargo workspace rooted at the repo root, and `git mv` the EHRbase Java into the matching crates (mapping in Section 9.1). This is one large, intentional restructuring commit. It detaches file history from upstream `ehrbase/ehrbase`; that is an accepted cost of a hard fork we never merge back. Use `git mv` (not delete+create) so per-file history is preserved across the move.
3. **`.rs` beside `.java`.** After the move, port each Java file to a Rust file in the same crate directory, mirroring its name. Java stays in place until its Rust counterpart reaches parity, then the Java file is deleted in the same phase.
4. **openEHR spec crates start empty.** The foundational crates (`openehr-foundation`, `openehr-base`, `openehr-rm`, `openehr-terminology`, `openehr-odin`, `openehr-bmm`, `openehr-adl`, `openehr-aql`, `openehr-serde`, `openehr-flat`) have **no Java to receive** — EHRbase got all of that from the external `archie`/openEHR-SDK libraries, which are not in the repo. These crates are written from the specifications (Section 7), not ported.
5. **v1 enterprise code as a read-only git reference.** The RBAC/enterprise code you want back was deleted before v2 and does not exist in the current tree. Keep the last pre-v2 tag as a read-only reference in the same fork (a `reference/v1` branch or the tag itself), consulted **only** during Stage 2. It is never merged into the working tree during Stage 1. *(Assumption: option 1 from our discussion — a read-only git ref, not a one-time file extraction. Override if you prefer the extraction model.)* Pin the exact tag in Phase 0.
6. **Protect Java during the port.** A `PreToolUse` hook blocks edits to any `**/*.java` that does not yet have a completed Rust counterpart, and blocks edits to Maven build files (`**/pom.xml`, `mvnw`, `mvnw.cmd`, `.mvn/**`). This keeps the reference implementation intact while it is still being read from.

Directory shape (single root workspace; Java and Rust coexist inside crates during the port):

```
<fork root>/                     # = Cargo workspace root
├── Cargo.toml                   # [workspace] overlaying the whole repo
├── rust-toolchain.toml
├── PORT_MASTER_PLAN.md          # this file
├── CLAUDE.md                    # generated (Section 12)
├── AGENTS.md                    # symlink/import of CLAUDE.md
├── .claude/                     # generated harness (Section 12)
├── crates/
│   ├── openehr-foundation/      # spec crate — starts empty, written from specs
│   ├── openehr-base/            # spec crate
│   ├── openehr-terminology/     # spec crate
│   ├── openehr-rm/              # spec crate
│   ├── openehr-serde/           # spec crate
│   ├── openehr-odin/            # spec crate
│   ├── openehr-bmm/             # spec crate
│   ├── openehr-adl/             # spec crate
│   ├── openehr-flat/            # spec crate
│   ├── openehr-aql/             # spec crate
│   ├── openehr-rest/            # EHRbase server Java moves here → ported in place
│   ├── openehr-ehrbase-compat/  # EHRbase server Java moves here → ported in place
│   └── openehr-server/          # EHRbase server Java moves here → ported in place (binary)
└── docs/                        # plans, research, ADRs (Section 13)
```

Inside a server crate during the port, a directory holds both files until parity, e.g.:

```
crates/openehr-server/src/aql/
├── AqlSqlLayer.java             # moved here by git mv in P0; deleted when parity reached
└── aql_sql_layer.rs             # written beside it during the AQL-engine phase
```

---

## 6. What we are porting: EHRbase architecture and difficulty map

EHRbase is a Spring Boot openEHR Clinical Data Repository. Modules (Maven reactor): `api`, `application`, `aql-engine`, `base`, `bom`, `cli`, `configuration`, `db_scripts`, `jooq-pg`, `plugin`, `rest-ehr-scape`, `rest-openehr`, `rm-db-format`, `service`, `test-coverage`/`tests`.

**REST surface.** Base path `/ehrbase/rest/openehr/v1` implementing openEHR ITS-REST 1.0.3: EHR, EHR_STATUS, COMPOSITION, DIRECTORY/FOLDER, CONTRIBUTION, QUERY (`/aql` + stored queries), DEFINITION (`/template/adl1.4`, `/template/adl2` mostly 501). EHRbase-specific: Admin API (`/rest/admin`), `/rest/status`, `/management/*`, experimental Item Tags, EhrScape (`/rest/ecis/v1/*`).

**AQL engine (the crown jewel).** Pipeline: parse (ANTLR grammar) → semantic/path analysis against Web Templates → **AST → ASL (Abstract SQL Layer, EHRbase's own IR)** → ASL rewrite/optimize → ASL → SQL (JSONB path extraction, array unnesting, current+history UNION) → execute → assemble RESULT_SET (schema 1.0.3).

**Persistence (v2).** jOOQ + PostgreSQL, HikariCP, Flyway. v2 schema decomposes compositions **row-per-locatable** in `ehr.comp_data`/`_history` with leaf-attribute JSONB, plus `ehr.comp_version`, `ehr.ehr`, `ehr.ehr_status_data`, `ehr.ehr_folder_data`, `ehr.contribution`, `ehr.audit_details`, `ehr.template_store`, `ehr.stored_query`, `ehr.item_tag`. Every write inserts an audit_details and a contribution row; versioning uses current + `_history` table pairs via triggers.

**Serialization.** Canonical JSON (primary, `_type` discriminator), canonical XML (JAXB/XSD), FLAT/simSDT, STRUCTURED/structSDT, Web Template JSON, plus a newer Matrix format.

### Port-difficulty map

| Area | Difficulty | Share |
|---|---|---|
| REST controllers, DTOs, admin, security wiring, cache, config, metrics, Swagger, migrations, CLI, item tags, stored queries, AQL DTO model | EASY | ~15% |
| Service orchestration & transactions, jOOQ→sea-query, schema, canonical JSON, OPT XML parsing, versioning, EhrScape, terminology client, AQL grammar parsing | MEDIUM | ~25% |
| RM classes (own transcription), WebTemplate builder, composition validation, canonical XML, FLAT/STRUCTURED, AQL path analysis, result rebuild, rm-db-format | HARD | ~35% |
| **AQL planner (AST→ASL), AQL SQL generator (ASL→SQL)**, full ADL2/AOM2 (deferrable) | VERY HARD | ~25% |

**Critical path (~60% of complexity):** RM transcription + AQL engine (parse+plan+SQL+result) + rm-db-format + validation. Sequence phases so an increasingly capable partial system is usable at each boundary.

---

## 7. openEHR native transcription scope

This is the pure-Rust spec build. Full detail is in `docs/research/02-openehr-spec-surface.md`; the essentials:

### 7.1 RM class inventory (transcribe literally)

- **rm.data_types (27):** `DATA_VALUE` root; Basic (`DV_BOOLEAN`, `DV_STATE`, `DV_IDENTIFIER`); Text (`DV_TEXT`, `DV_CODED_TEXT`, `CODE_PHRASE`, `TERM_MAPPING`, `DV_PARAGRAPH`); Quantity (`DV_ORDERED`, `DV_ORDINAL`, `DV_SCALE`, `DV_QUANTIFIED`, `DV_AMOUNT`, `DV_QUANTITY`, `DV_COUNT`, `DV_PROPORTION`, `DV_ABSOLUTE_QUANTITY`, `DV_INTERVAL<T>`, `REFERENCE_RANGE<T>`, `PROPORTION_KIND`); Date_time (`DV_TEMPORAL`, `DV_DATE`, `DV_TIME`, `DV_DATE_TIME`, `DV_DURATION`); Time_specification (`DV_TIME_SPECIFICATION`, `DV_PERIODIC_TIME_SPECIFICATION`, `DV_GENERAL_TIME_SPECIFICATION`); Encapsulated (`DV_ENCAPSULATED`, `DV_MULTIMEDIA`, `DV_PARSABLE`); URI (`DV_URI`, `DV_EHR_URI`).
- **rm.data_structures (12):** `DATA_STRUCTURE`; `ITEM_STRUCTURE` (`ITEM_SINGLE`, `ITEM_LIST`, `ITEM_TABLE`, `ITEM_TREE`); `ITEM`, `CLUSTER`, `ELEMENT`; `HISTORY<T>`, `EVENT<T>`, `POINT_EVENT<T>`, `INTERVAL_EVENT<T>`.
- **rm.common (~22):** `PATHABLE`, `LOCATABLE`, `ARCHETYPED`, `LINK`, `FEEDER_AUDIT`, `FEEDER_AUDIT_DETAILS`; `PARTY_PROXY`, `PARTY_SELF`, `PARTY_IDENTIFIED`, `PARTY_RELATED`, `PARTICIPATION`, `AUDIT_DETAILS`, `ATTESTATION`, `REVISION_HISTORY`, `REVISION_HISTORY_ITEM`; `VERSIONED_OBJECT<T>`, `VERSION<T>`, `ORIGINAL_VERSION<T>`, `IMPORTED_VERSION<T>`, `CONTRIBUTION`; `FOLDER`.
- **rm.ehr (20):** `EHR`, `EHR_STATUS`, `EHR_ACCESS`, `COMPOSITION`, `EVENT_CONTEXT`, `CONTENT_ITEM`, `SECTION`, `ENTRY`, `ADMIN_ENTRY`, `CARE_ENTRY`, `OBSERVATION`, `EVALUATION`, `INSTRUCTION`, `ACTION`, `ACTIVITY`, `INSTRUCTION_DETAILS`, `ISM_TRANSITION`, plus versioned bindings.
- **rm.integration (1):** `GENERIC_ENTRY`. **rm.ehr_extract:** experimental, defer.
- **rm.demographic (14):** `PARTY`, `ROLE`, `ACTOR`, `PERSON`, `ORGANISATION`, `GROUP`, `AGENT`, `PARTY_RELATIONSHIP`, `PARTY_IDENTITY`, `CONTACT`, `ADDRESS`, `CAPABILITY`, versioned binding.
- **rm.support:** terminology-service interfaces + `MEASUREMENT_SERVICE` + `EXTERNAL_ENVIRONMENT_ACCESS` mixin.
- **BASE (~25):** Foundation Types (primitives, `Interval<T>`, containers, ISO 8601 temporals, functional types) and Base Types (Identification: `UID`→`ISO_OID`/`UUID`/`INTERNET_ID`; `OBJECT_ID`→`UID_BASED_ID`→`HIER_OBJECT_ID`/`OBJECT_VERSION_ID`, plus `ARCHETYPE_ID`/`TEMPLATE_ID`/`TERMINOLOGY_ID`/`GENERIC_ID`; `OBJECT_REF`/`PARTY_REF`/`LOCATABLE_REF`; Resource classes).

### 7.2 Rust transcription hazards (decide once, in Phase 1)

- **Generics are constrained and pervasive** (11+ generic classes): `Interval<T:Ordered>`, `DV_INTERVAL<T:DV_ORDERED>`, `REFERENCE_RANGE<T:DV_ORDERED>`, `HISTORY<T:ITEM_STRUCTURE>`, `EVENT<T>`, `VERSIONED_OBJECT<T>`, `VERSION<T>`, etc.
- **Multiple inheritance** in `Ordered_Numeric`, `Iso8601_type`, `DV_DURATION`, plus the `EXTERNAL_ENVIRONMENT_ACCESS` mixin. Model as composition + traits.
- **Covariant redefinition:** `LOCATABLE_REF.id` (OBJECT_ID→UID_BASED_ID), `ITEM_STRUCTURE.as_hierarchy()`, `DV_COUNT.magnitude` (Integer vs Real).
- **`PATHABLE.parent()` reverse pointer:** do not use owning back-references; use `Weak` or path-index lookup.
- **Enum vs trait:** prefer closed enums for `DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`, `VERSION<T>`; trait objects only for genuinely archetype-driven runtime polymorphism.
- **Recursion:** box fields in `FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`, `DV_MULTIMEDIA.thumbnail`.
- **Watch-outs:** `EVENT_CONTEXT`, `INSTRUCTION_DETAILS`, `ISM_TRANSITION` inherit `PATHABLE` **not** `LOCATABLE`. Terminology-service interfaces live in `rm.support`, not BASE. `ACCESS_GROUP_REF` was not migrated to BASE 1.2.0 (implement only if legacy data needs it). `Octet` not "Byte". Symbolic operators (`++`, `and then`) become named methods.

### 7.3 Grammar and schema sources (reimplement, don't bind)

- **ANTLR grammars (reimplement in Rust):** active repo `github.com/openEHR/openEHR-antlr4` — `reader_adl2/`, `reader_adl14/`, `reader_aql/`, `reader_bmml/`, `reader_common/` (each `src/main/antlr/*.g4`). AQL canonical copy: `github.com/openEHR/specifications-QUERY/docs/AQL/grammar/AqlLexer.g4` and `AqlParser.g4`. ODIN canonical: `specifications-BASE/computable/grammar/odin.g4`.
- **XML schemas (ITS-XML):** RM 1.1.0 XSDs at `specifications-ITS-XML/components/RM/Release-1.1.0/` (`Common.xsd`, `DataTypes.xsd`, `DataStructures.xsd`, `Ehr.xsd`, `Demographic.xsd`, `EhrExtract.xsd`); AM 1.4 OPT XSDs at `components/AM/Release-1.4/`. Also support the legacy 1.0.2 bundle. Namespace `http://schemas.openehr.org/v1`.
- **JSON schemas (ITS-JSON, development):** entry point `components/openehr_rm_1.1.0_all.json`; snake_case attributes; `_type` discriminator (uppercase RM class name), required when the declared type is abstract; `_`-prefixed metadata; abstract classes flattened, not `$ref`-chained; nulls omitted; UIDs are `{_type, value}`; `DV_MULTIMEDIA.data` base64 inline. Pin a commit hash.
- **Terminology (bundle as static assets):** `specifications-TERM/computable/XML/en/openehr_terminology.xml` (+ other languages behind features), `computable/XML/PropertyUnitData.xml`. Preserve the `id=532` dual-rubric quirk (`complete` vs `completed`).

### 7.4 FLAT / STRUCTURED / Web Template (vendor formats)

These are Better/Marand conventions being retro-standardized as SDT (development). Target Better's `web-template` semantics (`github.com/better-care/web-template` + `web-template-tests`). Note: DV_QUANTITY uses `|unit` (**singular**) in both Better and EHRbase — the earlier "`|unit` vs `|units`" note was inaccurate; genuine Better extras are `|unit_system`/`|unit_display_name`, behind the `ehrbase-quirks` flag. WebTemplate/FLAT/STRUCTURED are a **Better/EHRbase interop layer, NOT CNF-conformance-gated** (CNF tests only OPT provisioning + canonical XML/JSON). MIME types: `application/openehr.wt+json`, `application/openehr.wt.flat+json`, `application/openehr.wt.structured+json`.

---

## 8. Rust stack (pinned)

Reject the Bun "no Tokio" stance: Bun avoided Tokio because it is a low-level runtime. EHRbase is a network service whose floor is a Postgres round-trip; idiomatic async Rust on Tokio is correct.

**The authoritative, fully-pinned dependency set is the root `Cargo.toml` `[workspace.dependencies]` (verified 2026-07). CLAUDE.md carries the full categorized list.** The table below is the orientation summary; the manifest wins on any discrepancy.

| Layer | Crate | Version |
|---|---|---|
| Web framework | axum (+ axum-extra, axum-server) | 0.8 |
| HTTP middleware / core | tower 0.5, tower-http 0.6, http 1, hyper 1 | current |
| OpenAPI (code-first) | utoipa + utoipa-axum | 5 |
| Swagger UI / Redoc / Scalar | utoipa-swagger-ui 9, utoipa-redoc/scalar/rapidoc 5 | current |
| Auth (no hand-rolling) | jsonwebtoken 9, oauth2 5, openidconnect 4, argon2 0.5, tower-sessions 0.14 | current |
| RBAC/ABAC (Stage 2) | casbin 2 or cedar-policy 4 | current |
| TLS | rustls 0.23, tokio-rustls 0.26 | current |
| Async runtime | tokio | 1 (rt-multi-thread, macros, signal, net, time, sync) |
| PG driver + pool + migrations | sqlx | 0.9 (MSRV 1.94; no jiff feature) |
| Dynamic SQL builder (jOOQ analogue) | sea-query (+ sea-query-binder 0.7) | 0.32 |
| JSON | serde + serde_json | 1 |
| Canonical JSON (JCS) | serde_jcs or custom | 0.1 / custom |
| XML | quick-xml (with `serialize`) | 0.37 |
| Canonical XML (C14N) | shell `xmllint --c14n` fallback | — |
| Decimal (BigDecimal analogue) | rust_decimal | 1 |
| Lexer | logos | 0.15 |
| Parser | chumsky (or winnow 0.7) | 0.10 (1.0 still alpha) |
| Parser diagnostics | miette 7, ariadne 0.5 | current |
| UUID | uuid (v4, v7, serde, fast-rng) | 1 |
| Time | jiff | 0.2 (1.0 not yet released) |
| Validation (outer surface) | garde | 0.22 |
| Caching (Caffeine analogue) | moka | 0.12 |
| Rate limiting | tower_governor + governor | current |
| Test runner | cargo-nextest | current |
| Snapshot tests | insta | 1 |
| Property tests | proptest | 1 |
| HTTP mocking | wiremock | 0.6 |
| DB integration | testcontainers + testcontainers-modules | current |
| Observability (lockstep) | opentelemetry / _sdk / -otlp 0.31, tracing-opentelemetry 0.33, tracing-subscriber 0.3 | current |
| Metrics | metrics 0.24, metrics-exporter-prometheus 0.16 | current |
| Errors | thiserror 2 (libs) + anyhow 1 (bins only) | current |
| Config | figment 0.10 or config 0.14 | current |
| HTTP client | reqwest | 0.12 |
| JSON Schema validation | jsonschema | 0.26 |

Deep RM invariant validation gets a custom framework mirroring Archie's validators (a `Validate` trait taking a validation context, a path, and an error accumulator), layered under `garde` for the request DTOs.

If sqlx's lack of query pipelining bottlenecks hot AQL reads, isolate that read path onto `tokio-postgres` + `deadpool-postgres`.

---

## 9. Cargo workspace layout

Crate boundaries mirror openEHR component boundaries so each crate maps ~1:1 to a spec component and versions independently. All crates live under `crates/` in the single root workspace (Section 5).

```
crates/                       (under the workspace root)
├── openehr-foundation        # BASE foundation_types: Any, Interval<T>, containers,
│                             # ISO8601 temporals, Terminology_code, functional types
├── openehr-base              # BASE base_types: definitions, builtins, identification, resource
│                             #   → deps: openehr-foundation
├── openehr-terminology       # TERM 3.x XML bundle + terminology service
│                             #   → deps: openehr-base
├── openehr-rm                # RM 1.1.0: data_types, data_structures, common, ehr,
│                             # demographic, integration, (ehr_extract feature-gated), support
│                             #   → deps: openehr-base, openehr-terminology
├── openehr-serde             # Canonical JSON (ITS-JSON) + canonical XML (ITS-XML), _type dispatch
│                             #   → deps: openehr-rm
├── openehr-odin              # ODIN parser (chumsky)
├── openehr-bmm               # BMM object model + P_BMM parser (schema v2.3)
│                             #   → deps: openehr-odin
├── openehr-adl               # ADL 1.4 + ADL 2 parsers, AOM 1.4 + AOM 2, OPT 1.4 XML, OPT 2 flattener
│                             #   → deps: openehr-odin, openehr-bmm, openehr-rm, openehr-serde
├── openehr-flat              # FLAT / STRUCTURED / Web Template (Better semantics + EHRbase quirks)
│                             #   → deps: openehr-rm, openehr-serde, openehr-adl
├── openehr-aql               # AQL 1.1.0 lexer + parser + AST + semantic analyser
│                             #   → deps: openehr-rm, openehr-adl
├── openehr-rest              # ITS-REST 1.0.3 server + client (axum). RECEIVES EHRbase REST Java.
│                             #   → deps: openehr-rm, openehr-serde, openehr-adl, openehr-aql
├── openehr-ehrbase-compat    # EHRbase-compatible endpoint aliases, admin API, OPT 1.4 ingestion,
│                             # WebTemplate export, EhrScape. RECEIVES EhrScape + admin Java.
│                             #   → deps: openehr-rest, openehr-flat
└── openehr-server            # reference binary: persistence (sqlx + sea-query), AQL execution
                              # engine (ASL), versioning, contributions. RECEIVES most server Java.
                              #   → deps: all of the above
```

The crates marked **RECEIVES** are where EHRbase's existing Java lands in the Phase 0 `git mv`, then gets ported in place. The other ten are openEHR spec crates that start empty and are written from the specifications (Section 7).

### 9.1 EHRbase Maven module → Rust crate mapping (Phase 0 `git mv`)

Server Java concentrates into three crates. The openEHR-spec surface EHRbase pulled from `archie`/openEHR-SDK has no Java in the repo and is therefore written fresh.

| EHRbase Maven module | Target crate | Notes |
|---|---|---|
| `rest-openehr` (controllers, DTOs, security wiring, Swagger) | `openehr-rest` | REST surface + request/response DTOs |
| `rest-ehr-scape` (EhrScape/Flat/Structured endpoints) | `openehr-ehrbase-compat` | plus admin API, WebTemplate export |
| `application` (Spring Boot entry, `application.yml`) | `openehr-server` | becomes the binary crate main + config |
| `service` (service impls, repositories) | `openehr-server` | orchestration, transactions, versioning |
| `aql-engine` (parser glue, ASL, SQL translator) | `openehr-server` (`src/aql/`) | ASL + ASL→SQL live here; **AQL grammar/AST/parse** is the separate `openehr-aql` spec crate |
| `rm-db-format` (RM ↔ JSONB decomposed rows) | `openehr-server` (`src/rm_db_format/`) | the row-per-locatable bridge |
| `jooq-pg` (generated jOOQ + Flyway migrations) | `openehr-server` (`migrations/`, `src/db/`) | migrations copied verbatim; jOOQ code discarded (replaced by sea-query) |
| `configuration` (Spring `@Configuration`s) | `openehr-server` (`src/config/`) | DataSource, cache, security, metrics config |
| `plugin` (PF4J SPI) | `openehr-server` (`src/plugin/`) | port stub only; real plugin system is a Stage 2 ADR |
| `cli` | `openehr-server` (`src/bin/` or subcommands) | |
| `api` (service interfaces + DTOs) | split: RM/openEHR DTOs → `openehr-rm`/`openehr-serde`; service traits → `openehr-server` | interfaces follow their implementations |
| `base` (utilities) | absorbed where used (mostly `openehr-server`, some `openehr-rm`) | v2 already thinned this module |
| `bom`, `test-coverage`, `tests` | workspace-level config + `crates/*/tests/` | BOM → `[workspace.dependencies]` |
| — (RM, AOM/ADL, AQL grammar, serialization, terminology: from `archie`/SDK, **not in repo**) | `openehr-foundation`, `openehr-base`, `openehr-rm`, `openehr-terminology`, `openehr-odin`, `openehr-bmm`, `openehr-adl`, `openehr-aql`, `openehr-serde`, `openehr-flat` | written from specs (Section 7) |



Feature flags: `lang-{de,es,fr,pt,ja}` on `openehr-terminology`; `adl2`, `opt2` on `openehr-adl`; `ehr-extract` on `openehr-rm`; `admin-api` on `openehr-rest`; `ehrbase-quirks` on `openehr-flat`.

Enable a workspace lint table (`[workspace.lints.clippy]`), `cargo-hakari` for feature unification, `mold` linker on Linux, `sccache` in CI, and `cargo audit` + `cargo deny` gates.

---

## 10. The master phase plan

Two nested sequences run inside Stage 1: the EHRbase-port phases (P) and the spec-transcription layers (L). Each L is a prerequisite for the P that consumes it. The "compile?" column states whether Phase A applies (does-not-need-to-compile) or the phase is itself a make-it-compile/parity phase.

### Stage 1 — the 1:1 faithful port

| Phase | Title | Consumes | Compile? |
|---|---|---|---|
| P0 | Scaffolding: fork; **`git mv` reorganization** into the single root workspace (Section 9.1); CI; `.claude/` harness; `PORTING.md`; `ROSETTA.md`; `LIFETIMES.tsv` seed; pin v1 tag as read-only ref | — | must build (empty + moved crates) |
| P1 | **L1+L2** Foundation + Identification (BASE 1.2.0): resolve all MI/covariance/generic decisions here | BASE specs | Phase A |
| P2 | **L3** Terminology bundle (TERM 3.x) + service API | BASE | should compile (leaf) |
| P3 | **L4** RM transcription: data_types → data_structures → common → ehr → demographic → integration | RM 1.1.0 | Phase A |
| P4 | **L5a** Canonical JSON serialization (ITS-JSON) + insta golden vectors | RM | Phase A |
| P5 | **L5b** Canonical XML serialization (ITS-XML 1.0.2 + 2.0.0) | RM | Phase A |
| P6 | REST skeleton (axum) matching every ITS-REST 1.0.3 endpoint + admin + item tags; utoipa from OAS | RM, serde | Phase A |
| P7 | Persistence schema + Flyway migrations copied verbatim; sea-query tables; testcontainers PG18 | — | should compile |
| P8 | **L6a** ODIN + BMM parsers | LANG | Phase A |
| P9 | **L6b** ADL 1.4 + AOM 1.4 + OPT 1.4 XML (the EHRbase-compatible slice); ADL 2/AOM 2 in parallel behind `adl2` | AM 2.3.0, ODIN, BMM | Phase A |
| P10 | WebTemplate builder (OptVisitor equivalent) | ADL | Phase A |
| P11 | Composition validation (ValidationWalker equivalent) + terminology binding | RM, WebTemplate | Phase A |
| P12 | **L6c** AQL parser + AST + semantic path analysis | QUERY 1.1.0, ADL | Phase A |
| P13 | AQL engine: AST → ASL → SQL (JSONB extraction, current+history UNION, JSON_TABLE where viable) | AQL, persistence | Phase A |
| P14 | rm-db-format: RM ↔ decomposed row-per-locatable bridge | RM, persistence | Phase A |
| P15 | Service layer: orchestration, transactions, versioning, contributions, audit | all above | Phase A |
| P16 | FLAT / STRUCTURED / Web Template (Better semantics) + EhrScape | flat, adl | Phase A |
| P17 | **Make it compile.** Tier crates leaf-first; drive `cargo check` to zero across the workspace | — | make-it-compile |
| P18 | **Test parity.** openEHR conformance corpora + EHRbase test-data + archie golden vectors; insta snapshots; target ≥99% parity on Linux x86_64 first | — | parity |
| P19 | Optimization: PG18 AIO tuning, pipelining for hot AQL reads, JSON_TABLE codegen | — | perf |
| P99 | Cutover: delete the last remaining ported-out Java files and any residual Maven config; final docs; tag the first pure-Rust release | — | — |

### Stage 2 — enterprise feature restoration
Begins only after P18 parity holds. See Section 11. Catalogue now; build later.

### Stage 3 — codebase improvement
Idiomatic refactors, performance, new capabilities, upstream-worthy cleanups. Only after Stage 2.

---

## 11. Enterprise features to recover (Stage 2 backlog)

EHRbase removed capabilities between its pre-v2 line and v2. These must come back after the port. **Do not build them during Stage 1.** Phase 0 includes an archaeology task to make this list exact.

### 11.1 Archaeology task (run in P0, do not act on until Stage 2)
1. Pin the last pre-v2 (v1.x / 0.x) tag and keep it as a read-only `reference/v1` git ref in the fork.
2. Diff the `reference/v1` tag against v2.31.0 (before the Phase 0 reorganization, i.e. `git diff <v1-tag> <v2-tag>` at their original Maven paths): produce `docs/enterprise/v1-vs-v2-delta.md` listing removed modules, classes, endpoints, config, and DB objects.
3. For each removed capability, record: what it did, why it was removed (release notes/commits), and whether it is in scope for restoration.

### 11.2 Candidate list (confirm via 11.1)
Treat as candidates, not confirmed inventory, until the diff is done:
- **RBAC / access control.** Role-based (and attribute-based/ABAC) authorization over EHR/composition/query operations, including any row-level or AQL-result filtering by permission. Highest-priority restoration.
- **Security integration.** Spring Security equivalents: authn (Basic/OAuth2/OIDC, Keycloak), token handling, and the security event/audit surface. In Rust: tower middleware + an OAuth2/OIDC layer (PG 18 also adds server-side OAuth support).
- **Plugin system.** EHRbase's PF4J-based extension SPI. Rust has no PF4J; design a replacement (trait-object registry, dynamic `cdylib` loading, or WASM plugins) as an ADR in Stage 2.
- **Multi-tenancy.** The tenant model (`ehr.tenant`) and per-tenant isolation, if present in v1 in a richer form.
- **Any commercial/enterprise connectors or admin tooling** surfaced by the diff.

Each confirmed item becomes its own Stage 2 phase file (`docs/plans/s2-phase-NN-*.md`) with the same check-off discipline.

---

## 12. Claude Code harness to scaffold

Generate this tree in P0.

```
.claude/
├── settings.json                 # hooks registration, model prefs
├── settings.local.json           # personal, gitignored
├── rules/
│   ├── rust-style.md             # paths: ["crates/**/*.rs"]
│   ├── sqlx-conventions.md       # paths: ["crates/openehr-server/**"]
│   ├── testing.md
│   ├── rm-transcription.md       # paths: ["crates/openehr-rm/**","crates/openehr-base/**","crates/openehr-foundation/**"]
│   ├── aql-engine.md             # paths: ["crates/openehr-aql/**","crates/openehr-server/src/aql/**"]
│   └── serialization.md          # paths: ["crates/openehr-serde/**","crates/openehr-flat/**"]
├── skills/
│   ├── port-file/                # /port-file <java-path> — port a moved .java into a .rs beside it
│   ├── transcribe-rm-class/      # /transcribe-rm-class <ClassName> — literal spec transcription helper
│   ├── rosetta-mapping/          # append Java↔Rust or spec↔Rust mappings
│   ├── run-parity-test/          # fork a subagent; run the parity harness
│   ├── phase-status/             # dynamic injection: current phase + git status
│   ├── next-task/                # /next-task
│   ├── phase-done/               # /phase-done
│   ├── crate-scaffold/           # /crate-scaffold <name>
│   └── write-adr/                # /write-adr <title>
├── agents/
│   ├── porter.md                 # Sonnet; ports one moved .java into a .rs beside it; worktree-isolated
│   ├── rm-transcriber.md         # Sonnet; literal RM/BASE spec transcription
│   ├── port-reviewer.md          # Opus; read-only fidelity review
│   ├── test-runner.md            # Haiku; runs cargo+parity; failures only
│   ├── parity-checker.md         # Sonnet; end-of-phase parity harness
│   ├── rosetta-curator.md        # maintains docs/ROSETTA.md
│   └── docs-writer.md            # Haiku; ADRs and PROGRESS.md
└── hooks/
    ├── protect_java.sh           # PreToolUse Write|Edit — block edits to *.java lacking a Rust counterpart, and to pom.xml/mvnw/.mvn (exit 2)
    ├── no_attribution_guard.sh   # PreToolUse Bash — block `git commit`/`gh pr create`/`git push` carrying Claude/AI attribution (exit 2)
    ├── block_dangerous.sh        # PreToolUse Bash — block rm -rf, force-push, deletion of docs/plans/*
    ├── rust_fmt_clippy.sh        # PostToolUse Edit|Write — cargo fmt + clippy on the edited crate
    ├── inject_phase_context.sh   # SessionStart — cat docs/plans/current-phase.md + git status + last 10 commits
    └── phase_gate.sh             # Stop — block if no phase checkbox was ticked and no commit made this session
```

### Root `CLAUDE.md` skeleton (generate ~150 lines from this)

Sections, in order:
1. **Project one-liner + repo map.** "Pure-Rust 1:1 port of EHRbase in a single root Cargo workspace. Java and Rust coexist inside `crates/` during the port; port each `.java` to a `.rs` beside it, then delete the Java. See `PORT_MASTER_PLAN.md`."
2. **Phase workflow** (the six-step loop in Section 13.2).
3. **Tech stack** (Rust 1.96, edition 2024, PG 18, tokio, axum, sqlx + sea-query, jiff, uuid v7; openEHR spec matrix from Section 3).
4. **Build/test commands** (`cargo nextest run --workspace`, `cargo clippy --workspace --all-targets`, parity harness invocation).
5. **Conventions** (crate boundaries per Section 9; `thiserror` in libs / `anyhow` in bins only; no `unwrap` outside tests; async-first; enums-over-trait-objects for closed RM polymorphism).
6. **IMPORTANT hard rules** (never edit a `.java` file that does not yet have a completed Rust counterpart; never edit Maven build files `pom.xml`/`mvnw`/`.mvn`; delete a Java file only in the same phase its Rust replacement reaches parity; every ported/transcribed file ends with a `// PORT STATUS` trailer; use `// TODO(port)`/`// PERF(port)`/`// PORT NOTE:`/`// SAFETY:`; branches are `claude/*`; never weaken a test; tick the phase checkbox before committing; prefer subagents over inline work; Phases P1–P16 do not need to compile).
7. **References** (`@AGENTS.md`, `@PORT_MASTER_PLAN.md`, `@docs/ROSETTA.md`, `@docs/PORTING.md`, `@docs/plans/current-phase.md`).

Keep each `CLAUDE.md` under ~200 lines. Reserve `IMPORTANT`/`YOU MUST` for the two or three genuinely critical rules. Running plans live in phase files, never in `CLAUDE.md`.

### Skill frontmatter template (all skills)
```yaml
---
name: <skill-name>
description: <what it does + when to use it, one or two sentences, concrete triggers>
allowed-tools: [Read, Edit, Write, Grep, Glob, Bash]   # trim to minimum
argument-hint: "<expected args>"
---
```

### Subagent frontmatter template
```yaml
---
name: porter
description: Ports one moved Java file into a Rust file beside it in the same crate, faithfully. Use proactively for per-file port work.
tools: [Read, Edit, Write, Grep, Glob, Bash]
model: sonnet
permissionMode: acceptEdits
isolation: worktree
memory: project
skills: [rosetta-mapping]
---
```

### Hook behaviour summary
- `protect_java.sh`: exit 2 if the edit targets a `.java` file that has no completed Rust counterpart in the same directory, or any Maven build file (`pom.xml`, `mvnw`, `mvnw.cmd`, `.mvn/**`).
- `no_attribution_guard.sh`: exit 2 if a `git commit`, `gh pr create`, or `git push` command carries any Claude/AI attribution token (`Co-authored-by: ...claude/anthropic/[bot]`, "Generated with/by ... Claude", "Claude Code", 🤖, "Assisted-by: Claude"). This is the pre-emptive layer of the no-attribution rule.

### Attribution stripping (two layers, both required)

The no-attribution rule (Section 12 hard rules) is enforced mechanically, not left to memory:

1. **Pre-emptive (Claude Code):** `no_attribution_guard.sh` above blocks a commit/PR command before it runs if its text carries attribution, forcing removal first.
2. **Mechanical backstop (git):** a tracked **`.githooks/commit-msg`** hook rewrites every commit message in place, deleting any `Co-authored-by`/"Generated with Claude"/🤖/"Assisted-by"/`Claude Code` lines and trimming the resulting blank space. It runs on every commit including `--amend`, squash, and merge, so attribution cannot land in history even if it slips past layer 1. Install with `scripts/install-hooks.sh`, which sets `git config core.hooksPath .githooks` (the hook is version-controlled and shared, unlike `.git/hooks/`). Generate `.githooks/commit-msg` and `scripts/install-hooks.sh` in Phase 0, and run the installer as a bootstrap step.
- `block_dangerous.sh`: block `rm -rf`, `git push --force` to main, and deletion of `docs/plans/*`.
- `rust_fmt_clippy.sh`: `cargo fmt` + `cargo clippy --fix` scoped to the edited crate.
- `inject_phase_context.sh`: print `docs/plans/current-phase.md`, `git status`, `git log --oneline -10` at session start.
- `phase_gate.sh`: exit 2 at Stop unless a new `- [x]` was added to the current phase file or a commit was made this session.

---

## 13. docs/ layout and the checklist convention

```
docs/
├── VERSIONS.md                   # the pinned matrix from Section 3
├── ROSETTA.md                    # Java→Rust and spec→Rust mapping (living)
├── PORTING.md                    # the rule set from Section 14
├── LIFETIMES.tsv                 # pre-computed ownership classification (Section 4.6)
├── architecture.md              # target architecture overview
├── research/
│   ├── 01-port-methodology-and-platform.md
│   └── 02-openehr-spec-surface.md
├── enterprise/
│   └── v1-vs-v2-delta.md         # output of the archaeology task (Section 11.1)
├── ADRs/
│   └── ADR-000-template.md
├── PROGRESS.md                   # one line per phase, updated at completion
└── plans/
    ├── README.md
    ├── current-phase.md          # 3-line pointer: phase file path, session goal, next action
    ├── phase-00-scaffolding.md
    ├── phase-01-foundation-identification.md
    ├── phase-02-terminology.md
    ├── phase-03-rm.md
    ├── phase-04-serialization-json.md
    ├── phase-05-serialization-xml.md
    ├── phase-06-rest-skeleton.md
    ├── phase-07-persistence-schema.md
    ├── phase-08-odin-bmm.md
    ├── phase-09-adl-aom-opt.md
    ├── phase-10-webtemplate.md
    ├── phase-11-validation.md
    ├── phase-12-aql-parser.md
    ├── phase-13-aql-engine.md
    ├── phase-14-rm-db-format.md
    ├── phase-15-service-layer.md
    ├── phase-16-flat-structured-ehrscape.md
    ├── phase-17-make-it-compile.md
    ├── phase-18-test-parity.md
    ├── phase-19-optimization.md
    └── phase-99-cutover.md
```

### 13.1 Phase file template
```markdown
# Phase NN — <title>

- Status: not-started | in-progress | blocked | done
- Started: <date>   Owner: <name>
- Consumes (spec/layer): <e.g. RM 1.1.0 / Layer 4>
- Compile required: no (Phase A) | yes (make-it-compile) | parity

## Objectives
<what this phase delivers>

## Preconditions
- [ ] <prior phase(s) complete>

## Scope
In: <...>
Out: <...>

## Tasks
- [ ] <task 1>
- [ ] <task 2>

## Exit criteria
- [ ] <verifiable condition>

## Decisions made this phase
- <ADR links, structural choices>

## Handoff for next session
<one paragraph: where things stand, what to do next>
```

### 13.2 The six-step loop (put this in CLAUDE.md)
1. Read `docs/plans/current-phase.md`.
2. Pick the next unchecked task in the referenced phase file.
3. Do the work (delegate to a subagent when it would otherwise burn context).
4. Tick the `- [ ]` → `- [x]`, add a one-line note.
5. Commit as `phase-NN: <task>` on a `claude/phase-NN-*` branch.
6. If the phase's exit criteria are all met, run `/phase-done`, update `PROGRESS.md`, and advance `current-phase.md`.

Markdown checkboxes on disk survive `/clear` and `/compact`; the built-in todo tool is session-scoped only, so the phase files are the durable layer.

---

## 14. PORTING.md rule set (the Rosetta core)

Generate `docs/PORTING.md` from this. It is a lookup table, not prose. Two mapping domains: **Java → Rust** (for the EHRbase application code) and **openEHR spec → Rust** (for the native transcription).

### 14.1 Ground rules
- Mirror the source: same file/module names, same type names, same method names, same field order, same control flow.
- Phases P1–P16 need not compile. Capture intent. Leave `todo!()` and `// TODO(port):` freely.
- Every file ends with the PORT STATUS trailer (Section 4.3).
- Prefer closed Rust enums for closed source hierarchies; trait objects only for open, archetype-driven polymorphism.

### 14.2 Java → Rust type map
| Java | Rust |
|---|---|
| `String` | `String` (owned) / `&str` (borrowed) |
| `boolean`/`int`/`long`/`double` | `bool`/`i32`/`i64`/`f64` |
| `Integer`/`Long`/`Double` (nullable) | `Option<i32>`/`Option<i64>`/`Option<f64>` |
| `BigDecimal` | `rust_decimal::Decimal` (add to stack if needed) |
| `byte[]` | `Vec<u8>` |
| `List<T>` | `Vec<T>` |
| `Set<T>` | `HashSet<T>` / `BTreeSet<T>` |
| `Map<K,V>` | `HashMap<K,V>` / `BTreeMap<K,V>` |
| `Optional<T>` | `Option<T>` |
| `UUID` | `uuid::Uuid` |
| `Instant`/`OffsetDateTime`/`LocalDate` | `jiff::Timestamp`/`jiff::Zoned`/`jiff::civil::Date` |
| `enum` | `enum` |
| interface with impls | `trait` + impls, or closed `enum` if the impl set is closed |
| `Stream<T>` pipeline | iterator chain |

### 14.3 Java → Rust idiom map
| Java idiom | Rust idiom |
|---|---|
| checked/unchecked exception | `Result<T, E>` with `thiserror` error enums |
| constructor that throws | `fn new(...) -> Result<Self, E>` |
| `AutoCloseable` / `close()` | `impl Drop` |
| `null` | `Option<T>` |
| inheritance (`extends`) | composition + trait, or enum variant |
| abstract class with fields | a shared struct embedded by concrete types |
| generics `<T extends X>` | `<T: XTrait>` |
| builder pattern | builder struct or `#[derive(bon::Builder)]`/typed-builder |
| `equals`/`hashCode` | `#[derive(PartialEq, Eq, Hash)]` |
| `toString` | `impl Display` |
| Jackson `@JsonProperty` | serde `#[serde(rename = "...")]` |
| Spring `@RestController` | axum handler + router |
| Spring DI (`@Autowired`) | explicit constructor injection / `axum` state |
| jOOQ DSL | sea-query builder + sqlx execution |

### 14.4 openEHR spec → Rust mappings (the literal-transcription rules)
- One RM class → one Rust struct or enum, named identically (upper snake preserved as Rust type case, e.g. `DV_TEXT` → `DvText`; keep the openEHR name in a doc comment and in serde rename).
- Abstract RM class with attributes → a struct the concrete types embed (composition), plus a marker trait if behaviour is shared.
- Closed subtype set (`DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`, `VERSION<T>`) → closed `enum`.
- Constrained generic (`DV_INTERVAL<T: DV_ORDERED>`) → generic with a trait bound.
- Covariant redefinition → encode the narrowed type directly on the concrete struct; document the override.
- Multiple inheritance → compose fields from all parents; implement each parent behaviour as a trait.
- `PATHABLE.parent()` → `Weak<..>` or path-index; never an owning back-reference.
- Recursive containment (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`, `DV_MULTIMEDIA.thumbnail`) → `Box`/`Vec<Box<..>>`.
- Symbolic operators (`++`, `and then`, `∀`) → named methods.
- serde: snake_case attribute names; `_type` discriminator; omit nulls; UIDs as `{_type, value}`; validate against `openehr_rm_1.1.0_all.json`.

### 14.5 Do-not-translate list
- Do not port JVM-specific plumbing (classloaders, Spring context internals, PF4J internals) literally; record a `// PORT NOTE:` and design the Rust equivalent in the relevant phase or defer to Stage 2.
- Do not port `archie`/JVM-SDK internals at all; we transcribe from the specs instead.
- Do not port build tooling (Maven) literally; use Cargo.

### 14.6 Trailer format
Repeat the PORT STATUS block from Section 4.3 verbatim at the end of every ported/transcribed file.

---

## 15. Testing and parity strategy

- **Oracles.** openEHR conformance corpora, the EHRbase `test-data` and `serialisation_conformance_test` sets, Better's `web-template-tests`, and openEHR reference archetypes. These are the acceptance authority for a greenfield-but-faithful implementation.
- **Snapshots.** `insta` pins canonical JSON/XML output against golden vectors; `cargo insta review` for intentional changes; redactions for volatile fields (timestamps, generated UIDs).
- **Properties.** `proptest` for RM round-trip (serialize→parse→equal), parser stability, and AQL parse/print round-trips.
- **Database.** `testcontainers` runs a real PostgreSQL 18 in Docker; `sqlx::test` fixtures; verify migrations apply cleanly.
- **Runner.** `cargo-nextest`.
- **Parity harness.** A cross-check that drives the Rust server and a stock Java EHRbase with identical requests and diffs responses. Gate every claimed-equivalent behaviour behind the negative test (`USE_REFERENCE_EHRBASE=1`): the test must fail against stock EHRbase without our fix.
- **Target.** ≥99% behavioural parity at the REST surface on Linux x86_64 first, then broaden.

---

## 16. Bootstrap checklist for Claude Code (do this first)

1. Confirm the pinned versions (Section 3) and write `docs/VERSIONS.md` and `rust-toolchain.toml`.
2. Fork is assumed already created. Pin the last pre-v2 tag as a read-only `reference/v1` git ref (do not merge it). Run the archaeology diff (Section 11.1) into `docs/enterprise/v1-vs-v2-delta.md` while the tree is still in its original Maven layout.
3. **Phase 0 reorganization:** create the root `Cargo.toml` workspace and the ten empty `openehr-*` spec crates, then `git mv` the EHRbase Java into the three server crates per the Section 9.1 mapping. Confirm the workspace builds (empty + moved crates).
4. Generate the `.claude/` harness (Section 12): `settings.json`, `rules/*`, `skills/*`, `agents/*`, `hooks/*` (including `no_attribution_guard.sh`). Also generate the tracked git hook `.githooks/commit-msg` and `scripts/install-hooks.sh`, then run `bash scripts/install-hooks.sh` to set `core.hooksPath`.
5. Generate `docs/` (Section 13): `ROSETTA.md`, `PORTING.md` (from Section 14), `PROGRESS.md`, `ADRs/ADR-000-template.md`, `plans/*` phase files (from the template), and seed `LIFETIMES.tsv`.
6. Commit the two research dossiers to `docs/research/`.
7. Write the root `CLAUDE.md` (from Section 12) and the `AGENTS.md` symlink.
8. Kick off the P0 archaeology task (Section 11.1) to produce `docs/enterprise/v1-vs-v2-delta.md` (record only; do not build).
9. Set `docs/plans/current-phase.md` to point at `phase-00-scaffolding.md`, then begin the six-step loop.

Once P0 is done, advance to P1 (Foundation + Identification) and start the literal RM-adjacent transcription. Compilation is not required until P17.

---

*End of master plan. Keep this file authoritative; when scope changes, update it here and let the phase files and `CLAUDE.md` reference it.*
