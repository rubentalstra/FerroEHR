---
paths: ["crates/openehr-rm/**", "crates/openehr-base/**", "crates/openehr-am/**", "crates/openehr-term/**"]
---

# RM / BASE / AM: generated from BMM (ADR-004) — do not hand-write

**These crates are GENERATED, not hand-transcribed.** The former "transcribe
each RM/BASE class by hand from the spec" workflow is retired
(`docs/ADRs/ADR-004-spec-driven-codegen.md`). openEHR publishes a
machine-readable meta-model (BMM); `openehr-codegen` turns the vendored
`*.bmm.json` into idiomatic Rust deterministically.

## What to do instead of hand-writing a class

- **Structure wrong / missing** (a field, type, enum variant, module layout):
  fix the **emitter** (`crates/openehr-codegen/src/emit.rs`) or its override
  map, then regenerate with `cargo run -p openehr-codegen -- emit`. Never edit a
  `// @generated` file — the change would be wiped on the next run.
- **Behaviour** (invariants, spec functions, ADR-003 policies): write it in a
  sibling `*_impl.rs` next to the generated file. The generator never touches
  `*_impl.rs`.
- **A cross-cutting emission rule** (new container mapping, a strong-newtype,
  a `(class, field)` type override like `UUID.value → uuid::Uuid`): add it to
  the emitter's override layer, ideally the eventual `codegen.toml`.

## Hard rules for these crates

- Do **not** create or edit `.rs` files under `openehr-base` / `openehr-rm` /
  `openehr-am` `src/` by hand — they are regenerated. (The exception is the
  hand-written `*_impl.rs` behaviour files.)
- Keep the generated crates compiling and **lib-clippy-clean by fixing the
  emitter**, not by adding per-file `#[allow]`s. Lint exceptions inherent to
  faithful spec generation are declared once, in the generated `lib.rs`.
- `openehr-am` holds **both** AM versions as separate modules: `am14` (AM 1.4.0,
  ADL 1.4) and `am24` (AM 2.4.0, ADL 2). Both are required; both are generated.

## `openehr-term` is the exception (still hand-written)

TERM's BMM defines only a handful of service-interface classes; the crate's
real content is the terminology **bundle**, the vendored **XML assets**, and the
**access logic** — none of which BMM can generate. Do **not** run the generator
against `openehr-term`. Hand-written rules for it:

- One TERM class → one Rust struct/enum, openEHR name in a doc comment.
- Preserve the TERM 3.x `id=532` dual-rubric quirk (`complete` vs `completed`)
  verbatim; do not normalize to one.
- `openehr-term` depends on `openehr-base`; never point a dependency arrow
  upward.

## Emission shapes the generator already applies (reference, do not relitigate)

- Abstract class with attributes → its fields flatten into every concrete
  descendant (flattened concrete structs; one hop to any field).
- Closed subtype set (`DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`,
  `ARCHETYPE_CONSTRAINT`, …) → untagged Rust `enum`, `_type`-dispatched.
- Enumeration class (a primitive on the wire, e.g. `VALIDITY_KIND`) →
  `#[serde(transparent)]` newtype.
- Generics kept only where a type parameter is explicitly used; bare generic
  references are bound-filled; unused-param classes are monomorphized.
- Recursion (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`,
  `DV_MULTIMEDIA.thumbnail`, F-bounded ranges) → `Box`.
- `_type` via `#[derive(OpenEhrType)]` (`openehr-derive`) — no per-struct tag
  field.
