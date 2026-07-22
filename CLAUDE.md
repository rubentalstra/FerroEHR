# CLAUDE.md

A pure-Rust, **openEHR-spec-conformant** CDR (ITS-REST 1.1.0 + AQL 1.1) in a single root Cargo workspace, with greenfield PG18-native internals. The openEHR spec + serialization + REST-contract layer is **generated** from the official machine-readable specs; the application is **modern idiomatic Rust of our own design on top of the generated crates**: our own storage, versioning, and AQL engine, validated by the openEHR CNF conformance suite — EHRbase is prior art, not an oracle. The forward product roadmap is the root `ROADMAP.md`; **the open-items tracker is GitHub Issues** (see the Issue workflow below); deep working plans live in `docs/plans/`. ROADMAP.md + the tracker (issues, PRs, `CHANGELOG.md`, git history) are the authoritative record — there is no PROGRESS.md (retired 2026-07-20 with the worklist; closing PR descriptions + issue handoff comments carry the build narrative). **There is no ADR/design-doc layer (owner rulings 2026-07-16 + 2026-07-17): decisions live in THIS file, `docs/architecture.md`, and the code; a plan/design markdown is DELETED in the PR that implements it; the vendored specs are the only doc oracle.**

## Repo map

Single workspace. **Crate naming:** `openehr-*` = the openEHR **specification** (generated from the vendored BMM/XSD/OAS — treat as `// @generated`); `ehrbase-*` = the **application** (idiomatic Rust of our own design consuming the `openehr-*` crates). The formerly in-tree EHRbase Java reference was removed with the greenfield pivot (git history / upstream repos are the prior-art record).

- `crates/openehr-base`, `openehr-rm`, `openehr-am`, `openehr-term`, `openehr-lang` — **generated** spec crates (`openehr-codegen -- emit`). `openehr-its` — canonical JSON + **generated** XML (`emit-xml`) + **generated** ITS-REST contract (`emit-rest`) + hand-written runtimes. `openehr-query` — hand-written AQL lexer/parser/AST. `openehr-adl` — hand-written ADL 2.4 engine (ADL2/cADL/ODIN parser, AOM2 validation, flattener, OPT2, ADL 1.4→2 conversion) over `openehr-am::am24`. `openehr-flat` — FLAT/STRUCTURED (hand-written). `openehr-codegen` (BMM/XSD/OAS→Rust generator) is the hand-written tooling.
- `app/*` — the application, **four crates**: `ehrbase` (the platform **library**: storage, service layer [one module per SM chapter, concrete `EhrbaseService` methods — no traits], AQL engine, versioning, config tree, telemetry, `signing` + `system_log`), `ehrbase-rest` (ITS-REST protocol adapter + auth + the `access` authz module, calling the concrete service directly — depends on `ehrbase`), `ehrbase-server` (the wiring-only binary; bin name stays `ehrbase`), and `ehrbase-admin-ui` (the Leptos SSR admin console — a standalone binary/OCI image that consumes the CDR STRICTLY over ITS-REST; may depend on `crates/openehr-*`, NEVER on `app/ehrbase`/`app/ehrbase-rest`; gates via `/ui-gates`, rules in `.claude/rules/leptos-ui.md`). **Zero re-exports (owner hard rule 2026-07-16): import every name from its defining module.** `tools/*` — dev/verification tooling, **not part of the app**: `cnf-runner` (the CNF 2.0 conformance runner — the acceptance instrument), `benchmark`, and `testkit` (the shared test-database harness — one PG18 server + template-database cloning; every DB-backed test uses `testkit::db()`, never a per-test container). Workspace `members = ["crates/*", "app/*", "tools/*"]`. The service layer follows the openEHR **SM Platform Service Model** (SM component map in `docs/architecture.md`, vendored SM spec at `docs/specs/openehr/SM/`).
- `docs/` — plans (`docs/plans/`: deep working plans, linked from their tracker issues), VERSIONS, architecture, postgres-features, `endpoint-map.md` (every endpoint traced to its SQL), conformance (generated reports), benchmarks; the forward product roadmap is the root `ROADMAP.md`. **`docs/specs/openehr/` — the vendored openEHR spec text + CNF test schedule (the conformance oracle; see its README + `/spec-lookup`).**
- `.claude/` — rules, skills, hooks, agents, **`memory/`** (the persistent agent memory, moved in-repo 2026-07-12 so it is visible and versioned; the harness memory dir under `~/.claude/projects/` is a symlink to it — never break that link). Agent defs (`spec-researcher`, `spec-conformance-reviewer`, `cnf-triage`, `implementer`, `ui-implementer`, `leptos-reviewer`) are the delegation targets for the Model-orchestration section below; the orchestrator keeps the critical path in-session.

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

