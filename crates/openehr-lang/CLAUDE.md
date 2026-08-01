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
- **The hand-written spec-function surface is the v2.x one** (`src/bmm/core/*_impl.rs`:
  `BMM_CLASSIFIER`, `BMM_CLASS`, `BMM_TYPE`, `BMM_PROPERTY`, `BMM_MODEL`,
  `BMM_PACKAGE`, `BMM_GENERIC_PARAMETER`, `BMM_OPEN_TYPE`), because P_BMM and
  `rm_access` materialise the v2.x model. The v3 tree carries only its own
  type-naming surface (`src/bmm3/core/entity/bmm_type_impl.rs`); the rest of the
  v3 function surface, its invariants, and a v3 materialisation from P_BMM are
  unimplemented and marked `TODO` at their sites. When a v2/v3 boundary is
  recorded, name WHICH generation it belongs to — attributing a
  generation-specific gap to "the openEHR specs" is a misattribution the
  citation rule exists to prevent.
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
  (`P_BMM` → `BMM_MODEL`, all name references resolved to object references),
  `loader.rs` (the composed `load_model`), `error.rs` (the one typed
  `PBmmReadError`), plus `p_bmm_*_impl.rs` spec functions. Class-name
  resolution is case-insensitive (`master04-syntax.adoc` §Non-primitive
  Classes: `name` — "any capitalisation can be used"); embedding depth and
  every other cycle/boundary decision is adjudicated in the module docs, with
  the boundaries written out — and each one names the GENERATION it belongs to:
  `value_constraint`, P_BMM `functions`/`invariants` and a generic ancestor's
  parameter binding have no destination in the **v2.x** `BMM_*` model this
  materialises (P_BMM is the v2.x persistence form,
  `LANG/docs/bmm/master06-persistence.adoc`), while the **v3** classes DO
  declare all three (`org.openehr.lang.bmm3.bmm_class.adoc` §Attributes,
  `…bmm3.bmm_model_type.adoc` §Attributes), so a v3 materialisation is the
  recorded `TODO`. A persisted `P_BMM_INTERFACE` IS a class-list
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
