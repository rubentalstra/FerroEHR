//! `AUTHORED_RESOURCE` — abstract idea of an online resource created by a
//! human author.
//!
//! openEHR class: `AUTHORED_RESOURCE` (abstract), package `base.resource`.
//!
//! Authored resources contain natural language elements and are therefore
//! created in some original language, recorded in `original_language`.
//! Information about translations is held in `translations`, allowing one or
//! more sets of translation details to be recorded. A resource is
//! translated by: translating every language-dependent element to the new
//! language; adding a new `TRANSLATION_DETAILS` instance to `translations`
//! (details about the translator, organisation, quality assurance, etc.);
//! and applying any further translations to language-specific elements in
//! instances of descendant types of `AUTHORED_RESOURCE`.
//!
//! What is normally considered the resource's "meta-data" — author, date of
//! creation, purpose, and other descriptive items — is described by
//! `description` (a `RESOURCE_DESCRIPTION`), which is optional so resources
//! with no meta-data at all (e.g. in a partial state of construction) remain
//! representable. `translations` may still be required even without
//! `description`, since other parts of a descendant type may be
//! language-dependent.
//!
//! When a resource is considered to be in a state where changes to it
//! should be controlled, `is_controlled` is set to `true`, and all
//! subsequent changes should have an audit trail recorded via
//! `revision_history` — a documentary copy of the revision history as known
//! inside the managing repository, for the benefit of resource users. Every
//! change to a resource committed to the relevant repository causes a new
//! addition to `revision_history`.
use std::collections::HashMap;

use super::resource_annotations::ResourceAnnotations;
use super::resource_description::ResourceDescription;
use super::translation_details::TranslationDetails;

// `UUID` (BASE 1.2.0 base_types.identification) and `Terminology_code`
// (BASE 1.2.0 foundation_types.terminology) are now real types in this
// workspace; the former `String` placeholder aliases are resolved to them
// (the same reconciliation already applied to `resource_description.rs` /
// `translation_details.rs` at P4 — see ROSETTA).
use crate::identification::uuid::Uuid;
use openehr_foundation::terminology_types::terminology_code::TerminologyCode;

// TODO(port): `REVISION_HISTORY` is defined in RM 1.1.0 `rm.common`
// (PORT_MASTER_PLAN.md §7.1), not BASE — `openehr-base` sits *below*
// `openehr-rm` in the dependency graph and cannot name the real class.
// Placeholder alias until the P17 layering reconciliation wires
// `openehr-rm::common::revision_history` back into this field (the same
// kind of upward-reference seam already flagged on
// `EXTERNAL_ENVIRONMENT_ACCESS`); the `revision_history.most_recent_version`
// half of `current_revision()` is blocked on the same reconciliation and
// until then the placeholder string is returned verbatim there.
//
// PORT NOTE: `revision_history` is referenced by name three times in the
// cached spec text — the chapter prose ("The revision_history attribute
// defined in the AUTHORED_RESOURCE class..."), the `current_revision()`
// function's postcondition (`Result = revision_history.most_recent_version`),
// and the `Revision_history_valid` invariant
// (`is_controlled xor revision_history = Void`) — but it is **not** listed
// as a row in the published `AUTHORED_RESOURCE` Attributes table itself
// (docs/research/spec-cache/BASE-1.2.0/uml_classes/authored_resource.adoc).
// This looks like an editorial omission in the attributes table rather than
// the attribute genuinely not existing (three independent parts of the same
// class description presuppose it). Modelled here as a real field, per the
// invoking task's own explicit anticipation of this class
// ("REVISION_HISTORY from RM common — that is RM 1.1.0, NOT this phase").
type RevisionHistory = String;

