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
  version match is missing**. Releases stay `prerelease: true` until the
  owner's production sign-off.
- **After the tag:** close the `vX.Y.Z` milestone (`gh api … milestones/N
  -f state=closed`) and make sure the NEXT milestone exists so triage always
  has a target. The milestone's closed-issue list + the changelog section
  together are the release's record.
- **Versioning split:** the product (workspace, `ehrbase-*`, tools, codegen
  tooling) follows the product SemVer (3.x line).
  The `openehr-*` **spec crates** carry the version of the openEHR
  specification they implement (BASE 1.3.0, RM 1.2.0, AM 2.4.0, ADL 2.4.0, TERM 3.1.0,
  LANG 1.0.0, QUERY 1.1.0, ITS 1.1.0 — see `docs/VERSIONS.md`); bump them
  only on a spec-pin bump, never with the product version.
