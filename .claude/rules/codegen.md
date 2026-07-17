---
paths: ["crates/openehr-codegen/**", "crates/openehr-base/**", "crates/openehr-rm/**", "crates/openehr-am/**", "crates/openehr-lang/**", "crates/openehr-term/**", "crates/openehr-its/src/**/generated/**"]
---

# Code generation

The openEHR **spec + serialization + REST-contract layer is generated**, not
hand-written. `openehr-codegen` reads the vendored specs and emits Rust:

- `emit` → the spec crates from BMM: `openehr-base`, `openehr-rm`, `openehr-am`
  (`am14`+`am24`), `openehr-term`, `openehr-lang`.
- `emit-xml` → canonical-XML `ToXml`/`FromXml` for the RM/BASE types into
  `openehr-its/src/xml/generated/` (from the vendored XSDs + the BMM field model).
- `emit-rest` → the ITS-REST contract (DTOs + `#[async_trait]` server traits +
  routes) into `openehr-its/src/rest/generated/` (from the vendored `-codegen` OAS).
- `emit-rm-model` → the static RM attribute/type model (attributes+types,
  multiplicity, descendant/ancestor sets, structure classification) into
  `openehr-rm/src/model/` — the AQL planner's oracle. `emit` already
  emits it as part of `openehr-rm`; this target refreshes just that subtree.
- `emit-opt` → the OPT 1.4 model + XML codec (`opt14`) into `openehr-its`.

## The one hard rule

**Never hand-edit a `// @generated` file.** To change generated output, edit the
emitter (`crates/openehr-codegen/src/{emit,emit_xml,emit_rest,emit_rm_model,xsd,oas,naming}.rs`)
or a hand-written `*_impl.rs` sibling (spec behaviour), then regenerate.
A hand edit is silently overwritten on the next `emit` and fails the CI
`codegen-drift` job.

## Workflow

1. Change the emitter/override, not the output.
2. Regenerate: `cargo run -p openehr-codegen -- emit && … emit-xml && … emit-rest`
   (or use the `/regen-codegen` skill).
3. `cargo build`/`clippy`/`nextest` the affected crates + `openehr-its` gates.
4. Commit the emitter change **and** the regenerated output together (the
   drift-check requires them in sync).

## Notes

- The generated crates are idiomatic + clippy-clean *by construction*; a
  lint exception inherent to verbatim spec docs is declared once in the generated
  `lib.rs`/file header, never per-hand-edit.
- Vendored inputs: BMM at `openehr-codegen/vendor/bmm/`, XSD/OAS/JSON schemas at
  `openehr-its/schemas/` + `openehr-its/vendor/rest-oas/` (each with `PROVENANCE.md`).
  The spec *text* at `docs/specs/openehr/` is read-only reference for humans/agents
  (spec-adherence.md) — never a codegen input, never hand-edited.
- Hand-written spec crates (NOT generated): `openehr-term` bundle/assets,
  `openehr-its` runtime (`xml/runtime.rs`, `rest/runtime.rs`), `openehr-query`
  (AQL parser), `openehr-flat`. These follow `rust-style.md`.