/// `AUTHORED_RESOURCE` — abstract idea of an online resource created by a
/// human author.
///
/// # Transcription approach
///
/// Abstract class with attributes (ADR-001 §3 / `.claude/rules/rm-
/// transcription.md`): transcribed as a plain struct holding the class's own
/// state, with `AuthoredResourceBehaviour` as the marker trait a concrete
/// descendant (e.g. an `ARCHETYPE`-family class in the AM specification,
/// outside BASE 1.2.0's scope) would implement and/or embed by composition
/// once such a descendant is transcribed. No concrete subtype of
/// `AUTHORED_RESOURCE` exists within the BASE 1.2.0 `resource` package
/// itself.
///
/// `Hash<K,V>` attributes map to `HashMap<K,V>`, `List<T>` to `Vec<T>`, per
/// `docs/PORTING.md` §6/§14.2.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthoredResource {
    /// `uid`: `UUID`, cardinality 0..1.
    ///
    /// Unique identifier of the family of archetypes having the same
    /// interface identifier (same major version).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub uid: Option<Uuid>,

    /// `original_language`: `Terminology_code`, cardinality 1..1.
    ///
    /// Language in which this resource was initially authored. Although
    /// there is no language primacy of resources overall, the language of
    /// original authoring is required to ensure natural language
    /// translations can preserve quality. Language is relevant in both the
    /// description and ontology sections.
    pub original_language: TerminologyCode,

    /// `description`: `RESOURCE_DESCRIPTION`, cardinality 0..1.
    ///
    /// Description and lifecycle information of the resource.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<ResourceDescription>,

    /// `is_controlled`: `Boolean`, cardinality 0..1.
    ///
    /// `true` if this resource is under any kind of change control (even
    /// file copying), in which case revision history is created.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub is_controlled: Option<bool>,

    /// `annotations`: `RESOURCE_ANNOTATIONS`, cardinality 0..1.
    ///
    /// Annotations on individual items within the resource, keyed by path.
    /// The inner table takes the form of a Hash table of String values
    /// keyed by String tags.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub annotations: Option<ResourceAnnotations>,

    /// `translations`: `Hash<String, TRANSLATION_DETAILS>`, cardinality
    /// 0..1.
    ///
    /// List of details for each natural translation made of this resource,
    /// keyed by language code. For each translation listed here, there must
    /// be corresponding sections in all language-dependent parts of the
    /// resource. `original_language` does not appear in this list.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub translations: Option<HashMap<String, TranslationDetails>>,

    /// `revision_history`: `REVISION_HISTORY`, cardinality implied 0..1 by
    /// the `Revision_history_valid` invariant.
    ///
    /// See the type-level `RevisionHistory` doc comment (`PORT NOTE`) above
    /// for the discrepancy between this attribute's presence in spec prose
    /// and function/invariant text versus its absence from the published
    /// Attributes table row list.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub revision_history: Option<RevisionHistory>,
}

/// Marker/behaviour trait for `AUTHORED_RESOURCE` descendants.
///
/// Exposes the class's function signatures so a concrete embedding type can
/// stay polymorphic over "is an authored resource" without inheriting a
/// struct directly (composition, per the RM transcription rules). No
/// concrete `AUTHORED_RESOURCE` descendant exists within BASE 1.2.0 itself;
/// this trait is implemented by `AuthoredResource` for the abstract class's
/// own default behaviour, and by future descendant types (outside this
/// crate's current scope) via an embedded `AuthoredResource` field.
pub trait AuthoredResourceBehaviour {
    /// `current_revision` (): `String`.
    ///
    /// __Post__: `Result = revision_history.most_recent_version`.
    ///
    /// Most recent revision in `revision_history` if `is_controlled`, else
    /// `"(uncontrolled)"`.
    fn current_revision(&self) -> String;

    /// `languages_available` (): `List<String>`.
    ///
    /// Total list of languages available in this resource, derived from
    /// `original_language` and `translations`.
    fn languages_available(&self) -> Vec<String>;
}

impl AuthoredResourceBehaviour for AuthoredResource {
    fn current_revision(&self) -> String {
        // "Most recent revision in revision_history if is_controlled else
        // '(uncontrolled)'". An absent `is_controlled` (0..1) reads as not
        // controlled, consistently with `Revision_history_valid` treating
        // the uncontrolled state as revision-history-free.
        if self.is_controlled.unwrap_or(false) {
            // Post: `Result = revision_history.most_recent_version` — see
            // the TODO(port) on the `RevisionHistory` alias above (P17
            // layering reconciliation); the placeholder string is the
            // whole history for now and is returned verbatim.
            self.revision_history.clone().unwrap_or_default()
        } else {
            "(uncontrolled)".to_string()
        }
    }

    fn languages_available(&self) -> Vec<String> {
        // Total list of languages: `original_language` plus the keys of
        // `translations` (which, per `Translations_valid`, never include
        // the original language). Translation keys are sorted for a
        // deterministic result — `Hash` iteration order is unspecified in
        // both the spec's `Hash<K,V>` and Rust's `HashMap`.
        let mut languages = vec![self.original_language.code_string.0.clone()];
        if let Some(translations) = &self.translations {
            let mut keys: Vec<String> = translations.keys().cloned().collect();
            keys.sort();
            languages.extend(keys);
        }
        languages
    }
}

