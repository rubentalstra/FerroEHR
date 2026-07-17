# `openehr-am` — AM 1.4.0 + 2.4.0 (GENERATED)

The Archetype Model, both versions as separate namespaces: `am14`
(ADL 1.4 / OPT 1.4) and `am24` (ADL 2 / AOM 2.4). **Generated from the
vendored BMM** by `openehr-codegen -- emit`.

- **NEVER hand-edit a file with a `// @generated … DO NOT EDIT` header.**
  Change the emitter and regenerate (`/regen-codegen`); `codegen-drift` CI
  guards it.
- Hand-written constraint semantics live ONLY in `*_impl.rs` siblings with
  AM spec citations (`docs/specs/openehr/AM/docs/` — ADL1.4, ADL2, AOM1.4,
  AOM2). AOM2 validity codes (VCOC, VATID, VCORM, …) are the error
  vocabulary for template-ingestion validation — keep codes exact.
- Keep `am14` and `am24` strictly parallel — never blend the two models or
  "share" a type across them outside what the BMM defines.
- Template/OPT ingestion and WebTemplate building live in the application
  (`app/ehrbase`), not here; this crate is the model only.
