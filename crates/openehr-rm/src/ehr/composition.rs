//! `COMPOSITION` — content of one version in a `VERSIONED_COMPOSITION`.
//!
//! openEHR class: `COMPOSITION`, package `rm.ehr.composition`.
//! Inherits: `LOCATABLE`.
//!
//! Content of one version in a `VERSIONED_COMPOSITION`. A Composition is
//! considered the unit of modification of the record, the unit of
//! transmission in record Extracts, and the unit of attestation by
//! authorising clinicians. In this latter sense, it may be considered
//! equivalent to a signed document.
//!
//! NOTE (spec): it is strongly recommended that the inherited attribute
//! `_uid_` be populated in Compositions, using the UID copied from the
//! `object_id()` of the `_uid_` field of the enclosing `VERSION` object.
//! For example, the `ORIGINAL_VERSION.uid`
//! `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2` would be copied
//! to the `_uid_` field of the Composition.
use crate::common::archetyped::locatable::LocatableData; // TODO(port): forward-reference; not yet transcribed. Path matches the sibling ehr_status.rs/ehr_access.rs convention.
use crate::data_types::text::code_phrase::CodePhrase; // TODO(port): forward-reference; not yet transcribed.
use crate::data_types::text::dv_coded_text::DvCodedText; // TODO(port): forward-reference; not yet transcribed.

// TODO(port): forward-reference — `PARTY_PROXY` lives in rm.common.generic
// (PORT_MASTER_PLAN.md §7.1), not yet transcribed. Closed subtype set per
// ADR-001 §4, so the eventual type here will be an enum.
use crate::common::generic::party_proxy::PartyProxy;

/// Canonical `_type` discriminator string for this class in serialized
/// form.
pub const TYPE_NAME: &str = "COMPOSITION";

/// `COMPOSITION` — content of one version in a `VERSIONED_COMPOSITION`.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait), `LOCATABLE`'s state is embedded as
/// `pub locatable: LocatableData` rather than simulated via a Rust
/// supertrait, matching the sibling `EhrStatus`/`EhrAccess` shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Composition {
    /// Embedded `LOCATABLE` state.
    pub locatable: LocatableData,

    /// `language`: mandatory indicator of the localised language in which
    /// this Composition is written. Coded from openEHR Code Set
    /// `languages`. The language of an Entry if different from the
    /// Composition is indicated in `ENTRY.language`.
    ///
    /// Invariant `Language_valid`: `code_set (Code_set_id_languages)
    /// .has_code (language)`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
    pub language: CodePhrase,

    /// `territory`: name of territory in which this Composition was
    /// written. Coded from openEHR `countries` code set, which is an
    /// expression of the ISO 3166 standard.
    ///
    /// Invariant `Territory_valid`: `code_set (Code_set_id_countries)
    /// .has_code (territory)`.
    ///
    /// TODO(port): invariant not yet enforced.
    pub territory: CodePhrase,

    /// `category`: temporal category of this Composition, i.e.
    /// `431|persistent|` (of potential life-time validity),
    /// `451|episodic|` (valid over the life of a care episode),
    /// `433|event|` (valid at the time of recording; long-term validity
    /// requires subsequent clinical assessment), or any other code
    /// defined in the openEHR terminology group `category`.
    ///
    /// Invariant `Category_validity`: `terminology
    /// (Terminology_id_openehr).has_code_for_group_id
    /// (Group_id_composition_category, category.defining_code)`.
    ///
    /// TODO(port): invariant not yet enforced.
    pub category: DvCodedText,

    /// `context`: the clinical session context of this Composition, i.e.
    /// the contextual attributes of the clinical session.
    pub context: Option<super::event_context::EventContext>,

    /// `composer`: the person primarily responsible for the content of
    /// the Composition (but not necessarily its committal into the EHR
    /// system). This is the identifier which should appear on the screen.
    /// It may or may not be the person who entered the data. When it is
    /// the patient, the special `self` instance of `PARTY_PROXY` will be
    /// used.
    pub composer: PartyProxy,

    /// `content`: the content of this Composition.
    ///
    /// Invariant `Content_valid`: `content /= Void implies not
    /// content.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced.
    pub content: Option<Vec<super::content_item::ContentItem>>,
}

impl Composition {
    /// `is_persistent` (): `Boolean`.
    ///
    /// True if `category` is `431|persistent|`, False otherwise. Useful
    /// for finding Compositions in an EHR which are guaranteed to be of
    /// interest to most users.
    ///
    /// TODO(port): requires reading `category`'s `DV_CODED_TEXT.defining_code`
    /// against the literal openEHR terminology code `431`; `DvCodedText` is
    /// itself a forward-reference not yet transcribed, so the comparison
    /// cannot be implemented yet.
    pub fn is_persistent(&self) -> bool {
        todo!(
            "port: Composition::is_persistent needs DvCodedText.defining_code compared against the openEHR terminology code 431 (\"persistent\")"
        )
    }

    /// Invariant `Category_validity`: `terminology
    /// (Terminology_id_openehr).has_code_for_group_id
    /// (Group_id_composition_category, category.defining_code)`.
    ///
    /// TODO(port): delegates to the not-yet-transcribed `TerminologyService`
    /// binding and the RM invariant framework
    /// (`.claude/rules/rm-transcription.md` "Invariants").
    pub fn invariant_category_validity(&self) -> bool {
        todo!(
            "port: Category_validity awaits the RM Validate-trait framework and terminology binding"
        )
    }

    /// Invariant `Territory_valid`: `code_set(Code_set_id_countries)
    /// .has_code(territory)`.
    ///
    /// TODO(port): as above.
    pub fn invariant_territory_valid(&self) -> bool {
        todo!(
            "port: Territory_valid awaits the RM Validate-trait framework and terminology binding"
        )
    }

    /// Invariant `Language_valid`: `code_set(Code_set_id_languages)
    /// .has_code(language)`.
    ///
    /// TODO(port): as above.
    pub fn invariant_language_valid(&self) -> bool {
        todo!("port: Language_valid awaits the RM Validate-trait framework and terminology binding")
    }

    /// Invariant `Content_valid`: `content /= Void implies not
    /// content.is_empty`.
    pub fn invariant_content_valid(&self) -> bool {
        self.content.as_ref().is_none_or(|c| !c.is_empty())
    }

    /// Invariant `Is_archetype_root`: `is_archetype_root`.
    ///
    /// Inherited unchanged from `LOCATABLE`, restated here so its presence
    /// on this class is not lost during transcription.
    ///
    /// TODO(port): delegates to `LOCATABLE.is_archetype_root()`, not yet
    /// implemented; awaits the `common::archetyped::locatable`
    /// transcription.
    pub fn invariant_is_archetype_root(&self) -> bool {
        todo!(
            "port: delegate to LocatableData::is_archetype_root() once common::archetyped::locatable lands"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.composition — docs/research/spec-cache/RM-1.1.0/uml_classes/composition.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-composition_package.adoc §Class Descriptions / composition.adoc §COMPOSITION Class
//   confidence: high
//   todos: 13
//   note: LOCATABLE embedded per ADR-001 §3; content typed as Option<Vec<ContentItem>> using the ADR-001 §4 closed enum from content_item.rs; is_persistent() and four of five invariants stubbed pending DvCodedText/terminology-service/LOCATABLE transcription — Content_valid is the one invariant implementable today (pure structural check) and is implemented, not stubbed; most of the 13 markers are forward-reference import/embed comments.
// ─────────────────────────────────────────────
