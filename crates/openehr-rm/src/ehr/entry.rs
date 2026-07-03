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
use crate::common::generic::participation::Participation;
use crate::common::generic::party_proxy::PartyProxy;
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_base::identification::object_ref::ObjectRef;
use openehr_term::{CodeSetAccess, OpenehrCodeSetIdentifiers, TerminologyService};
use serde::{Deserialize, Serialize};

/// Embedded attribute state of the abstract `ENTRY` class.
///
/// Per ADR-001 §3, concrete `ENTRY` descendants (via [`super::care_entry`]
/// and [`super::admin_entry::AdminEntry`]) embed this struct by
/// composition rather than inheriting from it.
///
/// TODO(port): P4 — `#[serde(flatten)]` on `content_item` requires
/// `ContentItemData` (this same batch) to itself derive
/// `Serialize`/`Deserialize`, which in turn requires `LocatableData`
/// (sibling P4 wave over `common/`) to do the same.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryData {
    // NOTE: `ENTRY` inherits `CONTENT_ITEM` (in turn `LOCATABLE`), not
    // `PATHABLE` directly — this is the ordinary `LOCATABLE`-chain case,
    // unlike the settled `EVENT_CONTEXT`/`INSTRUCTION_DETAILS`/
    // `ISM_TRANSITION` hazard. `content_item` is the composed parent state.
    /// Embedded `CONTENT_ITEM` (in turn `LOCATABLE`) state.
    #[serde(flatten)]
    pub content_item: super::content_item::ContentItemData,

    /// `language`: mandatory indicator of the localised language in which
    /// this Entry is written. Coded from openEHR Code Set `languages`.
    ///
    /// Invariant `Language_valid`: `code_set (Code_set_id_languages)
    /// .has_code (language)` — see [`EntryApi::invariant_language_valid`].
    pub language: CodePhrase,

    /// `encoding`: name of character set in which text values in this
    /// Entry are encoded. Coded from openEHR Code Set `character sets`.
    ///
    /// Invariant `Encoding_valid`: `code_set (Code_set_id_character_sets)
    /// .has_code (encoding)` — see [`EntryApi::invariant_encoding_valid`].
    pub encoding: CodePhrase,

    /// `other_participations`: other participations at `ENTRY` level.
    ///
    /// Invariant `Other_participations_valid`: `other_participations /=
    /// Void implies not other_participations.is_empty` — see
    /// [`EntryApi::invariant_other_participations_valid`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub other_participations: Option<Vec<Participation>>,

    /// `workflow_id`: identifier of externally held workflow engine data
    /// for this workflow execution, for this subject of care.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub workflow_id: Option<ObjectRef>,

    /// `subject`: id of human subject of this `ENTRY`, e.g. organ donor,
    /// foetus, a family member, another clinically relevant person.
    ///
    /// Invariant `Subject_validity`: `subject_is_self implies
    /// subject.generating_type = "PARTY_SELF"` — see
    /// [`EntryApi::invariant_subject_validity`].
    pub subject: PartyProxy,

    /// `provider`: optional identification of provider of the information
    /// in this `ENTRY`, which might be the patient, a patient agent (e.g.
    /// parent, guardian), the clinician, or a device or software.
    /// Generally only used when the recorder needs to make it explicit.
    /// Otherwise, Composition composer and other participants are assumed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
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
    /// "PARTY_SELF"` — satisfied by construction, since this returns true
    /// exactly when `subject` is the `PARTY_SELF` variant of the
    /// [`PartyProxy`] closed enum (ADR-001 §4).
    fn subject_is_self(&self) -> bool {
        matches!(self.entry_data().subject, PartyProxy::PartySelf(_))
    }

    /// Invariant `Is_archetype_root`: `is_archetype_root`.
    ///
    /// Inherited unchanged from `LOCATABLE` via `CONTENT_ITEM`. Implemented
    /// per ADR-003 §8 as the derived value `LOCATABLE.is_archetype_root`
    /// computes — `archetype_details /= Void`, read through the embedded
    /// `CONTENT_ITEM`/`LOCATABLE` chain.
    fn invariant_is_archetype_root(&self) -> bool {
        self.entry_data()
            .content_item
            .locatable
            .archetype_details
            .is_some()
    }

    /// Invariant `Language_valid`: `code_set (Code_set_id_languages)
    /// .has_code (language)`.
    ///
    /// Terminology-bound invariant (ADR-003 §8): checks `language` against
    /// the openEHR `languages` code set (ISO 639-1).
    fn invariant_language_valid(&self, terminology: &TerminologyService) -> bool {
        terminology
            .code_set_for_id(OpenehrCodeSetIdentifiers::CODE_SET_ID_LANGUAGES)
            .is_some_and(|code_set| code_set.has_code(&self.entry_data().language.code_string))
    }

    /// Invariant `Encoding_valid`: `code_set (Code_set_id_character_sets)
    /// .has_code (encoding)`.
    ///
    /// Terminology-bound invariant (ADR-003 §8): checks `encoding` against
    /// the openEHR `character sets` code set.
    fn invariant_encoding_valid(&self, terminology: &TerminologyService) -> bool {
        terminology
            .code_set_for_id(OpenehrCodeSetIdentifiers::CODE_SET_ID_CHARACTER_SETS)
            .is_some_and(|code_set| code_set.has_code(&self.entry_data().encoding.code_string))
    }

    /// Invariant `Other_participations_valid`: `other_participations /= Void
    /// implies not other_participations.is_empty` (ADR-003 §8).
    fn invariant_other_participations_valid(&self) -> bool {
        self.entry_data()
            .other_participations
            .as_ref()
            .is_none_or(|p| !p.is_empty())
    }

    /// Invariant `Subject_validity`: `subject_is_self implies
    /// subject.generating_type = "PARTY_SELF"` (ADR-003 §8). Structurally
    /// guaranteed by [`EntryApi::subject_is_self`]'s definition, but
    /// evaluated literally here.
    fn invariant_subject_validity(&self) -> bool {
        !self.subject_is_self() || matches!(self.entry_data().subject, PartyProxy::PartySelf(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::common::generic::party_identified::{PartyIdentified, PartyIdentifiedData};
    use crate::common::generic::party_proxy::{PartyProxy, PartyProxyData};
    use crate::common::generic::party_self::PartySelf;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_base::identification::object_id::ObjectIdData;
    use openehr_base::identification::terminology_id::TerminologyId;
    use openehr_foundation::serde_support::TypeTag;
    use openehr_term::TerminologyService;

    /// Minimal concrete `EntryApi` implementor for exercising the trait's
    /// default methods (mirrors `locatable.rs`'s `TestLocatable`), avoiding
    /// the heavier concrete RM leaves and their `ItemStructure` fields.
    struct TestEntry {
        entry: EntryData,
    }

    impl EntryApi for TestEntry {
        fn entry_data(&self) -> &EntryData {
            &self.entry
        }
    }

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

    fn party_self() -> PartyProxy {
        PartyProxy::PartySelf(PartySelf {
            type_tag: TypeTag::new(),
            party_proxy: PartyProxyData { external_ref: None },
        })
    }

    fn party_identified() -> PartyProxy {
        PartyProxy::PartyIdentified(PartyIdentified {
            type_tag: TypeTag::new(),
            data: PartyIdentifiedData {
                party_proxy: PartyProxyData { external_ref: None },
                name: Some("Dr Marlowe".to_string()),
                identifiers: None,
            },
        })
    }

    fn test_entry(language: &str, encoding: &str, subject: PartyProxy) -> TestEntry {
        TestEntry {
            entry: EntryData {
                content_item: super::super::content_item::ContentItemData {
                    locatable: LocatableData {
                        name: DvText::Text {
                            type_tag: TypeTag::new(),
                            data: DvTextData {
                                value: "Entry".to_string(),
                                hyperlink: None,
                                formatting: None,
                                mappings: None,
                                language: None,
                                encoding: None,
                            },
                        },
                        archetype_node_id: "at0000".to_string(),
                        uid: None,
                        links: None,
                        archetype_details: None,
                        feeder_audit: None,
                        parent: None,
                    },
                },
                language: code_phrase("ISO_639-1", language),
                encoding: code_phrase("IANA_character-sets", encoding),
                other_participations: None,
                workflow_id: None,
                subject,
                provider: None,
            },
        }
    }

    #[test]
    fn subject_is_self_reflects_the_party_proxy_variant() {
        assert!(test_entry("en", "UTF-8", party_self()).subject_is_self());
        assert!(!test_entry("en", "UTF-8", party_identified()).subject_is_self());
    }

    #[test]
    fn subject_validity_holds_for_both_variants() {
        // Subject_validity is structurally guaranteed either way.
        assert!(test_entry("en", "UTF-8", party_self()).invariant_subject_validity());
        assert!(test_entry("en", "UTF-8", party_identified()).invariant_subject_validity());
    }

    #[test]
    fn language_and_encoding_validity_check_the_code_sets() {
        let terminology = TerminologyService::bundled().expect("bundled terminology parses");
        let valid = test_entry("en", "UTF-8", party_self());
        assert!(valid.invariant_language_valid(terminology));
        assert!(valid.invariant_encoding_valid(terminology));

        let bogus = test_entry("zz", "NOT-A-CHARSET", party_self());
        assert!(!bogus.invariant_language_valid(terminology));
        assert!(!bogus.invariant_encoding_valid(terminology));
    }

    #[test]
    fn other_participations_and_archetype_root_invariants() {
        let mut e = test_entry("en", "UTF-8", party_self());
        assert!(e.invariant_other_participations_valid()); // None: valid
        e.entry.other_participations = Some(Vec::new()); // present-but-empty: invalid
        assert!(!e.invariant_other_participations_valid());

        // No archetype_details → not an archetype root.
        assert!(!test_entry("en", "UTF-8", party_self()).invariant_is_archetype_root());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/entry.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / entry.adoc §ENTRY Class
//   confidence: high
//   todos: 1
//   note: abstract-with-attributes per ADR-001 §3 (EntryData + EntryApi); content_item field composes the CONTENT_ITEM/LOCATABLE chain. P5/ADR-003 §8: EntryApi default methods implemented — subject_is_self() (PartyProxy::PartySelf match), invariant_is_archetype_root (derived), Language_valid/Encoding_valid (terminology-bound, &TerminologyService), Other_participations_valid and Subject_validity (structural), pinned by tests via an in-module TestEntry implementor. Sole remaining TODO(port) is the P4 flatten scaffolding note. P4: serde derives (flatten on content_item); no _type of its own (embedded-only).
// ─────────────────────────────────────────────