/// Class invariants, transcribed as working validity methods per ADR-003
/// decision 8 (invariants become `is_*_valid()`-family methods now; the
/// deep walker/accumulator validation framework remains the P11
/// deliverable, and will call these directly).
impl AuthoredResource {
    /// Invariant `Original_language_valid`:
    /// `code_set(Code_set_id_languages).has_code(original_language.as_string)`.
    ///
    /// PORT NOTE: the languages code set lives in `openehr-terminology`,
    /// which depends *on* this crate — the dependency cannot point upward,
    /// so the code-set membership test is injected as a predicate instead
    /// of a `&TerminologyService` parameter (ADR-003 §8's shape, inverted
    /// one level). A caller passes e.g.
    /// `|code| code_set_languages.has_code(code)`.
    pub fn is_original_language_valid(
        &self,
        languages_code_set_has_code: impl FnOnce(&TerminologyCode) -> bool,
    ) -> bool {
        languages_code_set_has_code(&self.original_language)
    }

    /// Invariant `Current_revision_valid`:
    /// `(current_revision /= Void and not is_controlled) implies
    /// current_revision.is_equal("(uncontrolled)")`.
    pub fn is_current_revision_valid(&self) -> bool {
        self.is_controlled.unwrap_or(false) || self.current_revision() == "(uncontrolled)"
    }

    /// Invariant `Translations_valid`:
    /// `translations /= Void implies (not translations.is_empty and not
    /// translations.has(original_language.code_string))`.
    pub fn is_translations_valid(&self) -> bool {
        self.translations.as_ref().is_none_or(|translations| {
            !translations.is_empty()
                && !translations.contains_key(&self.original_language.code_string.0)
        })
    }

    /// Invariant `Description_valid`:
    /// `translations /= Void implies (description.details.for_all(d |
    /// translations.has_key(d.language.code_string)))`.
    ///
    /// PORT NOTE: the invariant text dereferences `description.details`
    /// unconditionally even though both `description` (0..1) and its
    /// `details` (0..1) are optional; an absent `description`/`details`
    /// makes the `for_all` range empty, so it is read as vacuously true
    /// rather than as a Void-call failure.
    pub fn is_description_valid(&self) -> bool {
        let Some(translations) = &self.translations else {
            return true;
        };
        self.description
            .as_ref()
            .and_then(|description| description.details.as_ref())
            .is_none_or(|details| {
                details
                    .values()
                    .all(|d| translations.contains_key(&d.language.code_string.0))
            })
    }

    /// Invariant `Languages_available_valid`:
    /// `languages_available.has(original_language)`.
    pub fn is_languages_available_valid(&self) -> bool {
        self.languages_available()
            .contains(&self.original_language.code_string.0)
    }

