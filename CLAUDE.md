# CLAUDE.md

A pure-Rust, **openEHR-spec-conformant** CDR (ITS-REST 1.0.3 + AQL 1.1) in a single root Cargo workspace, with greenfield PG18-native internals (ADR-008). The openEHR spec + serialization + REST-contract layer is **generated** from the official machine-readable specs (ADR-004/005); the application is **modern idiomatic Rust of our own design on top of the generated crates** (ADR-006/ADR-008): our own storage, versioning, and AQL engine, validated by the openEHR CNF conformance suite — EHRbase is prior art, not an oracle. **Read `docs/ADRs/ADR-008`, then ADR-004/005, before non-trivial work.** The forward product roadmap is the root `ROADMAP.md`; `docs/blueprint/00-THE-BLUEPRINT.md` is the spec-compliance ledger (§2 = the consolidated spec-gap surface); running plans live in `docs/plans/`. ROADMAP.md + the blueprint + the phase files + `docs/PROGRESS.md` are the authoritative record.

## Repo map

Single workspace. **Crate naming:** `openehr-*` = the openEHR **specification** (generated from the vendored BMM/XSD/OAS, ADR-004/005 — treat as `// @generated`); `ehrbase-*` = the **application** (idiomatic Rust of our own design consuming the `openehr-*` crates, ADR-006/008). The formerly in-tree EHRbase Java reference was removed by ADR-008 (git history / upstream repos are the prior-art record).

- `crates/openehr-base`, `openehr-rm`, `openehr-am`, `openehr-term`, `openehr-lang` — **generated** spec crates (`openehr-codegen -- emit`). `openehr-its` — canonical JSON + **generated** XML (`emit-xml`) + **generated** ITS-REST contract (`emit-rest`) + hand-written runtimes. `openehr-query` — hand-written AQL lexer/parser/AST. `openehr-flat` — FLAT/STRUCTURED (hand-written). `openehr-codegen` (BMM/XSD/OAS→Rust generator) + `openehr-derive` (proc-macro) are the hand-written tooling.
- `app/*` — the application, **three crates** (ADR-010, packaging redesigned by **ADR-011**): `ehrbase` (the binary + `Platform` impl: storage, service layer, AQL engine, versioning; the `signing` [VERSION.signature, was `ehrbase-signing`] + `system_log` [ATNA, was `ehrbase-audit`] modules), `ehrbase-rest` (ITS-REST protocol adapter + auth + the `access` authz module [was `ehrbase-authz`] + EhrScape), and `ehrbase-sm` (the protocol-free SM native-API catalog). `tools/*` — dev/verification tooling, **not part of the app**: `conformance` (the ECC runner) and `benchmark`. Workspace `members = ["crates/*", "app/*", "tools/*"]`. The service layer follows the openEHR **SM Platform Service Model** (ADR-010/011; SM component map in `docs/architecture.md`, vendored SM spec at `docs/specs/openehr/SM/`).
- `docs/` — **`docs/blueprint/` (the spec-compliance ledger + consolidated spec-gap surface in `00-THE-BLUEPRINT.md` §2)**, plans (active phases), ADRs, VERSIONS, architecture, postgres-features, design, conformance (generated reports), benchmarks; the forward product roadmap is the root `ROADMAP.md`. **`docs/specs/openehr/` — the vendored openEHR spec text + CNF test schedule (the conformance oracle; see its README + `/spec-lookup`).**
- `.claude/` — rules, skills, hooks, agents, **`memory/`** (the persistent agent memory, moved in-repo 2026-07-12 so it is visible and versioned; the harness memory dir under `~/.claude/projects/` is a symlink to it — never break that link). Agent defs (`spec-researcher`, `spec-conformance-reviewer`, `implementer`, `ui-implementer`, `leptos-reviewer`) are the delegation targets for the Model-orchestration section below; the orchestrator keeps the critical path in-session.

