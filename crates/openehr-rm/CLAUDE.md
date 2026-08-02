# `openehr-rm` — RM 1.2.0 (GENERATED) — the domain model

The Reference Model everything consumes, **generated from the vendored
BMM** by `openehr-codegen -- emit`. Versioned by the spec
(1.2.0).

- **NEVER hand-edit a file with a `// @generated … DO NOT EDIT` header.**
  Change the emitter (`tools/openehr-codegen/src/render/emit.rs`) and regenerate
  (`/regen-codegen`); the `codegen-drift` CI job guards this.
- Hand-written invariants / spec functions live ONLY in `*_impl.rs`
  siblings, each citing its RM spec section
  (`docs/specs/openehr/RM/docs/`). Behavioural back-references
  (`PATHABLE.parent()`) use `Weak`/index, never an owning reference.
- **`src/validate.rs` is the hand-written value-level validation layer** (the
  one non-`*_impl.rs` exception): the allocation-free fast path, the shared
  invariant helpers, and the model-driven per-node checks run as their own
  layers beside the fast/typed core pair (`check_mandatory_containers`,
  `nonempty_list_violations` — the `x /= Void implies not x.is_empty` family,
  evaluated over the BMM-derived `generated::NONEMPTY_LIST_RULES` table and its
  descendant closure —, `check_archetyped_valid`,
  `check_data_structure_shapes`). **Container shapes carry their own bounds:**
  an optional container emits `Option<Vec<T>>` (absence and present-but-
  emptiness are distinct states the RM relies on) and a `1..*` container emits
  `openehr_base::containers::NonEmptyVec<T>` (the bound is structural, so an
  empty list is unrepresentable rather than merely rejected). Its sibling
  `validate/terminology.rs` is the openEHR terminology-group / code-set binding
  table over the `openehr-term` bundle — **this crate DOES depend on
  `openehr-term`** (`openehr-term` reaches only `openehr-base`, so there is no
  cycle). Every RM decision belongs here; `openehr-its` only dispatches, walks
  instances, and prefixes paths.
- Emission conventions are settled — do not re-litigate per class: closed
  subtype sets → untagged enums; recursion → `Box`; `_type` via
  the native `ToJson`/`FromJson` codec in `openehr-its` (no serde on the spec
  types); flattened concrete structs.
- The BMM-generated RM *model* (attribute→types, multiplicity,
  descendants) feeds the AQL planner and the validator — treat model-shape
  changes as engine-facing and re-run the CNF pipeline.
- Downstream crates consume these types directly; never add
  application-specific helpers here.
