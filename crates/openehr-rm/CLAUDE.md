# `openehr-rm` — RM 1.2.0 (GENERATED) — the domain model

The Reference Model everything consumes, **generated from the vendored
BMM** by `openehr-codegen -- emit` (ADR-004). Versioned by the spec
(1.2.0).

- **NEVER hand-edit a file with a `// @generated … DO NOT EDIT` header.**
  Change the emitter (`crates/openehr-codegen/src/emit.rs`) and regenerate
  (`/regen-codegen`); the `codegen-drift` CI job guards this.
- Hand-written invariants / spec functions live ONLY in `*_impl.rs`
  siblings, each citing its RM spec section
  (`docs/specs/openehr/RM/docs/`). Behavioural back-references
  (`PATHABLE.parent()`) use `Weak`/index, never an owning reference.
- Emission conventions are settled — do not re-litigate per class: closed
  subtype sets → untagged enums; recursion → `Box`; `_type` via
  `#[derive(OpenEhrType)]`; flattened concrete structs.
- The BMM-generated RM *model* (attribute→types, multiplicity,
  descendants) feeds the AQL planner and the validator — treat model-shape
  changes as engine-facing and re-run ECC.
- Downstream crates consume these types directly; never add
  application-specific helpers here.