### Layered memory (nested CLAUDE.md — 2026-07-13)

**Every crate carries its own `CLAUDE.md`** (`app/*/CLAUDE.md`,
`crates/*/CLAUDE.md`, `tools/*/CLAUDE.md`) with crate-local discipline: role,
generated-vs-hand-written split, never-do rules, gates. Per the official
Claude Code memory docs, nested files load **on demand** when files in that
crate are read (not at launch) and are **not re-injected after `/compact`
until the next read** — therefore: repo-wide hard rules live ONLY in this
root file; crate-local detail lives in the crate file; never move a global
rule down into a nested file. Path-scoped deep-dives stay in
`.claude/rules/*.md` (with `paths:` frontmatter). When crate reality changes
(a module moves, a gate changes), update that crate's `CLAUDE.md` in the same
change.

## Code generation (ADR-004) — READ THIS FIRST

**The openEHR spec crates are GENERATED from the vendored BMM meta-model, not hand-written.** Do **not** hand-transcribe or hand-edit RM/BASE/AM classes — that era is over (see `docs/ADRs/ADR-004-spec-driven-codegen.md`). openEHR publishes a machine-readable meta-model (BMM); we generate idiomatic Rust from it deterministically.

- **Pipeline:** vendored `*.bmm.json` (in `crates/openehr-codegen/vendor/bmm/`) → `openehr-lang::bmm` (loader) → `openehr-codegen` (emitter) → the spec crates. `openehr-derive`'s `#[derive(OpenEhrType)]` supplies canonical-JSON `_type` (de)serialization on the emitted types.
- **Regenerate:** `cargo run -p openehr-codegen -- emit` (spec crates from BMM) · `-- emit-xml` (canonical-XML `ToXml`/`FromXml` into `openehr-its`, ADR-005) · `-- emit-rest` (the ITS-REST contract into `openehr-its`, ADR-005) · `-- check`/`check-xsd` (validate inputs). The `/regen-codegen` skill runs all three + the drift check; a `codegen-drift` CI job guards it.
- **Generated crates** (`openehr-base`, `openehr-rm`, `openehr-am` with both `am14`/`am24`): every type file starts with `// @generated … DO NOT EDIT`. **Never hand-edit a generated file.** To change output, edit the emitter (`crates/openehr-codegen/src/emit.rs`) or its override map, then regenerate. `openehr-foundation` no longer exists (folded into `openehr-base`).
- **Hand-written spec behaviour** (invariants, spec functions per ADR-003) lives in sibling `*_impl.rs` files the generator never rewrites.
- **Hand-written tooling** (edit freely, normal Rust): `openehr-lang` (ODIN + BMM reader), `openehr-codegen` (emitter), `openehr-derive` (proc-macro).
- **Partly generated:** `openehr-its` — the XML `ToXml`/`FromXml` impls (`emit-xml`) and the ITS-REST contract (`emit-rest`) are generated into `src/xml/generated/` + `src/rest/generated/`; the hand-written parts are the runtimes (`xml/runtime.rs`, `rest/runtime.rs`), the canonical-JSON entry points + validation, and the fidelity gates.
- **NOT generated** (hand-written): `openehr-term` (terminology bundle + XML assets + access logic — BMM only has ~6 interface classes), `openehr-query` (AQL lexer/parser/AST), `openehr-flat` (SDT), and the `ehrbase-*` application crates (idiomatic Rust of our own design on the generated crates — ADR-006/008).
- **Pinned spec versions:** RM 1.2.0, BASE 1.3.0, TERM 3.1.0, AM 1.4.0 + 2.4.0 (see `docs/VERSIONS.md`). These are the latest published spec versions — the conformance target (ADR-008).

## Phase workflow (the loop)

