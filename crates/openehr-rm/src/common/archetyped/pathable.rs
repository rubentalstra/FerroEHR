//! `PATHABLE` — pathing capabilities used by nearly all RM classes.
//!
//! openEHR class: `PATHABLE` (abstract), package `common.archetyped`.
//! Inherits: `Any` (BASE foundation_types).
//!
//! The `PATHABLE` class defines the pathing capabilities used by nearly
//! all classes in the openEHR reference model, mostly via inheritance of
//! `LOCATABLE`. The defining characteristics of `PATHABLE` objects are
//! that they can locate child objects using paths, and they know their
//! parent object in a compositional hierarchy. The `parent` feature is
//! defined as abstract in the model, and may be implemented in any way
//! convenient.
//!
//! # `PATHABLE.parent()` — the reverse-pointer hazard, resolved here
//!
//! This file is the reference implementation of the
//! `.claude/rules/rm-transcription.md` rule "`PATHABLE.parent()` and any
//! other reverse pointer use `Weak<..>` or a path-index — never an owning
//! back-reference" (restated from `PORT_MASTER_PLAN.md` §7.2 and §14.4,
//! and generalized as ADR-001 §8). Every later class that is `PATHABLE`
//! (directly, or via `LOCATABLE`) must model its parent link the same way
//! this trait does — do not re-derive the decision per class.
//!
//! **Why `parent()` cannot be an owning reference.** The compositional
//! hierarchy the spec describes is a tree of `PATHABLE` nodes where each
//! child holds (directly or transitively) a *containing* reference to its
//! parent — e.g. an `ELEMENT` inside a `CLUSTER` inside an `ITEM_TREE`
//! inside a `COMPOSITION`. If `parent()` returned an owned value or an
//! owning smart pointer (`Rc<PATHABLE>`/`Box<PATHABLE>`) back up the same
//! tree the child is already reachable from via its owner's `Vec`/`Box`
//! field, the object graph becomes a reference cycle: parent owns child,
//! child (transitively) owns parent. In Rust this either fails to compile
//! (a `Box` cannot alias) or, with `Rc`, leaks forever (`Rc` cycles are
//! never collected) — neither is acceptable for a long-lived, EHR-scale,
//! deeply-nested tree.
//!
//! **The two sanctioned shapes**, per the rule text ("`Weak<..>` or a
//! path-index"):
//!
//! 1. **`std::sync::Weak<T>` back-reference.** The parent is genuinely
//!    reachable as a live Rust reference, but as a non-owning one: the
//!    tree's forward edges (parent → child) are `Arc<T>` (or an owned
//!    field, if the child never needs to outlive its parent), and the
//!    reverse edge (child → parent) is `Weak<T>`, upgraded to
//!    `Option<Arc<T>>` at the point of use via [`Weak::upgrade`]. This is
//!    the shape already established for other RM/BASE back-references,
//!    e.g. `RESOURCE_DESCRIPTION.parent_resource` in
//!    `openehr-base::resource::resource_description` (`Weak<AuthoredResource>`).
//! 2. **Path-index lookup.** The parent is not held as a live reference at
//!    all; instead the object graph is addressed by an external index
//!    (e.g. a `HashMap<Path, NodeId>` or an arena with typed indices), and
//!    "parent" becomes "look up the entry one path segment shorter than
//!    mine" rather than a stored pointer of any kind. This shape suits
//!    contexts where the whole tree is deserialized as an owned value
//!    graph (e.g. `serde_json::Value`-shaped trees, or an
//!    `openehr-flat`/`openehr-serde` intermediate representation) with no
//!    natural place to embed a `Weak` at all.
//!
//! **Decision for this trait.** `PATHABLE` is a bare abstract class (no
//! attributes at all — see the spec table's empty `Attributes` section);
//! it exists purely to declare the five functions below as an interface.
//! Per ADR-001 §1 (abstract class with no attributes → trait), it is
//! transcribed as [`PathableApi`], a trait with no associated state.
//! Because this trait cannot itself know which of the two sanctioned
//! shapes a given implementor will use — a `COMPOSITION` root has no
//! parent at all (`Option::None`/no upgrade target), while an `ELEMENT`
//! deep in a tree has a concrete, addressable parent — `parent()` is
//! specified to return `Option<Weak<dyn PathableApi>>`:
//!
//! * `Weak<dyn PathableApi>` (not `Weak<Self>` or a concrete type)
//!   because the spec's own signature returns `PATHABLE` — the abstract
//!   supertype, not the concrete parent's own type. A `COMPOSITION`'s
//!   child `SECTION`'s parent is typed `PATHABLE` in the spec precisely so
//!   callers do not need to know the parent's concrete class to walk
//!   upward. `dyn PathableApi` is the direct Rust rendering of that
//!   abstract, polymorphic return type; using a trait object here is not a
//!   departure from "closed enum over trait object" (ADR-001 §4) because
//!   the *closed set* of `PATHABLE` implementors is enormous and spans
//!   every RM package (`LOCATABLE` alone is inherited by dozens of
//!   classes, plus the direct-`PATHABLE` exceptions `EVENT_CONTEXT`,
//!   `INSTRUCTION_DETAILS`, `ISM_TRANSITION`) — collecting all of them
//!   into one giant enum defeats the purpose of per-package modularity and
//!   would force every RM package to depend on every other RM package just
//!   to name the variant. This is the one place in the RM where an open,
//!   Rust-trait-object shape is the faithful rendering of the spec's own
//!   abstract polymorphic type, not an ADR-001 exception.
//! * `Option<..>` because the spec's own cardinality on `parent()` is
//!   `1..1` (always present) — but that cardinality assumes every
//!   `PATHABLE` is already attached somewhere in a tree. In practice the
//!   root of a compositional hierarchy (e.g. a freshly-constructed
//!   `COMPOSITION` before it is attached to anything, or any node
//!   constructed standalone for testing) has no parent to report. `Weak`
//!   itself already models "may no longer be reachable" via
//!   [`Weak::upgrade`] returning `None`, so wrapping in `Option` at the
//!   trait-method level captures the distinct, structural "never had a
//!   parent to begin with" case (root-of-tree) without conflating it with
//!   "had a parent that has since been dropped" (`Weak::upgrade() ==
//!   None`) — both are real, distinguishable states for a long-lived RM
//!   object graph.
//!
//! Concrete implementors that follow the path-index shape instead (e.g. an
//! in-memory query/AQL engine walking a deserialized JSON tree with no
//! natural `Weak` targets) are free to implement [`PathableApi::parent`]
//! by resolving through their own index and wrapping the result the same
//! way — the trait signature does not mandate which storage a concrete
//! type actually uses, only that whichever shape is chosen is non-owning.
//!
//! All five functions are declared with `todo!()` bodies here — `PATHABLE`
//! has no state to operate on by itself; the actual path-resolution logic
//! only becomes implementable once concrete `LOCATABLE` descendants with
//! real child collections exist (RM data_structures phase).
use std::sync::Weak;

