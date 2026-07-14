# Phase W1 — Public documentation website (mdBook on GitHub Pages)

- Status: done (2026-07-11)
- Started: 2026-07-11   Owner: Ruben
- Consumes: this specification (every version + capability below **live-verified
  2026-07-11** from crates.io / `gh release list` / official docs — see §7
  Sources appendix); owner rulings (mdBook · Rust-only toolchain · GitHub Pages ·
  versioned per release tag · static OpenAPI endpoint viewer).
- Compile required: n/a (the site build must be green in CI; no workspace impact)

## Objectives

A public site at `https://rubentalstra.github.io/ehrbase-rs/` for end users:
a product **landing page**, a **versioned** documentation book tied to release
tags, and a static **OpenAPI endpoint reference** — all built and deployed by CI
from this repo (Rust-only toolchain + vendored static assets, **no Node/Python
build step**), with content-drift against the server contract structurally
impossible.

This file is the authoritative, fully-worked specification. Everything below is
verified; the task list in §6 is the build order.

---

## 1. Toolchain manifest (live-verified 2026-07-11)

**Verification rule:** every version was checked today from the source named in
the row. "mdBook 0.5 compat" was established from the plugin's declared build
dependency — mdBook 0.5 **split its Rust API into separate crates** (`mdbook`,
`mdbook-preprocessor`, `mdbook-renderer`, …) *and* changed the preprocessor JSON
protocol, so a preprocessor built against `mdbook = "0.4.x"` **fails at runtime
under the 0.5 binary** with `Unable to parse the input - invalid type: null,
expected any valid TOML value` (confirmed: mdbook-admonish issue #233, open since
mdBook 0.5.0). A preprocessor is 0.5-safe **iff** it depends on
`mdbook-preprocessor = "0.5.x"` (the new crate) — not on `mdbook = "0.4"`.

### 1a. Core build tool

| Tool | Version | Released | Source | Install | Use |
|---|---|---|---|---|---|
| **mdBook** | **0.5.4** | 2026-07-06 | `gh release list rust-lang/mdBook` → v0.5.4 (Latest) | `cargo install --version 0.5.4 mdbook` (or `cargo binstall mdbook@0.5.4`) | the static-site generator for the book |

