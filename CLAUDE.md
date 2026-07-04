# CLAUDE.md

A pure-Rust openEHR CDR that is **spec-conformant and behavior-compatible with EHRbase** at the REST/AQL surface, in a single root Cargo workspace. The openEHR spec + serialization + REST-contract layer is **generated** from the official machine-readable specs (ADR-004/005); the EHRbase application is **built as modern idiomatic Rust on top of the generated crates** (ADR-006) — **not** a 1:1 Java-structure port. **Read `docs/ADRs/ADR-004`, `ADR-005`, `ADR-006` before non-trivial work.** Running plans live in `docs/plans/` (the roadmap; the phase files + `docs/PROGRESS.md` are authoritative over `PORT_MASTER_PLAN.md` prose where they differ). `PORT_MASTER_PLAN.md` is a historical bootstrap doc, amendment-redirected to the phase files.

## Repo map

Single workspace. **Crate naming:** `openehr-*` = the openEHR **specification** (generated from the vendored BMM/XSD/OAS, ADR-004/005 — treat as `// @generated`); `ehrbase-*` = the **EHRbase application** (idiomatic Rust consuming the `openehr-*` crates, ADR-006). EHRbase's Java lives in the `ehrbase-*` crates as the **read-only behavioural reference** (deleted per-subsystem at parity, P99).

- `crates/openehr-base`, `openehr-rm`, `openehr-am`, `openehr-term`, `openehr-lang` — **generated** spec crates (`openehr-codegen -- emit`). `openehr-its` — canonical JSON + **generated** XML (`emit-xml`) + **generated** ITS-REST contract (`emit-rest`) + hand-written runtimes. `openehr-query` — hand-written AQL lexer/parser/AST. `openehr-flat` — FLAT/STRUCTURED (hand-written). `openehr-codegen` (BMM/XSD/OAS→Rust generator) + `openehr-derive` (proc-macro) are the hand-written tooling.
- `crates/ehrbase-rest`, `ehrbase-compat`, `ehrbase` — the EHRbase application (built idiomatically; `ehrbase` is the binary). These carry the read-only Java reference.
- `docs/` — plans (the roadmap), ADRs, VERSIONS, architecture, postgres-features, research, enterprise.
- `.claude/` — rules, skills, hooks (no agents/worktrees — all work is done in-session).

## Code generation (ADR-004) — READ THIS FIRST

**The openEHR spec crates are GENERATED from the vendored BMM meta-model, not hand-written.** Do **not** hand-transcribe or hand-edit RM/BASE/AM classes — that era is over (see `docs/ADRs/ADR-004-spec-driven-codegen.md`). openEHR publishes a machine-readable meta-model (BMM); we generate idiomatic Rust from it deterministically.

- **Pipeline:** vendored `*.bmm.json` (in `crates/openehr-codegen/vendor/bmm/`) → `openehr-lang::bmm` (loader) → `openehr-codegen` (emitter) → the spec crates. `openehr-derive`'s `#[derive(OpenEhrType)]` supplies canonical-JSON `_type` (de)serialization on the emitted types.
- **Regenerate:** `cargo run -p openehr-codegen -- emit` (spec crates from BMM) · `-- emit-xml` (canonical-XML `ToXml`/`FromXml` into `openehr-its`, ADR-005) · `-- emit-rest` (the ITS-REST contract into `openehr-its`, ADR-005) · `-- check`/`check-xsd` (validate inputs). The `/regen-codegen` skill runs all three + the drift check; a `codegen-drift` CI job guards it.
- **Generated crates** (`openehr-base`, `openehr-rm`, `openehr-am` with both `am14`/`am24`): every type file starts with `// @generated … DO NOT EDIT`. **Never hand-edit a generated file.** To change output, edit the emitter (`crates/openehr-codegen/src/emit.rs`) or its override map, then regenerate. `openehr-foundation` no longer exists (folded into `openehr-base`).
- **Hand-written spec behaviour** (invariants, spec functions per ADR-003) lives in sibling `*_impl.rs` files the generator never rewrites.
- **Hand-written tooling** (edit freely, normal Rust): `openehr-lang` (ODIN + BMM reader), `openehr-codegen` (emitter), `openehr-derive` (proc-macro).
- **Partly generated:** `openehr-its` — the XML `ToXml`/`FromXml` impls (`emit-xml`) and the ITS-REST contract (`emit-rest`) are generated into `src/xml/generated/` + `src/rest/generated/`; the hand-written parts are the runtimes (`xml/runtime.rs`, `rest/runtime.rs`), the canonical-JSON entry points + validation, and the fidelity gates.
- **NOT generated** (hand-written): `openehr-term` (terminology bundle + XML assets + access logic — BMM only has ~6 interface classes), `openehr-query` (AQL lexer/parser/AST), `openehr-flat` (SDT), and the `ehrbase-*` application crates (idiomatic Rust on the generated crates, EHRbase Java as reference — ADR-006).
- **Pinned spec versions:** RM 1.2.0, BASE 1.3.0, TERM 3.1.0, AM 1.4.0 + 2.4.0 (see `docs/VERSIONS.md`). These are the latest; they diverge from what stock EHRbase/`archie` emits (RM 1.1.0-era) — a Stage-1 REST-parity consideration.

