# `openehr-lang` — BMM / P_BMM / ODIN (hand-written tooling layer)

The LANG component: the BMM object model + the `*.bmm.json` loader that
**feeds codegen**, and the hand-written ODIN reader (`src/odin/` — for
ADL/ODIN *instance* parsing — deliberately off the codegen path).

- **This crate is upstream of everything generated.** A change to the BMM
  loader can silently change what `openehr-codegen` emits across five
  crates — after ANY loader/model change, run `/regen-codegen` and inspect
  the diff; the `codegen-drift` CI job is the backstop.
- **TWO BMM GENERATIONS, emitted side by side — never merge them.** openEHR
  publishes the stable **v2.x** BMM (`LANG/docs/bmm/`, which
  `master01-preface.adoc` §History calls "the normative, tool-implemented
  version", plus its `master06-persistence.adoc` P_BMM form) and the **v3**
  development line (`LANG/docs/bmm3/`, which adds the expression/statement
  meta-model); SPECLANG-14 formalised the split
  (`LANG/docs/bmm3/master00-amendment_record.adoc`). Both are vendored as
  separate `*.bmm.json` files and BOTH are emitted COMPLETELY:
  - v2.x → `src/bmm/core/`, `src/bmm/rm_access/`, `src/bmm_persistence/`,
    `src/beom/`;
  - v3 → `src/bmm3/**`.
  18 class names occur in both files with materially different shapes
  (`BMM_CLASS`, `BMM_TYPE`, `BMM_PROPERTY`, `BMM_MODEL`, …), so each name
  yields TWO Rust types at two paths, each with its own intra-generation
  cross-references. The crate **prelude carries one entry per Rust name** —
  the v3 twin for a colliding name; the v2.x twin is reachable by its full
  module path only, which is how the hand-written v2 surface below imports it.
  Never "unify" the two, never reach across generations in a type position,
  and never add a shadow/adapter type: a shape gap is a
  `tools/openehr-codegen` fix + regeneration.
- **BOTH generations now carry a hand-written spec-function surface, and they
  never share code.** v2.x: `src/bmm/core/*_impl.rs` (`BMM_CLASSIFIER`,
  `BMM_CLASS`, `BMM_TYPE`, `BMM_PROPERTY`, `BMM_MODEL`, `BMM_PACKAGE`,
  `BMM_GENERIC_PARAMETER`, `BMM_OPEN_TYPE`) — the generation `rm_access`
  publishes. v3: `src/bmm3/core/entity/bmm_type_impl.rs` (the type naming trio +
  flattening + the meta-type lattice: `is_abstract`/`is_primitive`/
  `type_base_name`/`unitary_type`/`effective_type`/`effective_base_class`/
  `is_open`/`is_closed`/`is_partially_closed`, plus the `From` impls that ARE the
  `BMM_TYPE ⊃ BMM_UNITARY_TYPE ⊃ BMM_EFFECTIVE_TYPE ⊃ BMM_MODEL_TYPE` lattice),
  `src/bmm3/core/entity/bmm_class_impl.rs` (class attributes, `type()` generators,
  `generic_parameter_conformance_type`, `has_ancestor_class`, `all_ancestors`,
  `flat_features`, `BMM_ENUMERATION.name_map`),
  `src/bmm3/core/feature/bmm_feature_impl.rs` (`signature()`/`arity()`/
  `is_boolean()`), `src/bmm3/core/literal_value/bmm_literal_value_impl.rs`
  (`value_literal`/`syntax` + the literal-evaluation boundary), and
  `src/bmm3/core/model/bmm_model_impl.rs` — the MODEL-level navigation the type
  lattice is the precondition for (`class_definition`, `all_ancestor_classes`,
  `property_definition` over the model-flattened property set, and
  `type_conforms_to` per `LANG/docs/bmm3/master06-core-types.adoc` §Type
  Conformance, whose Tuple and Signature branches are empty upstream and are
  therefore not realized). When a v2/v3
  boundary is recorded, name WHICH generation it belongs to — attributing a
  generation-specific gap to "the openEHR specs" is a misattribution the citation
  rule exists to prevent.
- **`src/bmm3/expression/` and `src/bmm3/statement/` are DELIBERATELY inert (an
  adjudicated boundary, not an oversight).** All 33 expression classes and the
  statement package are emitted completely, with a canonical-JSON codec, and carry
  **no** behaviour: none of `eval_type()`, `reference()`, `is_callable()`,
  `operator_definition()`, `equivalent_call()` exists, none of the 9 declared
  invariants is enforced, and no `*_impl.rs` should be added speculatively. Why:
  the Expression Language is DEVELOPMENT status with no vendored grammar
  (`LANG/docs/EL/masterAppA-syntax.adoc` points at an external ANTLR repo — the
  same reason `src/lexer/` excludes EL), and the statement package is optional by
  the spec's own words (`LANG/docs/bmm3/master12-statements.adoc` §Overview: "This
  facility is not needed for achieving the original purpose of BMM"). The one
  consequence to know: `BMM_ASSERTION.expression` is a `1..1`
  `EL_BOOLEAN_EXPRESSION`, so the P_BMM→v3 materialisation cannot land class
  invariants or routine pre/post-conditions (they stay opaque tagged strings in the
  P_BMM graph); `create_bmm3_model.rs` states that skip in its module docs rather
  than inventing an expression tree, and the EL grammar must be VENDORED before
  the gap can close.
  `beom` is a DIFFERENT spec's object model (BEL, STABLE) and is never wired into
  `BMM_ASSERTION`; that would be a category error.
- **Two abstract, attribute-free v3 classes emit as instantiable empty structs** —
  `BMM_VISIBILITY` and `BMM_FEATURE_EXTENSION` (`bmm3/core/feature/`). Upstream
  declares them abstract with zero attributes and zero descendants, and marks the
  visibility meta-model unfinished (`LANG/docs/bmm3/master08-core-features.adoc`
  §Feature Groups and Visibility: "TBD: define visibility meta-model"), so the
  emitter has no better shape; the adjudication is recorded at the decision site
  (`tools/openehr-codegen/src/plan/mod.rs`, the abstract-with-no-descendants
  branch). Do not "fix" this in the crate — a declared subtype set upstream turns
  each into an untagged enum automatically.
- Codegen consumes the JSON BMM serialization only
  (`tools/openehr-codegen/vendor/bmm/`); the ODIN reader exists for
  ADL/ODIN instance text, not for loading meta-models.
- **`src/lexer/` is the ONE lexer in the workspace** for the ADL/ODIN/BEL
  family — one `logos` token superset (`lexer::Token`/`Spanned`, the union of
  `base_lexer.g4` + `adl_keywords.g4` + `odin.g4`/`odin_values.g4` +
  `base_expressions.g4`) plus three thin entry points `lex_adl` / `lex_odin` /
  `lex_bel` that apply a per-language **reclassification pass** over the shared
  stream. Never add a second lexer, here or in a consuming crate; a language's
  lexical difference is a rule in `lexer/reclassify.rs`, never a new DFA.
  - Keyword variants stay UNIT variants: a language that does not reserve a
    word gets the identifier variant back, re-tagged from the source slice at
    the token's span. `LANG/docs/odin/master03-basics.adoc` §Keywords ("ODIN
    has no keywords of its own") is why the ODIN pass demotes every cADL/BEL
    keyword; `AM/docs/ADL2/master07.04` (section keywords "can safely appear
    as identifiers") is why section words are not tokens at all.
  - Where a language's longest match is SHORTER than the union's, the pass
    narrows by retrying shorter prefixes — the union always matches at least
    as far as any member, so no member ever needs a merge.
  - The Expression Language (`LANG/docs/EL/`) is deliberately OUT of the
    union (`#`-codes, a different bracket algebra, `|`-comments; DEVELOPMENT
    status, no vendored grammar).
  - The three readings are pinned token-, span- and payload-for-payload by
    `tests/it/lexer_equivalence.rs` against fixtures captured from the three
    pre-unification lexers. **Editing a fixture line changes an accepted or
    refused lexical surface** and needs an adjudicated, spec-cited reason.
- **`src/bmm_persistence/` is generated types + a hand-written P_BMM SCHEMA
  READER.** The `P_BMM_*` type files are generated; beside them live the
  hand-written pipeline `master02-overview.adoc` §Conceptual Approach
  prescribes — `reader.rs` (ODIN text → `P_BMM_SCHEMA`, a STRICT read: an
  attribute the class docs do not declare is a typed error, with two
  spec-cited tolerance lists), `include_resolution.rs` (transitive inclusion
  merge, includer wins, collisions marked `is_override`), `create_model.rs`
  (`P_BMM` → **v2.x** `BMM_MODEL`, all name references resolved to object
  references), `create_bmm3_model.rs` (`P_BMM` → **v3** `BMM_MODEL` — its own
  module precisely because the two generations give the same Rust NAMES to
  different types and import renaming is forbidden), `loader.rs` (the composed
  `load_model`), `error.rs` (the one typed `PBmmReadError`), plus
  `p_bmm_*_impl.rs` spec functions. Class-name
  resolution is case-insensitive (`master04-syntax.adoc` §Non-primitive
  Classes: `name` — "any capitalisation can be used"); embedding depth and
  every other cycle/boundary decision is adjudicated in the module docs, with
  the boundaries written out — and each one names the GENERATION it belongs to:
  `value_constraint`, P_BMM `functions`/`constants` and a generic ancestor's
  parameter binding have no destination in the **v2.x** `BMM_*` model
  `create_model.rs` materialises (P_BMM is the v2.x persistence form,
  `LANG/docs/bmm/master06-persistence.adoc`), while the **v3** classes DO declare
  all three (`org.openehr.lang.bmm3.bmm_class.adoc` §Attributes,
  `…bmm3.bmm_model_type.adoc` §Attributes) — and `create_bmm3_model.rs` lands
  them, together with the generic-substituted properties
  `LANG/docs/bmm3/master13-model_semantics.adoc` §Generic Inheritance requires
  (an ancestor's formal parameter replaced by the descendant's binding, stamped
  `is_synthesised_generic`; a declared property is never displaced, and the
  ancestor is rebuilt rather than read off its embedded stub, which is what
  propagates a substitution down a partially-closed chain). The one thing NO
  generation can land is a class invariant or routine pre/post-condition,
  because that needs an EL parse (see the inert expression/statement bullet
  above). Both transforms share one index +
  enumeration-validity check (`Builder`, `check_enumeration_validity`, module-
  private via `pub(super)`), so they can never disagree about what a class or a
  valid enumeration IS. A persisted `P_BMM_INTERFACE` IS a class-list
  member (`master02-overview.adoc` §Conceptual Approach + openEHR's own
  published BASE/RM schemas): the emitter re-opens the `P_BMM_CLASS` subtype set
  for it (`plan/overrides.rs` `SUBTYPE_EXTENSIONS`), the reader materialises its
  name/documentation/functions, and `create_model.rs` projects it as an abstract
  `BMM_CLASS`. This reader is NOT the codegen path either — codegen still
  consumes only the `.bmm.json` serialisation via `openehr-codegen`.
- **`src/bmm/rm_access/` is the schema-REPOSITORY facade over that reader**
  (`LANG/docs/bmm/master04-rm_access.adoc`): `*_impl.rs` behaviour on the
  generated `REFERENCE_MODEL_ACCESS` / `SCHEMA_DESCRIPTOR` data classes plus a
  typed `error.rs`. It is the ONLY module in this crate that touches the
  filesystem (the spec puts `schema_directories` on the class), and it adds
  nothing to the pipeline semantics: directory scan → load-list closure →
  descriptor lifecycle (`load` → `validate` + `validate_includes` →
  `create_schema`) → `valid_models`. Zero-argument spec signatures that need the
  candidate set (`is_top_level`, `create_schema`) take it as a parameter, each
  with the adjudication at the site — never a synthetic `meta_data` key or a
  shadow field.
- **`src/odin/` is the real ODIN reader** (a `chumsky` parser over
  `lexer::lex_odin` + an `OdinValue` tree; `openehr_lang::odin::parse`), NOT
  part of the generated/`bmm*`/`beom` model — never route it through codegen.
  Spec oracle: `docs/specs/openehr/LANG/docs/odin/` + the vendored grammars
  `vendor/grammar/{odin.g4,odin_values.g4,base_lexer.g4}`. `openehr-adl`
  consumes it to parse ADL2 ODIN sections. Do not touch
  `bmm`/`bmm3`/`beom`/`bmm_persistence` when editing `odin`.
- **`src/escape.rs` is the ONE home for `master03` string-escape semantics**
  (`LANG/docs/odin/master03-basics.adoc` §File Encoding + §Special Character
  Sequences, verbatim in `AM/docs/ADL2/master03-file_encoding.adoc`): the six
  quoted forms plus BOTH `\uHHHH` and `\uHHHHHHHH`, and NOTHING else — the set
  is closed ("Any other character combination starting with a backslash is
  illegal"), so an unknown sequence, an unpaired trailing backslash, a `\u`
  with the wrong digit count, and a `\u` that denotes no character are each a
  typed `EscapeError`, never pass-through text. The shared lexer's
  `STRING`/`CHARACTER` callbacks call `escape::validate` beside their
  structural escape scan (so a token never carries an undecodable escape,
  which is what lets the ODIN and BEL parsers decode infallibly), and
  `openehr-adl`'s cADL parser calls `escape::decode` and reports a defect at
  the literal's span. Never re-implement escape decoding anywhere.
- Spec authority: `docs/specs/openehr/LANG/docs/` (bmm, odin). Parser
  behaviour divergences are spec-citable, never silent.
- Versioned LANG 1.0.0 (spec pin).
- Gates: `cargo clippy -p openehr-lang --all-targets` +
  `cargo nextest run -p openehr-lang`, plus a drift check when the model
  changed.

- `src/json_serde.rs` is GENERATED by `emit-json` — the canonical-JSON
  `serde::Serialize`/`Deserialize` impls for this crate's types, over the shared
  hand-written runtime `openehr_base::serde_support`. Never hand-edit it; change
  `tools/openehr-codegen/src/render/emit_json.rs` and regenerate.
