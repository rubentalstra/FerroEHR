//! `Set<T>` — unordered container that may not contain duplicates.
//!
//! openEHR class: `Set<T>`, package `base.foundation_types.structures`.
//! Inherits: `Container<T>`.
//!
//! Unordered container that may not contain duplicates. Declares no
//! functions or attributes of its own beyond those inherited from
//! `Container<T>`.
use super::super::primitive_types::any::Any;
use super::container::Container;
use serde::de::Error as _;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::hash::Hash;

/// Per `docs/PORTING.md` §14.2 (`Set<T>` → `HashSet<T>`/`BTreeSet<T>`),
/// transcribed as a transparent newtype over `std::collections::HashSet<T>`
/// — unordered, unique-membership, matching the spec's stated semantics
/// exactly. `BTreeSet` (ordered) would over-constrain the spec's explicit
/// "no order" description, so `HashSet` is the more faithful choice of the
/// two `docs/PORTING.md` options.
///
/// `T: Eq + Hash` is required by `HashSet` itself; this is a structural
/// requirement of the chosen backing container, not a spec-declared
/// constraint (the spec's own `Set<T>` has no bound on `T` at all).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Set<T: Eq + Hash>(pub HashSet<T>);

impl<T: Eq + Hash> Container<T> for Set<T> {
    fn has(&self, v: &T) -> bool {
        self.0.contains(v)
    }

    fn count(&self) -> i32 {
        // TODO(port): see `List::count` PORT NOTE — same i32-cast overflow
        // gap, unaddressed by the spec.
        self.0.len() as i32
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iteration accessor required by `Container<T>` (ADR-003 decision 6),
    /// implemented directly over the backing `HashSet`'s own iterator.
    ///
    /// PORT NOTE: a `HashSet` has no contiguous storage and no defined
    /// order, so the inherited `there_exists`/`for_all`/`matching`/`select`
    /// operate in unspecified iteration order here — `select`'s "first
    /// item matching" is an arbitrary matching item for a Set, which is
    /// consistent with the spec's own "unordered container" description of
    /// this class.
    fn items<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.0.iter()
    }
}

impl<T: Eq + Hash> Any for Set<T> {
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn type_of(&self) -> String {
        "Set".to_string()
    }
}

impl<T: Eq + Hash + Serialize> Serialize for Set<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let field_count = if self.0.is_empty() { 1 } else { 2 };
        let mut state = serializer.serialize_struct("SET", field_count)?;
        state.serialize_field("_type", "SET")?;
        if !self.0.is_empty() {
            state.serialize_field("items", &self.0)?;
        }
        state.end()
    }
}

impl<'de, T: Eq + Hash + Deserialize<'de>> Deserialize<'de> for Set<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire<T: Eq + Hash> {
            #[serde(rename = "_type")]
            type_name: Option<String>,
            items: Option<HashSet<T>>,
        }

        let wire = Wire::deserialize(deserializer)?;
        if wire.type_name.as_deref().is_some_and(|name| name != "SET") {
            return Err(D::Error::custom("expected _type \"SET\""));
        }
        Ok(Set(wire.items.unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::container::Container;
    use super::Set;
    use std::collections::HashSet;

    fn set(items: &[i32]) -> Set<i32> {
        Set(items.iter().copied().collect::<HashSet<i32>>())
    }

    // Spec Container functions over the unordered, unique-membership Set.
    #[test]
    fn container_functions_over_a_set() {
        let s = set(&[1, 2, 3]);
        assert!(s.has(&2));
        assert!(!s.has(&9));
        assert_eq!(s.count(), 3);
        assert!(!s.is_empty());
        assert!(set(&[]).is_empty());
    }

    // The inherited quantifiers/selectors work directly over the HashSet
    // backing (ADR-003 decision 6); select returns an arbitrary matching
    // item since a Set is unordered.
    #[test]
    fn quantifiers_and_selectors_over_the_hashset_backing() {
        let s = set(&[1, 2, 3, 4]);
        assert!(s.there_exists(|v| *v == 3));
        assert!(s.for_all(|v| *v >= 1));
        assert!(!s.for_all(|v| v % 2 == 0));
        let evens = s.matching(|v| v % 2 == 0);
        let mut evens_sorted = evens.0.clone();
        evens_sorted.sort_unstable();
        assert_eq!(evens_sorted, vec![2, 4]);
        let selected = s.select(|v| v % 2 == 0);
        assert!(matches!(selected, Some(2 | 4)));
        assert_eq!(s.select(|v| *v > 10), None);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.structures §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/set.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-structure_types.adoc §Class Definitions / set.adoc §Set Class
//   confidence: high
//   todos: 1
//   note: T: Eq + Hash bound is a structural requirement of HashSet, not a spec-declared constraint (Set<T> itself is unconstrained); count()'s i32 cast shares List's unspecified-overflow gap. P4: canonical JSON uses object form `{_type:"SET",items?}` so the class definition is schema-coverable without changing the storage type.
// ─────────────────────────────────────────────
