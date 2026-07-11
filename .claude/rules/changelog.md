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
- **Cutting a release:** rename `[Unreleased]` → `## [X.Y.Z] - YYYY-MM-DD`,
  re-add an empty `[Unreleased]`, update the link references at the bottom,
  bump the workspace `version` in the root `Cargo.toml` (+ Helm
  `appVersion`, golden renders via `deploy/helm/validate.sh --update`), then
  tag `vX.Y.Z`. The release workflow publishes the GitHub Release from the
  matching changelog section and **fails if the section or version match is
  missing**. Releases stay `prerelease: true` until the owner's production
  sign-off.
- **Versioning split:** the product (workspace, `ehrbase-*`, tools,
  `openehr-flat`, codegen tooling) follows the product SemVer (3.x line).
  The `openehr-*` **spec crates** carry the version of the openEHR
  specification they implement (BASE 1.3.0, RM 1.2.0, AM 2.4.0, TERM 3.1.0,
  LANG 1.0.0, QUERY 1.1.0, ITS 1.0.3 — see `docs/VERSIONS.md`); bump them
  only on a spec-pin bump, never with the product version.
