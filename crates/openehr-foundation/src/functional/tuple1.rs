//! `TUPLE1<A>` — a Tuple type for a single typed argument.
//!
//! openEHR class: `TUPLE1<A>`, package `base.foundation_types.functional`.
//! Inherits: `TUPLE`.
//!
//! A Tuple type used, among other things, for representing a single typed
//! argument within a Routine signature. Declares no attributes or functions
//! of its own beyond its `TUPLE` ancestry.
use super::super::primitive_types::any::Any;
use super::tuple::Tuple;

/// `TUPLE1<A>` is a pure meta-type with no declared functions or attributes,
/// used only to name a one-slot argument list shape. Rust's native
/// single-element tuple `(A,)` is the direct, faithful representation — a
/// documented type alias, per the task's functional-types mapping guidance,
/// rather than a wrapper struct that would add no capability the native
/// tuple lacks.
///
/// PORT NOTE: a type alias cannot itself carry a trait bound or an `impl`
/// block, so the `TUPLE1<A> inherits TUPLE` relationship is expressed by
/// implementing `Tuple` directly on the underlying `(A,)` type below, rather
/// than on the alias name (aliases are not distinct types in Rust — `impl
/// Tuple for Tuple1<A>` and `impl Tuple for (A,)` are the same impl).
pub type Tuple1<A> = (A,);

/// `Any` for the underlying native tuple (required by the `Tuple: Any`
/// supertrait): slot-wise `is_equal`, `type_of` composed from the slot type.
impl<A: Any> Any for (A,) {
    fn is_equal(&self, other: &Self) -> bool {
        self.0.is_equal(&other.0)
    }

    fn type_of(&self) -> String {
        format!("Tuple1<{}>", self.0.type_of())
    }
}

impl<A: Any> Tuple for (A,) {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.functional §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/tuple1.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master08-functional.adoc §Class Definitions / tuple1.adoc §TUPLE1 Class
//   confidence: medium
//   todos: 0
//   note: transcribed as a type alias over Rust's native (A,) tuple rather than a wrapper struct, per the task's functional-types mapping guidance; the TUPLE ancestry is implemented on the underlying (A,) type since aliases carry no distinct impl target.
// ─────────────────────────────────────────────