1. Read `docs/plans/current-phase.md`.
2. Pick the next unchecked task in the referenced phase file.
3. Do the work. **First read the governing spec sections** under `docs/specs/openehr/` for anything spec-facing (`/spec-lookup`; hard rule below). The orchestrator keeps the critical path in-session (build in the open so it can be watched and corrected) and may fan bounded implementation/research out per the Model-orchestration section, handing each agent the relevant spec paths. For the openEHR **spec** layer, change the generator and regenerate (never hand-write spec classes — see Code generation above). For the **application** (`ehrbase-*`), build **idiomatic modern Rust of our own design on the generated `openehr-*` crates** (ADR-006/008), with the openEHR specs as the authority (EHRbase/other CDRs as prior art via their upstream repos when useful). Build compiling, tested increments.
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

- **Orchestrator (Fable 5, high):** owns the phase loop, architecture, ADR decisions, spec-conformance judgement, and the hard bespoke logic (AQL IR→SQL, versioning, validation, the node codec). Keeps these in-context rather than delegating — they need top intelligence + taste and are the project's critical path.
- **Delegate to Opus-4.8 subagents:** bulk/parallelizable implementation on a clear spec (wiring handlers, DTO/trait impls against the generated ITS-REST contract, migrations, sqlx/sea-query query building, test scaffolding), file-heavy investigation, and codebase analysis — anything that would otherwise burn the orchestrator's context. Fan out several concurrently in one message.
- **Fable-5 subagents:** use when a delegated task still needs top intelligence/taste (a tricky algorithm port, an API-shape decision) but you want it off the main context.
- **sonnet-5:** cheap mechanical passes where correctness is easy to verify (mechanical refactors, boilerplate). **Never use Haiku for substantive work.**
- **Reviews:** the `spec-conformance-reviewer` agent (read-only, Opus), as an independent perspective before committing a subsystem — especially spec/wire conformance and the AQL engine. Spec questions / requirements extraction go to `spec-researcher`; bounded implementation to `implementer` — all defined in `.claude/agents/`, all handed the governing `docs/specs/openehr/...` paths in the prompt.
- Effort: keep Fable on `high` (xhigh is token-hungry; max/extra is a furnace for worse output). Use `effort: 'low'` for cheap mechanical worker stages, higher tiers only for the hardest verify/judge stages.

Discipline unchanged: subagents still obey the hard rules below (never hand-edit `// @generated`, no test-weakening, `claude/*` branches, no AI attribution) — deviations surface at the conformance/corpus suites, so delegate with a tight spec and verify the result.

## Tech stack (pinned)

Toolchain: Rust stable **1.96** (1.96.1), MSRV 1.96, **edition 2024**, resolver v3. Pin via `rust-toolchain.toml`.
Database: **PostgreSQL 18** (target 18.4+): AIO, `uuidv7()`, skip scan, temporal constraints, RETURNING OLD/NEW, plus `JSON_TABLE` from PG 17. Extensions: `uuid-ossp`, `pgcrypto`, `pg_trgm`.

**The authoritative, fully-pinned dependency set lives in the root `Cargo.toml` `[workspace.dependencies]`.** Versions below are current as of 2026-07; do not hand-roll anything a crate here already provides (auth, HTTP status codes, OpenAPI/Swagger, etc.). Add a crate to a member with `dep.workspace = true`. Items marked *(verify)* had their major/minor unconfirmed at authoring; check crates.io before first use.

Web & HTTP core: `axum` 0.8, `axum-extra` 0.12, `axum-server` 0.8 (graceful shutdown + TLS), `tower` 0.5, `tower-http` 0.7 (trace, cors, compression, timeout, limit, request-id, sensitive-headers, catch-panic, normalize-path), `hyper` 1, `hyper-util` 0.1, `http` 1 (status codes/headers), `http-body` 1, `http-body-util` 0.1, `mime` 0.3, `mime_guess` 2, `headers` 0.4, `bytes` 1.

