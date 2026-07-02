//! `RESOURCE_DESCRIPTION_ITEM` — language-specific detail of a resource
//! description.
//!
//! openEHR class: `RESOURCE_DESCRIPTION_ITEM` (concrete), package
//! `base.resource`.
//!
//! Holds the natural-language-dependent parts of a `RESOURCE_DESCRIPTION`
//! for one language. If a `RESOURCE_DESCRIPTION` has more than one
//! `RESOURCE_DESCRIPTION_ITEM`, each carries the same information in a
//! different natural language; when a resource is translated for use in
//! another language environment, each `RESOURCE_DESCRIPTION_ITEM` needs to
//! be copied and translated into the new language.
use openehr_foundation::serde_support::{TypeName, TypeTag};
use std::collections::HashMap;

// TODO(port): `Terminology_code` is BASE 1.2.0 `foundation_types.primitive_types`
// (docs/research/spec-cache/BASE-1.2.0/uml_classes/terminology_code.adoc) and has
// not yet been transcribed into `openehr-foundation` in this worktree.
// Placeholder alias until that class exists; see the identical note in
// `translation_details.rs`.
type TerminologyCode = String;

/// `RESOURCE_DESCRIPTION_ITEM` — language-specific detail of resource
/// description.
///
/// # Transcription approach
///
/// Concrete class with no ancestors in the spec table (no `Inherit` row).
/// `Hash<String, String>` and `List<String>` attributes map per
/// `docs/PORTING.md` §6/§14.2.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResourceDescriptionItem {
    /// Canonical `_type` discriminator (`"RESOURCE_DESCRIPTION_ITEM"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `language`: `Terminology_code`, cardinality 1..1.
    ///
    /// The localised language in which the items in this description item
    /// are written. Coded using ISO 639-1 (2 character) language codes.
    pub language: TerminologyCode,

    /// `purpose`: `String`, cardinality 1..1.
    ///
    /// Purpose of the resource.
    pub purpose: String,

    /// `keywords`: `List<String>`, cardinality 0..1.
    ///
    /// Keywords which characterise this resource, used e.g. for indexing
    /// and searching.
    pub keywords: Option<Vec<String>>,

    /// `use`: `String`, cardinality 0..1.
    ///
    /// Description of the uses of the resource, i.e. contexts in which it
    /// could be used.
    ///
    /// PORT NOTE: the spec attribute name `use` is a Rust reserved keyword
    /// (`use` import statement); the field is renamed `use_` with an
    /// explicit serde rename back to the spec's `use` on the wire.
    #[serde(rename = "use")]
    pub use_: Option<String>,

    /// `misuse`: `String`, cardinality 0..1.
    ///
    /// Description of any misuses of the resource, i.e. contexts in which
    /// it should not be used.
    pub misuse: Option<String>,

    /// `original_resource_uri`: `Hash<String, String>`, cardinality 0..1.
    ///
    /// URIs of original clinical document(s) or description of which
    /// resource is a formalisation, in the language of this description
    /// item; keyed by meaning.
    pub original_resource_uri: Option<HashMap<String, String>>,

    /// `other_details`: `Hash<String, String>`, cardinality 0..1.
    ///
    /// Additional language-sensitive resource metadata, as a list of
    /// name/value pairs.
    pub other_details: Option<HashMap<String, String>>,
}

impl TypeName for ResourceDescriptionItem {
    const NAME: &'static str = "RESOURCE_DESCRIPTION_ITEM";
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 resource §RESOURCE_DESCRIPTION_ITEM — docs/research/spec-cache/BASE-1.2.0/uml_classes/resource_description_item.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master02-resource_package.adoc §Class Descriptions / resource_description_item.adoc §RESOURCE_DESCRIPTION_ITEM Class
//   confidence: high
//   todos: 1
//   note: no invariants published for this class in the spec table; `use` field renamed `use_` to avoid the Rust keyword, serde rename preserves the wire name.
// ─────────────────────────────────────────────
