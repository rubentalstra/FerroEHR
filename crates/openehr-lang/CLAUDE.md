# `openehr-lang` — BMM / P_BMM / ODIN (hand-written tooling layer)

The LANG component: the BMM object model + the `*.bmm.json` loader that
**feeds codegen**, and the hand-written ODIN reader (`src/odin/` — for
ADL/ODIN *instance* parsing — deliberately off the codegen path).

- **This crate is upstream of everything generated.** A change to the BMM
  loader can silently change what `openehr-codegen` emits across five
  crates — after ANY loader/model change, run `/regen-codegen` and inspect
  the diff; the `codegen-drift` CI job is the backstop.
- Codegen consumes the JSON BMM serialization only
  (`crates/openehr-codegen/vendor/bmm/`); the ODIN reader exists for
  ADL/ODIN instance text, not for loading meta-models.
- **`src/odin/` is the real ODIN reader** (a self-contained `logos` lexer +
  `chumsky` parser + `OdinValue` tree; `openehr_lang::odin::parse`), NOT part
  of the generated/`bmm*`/`beom` model — never route it through codegen.
  Spec oracle: `docs/specs/openehr/LANG/docs/odin/` + the vendored grammars
  `vendor/grammar/{odin.g4,odin_values.g4,base_lexer.g4}`. `openehr-adl`
  consumes it to parse ADL2 ODIN sections. Do not touch
  `bmm`/`bmm3`/`beom`/`bmm_persistence` when editing `odin`.
- Spec authority: `docs/specs/openehr/LANG/docs/` (bmm, odin). Parser
  behaviour divergences are spec-citable, never silent.
- Versioned LANG 1.0.0 (spec pin).
- Gates: `cargo clippy -p openehr-lang --all-targets` +
  `cargo nextest run -p openehr-lang`, plus a drift check when the model
  changed.