/// Behaviour trait for `PATHABLE` and every RM class that inherits it,
/// directly or (far more commonly) via `LOCATABLE`
/// ([`super::locatable::LocatableApi`]).
///
/// See the module-level documentation above for the full reasoning behind
/// the `parent()` signature — this is the settled hazard from
/// `PORT_MASTER_PLAN.md` §7.2 ("`PATHABLE.parent()` reverse pointer: do
/// not use owning back-references; use `Weak` or path-index lookup").
pub trait PathableApi {
    /// `parent(): PATHABLE`, cardinality `1..1` in the spec.
    ///
    /// Parent of this node in a compositional hierarchy.
    ///
    /// Widened to `Option<Weak<dyn PathableApi>>` in this transcription:
    /// `Weak` for the non-owning back-reference (never an owning
    /// reference, per the reverse-pointer rule), `dyn PathableApi` because
    /// the spec's own return type is the abstract `PATHABLE` supertype
    /// (not a concrete sibling type), and the outer `Option` for the
    /// root-of-tree case, which has no parent to report at all. See the
    /// module doc for the full justification of each layer.
    ///
    /// TODO(port): body is `todo!()` — `PATHABLE` itself carries no state;
    /// each concrete implementor stores its own parent link (as `Weak<..>`
    /// or via a path-index) and must override this method once RM
    /// data_structures/common concrete types with real containment exist.
    fn parent(&self) -> Option<Weak<dyn PathableApi>> {
        todo!(
            "PathableApi::parent: no default implementation — every concrete PATHABLE stores its own parent link"
        )
    }

