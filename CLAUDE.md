# CLAUDE.md

Pure-Rust 1:1 port of EHRbase (Java/Spring Boot to Rust), in a single root Cargo workspace, executed Bun-style. The authoritative plan is `PORT_MASTER_PLAN.md`; read it before non-trivial work. Running plans and task lists live in `docs/plans/`, never here.

## Repo map

Single workspace. EHRbase's Java has been moved into `crates/openehr-*` (Phase 0 `git mv`). During the port, Java and Rust coexist in the same directory: port each `.java` to a `.rs` beside it, then delete the Java only in the phase that reaches parity.

**Crate naming (ADR-004):** `openehr-*` = the openEHR **specification** (increasingly *generated* from the vendored BMM meta-model, not hand-transcribed); `ehrbase-*` = the ported **EHRbase application**.

- `crates/openehr-foundation`, `openehr-base`, `openehr-term`, `openehr-rm`, `openehr-serde`, `openehr-lang`, `openehr-am`, `openehr-flat`, `openehr-query` — openEHR spec crates, one per spec component (`openehr-lang` = LANG: ODIN + BMM; `openehr-am` = AM; `openehr-query` = AQL/QUERY). `openehr-codegen` (BMM→Rust generator) + `openehr-derive` (proc-macro) join this set. `openehr-foundation` folds into `openehr-base` at the codegen pass.
- `crates/ehrbase-rest`, `ehrbase-compat`, `ehrbase` — the ported EHRbase application; receive EHRbase's server Java, ported in place. `ehrbase` is the binary.
- `docs/` — plans, research, ADRs, ROSETTA, PORTING, VERSIONS, LIFETIMES.
- `.claude/` — rules, skills, agents, hooks.

## Phase workflow (the loop)

1. Read `docs/plans/current-phase.md`.
2. Pick the next unchecked task in the referenced phase file.
3. Do the work. Delegate per-file ports to the `porter` subagent; delegate spec transcription to `rm-transcriber`. Prefer subagents over inline work when it would burn context.
4. Tick the task `- [ ]` to `- [x]` and add a one-line note.
5. Commit as `phase-NN: <task>` on a `claude/phase-NN-*` branch.
6. When the phase's exit criteria are all met, run `/phase-done`, update `docs/PROGRESS.md`, and advance `current-phase.md`.

Checkboxes on disk survive `/clear` and `/compact`; the built-in todo tool is session-scoped, so the phase files are the durable layer.

## Tech stack (pinned)

Toolchain: Rust stable **1.96** (1.96.1), MSRV 1.96, **edition 2024**, resolver v3. Pin via `rust-toolchain.toml`.
Database: **PostgreSQL 18** (target 18.4+): AIO, `uuidv7()`, skip scan, temporal constraints, RETURNING OLD/NEW, plus `JSON_TABLE` from PG 17. Extensions: `uuid-ossp`, `pgcrypto`, `pg_trgm`.

**The authoritative, fully-pinned dependency set lives in the root `Cargo.toml` `[workspace.dependencies]`.** Versions below are current as of 2026-07; do not hand-roll anything a crate here already provides (auth, HTTP status codes, OpenAPI/Swagger, etc.). Add a crate to a member with `dep.workspace = true`. Items marked *(verify)* had their major/minor unconfirmed at authoring; check crates.io before first use.

Web & HTTP core: `axum` 0.8, `axum-extra` 0.10, `axum-server` 0.7 (graceful shutdown + TLS), `tower` 0.5, `tower-http` 0.6 (trace, cors, compression, timeout, limit, request-id, sensitive-headers, catch-panic, normalize-path), `hyper` 1, `hyper-util` 0.1, `http` 1 (status codes/headers), `http-body` 1, `http-body-util` 0.1, `mime` 0.3, `mime_guess` 2, `headers` 0.4, `bytes` 1.

Async runtime: `tokio` 1, `tokio-util` 0.7, `tokio-stream` 0.1, `futures` 0.3, `async-trait` 0.1, `pin-project-lite` 0.2.

Auth & authz (never hand-roll): `jsonwebtoken` 9 (JWT), `oauth2` 5 *(verify)*, `openidconnect` 4 (OIDC/Keycloak) *(verify)*, `argon2` 0.5 + `password-hash` 0.5 (password hashing), `tower-sessions` 0.14 *(verify)*, `axum-login` 0.17 *(verify)*, `secrecy` 0.10, `zeroize` 1. RBAC/ABAC for the Stage 2 restoration: `casbin` 2 *(verify)* or `cedar-policy` 4 *(verify)* (decide at S2).

