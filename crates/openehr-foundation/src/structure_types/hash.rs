//! `Hash<K,V>` — a table of values keyed by an `Ordered` descendant.
//!
//! openEHR class: `Hash<K,V>`, package `base.foundation_types.structures`.
//! Inherits: `Container<V>`.
//!
//! Type representing a keyed table of values. `V` is the value type, and `K`
//! the type of the keys. Per the `structure_types` chapter overview: `K` is
//! typically `String` or `Integer` but may be more complex `Ordered`
//! descendants, e.g. a coded term type.
use super::super::primitive_types::any::Any;
use super::super::primitive_types::ordered::Ordered;
use super::container::Container;
use std::collections::HashMap;
use std::hash::Hash as StdHash;

/// Rust type name: `OpenEhrHash`, **not** `Hash`.
///
/// PORT NOTE: the openEHR class is literally named `Hash`, but transcribing
/// it as a struct literally called `Hash` in the same module tree as every
/// other foundation type risks being read as `std::hash::Hash` (the
/// near-ubiquitous derive-macro trait used throughout this very crate, e.g.
/// `#[derive(..., Hash)]` on `List`/`Set`/`Array` in this same directory).
/// The two do not actually collide in Rust's namespaces (one is a trait, one
/// would be a struct, and `#[derive(Hash)]` always resolves to
/// `core::hash::Hash` regardless of local `use` statements) but the reading
/// confusion is exactly the kind of hazard the `String` → `OpenEhrString`
/// precedent (`primitive_types::string`) was recorded to avoid. Renamed here
/// for the same reason and recorded in `docs/ROSETTA.md` so this naming
/// decision is not relitigated by a later transcriber.
///
/// Per `docs/PORTING.md` §14.2 (`Map<K,V>` → `HashMap<K,V>`/`BTreeMap<K,V>`),
/// transcribed as a transparent newtype over
/// `std::collections::HashMap<K, V>`.
///
/// The spec constrains `K` to an `Ordered` descendant (`Hash<K:Ordered, V>`);
/// transcribed as a Rust trait bound `K: Ordered` per the constrained-generic
/// rule (ADR-001 §5). `HashMap` itself additionally requires `K: Eq +
/// std::hash::Hash` structurally — this is a Rust-container requirement
/// layered on top of the spec's own `Ordered` constraint, not a replacement
/// for it; both bounds are carried here rather than silently dropping the
/// spec's declared constraint in favour of only the structural one.
#[derive(Debug, Clone, Default)]
#[repr(transparent)]
pub struct OpenEhrHash<K: Ordered + Eq + StdHash, V>(pub HashMap<K, V>);

impl<K: Ordered + Eq + StdHash, V> OpenEhrHash<K, V> {
    /// `has_key` `(a_key: K[1]) -> Boolean`.
    ///
    /// Test for presence of `a_key`.
    pub fn has_key(&self, a_key: &K) -> bool {
        self.0.contains_key(a_key)
    }

    /// `item` __alias__ `"[]"` `(a_key: K[1]) -> V`.
    ///
    /// Return item for key `a_key`.
    ///
    /// PORT NOTE: the spec types this as a bare `V`, but does not state the
    /// result of looking up a key that is not present (`has_key(a_key)`
    /// false); widened to `Option<&V>` rather than a bare `&V`/panic, matching
    /// the same "or Void"-style widening applied to `List::first`/`last` and
    /// `Container::select` elsewhere in this cluster.
    pub fn item(&self, a_key: &K) -> Option<&V> {
        self.0.get(a_key)
    }
}

impl<K: Ordered + Eq + StdHash, V: PartialEq> Container<V> for OpenEhrHash<K, V> {
    /// `has` `(v: V[1]) -> Boolean` (abstract, inherited from `Container<V>`).
    ///
    /// PORT NOTE: `Container<V>::has` tests for membership of a *value*
    /// `v: T` (here `T = V`), which is a different query from this class's
    /// own `has_key` (tests for a *key* `K`). Both are transcribed distinctly
    /// per their own spec signatures — `has` here scans map values, `has_key`
    /// above scans map keys.
    fn has(&self, v: &V) -> bool {
        self.0.values().any(|value| value == v)
    }

