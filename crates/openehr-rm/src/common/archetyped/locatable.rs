//! `LOCATABLE` — root class of all archetypable RM classes.
//!
//! openEHR class: `LOCATABLE` (abstract), package `common.archetyped`.
//! Inherits: `PATHABLE`.
//!
//! Root class of all information model classes that can be archetyped.
//! Most classes in the openEHR reference model inherit from `LOCATABLE`,
//! which defines the idea of "locatability in an archetyped structure".
//! `LOCATABLE` defines a runtime `name` and an `archetype_node_id`.
//!
//! # This is the pattern every later RM concrete class reuses
//!
//! Nearly every concrete RM class transcribed in subsequent phases
//! (`ENTRY`, `SECTION`, `COMPOSITION`, `CLUSTER`, `ELEMENT`, `FOLDER`,
//! `EHR_STATUS`, `PARTY`, and dozens more) inherits `LOCATABLE`. Per
//! ADR-001 §3 (abstract class with attributes → embedded struct + marker
//! trait), those classes do **not** re-declare `name`,
//! `archetype_node_id`, `uid`, `links`, `archetype_details`, and
//! `feeder_audit` themselves — they embed [`LocatableData`] as a field
//! (conventionally named `locatable`, mirroring how `ObjectRef` is
//! embedded verbatim in `PartyRef` — see
//! `openehr-base::identification::party_ref`) and implement
//! [`LocatableApi`] (which requires [`PathableApi`] as a supertrait,
//! matching the spec's `LOCATABLE inherits PATHABLE`) by delegating to
//! that embedded field. This file's shape is the load-bearing precedent
//! for every subsequent `LOCATABLE` descendant — deviating from it here
//! would fork the pattern before it starts.
//!
//! # Known hazard restated
//!
//! Per `.claude/rules/rm-transcription.md` "Known hazards": `EVENT_CONTEXT`,
//! `INSTRUCTION_DETAILS`, and `ISM_TRANSITION` inherit `PATHABLE` directly,
//! **not** `LOCATABLE` — they must never embed [`LocatableData`] or gain a
//! `uid`/`archetype_details`/`feeder_audit` field. Those three classes
//! belong to other RM packages (`rm.ehr`) and are out of scope for this
//! transcription pass, but the warning is repeated here because this file
//! is where a future transcriber is most likely to reach for "just embed
//! `LocatableData`" out of habit.
use std::sync::Weak;

use super::archetyped::Archetyped;
use super::feeder_audit::FeederAudit;
use super::link::Link;
use super::pathable::PathableApi;

use crate::data_types::text::dv_text::{DvText, DvTextApi, DvTextData};
use openehr_foundation::serde_support::TypeTag;
use serde::{Deserialize, Serialize};

// TODO(port): `UID_BASED_ID` is BASE 1.2.0 `base_types.identification`,
// already transcribed (see `openehr_base::identification::uid_based_id`).
// Imported directly since `openehr-rm` depends on `openehr-base`.
use openehr_base::identification::uid_based_id::UidBasedId;

// Canonical `_type` discriminator is not applicable to `LOCATABLE`
// itself — it is abstract and never serialized as a standalone value; per
// ADR-002, abstract classes and embedded `*Data` structs carry no
// `TypeTag`/`TypeName` of their own (the pinned ITS-JSON schema defines no
// entry for abstract classes). Every concrete descendant self-tags and
// `#[serde(flatten)]`s this struct, so `LocatableData` must stay untagged.

