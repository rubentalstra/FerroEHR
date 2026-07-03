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
use crate::common::archetyped::locatable::LocatableData;
use crate::data_types::text::code_phrase::CodePhrase;
use crate::data_types::text::dv_coded_text::DvCodedText;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_term::{
    CodeSetAccess, OpenehrCodeSetIdentifiers, OpenehrTerminologyGroupIdentifiers,
    TerminologyAccess, TerminologyCode, TerminologyService,
};
use serde::{Deserialize, Serialize};

/// openEHR terminology code for the `431|persistent|` composition category
/// (`composition category` group). `Composition::is_persistent` compares
/// `category.defining_code.code_string` against this literal, per the class
/// description's `431&#124;persistent&#124;` and the `is_persistent()`
/// function definition ("True if category is `431|persistent|`").
const CATEGORY_CODE_PERSISTENT: &str = "431";

// TODO(port): forward-reference — `PARTY_PROXY` lives in rm.common.generic
// (PORT_MASTER_PLAN.md §7.1), not yet transcribed. Closed subtype set per
// ADR-001 §4, so the eventual type here will be an enum.
use crate::common::generic::party_proxy::PartyProxy;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sourced into the `TypeName` impl below (ADR-002).
pub const TYPE_NAME: &str = "COMPOSITION";

/// `COMPOSITION` — content of one version in a `VERSIONED_COMPOSITION`.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait), `LOCATABLE`'s state is embedded as
/// `pub locatable: LocatableData` rather than simulated via a Rust
/// supertrait, matching the sibling `EhrStatus`/`EhrAccess` shape.
/// `#[serde(flatten)]` folds those six attributes into `COMPOSITION`'s own
/// JSON object, per the ITS-JSON abstract-class-flattening rule.
///
/// TODO(port): P4 — the flatten below requires `LocatableData` to itself
/// derive `Serialize`/`Deserialize`; that is a sibling P4 wave over
/// `common/`, not yet landed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    /// Canonical `_type` discriminator (`"COMPOSITION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `LOCATABLE` state.
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `language`: mandatory indicator of the localised language in which
    /// this Composition is written. Coded from openEHR Code Set
    /// `languages`. The language of an Entry if different from the
    /// Composition is indicated in `ENTRY.language`.
    ///
    /// Invariant `Language_valid`: `code_set (Code_set_id_languages)
    /// .has_code (language)` — see
    /// [`Composition::invariant_language_valid`].
    pub language: CodePhrase,

    /// `territory`: name of territory in which this Composition was
    /// written. Coded from openEHR `countries` code set, which is an
    /// expression of the ISO 3166 standard.
    ///
    /// Invariant `Territory_valid`: `code_set (Code_set_id_countries)
    /// .has_code (territory)` — see
    /// [`Composition::invariant_territory_valid`].
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
    /// (Group_id_composition_category, category.defining_code)` — see
    /// [`Composition::invariant_category_validity`].
    pub category: DvCodedText,

    /// `context`: the clinical session context of this Composition, i.e.
    /// the contextual attributes of the clinical session.
    #[serde(skip_serializing_if = "Option::is_none", default)]
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
    /// content.is_empty` — see [`Composition::invariant_content_valid`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content: Option<Vec<super::content_item::ContentItem>>,
}

impl TypeName for Composition {
    const NAME: &'static str = TYPE_NAME;
}

impl Composition {
    /// `is_persistent` (): `Boolean`.
    ///
    /// True if `category` is `431|persistent|`, False otherwise. Useful
    /// for finding Compositions in an EHR which are guaranteed to be of
    /// interest to most users.
    ///
    /// Implemented per ADR-003 §8 as the direct code comparison the spec
    /// states — `category.defining_code.code_string = "431"` — using the
    /// [`CATEGORY_CODE_PERSISTENT`] literal.
    #[must_use]
    pub fn is_persistent(&self) -> bool {
        self.category.defining_code.code_string == CATEGORY_CODE_PERSISTENT
    }