## Phase workflow (the loop)

1. Read `docs/plans/current-phase.md`.
2. Pick the next unchecked task in the referenced phase file.
3. Do the work **in this session** — no subagent or worktree delegation (build in the open so it can be watched and corrected). For the openEHR **spec** layer, change the generator and regenerate (never hand-write spec classes — see Code generation above). For the EHRbase **application** (`ehrbase-*`), build **idiomatic modern Rust on top of the generated `openehr-*` crates** (ADR-006), consulting the in-tree EHRbase Java as the *behavioural reference* — not a per-file 1:1 port. Build compiling, tested increments.
4. Tick the task `- [ ]` to `- [x]` and add a one-line note.
5. Commit as `phase-NN: <task>` on a `claude/phase-NN-*` branch.
6. When the phase's exit criteria are all met, run `/phase-done`, update `docs/PROGRESS.md`, and advance `current-phase.md`.

Checkboxes on disk survive `/clear` and `/compact`; the built-in todo tool is session-scoped, so the phase files are the durable layer.

## Model orchestration (workflows & subagents)

**When the session runs on Fable 5 (effort `high`), Fable is the orchestrator, not the implementer.** Fable plans, coordinates, reviews, and does the taste- and intelligence-heavy work itself; it fans implementation out to subagents via the `Agent` tool (`model: 'opus'`) or a `Workflow` (per-agent `model`). The main loop does not auto-delegate — this section is the standing instruction that it should.

Why this split (not "spin everything to a cheaper worker"): the win is **context isolation, parallelism, and sparing the orchestrator's capacity** — keep Fable's context clean and let Opus workers grind through file-heavy implementation in parallel. It is *not* an intelligence upgrade (see the table — Fable outranks Opus on both intelligence and cost), so delegate by the *nature of the work*, never reflexively.

Rankings, higher = better. `cost` is relative spend, `intelligence` is how hard a problem the model can be handed unsupervised, `taste` covers code quality / API design / clarity. Models are the ones the `Agent`/`Workflow` `model` parameter accepts.

| model     | cost | intelligence | taste |
|-----------|------|--------------|-------|
| fable-5   | 2    | 9            | 9     |
| opus-4.8  | 4    | 7            | 8     |
| sonnet-5  | 5    | 5            | 7     |
| haiku-4.5 | 1    | 4            | 3     |

