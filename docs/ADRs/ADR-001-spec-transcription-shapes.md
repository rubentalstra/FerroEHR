# ADR-001 — Rust shapes for openEHR spec transcription (MI, covariance, generics)

- Status: **superseded in part by ADR-004** (as hand-authoring *conventions* —
  the spec crates are generated, not hand-transcribed). The MI/covariance/generic
  *outcomes* still describe what the emitter produces.
- Date: 2026-07-02
- Phase: P1 (docs/plans/phase-01-foundation-identification.md)

> ## ⚠️ AMENDMENT (2026-07-03, ADR-004): generation replaced hand-transcription
>
> ADR-004 supersedes this ADR **as hand-authoring conventions**: the openEHR spec
> crates (`openehr-base`, `openehr-rm`, `openehr-am`) are now **generated** from
> the vendored BMM meta-model — no one hand-transcribes RM classes copying these
> shapes. The MI/covariance/constrained-generic/closed-enum mappings below still
> describe the *emitter's* choices (ADR-004 §3), so the decisions read true; the
> "P3 transcribers copy shapes mechanically" framing does not. Note also: the
> `crates/openehr-foundation/…` worked-example paths are stale — that crate was
> folded into `openehr-base`; and the `TYPE_NAME` const / deferred-serde plan was
> replaced by `#[derive(OpenEhrType)]` (see ADR-002's amendment).

## Context

The openEHR BASE/RM specifications use multiple inheritance, covariant
redefinition, and constrained generics (PORT_MASTER_PLAN.md Section 7.2).
Rust has none of these directly. P1 must fix the mapping once so all ~108 RM
classes in P3 (and every later phase) reuse the same shapes instead of
re-deciding per class. These decisions were sketched in Section 14.4 and
`.claude/rules/rm-transcription.md`; this ADR is the binding record.

## Decision

1. **Abstract class without attributes → trait.** `Any`, `Ordered`,
   `Numeric` are traits; spec functions become trait methods with named
   (never symbolic) identifiers: `+` → `add`, `<` → `less_than`,
   `and then` → `and_then`, `∀` → iterator `all`.
2. **Multiple inheritance → supertrait composition (behaviour) plus field
   embedding (state).** `Ordered_Numeric` is `trait OrderedNumeric:
   Ordered + Numeric {}`. Where a parent carries attributes (e.g. RM
   abstract classes), each concrete type embeds the parent's fields via a
   flattened struct and implements one trait per parent behaviour. Worked
   examples: `crates/openehr-foundation/src/primitive_types/ordered_numeric.rs`
   and the `Iso8601_type` family in `crates/openehr-foundation/src/time/`.
3. **Abstract class with attributes → embedded struct + marker trait.**
   Concrete types hold the parent struct as a `#[serde(flatten)]` field (or
   copy its fields verbatim when flattening is impossible); the trait
   exposes accessor methods so code can stay polymorphic.
4. **Closed subtype set → enum.** `DATA_VALUE`, `ITEM`, `CONTENT_ITEM`,
   `PARTY_PROXY`, `VERSION<T>` and every other closed hierarchy become a
   Rust `enum` with one variant per concrete class. Trait objects only for
   genuinely archetype-driven open polymorphism.
5. **Constrained generic → generic with trait bound.**
   `Interval<T: Ordered>` → `pub struct Interval<T: Ordered>`;
   `DV_INTERVAL<T: DV_ORDERED>`, `HISTORY<T: ITEM_STRUCTURE>`,
   `VERSIONED_OBJECT<T>` follow identically.
6. **Covariant redefinition → narrowed type on the concrete struct.**
   `LOCATABLE_REF.id` is declared `UID_BASED_ID` (not `OBJECT_ID`) directly
   on `LocatableRef`, with a doc comment naming the override;
   `DV_COUNT.magnitude` is `i64` where the parent says `Ordered_Numeric`.
   No generic parameter gymnastics to simulate covariance.
7. **Primitive types → std mappings behind spec-named aliases/newtypes.**
   Boolean→`bool`, Character→`char`, Octet→`u8` (Octet, not "Byte"),
   String→`String`, Integer→`i32`, Integer64→`i64`, Real/Double→`f64`
   (PORT NOTE on the Real/Double collapse), Uri→newtype over `String`.
8. **Back-references → `Weak` or path-index, never owning; recursive
   containment → `Box`.** (Restated from Section 14.4; applies from P3 on.)
9. **File layout: one class per file**, snake_case, grouped by spec package
   (`src/primitive_types/`, `src/interval/`, `src/time/`, …). Files stay
   unwired (no `mod` declarations) until P17 so the workspace keeps
   compiling and CI stays green through Phase A.

## Consequences

- P3 transcribers copy shapes mechanically; fidelity reviews diff against
  the spec table plus this ADR.
- Enums make exhaustive `match` the dispatch mechanism — adding a subtype
  is a compile-visible event, which is what we want for closed spec sets.
- The Real/Double collapse to `f64` and Integer→`i32` are the only lossy
  primitive choices; both carry PORT NOTEs and can be revisited at P17
  without touching call sites that use the spec-named aliases.
- Unwired files mean `cargo build` proves nothing about transcription
  correctness before P17 — the per-file `rustfmt` parse check and
  port-reviewer passes are the Phase A quality gates.

## Refinements (P1, applied)

- **Abstract class with attributes that is also used polymorphically as a
  declared field type** (`UID`, `OBJECT_ID`, `UID_BASED_ID`) combines §3 and
  §4: an `XxxData` struct (embeddable state) + a closed `Xxx` enum (one
  variant per concrete descendant) + an `XxxApi` trait (shared accessors,
  implemented on both). Narrower enums nest inside wider ones
  (`ObjectId::UidBased(UidBasedId)`) so §6 covariant narrowing stays a plain
  field type. Worked example: `crates/openehr-base/src/identification/`.
- **serde derives wait until P4** (serde is not yet a dependency of the spec
  crates); until then each concrete class carries
  `pub const TYPE_NAME: &str = "<_type>"`.

## Alternatives considered

- **Trait objects everywhere** (archie-like): rejected — open polymorphism
  where the spec is closed, loses exhaustiveness, boxes everything.
- **Simulating covariance with generics** (`ObjectRef<Id: ObjectIdTrait>`):
  rejected — infects every containing type with parameters the spec does
  not have; the narrowed-field encoding matches the spec text 1:1.
- **One file per spec chapter**: rejected — 108 RM classes in a handful of
  files kills side-by-side diffing against the spec and per-file PORT
  STATUS tracking.
