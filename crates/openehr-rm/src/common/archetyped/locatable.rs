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

// TODO(port): `DV_TEXT` is RM 1.1.0 `data_types.text`, transcribed by a
// sibling agent in this same phase but not yet landed in this worktree.
// Forward-reference to its eventual module path.
use crate::data_types::text::dv_text::DvText;
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
    /// archetype_node_id.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant
    /// framework (`.claude/rules/rm-transcription.md` "Invariants").
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
    /// `Some(empty)`-equivalent state exist unchecked).
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<Link>>,

    /// `archetype_details`: `ARCHETYPED`, cardinality `0..1`.
    ///
    /// Details of archetyping used on this node.
    ///
    /// Invariant `Archetyped_valid`: `is_archetype_root xor
    /// archetype_details = Void` — see
    /// [`LocatableApi::is_archetype_root`].
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
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
    /// so [`LocatableApi`]'s blanket `PathableApi::parent()` delegation
    /// (see the `impl<T: LocatableApi> PathableApi for T` note below) has
    /// concrete state to read from, per the settled `Weak`-back-reference
    /// pattern established in `pathable.rs`. `dyn LocatableApi` (not `dyn
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
    /// TODO(port): "derived from the archetype_node_id of the root node"
    /// requires walking up to the archetype root via `parent()`
    /// (`PathableApi`) and resolving the node id against the generating
    /// archetype's ontology section — neither the archetype-resolution
    /// service nor the parent-walk is implementable yet. `todo!()` for
    /// now.
    fn concept(&self) -> DvText {
        todo!("LocatableApi::concept: requires archetype-ontology resolution, not yet implemented")
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
// `impl PathableApi for ConcreteType` is expected to read
// `self.locatable_data().parent` (this file's `LocatableData::parent`,
// typed `Option<Weak<dyn LocatableApi>>`) and re-wrap each upgraded
// `Arc<dyn LocatableApi>` as the wider `Weak<dyn PathableApi>` the trait
// signature demands — an unsizing coercion from `Weak<dyn LocatableApi>`
// to `Weak<dyn PathableApi>` is not automatic in stable Rust for trait
// objects behind `Weak`, so that glue code is left to each concrete
// implementor (or a shared free function once enough concretes exist to
// show the common shape) rather than provided as a default method here,
// where `Self: Sized` is not guaranteed and no blanket impl can safely
// assume the coercion path.
//
// TODO(port): no blanket `impl<T: LocatableApi> PathableApi for T` is
// provided in this pass — left for the first concrete `LOCATABLE`
// descendant (RM data_structures/ehr phase) to resolve, once the
// `Weak<dyn LocatableApi>` → `Weak<dyn PathableApi>` upgrade-and-rewrap
// shape has a real implementor to validate against.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.archetyped — docs/research/spec-cache/RM-1.1.0/uml_classes/locatable.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master03-archetyped_package.adoc §The LOCATABLE Class / uml_classes/locatable.adoc §LOCATABLE Class
//   confidence: high
//   todos: 5
//   note: LocatableData + LocatableApi (: PathableApi) is the pattern every later LOCATABLE-inheriting concrete class must reuse (do not re-derive). Three invariants (Archetype_node_id_valid, Links_valid, Archetyped_valid) recorded as doc notes / derived methods but not yet Validate-enforced. concept() stubbed pending archetype-ontology resolution. No blanket PathableApi impl provided yet — first concrete LOCATABLE descendant must supply the Weak<dyn LocatableApi>-to-Weak<dyn PathableApi> rewrap. Forward-refs DvText (data_types.text, sibling-agent territory, not yet landed).
// ─────────────────────────────────────────────