/// Shared attribute state of `LOCATABLE` and every RM class that inherits
/// it (directly, or — far more commonly — via one of the intermediate
/// abstract classes in later RM packages).
///
/// Per ADR-001 §3, every concrete `LOCATABLE` descendant embeds this
/// struct as a field rather than duplicating its six attributes, and
/// implements [`LocatableApi`] by delegating to the embedded field's
/// accessors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocatableData {
    /// `name`: `DV_TEXT`, cardinality `1..1`.
    ///
    /// Runtime name of this fragment, used to build runtime paths. This
    /// is the term provided via a clinical application or batch process
    /// to name this EHR construct: its retention in the EHR faithfully
    /// preserves the original label by which this entry was known to end
    /// users.
    pub name: DvText,

    /// `archetype_node_id`: `String`, cardinality `1..1`.
    ///
    /// Design-time archetype identifier of this node taken from its
    /// generating archetype; used to build archetype paths. Always in the
    /// form of an at-code, e.g. `at0005`. This value enables a
    /// "standardised" name for this node to be generated, by referring to
    /// the generating archetype local terminology.
    ///
    /// At an archetype root point, the value of this attribute is always
    /// the stringified form of the `archetype_id` found in the
    /// `archetype_details` object.
    ///
    /// Invariant `Archetype_node_id_valid`: `not
    /// archetype_node_id.is_empty`. Checked by
    /// [`LocatableData::invariant_archetype_node_id_valid`] (ADR-003 §8);
    /// the deep walker/accumulator `Validate` framework remains the P11
    /// deliverable.
    pub archetype_node_id: String,

    /// `uid`: `UID_BASED_ID`, cardinality `0..1`.
    ///
    /// Optional globally unique object identifier for root points of
    /// archetyped structures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<UidBasedId>,

    /// `links`: `List<LINK>`, cardinality `0..1`.
    ///
    /// Links to other archetyped structures (data whose root object
    /// inherits from `ARCHETYPED`, such as `ENTRY`, `SECTION` and so on).
    /// Links may be to structures in other compositions.
    ///
    /// Invariant `Links_valid`: `links /= Void implies not
    /// links.is_empty` — modelled as `Option<Vec<Link>>` rather than a
    /// bare `Vec<Link>` so `None` (attribute genuinely absent) and
    /// `Some(non_empty)` are the only two representable states, matching
    /// the invariant's own "if present, non-empty" shape exactly (an
    /// always-present-but-possibly-empty `Vec` would let an illegal
    /// `Some(empty)`-equivalent state exist unchecked). Checked by
    /// [`LocatableData::invariant_links_valid`] (ADR-003 §8); the deep
    /// walker/accumulator `Validate` framework remains the P11
    /// deliverable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<Link>>,

    /// `archetype_details`: `ARCHETYPED`, cardinality `0..1`.
    ///
    /// Details of archetyping used on this node.
    ///
    /// Invariant `Archetyped_valid`: `is_archetype_root xor
    /// archetype_details = Void` — structurally guaranteed rather than
    /// checked: [`LocatableApi::is_archetype_root`] is *derived* from this
    /// field (`archetype_details.is_some()`), so the two facts the `xor`
    /// relates can never disagree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archetype_details: Option<Archetyped>,

    /// `feeder_audit`: `FEEDER_AUDIT`, cardinality `0..1`.
    ///
    /// Audit trail from non-openEHR system of original commit of
    /// information forming the content of this node, or from a conversion
    /// gateway which has synthesised this node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feeder_audit: Option<FeederAudit>,

    /// Non-owning back-reference to this node's parent in the
    /// compositional hierarchy, satisfying `PATHABLE.parent()`.
    ///
    /// PORT NOTE: not a spec attribute — `LOCATABLE`'s own attribute table
    /// has no `parent` row (the spec models `parent()` purely as a
    /// `PATHABLE` function, not a `LOCATABLE` attribute). This field exists
    /// so a concrete class's `PathableApi::parent()` has concrete state to
    /// read from — via [`LocatableData::parent_pathable`], the one-line
    /// delegation every `LOCATABLE` descendant uses — per the settled
    /// `Weak`-back-reference pattern established in `pathable.rs`. `dyn LocatableApi` (not `dyn
    /// PathableApi`) because every concrete `LOCATABLE` descendant's
    /// parent, when it has one, is itself typically another `LOCATABLE`
    /// (the compositional hierarchy of archetyped structures is a
    /// `LOCATABLE` tree in practice), and exposing the narrower trait
    /// object here lets a `LOCATABLE`-aware caller call `LocatableApi`
    /// methods on the parent without a downcast — while `PathableApi`
    /// methods remain available too since `LocatableApi: PathableApi`.
    #[serde(skip)]
    pub parent: Option<Weak<dyn LocatableApi>>,
}

