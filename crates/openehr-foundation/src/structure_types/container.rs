//! `Container<T>` — abstract ancestor of container types.
//!
//! openEHR class: `Container<T>` (abstract), package
//! `base.foundation_types.structures`.
//! Inherits: `Any`.
//!
//! Abstract ancestor of container types whose items are addressable in some
//! way. Declares membership/size queries (`has`, `count`, `is_empty`) plus
//! agent-driven quantifiers and selectors (`there_exists`, `for_all`,
//! `matching`, `select`) that every concrete container in this module
//! (`List<T>`, `Set<T>`, `Array<T>`, `Hash<K,V>`) inherits.
use super::super::primitive_types::any::Any;

/// `Container<T>` has no attributes and is the shared abstract ancestor of
/// every concrete container type in this module, so — mirroring the
/// treatment of `Any`/`Ordered`/`Numeric` in `primitive_types`
/// (ADR-001 §1) — it is modelled as a Rust trait rather than a struct,
/// generic over the element type `T` to match the spec's own `Container<T>`
/// generic parameter.
///
/// `has`/`count`/`is_empty` are the spec's `(abstract)` functions (no
/// default body, required of every implementor). `there_exists`/`for_all`/
/// `matching`/`select` have default bodies, matching the spec's own
/// non-`(abstract)` marker on those four; the bodies iterate via the
/// non-spec `items()` accessor required below (ADR-003 decision 6).
///
/// The predicate argument `test: Operation[1]` (signature `(v:T): Boolean`)
/// is transcribed as `impl Fn(&T) -> bool`, the direct Rust closure
/// equivalent of an openEHR agent — see `functional::function` for the
/// `Function<ARGS,RESULT>` meta-type this argument shape approximates.
/// Booleans are transcribed as plain `bool` at this trait-method boundary
/// (not the `Boolean` newtype), matching the established convention in
/// `primitive_types::any::Any` and `primitive_types::ordered::Ordered`.
pub trait Container<T>: Any {
    /// `has` `(v: T[1]) -> Boolean` (abstract).
    ///
    /// Test for membership of a value.
    fn has(&self, v: &T) -> bool;

    /// `count() -> Integer` (abstract).
    ///
    /// Number of items in container.
    fn count(&self) -> i32;

    /// `is_empty() -> Boolean` (abstract).
    ///
    /// True if container is empty.
    fn is_empty(&self) -> bool;

    /// Iteration accessor over the container's items.
    ///
    /// PORT NOTE: not a spec function. The spec declares `there_exists`/
    /// `for_all`/`matching`/`select` non-abstractly on `Container<T>` but
    /// gives the class no iteration primitive to express them with (`has`,
    /// `count`, and `is_empty` are its only abstract functions). Per
    /// ADR-003 decision 6, the Rust trait adds this required accessor so the
    /// four spec functions can be true default methods. Declared as an
    /// iterator rather than the ADR's illustrative `&[T]` shape because two
    /// of the four concrete containers (`Set` over `HashSet`, `Hash` over
    /// `HashMap`) have no contiguous backing storage to lend a slice from;
    /// for unordered backings the iteration order is unspecified, which the
    /// affected concrete impls document at their `items` override.
    fn items<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a;

    /// `there_exists` __alias__ `"there exists"`, `"∃"` `(test: Operation[1]) -> Boolean`.
    ///
    /// Existential quantifier applied to container, taking one agent argument
    /// `test` whose signature is `(v:T): Boolean`.
    fn there_exists(&self, test: impl Fn(&T) -> bool) -> bool {
        self.items().any(test)
    }

    /// `for_all` __alias__ `"for all"`, `"∀"` `(test: Operation[1]) -> Boolean`.
    ///
    /// Universal quantifier applied to container, taking one agent argument
    /// `test` whose signature is `(v:T): Boolean`.
    fn for_all(&self, test: impl Fn(&T) -> bool) -> bool {
        self.items().all(test)
    }

