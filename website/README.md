# EHRbase-rs documentation website

This directory holds the public documentation site published to
**https://rubentalstra.github.io/ehrbase-rs/** (GitHub Pages, project sub-path
`/ehrbase-rs/`). The authoritative specification for the whole site — toolchain
pins, URL scheme, versioning machinery, design tokens, content map — is the
phase file [`docs/plans/w1-docs-website.md`](../docs/plans/w1-docs-website.md).

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
bash scripts/assemble-oas.sh          # copy the 7 vendored OAS bundles into api/spec/
bash scripts/build-site.sh --dev-only # assemble ./_site (landing + /docs/dev/ + /api/)
# serve _site/ with any static server, e.g.:  python3 -m http.server -d _site
```

`--full` additionally materializes the frozen versions from the `docs-dist`
branch plus `/docs/latest/`, exactly as CI does before deploy.

## Rules

- Never hand-edit `api/spec/**` or the vendored `api/vendor/swagger-ui/**` —
  run `scripts/assemble-oas.sh` (the served OAS is a byte copy of the vendored
  ITS-REST bundles, drift-gated in CI).
- Authoring voice is end-user and task-first; never publish internal trees
  (ADRs, plans, blueprint, spec oracle) — see the phase file §4.