TLS/crypto: `rustls` 0.23, `tokio-rustls` 0.26, `rustls-pemfile` 2, `webpki-roots` 0.26, `rand` 0.9, `getrandom` 0.3, `sha2` 0.10, `hmac` 0.12, `blake3` 1.

OpenAPI/Swagger (code-first, do not write specs by hand): `utoipa` 5, `utoipa-axum` 5, `utoipa-swagger-ui` 9 (serves Swagger UI), `utoipa-redoc`/`utoipa-scalar`/`utoipa-rapidoc` 5.

Database & persistence: `sqlx` 0.9 (postgres, macros, migrate, uuid, json, rust_decimal, chrono; TLS via `tls-rustls-aws-lc-rs`) — note sqlx has no `jiff` feature, bridge manually; `sea-query` 0.32 + `sea-query-binder` 0.7 (dynamic SQL for the AQL→SQL engine); `deadpool-postgres` 0.14 + `tokio-postgres` 0.7 for an optional pipelined hot-read path.

Serialization & formats: `serde` 1, `serde_json` 1 (`preserve_order`), `serde_with` 3, `serde_path_to_error` 0.1, `quick-xml` 0.37 *(verify)* (with `serialize`), `base64` 0.22, `rust_decimal` 1 (+`rust_decimal_macros`) as the BigDecimal replacement for DV_QUANTITY, `ordered-float` 4, canonical JSON via `serde_jcs` 0.1 *(verify)* or a ~150-LoC hand-roll. C14N (canonical XML): `xmllint --c14n` fallback for now.

Parsers (native ADL/cADL/ODIN/AQL): `logos` 0.15 (lexer), `chumsky` 0.10 (usable stable; 1.0 still alpha, repo now on Codeberg) or `winnow` 0.7, `regex` 1, `fancy-regex` 0.14 *(verify)* for cADL backreferences; diagnostics via `miette` 7 and/or `ariadne` 0.5 *(verify)*.

IDs / time / validation: `uuid` 1 (v4+v7, serde, fast-rng), `jiff` 0.2 (1.0 not yet released as of 2026-07), `garde` 0.22 *(verify)* + a custom RM-invariant framework, `url` 2.

Observability (opentelemetry set is lockstep — keep equal): `tracing` 0.1, `tracing-subscriber` 0.3, `tracing-opentelemetry` 0.33, `opentelemetry` 0.31, `opentelemetry_sdk` 0.31, `opentelemetry-otlp` 0.31, `opentelemetry-semantic-conventions` 0.31, `metrics` 0.24, `metrics-exporter-prometheus` 0.16 *(verify)*, `axum-prometheus` 0.8 *(verify)*.

Caching / rate limiting / resilience: `moka` 0.12 (Caffeine equivalent for the template/WebTemplate cache), `quick_cache` 0.6, `tower_governor` 0.4 *(verify)* + `governor` 0.6 *(verify)*, `backon` 1 (retry; the `backoff` crate is deprecated).

Errors & utilities: `thiserror` 2 (libs), `anyhow` 1 (bins only), `config` 0.14 or `figment` 0.10, `dotenvy` 0.15, `clap` 4, `parking_lot` 0.12, `dashmap` 6, `arc-swap` 1, `indexmap` 2, `smallvec` 1, `itertools` 0.14, `bitflags` 2. Use `std::sync::LazyLock` (edition 2024) instead of `once_cell` for statics.

HTTP client & external integration: `reqwest` 0.12 (rustls, json) for the terminology/FHIR client and parity harness; `jsonschema` 0.26 *(verify)* to validate against the openEHR ITS-JSON schemas.

Testing & benches (dev-deps): `cargo-nextest`, `insta` 1 (snapshots — the key tool for canonical JSON/XML parity), `proptest` 1, `rstest` 0.23 *(verify)*, `wiremock` 0.6, `mockall` 0.13 *(verify)*, `fake` 3 *(verify)*, `assert_cmd` 2, `assert_fs` 1, `testcontainers` 0.24 *(verify)* + `testcontainers-modules` 0.12 *(verify)* (real PG 18), `criterion` 0.5 + `divan` 0.1.

