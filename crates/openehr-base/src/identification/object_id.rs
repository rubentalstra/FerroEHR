//! `OBJECT_ID` — ancestor of informational object identifiers.
//!
//! openEHR class: `OBJECT_ID` (abstract), package
//! `base.base_types.identification`.
//!
//! Ancestor class of identifiers of informational objects. Ids may be
//! completely meaningless, in which case their only job is to refer to
//! something, or may carry some information to do with the identified
//! object.
//!
//! Object ids are used inside an object to identify that object. To
//! identify another object in another service, use an `OBJECT_REF`, or
//! else use a UID for local objects identified by UID. If none of the
//! subtypes is suitable, direct instances of this class may be used.
use super::archetype_id::ArchetypeId;
use super::generic_id::GenericId;
use super::template_id::TemplateId;
use super::terminology_id::TerminologyId;
use super::uid_based_id::{UidBasedId, UidBasedIdApi};

/// Shared attribute state of `OBJECT_ID` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait). `OBJECT_ID` declares one attribute, `value: String`, "the
/// value of the id in the form defined below" (i.e. by the per-subtype
/// EBNF grammar in the identification package's Syntaxes section).
///
/// PORT NOTE: `UID_BASED_ID` (and therefore `HIER_OBJECT_ID` and
/// `OBJECT_VERSION_ID`) does not use `ObjectIdData` directly — it re-embeds
/// its own copy of the `value: String` attribute, since `UID_BASED_ID`
/// layers additional behaviour (the `root`/`extension`/`has_extension`
/// parsing functions) on top of the same single attribute rather than
/// adding new fields. See `uid_based_id.rs`.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ObjectIdData {
    /// `value`: the value of the id, in the form defined by the concrete
    /// subtype's lexical grammar.
    pub value: String,
}

