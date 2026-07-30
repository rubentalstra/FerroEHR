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
- **A 1.4 upload is judged AS 1.4, never as an ADL2 superset.**
  `cadl::Dialect::Adl14` both ADDS the 1.4-only forms (qualified/listed term
  constraints, inline dADL domain blocks) and REMOVES the constructs ADL 2
  introduced — the cADL 1.4 keyword set is closed (`ADL1.4/master05-cadl.adoc`
  §Keywords). The 1.4-only validity rules (VCOC; VATDF/VACDF over the
  qualified/listed spelling) run on the `Dialect::Adl14` phase-1 path only.
  The `S*` error space is a verbatim 1:1 mirror of the openEHR catalogue
  (`ADL2/master04.6`): never invent a code — reuse the catalogue code for the
  parse position and name the construct in the message.
- **1.4 defaults are EFFECTIVE-value accessors, never mutations** —
  `validate::conformance::effective_{existence,occurrences}_adl14` apply
  master05's `{1..1}` defaults (and the `use_node` inheritance rule) on read;
  an absent value stays absent in the parsed AOM. Only the 1.4→2 CONVERTER
  writes a default out, because ADL 2 infers a different one.
- **ADL 1.4→2 conversion (`adl14/`) has NO spec basis** — the whole module is
  our own design (archie is prior art only), flagged as such; every heuristic is
  pinned by the paired `upgrade_from_14` corpus fixtures, not a spec clause.
  `ConvertConfig::collapse_specialised_codes` collapses dotted specialised codes
  to a depth-0 flat code space for a standalone flattened-OPT root (the
  differential lineage is unresolvable there); reused 1.4 node codes are
  re-minted archetype-wide-unique (VCOSU) with their terminology bindings cloned,
  and all remaps are recorded in `RESOURCE_DESCRIPTION.conversion_details`.
- **Conformance corpus:** `tests/corpus/` (the openEHR ADL2 regression library;
  file names encode the expected rule code) plus two HAND-WRITTEN ADL 1.4 trees
  the vendored ADL2 library cannot cover — `adl14-dadl/` (master04 dADL breadth)
  and `adl14-cadl/` (master05 cADL: dialect gates, domain lowering,
  VATDF/VACDF/STCDC/STCAC, VCOC, the operators the chapter names but no grammar
  defines — `~matches`/`~is_in`/`∉`, `=~`/`!~` — and the
  `cadl_breadth_{structure,primitives,datetime}` trio covering every construct
  of the chapter). `app/ehrbase/tests/adl14_knowledge_archetypes.rs` is the
  DB-free parse gate over the app's real-world CKM 1.4 knowledge resources. Corpus cases are the regression net — never delete/weaken
  one to get green; a defect goes through adjudication, not case edits, and
  every refusal keeps its accepting twin.
- Dependencies point downward only: `openehr-am`, `openehr-base`, `openehr-lang`
  (ODIN/BEL), `openehr-term`. No app-crate knowledge, no SQL, no REST.
- Versioned by the spec (ADL/AOM 2.4.0) — bumps only on a spec-pin bump.
- Gates: `cargo clippy -p openehr-adl --all-targets` +
  `cargo nextest run -p openehr-adl`.
