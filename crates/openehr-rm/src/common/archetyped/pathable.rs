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
//! # How the five path functions get real bodies on an abstract class
//!
//! `PATHABLE` carries no state, and Rust has no runtime reflection to
//! enumerate an arbitrary implementor's attributes. The Rust rendering
//! therefore splits the trait in two layers:
//!
//! * **Traversal hooks** ([`PathableApi::as_pathable`],
//!   [`PathableApi::path_attribute_names`],
//!   [`PathableApi::path_child_nodes`], [`PathableApi::path_node_id`],
//!   [`PathableApi::path_node_name`]) — the reflection substitute. They
//!   are *not* spec features (each carries a PORT NOTE); a concrete class
//!   opts into path resolution by overriding them to expose its own
//!   `PATHABLE`-typed children per attribute, plus its
//!   `archetype_node_id`/`name` for predicate matching. The hooks default
//!   to "no children / no identifiers", so a class that has not opted in
//!   simply resolves no paths below itself rather than failing to compile.
//! * **The five spec functions** — now *working default methods* over the
//!   hooks, delegating to the parser/evaluator in [`super::paths`]
//!   (`RmPath::parse`, `resolve`, `path_of_descendant`). Implementors
//!   inherit them as-is; only the hooks (and `parent()`) need per-class
//!   code.
//!
//! TODO(port): the traversal-hook overrides for the concrete RM classes
//! (`COMPOSITION.content`, `SECTION.items`, `CLUSTER.items`,
//! `HISTORY.events`, ...) are wired across the RM at P17 (make-it-compile),
//! when the crate-wide `impl PathableApi for ..` pass happens; this file
//! deliberately touches no other class.
use std::sync::Weak;

use super::paths;

/// Behaviour trait for `PATHABLE` and every RM class that inherits it,
/// directly or (far more commonly) via `LOCATABLE`
/// ([`super::locatable::LocatableApi`]).
///
/// See the module-level documentation above for the full reasoning behind
/// the `parent()` signature — this is the settled hazard from
/// `PORT_MASTER_PLAN.md` §7.2 ("`PATHABLE.parent()` reverse pointer: do
/// not use owning back-references; use `Weak` or path-index lookup") — and
/// for the traversal-hook design behind the five path functions.
pub trait PathableApi {
    // ── Traversal hooks (not spec features) ─────────────────────────────

    /// Upcast to the abstract `PATHABLE` trait object.
    ///
    /// PORT NOTE: not a spec function — the Rust seam that lets the
    /// default methods below hand `self` to the shared evaluator in
    /// [`super::paths`] (an unsized coercion `&Self → &dyn PathableApi`
    /// requires `Self: Sized`, which a default method body cannot assume).
    /// Every implementor writes the same one-liner: `fn as_pathable(&self)
    /// -> &dyn PathableApi { self }`.
    fn as_pathable(&self) -> &dyn PathableApi;