Dev tooling (CI, not deps): `cargo-nextest`, `cargo-audit`, `cargo-deny`, `cargo-machete`, `cargo-hakari`, `cargo-llvm-cov`, `sccache`; `mold` linker on Linux.

openEHR spec versions to target: BASE 1.2.0, RM 1.1.0, AM 2.3.0, QUERY 1.1.0, LANG 1.0.0 (BMM v2.3), TERM 3.0.0, ITS-XML 2.0.0 (plus 1.0.2 for round-trip), ITS-REST 1.0.3, ITS-JSON (pin a commit).

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

Note: Phases P1 through P16 are not required to compile. `cargo build` is expected to fail until P17 (make-it-compile). This is by design; do not chase build errors during translation.

Exception (since P4): `openehr-foundation`, `openehr-base`, `openehr-term`, `openehr-rm`, and `openehr-serde` DO compile and have green tests (`cargo test -p <crate>`). Changes to these five crates must keep them compiling, tested, and clippy-clean; ADR-003 fixes the policies for spec-underdetermined behaviour in them. The Phase-A no-compile allowance now applies only to the crates that have not yet reached this bar.

## Conventions

- Crate boundaries mirror openEHR components (see `PORT_MASTER_PLAN.md` Section 9). Keep dependencies pointing downward: server → spec crates, never the reverse.
- Faithful port first. Mirror the source file's names, method names, field order, and control flow. Note idiomatic alternatives in comments rather than applying them during Stage 1.
- Closed openEHR subtype sets become Rust `enum`s (`DataValue`, `Item`, `ContentItem`, `PartyProxy`, `Version<T>`). Trait objects only for genuinely archetype-driven runtime polymorphism.
- `PATHABLE.parent()` and other back-references use `Weak` or an index, never an owning reference.
- Recursive containment (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`, `DV_MULTIMEDIA.thumbnail`) is boxed.
- `thiserror` in library crates; `anyhow` only in the binary. No `unwrap`/`expect` outside tests; use `todo!()` or `// TODO(port):` where the real value is not yet available.
- Async-first: the server is I/O-bound on Postgres; use idiomatic tokio/axum (this is not the Bun "no-Tokio" case).
- Keep the openEHR class name visible on each transcribed type (doc comment + serde rename to the canonical `_type` string).

## IMPORTANT hard rules

- **Never edit a `.java` file that has no completed Rust counterpart, and never edit Maven build files** (`pom.xml`, `mvnw`, `mvnw.cmd`, `.mvn/`). A `PreToolUse` hook enforces this. Delete a Java file only in the same phase its Rust replacement reaches parity.
- **Every ported or transcribed file ends with a `// PORT STATUS` trailer** (source, source_loc, confidence, todos, note).
- **Use the annotation vocabulary**: `// TODO(port):`, `// PERF(port):`, `// PORT NOTE:`, `// SAFETY:`.
- **Branches are `claude/*`.** Never force-push `main`. Never delete files under `docs/plans/`.
- **NEVER add AI/Claude attribution to commits or PRs. This is an absolute rule with no exceptions.** Do not add a `Co-Authored-By: Claude` trailer (or any co-author trailer). Do not write "Generated with Claude Code", "🤖 Generated with...", "Co-authored-by: Claude", or any similar line, emoji, or footer in a commit message, commit body, commit trailer, PR title, PR description, PR comment, issue, or code comment. Commit messages and PR text describe only the change itself. If you ever find yourself about to add such a line, stop and remove it. When configuring git or opening PRs, do not pass any flag or template that injects attribution.
- **Never weaken, skip, or delete a test** to make the port pass, and never edit a test to route around a bug it exposes. A parity test is valid only if it still fails against stock EHRbase without our fix (`USE_REFERENCE_EHRBASE=1`).
- **Tick the phase checkbox and commit before ending a session.** A `Stop` hook enforces this.
- **Phases P1-P16 do not need to compile.** Capture intent; do not get stuck on the compiler.
- The v1 enterprise code (RBAC and others) is a Stage 2 concern. Do not build it during Stage 1; it lives only as the read-only `reference/v1` git ref until then.

## References

- @AGENTS.md
- @PORT_MASTER_PLAN.md
- @docs/PORTING.md
- @docs/ROSETTA.md
- @docs/VERSIONS.md
- @docs/plans/current-phase.md
