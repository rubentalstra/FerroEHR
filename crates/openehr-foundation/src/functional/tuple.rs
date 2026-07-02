//! `TUPLE` — parent type of all TUPLE types.
//!
//! openEHR class: `TUPLE`, package `base.foundation_types.functional`.
//!
//! Per the functional meta-types chapter overview: "the 'tuple' type is
//! defined as a generic meta-type whose descendants may additionally define
//! any number of generic parameter types, corresponding to a type list."
//! `TUPLE` itself declares no attributes, no functions, and no explicit
//! parent (`Inherit` is absent from its per-class table) — it exists purely
//! to name the shared ancestor of `Tuple1<A>`, `Tuple2<A,B>`, and any further
//! `TupleN<...>` the spec may define.
use super::super::primitive_types::any::Any;

/// `TUPLE` has no attributes and no declared functions, so it is modelled as
/// an empty Rust marker trait — the same treatment given to behaviour-only
/// abstract classes elsewhere in this crate (ADR-001 §1), except here there
/// is no behaviour at all to declare as a method, only the "is-a TUPLE"
/// relationship itself.
///
/// PORT NOTE: the spec's per-class table for `TUPLE` has no `Inherit` row
/// (unlike every other class transcribed in this crate so far, which
/// declares `Any` or a named parent explicitly). `Any` is added as a
/// supertrait here for consistency with every other foundation type in this
/// crate ("value and reference equality semantics that every other
/// foundation type inherits," per `primitive_types::any`'s own module doc),
/// not because the `TUPLE` per-class table states it — flagged rather than
/// silently assumed, since this is the one class in this transcription batch
/// whose `Inherit` row the spec leaves blank.
pub trait Tuple: Any {}

/// A zero-argument tuple. Rust's own zero-element tuple `()` is the direct,
/// native representation of "a TUPLE with no typed argument slots" — the
/// base case the arity-parameterised `Tuple1<A>`/`Tuple2<A,B>` descendants
/// build on. The spec does not itself declare a `Tuple0` class, but every
/// `ROUTINE<ARGS>`/`PROCEDURE<ARGS>` needs an `ARGS` type for the
/// zero-argument case, and `()` is that faithful instantiation.
///
/// `Any` is implemented for the native tuple so the `Tuple: Any` supertrait
/// holds; a zero-slot tuple is trivially equal to itself.
impl Any for () {
    fn is_equal(&self, _other: &Self) -> bool {
        true
    }

    fn type_of(&self) -> String {
        "Tuple".to_string()
    }
}

impl Tuple for () {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.functional §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/tuple.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master08-functional.adoc §Class Definitions / tuple.adoc §TUPLE Class
//   confidence: medium
//   todos: 0
//   note: TUPLE's per-class table has no Inherit row; Any supertrait added here for consistency with the rest of the crate rather than read from the spec table itself — flagged explicitly rather than silently assumed. impl Tuple for () is a transcriber-added zero-arity case, not a spec-declared class, needed for ROUTINE<ARGS>/PROCEDURE<ARGS> to have a zero-argument instantiation.
// ─────────────────────────────────────────────
