---
name: project-primitive-types-precedent
description: Precedent decisions set while transcribing BASE 1.2.0 primitive_types (Any/Ordered/Numeric/Ordered_Numeric/Boolean/Character/Octet/String/Integer/Integer64/Real/Double/Uri) into openehr-foundation — the first files in that crate.
metadata:
  type: project
---

Transcribed the BASE 1.2.0 Foundation Types `primitive_types` cluster into
`crates/openehr-foundation/src/primitive_types/` (13 files: any, ordered,
numeric, ordered_numeric, boolean, character, octet, string, integer,
integer64, real, double, uri). This was the first content ever written into
`openehr-foundation` — no sibling `.rs` files existed before it — so several
naming/shape decisions here are precedent for the whole crate, not just this
cluster.

**Why:** PORT_MASTER_PLAN.md Phase P1 tasks the RM-transcriber with settling
MI/covariance/generic decisions once, in Phase 1, so later phases reuse them
without relitigating.

**How to apply:** any later transcription into `openehr-foundation` or
`openehr-base` that touches these types, or any RM class whose spec table
references `String`, `Integer`, `Real`, etc., should check these decisions
before inventing a new one.

Key decisions:

1. **Abstract classes `Any`, `Ordered`, `Numeric` are Rust traits**, not
   structs — they have no attributes and are pure-behaviour ancestors.
   `Ordered_Numeric` (the spec's own multiple-inheritance example) is a
   supertrait `OrderedNumeric: Ordered + Numeric {}`, **blanket-implemented**
   for any `T: Ordered + Numeric`. Concrete types must NOT also write an
   explicit `impl OrderedNumeric for ConcreteType {}` — it conflicts with the
   blanket impl (E0119, duplicate implementation). This is the reusable
   pattern for every other multiple-inheritance case in Section 7.2
   (`Iso8601_type`, `DV_DURATION`, `EXTERNAL_ENVIRONMENT_ACCESS`): if the
   parent traits are already independently satisfiable, prefer a blanket
   impl over per-type impls.

2. **`Numeric` trait is same-type only** (`fn add(&self, other: &Self) ->
   Self`, etc.) — the spec's abstract signatures are open
   (`Numeric -> Numeric`, "actual type of result depends on arithmetic
   balancing rules"), which Rust traits cannot express without associated
   types keyed per call site. Every concrete effector in this cluster that
   needs a heterogeneous result (`Integer.divide -> Double`,
   `Integer64.add(Integer) -> Integer64`) gets that as an **inherent
   method** on the concrete type, in addition to a same-type trait impl
   (stubbed `todo!()` for `divide`/`exponent` where the trait shape can't
   carry the real signature). Expect this same trait/inherent split to
   recur for other constrained-generic or covariant-arithmetic RM classes
   (e.g. `DV_QUANTITY`/`DV_COUNT` arithmetic).

3. **`String` (the openEHR class) is named `OpenEhrString` in Rust**, not
   `String`, to avoid colliding with `std::string::String`. This is
   deliberately scoped: an ordinary RM struct field of spec type `String`
   still maps directly to `std::string::String` per `docs/PORTING.md`
   §14.2 — `OpenEhrString` is only the *foundation-types class itself* (with
   its own methods: `is_empty`, `is_integer`, `as_integer`, `append`,
   `contains`, `less_than`). Do not use `OpenEhrString` as a generic
   "the RM's String type" substitute for ordinary attribute fields
   elsewhere — that would be over-applying this precedent.

4. **`Character` backed by `char`, not `u8`** — chosen over the more
   "8-bit value" literal spec wording because the spec chapter's own prose
   states Unicode/UTF-8 is assumed for `String` (and by extension its
   `Character` element type), and `Octet` already owns the truly-8-bit
   case in this same cluster.

5. **`Real` backed by `f64`, matching `Double`** — this is a **directed
   deviation** from literal spec text (the spec explicitly says `Real` is
   single-precision/32-bit, "corresponds to a single-precision floating
   point value in most languages"). The task that invoked this
   transcription explicitly instructed `Real -> f64` with a mandated PORT
   NOTE. Recorded loudly (module doc, struct doc, and PORT STATUS trailer)
   as directed rather than inferred, per the hard rule "if the
   specification is genuinely ambiguous... say so rather than guessing
   quietly" — here it isn't ambiguous, it was overridden by explicit
   instruction, which is different and must be flagged even more visibly.
   If f32 vs f64 precision parity ever becomes a concern (e.g. exact
   round-trip against a Java `float`-backed reference value), this is the
   file to revisit.

6. **`Uri` wraps `OpenEhrString`, not `std::string::String` directly** —
   because the spec states `Uri` inherits `String` (the foundation-types
   class), not the raw primitive. Applies the same reasoning as decision 3.

See also [[project-openehr-foundation-crate-state]] for the crate's overall
file inventory as of this transcription.