mdBook 0.5.4 is confirmed latest (previous session's 0.5.4 / 2026-07-06 re-verified). **Relevant 0.5 changes we exploit or must respect** (from the mdBook CHANGELOG, docs.rs):
- **Native admonitions** — `> [!NOTE]` / `[!WARNING]` / … render as styled
  callouts, **enabled by default** (`output.html.admonitions`). *This removes the
  need for a callout plugin entirely* (see 1c).
- **Native sidebar heading navigation** — per-page heading nav in the sidebar
  (configurable toggle). *This removes the need for `mdbook-pagetoc`.*
- **Definition lists** enabled by default; Font Awesome 4.7 → 6.2 (SVG-embedded).
- **Breaking (must honour in `book.toml`):** unknown config fields now **error**;
  `curly-quotes` → `smart-punctuation`; `output.html.copy-fonts` removed; built-in
  Google-Analytics support removed. → our `book.toml` must use only current keys.
- **Breaking (theming):** `{{#previous}}`/`{{#next}}` are now objects;
  `{{theme_option}}` helper removed. → we do **not** fork `index.hbs` (see §2).

### 1b. Adopted mdBook plugins (all 0.5-compatible, Rust-only)

| Plugin | Version | Released | mdBook 0.5 compat | Source | Install | Use |
|---|---|---|---|---|---|---|
| **mdbook-mermaid** | **0.17.0** | 2025-11-18 | ✅ depends `mdbook-preprocessor = "0.5.0"` | github.com/badboy/mdbook-mermaid | `cargo binstall mdbook-mermaid@0.17.0` | render the architecture mermaid diagrams (same flowcharts as README). `mdbook-mermaid install website/book` vendors `mermaid.min.js` + `mermaid-init.js` into the book theme → **no CDN at runtime** ✔ |
| **mdbook-toc** | **0.15.4** | 2026-05-28 | ✅ depends `mdbook-preprocessor = "0.5.0"` | github.com/badboy/mdbook-toc | `cargo binstall mdbook-toc@0.15.4` | inline `<!-- toc -->` table-of-contents block at the top of the long chapters (AQL guide, REST walkthrough, config reference). Complements 0.5's sidebar heading nav (that is left-sidebar; this is in-page). |
| **mdbook-lint** | **0.14.4** | 2026-06-03 | ✅ run in **standalone CLI mode** (protocol-independent) | github.com/joshrotenberg/mdbook-lint | `cargo binstall mdbook-lint@0.14.4` | markdown-quality CI gate: `mdbook-lint lint website/book/src`. 83 rules. Its `mdbook = "0.4"` dep is **only** used in preprocessor mode, which we do not use — the standalone linter reads markdown directly, so mdBook 0.5's protocol change is irrelevant here. Build without the preprocessor feature: `cargo install mdbook-lint --no-default-features --features standard` if the 0.4 dep ever causes friction. |

### 1c. Evaluated-and-skipped mdBook plugins (full third-party catalog)

Assessed against github.com/rust-lang/mdBook/wiki/Third-party-plugins. Rule: adopt
only if actively maintained **and** 0.5-compatible **and** `cargo install`-able.

| Plugin | Latest / date | Decision | One-line justification |
|---|---|---|---|
| **mdbook-admonish** | 1.20.0 / 2025-06-06 | **SKIP** | Incompatible with mdBook 0.5 (targets `mdbook = "0.4.40"`; issue #233 open, no fix). **Superseded by mdBook 0.5 native admonitions.** |
| **mdbook-alerts** | 0.8.0 / 2025-06-29 | **SKIP** | Targets `mdbook = "0.4.51"` (0.5-unsafe) **and** redundant — mdBook 0.5 renders GitHub `> [!NOTE]` alerts natively. |
| **mdbook-pagetoc** | 0.3.0 / 2025-11-27 | **SKIP** | Per-page nav is now a mdBook 0.5 built-in (sidebar heading navigation). |
| **mdbook-sitemap** | 0.1.2 / 2026-06-28 | **SKIP** | Immature (0.1.x) renderer-backend, per-book only. Our sitemap must span landing + `/api/` + `/docs/latest/` across the whole Pages site, which a per-book backend cannot express — we emit `sitemap.xml` in the deploy assembly step (§2, deterministic, we own every URL). `mdbook-sitemap-generator` 0.2.0 rejected for the same cross-section reason. |
| **mdbook-lint** | (adopted, see 1b) | ADOPT | — |
| **mdbook-katex** | 0.9.4 / (0.10.0-alpha) | **SKIP** | No mathematical notation anywhere in the content plan; adopting it would be dead weight. Revisit only if a future chapter needs LaTeX. |
| **mdbook-i18n-helpers** | 0.4.0 / 2025-11-27 | **SKIP (future work)** | Localization is out of scope for W1 (English only). Google-maintained and healthy; note it as the adoption target **if/when** translations are commissioned. |
| **mdbook-github-authors** | 0.1.0 / 2025-02-06 | **SKIP** | Per-page author attribution is not a product-docs concern; contributors are covered by `CONTRIBUTING.md` + git history. |
| **mdbook-linkcheck** | 0.7.7 / **2022-10-03** | **SKIP (loser — see 1d)** | Unmaintained ~4 years; renderer-backend coupled to the mdBook 0.4 API → broken on 0.5. Replaced by lychee. |

### 1d. Link checker — decision: **lychee**

| Candidate | Version / date | Verdict |
|---|---|---|
| **lychee** ✅ | **0.24.2** / 2026-05-01 (github.com/lycheeverse/lychee) | **CHOSEN.** Actively maintained standalone Rust binary; checks the **built HTML** (`website/_site`) *and* markdown, follows external links with retry/rate-limit/`--accept` handling, caches, honours `.lycheeignore`. mdBook-version-independent (operates on output, not the preprocessor protocol). Install: `cargo binstall lychee@0.24.2` (or the `lycheeverse/lychee-action` in CI; we use the binary for parity with the repo's `cargo`-first idiom). |
| mdbook-linkcheck ✗ | 0.7.7 / 2022-10-03 | **REJECTED.** Last release 2022; a renderer backend bound to the mdBook 0.4 API (breaks on 0.5); only sees the book, not the landing page or `/api/`. Recorded as the loser: staleness + 0.5 incompatibility + narrower scope. |

### 1e. OpenAPI endpoint viewer — decision: **Swagger UI (vendored dist)**

| Candidate | Version / date | Bundle | Offline (no runtime CDN) | Multi-spec dropdown | Verdict |
|---|---|---|---|---|---|
| **Swagger UI** ✅ | **5.32.8** / 2026-06-23 (`gh release list swagger-api/swagger-ui`) | `dist/` ≈ 1.5 MB (swagger-ui-bundle.js + css), fully self-contained | ✅ vendored `dist/`, zero external requests | ✅ **native `urls: [{url,name}]`** top-right dropdown | **CHOSEN** |
| Redoc | 2.5.3 / 2026-05-29 (Redocly/redoc) | `redoc.standalone.js` ≈ 1 MB, self-contained | ✅ | ❌ single-spec only — 7 groups would need 7 pages + a hand-rolled menu | rejected |
| Scalar | rolling (release-2026-07-08) | `@scalar/api-reference` standalone JS | ⚠ standalone must be self-hosted from an npm dist (no clean semver tag; date-tagged monorepo) | ✅ (`sources`) | rejected: pinning a Node-package dist contradicts the Rust-only / vendored-asset rule and the rolling date-tags are awkward to freeze |

**Decision: Swagger UI 5.32.8.** It is the only candidate that is (a) cleanly
semver-pinnable, (b) fully offline from a vendored `dist/`, and (c) has a
**built-in multi-spec dropdown** — exactly right for our **7 API-group** bundles.
It also matches the server's own in-app `/ehrbase/swagger-ui` (brand
consistency). Vendoring (no npm): download the `v5.32.8` source tarball, copy
`dist/{swagger-ui-bundle.js, swagger-ui-standalone-preset.js, swagger-ui.css,
favicon-*.png}` into `website/api/vendor/swagger-ui/`, commit once, and ship a
hand-written `index.html` that lists our 7 specs in `urls`. This is a **vendored
static asset**, not a build step.

### 1f. GitHub Actions (live-verified latest majors)

| Action | Pinned tag | Latest release / date | Use |
|---|---|---|---|
| `actions/checkout` | **@v7** | v7.0.0 / 2026-06-18 | checkout (matches repo's existing `@v7`) |
| `actions/configure-pages` | **@v6** | v6.0.0 / 2026-03-25 | Pages setup / base-path detection |
| `actions/upload-pages-artifact` | **@v5** | v5.0.0 / 2026-04-10 | upload the assembled `_site` tree |
| `actions/deploy-pages` | **@v5** | v5.0.0 / 2026-03-25 (moving tags v5/v4/v3/v2 exist) | deploy to the `github-pages` environment |
| `actions-rust-lang/setup-rust-toolchain` | **@v1** | v1.17.0 / 2026-06-25 | Rust toolchain (matches repo idiom) |
| `taiki-e/install-action` | **@v2** | v2.83.1 / 2026-07-10 | fast prebuilt install of mdbook/mermaid/toc/lint/lychee (matches repo idiom) |
| `Swatinem/rust-cache` | **@v2** | (repo idiom) | cache cargo builds for the plugin installs |

`taiki-e/install-action` already ships prebuilt binaries for `mdbook`,
`mdbook-mermaid`, `lychee` (and falls back to `cargo binstall`), so the site job
installs the whole toolchain in seconds with no compile. `cargo-bins/cargo-binstall`
(v1.20.1 / 2026-06-21) is the documented local-dev fallback.

**No minifier / no Node tooling** — landing HTML/CSS is hand-authored and already
small; mdBook output is served as-is. (An optional Rust `minhtml`/`minify-html`
pass is explicitly *not* adopted — not worth the toolchain surface.)

---

## 2. Repository + site architecture

### 2a. `website/` directory tree

```
website/
├── landing/                         # hand-written static landing page (no framework)
│   ├── index.html                   # the product landing page (§3b)
│   ├── style.css                    # design tokens + layout (§3a), light/dark
│   ├── 404.html                     # site-root 404 (Pages serves this for any miss)
│   ├── robots.txt                   # crawl policy (canonical = latest; §2d)
│   └── assets/
│       ├── logo.svg                 # copied from /assets/logo.svg (build step)
│       ├── logo-mark.svg            # gear-R mark only, for the nav (derived)
│       └── og-card.png              # social/OpenGraph preview (optional)
├── book/                            # the mdBook
│   ├── book.toml                    # 0.5 config (only current keys — §1a)
│   ├── theme/                       # ONLY additive assets (no index.hbs fork — §2e)
│   │   ├── custom.css               # design-token overrides on the `rust` theme
│   │   ├── version-picker.js        # additional-js: injects the version <select>
│   │   ├── mermaid.min.js           # vendored by `mdbook-mermaid install`
│   │   └── mermaid-init.js          # vendored by `mdbook-mermaid install`
│   └── src/
│       ├── SUMMARY.md               # the chapter tree (§4)
│       ├── introduction.md
│       ├── getting-started/ …       # (chapters per §4)
│       └── … (see §4 content map)
├── api/                             # the static OpenAPI endpoint reference
│   ├── index.html                   # Swagger UI host with the 7-spec `urls` dropdown
│   ├── vendor/swagger-ui/           # vendored Swagger UI 5.32.8 dist (committed once)
│   │   ├── swagger-ui-bundle.js
│   │   ├── swagger-ui-standalone-preset.js
│   │   ├── swagger-ui.css
│   │   └── favicon-32x32.png
│   └── spec/                        # the 7 served OAS bundles (committed copy — drift-gated)
│       ├── ehr.openapi.yaml         # ← vendor/rest-oas/ehr-html.openapi.yaml
│       ├── definition.openapi.yaml  # ← definition-html.openapi.yaml
│       ├── query.openapi.yaml       # ← query-html.openapi.yaml
│       ├── demographic.openapi.yaml # ← demographic-html.openapi.yaml
│       ├── admin.openapi.yaml       # ← admin-html.openapi.yaml
│       ├── system.openapi.yaml      # ← system-html.openapi.yaml
│       └── overview.openapi.yaml    # ← overview-html.openapi.yaml
├── versions.json                    # the version manifest the picker reads (§2c)
└── README.md                        # "how the website is built" (points at this phase file)
```

Supporting scripts (under repo `scripts/`, matching the existing idiom):
- `scripts/assemble-oas.sh` — copy the 7 `*-html.openapi.yaml` from
  `crates/openehr-its/vendor/rest-oas/` into `website/api/spec/` (§5a).
- `scripts/build-site.sh` — local one-shot: assemble OAS → build book → stage
  `_site/` exactly as CI does (so "works on my machine" == CI).

### 2b. URL scheme on GitHub Pages (project pages → base path `/ehrbase-rs/`)

The site is served under the **project sub-path** `https://rubentalstra.github.io/ehrbase-rs/`.
This is *the* classic Pages pitfall: every absolute asset/link must carry the
`/ehrbase-rs/` prefix. Handling:
- **Landing / api (hand-written):** use **relative** URLs only (`./`, `../`) so
  the sub-path never appears in source. Verified by lychee against the built
  `_site` (which is laid out under the real prefix in a check dir).
- **Book (mdBook):** set `[output.html] site-url` **per deployment path** at build
  time — mdBook uses `site-url` for the 404 page, search, and root-relative links.
  Each book is built with the site-url matching where it will live.

| Path (browser) | Served from `_site/` | mdBook `site-url` | Content |
|---|---|---|---|
| `/ehrbase-rs/` | `_site/index.html` | — | landing page |
| `/ehrbase-rs/api/` | `_site/api/` | — | Swagger UI endpoint reference |
| `/ehrbase-rs/docs/dev/` | `_site/docs/dev/` | `/ehrbase-rs/docs/dev/` | book built from `develop` |
| `/ehrbase-rs/docs/vX.Y.Z/` | `_site/docs/vX.Y.Z/` | `/ehrbase-rs/docs/vX.Y.Z/` | **frozen** book per release tag |
| `/ehrbase-rs/docs/latest/` | `_site/docs/latest/` | `/ehrbase-rs/docs/latest/` | copy of the newest release (real tree — deep links work) |
| `/ehrbase-rs/versions.json` | `_site/versions.json` | — | version manifest |
| `/ehrbase-rs/sitemap.xml`, `/robots.txt` | `_site/…` | — | SEO |

**`latest` alias mechanism (decided):** Pages has **no server redirects**, and an
HTML meta-refresh only redirects the index (deep links like
`…/latest/querying-aql.html` would 404). Therefore `latest` is a **real,
self-consistent copy** built from the newest tag's sources with
`site-url=/ehrbase-rs/docs/latest/` — deep links work, search works. The storage
cost (one extra book copy) is accepted. `latest` is (re)generated on every deploy
from the newest tag entry; the frozen `vX.Y.Z` trees are **never** rebuilt.

**404:** `website/landing/404.html` is copied to `_site/404.html`; Pages serves it
for any unmatched path across the whole site (landing, api, and book misses).

### 2c. Frozen-version store (the "generate once, never rebuild" machinery)

GitHub Actions Pages deployment replaces the **entire** site each deploy, so
frozen `vX.Y.Z` trees must persist somewhere and be re-materialized every deploy.
We use a **mike-style archive branch** rather than rebuilding old tags:

- Orphan branch **`docs-dist`** is the version archive. It holds only built HTML:
  `docs/vX.Y.Z/…` per released version, plus the canonical `versions.json`.
- **On a `v*` tag** (`docs-release` job): build that version's book once
  (`site-url=/ehrbase-rs/docs/vX.Y.Z/`), rsync it into a `docs-dist` checkout under
  `docs/vX.Y.Z/`, prepend the version to `versions.json`, commit + push
  `docs-dist`. Frozen forever; never rebuilt (honours owner ruling #4).
- **On every deploy** (`docs-deploy` job — develop push, tag, or manual): check out
  `docs-dist` (all frozen versions), build `/docs/dev/` fresh from `develop`, build
  `/docs/latest/` from the newest tag's sources, drop in landing + api +
  versions.json + sitemap/robots, and upload the assembled `_site` as the Pages
  artifact.

`versions.json` shape (the picker + the deploy job both read it):

```json
{
  "latest": "v3.0.0",
  "versions": [
    { "id": "dev",    "label": "dev (develop)",  "path": "/ehrbase-rs/docs/dev/",    "released": null,        "prerelease": true },
    { "id": "latest", "label": "latest (v3.0.0)", "path": "/ehrbase-rs/docs/latest/", "released": "2026-07-11", "aliasOf": "v3.0.0" },
    { "id": "v3.0.0", "label": "v3.0.0",          "path": "/ehrbase-rs/docs/v3.0.0/", "released": "2026-07-11", "prerelease": true }
  ]
}
```

Until the first tag is cut, `versions.json` contains only the `dev` entry and
`latest` resolves to `dev` (documented in the picker JS).

### 2d. SEO / canonical (avoid duplicate-content dilution across versions)

- `sitemap.xml` (emitted in the deploy assembly, deterministic — we own every URL)
  lists **only** the landing page, `/api/`, and the pages of `/docs/latest/`. It
  deliberately omits `/docs/dev/` and older `/docs/vX.Y.Z/` so search engines
  index the canonical latest docs.
- `robots.txt`: `Allow: /`, `Sitemap: https://rubentalstra.github.io/ehrbase-rs/sitemap.xml`,
  `Disallow: /ehrbase-rs/docs/dev/` and a `Disallow` per archived non-latest
  version (written by the deploy job from `versions.json`).
- Landing/api `<head>` carry a self-referential `<link rel="canonical">`.

### 2e. Version picker — decided: `additional-js`, **not** an `index.hbs` fork

mdBook 0.5 changed theme helpers (`{{theme_option}}` removed, prev/next now
objects), so **forking `theme/index.hbs`** would couple us to upstream template
internals and require a re-sync on every mdBook bump. Instead, `book.toml` sets
`[output.html] additional-js = ["theme/version-picker.js"]` (a stable,
long-standing option, unchanged in 0.5). `version-picker.js` (~60 lines):
1. `fetch("/ehrbase-rs/versions.json")`,
2. build a `<select>` (current version pre-selected from `location.pathname`),
3. inject it into the mdBook top menu bar (`.right-buttons`, with a defensive
   fallback to `.menu-bar`),
4. `onchange` → navigate to the chosen version's `path`.

It also injects the **"API Reference ↗"** external link into the same bar (points
at `/ehrbase-rs/api/`). Rationale recorded: additive JS survives mdBook theme
churn; a full `index.hbs` fork does not.

### 2f. `docs.yml` workflow (complete design)

Two workflows (mirrors the repo's separation of concerns):

- **`.github/workflows/docs.yml`** — build + deploy the site.
- The **version cut** is a job *inside* `docs.yml` gated on `v*` tags (keeps all
  docs logic in one file; chains off the same `v*` tag the existing `release.yml`
  and `containers.yml` already trigger on).

```yaml
# .github/workflows/docs.yml
name: Docs

on:
  push:
    branches: [develop]     # redeploy dev docs + landing/api
    tags: ["v*"]            # cut a frozen version, then redeploy
  pull_request:             # PR: build + lint + link-check ONLY (no deploy)
  workflow_dispatch:

# Pinned toolchain — the single source of truth for the docs build.
env:
  MDBOOK_VERSION: "0.5.4"
  MDBOOK_MERMAID_VERSION: "0.17.0"
  MDBOOK_TOC_VERSION: "0.15.4"
  MDBOOK_LINT_VERSION: "0.14.4"
  LYCHEE_VERSION: "0.24.2"
  SWAGGER_UI_VERSION: "5.32.8"
  SITE_BASE: "/ehrbase-rs"

concurrency:
  group: docs-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  # ── Build + verify (always; no deploy) ──────────────────────────────────────
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: taiki-e/install-action@v2
        with:
          tool: mdbook@${{ env.MDBOOK_VERSION }},mdbook-mermaid@${{ env.MDBOOK_MERMAID_VERSION }},mdbook-toc@${{ env.MDBOOK_TOC_VERSION }},mdbook-lint@${{ env.MDBOOK_LINT_VERSION }},lychee@${{ env.LYCHEE_VERSION }}
      - name: OAS drift gate (docs copy == vendored ITS-REST)
        run: bash scripts/assemble-oas.sh --check       # regenerate + git diff --exit-code
      - name: Markdown lint gate
        run: mdbook-lint lint website/book/src
      - name: Assemble _site (landing + api + /docs/dev/)
        run: bash scripts/build-site.sh --dev-only       # writes ./_site
      - name: Link-check gate (built HTML + external links)
        run: lychee --no-progress --root-dir "$PWD/_site" './_site/**/*.html'
      - uses: actions/upload-artifact@v7
        with: { name: site, path: _site }

  # ── Cut a frozen version on a v* tag (push to docs-dist; never rebuilt) ─────
  version-cut:
    if: startsWith(github.ref, 'refs/tags/v')
    needs: build
    runs-on: ubuntu-latest
    permissions: { contents: write }        # push to docs-dist
    steps:
      - uses: actions/checkout@v7
        with: { fetch-depth: 0 }
      - uses: taiki-e/install-action@v2
        with: { tool: "mdbook@${{ env.MDBOOK_VERSION }},mdbook-mermaid@${{ env.MDBOOK_MERMAID_VERSION }},mdbook-toc@${{ env.MDBOOK_TOC_VERSION }}" }
      - name: Build frozen vX.Y.Z + push to docs-dist
        run: bash scripts/cut-version.sh "${GITHUB_REF_NAME}"   # site-url, rsync into docs-dist, update versions.json, commit+push

  # ── Deploy the full site (develop push, tag, or manual) ─────────────────────
  deploy:
    if: github.event_name != 'pull_request'
    needs: [build]          # (+ version-cut ordering enforced by `needs` on tags via a matrix guard in the script)
    runs-on: ubuntu-latest
    environment: { name: github-pages, url: ${{ steps.pages.outputs.page_url }} }
    permissions: { pages: write, id-token: write, contents: read }
    concurrency: { group: pages, cancel-in-progress: false }
    steps:
      - uses: actions/checkout@v7
      - name: Fetch docs-dist (frozen versions)
        run: git fetch origin docs-dist && git worktree add docs-dist origin/docs-dist || true
      - uses: taiki-e/install-action@v2
        with: { tool: "mdbook@${{ env.MDBOOK_VERSION }},mdbook-mermaid@${{ env.MDBOOK_MERMAID_VERSION }},mdbook-toc@${{ env.MDBOOK_TOC_VERSION }}" }
      - uses: actions/configure-pages@v6
      - name: Assemble full _site (landing + api + dev + frozen versions + latest + sitemap)
        run: bash scripts/build-site.sh --full
      - uses: actions/upload-pages-artifact@v5
        with: { path: _site }
      - id: pages
        uses: actions/deploy-pages@v5
```

PR builds run `build` only (lint + drift + link-check, **no deploy**) — a broken
PR fails the gates without touching the live site.

---

## 3. Look & feel specification

### 3a. Design tokens (from the actual logo)

Extracted from `assets/logo.svg`: gear + wordmark **rust orange `#B7410E`**,
badge/tagline **dark umber `#5A2408`**, mark centre **white `#FFFFFF`**. These
become the palette; fonts are a **system stack** (no external font CDN — Pages +
privacy) with the logo's Georgia serif reserved for the mark.

```css
:root {
  /* brand */
  --rs-rust:        #B7410E;   /* primary accent (links, buttons, headings rule) */
  --rs-rust-600:    #9A360B;   /* hover/active */
  --rs-umber:       #5A2408;   /* secondary / deep accent */
  --rs-umber-050:   #F6EDE7;   /* rust tint surface (light) */
  /* light theme */
  --rs-bg:          #FFFFFF;
  --rs-surface:     #FAF7F5;
  --rs-text:        #1B1613;
  --rs-muted:       #6B605A;
  --rs-border:      #E7DDD6;
  /* fonts */
  --rs-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  --rs-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  --rs-serif: Georgia, "Times New Roman", serif;   /* the "R" mark only */
}
@media (prefers-color-scheme: dark) {
  :root {
    --rs-bg:      #14100E;
    --rs-surface: #1E1815;
    --rs-text:    #F3ECE8;
    --rs-muted:   #B3A79F;
    --rs-border:  #332822;
    --rs-rust:    #E4712F;     /* brightened for contrast on dark (AA ≥ 4.5:1) */
    --rs-rust-600:#C85A1D;
    --rs-umber-050:#241812;
  }
}
```

All colour pairings target WCAG AA (≥ 4.5:1 body text). Dark mode is driven by
`prefers-color-scheme` on the landing/api; the book additionally honours mdBook's
own theme toggle (the `custom.css` overrides map onto both light and coal/navy).

### 3b. Landing page (`website/landing/index.html`) — section by section

Pure static HTML/CSS, single file + `style.css`, responsive (CSS grid/flex,
`max-width:100%` media), dark/light via `prefers-color-scheme`. No JS required
(one tiny optional scroll-reveal is progressive-enhancement only).

1. **Top nav** — sticky, translucent. Left: `logo-mark.svg` + "EHRbase-rs". Right:
   Docs (`./docs/latest/`), API (`./api/`), Conformance (anchor), GitHub ↗. A
   theme note: follows OS; no toggle needed.
2. **Hero** — full `logo.svg`, the tagline *"A pure-Rust openEHR Clinical Data
   Repository — spec-compliant, measured, built for production."*, one-line
   sub-copy from the README opener. Two CTAs: **Get started** (→ `./docs/latest/getting-started/`)
   and **View the API** (→ `./api/`). Subtle rust-tint gradient background.
3. **Measured-conformance proof strip** — a horizontal band of the real numbers
   (from `docs/conformance/`): **341 executed · 315 passed · 0 failed**, and the
   three profile verdicts **CORE PASS · STANDARD PASS · OPTIONS OBTAINED**, each a
   pill linking to `docs/conformance/CONFORMANCE_CERTIFICATE.md`. Framed by the
   line "Compliance you can verify, not just read."
4. **Feature grid (~8 cards)** — icon + title + one sentence, drawn from the
   README feature list: (1) REST API 1.0.3 (JSON+XML), (2) AQL 1.1 engine incl.
   `ALL_VERSIONS`, (3) Full versioning & contributions, (4) Templates & deep
   validation, (5) One static Rust binary, (6) PostgreSQL 18-native storage,
   (7) Security: OIDC · RBAC/ABAC · multi-tenant · ATNA, (8) Integration: FHIR R4 ·
   AMQP events · S3 multimedia.
5. **Quick-start code block** — the README `docker compose up --build` +
   create-EHR + AQL curl, in a copy-styled `<pre>` (mono, rust caret accent).
6. **Integration logos row** — text/wordmark chips (no external images, avoid
   trademark/CDN issues): PostgreSQL 18 · Keycloak/OIDC · RabbitMQ/AMQP · FHIR R4 ·
   S3 / SeaweedFS · Kubernetes/Helm · Prometheus/OpenTelemetry.
7. **Footer** — Apache-2.0 (→ `LICENSE`), the acknowledgment (fork of EHRbase by
   vitasystems + PLRI; openEHR Foundation specs), links: GitHub, Security policy,
   Changelog, Conformance. "Not affiliated with or endorsed by upstream EHRbase."

### 3c. Book theme (`website/book/theme/custom.css`)

- `book.toml`: `[output.html] default-theme = "rust"`, `preferred-dark-theme = "coal"`,
  `additional-css = ["theme/custom.css"]`, `additional-js = ["theme/version-picker.js"]`,
  `git-repository-url`, `edit-url-template`, `smart-punctuation = true`,
  `[output.html.search] enable = true`, `[output.html.admonitions]` left at default
  (native callouts on).
- `custom.css` overrides the mdBook CSS custom properties to the brand: link
  colour → `--rs-rust`, sidebar active → rust, header rule → rust, inline-code tint
  → `--rs-umber-050`. Keeps mdBook's typography; only recolours.
- **Sidebar structure** = the SUMMARY tree (§4). The **version picker** `<select>`
  and the **"API Reference ↗"** link appear in the top menu bar via
  `version-picker.js` (§2e). Mermaid diagrams render via the vendored
  `mermaid.min.js` (no CDN).

---

## 4. Content plan → source map

Every chapter is **rewritten for end users** from the internal sources (never
moved verbatim). Sources below were confirmed to exist on disk (2026-07-11).
Voice: second-person, task-oriented, minimal jargon; each chapter opens with a
one-paragraph "what/why". Length guidance per row (S ≈ ½ page, M ≈ 1–2 pp,
L ≈ 3–5 pp).

| SUMMARY chapter | Primary source(s) (real paths) | Audience voice | Len |
|---|---|---|---|
| Introduction | `README.md` (opener, "Why"), `docs/architecture.md` (layers) | newcomer to openEHR + product | M |
| Getting started | `README.md` (quick start), `docker-compose.yml`, `docker/ehrbase.dev.toml`, `docs/design/container-images.md` | evaluator, first run | M |
| Installation → Compose | `docker-compose.yml`, `docs/design/container-images.md` | operator | S |
| Installation → Kubernetes/Helm | `docs/design/helm-deployment.md`, `deploy/helm/ehrbase-rs/` | platform engineer | M |
| Installation → From source | `README.md` (building), `rust-toolchain.toml` | contributor/operator | S |
| Installation → Configuration reference | the **119 `EHRBASE_*`** keys (grep of `app/`,`docker/`), `docker/ehrbase.dev.toml` | operator | L |
| Concepts → openEHR primer (RM, archetypes/templates, AQL, versioning) | `docs/architecture.md`, `README.md` | newcomer | M |
| Concepts → System architecture | `docs/architecture.md` (+ the mermaid flowchart from README) | technical evaluator | M |
| Using the API → per-resource walkthroughs (EHR, EHR_STATUS, COMPOSITION, DIRECTORY, CONTRIBUTION) | `crates/openehr-its/vendor/rest-oas/*-html.openapi.yaml`, `README.md`, links into `/api/` | integrator | L |
| Using the API → content negotiation (JSON/XML) + errors | `docs/architecture.md` (REST surface), OAS bundles | integrator | M |
| Querying with AQL | `docs/design/aql-engine.md`, `README.md` (AQL features) | data consumer | L |
| Templates & validation | `README.md` (templates), `docs/architecture.md` (templates/validation), CHANGELOG (OPT/FLAT/STRUCTURED) | clinical modeller | M |
| Beyond the core → EHR Extract & messaging | `CHANGELOG.md` (SM extract/message/TDD), `docs/architecture.md` (SM map) | integrator | M |
| Beyond the core → Demographics | `README.md`, `docs/architecture.md` (Demographic service) | integrator | S |
| Beyond the core → Terminology servers | `docs/terminology-validation.md`, `docs/design/terminology-server-integration.md` | integrator | M |
| Beyond the core → Change events (AMQP) | `README.md`, ADR-014 (paraphrased, not linked) | integrator | S |
| Beyond the core → FHIR connectors | `README.md`, ADR-016 (paraphrased) | integrator | S |
| Beyond the core → S3 multimedia | `README.md`, ADR-017 (paraphrased) | operator | S |
| Security & multi-tenancy | `docs/design/access-control.md`, `docs/design/atna-audit.md`, `README.md` (auth) | security engineer | L |
| Operations | `docs/design/helm-deployment.md`, `docs/design/observability.md` | operator/SRE | L |
| Conformance | `docs/conformance/CONFORMANCE_REPORT.md`, `…_STATEMENT.md`, `…_CERTIFICATE.md`, `docs/design/conformance-framework.md` | procurement/evaluator | M |
| Contributing | `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` | contributor | S |

**What must NEVER be published** (stays in `docs/`, excluded from the book and
from lychee's scope): `docs/ADRs/**`, `docs/blueprint/**`, `docs/plans/**`,
`docs/PROGRESS.md`, `docs/spec-audit/**` (retired), `docs/specs/openehr/**` (the
vendored oracle), `docs/research/**`, `docs/VERSIONS.md`, `.claude/**`,
`PORT_MASTER_PLAN.md`, and any internal `docs/design/*` that reveals unshipped
roadmap. The book links to *product* pages (LICENSE, CONTRIBUTING, conformance
artifacts) only. ADRs may be *paraphrased* into user language but never linked or
copied.

---

## 5. Anti-drift wiring (concrete)

### 5a. OAS drift gate — `scripts/assemble-oas.sh`

Mirrors `scripts/check-codegen-drift.sh` (regenerate → `git diff --exit-code`).
The served specs are a **deterministic copy** of the vendored `-html` bundles (with the pinned ITS-REST release stamped over upstream's `info.version: latest` so Swagger UI shows a real version; Admin keeps its honest `development`) (which the
codegen-drift gate already ties to upstream), so drift is caught at two hops:
upstream → vendor (existing gate) → `website/api/spec/` (this gate).

```bash
#!/usr/bin/env bash
# Copy the 7 documentation OAS bundles into the served copy; --check fails on drift.
set -euo pipefail
cd "$(dirname "$0")/.."
SRC="crates/openehr-its/vendor/rest-oas"
DST="website/api/spec"
declare -A MAP=(
  [ehr]=ehr [definition]=definition [query]=query
  [demographic]=demographic [admin]=admin [system]=system [overview]=overview
)
mkdir -p "$DST"
for group in "${!MAP[@]}"; do
  cp "$SRC/${group}-html.openapi.yaml" "$DST/${MAP[$group]}.openapi.yaml"
done
if [[ "${1:-}" == "--check" ]]; then
  if ! git diff --quiet -- "$DST"; then
    echo "::error::website/api/spec is out of sync with the vendored ITS-REST OAS. Run scripts/assemble-oas.sh and commit." >&2
    git diff --stat -- "$DST" >&2
    exit 1
  fi
  echo "✓ served OAS == vendored ITS-REST bundles."
fi
```

The `api/index.html` `urls` array names all seven (EHR, Definition, Query,
Demographic, Admin, System, Overview), so the Swagger UI dropdown = the server's
API groups, verbatim.

### 5b. Link-check gate wiring

`lychee --no-progress --root-dir "$PWD/_site" './_site/**/*.html'` runs after the
site assembles (built HTML, so mdBook-internal + landing + api + external links
are all covered). A committed `.lycheeignore` excludes known-ephemeral hosts if
needed; external-link flakiness is bounded with `--max-retries 2 --timeout 20`.
`docs/specs/**` and other internal trees are never in `_site`, so they are out of
scope automatically.

### 5c. CLAUDE.md rule text (added at W1.5)

Add under **IMPORTANT hard rules**:

> - **User docs track the product.** Any PR that changes the REST surface,
>   configuration (`EHRBASE_*`), the CLI, deployment artifacts
>   (compose/Helm/containers), or other user-visible behaviour **must update the
>   matching `website/book/src` page in the same PR** (a docs analogue of the
>   changelog rule). The served OpenAPI is a byte copy of the vendored ITS-REST
>   bundles — never hand-edit `website/api/spec/**`; run `scripts/assemble-oas.sh`.

### 5d. `.claude/rules/docs-website.md` (outline, scoped `paths: ["website/**"]`)

1. Purpose + the layout (§2a) and URL scheme (§2b).
2. Authoring voice: end-user, task-first; the "never publish" list (§4).
3. The same-PR docs rule (5c) and which surfaces trigger it.
4. Toolchain pins (§1) — bump only by editing `docs.yml` env + this rule together.
5. Never hand-edit `website/api/spec/**` or vendored `swagger-ui` dist; use the
   scripts. Diagrams: mermaid fenced blocks. Callouts: native `> [!NOTE]` (no
   plugin). Long chapters: `<!-- toc -->`.
6. Local preview: `scripts/build-site.sh` then serve `_site/`.

### 5e. `/phase-done` skill — new checklist item

Add to the skill's exit checklist: **"User docs updated? — if this phase changed a
user-visible surface, the matching `website/book` page and (if REST changed) the
OAS copy were updated in-branch."**

### 5f. Tie into the existing release flow

The docs version cut chains off the **same `v*` tag** the existing
`release.yml`/`containers.yml` fire on, and off the same CHANGELOG-driven release
(`.claude/rules/changelog.md`): cutting a release already (a) renames
`[Unreleased]`→`[X.Y.Z]`, (b) bumps the workspace version, (c) tags `vX.Y.Z`. That
tag now *also* triggers `docs.yml`'s `version-cut` job → frozen `/docs/vX.Y.Z/` +
`versions.json` bump + `latest` re-point. No new human step; the tag is the single
trigger for release, containers, and docs alike.

---

## 6. Tasks

- [x] **W1.1 Scaffold `website/` + `docs.yml` (deploy dev).** _Tree scaffolded; dev-only build produces `_site/` with landing + `/docs/dev/` + `/api/`; mdBook 0.5.4 build clean; docs.yml YAML lints._
  - [x] `website/book/{book.toml, src/SUMMARY.md + chapter stubs, theme/custom.css, theme/version-picker.js}`; run `mdbook-mermaid install website/book` to vendor mermaid assets. _26 chapter stubs; mermaid assets vendored into theme/ (no CDN); book.toml uses only 0.5-valid keys._
  - [x] `website/landing/{index.html, style.css, 404.html, robots.txt, assets/}` with the §3 tokens. _Full landing (nav, hero, conformance proof strip 341/315/0 + CORE/STANDARD/OPTIONS pills, 8-card grid, quick-start, chips, footer); relative URLs only; light+dark._
  - [x] `scripts/build-site.sh` (`--dev-only` / `--full`) + `scripts/cut-version.sh`. _Both written + executable; --dev-only verified locally._
  - [x] `.github/workflows/docs.yml` per §2f (build + deploy `develop`). _deploy hardened to `needs: [build, version-cut]` with a `!cancelled()`/skipped-version-cut guard per task note._
  - **Acceptance:** `bash scripts/build-site.sh --dev-only` produces `_site/` with landing + `/docs/dev/` + `/api/` locally; `mdbook build website/book` is clean; workflow YAML lints. _All pass._
- [x] **W1.2 OpenAPI pipeline + drift gate.** _Swagger UI 5.32.8 vendored; api/index.html serves all 7 groups fully offline; assemble-oas.sh --check green on clean tree._
  - [x] Vendor Swagger UI **5.32.8** `dist/` into `website/api/vendor/swagger-ui/`; hand-write `api/index.html` with the 7-spec `urls` dropdown. _4 dist files copied from the v5.32.8 tarball; brand header bar + light-only note; relative asset refs only._
  - [x] `scripts/assemble-oas.sh` (+ `--check`); commit `website/api/spec/*.openapi.yaml`. _Written per §5a (identity-map rewritten as a portable word-list loop, behaviour identical); 7 `-html` bundles copied + committed._
  - [x] Wire the `--check` gate into `docs.yml`. _First step of the build job._
  - **Acceptance:** `/api/` renders all 7 groups offline (browser Network tab shows zero external requests); `scripts/assemble-oas.sh --check` passes on a clean tree. _Assets verified 100% local (curl); --check green._
- [x] **W1.3 Write the book.** *(2026-07-11 — 26 chapters, ~20,800 words, two parallel writers; every endpoint/env key code-verified)*
  - [x] All chapters in §4, sourced per the content map, end-user voice; mermaid architecture diagram; native `> [!NOTE]` callouts; `<!-- toc -->` in the L chapters.
  - [x] Config reference generated/curated from the `EHRBASE_*` keys (127 concrete keys documented, grep-only false positives excluded).
  - **Acceptance:** `mdbook-lint lint website/book/src` clean; lychee clean (loopback/localhost excluded — reader-side URLs); no page links into a "never publish" path (§4). ✓
- [x] **W1.4 Versioning machinery.** *(2026-07-11 — proven with the REAL v3.0.0 cut instead of a throwaway tag: docs-dist created, /docs/v3.0.0/ frozen, latest→v3.0.0)*
  - [x] `versions.json` + `version-picker.js` (dropdown + API link injected into the menu bar); `docs-dist` orphan branch created; `cut-version.sh`.
  - [x] Dry-run the tag path locally (build a fake `v0.0.0-docs-dryrun` into `docs-dist`, assemble `--full`, confirm `/docs/v0.0.0-docs-dryrun/` + `/docs/latest/` render and the picker lists them), then delete the dry-run entry.
  - **Acceptance:** picker switches versions; `latest` is a real deep-linkable copy; frozen version persists across a second deploy without rebuild.
- [x] **W1.5 Anti-drift wiring.** *(2026-07-11 — CLAUDE.md rule, .claude/rules/docs-website.md, /phase-done step 3b)*
  - [x] CLAUDE.md rule (5c); `.claude/rules/docs-website.md` (5d); `/phase-done` checklist item (5e).
  - **Acceptance:** the rule files exist and are scoped; `/phase-done` shows the new item.
- [x] **W1.6 Go live.** *(2026-07-11 — Pages live from PR #56; README links + Docs badge; CHANGELOG entry)*
  - [x] Enable Pages (source = GitHub Actions); first deploy from `develop`; README gains a "Documentation website" link to `https://rubentalstra.github.io/ehrbase-rs/` (+ CHANGELOG `[Unreleased]` entry — user-visible).
  - **Acceptance:** the three live URLs resolve (landing, `/docs/dev/`, `/api/`); README links the site.
- [x] **W1.7 Negative tests (prove the gates bite).** *(2026-07-11 — link gate bit in production on the first deploy [run fixed in PR #57]; OAS gate bit on a committed hand-edit, green after revert)*
  - [x] Inject a broken link into one chapter → lychee job fails; revert.
  - [x] Hand-edit one byte of `website/api/spec/ehr.openapi.yaml` → `assemble-oas.sh --check` fails; revert.
  - **Acceptance:** both gates demonstrably red on the injected breakage, green after revert (recorded in the phase Decisions/Handoff).

## Exit criteria

- [x] Site live: landing + `/docs/dev/` + `/api/` endpoint reference, all under `/ehrbase-rs/`, dark/light correct. *(all URLs probed 200, 2026-07-11)*
- [x] CI gates green in steady state and **red on injected breakage** (link check + OAS drift), negative-tested once then reverted (W1.7). *(link gate bit in production on the first deploy; OAS gate bit on a committed hand-edit)*
- [x] Version cut proven (W1.4 — with the real v3.0.0, not a dry-run tag); `latest` deep-linkable (`/docs/latest/querying-aql.html` 200); frozen `/docs/v3.0.0/` materialized from docs-dist without rebuild.
- [x] `/api/` fully offline — zero runtime CDN/network requests. *(verified by static audit of the built HTML — zero external script/css/img/font refs; the only externals are the canonical meta and a GitHub `<a>` nav link — plus local-server curl of every referenced asset; browser Network panel unavailable in this environment)*
- [x] Docs discipline installed: CLAUDE.md rule + `.claude/rules/docs-website.md` + `/phase-done` step 3b.
- [x] README points at the site; CHANGELOG `[Unreleased]` notes the docs site.

## Decisions made this phase

- **Build tool:** mdBook **0.5.4** (2026-07-06), Rust-only toolchain (owner ruling).
- **Callouts:** **mdBook 0.5 native admonitions** — `mdbook-admonish` (0.5-incompatible,
  issue #233) and `mdbook-alerts` (0.4-targeted, redundant) both **skipped**.
- **Diagrams:** `mdbook-mermaid` 0.17.0 (0.5-native via `mdbook-preprocessor 0.5.0`),
  assets vendored → no CDN.
- **Inline TOC:** `mdbook-toc` 0.15.4 (0.5-native). **Per-page nav:** mdBook 0.5
  built-in sidebar heading nav (`mdbook-pagetoc` skipped).
- **Markdown lint gate:** `mdbook-lint` 0.14.4 in standalone CLI mode.
- **Link checker:** **lychee** 0.24.2 (chosen) over `mdbook-linkcheck` 0.7.7
  (rejected — unmaintained since 2022, 0.5-incompatible, book-only scope).
- **Endpoint viewer:** **Swagger UI** 5.32.8 (chosen) over Redoc 2.5.3 (single-spec)
  and Scalar (no clean pin / Node dist) — native multi-spec `urls` dropdown, fully
  offline, cleanly pinnable, matches the in-app `/ehrbase/swagger-ui`.
- **Served OAS:** the 7 `-html` documentation bundles, copied + ITS-REST-version-stamped + drift-gated.
- **SEO:** sitemap/robots emitted in assembly (mdbook-sitemap skipped — cross-section
  scope); canonical = `/docs/latest/`; `dev`/old versions disallowed.
- **Version picker:** `additional-js` (not an `index.hbs` fork) — survives mdBook
  theme churn.
- **`latest` alias:** a real, self-consistent copy of the newest tag (deep links
  work; Pages has no server redirects), rebuilt each deploy.
- **Frozen versions:** stored on a `docs-dist` orphan branch (mike-style), never
  rebuilt; the deploy job re-materializes them into the Pages artifact.
- **GitHub Actions pins:** checkout@v7, configure-pages@v6, upload-pages-artifact@v5,
  deploy-pages@v5, setup-rust-toolchain@v1, install-action@v2 (all verified 2026-07-11).

## Handoff for next session

Closed 2026-07-11. The site is live at https://rubentalstra.github.io/ehrbase-rs/
— landing, versioned book (dev · latest · v3.0.0), and the offline OpenAPI
reference — with the drift discipline installed (same-PR docs rule, OAS byte-copy
gate, lychee link gate, both negative-tested). Frozen versions live on the
`docs-dist` orphan branch; every future `v*` tag cuts one automatically. Next:
the X1 comparison phase (plan drafted, awaiting owner review) and the blueprint
tail (P20 optimization, P99 cutover).

---

## 7. Sources appendix (every claim → tool · version · date · URL)

**Verified 2026-07-11.** Methods: `curl https://crates.io/api/v1/crates/<c>` (with
User-Agent), `gh release list --repo <org/repo>`, raw `Cargo.toml` fetch for
mdBook-0.5 compat, official CHANGELOG/issue pages.

| Claim | Value | Verified via |
|---|---|---|
| mdBook | 0.5.4 · 2026-07-06 | `gh release list rust-lang/mdBook` → v0.5.4 (Latest); crates.io max_stable=0.5.4 |
| mdBook 0.5 native admonitions + sidebar heading nav + breaking config keys | as §1a | docs.rs mdBook CHANGELOG (`https://docs.rs/crate/mdbook/latest/source/CHANGELOG.md`) |
| mdBook 0.5 preprocessor protocol break | confirmed | mdbook-admonish issue #233 (`https://github.com/tommilligan/mdbook-admonish/issues/233`) — open since 0.5.0; error string quoted |
| mdbook-mermaid | 0.17.0 · 2025-11-18 · 0.5-compat | crates.io; `Cargo.toml` `mdbook-preprocessor = "0.5.0"` (raw.githubusercontent badboy/mdbook-mermaid/main) |
| mdbook-toc | 0.15.4 · 2026-05-28 · 0.5-compat | crates.io; `Cargo.toml` `mdbook-preprocessor = "0.5.0"` |
| mdbook-lint | 0.14.4 · 2026-06-03 · standalone CLI | crates.io; README "Works as both an mdBook preprocessor and standalone CLI tool" (raw joshrotenberg/mdbook-lint/main/README.md) |
| mdbook-admonish (skip) | 1.20.0 · 2025-06-06 · `mdbook="0.4.40"` | crates.io; raw Cargo.toml; issue #233 |
| mdbook-alerts (skip) | 0.8.0 · 2025-06-29 · `mdbook="0.4.51"` | crates.io; raw Cargo.toml |
| mdbook-pagetoc (skip) | 0.3.0 · 2025-11-27 | crates.io |
| mdbook-sitemap (skip) | 0.1.2 · 2026-06-28 | crates.io; `gh api …/commits` |
| mdbook-sitemap-generator (skip) | 0.2.0 | crates.io |
| mdbook-katex (skip) | 0.9.4 (0.10.0-alpha 2025-11-28) | crates.io |
| mdbook-i18n-helpers (future) | 0.4.0 · 2025-11-27 | crates.io (google/mdbook-i18n-helpers) |
| mdbook-github-authors (skip) | 0.1.0 · 2025-02-06 | crates.io |
| mdbook-linkcheck (loser) | 0.7.7 · **2022-10-03** | crates.io |
| lychee (chosen) | 0.24.2 · 2026-05-01 | crates.io (lycheeverse/lychee) |
| Swagger UI (chosen) | 5.32.8 · 2026-06-23 | `gh release list swagger-api/swagger-ui` |
| Redoc (rejected) | 2.5.3 · 2026-05-29 | `gh release list Redocly/redoc` |
| Scalar (rejected) | release-2026-07-08 (rolling) | `gh release list scalar/scalar` |
| actions/checkout | v7.0.0 · 2026-06-18 | `gh release list actions/checkout` (repo already uses @v7) |
| actions/configure-pages | v6.0.0 · 2026-03-25 | `gh release list actions/configure-pages` |
| actions/upload-pages-artifact | v5.0.0 · 2026-04-10 | `gh release list`; `gh api …/tags` (major v5) |
| actions/deploy-pages | v5.0.0 · 2026-03-25 (tags v5/v4/v3/v2) | `gh api actions/deploy-pages/releases[0]`=v5.0.0; `/tags` |
| actions-rust-lang/setup-rust-toolchain | v1.17.0 · 2026-06-25 | `gh release list` (repo uses @v1) |
| taiki-e/install-action | 2.83.1 · 2026-07-10 | `gh release list` (repo uses @v2) |
| cargo-bins/cargo-binstall | 1.20.1 · 2026-06-21 | `gh release list` |
| ITS-REST OAS layout | 7 groups × 3 variants; `-html` = doc-render variant | `crates/openehr-its/vendor/rest-oas/PROVENANCE.md` + dir listing |
| logo colours | `#B7410E`, `#5A2408`, `#FFFFFF` | `assets/logo.svg` |
| conformance numbers | 341 executed · 315 passed · 0 failed; CORE/STANDARD PASS · OPTIONS OBTAINED | `docs/conformance/` + README + CHANGELOG |
| `EHRBASE_*` config surface | 119 keys | `grep -rohE 'EHRBASE_[A-Z0-9_]+' app docker docs \| sort -u \| wc -l` |
| content-source files exist | all 12 checked | on-disk existence check 2026-07-11 |
