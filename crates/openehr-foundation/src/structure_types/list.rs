//! `List<T>` — ordered container that may contain duplicates.
//!
//! openEHR class: `List<T>`, package `base.foundation_types.structures`.
//! Inherits: `Container<T>`.
//!
//! Ordered container that may contain duplicates. Adds `first()`/`last()`
//! element access plus two validity invariants over those functions.
use super::super::primitive_types::any::Any;
use super::container::Container;

/// Per `docs/PORTING.md` §14.2 (`List<T>` → `Vec<T>`), transcribed as a
/// transparent newtype over `std::vec::Vec<T>` — the standard Rust container
/// with exactly the spec's stated semantics: implied insertion order,
/// non-unique membership.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct List<T>(pub Vec<T>);

impl<T> List<T> {
    /// `first() -> T`.
    ///
    /// Return first element.
    ///
    /// PORT NOTE: the spec types this as a bare `T`, but the class invariant
    /// `First_validity` (`not is_empty implies first /= Void`) only
    /// constrains the *non-empty* case — it does not state `first`'s result
    /// on an empty list, which Eiffel's "or Void" convention elsewhere in
    /// this spec (see `Container::select`) suggests should not panic
    /// silently. Transcribed as `Option<&T>` rather than a bare `&T`/panic,
    /// so the empty case is representable without violating the "never
    /// unwrap/expect outside tests" rule.
    #[must_use]
    pub fn first(&self) -> Option<&T> {
        self.0.first()
    }

    /// `last() -> T`.
    ///
    /// Return last element.
    ///
    /// PORT NOTE: same `Option`-widening rationale as `first` above, for the
    /// `Last_validity` invariant.
    #[must_use]
    pub fn last(&self) -> Option<&T> {
        self.0.last()
    }
}

impl<T: PartialEq> Container<T> for List<T> {
    fn has(&self, v: &T) -> bool {
        self.0.contains(v)
    }

    fn count(&self) -> i32 {
        // TODO(port): spec `count()` returns `Integer` (32-bit); a `List`
        // holding more than `i32::MAX` elements cannot be faithfully
        // represented by this cast. No spec guidance on overflow behaviour;
        // left as a direct cast pending a decision (see also
        // `primitive_types::integer::Integer`'s modulo TODO for the same
        // class of unspecified-edge-case gap).
        self.0.len() as i32
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T: PartialEq> Any for List<T> {
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn type_of(&self) -> String {
        "List".to_string()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.structures §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/list.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-structure_types.adoc §Class Definitions / list.adoc §List Class
//   confidence: medium
//   todos: 1
//   note: first/last widened to Option<&T> (spec's own "or Void" convention elsewhere, no stated empty-list behaviour here); count()'s i32 cast has no spec-defined overflow behaviour for lists exceeding i32::MAX elements. First_validity/Last_validity invariants are encoded structurally by returning None on empty rather than as a runtime Validate check.
// ─────────────────────────────────────────────