    /// Invariant `Category_validity`: `terminology
    /// (Terminology_id_openehr).has_code_for_group_id
    /// (Group_id_composition_category, category.defining_code)`.
    ///
    /// Terminology-bound invariant (ADR-003 §8): takes the
    /// [`TerminologyService`] and checks that `category.defining_code` is a
    /// member of the openEHR `composition category` group.
    #[must_use]
    pub fn invariant_category_validity(&self, terminology: &TerminologyService) -> bool {
        terminology
            .terminology(OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR)
            .is_some_and(|access| {
                access.has_code_for_group_id(
                    OpenehrTerminologyGroupIdentifiers::GROUP_ID_COMPOSITION_CATEGORY,
                    &TerminologyCode::new(
                        self.category.defining_code.terminology_id.value(),
                        self.category.defining_code.code_string.clone(),
                    ),
                )
            })
    }

    /// Invariant `Territory_valid`: `code_set(Code_set_id_countries)
    /// .has_code(territory)`.
    ///
    /// Terminology-bound invariant (ADR-003 §8): checks `territory` against
    /// the openEHR `countries` code set (ISO 3166-1).
    #[must_use]
    pub fn invariant_territory_valid(&self, terminology: &TerminologyService) -> bool {
        terminology
            .code_set_for_id(OpenehrCodeSetIdentifiers::CODE_SET_ID_COUNTRIES)
            .is_some_and(|code_set| code_set.has_code(&self.territory.code_string))
    }

    /// Invariant `Language_valid`: `code_set(Code_set_id_languages)
    /// .has_code(language)`.
    ///
    /// Terminology-bound invariant (ADR-003 §8): checks `language` against
    /// the openEHR `languages` code set (ISO 639-1).
    #[must_use]
    pub fn invariant_language_valid(&self, terminology: &TerminologyService) -> bool {
        terminology
            .code_set_for_id(OpenehrCodeSetIdentifiers::CODE_SET_ID_LANGUAGES)
            .is_some_and(|code_set| code_set.has_code(&self.language.code_string))
    }

    /// Invariant `Content_valid`: `content /= Void implies not
    /// content.is_empty`.
    #[must_use]
    pub fn invariant_content_valid(&self) -> bool {
        self.content.as_ref().is_none_or(|c| !c.is_empty())
    }

    /// Invariant `Is_archetype_root`: `is_archetype_root`.
    ///
    /// Inherited unchanged from `LOCATABLE`; a `COMPOSITION` is always an
    /// archetype root. Implemented per ADR-003 §8 as the derived value
    /// `LOCATABLE.is_archetype_root` computes — `archetype_details /= Void`
    /// (see [`LocatableData`] / `LocatableApi::is_archetype_root`).
    #[must_use]
    pub fn invariant_is_archetype_root(&self) -> bool {
        self.locatable.archetype_details.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::Composition;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::common::generic::party_proxy::{PartyProxy, PartyProxyData};
    use crate::common::generic::party_self::PartySelf;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_coded_text::DvCodedText;
    use crate::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_base::identification::object_id::ObjectIdData;
    use openehr_base::identification::terminology_id::TerminologyId;
    use openehr_foundation::serde_support::TypeTag;
    use openehr_term::TerminologyService;

    fn code_phrase(terminology: &str, code: &str) -> CodePhrase {
        CodePhrase {
            type_tag: TypeTag::new(),
            terminology_id: TerminologyId {
                type_tag: TypeTag::new(),
                object_id: ObjectIdData {
                    value: terminology.to_string(),
                },
            },
            code_string: code.to_string(),
            preferred_term: None,
        }
    }

    fn coded_text(value: &str, terminology: &str, code: &str) -> DvCodedText {
        DvCodedText {
            type_tag: TypeTag::new(),
            text: DvTextData {
                value: value.to_string(),
                hyperlink: None,
                formatting: None,
                mappings: None,
                language: None,
                encoding: None,
            },
            defining_code: code_phrase(terminology, code),
        }
    }

    /// A `COMPOSITION` with the given `language`/`territory`/`category`
    /// codes; content is empty and the composer is `PARTY_SELF`.
    fn composition(language: &str, territory: &str, category_code: &str) -> Composition {
        Composition {
            type_tag: TypeTag::new(),
            locatable: LocatableData {
                name: DvText::Text {
                    type_tag: TypeTag::new(),
                    data: DvTextData {
                        value: "Encounter".to_string(),
                        hyperlink: None,
                        formatting: None,
                        mappings: None,
                        language: None,
                        encoding: None,
                    },
                },
                archetype_node_id: "openEHR-EHR-COMPOSITION.encounter.v1".to_string(),
                uid: None,
                links: None,
                archetype_details: None,
                feeder_audit: None,
                parent: None,
            },
            language: code_phrase("ISO_639-1", language),
            territory: code_phrase("ISO_3166-1", territory),
            category: coded_text("category", "openehr", category_code),
            context: None,
            composer: PartyProxy::PartySelf(PartySelf {
                type_tag: TypeTag::new(),
                party_proxy: PartyProxyData { external_ref: None },
            }),
            content: None,
        }
    }

    /// ADR-002 smoke test: a minimal `COMPOSITION` serializes with
    /// `"_type":"COMPOSITION"` as the first key, emitted by the
    /// `TypeTag<Self>` first field (not any struct-level rename, which was
    /// a verified no-op and has been deleted).
    #[test]
    fn serializes_with_type_composition_first() {
        let composition = composition("en", "NL", "433");

        let json = serde_json::to_string(&composition).expect("serialize");
        assert!(
            json.starts_with("{\"_type\":\"COMPOSITION\","),
            "canonical JSON must open with the _type discriminator, got: {json}"
        );

        let parsed: Composition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, composition);
    }

