# FerroEHR rename programme — working checklist

Tracker: issue #1353 (owner decision 2026-07-31: the product rebrands from
**EHRbase-rs** to **FerroEHR**). This is the deep working plan behind that
issue; per the plan lifecycle it is DELETED in the PR that completes the
rename. Each phase below becomes a sub-issue of #1353 when picked up; ticking
here is working state, the tracker is authoritative.

Standing constraints (from the decision comments on #1353):

- The **generated `openehr-*` spec crates keep their names** — they are
  versioned by the openEHR spec they implement, not by the product brand.
- The brand never contains "openEHR" (Foundation trademark; a Product Use
  License would be required). Prose says "an openEHR® CDR" with the required
  attribution line.
- The GitHub **org `FerroEHR` stays a parked name reservation** for now; the
  repository remains on the owner's personal account (feature downgrade on
  free-tier org). Repo *rename* and org *transfer* are separate steps.
- Fork provenance stays recorded (`docs/VERSIONS.md` §EHRbase reference
  point); the rename changes the brand, not the history.

## Phase 0 — brand assets (blocks everything visual)

- [x] Owner picks the logo option + palette (2026-07-31: the "Fe element
      tile" logo with the "Oxide & Iron" palette)
- [x] Production SVG set under `assets/brand/`: icon, icon+wordmark lockup,
      light + dark variants, monochrome variant
- [ ] Convert the SVG `<text>` elements to outlined paths (rendering must not
      depend on viewer-installed fonts) and settle the final wordmark typeface
- [ ] Favicon set (32/16 px PNG + .ico from the committed `favicon.svg`) and
      README banner
- [ ] Palette committed as design tokens (admin-ui CSS custom properties +
      website book theme)
- [ ] Admin console logo/title swap (`app/ehrbase-admin-ui`; screenshot-guard
      baselines regenerate)

## Phase 1 — GitHub repo rename (cheap, reversible, do early)

- [ ] `gh repo rename ferroehr` (stays under the personal account; GitHub
      auto-redirects old URLs, but redirects break the moment a new repo
      reuses the old name — so update references anyway, below)
- [ ] Verify: open PRs/issues, Actions, branch protection, Pages (if any),
      webhooks survived the rename
- [ ] Badges/links that embed the repo slug: README badges, website book
      links, `docs/conformance/` report links, `CITATION.cff` `repository`,
      Cargo.toml `repository` fields

## Phase 2 — local references (every machine/agent that has a clone)

- [ ] `git remote set-url origin git@github.com:rubentalstra/ferroehr.git`
- [ ] Local directory rename `~/RustroverProjects/ehrbase-rs` →
      `~/RustroverProjects/ferroehr` — **gotcha:** the Claude harness keys
      its project dir on the absolute path
      (`~/.claude/projects/-Users-…-ehrbase-rs/`), and that dir's `memory/`
      is a symlink into the repo's `.claude/memory/`. After the folder
      rename, recreate the symlink from the NEW harness project dir and
      confirm `MEMORY.md` loads; never break the link (root `CLAUDE.md`)
- [ ] RustRover: reopen from the new path (one shared `./target` rule
      unchanged; expect one cold build after the move)
- [ ] Re-run `scripts/gh-rel.sh tree 1353` + one `gh issue list` to confirm
      `gh` resolves the renamed repo from the new clone
- [ ] Grep sweep for the old slug/path: `rubentalstra/ehrbase-rs`,
      `RustroverProjects/ehrbase-rs` in scripts, workflows, compose files,
      docs, hooks, `.claude/` settings

## Phase 3 — product identity in the workspace (the big PR)

- [ ] Workspace/application crate renames: `ehrbase` → `ferroehr`,
      `ehrbase-rest` → `ferroehr-rest`, `ehrbase-server` → `ferroehr-server`,
      `ehrbase-admin-ui` → `ferroehr-admin-ui`; bin name `ehrbase` →
      `ferroehr` (tools `cnf-runner`/`testkit`/`openehr-codegen` unaffected)
- [ ] Config env prefix `EHRBASE_*` → `FERROEHR_*` — decide the deprecation
      story (accept both for one minor with a startup warning, or hard cut
      pre-announcement while user count is ~0); no openEHR spec governs this
      — our own design
- [ ] REST base path: decide whether `/ehrbase/rest/openehr/v1` becomes
      `/ferroehr/rest/openehr/v1` (wire-visible! CNF catalogue + ixit +
      admin-ui client + website examples all update in the same change;
      spec-check: ITS-REST prescribes the path suffix, the leading segment
      is deployment-specific)
- [ ] `docs/conformance/<sut>` identifier `ehrbase-rs` → `ferroehr` (runner
      artifacts, ixit party ids, badges, render scripts) — baseline artifacts
      re-emitted, numbers unchanged
- [ ] Test/database/telemetry identifiers: `testkit` template-db names,
      service names in compose/Helm, OTLP service.name, `ehrbase-testkit-pg18`
      container name
- [ ] Docs sweep: root `CLAUDE.md`, nested `CLAUDE.md`s, `docs/architecture.md`,
      `docs/VERSIONS.md` (provenance section STAYS, product line renamed),
      `ROADMAP.md`, `.claude/rules/*`, agent defs, `CHANGELOG.md` entry
      (### Changed: product renamed to FerroEHR)

## Phase 4 — distribution & web

- [ ] OCI images: `ehrbase` / `ehrbase-admin-ui` → `ferroehr` /
      `ferroehr-admin-ui` (GHCR under the renamed repo; old tags stay
      pullable, publish a final old-name tag pointing at the release notes)
- [ ] Helm chart rename + `appVersion`/values/golden renders
      (`deploy/helm/validate.sh --update`)
- [ ] Website/book: title, logo, `ferroehr.eu` as canonical domain (DNS +
      hosting + redirect from any old URL), `scripts/assemble-oas.sh` output
      metadata (served OpenAPI `info.title`)
- [ ] `README.md` rewrite: new name/logo, "formerly EHRbase-rs" note, openEHR®
      attribution line, provenance paragraph
- [ ] Release note announcing the rename (rides the next minor, not a patch)

## Phase 5 — protection & follow-through

- [ ] Formal trademark search (EUIPO + USPTO, Nice classes 9/42/44) for
      "FerroEHR"/"Ferro" in health software — before any public announcement
- [ ] Register `ferroehr.com` + `ferroehr.org` (squat insurance;
      `ferroehr.eu` already held)
- [ ] Decide org-transfer timing (parked `FerroEHR` org; revisit when a paid
      org plan or GitHub feature parity makes it costless)
- [ ] Delete this plan file in the PR that completes the final phase
