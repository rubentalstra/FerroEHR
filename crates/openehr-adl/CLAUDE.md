# `openehr-adl` — ADL 2.4.0 engine (hand-written)

The ADL2 text + semantics engine: ADL2/cADL/ODIN parser (`logos` + `chumsky`,
like `openehr-query`; `lexer`, `parse/`, `source`), the AOM2 validation engine
(`validate/`, grouped by TOPIC — see below), the `master04.5` conformance
functions (`validate/conformance.rs`), specialisation flattening (`flatten.rs`),
OPT2 generation (`opt.rs` — raw via `create_opt`, profiled via `profile_opt`),
the ADL printer (`printer`), and ADL 1.4→2 conversion (`adl14/`). Builds directly
into the generated `openehr_am::am24::aom2` model — never re-model AOM2.

- **The cADL parser is `parse/`, one module per production family** — `mod`
  (the `Dialect` selector, the `Parser` state, the entry points
  `parse_definition_body`/`parse_definition_body_adl14`, the re-entrant
  sub-parsers `crate::rules` drives, and the cursor/error helpers), `parser`
  (structure / attribute / tuple productions), `refs` (archetype slots,
  `use_archetype` roots, `use_node` proxies), `primitives` (the inline
  `C_PRIMITIVE` family), `values` (value lists, `|…|` intervals, endpoints,
  the `CadlValue` kind trait), `patterns` (the date/time constraint-pattern
  validators). **The ADL 1.4-only productions do NOT live here**: they are
  `adl14/lower.rs` (with the inline dADL domain lowering in
  `adl14/domain.rs`), and the three dialect-gated dispatch points in
  `parse::parser` are the only coupling — keep it that way.
- **`validate/` is grouped by TOPIC, never by phase number.** `mod` is
  orchestration ONLY — `ValidationIssue`, the six public entry points, the
  `push_issue` helper every walker builds its findings with, and the three
  phase drivers (`run_phase1`, `run_phase2_spec`, `run_phase3`) whose call
  order + error gating ARE the `master08` phase schedule. Everything else is a
  topic module: `catalogue` (the `Severity` + `ValidationCode` vocabulary, 91
  codes), `identification` (id/root/versions/languages + the STCNT/VOLT gate),
  `structure` (the phase-1 definition walk), `terminology` (term definitions,
  value sets, code usage), `bindings` (binding keys + the
  `TerminologyResolver` seam for VETDF), `annotations` (`annotations` +
  `rm_overlay`), `source_level` (VOKU/VRRLP over the raw parsed source),
  `specialisation` (the differential-vs-flat-parent walk + the parent-dependent
  VACSD/VASID/VALC), `slots` (the slot arm of that walk — a second `impl
  Phase2` block — plus `validate_fillers`), `rm` (the reference-model seam),
  `conformance` (the `master04.5` functions), `flat` (phase 3 + the deferred
  flat-form halves). A new rule goes in the topic module that owns its subject,
  and its driver call keeps the existing order — issue emission order is
  behaviour.
- **The shared substrate has ONE home each — never re-inline a copy.** `aom/`
  (`access` = the 13-arm `C_OBJECT` field accessors + `AomType`, `build` = the
  AOM2 constructors, `interval` = `Bounds`/multiplicity + `INTERVAL<T>` maths),
  `artefact` (`ArchetypeView`/`view` + `ArchetypeRepository`/`FlatParent`),
  `hrid` (parse/print/lookup-key of `ARCHETYPE_HRID`), `odin` (the ODIN reading
  bridge + the `master03` escape decoding + delimited-regex handling), and
  `paths::child_path`. They sit BELOW `validate`/`flatten`/`opt`/`printer`, which
  all read through them. Two pairs are deliberately kept divergent and
  co-located with `// TODO:`s naming their issues: the interval point-of
  extractors (`aom::interval`, #1339) and the escape decoding's 4-digit-only
  `\uHHHH` handling (`odin`, #1340).
- **Spec oracle:** `docs/specs/openehr/AM/docs/{ADL2,AOM2,OPT2}/` +
  `LANG/docs/odin/` (`/spec-lookup`). Every validation rule cites its code + spec
  file/section; master08-only codes and all other spec-silences carry the
  explicit `// NOTE:` flag.
- **No ANTLR runtime, ever.** The normative `.g4` grammars from
  `openEHR/adl-antlr` are vendored under `vendor/grammar/` (with PROVENANCE.md)
  as reference input only.
- **A 1.4 upload is judged AS 1.4, never as an ADL2 superset.**
  `parse::Dialect::Adl14` both ADDS the 1.4-only forms (qualified/listed term
  constraints, inline dADL domain blocks) and REMOVES the constructs ADL 2
  introduced — the cADL 1.4 keyword set is closed (`ADL1.4/master05-cadl.adoc`
  §Keywords). The dialect reaches the OUTER structure too
  (`source::parse_source_adl14`, the entry every 1.4 caller uses): 1.4 section
  keywords are case-insensitive (`master08-adl.adoc` §Symbols), an old-form
  archetype with `primary_language`/`languages_available` in its ontology and
  no `language` section is accepted and upgraded on parse (§Language Section +
  §Ontology Header Statements), and the `concept` section is mandatory
  (§Syntax Specification `arch_concept`; VARCN). ADL2 outer parsing is
  unchanged — exact lowercase keywords, unconditional `SALAN`, no concept
  clause. The 1.4-only validity rules (VCOC; VATDF/VACDF over the
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