Async runtime: `tokio` 1, `tokio-util` 0.7, `tokio-stream` 0.1, `futures` 0.3, `async-trait` 0.1, `pin-project-lite` 0.2.

Auth & authz (never hand-roll): `jsonwebtoken` 10 (JWT), `oauth2` 5 *(verify)*, `openidconnect` 4 (OIDC/Keycloak) *(verify)*, `argon2` 0.5 + `password-hash` 0.5 (password hashing), `tower-sessions` 0.15 *(verify)*, `axum-login` 0.18 *(verify)*, `secrecy` 0.10, `zeroize` 1. RBAC/ABAC for the Stage 2 restoration: `casbin` 2 *(verify)* or `cedar-policy` 4 *(verify)* (decide at S2).

TLS/crypto: `rustls` 0.23, `tokio-rustls` 0.26, `rustls-pemfile` 2, `webpki-roots` 0.26, `rand` 0.9, `getrandom` 0.3, `sha2` 0.10, `hmac` 0.12, `blake3` 1.

OpenAPI/Swagger: `utoipa` 5, `utoipa-axum` 5, `utoipa-swagger-ui` 9 (serves Swagger UI), `utoipa-redoc`/`utoipa-scalar`/`utoipa-rapidoc` 5. Note (ADR-005): the ITS-REST contract is **spec-first** — the vendored OpenAPI is authoritative and is generated into `openehr-its` (`emit-rest`); `utoipa` serves Swagger UI and an optional code→OAS drift-check, not the source of truth.

