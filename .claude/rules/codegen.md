---
paths: ["tools/openehr-codegen/**", "crates/openehr-base/**", "crates/openehr-rm/**", "crates/openehr-am/**", "crates/openehr-lang/**", "crates/openehr-term/**", "crates/openehr-its/src/**/generated/**"]
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

## The three hard rules

**The generator emits the COMPLETE model — never minimize (owner hard
rule, 2026-07-19).** Everything the vendored inputs (and any legitimate
emission closure over them) define gets emitted in full, mirrored to its
source package path — including classes nothing consumes yet; future need
is the point. Forbidden moves: narrowing a schema merge to shrink a
closure, pruning "unrelated" classes out of an emission, suppressing
generated files to quiet a diff, or restoring-around a generation defect
instead of fixing it. If an emission change pulls in a large new class
set, that is the CORRECT outcome — emit it all and let the diff be big.

**A generated-model gap is fixed in the GENERATOR, never worked around in a
consumer (owner hard rule, 2026-07-19).** When a consumer (`ehrbase-*`,
`openehr-adl`, `openehr-its` runtime, …) hits a generated shape that is
wrong or insufficient versus the vendored spec/BMM — a missing subtype
seam, a too-narrow field, a closed enum a downstream component's BMM
extends — the fix is an emitter/override change + regeneration. Never a
shadow type, duplicate model, adapter layer, placeholder value, or
"temporary" local representation: that silently forks the spec model the
generated crates exist to guarantee. Cross-component subtype extension
(e.g. AM classes extending LANG's expression classes) is re-opened by the
emitter at the DOWNSTREAM crate boundary (an extender-level enum composing
the upstream variants + the downstream leaves) — upstream crates never
gain downstream variants (dependency arrows point one way). If the emitter
fix is large, register a worklist row; the workaround is still forbidden.
Existing workarounds get removal rows on discovery.

**Never hand-edit a `// @generated` file.** To change generated output, edit the
emitter (`tools/openehr-codegen/src/`, the `load/`→`analyze/`→`plan/`→`render/`
pipeline stages; decision maps live in `plan/mod.rs`, text producers in
`render/`) or a hand-written `*_impl.rs` sibling (spec behaviour), then regenerate.
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
