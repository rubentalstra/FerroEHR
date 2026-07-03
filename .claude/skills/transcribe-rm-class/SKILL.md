---
name: transcribe-rm-class
description: >
  RETIRED by ADR-004 — do NOT use. openEHR RM/BASE/AM classes are GENERATED
  from the vendored BMM meta-model by `openehr-codegen`, not hand-transcribed.
  To change a spec type, edit the emitter and run
  `cargo run -p openehr-codegen -- emit`.
argument-hint: "(retired — use the code generator instead)"
---

# /transcribe-rm-class — RETIRED (ADR-004)

**Do not use this skill.** Hand-transcribing openEHR classes is over. The
RM/BASE/AM crates are generated from the BMM meta-model.

- To change a generated spec type: edit `crates/openehr-codegen/src/emit.rs`
  (or its override map) and regenerate with `cargo run -p openehr-codegen -- emit`.
- To add spec-function/invariant behaviour: write a sibling `*_impl.rs`.
- `openehr-term` is the one hand-written spec crate (bundle/assets/logic not in
  BMM).

See `docs/ADRs/ADR-004-spec-driven-codegen.md` and
`.claude/rules/rm-transcription.md`.