    /// Invariant `Revision_history_valid`:
    /// `is_controlled xor revision_history = Void`.
    pub fn is_revision_history_valid(&self) -> bool {
        self.is_controlled.unwrap_or(false) != self.revision_history.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openehr_foundation::primitive_types::string::OpenEhrString;

    fn language(code: &str) -> TerminologyCode {
        TerminologyCode {
            terminology_id: OpenEhrString("ISO_639-1".to_string()),
            terminology_version: None,
            code_string: OpenEhrString(code.to_string()),
            uri: None,
        }
    }

    fn resource() -> AuthoredResource {
        AuthoredResource {
            uid: None,
            original_language: language("en"),
            description: None,
            is_controlled: None,
            annotations: None,
            translations: None,
            revision_history: None,
        }
    }

    fn translation(code: &str) -> (String, TranslationDetails) {
        (
            code.to_string(),
            TranslationDetails {
                type_tag: openehr_foundation::serde_support::TypeTag::new(),
                language: language(code),
                author: HashMap::new(),
                accreditation: None,
                other_details: None,
                version_last_translated: None,
                other_contributors: None,
            },
        )
    }

    #[test]
    fn languages_available_is_original_language_plus_translation_keys() {
        let mut resource = resource();
        assert_eq!(resource.languages_available(), vec!["en".to_string()]);

        resource.translations = Some([translation("de"), translation("nl")].into());
        assert_eq!(
            resource.languages_available(),
            vec!["en".to_string(), "de".to_string(), "nl".to_string()]
        );
        assert!(resource.is_languages_available_valid());
    }

    #[test]
    fn current_revision_is_uncontrolled_when_not_controlled() {
        let mut resource = resource();
        assert_eq!(resource.current_revision(), "(uncontrolled)");
        assert!(resource.is_current_revision_valid());

        resource.is_controlled = Some(false);
        assert_eq!(resource.current_revision(), "(uncontrolled)");
        assert!(resource.is_current_revision_valid());

        resource.is_controlled = Some(true);
        resource.revision_history = Some("1.0.1".to_string());
        assert_eq!(resource.current_revision(), "1.0.1");
        assert!(resource.is_current_revision_valid());
    }

    #[test]
    fn translations_valid_rejects_empty_and_original_language_entries() {
        let mut resource = resource();
        assert!(resource.is_translations_valid());

        resource.translations = Some(HashMap::new());
        assert!(!resource.is_translations_valid());

        resource.translations = Some([translation("en")].into());
        assert!(!resource.is_translations_valid());

        resource.translations = Some([translation("de")].into());
        assert!(resource.is_translations_valid());
    }

    #[test]
    fn revision_history_valid_is_an_xor_with_is_controlled() {
        let mut resource = resource();
        // Not controlled, no history: valid.
        assert!(resource.is_revision_history_valid());
        // Controlled but no history: invalid.
        resource.is_controlled = Some(true);
        assert!(!resource.is_revision_history_valid());
        // Controlled with history: valid.
        resource.revision_history = Some("1".to_string());
        assert!(resource.is_revision_history_valid());
        // Not controlled with history: invalid.
        resource.is_controlled = Some(false);
        assert!(!resource.is_revision_history_valid());
    }

    #[test]
    fn original_language_valid_delegates_to_the_injected_code_set() {
        let resource = resource();
        assert!(resource.is_original_language_valid(|code| code.code_string.0 == "en"));
        assert!(!resource.is_original_language_valid(|_| false));
    }

    #[test]
    fn description_valid_requires_detail_languages_to_be_translated() {
        use crate::resource::resource_description_item::ResourceDescriptionItem;
        use openehr_foundation::serde_support::TypeTag;
        use std::sync::Weak;

        let item = |code: &str| ResourceDescriptionItem {
            type_tag: TypeTag::new(),
            language: language(code),
            purpose: "test".to_string(),
            keywords: None,
            use_: None,
            misuse: None,
            original_resource_uri: None,
            other_details: None,
        };
        let description = |codes: &[&str]| ResourceDescription {
            type_tag: TypeTag::new(),
            original_author: HashMap::new(),
            original_namespace: None,
            original_publisher: None,
            other_contributors: None,
            lifecycle_state: language("published"),
            parent_resource: Weak::new(),
            custodian_namespace: None,
            custodian_organisation: None,
            copyright: None,
            licence: None,
            ip_acknowledgements: None,
            references: None,
            resource_package_uri: None,
            conversion_details: None,
            other_details: None,
            details: Some(
                codes
                    .iter()
                    .map(|code| ((*code).to_string(), item(code)))
                    .collect(),
            ),
        };

        let mut resource = resource();
        // No translations: vacuously valid.
        resource.description = Some(description(&["de"]));
        assert!(resource.is_description_valid());

        // Translations present and covering every details language: valid.
        resource.translations = Some([translation("de")].into());
        assert!(resource.is_description_valid());

        // A details language with no matching translation entry: invalid.
        resource.description = Some(description(&["de", "fr"]));
        assert!(!resource.is_description_valid());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 resource §AUTHORED_RESOURCE — docs/research/spec-cache/BASE-1.2.0/uml_classes/authored_resource.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master02-resource_package.adoc §Overview + §Class Descriptions / authored_resource.adoc §AUTHORED_RESOURCE Class
//   confidence: medium
//   todos: 1
//   note: revision_history added despite being absent from the published Attributes table row list (present in prose/function/invariant text — flagged as likely editorial gap); uid/original_language now use the real Uuid/Terminology_code types; REVISION_HISTORY stays a String placeholder (sole TODO — RM-layer class, P17 layering reconciliation); current_revision/languages_available implemented, invariants are working is_*_valid methods per ADR-003 §8 (Original_language_valid takes an injected code-set predicate since openehr-terminology sits above this crate).
// ─────────────────────────────────────────────
