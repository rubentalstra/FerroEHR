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
/// `matching`/`select` have default bodies since the spec expresses them
/// purely in terms of the abstract functions plus an `Operation` agent
/// argument, matching the spec's own `1..1` (non-abstract) cardinality
/// marker on those four.
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

    /// `there_exists` __alias__ `"there exists"`, `"∃"` `(test: Operation[1]) -> Boolean`.
    ///
    /// Existential quantifier applied to container, taking one agent argument
    /// `test` whose signature is `(v:T): Boolean`.
    ///
    /// TODO(port): the spec declares this function non-abstractly at the
    /// `Container<T>` level with no stated implementation strategy beyond
    /// "existential quantifier"; a faithful default body requires an
    /// iteration capability that `Container<T>` itself does not declare (no
    /// `iterate`/`for_each` function exists in this class — `has`, `count`,
    /// and `is_empty` are the only abstract primitives available to build
    /// one from). Left unimplemented pending a decision on whether this
    /// trait should also require an iterator-producing method, or whether
    /// each concrete container (`List`, `Set`, `Array`, `Hash`) should
    /// override this default individually using its own std-container
    /// iteration.
    fn there_exists(&self, test: impl Fn(&T) -> bool) -> bool {
        let _ = test;
        todo!("Container::there_exists: no iteration primitive declared on Container<T> itself")
    }

    /// `for_all` __alias__ `"for all"`, `"∀"` `(test: Operation[1]) -> Boolean`.
    ///
    /// Universal quantifier applied to container, taking one agent argument
    /// `test` whose signature is `(v:T): Boolean`.
    ///
    /// TODO(port): see `there_exists` above — same missing-iteration-
    /// primitive gap applies here.
    fn for_all(&self, test: impl Fn(&T) -> bool) -> bool {
        let _ = test;
        todo!("Container::for_all: no iteration primitive declared on Container<T> itself")
    }

    /// `matching` `(test: Operation[1]) -> List<T>`.
    ///
    /// Return a List of all items matching the predicate function `test`
    /// which has signature `(v:T): Boolean`. If no matches, an empty List is
    /// returned.
    ///
    /// TODO(port): same missing-iteration-primitive gap as `there_exists`;
    /// additionally depends on `List<T>` (`super::list::List`), transcribed
    /// alongside this file.
    fn matching(&self, test: impl Fn(&T) -> bool) -> super::list::List<T> {
        let _ = test;
        todo!("Container::matching: no iteration primitive declared on Container<T> itself")
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
    ///
    /// TODO(port): same missing-iteration-primitive gap as `there_exists`.
    fn select(&self, test: impl Fn(&T) -> bool) -> Option<T> {
        let _ = test;
        todo!("Container::select: no iteration primitive declared on Container<T> itself")
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.structures §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/container.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-structure_types.adoc §Class Definitions / container.adoc §Container Class
//   confidence: medium
//   todos: 4
//   note: there_exists/for_all/matching/select all need an iteration primitive Container<T> itself does not declare; each concrete container (List/Set/Array/Hash) should likely override these defaults using its own std-container iterator once the crate reaches a compiling phase, rather than relying on this trait's todo!() default.
// ─────────────────────────────────────────────
