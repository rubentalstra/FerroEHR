# `openehr-adl` — ADL 2.4.0 engine (hand-written)

The ADL2 text + semantics engine: ADL2/cADL/ODIN parser (`logos` +
`chumsky`, like `openehr-query`), the complete AOM2 validation catalogue
(S-codes + V-codes, phases 1–3), the master04.5 conformance functions,
specialisation flattening, OPT2 generation (raw + profiled), the ADL
printer, and ADL 1.4→2 conversion. Builds directly into the generated
`openehr_am::am24::aom2` model — never re-model AOM2.

- **Spec oracle:** `docs/specs/openehr/AM/docs/{ADL2,AOM2,OPT2}/` +
  `LANG/docs/odin/` (`/spec-lookup`). Every validation rule cites its code
  + spec file/section; the 14 master08-only codes and all other
  spec-silences carry the explicit `// NOTE:` flag.
- **No ANTLR runtime, ever.** The normative `.g4` grammars from
  `openEHR/adl-antlr` are vendored under `vendor/grammar/` (with
  PROVENANCE.md) as reference input only.
- **Conformance corpus:** `tests/corpus/` (the openEHR ADL2 regression
  library; file names encode the expected rule code). Corpus cases are
  the regression net — never delete/weaken one to get green; a defect goes
  through adjudication, not case edits.
- ADL 1.4→2 conversion has NO spec basis — the whole `adl14` module is
  our own design (archie is prior art only), flagged as such.
- Dependencies point downward only: `openehr-am`, `openehr-base`,
  `openehr-lang` (ODIN/BEL), `openehr-term`. No app-crate knowledge, no
  SQL, no REST.
- Versioned by the spec (ADL/AOM 2.4.0) — bumps only on a spec-pin bump.
- Gates: `cargo clippy -p openehr-adl --all-targets` +
  `cargo nextest run -p openehr-adl`.