    fn count(&self) -> i32 {
        // TODO(port): see `List::count` PORT NOTE — same i32-cast overflow
        // gap, unaddressed by the spec.
        self.0.len() as i32
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iteration accessor required by `Container<V>` (ADR-003 decision 6),
    /// implemented directly over the backing `HashMap`'s value iterator.
    ///
    /// PORT NOTE: `Hash<K,V>` inherits `Container<V>` (the *values* are the
    /// contained items), so iteration is over values; a `HashMap` has no
    /// defined order, so the inherited quantifiers/selectors operate in
    /// unspecified iteration order here (same note as `Set::items`).
    fn items<'a>(&'a self) -> impl Iterator<Item = &'a V>
    where
        V: 'a,
    {
        self.0.values()
    }
}

impl<K: Ordered + Eq + StdHash, V: PartialEq> Any for OpenEhrHash<K, V> {
    fn is_equal(&self, other: &Self) -> bool {
        // TODO(port): `HashMap<K, V>` requires `V: PartialEq` for a
        // structural `==`, which std's own `HashMap` derives conditionally;
        // implemented here via `PartialEq` on `V` directly since `HashMap`
        // itself does not derive `PartialEq`/`Eq` unconditionally, only when
        // both K and V support it. Equivalent in effect to `self.0 ==
        // other.0` once `HashMap: PartialEq` is satisfied for these type
        // parameters; kept as an explicit comparison here to document why
        // the bound is not carried on the struct's own `#[derive]` line
        // (K/V are only known at `impl` time in this generic context).
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .all(|(k, v)| other.0.get(k).is_some_and(|ov| ov == v))
    }

    fn type_of(&self) -> String {
        "Hash".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::super::container::Container;
    use super::OpenEhrHash;
    use crate::primitive_types::string::OpenEhrString;
    use std::collections::HashMap;

    fn sample() -> OpenEhrHash<OpenEhrString, i32> {
        let mut map = HashMap::new();
        map.insert(OpenEhrString("one".to_string()), 1);
        map.insert(OpenEhrString("two".to_string()), 2);
        OpenEhrHash(map)
    }

    // Spec: has_key "Test for presence of a_key"; item "Return item for key
    // a_key" (missing key widened to None per the PORT NOTE on the method).
    #[test]
    fn has_key_and_item_look_up_by_key() {
        let hash = sample();
        let one = OpenEhrString("one".to_string());
        let three = OpenEhrString("three".to_string());
        assert!(hash.has_key(&one));
        assert!(!hash.has_key(&three));
        assert_eq!(hash.item(&one), Some(&1));
        assert_eq!(hash.item(&three), None);
    }

    // Spec Container<V> functions: `has` tests membership of a *value*
    // (distinct from has_key), count/is_empty, plus the inherited
    // quantifiers over the HashMap's values.
    #[test]
    fn container_functions_operate_over_values() {
        let hash = sample();
        assert!(hash.has(&2));
        assert!(!hash.has(&9));
        assert_eq!(hash.count(), 2);
        assert!(!hash.is_empty());
        assert!(hash.there_exists(|v| *v == 1));
        assert!(hash.for_all(|v| *v >= 1));
        assert_eq!(hash.select(|v| *v > 10), None);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.structures §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/hash.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-structure_types.adoc §Class Definitions / hash.adoc §Hash Class
//   confidence: medium
//   todos: 2
//   note: named OpenEhrHash (not Hash) to avoid reading confusion with std::hash::Hash, mirroring the OpenEhrString precedent — recorded in ROSETTA. K bound carries both the spec's Ordered constraint and HashMap's structural Eq+StdHash requirement. item()'s missing-key case and count()'s i32 cast share the same unspecified-edge-case gaps as List/Set/Array.
// ─────────────────────────────────────────────
