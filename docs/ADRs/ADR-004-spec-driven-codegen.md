# ADR-004: Spec-driven code generation of the openEHR crates from BMM

- **Status:** accepted
- **Date:** 2026-07-03
- **Supersedes (in part):** ADR-001 (spec-transcription shapes) and ADR-002
  (canonical-JSON self-tagging) as *hand-authoring conventions* — their intent
  is now realized by the generator, not by hand. ADR-003 (spec-gap policies)
  still governs the hand-written behaviour layer.

## Context

The openEHR spec crates (`openehr-base`, `openehr-rm`, `openehr-am`, …) were
being **hand-transcribed** class-by-class from the published specifications
(ADR-001/002/003). That produced ~37,000 lines of dense, annotated Rust (RM
alone: 23.6K lines / 133 files) and was slow, error-prone, and — worst — a
prerequisite blocking the actual goal, the EHRbase Java→Rust server port.

openEHR publishes a **machine-readable meta-model**: the Basic Meta-Model
(BMM). `specifications-ITS-BMM` ships a JSON serialization (`*.bmm.json`) of
every component for our exact target versions. A class entry carries everything
the hand-written code encoded: each property with its type, `is_mandatory`
(→ optionality), generic instantiation, container kind + cardinality,
`ancestors`, `documentation` (verbatim), `invariants`, and function signatures.
The `packages` tree gives the module layout; `includes` gives cross-schema
dependencies. openEHR's own `schema_author` field literally reads
"code-generator" — these files are designed to drive tooling.

The forcing question: should the structural layer of the spec crates be
generated deterministically from BMM, or kept hand-written?

## Decision

**Generate the structural layer of the openEHR spec crates from the vendored
BMM, deterministically.** Concretely:

1. **Pipeline.** `openehr-lang::bmm` loads a vendored `*.bmm.json` (via
   `serde_json`) into a typed model; `openehr-codegen` walks it and emits Rust;
   `openehr-derive` supplies the `#[derive(OpenEhrType)]` macro used by the
   emitted types. Input is JSON, not the ODIN form — it is a cleaner, structured
   serialization of the identical meta-model. (The ODIN reader in
   `openehr-lang::odin` is retained for ADL/ODIN *instance* parsing at P06/P13,
   off the codegen path.)

2. **Target = best-possible idiomatic Rust, not a mirror of the old code, and
   not a literal transcription of the Eiffel-shaped spec.** The binding
   fidelity constraint is **wire + semantic + invariant parity** (canonical
   JSON/XML round-trips against the EHRbase corpus; invariants preserved), never
   struct-shape equivalence with the prior hand-written crates.

3. **Emission rules.**
   - **Flattened concrete structs**: a concrete class inlines all inherited
     fields (ancestor-first, `// inherited: X` banners); one hop to any field.
     Abstract classes emit no struct — their fields flatten into descendants.
   - **Closed subtype sets** (`DATA_VALUE`, `ITEM`, `ARCHETYPE_CONSTRAINT`, …):
     an abstract class used as a field type becomes an untagged Rust `enum`,
     one variant per concrete descendant, dispatched on each payload's `_type`.
   - **Enumeration classes** (spec type = a primitive on the wire, e.g.
     `VALIDITY_KIND`): a `#[serde(transparent)]` newtype over the primitive.
   - **`_type` discriminator**: handled by `#[derive(OpenEhrType)]` — a manual
     `Serialize` emits `_type` first and omits `None`/empty; a tolerant
     `Deserialize` validates `_type` when present. No per-struct tag field.
   - **Strong typing / proven crates where unambiguous**: `Integer`→`i32`,
     `Real`/`Double`→`f64`, `Octet`→`u8`, and `UUID.value`→`uuid::Uuid`.
     Where openEHR's semantics are deliberately broader than a crate — ISO 8601
     partial precision, plain-text/relative URIs — the field stays `String` and
     the crate is used only in the hand-written behaviour layer.
   - **Generics**: kept only where a type parameter is *explicitly* used in a
     field (`DvInterval<T>`, `OriginalVersion<T>`). A bare reference to a
     generic class is auto-filled with the parameter's bound
     (`DV_INTERVAL` → `DvInterval<DvOrdered>`); a class whose parameter is unused
     after filling is monomorphized (emitted non-generic).
   - **Recursion**: direct, mutual, and F-bounded cycles are boxed
     (`Box<..>`), detected over the set of spec names a rendered type embeds by
     value; types already behind `Vec`/`BTreeMap`/`BTreeSet` are not boxed.
   - **Cross-crate resolution**: a reference to a type a dependency crate emits
     resolves to that crate's prelude (`openehr_base::prelude::X`); a reference
     resolvable nowhere degrades to `serde_json::Value`.
   - **Layout**: module tree derived from the BMM `packages`; generated
     `mod.rs` per package, a `prelude`, and `lib.rs`. Multi-version components
     (AM) place each version in its own namespace module (`am14`, `am24`).