    /// `matching` `(test: Operation[1]) -> List<T>`.
    ///
    /// Return a List of all items matching the predicate function `test`
    /// which has signature `(v:T): Boolean`. If no matches, an empty List is
    /// returned.
    ///
    /// PORT NOTE: `T: Clone` is a Rust-only bound — the spec's `List<T>`
    /// result holds the matching items by (Eiffel) reference, while building
    /// a fresh owned `List<T>` from `&self` requires cloning each match.
    /// Every foundation type in this crate is `Clone`, so nothing is
    /// excluded in practice.
    fn matching(&self, test: impl Fn(&T) -> bool) -> super::list::List<T>
    where
        T: Clone,
    {
        super::list::List(self.items().filter(|v| test(v)).cloned().collect())
    }

    /// `select` `(test: Operation[1]) -> T`.
    ///
    /// Return first item matching the predicate function `test` which has
    /// signature `(v:T): Boolean`, or Void if no match.
    ///
    /// PORT NOTE: the spec's `0..1` cardinality marker plus "or Void if no
    /// match" means the true return type is optional; transcribed as
    /// `Option<T>` rather than a bare `T`, since "Void" in the
    /// openEHR/Eiffel type system is the direct analogue of Rust's `None`.
    /// "First" is well-defined only for ordered containers; over an
    /// unordered backing (`Set`, `Hash`) an arbitrary matching item is
    /// returned — see the `items` PORT NOTE above. `T: Clone` as on
    /// `matching`.
    fn select(&self, test: impl Fn(&T) -> bool) -> Option<T>
    where
        T: Clone,
    {
        self.items().find(|v| test(v)).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::super::list::List;
    use super::Container;

    // Spec: "Existential quantifier applied to container" — true iff at
    // least one item satisfies the test.
    #[test]
    fn there_exists_finds_a_matching_item() {
        let list = List(vec![1, 2, 3]);
        assert!(list.there_exists(|v| *v == 2));
        assert!(!list.there_exists(|v| *v == 9));
        // Vacuously false on an empty container.
        assert!(!List::<i32>(vec![]).there_exists(|_| true));
    }

    // Spec: "Universal quantifier applied to container" — true iff every
    // item satisfies the test (vacuously true when empty).
    #[test]
    fn for_all_requires_every_item_to_match() {
        let list = List(vec![2, 4, 6]);
        assert!(list.for_all(|v| v % 2 == 0));
        assert!(!list.for_all(|v| *v < 6));
        assert!(List::<i32>(vec![]).for_all(|_| false));
    }

    // Spec: "Return a List of all items matching the predicate ... If no
    // matches, an empty List is returned."
    #[test]
    fn matching_returns_all_matches_or_an_empty_list() {
        let list = List(vec![1, 2, 3, 4]);
        assert_eq!(matching_vec(&list, |v| v % 2 == 0), vec![2, 4]);
        assert!(matching_vec(&list, |v| *v > 10).is_empty());
    }

    fn matching_vec(list: &List<i32>, test: impl Fn(&i32) -> bool) -> Vec<i32> {
        list.matching(test).0
    }

    // Spec: "Return first item matching the predicate ... or Void if no
    // match" — Void maps to None.
    #[test]
    fn select_returns_first_match_or_none() {
        let list = List(vec![1, 2, 3, 4]);
        assert_eq!(list.select(|v| v % 2 == 0), Some(2));
        assert_eq!(list.select(|v| *v > 10), None);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.structures §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/container.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-structure_types.adoc §Class Definitions / container.adoc §Container Class
//   confidence: high
//   todos: 0
//   note: there_exists/for_all/matching/select are default methods over a non-spec `items()` iteration accessor added per ADR-003 decision 6 (the spec class declares no iteration primitive of its own); matching/select carry a Rust-only T: Clone bound since they build owned results from &self.
// ─────────────────────────────────────────────
