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
  `date-released` (the `citation-guard` CI job enforces the version match),
  then re-render `.zenodo.json` (`bash scripts/render/zenodo-json.sh`, checked
  by the same job) — it is GENERATED, never hand-edited, because Zenodo ignores
  `CITATION.cff` completely whenever a `.zenodo.json` exists
  (<https://help.zenodo.org/docs/github/describe-software/zenodo-json/>),
  + the default image tags in `docker-compose.yml` — the `ghcr.io/…:X.Y.Z`
  fallbacks the standalone quickstart pulls, guarded by
  `scripts/checks/compose-image-tags.sh` / the `compose-version-guard` CI
  job), merge the release PR, then tag
  `vX.Y.Z` on the merge commit. The release workflow publishes the GitHub
  Release from the matching changelog section and **fails if the section or
  version match is missing**. Releases publish as OFFICIAL
  releases — `prerelease` is true only for an explicitly suffixed tag
  (`vX.Y.Z-rc1`, ...) — owner sign-off 2026-07-31.
- **The Helm chart, in the SAME release PR:** bump
  `deploy/helm/ferroehr/Chart.yaml` `appVersion` to the release version (the
  `chart-appversion-guard` CI job enforces it, and so do the
  `artifacthub.io/images` tags in the same file), and bump the chart's own
  `version` if any packaged chart content changed in the cycle
  (`chart-version-guard`). The two are INDEPENDENT SemVer lines and stay so.
  Regenerate the golden renders (`deploy/helm/validate.sh --update`).
- **The chart publishes on the tag** (`publish-chart.yml`): it refuses to
  overwrite a published chart version, checks that the `appVersion` image
  accepts the chart's rendered defaults, injects that release's
  `artifacthub.io/changes` from this changelog, pushes to
  `oci://ghcr.io/rubentalstra/charts`, attests it through Sigstore, and
  re-pushes the Artifact Hub ownership tag. A chart-only fix between releases
  uses the same lane by `workflow_dispatch` with `publish: true`. **Never
  re-publish a chart version — bump it:** an OCI tag is MUTABLE, so `helm push`
  would silently replace it with a different digest, and the immutability the
  guards assume exists only because the lane enforces it.
- **The tag also produces an archived artifact.** Zenodo is connected to this
  repository, so publishing a release makes Zenodo take the zipball and mint a
  DOI from `.zenodo.json`. That deposit is immutable — whatever the metadata
  says at publish time is what the DOI carries — so the metadata is verified
  BEFORE the cut, never after. A badge cites the CONCEPT DOI; a version DOI
  freezes on whichever release was current the day it was added.
- **After the tag:** close the `vX.Y.Z` milestone (`gh api … milestones/N
  -f state=closed`) and make sure the NEXT milestone exists so triage always
  has a target. The milestone's closed-issue list + the changelog section
  together are the release's record. Then post a **status update on the
  public roadmap board** (`scripts/gh/project.sh update on-track "…"` —
  policy + honesty rules in `.claude/rules/project-board.md`): what the
  release shipped, what the next milestone targets, `--target` only from a
  real milestone due date. Give the next milestone its due date, then
  `scripts/gh/project.sh sync-dates` so the board's Roadmap timeline
  follows the re-milestoned issues.
- **Versioning split:** the product (workspace, `ferroehr-*`, tools, codegen
  tooling) follows the product SemVer (3.x line). The `openehr-*` **spec
  crates** publish to crates.io on their own independent SemVer line —
  permanently decoupled from the vendored spec versions (owner rulings
  2026-08-04, issue #1886 + same-day correction; the implemented spec pins
  live per generation — the emitted `Generation` enum (alone) in the
  generated crates, a literal crate-level `SPEC_VERSION` in the
  hand-written ones — full policy in
  `docs/VERSIONS.md` §Product and crate versioning); they release in
  lockstep and never with the product version.

- **Publishing the `openehr-*` crates (crates.io):** releases go through
  `.github/workflows/publish-crates.yml` — a manual `workflow_dispatch` lane
  (dry-run by default; a real publish needs the `publish` input set to
  `true`) that authenticates via **crates.io Trusted Publishing** (OIDC,
  `rust-lang/crates-io-auth-action`, `id-token: write`, the `crates-io`
  environment) and publishes the eight `crates/*` members **one at a time in
  dependency order**, treating "already exists on crates.io index" as done —
  no long-lived crates.io token exists anywhere. It is deliberately NOT
  `cargo publish --workspace`: that command is all-or-nothing at the START
  (it refuses the entire run if any member version already exists) while
  being non-atomic at the END, so a partial publish cannot be finished by
  re-running it. That combination stranded `openehr-its` and `openehr-adl` at
  `0.0.10` while six siblings reached `0.0.15` (issue #2211), and a split set
  is a broken graph for every consumer, not a cosmetic lag. The lane also
  reads the registry back before reporting success, so a publish that
  finished half the set fails instead of going green. Version bumps happen in the CONTENT PR, not at publish
  time: any PR changing packaged crate content bumps every crate's `version`
  and the internal `version =` requirements together (lockstep `0.0.x` —
  the `crate-version-guard` CI job and the local push hook enforce it; full
  rule in `.claude/rules/crates-publishing.md`), so a publish just ships the
  version already in the tree — verify locally with
  `cargo publish --workspace --dry-run` (the whole-workspace form is still
  the right DRY RUN: nothing is uploaded, so its all-or-nothing behaviour
  costs nothing and it checks every member together). The very first release
  of a crate cannot use OIDC (crates.io requires an existing crate to
  configure a Trusted Publisher) — it is pushed manually with a scoped API
  token, after which each crate's Trusted Publisher is configured on
  crates.io (repository `rubentalstra/FerroEHR`, workflow
  `publish-crates.yml`, environment `crates-io`).
