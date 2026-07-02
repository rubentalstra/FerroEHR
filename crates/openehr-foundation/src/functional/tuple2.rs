//! `TUPLE2<A,B>` — a Tuple type for two typed arguments.
//!
//! openEHR class: `TUPLE2<A,B>`, package `base.foundation_types.functional`.
//! Inherits: `TUPLE`.
//!
//! A Tuple type used, among other things, for representing two typed
//! arguments within a Routine signature. Declares no attributes or functions
//! of its own beyond its `TUPLE` ancestry.
use super::super::primitive_types::any::Any;
use super::tuple::Tuple;

/// Same treatment as `Tuple1<A>` (`super::tuple1`): a documented type alias
/// over Rust's native two-element tuple `(A, B)`, the faithful representation
/// of a two-slot argument list.
pub type Tuple2<A, B> = (A, B);

/// `Any` for the underlying native tuple (required by the `Tuple: Any`
/// supertrait): slot-wise `is_equal`, `type_of` composed from the slot types.
impl<A: Any, B: Any> Any for (A, B) {
    fn is_equal(&self, other: &Self) -> bool {
        self.0.is_equal(&other.0) && self.1.is_equal(&other.1)
    }

    fn type_of(&self) -> String {
        format!("Tuple2<{}, {}>", self.0.type_of(), self.1.type_of())
    }
}

impl<A: Any, B: Any> Tuple for (A, B) {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.functional §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/tuple2.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master08-functional.adoc §Class Definitions / tuple2.adoc §TUPLE2 Class
//   confidence: medium
//   todos: 0
//   note: transcribed as a type alias over Rust's native (A, B) tuple, matching Tuple1's treatment; TUPLE ancestry implemented on the underlying (A, B) type.
// ─────────────────────────────────────────────
