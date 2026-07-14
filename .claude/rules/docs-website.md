---
paths: ["website/**", "scripts/assemble-oas.sh", "scripts/build-site.sh", "scripts/cut-version.sh", ".github/workflows/docs.yml"]
---

# The documentation website (`website/**`)

<!-- Path-scoped 2026-07-13: the global "same-PR docs" reminder lives in the
     root CLAUDE.md hard rules; this file carries the detail and loads when
     website files are touched. -->

The public site — https://rubentalstra.github.io/ehrbase-rs/ — is built from
`website/` by `.github/workflows/docs.yml` and deployed to GitHub Pages.
Design authority: `docs/design/docs-website.md` (§2 layout/URLs, §3 look &
feel, §4 content map).

## Layout & URLs

- `website/landing/` — the hand-written landing page (root `/`). Relative
  URLs only; light+dark via `prefers-color-scheme`; zero external runtime
  requests.
- `website/book/` — the mdBook (served at `/docs/dev/`, frozen per release at
  `/docs/vX.Y.Z/`, newest release copied to `/docs/latest/`).
- `website/api/` — the OpenAPI endpoint reference (vendored Swagger UI + the
  served OAS copies).
- Frozen versions live on the `docs-dist` orphan branch (generate once,
  never rebuilt); `scripts/cut-version.sh vX.Y.Z` cuts one (CI does this on
  every `v*` tag).

## The same-PR docs rule (mirror of the changelog rule)

**Any PR that changes the REST surface, configuration (`EHRBASE_*`), the
CLI, deployment artifacts (compose/Helm/containers), or other user-visible
behaviour must update the matching `website/book/src` page in the same PR.**
The `/phase-done` checklist enforces it at phase close.

## Authoring rules

- End-user voice: second person, task-first. Audiences are integrators,
  operators, and evaluators — never repo-internal.
- **Never publish or link:** `docs/ADRs/**`, `docs/blueprint/**`,
  `docs/plans/**`, `docs/PROGRESS.md`, `docs/specs/**`, `.claude/**`.
  Paraphrase into user language instead.
- Callouts: mdBook 0.5 native `> [!NOTE]` / `[!WARNING]` / `[!TIP]` (no
  plugin syntax). Diagrams: ```mermaid fenced blocks. Long chapters get
  `<!-- toc -->` after the intro paragraph.
- Every endpoint, header, status code, and config key must be verified
  against the code or the vendored OAS before it is written down.

## Never hand-edit

- `website/api/spec/**` — produced by `scripts/assemble-oas.sh` from the
  vendored ITS-REST bundles (CI drift gate `--check` fails otherwise).
- `website/api/vendor/**`, `website/book/theme/mermaid*` — vendored assets.

## Toolchain

Pinned in `docs.yml` `env` (mdBook, mdbook-mermaid, mdbook-toc, mdbook-lint,
lychee, Swagger UI). Bump only after live-verifying the new version and
update the pins + `docs/design/docs-website.md` §1 together.

## Local preview

`bash scripts/build-site.sh` then serve `_site/`
(`python3 -m http.server -d _site`). CI equivalence is the point — if it
works locally it works deployed.
