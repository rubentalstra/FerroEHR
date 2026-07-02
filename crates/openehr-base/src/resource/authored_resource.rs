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

// TODO(port): `UUID` is BASE 1.2.0 `base_types.identification` and has not
// yet been transcribed into `openehr-base` in this worktree (identification
// is scoped to a separate transcription pass within P1, not this resource-
// package pass). Placeholder alias over `std::string::String` until that
// class exists.
type Uuid = String;

// TODO(port): `Terminology_code` is BASE 1.2.0 `foundation_types.primitive_types`
// and has not yet been transcribed into `openehr-foundation` in this
// worktree. Placeholder alias until that class exists; see the identical
// note in `translation_details.rs`.
type TerminologyCode = String;

// TODO(port): `REVISION_HISTORY` is defined in RM 1.1.0 `rm.common`
// (PORT_MASTER_PLAN.md §7.1), not BASE — it is out of scope for this
// resource-package transcription pass (P3, not P1). Placeholder alias until
// `openehr-rm::common::revision_history` exists.
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
        // TODO(port): depends on `RevisionHistory::most_recent_version`,
        // which cannot be implemented until `REVISION_HISTORY` (RM common,
        // P3) is transcribed. Per the `Current_revision_valid` invariant,
        // the non-controlled case must return exactly `"(uncontrolled)"`.
        todo!("depends on REVISION_HISTORY (RM common, P3) and its most_recent_version function")
    }

    fn languages_available(&self) -> Vec<String> {
        // TODO(port): derive from `original_language` plus the keys of
        // `translations`, per the `Languages_available_valid` invariant
        // (`languages_available.has(original_language)`).
        todo!("derive from original_language + translations.keys()")
    }
}

// Invariants (from the spec table; not yet enforced — see `Validate` note
// below):
//
// - `Original_language_valid`:
//   `code_set(Code_set_id_languages).has_code(original_language.as_string)`.
// - `Current_revision_valid`:
//   `(current_revision /= Void and not is_controlled) implies
//   current_revision.is_equal("(uncontrolled)")`.
// - `Translations_valid`:
//   `translations /= Void implies (not translations.is_empty and not
//   translations.has(original_language.code_string))`.
// - `Description_valid`:
//   `translations /= Void implies (description.details.for_all(d |
//   translations.has_key(d.language.code_string)))`.
// - `Languages_available_valid`:
//   `languages_available.has(original_language)`.
// - `Revision_history_valid`: `is_controlled xor revision_history = Void`.
//
// TODO(port): model these as a `Validate` impl (context + path + error
// accumulator, per `.claude/rules/rm-transcription.md` "Invariants") once
// `Terminology_code`, `Uuid`, and `RevisionHistory` are real types rather
// than `String` placeholders — several invariants (`Original_language_valid`,
// `Translations_valid`, `Description_valid`) need real terminology-service
// and code-string operations that do not exist yet at this layer.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 resource §AUTHORED_RESOURCE — docs/research/spec-cache/BASE-1.2.0/uml_classes/authored_resource.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master02-resource_package.adoc §Overview + §Class Descriptions / authored_resource.adoc §AUTHORED_RESOURCE Class
//   confidence: medium
//   todos: 6
//   note: revision_history added despite being absent from the published Attributes table row list (present in prose/function/invariant text — flagged as likely editorial gap); UUID, Terminology_code, and REVISION_HISTORY are all cross-package placeholders pending later phases; no Validate impl yet, invariants left as doc comments.
// ─────────────────────────────────────────────
