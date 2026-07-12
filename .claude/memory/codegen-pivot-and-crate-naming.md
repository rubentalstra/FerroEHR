---
name: codegen-pivot-and-crate-naming
description: Strategic pivot — generate the openEHR spec crates from BMM computable artifacts instead of hand-transcribing; and the openehr-*/ehrbase-* crate naming split.
metadata: 
  node_type: memory
  type: project
  originSessionId: 09bd2705-c314-4634-aa51-5c485a9358b2
---

Approved 2026-07-03 (plan: spec-driven code generation). Two durable decisions:

**1. Stop hand-transcribing the openEHR spec; generate it.** openEHR ships a
machine-readable meta-model (BMM, ODIN format) in each spec repo's `computable/`
folder — e.g. `specifications-ITS-BMM/components/RM/odin/openehr_rm_1.1.0.bmm`. It
carries every field/type/cardinality/generic/ancestor/invariant/doc. A generator
(`openehr-codegen`, fed by a BMM reader living in `openehr-lang`'s `odin`+`bmm`
modules) emits the structural Rust. Target = **best-possible idiomatic Rust**
(flattened concrete structs, `Option`/`Vec`, newtypes for IDs, enums for closed
sets, `#[derive(OpenEhrType)]` for `_type` — NOT the old TypeTag-field/F-bounded
style, NOT literal Eiffel-shape mirroring). Binding fidelity = **wire + semantic +
invariant parity** (EHRbase JSON corpus round-trip + ITS-JSON schema + behavior
tests), not matching old struct shapes. The ~880 hand-written behavior fns get
re-homed into never-regenerated `*_impl.rs` files. This supersedes ADR-001/002 for
structure; ADR-003 behavior policies still apply. To be recorded as ADR-004.

**Why:** ~37K lines of hand-transcribed spec Rust was the wrong method — slow,
guessy, and blocking the actual EHRbase Java→Rust port.

**2. Crate naming splits spec from application.** `openehr-*` = openEHR spec
(one crate per component, named like the spec repos); `ehrbase-*` = the ported
EHRbase app. Consolidation (phase 0R, `git mv`): foundation+base→`openehr-base`,
terminology→`openehr-term`, odin+bmm→`openehr-lang`, adl→`openehr-am`,
aql→`openehr-query`; rm/serde/flat keep names. rest→`ehrbase-rest`,
ehrbase-compat→`ehrbase-compat`, server→`ehrbase` (binary). Supersedes
PORT_MASTER_PLAN §9/§9.1. See [[commit-subject-attribution-tokens]].