How to apply (these are defaults, not limits — override when the output doesn't meet the bar; intelligence > taste > cost when axes conflict for anything that ships):

- **Orchestrator (Fable 5, high):** owns the phase loop, architecture, ADR decisions, spec-parity judgement, and the hard bespoke logic (AQL AST→ASL→SQL, versioning, validation, RM↔JSONB). Keeps these in-context rather than delegating — they need top intelligence + taste and are the project's critical path.
- **Delegate to Opus-4.8 subagents:** bulk/parallelizable implementation on a clear spec (wiring handlers, DTO/trait impls against the generated ITS-REST contract, migrations, sqlx/sea-query query building, test scaffolding), file-heavy investigation, and codebase analysis — anything that would otherwise burn the orchestrator's context. Fan out several concurrently in one message.
- **Fable-5 subagents:** use when a delegated task still needs top intelligence/taste (a tricky algorithm port, an API-shape decision) but you want it off the main context.
- **sonnet-5:** cheap mechanical passes where correctness is easy to verify (mechanical refactors, boilerplate). **Never use Haiku for substantive work.**
- **Reviews:** an Opus-4.8 (or Fable-5) reviewer subagent, read-only, as an independent perspective before committing a subsystem — especially spec/wire parity and the AQL engine.
- Effort: keep Fable on `high` (xhigh is token-hungry; max/extra is a furnace for worse output). Use `effort: 'low'` for cheap mechanical worker stages, higher tiers only for the hardest verify/judge stages.

Discipline unchanged: subagents still obey the hard rules below (never hand-edit `// @generated`, never edit unported `.java`/Maven files, no test-weakening, `claude/*` branches, no AI attribution) — deviations surface at the parity harness, so delegate with a tight spec and verify the result.

## Tech stack (pinned)

Toolchain: Rust stable **1.96** (1.96.1), MSRV 1.96, **edition 2024**, resolver v3. Pin via `rust-toolchain.toml`.
Database: **PostgreSQL 18** (target 18.4+): AIO, `uuidv7()`, skip scan, temporal constraints, RETURNING OLD/NEW, plus `JSON_TABLE` from PG 17. Extensions: `uuid-ossp`, `pgcrypto`, `pg_trgm`.

**The authoritative, fully-pinned dependency set lives in the root `Cargo.toml` `[workspace.dependencies]`.** Versions below are current as of 2026-07; do not hand-roll anything a crate here already provides (auth, HTTP status codes, OpenAPI/Swagger, etc.). Add a crate to a member with `dep.workspace = true`. Items marked *(verify)* had their major/minor unconfirmed at authoring; check crates.io before first use.

Web & HTTP core: `axum` 0.8, `axum-extra` 0.10, `axum-server` 0.7 (graceful shutdown + TLS), `tower` 0.5, `tower-http` 0.6 (trace, cors, compression, timeout, limit, request-id, sensitive-headers, catch-panic, normalize-path), `hyper` 1, `hyper-util` 0.1, `http` 1 (status codes/headers), `http-body` 1, `http-body-util` 0.1, `mime` 0.3, `mime_guess` 2, `headers` 0.4, `bytes` 1.

Async runtime: `tokio` 1, `tokio-util` 0.7, `tokio-stream` 0.1, `futures` 0.3, `async-trait` 0.1, `pin-project-lite` 0.2.

Auth & authz (never hand-roll): `jsonwebtoken` 9 (JWT), `oauth2` 5 *(verify)*, `openidconnect` 4 (OIDC/Keycloak) *(verify)*, `argon2` 0.5 + `password-hash` 0.5 (password hashing), `tower-sessions` 0.14 *(verify)*, `axum-login` 0.17 *(verify)*, `secrecy` 0.10, `zeroize` 1. RBAC/ABAC for the Stage 2 restoration: `casbin` 2 *(verify)* or `cedar-policy` 4 *(verify)* (decide at S2).

TLS/crypto: `rustls` 0.23, `tokio-rustls` 0.26, `rustls-pemfile` 2, `webpki-roots` 0.26, `rand` 0.9, `getrandom` 0.3, `sha2` 0.10, `hmac` 0.12, `blake3` 1.

OpenAPI/Swagger: `utoipa` 5, `utoipa-axum` 5, `utoipa-swagger-ui` 9 (serves Swagger UI), `utoipa-redoc`/`utoipa-scalar`/`utoipa-rapidoc` 5. Note (ADR-005): the ITS-REST contract is **spec-first** — the vendored OpenAPI is authoritative and is generated into `openehr-its` (`emit-rest`); `utoipa` serves Swagger UI and an optional code→OAS drift-check, not the source of truth.

Database & persistence: `sqlx` 0.9 (postgres, macros, migrate, uuid, json, rust_decimal, chrono; TLS via `tls-rustls-aws-lc-rs`) — note sqlx has no `jiff` feature, bridge manually; `sea-query` 1.0 + `sea-query-binder` (dynamic SQL for the AQL→SQL engine — **not** sea-orm, ADR-006); `deadpool-postgres` + `tokio-postgres` for an optional pipelined hot-read path.

Serialization & formats: `serde` 1, `serde_json` 1 (`preserve_order`), `serde_with` 3, `serde_path_to_error` 0.1, `quick-xml` 0.37 *(verify)* (with `serialize`), `base64` 0.22, `rust_decimal` 1 (+`rust_decimal_macros`) as the BigDecimal replacement for DV_QUANTITY, `ordered-float` 4, canonical JSON via `serde_jcs` 0.1 *(verify)* or a ~150-LoC hand-roll. C14N (canonical XML): `xmllint --c14n` fallback for now.

Parsers (native ADL/cADL/ODIN/AQL): `logos` 0.15 (lexer), `chumsky` 0.10 (usable stable; 1.0 still alpha, repo now on Codeberg) or `winnow` 0.7, `regex` 1, `fancy-regex` 0.14 *(verify)* for cADL backreferences; diagnostics via `miette` 7 and/or `ariadne` 0.5 *(verify)*.

IDs / time / validation: `uuid` 1 (v4+v7, serde, fast-rng), `jiff` 0.2 (1.0 not yet released as of 2026-07), `garde` 0.22 *(verify)* + a custom RM-invariant framework, `url` 2.

Observability (opentelemetry set is lockstep — keep equal): `tracing` 0.1, `tracing-subscriber` 0.3, `tracing-opentelemetry` 0.33, `opentelemetry` 0.31, `opentelemetry_sdk` 0.31, `opentelemetry-otlp` 0.31, `opentelemetry-semantic-conventions` 0.31, `metrics` 0.24, `metrics-exporter-prometheus` 0.16 *(verify)*, `axum-prometheus` 0.8 *(verify)*.

Caching / rate limiting / resilience: `moka` 0.12 (Caffeine equivalent for the template/WebTemplate cache), `quick_cache` 0.6, `tower_governor` 0.4 *(verify)* + `governor` 0.6 *(verify)*, `backon` 1 (retry; the `backoff` crate is deprecated).

Errors & utilities: `thiserror` 2 (libs), `anyhow` 1 (bins only), `config` 0.14 or `figment` 0.10, `dotenvy` 0.15, `clap` 4, `parking_lot` 0.12, `dashmap` 6, `arc-swap` 1, `indexmap` 2, `smallvec` 1, `itertools` 0.14, `bitflags` 2. Use `std::sync::LazyLock` (edition 2024) instead of `once_cell` for statics.

HTTP client & external integration: `reqwest` 0.12 (rustls, json) for the terminology/FHIR client and parity harness; `jsonschema` 0.26 *(verify)* to validate against the openEHR ITS-JSON schemas.

Testing & benches (dev-deps): `cargo-nextest`, `insta` 1 (snapshots — the key tool for canonical JSON/XML parity), `proptest` 1, `rstest` 0.23 *(verify)*, `wiremock` 0.6, `mockall` 0.13 *(verify)*, `fake` 3 *(verify)*, `assert_cmd` 2, `assert_fs` 1, `testcontainers` 0.24 *(verify)* + `testcontainers-modules` 0.12 *(verify)* (real PG 18), `criterion` 0.5 + `divan` 0.1.

Dev tooling (CI, not deps): `cargo-nextest`, `cargo-audit`, `cargo-deny`, `cargo-machete`, `cargo-hakari`, `cargo-llvm-cov`, `sccache`; `mold` linker on Linux.

openEHR spec versions are pinned in the "Code generation" section above (RM 1.2.0, BASE 1.3.0, TERM 3.1.0, AM 1.4.0 + 2.4.0) and in `docs/VERSIONS.md` (the single source of truth) — including the ITS-XML/REST/JSON pins.

## Build and test

```bash
cargo build --workspace
cargo nextest run --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
cargo audit && cargo deny check
# parity harness (Stage 1 acceptance): drives the Rust server and a stock EHRbase and diffs responses
scripts/parity.sh              # add USE_REFERENCE_EHRBASE=1 for the negative-test gate
```

Note (ADR-006 superseded the old "phases need not compile" gate): the spec + ITS
foundation is generated and **compiling**, and the **application phases build as
compiling, tested increments** on top of it. Do not defer compilation; keep every
crate you touch green. (The historical Bun-style "P1–P16 need not compile" rule
applied only while hand-transcribing the spec, which no longer happens.)

Compile status: the **generated** spec crates `openehr-base`, `openehr-rm`,
`openehr-am` compile and are lib-clippy-clean — keep them that way by fixing the
*emitter*, never by hand-editing generated files. `openehr-lang`,
`openehr-codegen`, `openehr-derive` (hand-written tooling), `openehr-term`
(hand-written), `openehr-query` (AQL parser), and `openehr-its` (canonical
JSON/XML + generated ITS-REST contract; all fidelity gates green) all compile +
clippy-clean. The `ehrbase-*` application crates are the remaining work (the
Stage-1 app build, `docs/plans/` phases 09–20) and are built compiling per phase.

## Conventions

- Crate boundaries mirror openEHR components. Keep dependencies pointing downward: app (`ehrbase-*`) → spec (`openehr-*`), never the reverse. The `ehrbase-*` crates consume the generated `openehr-*` types directly as their domain model — never re-model the RM or re-serialize.
- **Two disciplines by layer.** Spec/ITS crates (`openehr-*`) = *generated* from the vendored specs (ADR-004/005): change the emitter and regenerate, never hand-edit `// @generated`; the bar is wire + semantic + invariant parity. Application crates (`ehrbase-*`) = *modern idiomatic Rust* (ADR-006): use proper crates (axum, sqlx+sea-query, oauth2, utoipa), follow EHRbase's *behaviour/algorithm* as the reference (not its class structure), verify at the REST/AQL surface with the parity harness. Build compiling, tested increments.
- Emission choices the generator already makes (do not re-litigate per class): closed openEHR subtype sets → untagged Rust `enum`s; recursion (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`, `DV_MULTIMEDIA.thumbnail`, F-bounded ranges) → `Box`; `_type` via `#[derive(OpenEhrType)]`; strong types where unambiguous (`uuid::Uuid`, etc.). Behavioural back-references (`PATHABLE.parent()`) in hand-written `*_impl.rs` use `Weak` or an index, never an owning reference.
- `thiserror` in hand-written library crates; `anyhow` only in the binary. No `unwrap`/`expect` outside tests.
- Async-first: the server is I/O-bound on Postgres; use idiomatic tokio/axum (this is not the Bun "no-Tokio" case).

## IMPORTANT hard rules

- **Never hand-edit a `// @generated` file.** The generated spec crates (`openehr-base`, `openehr-rm`, `openehr-am`) are produced by `openehr-codegen`; edit the emitter or the `*_impl.rs` sibling and regenerate, never the generated file itself.
- **Never edit a `.java` file that has no completed Rust counterpart, and never edit Maven build files** (`pom.xml`, `mvnw`, `mvnw.cmd`, `.mvn/`). A `PreToolUse` hook enforces this. Delete a Java file only in the same phase its Rust replacement reaches parity.
- **No PORT STATUS trailer** (that was the retired 1:1-port convention). Generated files carry a `// @generated` header; application code is plain idiomatic Rust.
- **Use the annotation vocabulary** where relevant: `// TODO(port):` (unfinished), `// PERF(port):` (optimize after parity), `// PORT NOTE:` (a deliberate behavioural deviation from EHRbase, with the reason), `// SAFETY:` (any `unsafe`).
- **Branches are `claude/*`.** Never force-push `main`. Never delete files under `docs/plans/`.
- **NEVER add AI/Claude attribution to commits or PRs. This is an absolute rule with no exceptions.** Do not add a `Co-Authored-By: Claude` trailer (or any co-author trailer). Do not write "Generated with Claude Code", "🤖 Generated with...", "Co-authored-by: Claude", or any similar line, emoji, or footer in a commit message, commit body, commit trailer, PR title, PR description, PR comment, issue, or code comment. Commit messages and PR text describe only the change itself. If you ever find yourself about to add such a line, stop and remove it. When configuring git or opening PRs, do not pass any flag or template that injects attribution.
- **Never weaken, skip, or delete a test** to make the port pass, and never edit a test to route around a bug it exposes. A parity test is valid only if it still fails against stock EHRbase without our fix (`USE_REFERENCE_EHRBASE=1`).
- **Tick the phase checkbox and commit before ending a session.** A `Stop` hook enforces this.
- **Application phases build as compiling, tested increments** (ADR-006) on top of the generated `openehr-*` crates — do not defer compilation. (The old "phases need not compile" rule applied only to the retired hand-transcription era.)
- The v1 enterprise code (RBAC and others) is a Stage 2 concern. Do not build it during Stage 1; it lives only as the read-only `reference/v1` git ref until then.

## References

- @docs/plans/current-phase.md (what's next + the goal)
- @docs/ADRs/ADR-006-application-port-philosophy.md (the app-build philosophy — read first)
- @docs/ADRs/ADR-004-spec-driven-codegen.md + @docs/ADRs/ADR-005-its-codegen.md (the codegen — read before touching any `openehr-*` crate)
- @docs/architecture.md (the current design)
- @docs/VERSIONS.md + @docs/postgres-features.md (pins + the PG 17/18 features we use)
- @PORT_MASTER_PLAN.md (historical bootstrap; superseded by the phase files)
