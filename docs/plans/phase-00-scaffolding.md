# Phase 00 — Scaffolding

- Status: done (research dossier texts pending delivery — sole open task)
- Started: 2026-07-02   Completed: 2026-07-02   Owner: Ruben
- Consumes (spec/layer): none (bootstrap phase)
- Compile required: yes — must build (empty + moved crates)

## Objectives

Fork EHRbase, convert the Maven reactor into a single root Cargo workspace,
`git mv` the existing Java into the three server crates, stand up the ten
empty openEHR spec crates, generate the `.claude/` agent harness and the
`docs/` tree, pin `reference/v1` as a read-only git ref, and record (not
build) the v1-vs-v2 enterprise-feature archaeology.

## Preconditions

- [x] Fork of `ehrbase/ehrbase` exists and is the working repo
- [x] `PORT_MASTER_PLAN.md` and `CLAUDE.md` are in place at the repo root

## Scope

In: workspace layout, crate skeletons, `git mv` reorganization, `.claude/`
harness, `docs/` tree, git hooks, CI workflows, reference/v1 tag, archaeology
diff (record only).
Out: any actual Java-to-Rust porting or spec transcription — that starts at
Phase 01. No enterprise feature (RBAC etc.) is built here, only catalogued.

## Tasks

- [x] Place delivered root configs, git hooks, CI workflows, and the porter agent at their target paths; install .githooks via core.hooksPath — done; AGENTS.md symlinked to CLAUDE.md
- [x] Point Cargo.toml [workspace.package] repository/authors at the fork — rubentalstra/ehrbase-rs
- [x] Register the agent-harness hooks in .claude/settings.json (attribution guard, java/Maven protection, dangerous-command block, fmt/clippy, phase context, phase gate) — commit-msg stripper proven live: deletes attribution lines, aborts fully-attributed messages (note: the literal tool name is itself a stripped token — keep it out of commit subjects)
- [x] Pin reference/v1 read-only ref at v0.32.0 (last pre-v2 tag) — v0.32.0 → v2.0.0 was the cut
- [x] Create the 13 workspace crates (10 spec + 3 server) with workspace lints and Section 9 dependency arrows; empty workspace builds — builds in ~1.7s; clippy zero warnings
- [x] Generate the .claude/ harness (rules, skills, agents) — 6 rules, 9 skills, 7 agents (incl. delivered porter)
- [x] Generate the docs/ tree (VERSIONS, PORTING, ROSETTA, LIFETIMES.tsv, PROGRESS, architecture, ADR template, research README, plans/) — 31 files
- [x] Run the v1-vs-v2 archaeology diff into docs/enterprise/v1-vs-v2-delta.md (record only; build nothing) — 287 lines; real losses: ABAC + multi-tenancy (PG RLS); authn/plugin survived
- [x] git mv the EHRbase Java into the three server crates per Section 9.1; Flyway migrations verbatim to crates/openehr-server/migrations/ — 428 java files, 42 migration files, zero left in Maven layout
- [x] Remove the Maven-era GitHub workflows and fork-sync pull.yml (replaced by the Rust CI pipeline) — 12 workflows + pull.yml (hardreset sync would have wiped the fork); all recoverable from git history
- [ ] Commit the two research dossiers to docs/research/ — pending delivery of the dossier texts (README placeholder in place)
- [x] Verify: cargo build --workspace, cargo fmt --all --check, cargo deny check, attribution-guard test commit — all green; nextest runs (0 tests yet)

## Exit criteria

- [x] Workspace builds with empty spec crates and relocated Java in place
- [x] `.claude/` harness and hooks are registered and functioning
- [x] `docs/` tree is complete per Section 13 (research dossier texts themselves still to be delivered)
- [x] Archaeology recorded in `docs/enterprise/v1-vs-v2-delta.md`
- [x] `reference/v1` pinned as a read-only ref
- [x] CI workflows are Rust-based (no Maven-era workflow remains)

## Decisions made this phase

- Java landing scheme: each Maven module gets its own directory under its
  crate, module root package stripped (rest-openehr `org/ehrbase/rest` →
  `openehr-rest/src/`; service/aql/rm_db_format/config/plugin/cli/api/db/
  application under `openehr-server/src/<module>/`). Keeps `service`'s own
  `plugin` subpackage separate from the `plugin` module; aql-engine's
  module-level `org.ehrbase.openehr.util` landed at `src/aql/util`.
- `api` module moved wholesale to `openehr-server/src/api/`; its openEHR-DTO
  half is superseded by the spec crates (written from the specs), the split
  into service traits happens at porting time (Section 9.1 note).
- Test trees to `crates/*/tests/java/<module>/`, test/main resources to
  `tests/resources/<module>/` and `resources/<module>/`.
- Maven poms are co-located with the sources they built (crate root for
  rest-openehr / rest-ehr-scape; `src/<module>/pom.xml` inside
  openehr-server), read-only until P99. Reactor-level files with no source
  to follow (root pom.xml, .mvn/, bom/, test-coverage/) stay at the
  workspace root.
- `base` module no longer exists at v2.33 (dissolved upstream) — mapping row moot.
- No generated jOOQ code was committed upstream (build-time generation), so
  "discard jOOQ" was a no-op; the one hand-written helper
  (AdditionalSQLFunctions) went to `openehr-server/src/db/`.
- 12 Maven-era GitHub workflows + fork-sync `pull.yml` (hardreset from
  upstream) deleted in favor of the Rust ci.yml/release.yml; recover any of
  them from git history when a phase needs the idea (Docker publish at P99,
  CodeQL-for-Rust any time, integration reporting at P18).
- Config fixes: rustfmt nightly-only import options commented out (stable
  1.96 warn-spam); deny.toml `allow-wildcard-paths = true` for internal
  path deps (all crates publish = false); .gitignore inline-comment bug on
  the Cargo.lock line fixed (pattern never matched).
- PostgreSQL pin bumped 18.3+ → 18.4+ (latest 18.x point release; tag
  `postgres:18.4` verified on Docker Hub). CI services, VERSIONS.md,
  CLAUDE.md, master plan, and the sqlx rule all updated together per the
  VERSIONS.md no-drift rule.

## Handoff for next session

P0 is complete except committing the two research dossier texts (external
deliverable; docs/research/README.md marks the slot). The workspace builds
green (build, fmt, clippy zero warnings, deny all four gates), 428 Java files
sit beside their future .rs homes, reference/v1 = v0.32.0, and the
archaeology is recorded. Next: P1 (docs/plans/phase-01-foundation-identification.md)
— transcribe BASE 1.2.0 Foundation + Identification into openehr-foundation /
openehr-base via the rm-transcriber agent, settling the multiple-inheritance,
covariance, and generics patterns that every later RM phase reuses. Needs the
published BASE 1.2.0 spec at hand; .claude/rules/rm-transcription.md carries
the settled hazards.
