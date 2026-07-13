# `openehr-flat` — FLAT / STRUCTURED / TDD (hand-written)

The simplified-data-format crate: web-template FLAT + STRUCTURED
(Better `web-template` semantics, P14) and the TDD → COMPOSITION converter
(`from_tdd`, corpus-verified, B3).

- Semantics follow the SM Simplified Data Template / SIM-B spec
  (`docs/specs/openehr/SM/docs/serial_data_formats/`,
  `.../simplified_im_b/`) with Better-compatibility behaviour where the
  ecosystem expects it; Better-specific quirks stay behind a feature flag
  and divergences from SDF-normative encodings carry a `// PORT NOTE:`
  with the citation (the P17 interop audit reconciles them).
- Path/key encoding (`a/b:0/c|unit`) is load-bearing wire surface — no
  ad-hoc changes; every accepted/emitted form needs a corpus round-trip
  test.
- Consumes `openehr-rm`/`openehr-am` types directly; WebTemplate building
  lives in the application — this crate is format conversion only.
- Follows the product version (not a spec pin — the FLAT formats have no
  versioned openEHR spec of their own; flag this "our own design +
  Better/SDF prior art" where relevant).
- Gates: `cargo clippy -p openehr-flat --all-targets` +
  `cargo nextest run -p openehr-flat` (corpus tests included).