    /// `item_at_path(a_path: String): Any`, cardinality `1..1`.
    ///
    /// Pre: `path_unique(a_path)`.
    ///
    /// The item at a path (relative to this item); only valid for unique
    /// paths, i.e. paths that resolve to a single item.
    ///
    /// Widened to `Option<..>` for the same reason as
    /// [`items_at_path`](PathableApi::items_at_path): a caller that
    /// violates the `Pre` clause (calls this on a non-unique or
    /// non-existent path) gets `None` rather than a panic, since RM
    /// invariant/precondition violations are modelled as `Validate`
    /// failures per `.claude/rules/rm-transcription.md` "Invariants"
    /// rather than as Rust panics.
    ///
    /// The spec types the return `Any` (BASE foundation_types) — the root
    /// marker trait, since a resolved path item could be any RM type at
    /// all (a `DATA_VALUE`, an `ITEM`, a nested `LOCATABLE`, ...).
    /// Rendered here as `Box<dyn std::any::Any>` for the same open-set
    /// reasoning as `parent()`'s `dyn PathableApi`: the set of things a
    /// path can resolve to spans the entire RM, so no closed enum can
    /// enumerate it without inverting every crate dependency arrow.
    ///
    /// TODO(port): path resolution against a concrete containment tree is
    /// not implementable until RM data_structures/common concrete types
    /// exist; `todo!()` for now.
    fn item_at_path(&self, a_path: &str) -> Option<Box<dyn std::any::Any>> {
        let _ = a_path;
        todo!("PathableApi::item_at_path: path resolution needs concrete RM data_structures")
    }

    /// `items_at_path(a_path: String): List<Any>`, cardinality `0..1`.
    ///
    /// List of items corresponding to a non-unique path.
    ///
    /// The spec's own cardinality on this function is `0..1`, i.e. the
    /// function itself may return nothing (Void) — modelled directly as
    /// `Vec<..>` being empty in that case, rather than doubling up with an
    /// `Option<Vec<..>>` (an empty `Vec` and "no items" are the same
    /// observable state for a list-typed result, unlike the `Option`
    /// fields on `LOCATABLE`'s attributes where `None` vs `Some(empty)`
    /// are kept distinct per the `Links_valid`-style invariants — see
    /// `locatable.rs`).
    ///
    /// TODO(port): see [`item_at_path`](PathableApi::item_at_path).
    fn items_at_path(&self, a_path: &str) -> Vec<Box<dyn std::any::Any>> {
        let _ = a_path;
        todo!("PathableApi::items_at_path: path resolution needs concrete RM data_structures")
    }

    /// `path_exists(a_path: String): Boolean`, cardinality `1..1`.
    ///
    /// Pre: `not a_path.is_empty`.
    ///
    /// True if the path exists in the data with respect to the current
    /// item.
    ///
    /// TODO(port): needs a concrete containment tree to walk; `todo!()`
    /// for now.
    fn path_exists(&self, a_path: &str) -> bool {
        let _ = a_path;
        todo!("PathableApi::path_exists: path resolution needs concrete RM data_structures")
    }

    /// `path_unique(a_path: String): Boolean`, cardinality `1..1`.
    ///
    /// Pre: `path_exists(a_path)`.
    ///
    /// True if the path corresponds to a single item in the data.
    ///
    /// TODO(port): needs a concrete containment tree to walk; `todo!()`
    /// for now.
    fn path_unique(&self, a_path: &str) -> bool {
        let _ = a_path;
        todo!("PathableApi::path_unique: path resolution needs concrete RM data_structures")
    }

    /// `path_of_item(a_loc: PATHABLE): String`, cardinality `1..1`.
    ///
    /// The path to an item relative to the root of this archetyped
    /// structure.
    ///
    /// The spec parameter type `PATHABLE` is rendered as `&dyn
    /// PathableApi`, matching the same open-polymorphism reasoning as
    /// `parent()`'s return type.
    ///
    /// TODO(port): needs a concrete containment tree to walk (to compute
    /// the path from this node down to `a_loc`); `todo!()` for now.
    fn path_of_item(&self, a_loc: &dyn PathableApi) -> String {
        let _ = a_loc;
        todo!("PathableApi::path_of_item: path resolution needs concrete RM data_structures")
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.archetyped — docs/research/spec-cache/RM-1.1.0/uml_classes/pathable.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master03-archetyped_package.adoc §The PATHABLE Class / uml_classes/pathable.adoc §PATHABLE Class
//   confidence: high
//   todos: 5
//   note: Reference implementation of the PATHABLE.parent() reverse-pointer hazard (rm-transcription rule + ADR-001 §8) — parent() returns Option<Weak<dyn PathableApi>>; all five function bodies are todo!() pending concrete RM data_structures to walk. Every LOCATABLE-embedding class in later phases must reuse this trait, not re-derive the parent-pointer shape.
// ─────────────────────────────────────────────
