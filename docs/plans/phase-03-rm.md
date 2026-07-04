# Phase 03 — Reference Model (RM)

- Status: **done** — GENERATED from BMM (ADR-004), not hand-transcribed
- Build order: complete (spec foundation)
- Decisions: ADR-004

## Outcome

RM 1.2.0 (data_types, data_structures, common, ehr, demographic, integration,
support) is **generated** by `openehr-codegen` into `openehr-rm` — flattened
concrete structs, untagged enums for closed subtype sets, `#[derive(OpenEhrType)]`
for `_type`, strong types where unambiguous. Compiles clean, clippy-clean.
`openehr-rm` is the **domain model consumed directly by the `ehrbase-*`
application crates** (ADR-006).

To change an RM type: edit the emitter/`*_impl.rs` and regenerate — never
hand-edit a `// @generated` file.

## Verification

`openehr-rm` builds + lib-clippy clean; the fidelity gate
(`openehr-its/tests/fidelity.rs`) reads + round-trips the real openEHR_SDK
canonical-JSON corpus (53 lossless, documented exclusions).
