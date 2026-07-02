---
name: project-openehr-foundation-crate-state
description: File inventory and spec-cache location for openehr-foundation as of the primitive_types transcription (2026-07-02) — check this is still current before assuming a file exists.
metadata:
  type: project
---

As of 2026-07-02, `crates/openehr-foundation/src/` contains only `lib.rs`
(placeholder doc comment, Phase A empty) plus the newly-added
`primitive_types/` module directory (13 files: `any.rs`, `ordered.rs`,
`numeric.rs`, `ordered_numeric.rs`, `boolean.rs`, `character.rs`,
`octet.rs`, `string.rs`, `integer.rs`, `integer64.rs`, `real.rs`,
`double.rs`, `uri.rs`). `lib.rs` does not yet `mod primitive_types;` — module
wiring is explicitly deferred to P17 (make-it-compile) per the invoking
task's instructions, so do not add `mod` statements to `lib.rs` during
Phase A transcription work.

**Why:** `openehr-foundation` is one of the ten spec crates that "start
empty" (PORT_MASTER_PLAN.md §5.4) — EHRbase never had Java for the RM/BASE
layer, so every file here is freshly transcribed from spec, not ported.
`primitive_types` was the first content landed in this crate.

**How to apply:** before assuming a sibling file exists to check naming
consistency against, actually list `crates/openehr-foundation/src/` — this
memory will go stale as soon as `Interval<T>`, containers, ISO 8601
temporals, or the functional types cluster (the rest of foundation_types
per PORT_MASTER_PLAN.md §7.1) land.

Spec ground truth lives at
`docs/research/spec-cache/BASE-1.2.0/` (chapters in `foundation_types/`,
`base_types/`, `resource/`; per-class tables in `uml_classes/`), pinned at
`specifications-BASE` tag `Release-1.2.0`, commit
`906441385b7c6cb54f1e281f7417a48381c5f057` (short form `9064413`), fetched
2026-07-02. This is the only ground truth to transcribe from — never
`archie` or EHRbase Java (there is none for this crate).

See [[project-primitive-types-precedent]] for the naming/shape decisions
made while writing these 13 files.