/// Equality over the six spec attributes only. `parent` is excluded: it is
/// a non-spec back-reference (see its PORT NOTE), `Weak<dyn ..>` has no
/// meaningful structural equality, and canonical-JSON round-trips (which
/// never carry parent pointers) must compare equal to their source.
/// PORT NOTE: manual impl because `Weak` cannot derive `PartialEq`.
impl PartialEq for LocatableData {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.archetype_node_id == other.archetype_node_id
            && self.uid == other.uid
            && self.links == other.links
            && self.archetype_details == other.archetype_details
            && self.feeder_audit == other.feeder_audit
    }
}

impl LocatableData {
    /// The stored parent back-reference, widened to the abstract
    /// `PATHABLE` trait object that [`PathableApi::parent`] returns.
    ///
    /// Every concrete `LOCATABLE` descendant's `impl PathableApi` is
    /// expected to be exactly `fn parent(&self) ->
    /// Option<Weak<dyn PathableApi>> { self.locatable_data().parent_pathable() }`.
    ///
    /// PORT NOTE: the `Weak<dyn LocatableApi>` → `Weak<dyn PathableApi>`
    /// widening is a plain unsizing coercion — trait-object upcasting to a
    /// supertrait is stable since Rust 1.86 and works through `Weak`
    /// directly (no upgrade-and-rewrap dance needed), which resolves the
    /// earlier open question recorded in this file about that glue code.
    pub fn parent_pathable(&self) -> Option<Weak<dyn PathableApi>> {
        self.parent
            .clone()
            .map(|weak| weak as Weak<dyn PathableApi>)
    }

    /// [`PathableApi::path_node_id`] delegate: the `archetype_node_id`
    /// this node matches in a path predicate. Concrete `LOCATABLE`
    /// descendants override the hook with exactly this one-liner.
    pub fn path_node_id(&self) -> Option<&str> {
        Some(&self.archetype_node_id)
    }

    /// [`PathableApi::path_node_name`] delegate: the runtime `name.value`
    /// this node matches in a `'name'` path predicate. Concrete
    /// `LOCATABLE` descendants override the hook with exactly this
    /// one-liner.
    pub fn path_node_name(&self) -> Option<&str> {
        Some(self.name.value())
    }

    /// `Links_valid`: `links /= Void implies not links.is_empty`
    /// (ADR-003 §8: invariants become working checks now; the deep
    /// walker/accumulator `Validate` framework is the P11 deliverable).
    pub fn invariant_links_valid(&self) -> bool {
        self.links.as_ref().is_none_or(|links| !links.is_empty())
    }

    /// `Archetype_node_id_valid`: `not archetype_node_id.is_empty`
    /// (ADR-003 §8).
    pub fn invariant_archetype_node_id_valid(&self) -> bool {
        !self.archetype_node_id.is_empty()
    }
}

/// Behaviour trait for `LOCATABLE` and every RM class that inherits it.
/// Supertrait bound to [`PathableApi`] mirrors the spec's `LOCATABLE
/// inherits PATHABLE` relationship (ADR-001 §2/§3: multiple-inheritance
/// and abstract-with-attributes classes both compose behaviour via
/// supertraits).
pub trait LocatableApi: PathableApi {
    /// Accessor for the embedded [`LocatableData`], required so every
    /// default method below has something to delegate to. Implementors
    /// (concrete `LOCATABLE` descendants) provide this by returning a
    /// reference to their embedded `locatable: LocatableData` field.
    fn locatable_data(&self) -> &LocatableData;

