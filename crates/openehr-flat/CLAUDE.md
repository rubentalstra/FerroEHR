# `openehr-flat` — Simplified Formats: FLAT / STRUCTURED / Web Template / TDD (hand-written)

The Simplified Formats crate: FLAT + STRUCTURED data instances, the Web
Template model, and the TDD → COMPOSITION converter (`tdd::from_tdd`,
corpus-verified).

- **The wire oracle is the ITS-REST Simplified Formats specification**
  (`docs/specs/openehr/ITS-REST/docs/simplified_formats/`, STABLE):
  `master04` (field identifiers, node-id algorithm, level removal, `|raw`,
  `|other`, FLAT⇄STRUCTURED algorithms), `master05` (per-RM-type mapping
  tables), `master06` (the `ctx/` vocabulary). SM SIM-B / SDF are
  DEVELOPMENT-state model documents — never implement their terse string
  encodings; SDT is retired. No vendor implementation is an oracle.
- **Architecture: one internal tree** (`sim::SimNode`) — FLAT
  (`sim::flat`) and STRUCTURED (`sim::structured`) are pure codecs over
  it; the template-driven RM conversion is written once (`flatten.rs` RM→sim,
  `build.rs` sim→RM, entry points in `convert.rs`). Datum codecs from the
  `master05` tables live in `map/`; the `ctx/` vocabulary in `ctx.rs`; the
  Web Template model/builder in `webtemplate/` (node ids per `master04
  §Node ID Generation Rules`; the document shape serves
  `application/openehr.wt+json`).
- Path/key encoding (`a/b:0/c|unit`) is load-bearing wire surface — no
  ad-hoc changes; every accepted/emitted form needs a spec citation and a
  round-trip test. Spec-example JSON blocks are the primary test vectors;
  the OPT corpus is regression.
- Consumes `openehr-rm`/`openehr-am` types directly (canonical JSON with
  `_type` tagging); never re-models the RM.
- Follows the product version (the Simplified Formats spec is part of
  ITS-REST development; no separate spec pin).
- Gates: `cargo clippy -p openehr-flat --all-targets` +
  `cargo nextest run -p openehr-flat` (spec vectors + corpus included).
