//! `List<T>` — ordered container that may contain duplicates.
//!
//! openEHR class: `List<T>`, package `base.foundation_types.structures`.
//! Inherits: `Container<T>`.
//!
//! Ordered container that may contain duplicates. Adds `first()`/`last()`
//! element access plus two validity invariants over those functions.
use super::super::primitive_types::any::Any;
use super::container::Container;
use serde::de::Error as _;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

    /// Iteration accessor required by `Container<T>` (ADR-003 decision 6);
    /// yields items in the list's insertion order.
    fn items<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.0.iter()
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

impl<T: Serialize> Serialize for List<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let field_count = if self.0.is_empty() { 1 } else { 2 };
        let mut state = serializer.serialize_struct("LIST", field_count)?;
        state.serialize_field("_type", "LIST")?;
        if !self.0.is_empty() {
            state.serialize_field("items", &self.0)?;
        }
        state.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for List<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire<T> {
            #[serde(rename = "_type")]
            type_name: Option<String>,
            items: Option<Vec<T>>,
        }

        let wire = Wire::deserialize(deserializer)?;
        if wire.type_name.as_deref().is_some_and(|name| name != "LIST") {
            return Err(D::Error::custom("expected _type \"LIST\""));
        }
        Ok(List(wire.items.unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::container::Container;
    use super::List;

    // Spec First_validity/Last_validity: `not is_empty implies first/last
    // /= Void` — non-empty lists yield the first/last element, empty lists
    // yield None (the "or Void" widening documented on the methods).
    #[test]
    fn first_and_last_follow_the_validity_invariants() {
        let list = List(vec![10, 20, 30]);
        assert_eq!(list.first(), Some(&10));
        assert_eq!(list.last(), Some(&30));
        let empty = List::<i32>(vec![]);
        assert_eq!(empty.first(), None);
        assert_eq!(empty.last(), None);
    }

    // Spec Container functions: has ("Test for membership of a value"),
    // count ("Number of items in container"), is_empty.
    #[test]
    fn container_functions_over_a_list() {
        let list = List(vec![1, 2, 2]);
        assert!(list.has(&2));
        assert!(!list.has(&9));
        assert_eq!(list.count(), 3);
        assert!(!list.is_empty());
        assert!(List::<i32>(vec![]).is_empty());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.structures §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/list.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-structure_types.adoc §Class Definitions / list.adoc §List Class
//   confidence: medium
//   todos: 1
//   note: first/last widened to Option<&T> (spec's own "or Void" convention elsewhere, no stated empty-list behaviour here); count()'s i32 cast has no spec-defined overflow behaviour for lists exceeding i32::MAX elements. First_validity/Last_validity invariants are encoded structurally by returning None on empty rather than as a runtime Validate check. P4: canonical JSON uses object form `{_type:"LIST",items?}` so the class definition is schema-coverable without changing the storage type.
// ─────────────────────────────────────────────
