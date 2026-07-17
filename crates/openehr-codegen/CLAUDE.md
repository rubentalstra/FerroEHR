# `openehr-codegen` — the BMM/XSD/OAS → Rust generator (hand-written tooling)

The single deterministic generator behind the whole spec layer.
Subcommands: `emit` (BMM → `openehr-base/rm/am` + the
`openehr-rm` model), `emit-xml` (XSD+BMM → `ToXml`/`FromXml` in
`openehr-its`), `emit-rest` (OAS → the ITS-REST contract in `openehr-its`),
`check`/`check-xsd` (input validation).

- **Every emitter change must be followed by regeneration + diff review**
  (`/regen-codegen` runs all three emits + the drift check). Never commit
  an emitter change without its regenerated output in the same change —
  the `codegen-drift` CI job fails otherwise.
- The emitter owns generated-code quality: generated crates must stay
  idiomatic and lib-clippy-clean **by construction**. A clippy warning in
  generated output is an emitter bug — fix it here, never in the output.
- Emission conventions are settled decisions (flattened concrete structs,
  untagged enums for closed subtype sets, `Box` for recursion, bound-fill
  + monomorphization for generics, `// @generated` headers) — do not
  re-litigate per class; extend the override map
  (`type_override`/`class_binding`/`field_default` in `emit.rs`) instead.
- Vendored inputs live at `vendor/bmm/` (with provenance) — never edit a
  vendored file; a spec bump re-vendors and regenerates.
- The generator writes ONLY generated files/subtrees; it must never touch
  `*_impl.rs` siblings or hand-written runtimes.
- Gates: `cargo clippy -p openehr-codegen --all-targets` +
  `cargo nextest run -p openehr-codegen` + a clean drift check.