4. **Generated vs hand-written split.** Generated files carry an `// @generated
   … DO NOT EDIT` header and no `PORT STATUS` trailer. Hand-written
   spec-function and invariant bodies (ADR-003) live in sibling `*_impl.rs`
   files the generator never rewrites. Regeneration wipes only generated files.

5. **Override layer.** A small set of decisions BMM cannot express (strong
   newtypes, primitive backing, name escapes, the `UUID`→`uuid::Uuid` mapping)
   is currently a hardcoded map in the emitter (`emit.rs`: `type_override`,
   `class_binding`, `field_default`), slated to move to a `codegen.toml` seeded
   from that map.

6. **Crate naming (from the same work).** `openehr-*` = the spec (generated);
   `ehrbase-*` = the ported application. `openehr-foundation` folds into
   `openehr-base` (BASE is one component / one BMM file).

## Consequences

- **Easier**: a spec-version bump (RM 1.2.0 → next) is a re-run, not a rewrite;
  the whole structural layer is regenerable and internally consistent; the
  team's effort concentrates on the non-computable behaviour (ADR-003) and the
  actual EHRbase port. Emitted code is idiomatic and clippy-clean by
  construction (lint exceptions inherent to verbatim spec docs are declared once
  in the generated `lib.rs`).
- **Harder / follow-on work, honestly**:
  - The fidelity gate (EHRbase canonical-JSON corpus round-trip) must be
    re-implemented against the generated types; the old hand-written tests in
    `openehr-its/tests` reference the removed API and are currently broken.
    The vendored corpus data is preserved.
  - Some precision is traded for a clean first cut: cross-schema-only references
    and free-form `Hash` degrade to `serde_json::Value`; monomorphized
    version-family classes carry `data: serde_json::Value`.
  - `openehr-term` is **not** generated: BMM defines only its handful of service
    interfaces, while the crate's real content is the terminology bundle, XML
    assets, and access logic — none of which BMM can produce.
  - The override map is not yet externalized to `codegen.toml`.
  - Pinned to the *latest* spec versions (RM 1.2.0, BASE 1.3.0, AM 1.4.0 +
    2.4.0), which diverge from what stock EHRbase/`archie` emits (RM 1.1.0-era)
    — a Stage-1 REST-parity consideration to revisit.

## Alternatives considered

- **Continue hand-transcribing** (ADR-001/002/003 as-is). Rejected: it is the
  slow, error-prone method the project set out to escape, and it blocks the port.
- **Retrofit + converge** (generate, but treat the existing hand-written code as
  the golden target). Rejected: it anchors the generator on reproducing the very
  shapes we want to leave behind (deep access chains, per-struct tag fields,
  F-bounded gymnastics); the correct target is idiomatic Rust with wire parity.
- **Consume the ODIN `.bmm` form** instead of JSON. Rejected for the codegen
  input: JSON is a cleaner, structured serialization (real arrays, structured
  cardinality, explicit `_type` tags) and `serde_json` parses it robustly; the
  hand-rolled ODIN reader stays for ADL/ODIN instance parsing where no
  pre-parsed form exists.
- **Faithfully mirror every spec generic and F-bound as a Rust generic.**
  Rejected: reintroduces the F-bounded-generic complexity that bloated the
  hand-written code; bound-fill + monomorphization compiles cleanly and stays
  readable.
- **`PhantomData` for unused type parameters** (instead of monomorphizing).
  Rejected: it would serialize a spurious field under the `OpenEhrType` derive
  and adds noise; dropping the unused parameter yields cleaner types.