    /// `name`: `DV_TEXT`, cardinality `1..1`. See
    /// [`LocatableData::name`].
    fn name(&self) -> &DvText {
        &self.locatable_data().name
    }

    /// `archetype_node_id`: `String`, cardinality `1..1`. See
    /// [`LocatableData::archetype_node_id`].
    fn archetype_node_id(&self) -> &str {
        &self.locatable_data().archetype_node_id
    }

    /// `uid`: `UID_BASED_ID`, cardinality `0..1`. See
    /// [`LocatableData::uid`].
    fn uid(&self) -> Option<&UidBasedId> {
        self.locatable_data().uid.as_ref()
    }

    /// `links`: `List<LINK>`, cardinality `0..1`. See
    /// [`LocatableData::links`].
    fn links(&self) -> Option<&[Link]> {
        self.locatable_data().links.as_deref()
    }

    /// `archetype_details`: `ARCHETYPED`, cardinality `0..1`. See
    /// [`LocatableData::archetype_details`].
    fn archetype_details(&self) -> Option<&Archetyped> {
        self.locatable_data().archetype_details.as_ref()
    }

    /// `feeder_audit`: `FEEDER_AUDIT`, cardinality `0..1`. See
    /// [`LocatableData::feeder_audit`].
    fn feeder_audit(&self) -> Option<&FeederAudit> {
        self.locatable_data().feeder_audit.as_ref()
    }

    /// `concept(): DV_TEXT`, cardinality `1..1`.
    ///
    /// Clinical concept of the archetype as a whole (derived from the
    /// `archetype_node_id` of the root node).
    ///
    /// Implemented as the spec states it: walk up the parent chain (via
    /// [`LocatableData::parent`], upgrading each `Weak`) to the nearest
    /// archetype root, and derive the `DV_TEXT` from that root's
    /// `archetype_node_id` — which, at an archetype root point, is always
    /// the stringified `archetype_id` (see
    /// [`LocatableData::archetype_node_id`]).
    ///
    /// Widened to `Option<..>`: a node that is not an archetype root and
    /// has no reachable parent chain (detached, or a dropped `Weak`) has
    /// no root to derive from — `None` rather than a panic, per the
    /// precondition-widening pattern established in `pathable.rs`.
    ///
    /// TODO(port): the *displayable rubric* for the concept (resolving the
    /// root at-code against the generating archetype's local terminology)
    /// needs the archetype/ontology machinery that lands with the
    /// WebTemplate builder at P10; until then the derived `DV_TEXT` value
    /// is the root node id string itself.
    fn concept(&self) -> Option<DvText> {
        let root_node_id: String = if self.is_archetype_root() {
            self.archetype_node_id().to_string()
        } else {
            let mut current = self.locatable_data().parent.as_ref()?.upgrade()?;
            while !current.is_archetype_root() {
                let next = current.locatable_data().parent.as_ref()?.upgrade()?;
                current = next;
            }
            current.archetype_node_id().to_string()
        };
        Some(DvText::Text {
            type_tag: TypeTag::new(),
            data: DvTextData {
                value: root_node_id,
                hyperlink: None,
                formatting: None,
                mappings: None,
                language: None,
                encoding: None,
            },
        })
    }

    /// `is_archetype_root(): Boolean`, cardinality `1..1`.
    ///
    /// True if this node is the root of an archetyped structure.
    ///
    /// Per invariant `Archetyped_valid`: `is_archetype_root xor
    /// archetype_details = Void`, this is a derived boolean, not
    /// independent state — implemented directly here as
    /// `archetype_details.is_some()` (the `xor` in the invariant states
    /// these two facts always agree, so computing one from the other is
    /// exactly correct, not just a convenient approximation).
    fn is_archetype_root(&self) -> bool {
        self.locatable_data().archetype_details.is_some()
    }
}