    /// The attribute names this node can be descended through, in
    /// declaration order (e.g. `["content"]` on a `COMPOSITION`,
    /// `["items"]` on a `CLUSTER`, `["data", "state", "protocol"]` on an
    /// `OBSERVATION` event).
    ///
    /// PORT NOTE: not a spec function — the reflection substitute that
    /// lets [`path_of_item`](PathableApi::path_of_item) search the whole
    /// containment tree without knowing each class's shape. Must name
    /// exactly the attributes [`path_child_nodes`](PathableApi::path_child_nodes)
    /// answers for. Defaults to none (leaf node).
    fn path_attribute_names(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// The `PATHABLE` children reachable from this node via `attribute`,
    /// in document order: a multiply-valued attribute (`items`, `events`,
    /// `content`) yields each element; a single-valued `PATHABLE`
    /// attribute yields one node; anything else yields none.
    ///
    /// PORT NOTE: not a spec function — the reflection substitute the
    /// path evaluator walks. Defaults to no children (leaf node), so a
    /// class that has not yet opted in resolves no paths below itself.
    /// Children that are not `PATHABLE` (leaf `DATA_VALUE` attributes such
    /// as `ELEMENT.value`) are out of range for this evaluator — see the
    /// note on [`item_at_path`](PathableApi::item_at_path).
    fn path_child_nodes(&self, attribute: &str) -> Vec<&dyn PathableApi> {
        let _ = attribute;
        Vec::new()
    }

    /// The archetype node id this node matches in a path predicate
    /// (`LOCATABLE.archetype_node_id`: an at-code, or the stringified
    /// archetype id at an archetype root point).
    ///
    /// PORT NOTE: not a spec function — predicate-matching hook. Defaults
    /// to `None`, which is exactly right for the bare-`PATHABLE` trio
    /// (`EVENT_CONTEXT`, `INSTRUCTION_DETAILS`, `ISM_TRANSITION`) that has
    /// no `archetype_node_id` attribute at all; `LOCATABLE` descendants
    /// override it (see `LocatableData::path_node_id`).
    fn path_node_id(&self) -> Option<&str> {
        None
    }

    /// The runtime name this node matches in a `'name'` path predicate
    /// (`LOCATABLE.name.value`).
    ///
    /// PORT NOTE: not a spec function — predicate-matching hook, same
    /// story as [`path_node_id`](PathableApi::path_node_id).
    fn path_node_name(&self) -> Option<&str> {
        None
    }

    // ── Spec functions ──────────────────────────────────────────────────

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
    /// Required (no default body): the spec itself declares `parent` as
    /// the one abstract feature ("may be implemented in any way
    /// convenient") — `PATHABLE` has no state, so every concrete
    /// implementor supplies its own link. `LOCATABLE` descendants delegate
    /// to `LocatableData::parent_pathable()` (see `locatable.rs`);
    /// path-index implementors resolve through their index.
    fn parent(&self) -> Option<Weak<dyn PathableApi>>;

    /// `item_at_path(a_path: String): Any`, cardinality `1..1`.
    ///
    /// Pre: `path_unique(a_path)`.
    ///
    /// The item at a path (relative to this item); only valid for unique
    /// paths, i.e. paths that resolve to a single item.
    ///
    /// Widened to `Option<..>`: a caller that violates the `Pre` clause
    /// (calls this on a non-unique, non-existent, or unparseable path)
    /// gets `None` rather than a panic, since RM invariant/precondition
    /// violations are modelled as fallible results per
    /// `.claude/rules/rm-transcription.md` "Invariants" rather than as
    /// Rust panics.
    ///
    /// PORT NOTE: the spec types the return `Any` (BASE foundation_types)
    /// — the root marker trait, since a resolved path item could be any RM
    /// type at all. This evaluator ranges over the compositional hierarchy
    /// of `PATHABLE` nodes (the only thing
    /// [`path_child_nodes`](PathableApi::path_child_nodes) can expose
    /// polymorphically), so the return is narrowed to
    /// `&dyn PathableApi`. Paths terminating in a non-`PATHABLE` leaf
    /// value (`ELEMENT.value` and other `DATA_VALUE` attributes) are the
    /// AQL path engine's territory. TODO(port): leaf `DATA_VALUE` path
    /// resolution lands with the AQL semantic path analysis at P12.
    fn item_at_path(&self, a_path: &str) -> Option<&dyn PathableApi> {
        let path = paths::RmPath::parse(a_path).ok()?;
        let mut matches = paths::resolve(self.as_pathable(), &path);
        if matches.len() == 1 {
            matches.pop()
        } else {
            None
        }
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
    /// `locatable.rs`). An unparseable path likewise yields the empty
    /// list (precondition violation, not a panic).
    ///
    /// Return element type narrowed from the spec's `Any` to
    /// `&dyn PathableApi` — see
    /// [`item_at_path`](PathableApi::item_at_path)'s PORT NOTE.
    fn items_at_path(&self, a_path: &str) -> Vec<&dyn PathableApi> {
        match paths::RmPath::parse(a_path) {
            Ok(path) => paths::resolve(self.as_pathable(), &path),
            Err(_) => Vec::new(),
        }
    }

    /// `path_exists(a_path: String): Boolean`, cardinality `1..1`.
    ///
    /// Pre: `not a_path.is_empty`.
    ///
    /// True if the path exists in the data with respect to the current
    /// item. A path that violates the precondition (empty) or does not
    /// parse yields `false`.
    fn path_exists(&self, a_path: &str) -> bool {
        !self.items_at_path(a_path).is_empty()
    }

    /// `path_unique(a_path: String): Boolean`, cardinality `1..1`.
    ///
    /// Pre: `path_exists(a_path)`.
    ///
    /// True if the path corresponds to a single item in the data. A path
    /// that violates the precondition (does not exist) yields `false`.
    fn path_unique(&self, a_path: &str) -> bool {
        self.items_at_path(a_path).len() == 1
    }

    /// `path_of_item(a_loc: PATHABLE): String`, cardinality `1..1`.
    ///
    /// The path to an item relative to the root of this archetyped
    /// structure — computed here as the path of `a_loc` relative to
    /// `self`, treating `self` as that root (the receiver *is* the
    /// structure the spec says the path is relative to).
    ///
    /// The spec parameter type `PATHABLE` is rendered as `&dyn
    /// PathableApi`, matching the same open-polymorphism reasoning as
    /// `parent()`'s return type. Item identity is by address, never by
    /// structural equality (two value-equal sibling `ELEMENT`s have
    /// distinct paths).
    ///
    /// Widened to `Option<String>`: the spec's `1..1` return assumes
    /// `a_loc` is inside this structure; an `a_loc` that is neither `self`
    /// nor reachable below it yields `None` (precondition violation, not
    /// a panic).
    fn path_of_item(&self, a_loc: &dyn PathableApi) -> Option<String> {
        paths::path_of_descendant(self.as_pathable(), a_loc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal local tree type opting into the traversal hooks — stands in
    /// for the concrete RM classes whose hook overrides are wired at P17.
    /// Shaped after a blood-pressure `OBSERVATION`:
    ///
    /// ```text
    /// root [openEHR-EHR-OBSERVATION.bp.v1] 'Blood pressure'
    /// └── data [at0001] 'History'
    ///     ├── events [at0006] 'First'
    ///     │   └── data [at0003]
    ///     │       ├── items [at0004] 'Systolic'
    ///     │       └── items [at0005] 'Diastolic'
    ///     └── events [at0006] 'Second'
    ///         └── data [at0003]
    ///             └── items [at0004] 'Systolic'
    /// ```
    struct TestNode {
        node_id: Option<&'static str>,
        name: Option<&'static str>,
        attrs: Vec<(&'static str, Vec<TestNode>)>,
    }

    impl TestNode {
        fn new(
            node_id: Option<&'static str>,
            name: Option<&'static str>,
            attrs: Vec<(&'static str, Vec<TestNode>)>,
        ) -> Self {
            TestNode {
                node_id,
                name,
                attrs,
            }
        }
    }

    impl PathableApi for TestNode {
        fn as_pathable(&self) -> &dyn PathableApi {
            self
        }

        fn parent(&self) -> Option<Weak<dyn PathableApi>> {
            None // standalone test tree; parent links are exercised in locatable.rs
        }

        fn path_attribute_names(&self) -> Vec<&'static str> {
            self.attrs.iter().map(|(attr, _)| *attr).collect()
        }

        fn path_child_nodes(&self, attribute: &str) -> Vec<&dyn PathableApi> {
            self.attrs
                .iter()
                .filter(|(attr, _)| *attr == attribute)
                .flat_map(|(_, children)| children.iter().map(|c| c as &dyn PathableApi))
                .collect()
        }

        fn path_node_id(&self) -> Option<&str> {
            self.node_id
        }

        fn path_node_name(&self) -> Option<&str> {
            self.name
        }
    }

    fn element(id: &'static str, name: &'static str) -> TestNode {
        TestNode::new(Some(id), Some(name), Vec::new())
    }

    fn bp_tree() -> TestNode {
        let first_event = TestNode::new(
            Some("at0006"),
            Some("First"),
            vec![(
                "data",
                vec![TestNode::new(
                    Some("at0003"),
                    None,
                    vec![(
                        "items",
                        vec![
                            element("at0004", "Systolic"),
                            element("at0005", "Diastolic"),
                        ],
                    )],
                )],
            )],
        );
        let second_event = TestNode::new(
            Some("at0006"),
            Some("Second"),
            vec![(
                "data",
                vec![TestNode::new(
                    Some("at0003"),
                    None,
                    vec![("items", vec![element("at0004", "Systolic")])],
                )],
            )],
        );
        TestNode::new(
            Some("openEHR-EHR-OBSERVATION.bp.v1"),
            Some("Blood pressure"),
            vec![(
                "data",
                vec![TestNode::new(
                    Some("at0001"),
                    Some("History"),
                    vec![("events", vec![first_event, second_event])],
                )],
            )],
        )
    }

    #[test]
    fn root_path_resolves_to_self() {
        let root = bp_tree();
        let item = root.item_at_path("/").expect("root path resolves");
        assert!(std::ptr::addr_eq(
            item as *const dyn PathableApi,
            &root as &dyn PathableApi as *const dyn PathableApi
        ));
        assert!(root.path_exists("/"));
        assert!(root.path_unique("/"));
    }

    #[test]
    fn path_exists_matches_attribute_and_at_code() {
        let root = bp_tree();
        assert!(root.path_exists("/data[at0001]"));
        assert!(root.path_exists("/data[at0001]/events[at0006]"));
        assert!(root.path_exists("/data")); // no predicate: any child via `data`
        assert!(!root.path_exists("/data[at9999]"));
        assert!(!root.path_exists("/state")); // unknown attribute
        assert!(!root.path_exists("")); // precondition violation → false
    }

    #[test]
    fn items_at_path_returns_all_matches() {
        let root = bp_tree();
        let events = root.items_at_path("/data[at0001]/events[at0006]");
        assert_eq!(events.len(), 2);
        // Both Systolic elements, one per event, through the shared at-code.
        let systolics =
            root.items_at_path("/data[at0001]/events[at0006]/data[at0003]/items[at0004]");
        assert_eq!(systolics.len(), 2);
        assert!(
            systolics
                .iter()
                .all(|n| n.path_node_name() == Some("Systolic"))
        );
    }

    #[test]
    fn path_unique_distinguishes_shared_at_codes() {
        let root = bp_tree();
        assert!(!root.path_unique("/data[at0001]/events[at0006]"));
        assert!(root.path_unique("/data[at0001]/events[at0006, 'First']"));
        assert!(root.path_unique("/data[at0001]"));
        assert!(!root.path_unique("/data[at9999]")); // precondition violation → false
    }

    #[test]
    fn item_at_path_resolves_unique_paths_only() {
        let root = bp_tree();
        // Unique deep path via combined predicate.
        let diastolic = root
            .item_at_path("/data[at0001]/events[at0006, 'First']/data[at0003]/items[at0005]")
            .expect("unique path resolves");
        assert_eq!(diastolic.path_node_name(), Some("Diastolic"));
        // Non-unique path → None (precondition `path_unique` violated).
        assert!(root.item_at_path("/data[at0001]/events[at0006]").is_none());
        // Non-existent path → None.
        assert!(root.item_at_path("/data[at9999]").is_none());
        // Unparseable path → None.
        assert!(root.item_at_path("/data[").is_none());
    }

    #[test]
    fn name_only_predicate_matches() {
        let root = bp_tree();
        let second = root
            .item_at_path("/data[at0001]/events['Second']")
            .expect("name-only predicate resolves");
        assert_eq!(second.path_node_id(), Some("at0006"));
        assert_eq!(second.path_node_name(), Some("Second"));
    }

    #[test]
    fn combined_predicate_requires_both_components() {
        let root = bp_tree();
        assert!(root.path_exists("/data[at0001]/events[at0006, 'First']"));
        // Right id, wrong name.
        assert!(!root.path_exists("/data[at0001]/events[at0006, 'Third']"));
        // Wrong id, right name.
        assert!(!root.path_exists("/data[at0001]/events[at0007, 'First']"));
    }

    #[test]
    fn path_of_item_round_trips_through_item_at_path() {
        let root = bp_tree();
        let diastolic = root
            .item_at_path("/data[at0001]/events[at0006, 'First']/data[at0003]/items[at0005]")
            .expect("unique path resolves");
        let path = root.path_of_item(diastolic).expect("descendant has a path");
        // Sibling events share at0006, so the rendered path disambiguates
        // by runtime name; items/at-codes are unique, so no name is added.
        assert_eq!(
            path,
            "/data[at0001]/events[at0006, 'First']/data[at0003]/items[at0005]"
        );
        // Round-trip: the rendered path resolves back to the same node.
        let resolved = root.item_at_path(&path).expect("rendered path resolves");
        assert!(std::ptr::addr_eq(
            resolved as *const dyn PathableApi,
            diastolic as *const dyn PathableApi
        ));
    }

    #[test]
    fn path_of_item_of_self_is_root() {
        let root = bp_tree();
        assert_eq!(root.path_of_item(root.as_pathable()).as_deref(), Some("/"));
    }

    #[test]
    fn path_of_item_of_stranger_is_none() {
        let root = bp_tree();
        let stranger = element("at0004", "Systolic"); // value-equal shape, different allocation
        assert!(root.path_of_item(&stranger).is_none());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.archetyped — docs/research/spec-cache/RM-1.1.0/uml_classes/pathable.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master03-archetyped_package.adoc §The PATHABLE Class / uml_classes/pathable.adoc §PATHABLE Class
//   confidence: high
//   todos: 2
//   note: Reference implementation of the PATHABLE.parent() reverse-pointer hazard (rm-transcription rule + ADR-001 §8) — parent() is the trait's one required spec method (Option<Weak<dyn PathableApi>>). The other four spec functions plus item/items_at_path are working default methods over five PORT-NOTEd traversal hooks (as_pathable/path_attribute_names/path_child_nodes/path_node_id/path_node_name), backed by the tiny parser/evaluator in paths.rs; concrete RM classes opt in by overriding the hooks (wired at P17). Leaf DATA_VALUE resolution deferred to the AQL path engine (P12).
// ─────────────────────────────────────────────