## Code generation — READ THIS FIRST

**The openEHR spec crates are GENERATED from the vendored BMM meta-model, not hand-written.** Do **not** hand-transcribe or hand-edit RM/BASE/AM classes — that era is over. openEHR publishes a machine-readable meta-model (BMM); we generate idiomatic Rust from it deterministically.

- **Pipeline:** vendored `*.bmm.json` (in `tools/openehr-codegen/vendor/bmm/`) → `openehr-lang::bmm` (loader) → `openehr-codegen` (emitter) → the spec crates. Canonical-JSON `_type` (de)serialization is the native `ToJson`/`FromJson` codec emitted into `openehr-its` (`emit-json`); the spec types carry no serde derive.
- **Regenerate:** `cargo run -p openehr-codegen -- emit` (spec crates from BMM) · `-- emit-xml` (canonical-XML `ToXml`/`FromXml` into `openehr-its`) · `-- emit-rest` (the ITS-REST contract into `openehr-its`) · `-- check`/`check-xsd` (validate inputs). The `/regen-codegen` skill runs all three + the drift check; a `codegen-drift` CI job guards it.
- **Generated crates** (`openehr-base`, `openehr-rm`, `openehr-am` with both `am14`/`am24`): every type file starts with `// @generated … DO NOT EDIT`. **Never hand-edit a generated file.** To change output, edit the emitter (`tools/openehr-codegen/src/render/emit.rs`) or its override map, then regenerate. `openehr-foundation` no longer exists (folded into `openehr-base`).
- **Hand-written spec behaviour** (invariants, spec functions) lives in sibling `*_impl.rs` files the generator never rewrites.
- **Hand-written tooling** (edit freely, normal Rust): `openehr-lang` (ODIN + BMM reader), `openehr-codegen` (emitter).
- **Partly generated:** `openehr-its` — the XML `ToXml`/`FromXml` impls (`emit-xml`) and the ITS-REST contract (`emit-rest`) are generated into `src/xml/generated/` + `src/rest/generated/`; the hand-written parts are the runtimes (`xml/runtime.rs`, `rest/runtime.rs`), the canonical-JSON entry points + validation, and the fidelity gates.
- **NOT generated** (hand-written): `openehr-term` (terminology bundle + XML assets + access logic — BMM only has ~6 interface classes), `openehr-query` (AQL lexer/parser/AST), `openehr-adl` (the ADL 2.4 engine), `openehr-flat` (Simplified Formats), and the `ehrbase-*` application crates (idiomatic Rust of our own design on the generated crates).
- **Pinned spec versions:** RM 1.2.0, BASE 1.3.0, TERM 3.1.0, AM 1.4.0 + 2.4.0 (see `docs/VERSIONS.md` — incl. the release ladder and pin-honesty column). **Version policy (owner 2026-07-20):** single pin per component (openEHR minors are compatible supersets within a major); dual generations only across major boundaries — AM's `am14`/`am24` is the one live, spec-mandated case; ITS-REST is single-version, always the latest RELEASED API. Full policy: `docs/VERSIONS.md` §Spec version policy.

## Issue workflow (the loop)

**The tracker is GitHub Issues (owner 2026-07-20): the open issue list IS the worklist.** The former `docs/plans/WORKLIST.md` is a pointer stub; its final pre-migration state lives in git history. Issue state is edited only via `gh`; never track work only in chat.

