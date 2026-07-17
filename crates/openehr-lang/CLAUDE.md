# `openehr-lang` — BMM / P_BMM / ODIN (hand-written tooling layer)

The LANG component: the BMM object model + the `*.bmm.json` loader that
**feeds codegen**, and the hand-rolled ODIN reader (for ADL/ODIN
*instance* parsing — deliberately off the codegen path).

- **This crate is upstream of everything generated.** A change to the BMM
  loader can silently change what `openehr-codegen` emits across five
  crates — after ANY loader/model change, run `/regen-codegen` and inspect
  the diff; the `codegen-drift` CI job is the backstop.
- Codegen consumes the JSON BMM serialization only
  (`crates/openehr-codegen/vendor/bmm/`); the ODIN reader exists for
  ADL 1.4/ODIN instance text, not for loading meta-models.
- Spec authority: `docs/specs/openehr/LANG/docs/` (bmm, odin). Parser
  behaviour divergences are spec-citable, never silent.
- Versioned LANG 1.0.0 (spec pin).
- Gates: `cargo clippy -p openehr-lang --all-targets` +
  `cargo nextest run -p openehr-lang`, plus a drift check when the model
  changed.
