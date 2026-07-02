//! `Any` — abstract ancestor class for all other classes.
//!
//! openEHR class: `Any` (abstract), package `base.foundation_types.primitive_types`.
//!
//! Usually maps to a type like `Any` or `Object` in an object-oriented
//! technology. Defined by the specification to provide value and reference
//! equality semantics that every other foundation type inherits.
//!
//! # Transcription approach
//!
//! `Any` has no attributes and is the root of every foundation-type
//! hierarchy, so it is modelled as a Rust trait rather than a struct. Every
//! concrete primitive type in this module (`Boolean`, `Character`, `Octet`,
//! `OpenEhrString`, `Integer`, `Integer64`, `Real`, `Double`, `Uri`) and the
//! abstract descendants `Ordered` and `Numeric` require `Any` as a supertrait,
//! mirroring the spec's inheritance diagram (drawn there "only for
//! convenience", per the chapter overview, to indicate the `=` operator is
//! assumed available on every type).
//!
//! Symbolic operators from the spec (`=`, `==`, `!=`, `≠`) are represented as
//! named methods, never `std::ops` trait overloads, per the RM transcription
//! rules.
pub trait Any {
    /// `is_equal(other: Any) -> Boolean` (abstract).
    ///
    /// Value equality: return `true` if `self` and `other` are attached to
    /// objects considered to be equal in value.
    fn is_equal(&self, other: &Self) -> bool;

    /// `equal` __alias__ `"="`, `"=="` `(other: Any) -> Boolean`.
    ///
    /// Reference equality for reference types, value equality for value
    /// types. The specification declares this as a distinct function from
    /// `is_equal`, but does not narrow it further at this abstract level;
    /// every primitive type transcribed in this module is a Rust value type,
    /// so the default implementation delegates to `is_equal`. Concrete types
    /// with the spec's `(redefined)` marker on `equal` override this method
    /// explicitly and document the redefinition.
    fn equal(&self, other: &Self) -> bool {
        self.is_equal(other)
    }

    /// `not_equal` __alias__ `"!="`, `"≠"` `(other: Ordered) -> Boolean`.
    ///
    /// True if the current object is not equal to `other`. Per the spec
    /// postcondition, this returns `not equal(other)`.
    ///
    /// PORT NOTE: the published table types the `other` parameter of
    /// `not_equal` as `Ordered`, not `Any`, even though the function is
    /// declared on `Any` — this looks like an editorial artifact in the
    /// specification rather than an intentional constraint (`Ordered` is
    /// itself an `Any` descendant, and no other `Any` function narrows to a
    /// subtype of its own receiver type). Transcribed as `&Self` for
    /// uniformity with `is_equal`/`equal` above; flagged here rather than
    /// silently "corrected" in the doc text.
    fn not_equal(&self, other: &Self) -> bool {
        !self.equal(other)
    }

    /// `type_of(an_object: Any) -> String`.
    ///
    /// Type name of an object as a string. May include generic parameters,
    /// as in `"Interval<Time>"`.
    ///
    /// TODO(port): the spec signature takes `an_object` as an explicit
    /// parameter (a class-level/static-style function on `Any`) rather than
    /// operating on `self`; that shape does not have a direct trait-method
    /// equivalent without a `Self: 'static` + `std::any::type_name` bridge
    /// or a per-type constant. Left as an instance method returning the
    /// receiver's own type name until the generic-parameter rendering
    /// question (needed once `Interval<T>` and other generics exist) is
    /// resolved.
    fn type_of(&self) -> String;

    /// `instance_of(a_type: String) -> Any` (abstract).
    ///
    /// Create a new instance of a type, named by string.
    ///
    /// PORT NOTE: this is a reflective factory function — construct an
    /// instance of an arbitrary type from its name at runtime. Rust has no
    /// built-in reflection-by-type-name; a faithful transcription requires a
    /// type registry (e.g. a `HashMap<&str, fn() -> Box<dyn Any>>`) that does
    /// not exist yet at this layer of the crate and would have to live above
    /// every concrete type this trait is implemented for, not inside the
    /// trait itself. Not modelled as a trait method here; left as a
    /// documented gap. A later phase may add a free function or registry in
    /// this module once the full set of foundation types is known.
    ///
    /// TODO(port): decide and implement the type registry, if this function
    /// is ever actually exercised by ported code (the spec marks it
    /// abstract, but no RM class transcribed so far calls it directly).
    fn instance_of(_a_type: &str) -> Option<Self>
    where
        Self: Sized,
    {
        None
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/any.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / any.adoc §Any Class
//   confidence: medium
//   todos: 2
//   note: instance_of has no faithful static-dispatch shape in Rust without a type registry; type_of's class-level signature needs revisiting once generic types (Interval<T>) exist to render "Interval<Time>"-style names.
// ─────────────────────────────────────────────
