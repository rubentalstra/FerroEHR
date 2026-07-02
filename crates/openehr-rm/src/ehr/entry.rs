//! `ENTRY` — abstract parent of all `ENTRY` subtypes.
//!
//! openEHR class: `ENTRY` (abstract), package `rm.ehr.entry`.
//! Inherits: `CONTENT_ITEM`.
//!
//! The abstract parent of all `ENTRY` subtypes. An `ENTRY` is the root of a
//! logical item of "hard" clinical information created in the "clinical
//! statement" context, within a clinical session. There can be numerous
//! such contexts in a clinical session. Observations and other Entry types
//! only ever document information captured/created in the event documented
//! by the enclosing Composition.
//!
//! An `ENTRY` is also the minimal unit of information any query should
//! return, since a whole `ENTRY` (including subparts) records spatial
//! structure, timing information, and contextual information, as well as
//! the subject and generator of the information.
//!
//! `ENTRY` is abstract with attributes; per ADR-001 §3 it transcribes as an
//! embedded struct ([`EntryData`]) plus a marker/accessor trait
//! ([`EntryApi`]), the same shape as [`super::content_item`]'s embedding of
//! `LOCATABLE`. `ENTRY` itself embeds `CONTENT_ITEM` (in turn embedding
//! `LOCATABLE`), so [`EntryData`] carries `content_item:
//! super::content_item::ContentItemData` as its own first field, mirroring
//! the spec's inheritance chain by nested composition. `ENTRY` has two
//! direct descendants in the RM: the abstract [`super::care_entry`] and the
//! concrete [`super::admin_entry::AdminEntry`], both of which embed
//! `EntryData` in turn.
use crate::common::generic::participation::Participation; // TODO(port): forward-reference; not yet transcribed.
use crate::data_types::text::code_phrase::CodePhrase; // TODO(port): forward-reference; not yet transcribed.
use openehr_base::identification::object_ref::ObjectRef;

// TODO(port): forward-reference — `PARTY_PROXY` lives in rm.common.generic
// (PORT_MASTER_PLAN.md §7.1), not yet transcribed. Closed subtype set per
// ADR-001 §4, so the eventual type here will be an enum.
use crate::common::generic::party_proxy::PartyProxy;

/// Embedded attribute state of the abstract `ENTRY` class.
///
/// Per ADR-001 §3, concrete `ENTRY` descendants (via [`super::care_entry`]
/// and [`super::admin_entry::AdminEntry`]) embed this struct by
/// composition rather than inheriting from it.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryData {
    // NOTE: `ENTRY` inherits `CONTENT_ITEM` (in turn `LOCATABLE`), not
    // `PATHABLE` directly — this is the ordinary `LOCATABLE`-chain case,
    // unlike the settled `EVENT_CONTEXT`/`INSTRUCTION_DETAILS`/
    // `ISM_TRANSITION` hazard. `content_item` is the composed parent state.
    /// Embedded `CONTENT_ITEM` (in turn `LOCATABLE`) state.
    pub content_item: super::content_item::ContentItemData,

    /// `language`: mandatory indicator of the localised language in which
    /// this Entry is written. Coded from openEHR Code Set `languages`.
    ///
    /// Invariant `Language_valid`: `code_set (Code_set_id_languages)
    /// .has_code (language)`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
    pub language: CodePhrase,

    /// `encoding`: name of character set in which text values in this
    /// Entry are encoded. Coded from openEHR Code Set `character sets`.
    ///
    /// Invariant `Encoding_valid`: `code_set (Code_set_id_character_sets)
    /// .has_code (encoding)`.
    ///
    /// TODO(port): invariant not yet enforced.
    pub encoding: CodePhrase,

    /// `other_participations`: other participations at `ENTRY` level.
    ///
    /// Invariant `Other_participations_valid`: `other_participations /=
    /// Void implies not other_participations.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced.
    pub other_participations: Option<Vec<Participation>>,

    /// `workflow_id`: identifier of externally held workflow engine data
    /// for this workflow execution, for this subject of care.
    pub workflow_id: Option<ObjectRef>,

    /// `subject`: id of human subject of this `ENTRY`, e.g. organ donor,
    /// foetus, a family member, another clinically relevant person.
    ///
    /// Invariant `Subject_validity`: `subject_is_self implies
    /// subject.generating_type = "PARTY_SELF"`.
    ///
    /// TODO(port): invariant not yet enforced.
    pub subject: PartyProxy,

    /// `provider`: optional identification of provider of the information
    /// in this `ENTRY`, which might be the patient, a patient agent (e.g.
    /// parent, guardian), the clinician, or a device or software.
    /// Generally only used when the recorder needs to make it explicit.
    /// Otherwise, Composition composer and other participants are assumed.
    pub provider: Option<PartyProxy>,
}

/// Marker/accessor trait shared by every `ENTRY` descendant, exposing the
/// abstract class's attributes and functions uniformly whether the caller
/// holds a concrete type or a closed enum value (e.g. the future
/// `ContentItem` enum's `Entry`-derived variants).
pub trait EntryApi {
    /// Access to the embedded [`EntryData`].
    fn entry_data(&self) -> &EntryData;

    /// `subject_is_self` (): `Boolean`.
    ///
    /// Returns True if this Entry is about the subject of the EHR, in
    /// which case the `subject` attribute is of type `PARTY_SELF`.
    ///
    /// Postcondition: `Result implies subject.generating_type =
    /// "PARTY_SELF"`.
    ///
    /// TODO(port): depends on the not-yet-transcribed `PartyProxy` enum's
    /// discriminant (`generating_type` equivalent).
    fn subject_is_self(&self) -> bool {
        todo!("port: EntryApi::subject_is_self awaits the PartyProxy closed enum (ADR-001 §4)")
    }

    /// Invariant `Is_archetype_root`: `is_archetype_root`.
    ///
    /// Inherited unchanged from `LOCATABLE` via `CONTENT_ITEM`, restated
    /// here so its presence on every `ENTRY` descendant is not lost during
    /// transcription.
    ///
    /// TODO(port): delegates to `LOCATABLE.is_archetype_root()`, not yet
    /// implemented; awaits the `common::archetyped::locatable`
    /// transcription.
    fn invariant_is_archetype_root(&self) -> bool {
        todo!(
            "port: delegate to LocatableData::is_archetype_root() once common::archetyped::locatable lands"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/entry.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / entry.adoc §ENTRY Class
//   confidence: high
//   todos: 9
//   note: abstract-with-attributes per ADR-001 §3 (EntryData + EntryApi); content_item field composes the CONTENT_ITEM/LOCATABLE chain; subject_is_self()/invariant delegates deferred pending PartyProxy enum and LOCATABLE transcription; several of the 9 markers are forward-reference import comments (Participation, CodePhrase, PartyProxy).
// ─────────────────────────────────────────────
