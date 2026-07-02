//! `TRANSLATION_DETAILS` — details of a natural language translation.
//!
//! openEHR class: `TRANSLATION_DETAILS` (concrete), package `base.resource`.
//!
//! Records who translated an `AUTHORED_RESOURCE` into a given language, and
//! any other meta-data about that translation, so that a translated resource
//! carries a documentary record of its provenance alongside the translated
//! content itself.
use std::collections::HashMap;

// TODO(port): `Terminology_code` is BASE 1.2.0 `foundation_types.primitive_types`
// (docs/research/spec-cache/BASE-1.2.0/uml_classes/terminology_code.adoc) and has
// not yet been transcribed into `openehr-foundation` in this worktree.
// Placeholder alias over `std::string::String` until that class exists;
// replace with the real `openehr_foundation::primitive_types::...` type once
// foundation_types.primitive_types transcribes `Terminology_code`.
type TerminologyCode = String;

/// `TRANSLATION_DETAILS` — class providing details of a natural language
/// translation.
///
/// # Transcription approach
///
/// Concrete class with no ancestors in the spec table (no `Inherit` row), so
/// this is a plain struct. All `Hash<String, String>` attributes map to
/// `HashMap<String, String>` per `docs/PORTING.md` §6/§14.2; `List<String>`
/// maps to `Vec<String>`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename = "TRANSLATION_DETAILS")]
pub struct TranslationDetails {
    /// `language`: `Terminology_code`, cardinality 1..1.
    ///
    /// Language of the translation, coded using ISO 639-1 (2 character)
    /// language codes.
    pub language: TerminologyCode,

    /// `author`: `Hash<String, String>`, cardinality 1..1.
    ///
    /// Primary translator name and other demographic details.
    pub author: HashMap<String, String>,

    /// `accreditation`: `String`, cardinality 0..1.
    ///
    /// Accreditation of primary translator or group, usually a national
    /// translator's registration or association membership id.
    pub accreditation: Option<String>,

    /// `other_details`: `Hash<String, String>`, cardinality 0..1.
    ///
    /// Any other meta-data.
    pub other_details: Option<HashMap<String, String>>,

    /// `version_last_translated`: `String`, cardinality 0..1.
    ///
    /// Version of this resource last time it was translated into the
    /// language represented by this `TRANSLATION_DETAILS` object.
    pub version_last_translated: Option<String>,

    /// `other_contributors`: `List<String>`, cardinality 0..1.
    ///
    /// Additional contributors to this translation, each listed in the
    /// preferred format of the relevant organisation for the artefacts in
    /// question. A typical default is `"name <email>"` if nothing else is
    /// specified.
    pub other_contributors: Option<Vec<String>>,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 resource §TRANSLATION_DETAILS — docs/research/spec-cache/BASE-1.2.0/uml_classes/translation_details.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master02-resource_package.adoc §Class Descriptions / translation_details.adoc §TRANSLATION_DETAILS Class
//   confidence: high
//   todos: 1
//   note: no invariants published for this class in the spec table; Terminology_code placeholder to resolve once foundation_types.primitive_types transcribes it.
// ─────────────────────────────────────────────
