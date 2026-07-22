# `openehr-adl` — ADL 2.4.0 engine (hand-written)

The ADL2 text + semantics engine: ADL2/cADL/ODIN parser (`logos` + `chumsky`,
like `openehr-query`; `lexer`, `cadl`, `source`), the AOM2 validation catalogue
(`validate/` phases 1–3 + RM + terminology + flat), the `master04.5` conformance
functions (`validate/conformance.rs`), specialisation flattening (`flatten.rs`),
OPT2 generation (`opt.rs` — raw via `create_opt`, profiled via `profile_opt`),
the ADL printer (`printer`), and ADL 1.4→2 conversion (`adl14/`). Builds directly
into the generated `openehr_am::am24::aom2` model — never re-model AOM2.

- **Spec oracle:** `docs/specs/openehr/AM/docs/{ADL2,AOM2,OPT2}/` +
  `LANG/docs/odin/` (`/spec-lookup`). Every validation rule cites its code + spec
  file/section; master08-only codes and all other spec-silences carry the
  explicit `// NOTE:` flag.
- **No ANTLR runtime, ever.** The normative `.g4` grammars from
  `openEHR/adl-antlr` are vendored under `vendor/grammar/` (with PROVENANCE.md)
  as reference input only.
- **ADL 1.4→2 conversion (`adl14/`) has NO spec basis** — the whole module is
  our own design (archie is prior art only), flagged as such; every heuristic is
  pinned by the paired `upgrade_from_14` corpus fixtures, not a spec clause.
  `ConvertConfig::collapse_specialised_codes` collapses dotted specialised codes
  to a depth-0 flat code space for a standalone flattened-OPT root (the
  differential lineage is unresolvable there); reused 1.4 node codes are
  re-minted archetype-wide-unique (VCOSU) with their terminology bindings cloned,
  and all remaps are recorded in `RESOURCE_DESCRIPTION.conversion_details`.
- **Conformance corpus:** `tests/corpus/` (the openEHR ADL2 regression library;
  file names encode the expected rule code). Corpus cases are the regression net
  — never delete/weaken one to get green; a defect goes through adjudication, not
  case edits.
- Dependencies point downward only: `openehr-am`, `openehr-base`, `openehr-lang`
  (ODIN/BEL), `openehr-term`. No app-crate knowledge, no SQL, no REST.
- Versioned by the spec (ADL/AOM 2.4.0) — bumps only on a spec-pin bump.
- Gates: `cargo clippy -p openehr-adl --all-targets` +
  `cargo nextest run -p openehr-adl`.