    /// `is_persistent()` is true only for the `431|persistent|` category.
    #[test]
    fn is_persistent_reflects_the_431_category() {
        assert!(composition("en", "NL", "431").is_persistent());
        assert!(!composition("en", "NL", "433").is_persistent()); // event
        assert!(!composition("en", "NL", "451").is_persistent()); // episodic
    }

    /// `Category_validity` passes for a real `composition category` code and
    /// fails for a bogus one, resolved against the bundled terminology.
    #[test]
    fn category_validity_checks_the_composition_category_group() {
        let terminology = TerminologyService::bundled().expect("bundled terminology parses");
        assert!(composition("en", "NL", "433").invariant_category_validity(terminology)); // event
        assert!(composition("en", "NL", "431").invariant_category_validity(terminology)); // persistent
        assert!(!composition("en", "NL", "999999").invariant_category_validity(terminology));
    }

    /// `Territory_valid` / `Language_valid` resolve against the ISO code
    /// sets, passing for real codes and failing for bogus ones.
    #[test]
    fn territory_and_language_validity_check_the_iso_code_sets() {
        let terminology = TerminologyService::bundled().expect("bundled terminology parses");
        let valid = composition("en", "NL", "433");
        assert!(valid.invariant_territory_valid(terminology));
        assert!(valid.invariant_language_valid(terminology));

        let bogus = composition("zz", "ZZ", "433");
        assert!(!bogus.invariant_territory_valid(terminology));
        assert!(!bogus.invariant_language_valid(terminology));
    }

    /// `Content_valid` (present-but-empty is invalid) and `Is_archetype_root`
    /// (derived from `archetype_details`).
    #[test]
    fn structural_invariants_hold_and_fail_as_specified() {
        let mut c = composition("en", "NL", "433");
        assert!(c.invariant_content_valid()); // content = None: valid
        c.content = Some(Vec::new()); // present-but-empty: invalid
        assert!(!c.invariant_content_valid());

        // No archetype_details on the fixture → not an archetype root.
        assert!(!composition("en", "NL", "433").invariant_is_archetype_root());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.composition — docs/research/spec-cache/RM-1.1.0/uml_classes/composition.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-composition_package.adoc §Class Descriptions / composition.adoc §COMPOSITION Class
//   confidence: high
//   todos: 2
//   note: LOCATABLE embedded per ADR-001 §3; content typed as Option<Vec<ContentItem>> using the ADR-001 §4 closed enum from content_item.rs. P5/ADR-003 §8: is_persistent() implemented (category code 431); Category_validity/Territory_valid/Language_valid implemented as terminology-bound checks taking &TerminologyService; Content_valid and Is_archetype_root implemented as structural checks; all five spec-listed invariants + is_persistent() now working, pinned by unit tests. Remaining 2 TODO(port) are the PARTY_PROXY forward-ref import comment and the P4 LocatableData-flatten note. P4/ADR-002: self-tagging TypeTag<Self> + TypeName; flatten kept on locatable.
// ─────────────────────────────────────────────