1. Orient: `gh issue list --state open` (the SessionStart hook injects it). Pick the pinned issue (pins = current focus, max 3) or the issue the user names; read the contract with `gh issue view <n> --comments`.
2. The issue body is the contract: `## Contract` (what + why, owner rulings, spec citations) + `## Exit criteria` checklist (+ optional `## Phases` task list). A deep working plan stays a `docs/plans/*.md` file linked from the issue (delete-on-implementation lifecycle unchanged — the issue replaces the tracker row, not the plan document). New work discovered en route gets its own issue (`gh issue create`), not a prose deferral.
3. Do the work. **First read the governing spec sections** under `docs/specs/openehr/` for anything spec-facing (`/spec-lookup`; hard rule below). The orchestrator keeps the critical path in-session (build in the open so it can be watched and corrected) and may fan bounded implementation/research out per the Model-orchestration section, handing each agent the relevant spec paths. For the openEHR **spec** layer, change the generator and regenerate (never hand-write spec classes — see Code generation above). For the **application** (`ehrbase-*`), build **idiomatic modern Rust of our own design on the generated `openehr-*` crates**, with the openEHR specs as the authority (EHRbase/other CDRs as prior art via their upstream repos when useful). Build compiling, tested increments.
4. Record progress on the issue: tick verified exit-criteria checkboxes (`gh issue edit <n>`), post substantive status/decisions as comments (`gh issue comment <n>`) — the issue thread replaces the old ever-growing status cell.
5. Commit on a conventional-type branch (`feat/…`, `fix/…`, `chore/…` — see the branch hard rule below) with a descriptive subject; the PR body declares `Closes #<n>` so the merge into develop auto-closes the issue (never close by hand when a PR carries the work).
6. When the issue's exit criteria are all met, run `/phase-done`: verify, write the dense close narrative into the PR description, post the handoff comment on the issue, and DELETE the implemented plan file in that same PR.

