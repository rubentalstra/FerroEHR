---
name: regen-codegen
description: >
  Regenerates the openEHR spec + ITS layer from the vendored specs
  (emit / emit-xml / emit-rest) and checks for drift. Use after changing the
  openehr-codegen emitter or bumping a vendored spec, or when asked to
  regenerate the generated crates or run the codegen drift check.
allowed-tools: [Bash, Read]
argument-hint: (none)
---

# /regen-codegen

Regenerate everything `openehr-codegen` produces, in-session, and verify it is
in sync with the vendored specs (ADR-004/005). Never hand-edit `// @generated`
files — that is what this regenerates.

## Steps

1. **Regenerate all targets** (order matters — spec crates first):
   ```
   cargo run -p openehr-codegen -- emit           # openehr-base/rm/am/term/lang (rm incl. src/model)
   cargo run -p openehr-codegen -- emit-xml        # openehr-its XML ToXml/FromXml
   cargo run -p openehr-codegen -- emit-rest       # openehr-its ITS-REST contract
   cargo run -p openehr-codegen -- emit-opt         # openehr-its opt14 (OPT 1.4 model + XML codec)
   cargo run -p openehr-codegen -- emit-rm-model    # openehr-rm src/model (static RM attribute/type model, ADR-008 §3)
   ```
   (`emit` already emits `openehr-rm/src/model`; `emit-rm-model` refreshes just
   that subtree and is byte-identical, so run order does not matter.)
2. **Build + gate the affected crates:**
   ```
   cargo build -q -p openehr-base -p openehr-rm -p openehr-am -p openehr-term -p openehr-lang -p openehr-its
   cargo clippy -q -p openehr-codegen -p openehr-its --all-targets
   cargo nextest run -p openehr-its        # fidelity gates: JSON round-trip + schema + XML round-trip + EHRbase XML
   ```
3. **Drift check** — the generated output must be a pure function of the specs +
   emitter, so a clean tree regenerates byte-identically:
   ```
   bash scripts/check-codegen-drift.sh
   ```
   (Also the CI `codegen-drift` job.) If it reports drift after *your* emitter
   change, that is expected — commit the emitter change **and** the regenerated
   output **together**.
4. **Report** what changed (which crates, file counts) and whether all gates +
   the drift check are green. If a generated diff appears with **no** emitter
   change, something hand-edited a `@generated` file — investigate, don't commit.

## Notes

- Vendored inputs: BMM at `crates/openehr-codegen/vendor/bmm/`, XSD/OAS/JSON at
  `crates/openehr-its/{schemas,vendor}/` (each with `PROVENANCE.md`).
- To change generated output, edit the emitter
  (`crates/openehr-codegen/src/{emit,emit_xml,emit_rest,emit_rm_model,xsd,oas,naming}.rs`)
  or a `*_impl.rs` sibling — see `.claude/rules/codegen.md`.