// PORT NOTE: `LocatableApi: PathableApi` requires every `LocatableApi`
// implementor to also provide `PathableApi`. Per the settled
// `pathable.rs` reverse-pointer design, `PathableApi::parent()` returns
// `Option<Weak<dyn PathableApi>>`; a concrete `LOCATABLE` descendant's
// `impl PathableApi for ConcreteType` is four one-liners over the
// embedded `LocatableData`:
//
//   fn as_pathable(&self) -> &dyn PathableApi { self }
//   fn parent(&self) -> Option<Weak<dyn PathableApi>> {
//       self.locatable_data().parent_pathable()
//   }
//   fn path_node_id(&self) -> Option<&str> { self.locatable_data().path_node_id() }
//   fn path_node_name(&self) -> Option<&str> { self.locatable_data().path_node_name() }
//
// plus per-class `path_attribute_names`/`path_child_nodes` overrides
// exposing its own PATHABLE-typed attributes. The earlier open question
// about a `Weak<dyn LocatableApi>` → `Weak<dyn PathableApi>` rewrap is
// resolved: trait-object upcasting to a supertrait is a stable unsizing
// coercion since Rust 1.86 and works through `Weak` directly (see
// `LocatableData::parent_pathable`). No blanket
// `impl<T: LocatableApi> PathableApi for T` is provided — it would forbid
// the per-class hook overrides above (E0119) and cannot coerce an unsized
// `Self`.
//
// TODO(port): wiring the `impl PathableApi`/`impl LocatableApi` pair
// across the concrete RM classes is the P17 (make-it-compile) pass; the
// `#[cfg(test)]` `TestLocatable` below is the validated template.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use openehr_base::identification::archetype_id::ArchetypeId;
    use openehr_base::identification::object_id::ObjectIdData;

    use super::*;

    /// Minimal concrete `LOCATABLE` descendant following exactly the
    /// four-one-liner `impl PathableApi` template this file prescribes —
    /// the validated shape P17 replicates across the RM concretes.
    struct TestLocatable {
        locatable: LocatableData,
    }

    impl PathableApi for TestLocatable {
        fn as_pathable(&self) -> &dyn PathableApi {
            self
        }

        fn parent(&self) -> Option<Weak<dyn PathableApi>> {
            self.locatable_data().parent_pathable()
        }

        fn path_node_id(&self) -> Option<&str> {
            self.locatable_data().path_node_id()
        }

        fn path_node_name(&self) -> Option<&str> {
            self.locatable_data().path_node_name()
        }
    }

    impl LocatableApi for TestLocatable {
        fn locatable_data(&self) -> &LocatableData {
            &self.locatable
        }
    }

    fn dv_text(value: &str) -> DvText {
        DvText::Text {
            type_tag: TypeTag::new(),
            data: DvTextData {
                value: value.to_string(),
                hyperlink: None,
                formatting: None,
                mappings: None,
                language: None,
                encoding: None,
            },
        }
    }

    fn archetyped(archetype_id: &str) -> Archetyped {
        Archetyped {
            type_tag: TypeTag::new(),
            archetype_id: ArchetypeId {
                type_tag: TypeTag::new(),
                object_id: ObjectIdData {
                    value: archetype_id.to_string(),
                },
            },
            template_id: None,
            rm_version: "1.1.0".to_string(),
        }
    }

    fn node(
        name: &str,
        archetype_node_id: &str,
        archetype_details: Option<Archetyped>,
        parent: Option<&Arc<TestLocatable>>,
    ) -> TestLocatable {
        TestLocatable {
            locatable: LocatableData {
                name: dv_text(name),
                archetype_node_id: archetype_node_id.to_string(),
                uid: None,
                links: None,
                archetype_details,
                feeder_audit: None,
                parent: parent.map(|arc| {
                    let arc: Arc<dyn LocatableApi> = Arc::clone(arc) as Arc<dyn LocatableApi>;
                    Arc::downgrade(&arc)
                }),
            },
        }
    }

    const BP_ARCHETYPE: &str = "openEHR-EHR-OBSERVATION.bp.v1";

    #[test]
    fn is_archetype_root_derives_from_archetype_details() {
        let root = node(
            "Blood pressure",
            BP_ARCHETYPE,
            Some(archetyped(BP_ARCHETYPE)),
            None,
        );
        let inner = node("History", "at0001", None, None);
        assert!(root.is_archetype_root());
        assert!(!inner.is_archetype_root());
    }

    #[test]
    fn concept_at_archetype_root_is_the_root_node_id() {
        let root = node(
            "Blood pressure",
            BP_ARCHETYPE,
            Some(archetyped(BP_ARCHETYPE)),
            None,
        );
        let concept = root.concept().expect("root derives its own concept");
        assert_eq!(concept.value(), BP_ARCHETYPE);
    }

    #[test]
    fn concept_walks_the_parent_chain_to_the_archetype_root() {
        let root = Arc::new(node(
            "Blood pressure",
            BP_ARCHETYPE,
            Some(archetyped(BP_ARCHETYPE)),
            None,
        ));
        let history = Arc::new(node("History", "at0001", None, Some(&root)));
        let event = node("Any event", "at0006", None, Some(&history));

        let concept = event.concept().expect("chained node reaches the root");
        assert_eq!(concept.value(), BP_ARCHETYPE);
    }

    #[test]
    fn concept_of_detached_non_root_is_none() {
        let detached = node("History", "at0001", None, None);
        assert!(detached.concept().is_none());
    }

    #[test]
    fn parent_pathable_upcasts_and_upgrades() {
        let root = Arc::new(node(
            "Blood pressure",
            BP_ARCHETYPE,
            Some(archetyped(BP_ARCHETYPE)),
            None,
        ));
        let child = node("History", "at0001", None, Some(&root));

        let weak = child.parent().expect("child stores a parent link");
        let parent = weak.upgrade().expect("root is still alive");
        assert_eq!(parent.path_node_id(), Some(BP_ARCHETYPE));
        assert_eq!(parent.path_node_name(), Some("Blood pressure"));
        // A root's own parent is structurally absent.
        assert!(root.parent().is_none());
    }

    #[test]
    fn path_hooks_delegate_to_locatable_data() {
        let root = node(
            "Blood pressure",
            BP_ARCHETYPE,
            Some(archetyped(BP_ARCHETYPE)),
            None,
        );
        assert_eq!(root.path_node_id(), Some(BP_ARCHETYPE));
        assert_eq!(root.path_node_name(), Some("Blood pressure"));
    }

    #[test]
    fn invariants_hold_and_fail_as_specified() {
        let mut data = node("History", "at0001", None, None).locatable;
        assert!(data.invariant_archetype_node_id_valid());
        assert!(data.invariant_links_valid()); // links = None: valid

        data.archetype_node_id.clear();
        assert!(!data.invariant_archetype_node_id_valid());

        data.links = Some(Vec::new()); // present-but-empty: invalid
        assert!(!data.invariant_links_valid());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.archetyped — docs/research/spec-cache/RM-1.1.0/uml_classes/locatable.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master03-archetyped_package.adoc §The LOCATABLE Class / uml_classes/locatable.adoc §LOCATABLE Class
//   confidence: high
//   todos: 2
//   note: LocatableData + LocatableApi (: PathableApi) is the pattern every later LOCATABLE-inheriting concrete class must reuse (do not re-derive); the cfg(test) TestLocatable is the validated four-one-liner impl template for P17. concept() implemented (parent-chain walk to the archetype root; rubric resolution deferred to P10). is_archetype_root derived; Links_valid/Archetype_node_id_valid are working invariant methods (ADR-003 §8), Archetyped_valid structurally guaranteed. parent_pathable() uses stable (1.86+) Weak dyn-upcasting.
// ─────────────────────────────────────────────
