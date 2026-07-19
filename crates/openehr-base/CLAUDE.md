# `openehr-base` — BASE 1.3.0 (GENERATED)

Foundation + base types, **generated from the vendored BMM** by
`openehr-codegen -- emit`. Versioned by the spec it implements
(1.3.0), never by the product version.

- **NEVER hand-edit a file with a `// @generated … DO NOT EDIT` header.**
  To change emitted output, edit the emitter
  (`tools/openehr-codegen/src/render/emit.rs` / its override map) and regenerate
  (`/regen-codegen`). The `codegen-drift` CI job fails on any divergence.
- Hand-written spec behaviour (invariants, spec functions)
  lives ONLY in sibling `*_impl.rs` files, which the generator never
  rewrites. Cite the BASE spec section (`docs/specs/openehr/BASE/docs/`)
  for every invariant/function body.
- Interval/`Multiplicity_interval`/`Cardinality` math and ISO 8601
  validation live here as `*_impl.rs` — they are constraint-evaluation
  primitives for the validator; behaviour changes need spec citations and
  ECC verification.
- Gates: crate must stay lib-clippy-clean; fix the emitter, never the
  output.
