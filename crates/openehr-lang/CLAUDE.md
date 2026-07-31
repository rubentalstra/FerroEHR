# `openehr-lang` — BMM / P_BMM / ODIN (hand-written tooling layer)

The LANG component: the BMM object model + the `*.bmm.json` loader that
**feeds codegen**, and the hand-written ODIN reader (`src/odin/` — for
ADL/ODIN *instance* parsing — deliberately off the codegen path).

- **This crate is upstream of everything generated.** A change to the BMM
  loader can silently change what `openehr-codegen` emits across five
  crates — after ANY loader/model change, run `/regen-codegen` and inspect
  the diff; the `codegen-drift` CI job is the backstop.
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
  the honest boundaries (`value_constraint` has no `BMM_*` destination; a
  persisted `P_BMM_INTERFACE` has no `P_BMM_SCHEMA` slot) written out. This
  reader is NOT the codegen path either — codegen still consumes only the
  `.bmm.json` serialisation via `openehr-codegen`.
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
