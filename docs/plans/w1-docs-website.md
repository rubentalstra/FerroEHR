# Phase W1 — Public documentation website (mdBook on GitHub Pages)

- Status: in-progress
- Started: 2026-07-11   Owner: Ruben
- Consumes: the 2026-07-11 framework research (two passes, live-verified);
  owner ruling: **Rust-only toolchain → mdBook** (rust-lang/mdBook 0.5.x)
- Compile required: n/a (site build must be green in CI)

## Objectives

A public site at `https://rubentalstra.github.io/ehrbase-rs/` for end users:
a product **landing page**, a **versioned** documentation book tied to release
tags, and an **OpenAPI endpoint reference** (Swagger-UI-style, static) — all
built and deployed by CI from this repo, with drift structurally impossible.

## Fixed design decisions

1. **mdBook, Rust-only toolchain.** All site tooling installs via
   `cargo install`/`cargo-binstall`; no Node/Python build step. Plugins
   (verify every version live on crates.io before pinning — hard rule):
   `mdbook-mermaid` (diagrams), `mdbook-admonish` (callouts). Link checking:
   prefer `lychee` (actively maintained Rust link checker, runs over the
   built HTML) — evaluate `mdbook-linkcheck` maintenance state first and
   record the choice here.
2. **Layout** — everything under `website/`:
   - `website/landing/` — hand-written static HTML/CSS landing page (logo,
     hero, feature grid, conformance badges, links into the book + API
     reference). No framework.
   - `website/book/` — the mdBook (`book.toml`, `src/SUMMARY.md`, theme
     overrides incl. the version picker).
   - `website/api/` — vendored **Swagger UI dist** static assets (pin the
     release, verify live) + the OAS documents. Endpoints shown per API
     group; "try it out" not required.
   - `website/versions.json` — the version manifest the picker reads.
3. **URL scheme on Pages:** `/` = landing · `/docs/dev/` = book built from
   `develop` · `/docs/vX.Y/` = frozen book per release tag ·
   `/docs/latest/` = alias of the newest release (dev until the first tag) ·
   `/api/` = the endpoint reference.
4. **Versioning machinery (ours, ~100 lines).** CI on tag `v*` builds the
   book into `/docs/vX.Y/`, appends to `versions.json`, and redeploys; a
   small `theme/` JS injects a version dropdown reading `/versions.json`.
   Old versions are frozen artifacts (mike-style: generate once, never
   rebuild).
5. **OpenAPI = the generated contract, verbatim.** The OAS served at `/api/`
   is produced from the same vendored ITS-REST bundles `openehr-codegen --
   emit-rest` consumes (`crates/openehr-its`), assembled by a script in CI
   and **diffed against the committed copy — any drift fails the build**.
   The API docs therefore cannot diverge from the server contract.
6. **Content is rewritten for end users**, sourced from the internal docs
   (`docs/architecture.md`, `docs/design/*`, `docs/enterprise/*`, README) —
   never moved verbatim. Internal engineering docs (ADRs, blueprint, specs)
   stay in `docs/` and are not published.

## Book content plan (SUMMARY skeleton)

- Introduction (what openEHR is, what EHRbase-rs is, why)
- Getting started — Docker Compose quick start, first EHR, first AQL query
- Installation — Compose · Kubernetes/Helm · from source · configuration
  reference (`EHRBASE_*`)
- Concepts — openEHR primer (RM, archetypes/templates, AQL, versioning),
  system architecture
- Using the API — REST walkthroughs per resource (EHR, EHR_STATUS,
  COMPOSITION, DIRECTORY, CONTRIBUTION), content negotiation (JSON/XML),
  errors; links into `/api/`
- Querying with AQL — language guide, stored queries, `ALL_VERSIONS`,
  TERMINOLOGY(), pagination
- Templates & validation — OPT upload, WebTemplate, FLAT/STRUCTURED formats
- Beyond the core — EHR Extract & messaging, demographics, terminology
  servers, change events (AMQP), FHIR connectors, S3 multimedia
- Security & multi-tenancy — authn (Basic/OIDC), RBAC/ABAC, tenants, ATNA
- Operations — deployment checklist, database roles, backup/PITR,
  observability, upgrades
- Conformance — what is measured, how to run it, reading the statement +
  certificate
- Contributing

## Anti-drift discipline (the standing rules this phase installs)

- **CI `docs.yml`:** pinned mdBook toolchain → regenerate the OAS bundle +
  fail on diff → `mdbook build` → link check (fail on any broken link) →
  assemble (landing + book + api) → `actions/deploy-pages`. Tag pushes cut
  a frozen version first.
- **CLAUDE.md hard rule (added at W1.5):** any PR that changes the REST
  surface, configuration, CLI, deployment artifacts, or user-visible
  behaviour updates the matching `website/book` page in the same PR.
- **`.claude/rules/docs-website.md`:** authoring guide + the same-PR rule,
  scoped to `website/**`.
- **`/phase-done` skill:** gains a "user docs updated?" checklist item.

## Tasks

- [ ] W1.1 Scaffold `website/` — book.toml + SUMMARY skeleton + theme +
      landing page + version-picker JS + `docs.yml` workflow (deploy dev)
- [ ] W1.2 OpenAPI pipeline — OAS assembly script from the vendored ITS-REST
      bundles + vendored Swagger UI + the drift gate
- [ ] W1.3 Write the book — all chapters in the content plan, sourced from
      the internal docs, end-user voice
- [ ] W1.4 Versioning machinery — versions.json + picker + tag workflow;
      prove with a dry-run tag build
- [ ] W1.5 Anti-drift wiring — CLAUDE.md rule, `.claude/rules/docs-website.md`,
      `/phase-done` checklist item
- [ ] W1.6 Go live — enable Pages, first deploy, README links the site

## Exit criteria

- [ ] Site live: landing + `/docs/dev/` + `/api/` endpoint reference
- [ ] CI gates green: link check + OAS drift check both demonstrably fail on
      injected breakage (negative-tested once, then reverted)
- [ ] Version cut proven by dry run
- [ ] The docs discipline installed (CLAUDE.md + rules + phase-done)
- [ ] README points at the site

## Decisions made this phase

- Framework: mdBook (owner ruling 2026-07-11 — Rust-only toolchain; accepts
  owning the version picker + static Swagger UI embed, both documented above).

## Handoff for next session

Phase opened 2026-07-11 after the docs cleanup (PR #51). Next action: W1.1.