Database & persistence: `sqlx` 0.9 (postgres, macros, migrate, uuid, json, rust_decimal, chrono; TLS via `tls-rustls-aws-lc-rs`); `sea-query` 1.0 + `sea-query-sqlx` (the sea-query 1.0↔sqlx 0.9 binder — `sea-query-binder` is stuck on sea-query 0.32; dynamic SQL for the AQL→SQL engine — **not** sea-orm, ADR-006); `jiff-sqlx` for jiff↔Postgres on plain sqlx queries (sqlx has no `jiff` feature; the binder's with-jiff is unimplemented upstream); `deadpool-postgres` + `tokio-postgres` for an optional pipelined hot-read path. Migrations: squashed `0001_baseline.sql` per schema, equality-gated against the EHRbase Flyway chain (ADR-007); new ones via `sqlx migrate add --sequential`.

Serialization & formats: `serde` 1, `serde_json` 1 (`preserve_order`), `serde_with` 3, `serde_path_to_error` 0.1, `quick-xml` 0.41 (with `serialize`), `base64` 0.22, `rust_decimal` 1 (+`rust_decimal_macros`) as the BigDecimal replacement for DV_QUANTITY, `ordered-float` 4, canonical JSON via `serde_jcs` 0.2 or a ~150-LoC hand-roll. C14N (canonical XML): `xmllint --c14n` fallback for now.

Parsers (native ADL/cADL/ODIN/AQL): `logos` 0.16 (lexer), `chumsky` 0.13 (stable; 1.0 still alpha, repo now on Codeberg) or `winnow` 0.7, `regex` 1, `fancy-regex` 0.18 *(verify)* for cADL backreferences; diagnostics via `miette` 7 and/or `ariadne` 0.6 *(verify)*.

IDs / time / validation: `uuid` 1 (v4+v7, serde, fast-rng), `jiff` 0.2 (1.0 not yet released as of 2026-07), `garde` 0.23 *(verify)* + a custom RM-invariant framework, `url` 2, `urlencoding` 2.1.3 (ALL URL/percent encoding+decoding — never hand-roll a percent codec; owner rule 2026-07-11).

Observability (opentelemetry set is lockstep — keep equal): `tracing` 0.1, `tracing-subscriber` 0.3, `tracing-opentelemetry` 0.33, `opentelemetry` 0.31, `opentelemetry_sdk` 0.31, `opentelemetry-otlp` 0.31, `opentelemetry-semantic-conventions` 0.31, `metrics` 0.24, `metrics-exporter-prometheus` 0.18 *(verify)*, `axum-prometheus` 0.10 *(verify)*.

Caching / rate limiting / resilience: `moka` 0.12 (Caffeine equivalent for the template/WebTemplate cache), `quick_cache` 0.6, `tower_governor` 0.8 *(verify)* + `governor` 0.10 *(verify)*, `backon` 1 (retry; the `backoff` crate is deprecated).

Errors & utilities: `thiserror` 2 (libs), `anyhow` 1 (bins only), `config` 0.14 or `figment` 0.10, `dotenvy` 0.15, `clap` 4, `parking_lot` 0.12, `dashmap` 6, `arc-swap` 1, `indexmap` 2, `smallvec` 1, `itertools` 0.14, `bitflags` 2. Use `std::sync::LazyLock` (edition 2024) instead of `once_cell` for statics.

HTTP client & external integration: `reqwest` 0.13 (rustls, json) for the terminology/FHIR client and conformance runner; `jsonschema` 0.46 *(verify)* to validate against the openEHR ITS-JSON schemas.

Testing & benches (dev-deps): `cargo-nextest`, `insta` 1 (snapshots — the key tool for canonical JSON/XML parity), `proptest` 1, `rstest` 0.26 *(verify)*, `wiremock` 0.6, `mockall` 0.15 *(verify)*, `fake` 5 *(verify)*, `assert_cmd` 2, `assert_fs` 1, `testcontainers` 0.27 *(verify)* + `testcontainers-modules` 0.12 *(verify)* (real PG 18), `criterion` 0.5 + `divan` 0.1.

Dev tooling (CI, not deps): `cargo-nextest`, `cargo-audit`, `cargo-deny`, `cargo-machete`, `cargo-hakari`, `cargo-llvm-cov`, `sccache`; `mold` linker on Linux.

openEHR spec versions are pinned in the "Code generation" section above (RM 1.2.0, BASE 1.3.0, TERM 3.1.0, AM 1.4.0 + 2.4.0) and in `docs/VERSIONS.md` (the single source of truth) — including the ITS-XML/REST/JSON pins.

## Build and test

```bash
cargo build --workspace
cargo nextest run --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
cargo audit && cargo deny check
# conformance runner (the ADR-008 acceptance instrument) — present and green:
bash scripts/conformance.sh   # compose up --build → full ECC → docs/conformance/ (341 executed · 315 passed · 0 failed at B6 close)
```

### Target-dir & warm-build discipline (owner rules 2026-07-12)

The workspace is huge; a cold build is expensive and `target/` bloat once hit
~90 GB. These rules keep builds warm and disk bounded:

- **One shared `./target` for all CLI cargo** in a session (build, clippy,
  nextest). Concurrent subagents that must build in parallel use **fixed lanes**
  `CARGO_TARGET_DIR=$PWD/target/agent-t1` … `agent-t4` (stable names, reused
  across sessions = warm) — never a fresh per-task name, never a `/tmp` or
  scratchpad target dir, and never more than four lanes.
- **`target-cli` is retired (deleted 2026-07-12) — do not recreate it.** The
  IDE-vs-CLI contention fix is inverted: RustRover gets its own small dir
  (Cargo settings → env `CARGO_TARGET_DIR=/Users/rubentalstra/RustroverProjects/ehrbase-rs/target/ide`
  — MUST be absolute: a relative value resolves against cargo's cwd and
  sprouts a nested `target/` inside every crate the IDE checks), the CLI
  keeps `./target`. Never `pkill -9 rustc` to "fix" slowness — it corrupts
  incremental caches.
- **Iterate scoped, gate wide.** While working: `cargo clippy -p <crate>
  --all-targets` and `cargo nextest run -p <crate>`. The full `--workspace`
  gates run once, before commit. `clippy` shares the check cache — running it
  repeatedly is cheap *if the flag set stays identical*.
- **Never vary `RUSTFLAGS`, features, or profile between runs** — any change
  rebuilds the world. Use the exact commands above, no ad-hoc flags.
- **Disk hygiene:** cargo never garbage-collects `target/debug/deps` (stale
  `.rlib`s accumulate on every dep bump). When `du -sh target` exceeds ~30 GB,
  run `cargo clean` once (one cold rebuild, then warm again) and delete stale
  agent lanes. Check no other session/IDE is mid-build first
  (`pgrep -fl 'cargo|rustc'`) — parallel sessions share this tree.

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
clippy-clean. The `ehrbase-*` application crates are built compiling per phase:
the Stage-1 app build P09–P15 is **done** (persistence, greenfield storage,
REST+auth, service layer, templates, WebTemplate/FLAT/STRUCTURED, validation);
the remaining work is P16–P20 (+P99) — AQL engine, FLAT/EhrScape wiring,
integration, conformance, optimization, cutover.

## Conventions

- Crate boundaries mirror openEHR components. Keep dependencies pointing downward: app (`ehrbase-*`) → spec (`openehr-*`), never the reverse. The `ehrbase-*` crates consume the generated `openehr-*` types directly as their domain model — never re-model the RM or re-serialize.
- **Two disciplines by layer.** Spec/ITS crates (`openehr-*`) = *generated* from the vendored specs (ADR-004/005): change the emitter and regenerate, never hand-edit `// @generated`; the bar is wire + semantic + invariant parity. Application crates (`ehrbase-*`) = *modern idiomatic Rust of our own design* (ADR-006/008): use proper crates (axum, sqlx+sea-query-sqlx, oauth2, utoipa); the openEHR specs are the authority; verify at the REST/AQL surface with the CNF conformance suite + corpus tests. Build compiling, tested increments.
- Emission choices the generator already makes (do not re-litigate per class): closed openEHR subtype sets → untagged Rust `enum`s; recursion (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`, `DV_MULTIMEDIA.thumbnail`, F-bounded ranges) → `Box`; `_type` via `#[derive(OpenEhrType)]`; strong types where unambiguous (`uuid::Uuid`, etc.). Behavioural back-references (`PATHABLE.parent()`) in hand-written `*_impl.rs` use `Weak` or an index, never an owning reference.
- `thiserror` in hand-written library crates; `anyhow` only in the binary. No `unwrap`/`expect` outside tests.
- **No `use X as Y` import renaming.** Direct names only; a bad name gets renamed at its definition, a genuine collision gets a qualified path at the use site. Alias only in highly exceptional cases with no other solution (a `use Trait as _;` trait import is not a rename and is fine).
- Async-first: the server is I/O-bound on Postgres; use idiomatic tokio/axum (this is not the Bun "no-Tokio" case).

## IMPORTANT hard rules

- **The vendored spec text is the oracle.** Before implementing or reviewing any spec-facing behaviour (RM semantics, invariants, REST wire, AQL, canonical JSON/XML, templates, terminology), read the relevant section under `docs/specs/openehr/` (use `/spec-lookup`) and cross-check the CNF platform test schedule (`docs/specs/openehr/CNF/`). Never resolve a spec question from memory or from EHRbase behaviour alone; cite the spec file + section for conformance-relevant decisions.
- **Cite ONLY the openEHR specs in code — NEVER an ADR (owner hard rule, 2026-07-11).** Code comments, SQL schema comments, doc comments, and PORT NOTEs justify behaviour with the openEHR spec file + section (e.g. `RM common master06 §Version tree`), never `ADR-NNN` — ADRs can be superseded and would leave stale/untrue statements in the code, while spec citations stay findable and updatable on a spec bump. Where the openEHR specs are SILENT (storage mechanics, indexes, infra, extensions like eventing/multi-tenancy/FHIR), FLAG it explicitly — "no openEHR spec governs this — our own design/extension" — instead of citing an ADR. ADRs stay in `docs/ADRs/` as decision history only, and any spec-facing claim in an ADR must be re-verified against the spec text before being relied on (the specs are leading; ADRs may be wrong).
- **Never hand-edit a `// @generated` file.** The generated spec crates (`openehr-base`, `openehr-rm`, `openehr-am`) are produced by `openehr-codegen`; edit the emitter or the `*_impl.rs` sibling and regenerate, never the generated file itself.
- **No PORT STATUS trailer** (that was the retired 1:1-port convention). Generated files carry a `// @generated` header; application code is plain idiomatic Rust.
- **Use the annotation vocabulary** where relevant: `// TODO(port):` (unfinished), `// PERF(port):` (optimize after conformance), `// PORT NOTE:` (a deliberate spec-gap or design decision, with the reason), `// SAFETY:` (any `unsafe`).
- **Branches are `claude/*`.** Never force-push `main`. Never delete `docs/plans/current-phase.md` or `docs/plans/README.md`; completed phase files may be pruned once their close is recorded in `docs/PROGRESS.md`.
- **NEVER add AI/Claude attribution to commits or PRs. This is an absolute rule with no exceptions.** Do not add a `Co-Authored-By: Claude` trailer (or any co-author trailer). Do not write "Generated with Claude Code", "🤖 Generated with...", "Co-authored-by: Claude", or any similar line, emoji, or footer in a commit message, commit body, commit trailer, PR title, PR description, PR comment, issue, or code comment. Commit messages and PR text describe only the change itself. If you ever find yourself about to add such a line, stop and remove it. When configuring git or opening PRs, do not pass any flag or template that injects attribution.
- **Keep the changelog** (`CHANGELOG.md`, Keep a Changelog 1.1.0): every PR with user-visible changes adds an `[Unreleased]` entry in the same PR — CI (`changelog-guard`) enforces it; releases are cut from the changelog (see `.claude/rules/changelog.md`). The `openehr-*` spec crates are versioned by the spec they implement; the product/workspace follows its own SemVer (3.x).
- **User docs track the product.** Any PR that changes the REST surface, configuration (`EHRBASE_*`), the CLI, deployment artifacts (compose/Helm/containers), or other user-visible behaviour must update the matching `website/book/src` page in the same PR (see `.claude/rules/docs-website.md`). Never hand-edit `website/api/spec/**` — run `scripts/assemble-oas.sh` (CI drift gate).
- **Never weaken, skip, or delete a test** to make a build pass, and never edit a test to route around a bug it exposes.
- **Tick the phase checkbox and commit before ending a session.** A `Stop` hook enforces this.
- **Application phases build as compiling, tested increments** (ADR-006) on top of the generated `openehr-*` crates — do not defer compilation. (The old "phases need not compile" rule applied only to the retired hand-transcription era.)
- The v1 enterprise code (RBAC and others) is a Stage 2 concern. Do not build it during Stage 1; it lives only as the read-only `reference/v1` git ref until then.

## References

- @docs/plans/current-phase.md (what's next + the goal)
- @docs/ADRs/ADR-008-greenfield-pg18-storage.md (the pivot: own storage/engine, spec conformance — read first)
- @docs/ADRs/ADR-006-application-port-philosophy.md (the app-build philosophy; partially superseded by ADR-008)
- @docs/ADRs/ADR-004-spec-driven-codegen.md + @docs/ADRs/ADR-005-its-codegen.md (the codegen — read before touching any `openehr-*` crate)
- @docs/architecture.md (the current design)
- @docs/VERSIONS.md + @docs/postgres-features.md (pins + the PG 17/18 features we use)
- @ROADMAP.md (the product roadmap) + @docs/blueprint/00-THE-BLUEPRINT.md (the spec-compliance ledger + spec-gap surface)
