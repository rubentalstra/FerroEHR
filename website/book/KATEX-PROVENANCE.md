# KaTeX assets (vendored)

`katex.min.css` and `fonts/*.woff2` are the KaTeX **0.16.4** distribution,
taken verbatim from the npm package tarball
(`https://registry.npmjs.org/katex/-/katex-0.16.4.tgz`, `dist/`).
License: MIT (KaTeX's upstream license; declared in `REUSE.toml`).

The version is load-bearing: math is pre-rendered at build time by
`mdbook-katex`, whose bundled renderer (the `katex` Rust crate, its
`KATEX-VERSION` file) is KaTeX 0.16.4 — the emitted markup and this
stylesheet must come from the same KaTeX release. Bump both together.

Only the `.woff2` fonts are vendored (every supported browser takes the
first `woff2` source in the CSS `src()` lists; the `woff`/`ttf` fallbacks
are never requested). The CSS is unmodified.

The assets live at `src/katex/` (static files mdBook copies verbatim —
not book content, so this note lives outside `src/`). The stylesheet is
linked from `theme/head.hbs`.
