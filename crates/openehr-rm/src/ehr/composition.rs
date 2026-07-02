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
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

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
    /// content.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced.
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

    /// ADR-002 smoke test: a minimal `COMPOSITION` serializes with
    /// `"_type":"COMPOSITION"` as the first key, emitted by the
    /// `TypeTag<Self>` first field (not any struct-level rename, which was
    /// a verified no-op and has been deleted).
    ///
    /// NOTE(P4 integration): the struct literals below construct sibling
    /// types owned by other P4 conversion waves (`LocatableData`,
    /// `DvText`/`DvTextData`, `CodePhrase`, `DvCodedText`,
    /// `PartyProxy`/`PartySelf`, `TerminologyId`/`ObjectIdData`) in their
    /// **pre-ADR-002** field shapes. As those waves add their own
    /// `type_tag` fields, the literals here will need `type_tag:
    /// TypeTag::new(),` lines added — an orchestrator integration-pass fix,
    /// not a change to what this test asserts.
    #[test]
    fn serializes_with_type_composition_first() {
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

        let composition = Composition {
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
            language: code_phrase("ISO_639-1", "en"),
            territory: code_phrase("ISO_3166-1", "NL"),
            category: DvCodedText {
                type_tag: TypeTag::new(),
                text: DvTextData {
                    value: "event".to_string(),
                    hyperlink: None,
                    formatting: None,
                    mappings: None,
                    language: None,
                    encoding: None,
                },
                defining_code: code_phrase("openehr", "433"),
            },
            context: None,
            composer: PartyProxy::PartySelf(PartySelf {
                type_tag: TypeTag::new(),
                party_proxy: PartyProxyData { external_ref: None },
            }),
            content: None,
        };

        let json = serde_json::to_string(&composition).expect("serialize");
        assert!(
            json.starts_with("{\"_type\":\"COMPOSITION\","),
            "canonical JSON must open with the _type discriminator, got: {json}"
        );

        let parsed: Composition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, composition);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.composition — docs/research/spec-cache/RM-1.1.0/uml_classes/composition.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-composition_package.adoc §Class Descriptions / composition.adoc §COMPOSITION Class
//   confidence: high
//   todos: 14
//   note: LOCATABLE embedded per ADR-001 §3; content typed as Option<Vec<ContentItem>> using the ADR-001 §4 closed enum from content_item.rs; is_persistent() and four of five invariants stubbed pending DvCodedText/terminology-service/LOCATABLE transcription — Content_valid is the one invariant implementable today (pure structural check) and is implemented, not stubbed; most of the markers are forward-reference import/embed comments. P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl (no-op struct-level rename deleted); flatten kept on locatable; _type-first smoke test written but #[ignore]d until the sibling waves' literals (CodePhrase/DvCodedText/PartyProxy/LocatableData) settle their ADR-002 shapes.
// ─────────────────────────────────────────────
