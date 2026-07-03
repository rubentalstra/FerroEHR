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
    /// TODO(port): P17 — the spec signature takes `an_object` as an explicit
    /// parameter (a class-level/static-style function on `Any`) rather than
    /// operating on `self`; that shape does not have a direct trait-method
    /// equivalent without a `Self: 'static` + `std::any::type_name` bridge
    /// or a per-type constant. Left as an instance method returning the
    /// receiver's own type name; revisit the generic-parameter rendering
    /// (`"Interval<Time>"`-style names for `Interval<T>` and other
    /// generics) in the P17 make-it-compile pass.
    fn type_of(&self) -> String;

    /// `instance_of(a_type: String) -> Any` (abstract).
    ///
    /// Create a new instance of a type, named by string.
    ///
    /// PORT NOTE (documented deviation, ADR-003 decision 7): this is a
    /// reflective factory function — construct an instance of an arbitrary
    /// type from its name at runtime. Rust has no reflection-by-type-name
    /// short of a global type registry (e.g. a `HashMap<&str, fn() ->
    /// Box<dyn Any>>`), and EHRbase has zero consumers of this function, so
    /// per the ADR the method deliberately stays a stub returning `None`
    /// rather than growing a registry nothing uses. This is a permanent,
    /// recorded deviation, not unfinished work; if a genuine consumer ever
    /// appears, the registry design question reopens then.
    #[must_use]
    fn instance_of(_a_type: &str) -> Option<Self>
    where
        Self: Sized,
    {
        None
    }
}

// PORT NOTE: raw `i64` stands in for `Integer64` where covariant
// redefinition (`DV_COUNT.magnitude`, ADR-001 §6) uses the bare primitive;
// together with the matching `Ordered`/`Numeric` impls (`ordered.rs`,
// `numeric.rs`) this resolves the P17-flagged bound conflict between
// `DV_COUNT.magnitude: i64` and the `T: OrderedNumeric` bound on
// `DvAmountApi`/`DvQuantifiedApi` in `openehr-rm`. Coherence requires the
// impl to live here in `openehr-foundation` (the trait owner).
impl Any for i64 {
    fn is_equal(&self, other: &Self) -> bool {
        self == other
    }

    fn type_of(&self) -> String {
        "Integer64".to_string()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/any.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / any.adoc §Any Class
//   confidence: medium
//   todos: 1
//   note: instance_of is a permanent documented deviation per ADR-003 decision 7 (no type registry, zero EHRbase consumers); type_of's class-level signature and generic-name rendering ("Interval<Time>") deferred to P17.
// ─────────────────────────────────────────────
