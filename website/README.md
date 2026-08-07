# FerroEHR documentation website

This directory holds the public documentation site published to
**https://ferroehr.eu/** (GitHub Pages, custom apex domain — the site is
served at the domain root). The site conventions — toolchain pins, URL scheme,
versioning machinery, design tokens, content map — are documented in
`.claude/rules/docs-website.md` and this file.

## Layout

| Path | What |
|---|---|
| `landing/` | Hand-written static landing page (`index.html` + `style.css`, `404.html`, `robots.txt`, `assets/`). Relative URLs only. |
| `book/` | The mdBook (`book.toml`, `src/`, additive `theme/`). Built per deployment path with the matching `site-url`. |
| `api/` | Static OpenAPI endpoint reference: vendored Swagger UI 5.32.8 + the 7 served OAS bundles (`spec/`). |
| `versions.json` | Version manifest the picker and deploy job read. |

## Building locally

The build needs a Rust-only toolchain: `mdbook`, `mdbook-mermaid`, `mdbook-toc`
(pins in `docs.yml` / the phase file §1). Then:

```shell
bash scripts/site/assemble-oas.sh          # copy the 7 vendored OAS bundles into api/spec/
bash scripts/site/build.sh --dev-only # assemble ./_site (landing + /docs/dev/ + /api/)
# serve _site/ with any static server, e.g.:  npx --yes serve _site
```

`--full` additionally materializes the frozen versions from the `docs-dist`
branch plus `/docs/latest/`, exactly as CI does before deploy.

## Rules

- Never hand-edit `api/spec/**` or the vendored `api/vendor/swagger-ui/**` —
  run `scripts/site/assemble-oas.sh` (the served OAS is a byte copy of the vendored
  ITS-REST bundles, drift-gated in CI).
- Authoring voice is end-user and task-first; never publish internal trees
  (plans, rules, the vendored spec oracle).