/// `OBJECT_ID` is abstract in the spec and is used polymorphically wherever
/// an attribute is declared of type `OBJECT_ID` (e.g. `OBJECT_REF.id`).
/// Per ADR-001 §4 (closed subtype set → enum), every concrete descendant —
/// both direct subtypes (`ARCHETYPE_ID`, `TEMPLATE_ID`, `TERMINOLOGY_ID`,
/// `GENERIC_ID`) and the `UID_BASED_ID` branch's own concretes
/// (`HIER_OBJECT_ID`, `OBJECT_VERSION_ID`) — is collected into this closed
/// `enum`.
///
/// The `UidBased` variant nests the narrower `UidBasedId` enum rather than
/// flattening `HierObjectId`/`ObjectVersionId` directly into `ObjectId`,
/// so that a field genuinely typed `UID_BASED_ID` in the spec (e.g.
/// `LOCATABLE_REF.id`, covariantly redefined — see `locatable_ref.rs`) can
/// be declared with the narrower `UidBasedId` type directly, matching
/// ADR-001 §6.
///
/// PORT NOTE: serialized `#[serde(untagged)]` rather than `#[serde(tag =
/// "_type")]`. Every variant's payload is itself a distinct Rust struct/enum
/// shape (`UidBasedId` is its own `_type`-tagged enum; `ArchetypeId`/
/// `TemplateId`/`TerminologyId`/`GenericId` are each a bare `{ value: ... }`
/// / `{ value, scheme }` struct with no discriminator field of its own yet),
/// so `serde(untagged)` dispatches purely by shape-matching the payload
/// against each variant in declaration order, rather than by reading an
/// explicit tag key. `openehr-serde`'s eventual manual `_type` dispatch (P17,
/// per ADR-001's "serde derives wait until P4" refinement + `docs/PORTING.md`
/// §6) replaces this mechanism entirely with hand-rolled deserialization that
/// reads the `_type` string first and only then picks the concrete Rust
/// type — this untagged derive is a P4 stopgap, not the final serialization
/// strategy for this enum.
///
/// TODO(port): untagged deserialization is fragile and order-sensitive by
/// construction — serde tries each variant in the order listed below and
/// commits to the first one whose shape matches, so a payload that happens
/// to satisfy an earlier variant's shape (e.g. any `{ value: String }`
/// object matches `UidBased`'s `HierObjectId`/`ObjectVersionId` branches
/// before ever reaching `ArchetypeId`) will silently deserialize into the
/// wrong concrete type. Variant order below is `UidBased` first (since its
/// own `_type` tag lets `UidBasedId`'s tagged-enum deserializer reject a
/// non-matching payload cleanly) followed by the four flat `OBJECT_ID`
/// leaves; this ordering is a "least-ambiguous-first" heuristic, not a
/// guarantee — a real fix requires the manual `_type`-dispatch replacement
/// named above, not a different untagged variant order.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(untagged)]
pub enum ObjectId {
    /// The `UID_BASED_ID` branch (`HIER_OBJECT_ID` or `OBJECT_VERSION_ID`).
    UidBased(UidBasedId),
    /// `ARCHETYPE_ID`.
    ///
    /// TODO(port): this leaf struct has no `_type` discriminator of its own
    /// on the wire yet (it still records `TYPE_NAME` as a `const` per
    /// ADR-001's P4-deferral note, not a `#[serde(rename)]`) — under this
    /// enum's `untagged` representation the serialized JSON for this variant
    /// is indistinguishable from a bare `ArchetypeId` value, i.e. the
    /// `_type: "ARCHETYPE_ID"` string is absent even though ITS-JSON
    /// generally requires it whenever the statically declared field type is
    /// abstract (`.claude/rules/serialization.md`) — `OBJECT_ID` is exactly
    /// such an abstract type. Left as-is rather than speculatively adding a
    /// rename to `archetype_id.rs`, which is out of scope for this file's
    /// edit pass; flagged for the `openehr-serde` P17 replacement.
    ArchetypeId(ArchetypeId),
    /// `TEMPLATE_ID`.
    ///
    /// See the `TODO(port)` on the `ArchetypeId` variant above — the same
    /// missing-`_type`-under-untagged gap applies here.
    TemplateId(TemplateId),
    /// `TERMINOLOGY_ID`.
    ///
    /// See the `TODO(port)` on the `ArchetypeId` variant above — the same
    /// missing-`_type`-under-untagged gap applies here.
    TerminologyId(TerminologyId),
    /// `GENERIC_ID`.
    ///
    /// See the `TODO(port)` on the `ArchetypeId` variant above — the same
    /// missing-`_type`-under-untagged gap applies here.
    GenericId(GenericId),
}

/// Marker/accessor trait shared by every `OBJECT_ID` descendant, exposing
/// the abstract class's sole attribute uniformly whether the caller holds a
/// concrete type or an `ObjectId` enum value.
pub trait ObjectIdApi {
    /// `value`: the value of the id, in the form defined by the concrete
    /// subtype.
    fn value(&self) -> &str;
}

impl ObjectIdApi for ObjectId {
    fn value(&self) -> &str {
        match self {
            ObjectId::UidBased(v) => v.value(),
            ObjectId::ArchetypeId(v) => v.value(),
            ObjectId::TemplateId(v) => v.value(),
            ObjectId::TerminologyId(v) => v.value(),
            ObjectId::GenericId(v) => v.value(),
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §OBJECT_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/object_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / object_id.adoc §OBJECT_ID Class
//   confidence: medium
//   todos: 2
//   note: ObjectId enum nests UidBasedId rather than flattening its two variants, so a UID_BASED_ID-typed field elsewhere can use the narrower enum directly per ADR-001 §6; ambiguity noted in the transcription report. P4 addendum: ObjectId is serde(untagged) since its four leaf variants have no _type discriminator of their own yet — order-sensitive and drops _type on the leaves; replace with openehr-serde's manual _type dispatch at P17 (the other three leaf variants cross-reference the same gap via doc comments rather than repeating a standalone TODO(port) marker each).
// ─────────────────────────────────────────────