**Taxonomy (industry-standard, nothing invented):** exactly one **type** label per issue, mapped to the conventional-commit types — `bug`↔fix, `enhancement`↔feat, `documentation`↔docs, plus `chore`/`refactor`/`perf`/`ci`. **Priority** = `P0` (critical, drop everything) / `P1` (high, current focus) / `P2` (normal) / `P3` (backlog). **Domain/area** labels: `spec:RM…CNF`, `spec-update`, `spec-impact:*` (triage adds exactly one), `admin-ui` (the console, its own OCI image). **Spec-version triage** (on `spec-update` issues): `spec-version:current` = fix inside a pinned line, act immediately; `spec-version:next` = lands in a different upstream release, collected under an on-demand `upstream:<comp>-<ver>` label (adoption per the `docs/VERSIONS.md` §Spec version policy). **PR-flow labels** (CI escape hatches, on PRs not issues): `no-changelog` (changelog-guard; genuinely invisible changes only) and `no-ui-visual-change` (ui-screenshot-guard; admin-ui source change with zero visual effect — see `.claude/rules/leptos-ui.md` §10). Both guards read labels from the PR event payload: a label added after a guard failed needs a close+reopen of the PR to re-evaluate. A label referenced by CI must exist in the repo (`gh label create`) — a missing label fails silently at apply time, not in the workflow. **Milestones = releases** (vX.Y.Z): a milestone is a delivery promise — a `blocked-upstream` issue carries NO milestone (it cannot promise; the watcher's auto-unblock note says to assign one at pickup); a release is cut when its milestone reaches zero open issues (procedure in `.claude/rules/changelog.md` — changelog rename, version bumps + goldens, release PR, tag on the merge commit, close the milestone, ensure the next one exists). Issues + git survive `/clear` and `/compact`; the built-in todo tool is session-scoped, so the tracker is the durable layer.

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

- **Orchestrator (Fable 5, high):** owns the phase loop, architecture and design decisions, spec-conformance judgement, and the hard bespoke logic (AQL IR→SQL, versioning, validation, the node codec). Keeps these in-context rather than delegating — they need top intelligence + taste and are the project's critical path.
- **Delegate to Opus-4.8 subagents:** bulk/parallelizable implementation on a clear spec (wiring handlers, DTO/trait impls against the generated ITS-REST contract, migrations, sqlx/sea-query query building, test scaffolding), file-heavy investigation, and codebase analysis — anything that would otherwise burn the orchestrator's context. Fan out several concurrently in one message.
- **Fable-5 subagents:** use when a delegated task still needs top intelligence/taste (a tricky algorithm port, an API-shape decision) but you want it off the main context.
- **sonnet-5:** cheap mechanical passes where correctness is easy to verify (mechanical refactors, boilerplate). **Never use Haiku for substantive work.**
- **Reviews:** the `spec-conformance-reviewer` agent (read-only, Opus), as an independent perspective before committing a subsystem — especially spec/wire conformance and the AQL engine. Spec questions / requirements extraction go to `spec-researcher`; bounded implementation to `implementer`; **every red CNF run goes to `cnf-triage`** (the spec-adjudicated attribution law — see the hard rule below) — all defined in `.claude/agents/`, all handed the governing `docs/specs/openehr/...` paths in the prompt.
- Effort: keep Fable on `high` (xhigh is token-hungry; max/extra is a furnace for worse output). Use `effort: 'low'` for cheap mechanical worker stages, higher tiers only for the hardest verify/judge stages.

Discipline unchanged: subagents still obey the hard rules below (never hand-edit `// @generated`, no test-weakening, conventional-type branches, no AI attribution) — deviations surface at the conformance/corpus suites, so delegate with a tight spec and verify the result.

## Tech stack (pinned)

Toolchain: Rust stable **1.96** (1.96.1), MSRV 1.96, **edition 2024**, resolver v3. Pin via `rust-toolchain.toml`.
Database: **PostgreSQL 18** (target 18.4+): AIO, `uuidv7()`, skip scan, temporal constraints, RETURNING OLD/NEW, plus `JSON_TABLE` from PG 17. Extensions: `uuid-ossp`, `pgcrypto`, `pg_trgm`.

**The authoritative, fully-pinned dependency set lives in the root `Cargo.toml` `[workspace.dependencies]`.** Versions below are current as of 2026-07; do not hand-roll anything a crate here already provides (auth, HTTP status codes, OpenAPI/Swagger, etc.). Add a crate to a member with `dep.workspace = true`. Items marked *(verify)* had their major/minor unconfirmed at authoring; check crates.io before first use.

Web & HTTP core: `axum` 0.8, `axum-extra` 0.12, `axum-server` 0.8 (graceful shutdown + TLS), `tower` 0.5, `tower-http` 0.7 (trace, cors, compression, timeout, limit, request-id, sensitive-headers, catch-panic, normalize-path), `hyper` 1, `hyper-util` 0.1, `http` 1 (status codes/headers), `http-body` 1, `http-body-util` 0.1, `mime` 0.3, `mime_guess` 2, `headers` 0.4, `bytes` 1.

Async runtime: `tokio` 1, `tokio-util` 0.7, `tokio-stream` 0.1, `futures` 0.3, `async-trait` 0.1, `pin-project-lite` 0.2.

Auth & authz (never hand-roll): `jsonwebtoken` 10 (JWT), `oauth2` 5 *(verify)*, `openidconnect` 4 (OIDC/Keycloak) *(verify)*, `argon2` 0.5 + `password-hash` 0.5 (password hashing), `tower-sessions` 0.15 *(verify)*, `axum-login` 0.18 *(verify)*, `secrecy` 0.10, `zeroize` 1. RBAC/ABAC for the Stage 2 restoration: `casbin` 2 *(verify)* or `cedar-policy` 4 *(verify)* (decide at S2).

TLS/crypto: `rustls` 0.23, `tokio-rustls` 0.26, `rustls-pemfile` 2, `webpki-roots` 0.26, `rand` 0.9, `getrandom` 0.3, `sha2` 0.10, `hmac` 0.12, `blake3` 1.

OpenAPI/Swagger: `utoipa` 5, `utoipa-axum` 5, `utoipa-swagger-ui` 9 (serves Swagger UI), `utoipa-redoc`/`utoipa-scalar`/`utoipa-rapidoc` 5. Note (owner rule 2026-07-17): the vendored ITS-REST OpenAPI is the **codegen input** for the generated `openehr-its` contract (`emit-rest`) and the **behavioural oracle** — it is NEVER imported into or served by `ehrbase-rest`. The server serves ONLY its own natively generated OpenAPI (`#[utoipa::path]` on every handler, composed in `ehrbase-rest::extensions::openapi`; owner hard rule: serve only what we generate). Any surface change updates our `#[utoipa::path]` declarations in the same PR.

Database & persistence: `sqlx` 0.9 (postgres, macros, migrate, uuid, json, rust_decimal, chrono; TLS via `tls-rustls-aws-lc-rs`); `sea-query` 1.0 + `sea-query-sqlx` (the sea-query 1.0↔sqlx 0.9 binder — `sea-query-binder` is stuck on sea-query 0.32; dynamic SQL for the AQL→SQL engine — **not** sea-orm); `jiff-sqlx` for jiff↔Postgres on plain sqlx queries (sqlx has no `jiff` feature; the binder's with-jiff is unimplemented upstream); `deadpool-postgres` + `tokio-postgres` for an optional pipelined hot-read path. Migrations: squashed `0001_baseline.sql` per schema; new ones via `sqlx migrate add --sequential`.

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
cargo clippy --workspace --all-targets --all-features   # the EXACT CI flags — dropping --all-features misses feature-gated lints
cargo fmt --all
cargo audit && cargo deny check
# conformance pipeline (the acceptance instrument) — the CNF 2.0 runner:
bash scripts/conformance.sh   # compose up --build (fresh volumes) → the CNF catalogue → verdicts → docs/conformance/<sut>/ (results + verdicts + report/statement/certificate + badges); baseline numbers live ONLY in the committed artifacts
# admin-console gates: /ui-gates (both-target clippy, nextest, leptosfmt, cargo-leptos build)
bash scripts/ui-e2e.sh        # the browser journey battery against the composed stack (merge gate in CI)
```

### Target-dir & warm-build discipline (owner rules 2026-07-12, tightened 2026-07-16)

The workspace is huge; a cold build is expensive and `target/` bloat has
twice filled the disk (~90 GB, then **394 GB on 2026-07-16** — one debug tree
plus four agent lanes plus the IDE dir). These rules keep builds warm and
disk bounded:

- **ONE `./target` for ALL cargo, period (owner ruling 2026-07-16).** The
  per-agent lane scheme (`target/agent-t1` … `t4`) is **retired — never
  recreate it**: every extra target dir is a full duplicate build tree, and
  the lanes alone held 140 GB. Subagents use the same `./target`; cargo's
  own file lock serializes concurrent builds — waiting on the lock is the
  intended behaviour, never work around it with a second target dir, a
  `/tmp`/scratchpad dir, or any `CARGO_TARGET_DIR` override. (Corollary:
  don't have subagents run cargo in parallel at all — the orchestrator runs
  the builds once at convergence.)
- **The IDE is back on the default too (owner, 2026-07-16):** RustRover's
  `CARGO_TARGET_DIR` override is removed — the IDE shares the same
  `./target` as everything else. There is NO `CARGO_TARGET_DIR` override
  anywhere anymore; if the IDE holds the cargo lock, CLI builds wait — that
  is expected, never answer it with a second target dir. Never
  `pkill -9 rustc` to "fix" slowness — it corrupts incremental caches.
- **Iterate scoped, gate wide.** While working: `cargo clippy -p <crate>
  --all-targets` and `cargo nextest run -p <crate>`. The full `--workspace`
  gates run once, before commit. `clippy` shares the check cache — running it
  repeatedly is cheap *if the flag set stays identical*.
- **Never vary `RUSTFLAGS`, features, or profile between runs** — any change
  rebuilds the world. Use the exact commands above, no ad-hoc flags.
- **Disk hygiene:** cargo never garbage-collects `target/debug/deps` (stale
  `.rlib`s accumulate on every dep bump and every wide refactor). Check
  `du -sh target` at the START of any heavy session and after any
  rewrite-scale change; above ~30 GB run `cargo clean` (one cold rebuild,
  then warm again). Check no other session/IDE is mid-build first
  (`pgrep -fl 'cargo|rustc'`) — parallel sessions share this tree.

Note (the old "phases need not compile" gate is retired): the spec + ITS
foundation is generated and **compiling**, and the **application phases build as
compiling, tested increments** on top of it. Do not defer compilation; keep every
crate you touch green. (The historical Bun-style "P1–P16 need not compile" rule
applied only while hand-transcribing the spec, which no longer happens.)

Compile status: the **generated** spec crates `openehr-base`, `openehr-rm`,
`openehr-am` compile and are lib-clippy-clean — keep them that way by fixing the
*emitter*, never by hand-editing generated files. `openehr-lang`,
`openehr-codegen` (hand-written tooling), `openehr-term`
(hand-written), `openehr-query` (AQL parser), and `openehr-its` (canonical
JSON/XML + generated ITS-REST contract; all fidelity gates green) all compile +
clippy-clean. The `ehrbase-*` application crates: **the Stage-1 CDR is
shipped** (v3.5.0 — persistence, greenfield storage, REST + auth incl.
RBAC/ABAC, the full SM service layer, templates, WebTemplate/FLAT/STRUCTURED,
validation, the AQL engine, the admin console); the conformance baseline
lives in the committed `docs/conformance/ehrbase-rs/` artifacts (the CNF 2.0
runner replaced the retired ECC harness 2026-07-22); the EhrScape surface
was cut, not built. Remaining work is tracked exclusively on GitHub Issues.

## Conventions

- Crate boundaries mirror openEHR components. Keep dependencies pointing downward: app (`ehrbase-*`) → spec (`openehr-*`), never the reverse. The `ehrbase-*` crates consume the generated `openehr-*` types directly as their domain model — never re-model the RM or re-serialize.
- **Two disciplines by layer.** Spec/ITS crates (`openehr-*`) = *generated* from the vendored specs: change the emitter and regenerate, never hand-edit `// @generated`; the bar is wire + semantic + invariant parity. Application crates (`ehrbase-*`) = *modern idiomatic Rust of our own design*: use proper crates (axum, sqlx+sea-query-sqlx, oauth2, utoipa); the openEHR specs are the authority; verify at the REST/AQL surface with the CNF conformance suite + corpus tests. Build compiling, tested increments.
- Emission choices the generator already makes (do not re-litigate per class): closed openEHR subtype sets → untagged Rust `enum`s; recursion (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`, `DV_MULTIMEDIA.thumbnail`, F-bounded ranges) → `Box`; `_type` via the native `ToJson`/`FromJson` codec (`openehr-its`, `emit-json`) — the spec types carry no serde; strong types where unambiguous (`uuid::Uuid`, etc.). Behavioural back-references (`PATHABLE.parent()`) in hand-written `*_impl.rs` use `Weak` or an index, never an owning reference.
- `thiserror` in hand-written library crates; `anyhow` only in the binary. No `unwrap`/`expect` outside tests.
- **No `use X as Y` import renaming.** Direct names only; a bad name gets renamed at its definition, a genuine collision gets a qualified path at the use site. Alias only in highly exceptional cases with no other solution (a `use Trait as _;` trait import is not a rename and is fine).
- Async-first: the server is I/O-bound on Postgres; use idiomatic tokio/axum (this is not the Bun "no-Tokio" case).

## IMPORTANT hard rules

- **The vendored spec text is the oracle.** Before implementing or reviewing any spec-facing behaviour (RM semantics, invariants, REST wire, AQL, canonical JSON/XML, templates, terminology), read the relevant section under `docs/specs/openehr/` (use `/spec-lookup`) and cross-check the CNF platform test schedule (`docs/specs/openehr/CNF/`). Never resolve a spec question from memory or from EHRbase behaviour alone; cite the spec file + section for conformance-relevant decisions.
- **CNF red-run triage is spec-adjudicated (owner hard rule, 2026-07-22; the full law is `.claude/rules/cnf-triage.md`, the delegation target is the `cnf-triage` agent).** When a CNF run goes red, the vendored spec text is ALWAYS right and never a suspect: every red row is attributed — before any fix — to exactly one of {the application (`app/*` / `crates/openehr-*` via the codegen emitter), the runner machinery (`tools/cnf-runner/src`), the catalogue artifacts (`tools/cnf-runner/artifacts`)} by three-way comparison of spec-required vs catalogue-expected vs SUT-observed, with the spec citation and the actual wire exchange as evidence. Never assume the failing side; never adjust an expectation to match observed behaviour; spec silence goes through the ambiguity register with a typed disposition.
- **Cite ONLY durable references (owner hard rules 2026-07-11 + 2026-07-17): the vendored openEHR specs (`docs/specs/openehr/`, file + section — e.g. `RM common master06 §Version tree`) or OFFICIAL external documentation (PostgreSQL docs, the Rust book/reference, docs.rs pages of pinned crates). NEVER an internal markdown file.** The ADR layer is deleted (2026-07-17 — it caused more confusion than value); plan/design markdowns are deleted once implemented, so any citation of them goes stale. Where the openEHR specs are SILENT (storage mechanics, indexes, infra, extensions like eventing/multi-tenancy/FHIR), FLAG it explicitly — "no openEHR spec governs this — our own design/extension" — never point at an internal document.
- **Delete a plan/design markdown in the PR that implements it (owner hard rule, 2026-07-17).** Internal markdown is working material, not a record: once its content has landed, the file goes — the durable record is the closed issues + PR descriptions, `CHANGELOG.md`, git history, and the living reference docs (`docs/architecture.md`, `docs/endpoint-map.md`, `docs/VERSIONS.md`). Keeping implemented docs breeds stale claims and conflicts.
- **Never hand-edit a `// @generated` file.** The generated spec crates (`openehr-base`, `openehr-rm`, `openehr-am`) are produced by `openehr-codegen`; edit the emitter or the `*_impl.rs` sibling and regenerate, never the generated file itself.
- **The generator emits the COMPLETE model from the vendored inputs — never trim, scope-down, or suppress output (owner hard rule, 2026-07-19).** Completeness is the entire point of code generation: if the vendored BMM/XSD/OAS (or a legitimate emission closure over them) yields classes, they are ALL emitted, in full, at their source-package-mirrored location — even ones nothing consumes yet ("we may need it in the future" is the design). Never narrow a schema merge, prune a closure, or suppress "missing" generated files to quiet a diff, shrink scope, or dodge a build error — that is HIDING code that should exist, not fixing anything. A generation-side defect discovered en route (e.g. module-declaration plumbing mishandling a hand-written module) is FIXED in the generator in the same change, not restored-around. (Review-enforced; `codegen-drift` guards sync, not completeness — reviewers check for narrowed inputs/closures explicitly.) When consuming code hits a shape in the generated `openehr-*` crates that is wrong or insufficient versus the vendored spec/BMM (a missing subtype seam, a field typed too narrowly, a closed enum a downstream component extends, …), the fix is an `openehr-codegen` emitter/override change + regeneration — NEVER a shadow type, duplicate model, adapter/re-modeling layer, placeholder value, or "temporary" local representation in the consumer. The generated crates exist precisely so implementation consumes the spec model directly and stays spec-conformant by construction; a consumer-side workaround silently forks the spec model and defeats the whole design. If the emitter fix is large, register a tracker issue — the workaround is still forbidden. On discovering an EXISTING workaround, register its removal. (Review-enforced; the codegen-drift job guards the regeneration half.)
- **Pending work is marked ONLY with the official `TODO` form (owner hard rule, 2026-07-17; REINFORCED 2026-07-19 after agents wrote prose deferrals):** `// TODO:` or a scoped `// TODO(perf):` etc. — tool-recognized, IDE-highlighted, CI-greppable. **This includes work deferred to a later phase or session**: a comment like "deferred to the flattener phase", "lands in a later phase", or "resolved in A7" is PENDING WORK and MUST be written `// TODO: <what is missing and what completes it>` — prose deferral notes are forbidden, and phase/plan markers (A5, P16, W-nn) in code are the banned tracker-ID pattern (scrub on touch). Visibility is the point: every unsolved thing must show up in a TODO search. `// NOTE:` stays reserved for SETTLED decisions (with spec citation or the spec-silence flag) — if the comment describes something not yet done, it is a TODO, never a NOTE. The retired bespoke `(port)` vocabulary (`TODO(port)`, `PERF(port)`, `PORT NOTE`, `PORT STATUS`) is banned and CI-enforced (the `comment-markers` guard). A deliberate spec-silent design decision is a plain `// NOTE:` comment carrying the spec citation or the explicit "no openEHR spec governs this — our own design/extension" flag — it is documentation, never pending work. `// SAFETY:` stays reserved for `unsafe` (which is `forbid`den anyway).
- **Branches use the industry-standard conventional types (owner hard rule, 2026-07-19 — the former `claude/*` scheme is RETIRED, never create a new `claude/*` branch):** `<type>/<kebab-case-slug>` with `type` ∈ `feat` | `fix` | `chore` | `docs` | `refactor` | `perf` | `test` | `ci` | `build` | `release` (mirroring the Conventional Commits type set) — e.g. `feat/adl2`, `fix/tenant-guc-pool-stamp`, `chore/dep-bumps`. Pick the type by the dominant change; an issue's phase branch is normally `feat/<issue-slug>` (mirroring the issue's type label). Existing historical `claude/*` branches/PR links stay as recorded facts. Never force-push `main`/`develop`. Never delete `docs/plans/WORKLIST.md` (the tracker pointer stub) or `docs/plans/README.md` (the lifecycle guide); implemented plan files are DELETED in the PR that lands them, with the close recorded in the PR description + the issue's handoff comment. (Enforced by the `block_dangerous` hook's force-push allowlist + review.)
- **NEVER add AI/Claude attribution to commits or PRs. This is an absolute rule with no exceptions.** Do not add a `Co-Authored-By: Claude` trailer (or any co-author trailer). Do not write "Generated with Claude Code", "🤖 Generated with...", "Co-authored-by: Claude", or any similar line, emoji, or footer in a commit message, commit body, commit trailer, PR title, PR description, PR comment, issue, or code comment. Commit messages and PR text describe only the change itself. If you ever find yourself about to add such a line, stop and remove it. When configuring git or opening PRs, do not pass any flag or template that injects attribution.
- **Keep the changelog** (`CHANGELOG.md`, Keep a Changelog 1.1.0): every PR with user-visible changes adds an `[Unreleased]` entry in the same PR — CI (`changelog-guard`) enforces it; releases are cut from the changelog (see `.claude/rules/changelog.md`). The `openehr-*` spec crates are versioned by the spec they implement; the product/workspace follows its own SemVer (3.x).
- **User docs track the product.** Any PR that changes the REST surface, configuration (`EHRBASE_*`), the CLI, deployment artifacts (compose/Helm/containers), or other user-visible behaviour must update the matching `website/book/src` page in the same PR (see `.claude/rules/docs-website.md`). Never hand-edit `website/api/spec/**` — run `scripts/assemble-oas.sh` (CI drift gate).
- **Never weaken, skip, or delete a test** to make a build pass, and never edit a test to route around a bug it exposes.
- **Reliability hard rules are machine-enforced** (`.claude/rules/reliability.md`, owner 2026-07-17): every safety rule pairs with the lint/CI check that fails on violation — a rule without a failing check is a wish. The rules + their enforcement register live in `.claude/rules/reliability.md` itself.
- **Record progress on the tracker (tick issue checkboxes / comment / open issues for new work) and commit before ending a session.** A `Stop` hook enforces this.
- **Application phases build as compiling, tested increments** on top of the generated `openehr-*` crates — do not defer compilation. (The old "phases need not compile" rule applied only to the retired hand-transcription era.)
- RBAC/ABAC authz (`ehrbase-rest::access`) and multi-tenancy are SHIPPED — they were built greenfield, not restored. The remaining v1 enterprise archaeology (plugin system and any other unrestored capability) stays a Stage 2/3 concern; `reference/v1` remains a read-only git ref consulted only for that work.

## References

- @docs/architecture.md (the current design)
- @docs/VERSIONS.md + @docs/postgres-features.md (pins + the PG 17/18 features we use)
- @ROADMAP.md (the product roadmap); the open-items tracker is GitHub Issues (`gh issue list --state open`)
