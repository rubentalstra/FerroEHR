# `openehr-codegen` — the BMM/XSD/OAS → Rust generator (hand-written tooling)

The single deterministic generator behind the whole spec layer. Lives under
`tools/*` (dev tooling; nothing ships it). Subcommands: `emit` (BMM →
`openehr-base/rm/am/term/lang` + the `openehr-rm` model), `emit-xml` (XSD+BMM →
`ToXml`/`FromXml` in `openehr-its`), `emit-rest` (OAS → the ITS-REST contract in
`openehr-its`), `emit-opt` (OPT 1.4 model + XML codec), `emit-rm-model` (the
static RM attribute/type model), `check`/`check-xsd` (input validation).

## Pipeline structure (four stages + CLI)

The generator is organised as a four-stage pipeline, one module per stage —
each stage consumes the previous stage's output, never the raw files:

- `src/load/` — parse the vendored inputs verbatim (`bmm`, `xsd`, `oas`); no
  analysis, no decisions.
- `src/analyze/` — model analysis over the loaded BMM: the merged include
  closures (`Model`), descendant/variant sets, the ownership graph +
  back-reference cycle breaking, the constructibility proof, and the
  cross-schema re-emission closure. Plain analysis results, no text.
- `src/plan/` — the emission-decision layer: `decide` (the Rust shape each
  class emits as) and the XML-shape classification, plus the declarative
  decision maps (`type_override` / `class_binding` / `back_reference` /
  `field_default`), each carrying its spec citation.
- `src/render/` — the only stage that produces text: the per-shape emitters
  (`emit`), plus `emit_xml` / `emit_rest` / `emit_rm_model` / `emit_opt` and
  the shared `naming` helper.
- `src/cli.rs` + a thin `src/main.rs` — argument dispatch and the
  stage-orchestrating `cmd_*` handlers that write each emit target's files.

## Discipline

- **Every emitter change must be followed by regeneration + diff review**
  (`/regen-codegen` runs all emits + the drift check). Never commit an
  emitter change without its regenerated output in the same change — the
  `codegen-drift` CI job fails otherwise.
- The emitter owns generated-code quality: generated crates must stay
  idiomatic and lib-clippy-clean **by construction**. A clippy warning in
  generated output is an emitter bug — fix it here, never in the output.
- Emission conventions are settled decisions (flattened concrete structs,
  untagged enums for closed subtype sets, `Box` for recursion, bound-fill
  + monomorphization for generics, `// @generated` headers) — do not
  re-litigate per class; extend the decision maps
  (`type_override`/`class_binding`/`field_default` in `plan/mod.rs`) instead.
- Vendored inputs live at `vendor/bmm/` (with provenance) — never edit a
  vendored file; a spec bump re-vendors and regenerates.
- The generator writes ONLY generated files/subtrees; it must never touch
  `*_impl.rs` siblings or hand-written runtimes.
- Gates: `cargo clippy -p openehr-codegen --all-targets` +
  `cargo nextest run -p openehr-codegen` + a clean drift check.
