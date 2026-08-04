# Changelog discipline (Keep a Changelog 1.1.0)

`CHANGELOG.md` at the repo root is the hand-curated release record
(https://keepachangelog.com/en/1.1.0/ + SemVer). It is enforced, not
optional:

- **Every PR that changes user-visible behaviour adds an entry under
  `## [Unreleased]` in the same PR.** User-visible = the REST surface, AQL
  semantics, validation behaviour, storage/migrations, configuration, CLI,
  container/compose/Helm artifacts, feature flags. The CI `changelog-guard`
  job fails the PR otherwise (escape hatch: the `no-changelog` label, only
  for genuinely invisible changes — internal refactors, CI, docs).
- Entries go under the standard subsections: `### Added` / `### Changed` /
  `### Deprecated` / `### Removed` / `### Fixed` / `### Security`. Write for
  the end user (what changed for them), not commit-message prose.
- **Releases are milestone-driven (owner 2026-07-20; milestones =
  releases):** the `vX.Y.Z` GitHub milestone collects the release's issues;
  the release is cut when the milestone reaches **zero open issues** (or
  the owner calls the cut and moves the stragglers to the next milestone).
- **Cutting a release** (on a `release/vX.Y.Z` branch): rename
  `[Unreleased]` → `## [X.Y.Z] - YYYY-MM-DD`, re-add an empty
  `[Unreleased]`, update the link references at the bottom, bump the
  workspace `version` in the root `Cargo.toml` (+ `Cargo.lock` via a
  `cargo check`, + Helm `appVersion`, golden renders via
  `deploy/helm/validate.sh --update`, + `CITATION.cff` `version` and
  `date-released` — the `citation-guard` CI job enforces the version
  match), merge the release PR, then tag
  `vX.Y.Z` on the merge commit. The release workflow publishes the GitHub
  Release from the matching changelog section and **fails if the section or
  version match is missing**. Releases publish as OFFICIAL
  releases — `prerelease` is true only for an explicitly suffixed tag
  (`vX.Y.Z-rc1`, ...) — owner sign-off 2026-07-31.
- **After the tag:** close the `vX.Y.Z` milestone (`gh api … milestones/N
  -f state=closed`) and make sure the NEXT milestone exists so triage always
  has a target. The milestone's closed-issue list + the changelog section
  together are the release's record. Then post a **status update on the
  public roadmap board** (`scripts/gh-project.sh update on-track "…"` —
  policy + honesty rules in `.claude/rules/project-board.md`): what the
  release shipped, what the next milestone targets, `--target` only from a
  real milestone due date. Give the next milestone its due date, then
  `scripts/gh-project.sh sync-dates` so the board's Roadmap timeline
  follows the re-milestoned issues.
- **Versioning split:** the product (workspace, `ferroehr-*`, tools, codegen
  tooling) follows the product SemVer (3.x line). The `openehr-*` **spec
  crates** publish to crates.io on their own independent SemVer line —
  permanently decoupled from the vendored spec versions (owner rulings
  2026-08-04, issue #1886 + same-day correction; each crate's `SPEC_VERSION`
  constant carries the implemented spec pin — full policy in
  `docs/VERSIONS.md` §Product and crate versioning); they release in
  lockstep and never with the product version.

- **Publishing the `openehr-*` crates (crates.io):** releases go through
  `.github/workflows/publish-crates.yml` — a manual `workflow_dispatch` lane
  (dry-run by default; a real publish needs the `publish` input set to
  `true`) that authenticates via **crates.io Trusted Publishing** (OIDC,
  `rust-lang/crates-io-auth-action`, `id-token: write`, the `crates-io`
  environment) and runs `cargo publish --workspace` over the eight
  `crates/*` members in dependency order — no long-lived crates.io token
  exists anywhere. Version bumps happen in the CONTENT PR, not at publish
  time: any PR changing packaged crate content bumps every crate's `version`
  and the internal `version =` requirements together (lockstep `0.0.x` —
  the `crate-version-guard` CI job and the local push hook enforce it; full
  rule in `.claude/rules/crates-publishing.md`), so a publish just ships the
  version already in the tree — verify locally with
  `cargo publish --workspace --dry-run`. The very first release
  of a crate cannot use OIDC (crates.io requires an existing crate to
  configure a Trusted Publisher) — it is pushed manually with a scoped API
  token, after which each crate's Trusted Publisher is configured on
  crates.io (repository `rubentalstra/FerroEHR`, workflow
  `publish-crates.yml`, environment `crates-io`).
