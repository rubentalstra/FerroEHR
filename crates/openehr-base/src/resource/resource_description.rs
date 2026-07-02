//! `RESOURCE_DESCRIPTION` — descriptive meta-data of a resource.
//!
//! openEHR class: `RESOURCE_DESCRIPTION` (concrete), package `base.resource`.
//!
//! What is normally considered the "meta-data" of a resource — its author,
//! date of creation, purpose, and other descriptive items — is described by
//! this class together with `RESOURCE_DESCRIPTION_ITEM`. The parts that are
//! in natural language, and therefore may require translated versions, live
//! in `details` (keyed by language code, one `RESOURCE_DESCRIPTION_ITEM`
//! per language); each item under `details` should carry exactly the same
//! information in a different natural language.
use std::collections::HashMap;
use std::sync::Weak;

use super::authored_resource::AuthoredResource;
use super::resource_description_item::ResourceDescriptionItem;

// TODO(port): `Terminology_code` is BASE 1.2.0 `foundation_types.primitive_types`
// and has not yet been transcribed into `openehr-foundation` in this
// worktree. Placeholder alias until that class exists; see the identical
// note in `translation_details.rs`.
type TerminologyCode = String;

/// `RESOURCE_DESCRIPTION` — defines the descriptive meta-data of a resource.
///
/// # Transcription approach
///
/// Concrete class with no ancestors in the spec table (no `Inherit` row).
///
/// `parent_resource`: `AUTHORED_RESOURCE`, cardinality 1..1, `{default = }`.
/// The spec table types this as a reference "to owning resource" — i.e. the
/// `RESOURCE_DESCRIPTION` is contained by an `AUTHORED_RESOURCE`
/// (`AUTHORED_RESOURCE.description`) and this attribute points back to that
/// same owner. Per the RM transcription rules
/// (`.claude/rules/rm-transcription.md`, "`PATHABLE.parent()` and any other
/// reverse pointer use `Weak<..>` or a path-index — never an owning
/// back-reference") and ADR-001 §8, this is transcribed as
/// `Weak<AuthoredResource>` rather than an owning field, even though
/// `RESOURCE_DESCRIPTION` is not itself a `PATHABLE`/`LOCATABLE` — the same
/// owning-cycle hazard applies to any parent-pointer attribute, not only the
/// RM's own `PATHABLE.parent()`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "RESOURCE_DESCRIPTION")]
pub struct ResourceDescription {
    /// `original_author`: `Hash<String, String>`, cardinality 1..1.
    ///
    /// Original author of this resource, with all relevant details,
    /// including organisation.
    pub original_author: HashMap<String, String>,

    /// `original_namespace`: `String`, cardinality 0..1.
    ///
    /// Namespace of original author's organisation, in reverse internet
    /// form, if applicable.
    pub original_namespace: Option<String>,

    /// `original_publisher`: `String`, cardinality 0..1.
    ///
    /// Plain text name of organisation that originally published this
    /// artefact, if any.
    pub original_publisher: Option<String>,

    /// `other_contributors`: `List<String>`, cardinality 0..1.
    ///
    /// Other contributors to the resource, each listed in `"name <email>"`
    /// form.
    pub other_contributors: Option<Vec<String>>,

    /// `lifecycle_state`: `Terminology_code`, cardinality 1..1.
    ///
    /// Lifecycle state of the resource, typically including states such as:
    /// initial, in_development, in_review, published, superseded, obsolete.
    pub lifecycle_state: TerminologyCode,

    /// `parent_resource`: `AUTHORED_RESOURCE`, cardinality 1..1,
    /// `{default = }`.
    ///
    /// Reference to owning resource.
    ///
    /// PORT NOTE: back-reference to the owning `AuthoredResource`, modelled
    /// as `Weak<..>` rather than an owning field per the RM transcription
    /// rules on reverse pointers — see the type-level doc above.
    ///
    /// TODO(port): the spec's `{default = }` annotation appears to mark
    /// this attribute as having a default value expression, but the table
    /// leaves the default's right-hand side empty/unspecified in the cached
    /// text. Not otherwise actionable until construction/assembly code
    /// exists to interpret it.
    #[serde(skip)]
    pub parent_resource: Weak<AuthoredResource>,

    /// `custodian_namespace`: `String`, cardinality 0..1.
    ///
    /// Namespace in reverse internet id form, of current custodian
    /// organisation.
    pub custodian_namespace: Option<String>,

    /// `custodian_organisation`: `String`, cardinality 0..1.
    ///
    /// Plain text name of current custodian organisation.
    pub custodian_organisation: Option<String>,

    /// `copyright`: `String`, cardinality 0..1.
    ///
    /// Optional copyright statement for the resource as a knowledge
    /// resource.
    pub copyright: Option<String>,

    /// `licence`: `String`, cardinality 0..1.
    ///
    /// Licence of current artefact, in format
    /// `"short licence name <URL of licence>"`, e.g.
    /// `"Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>"`.
    pub licence: Option<String>,

    /// `ip_acknowledgements`: `Hash<String, String>`, cardinality 0..1.
    ///
    /// List of acknowledgements of other IP directly referenced in this
    /// archetype, typically terminology codes, ontology ids etc. Recommended
    /// keys are the widely known name or namespace for the IP source.
    pub ip_acknowledgements: Option<HashMap<String, String>>,

    /// `references`: `Hash<String, String>`, cardinality 0..1.
    ///
    /// List of references of material on which this artefact is based, as a
    /// keyed list of strings. The keys should be in a standard citation
    /// format.
    pub references: Option<HashMap<String, String>>,

    /// `resource_package_uri`: `String`, cardinality 0..1.
    ///
    /// URI of package to which this resource belongs.
    pub resource_package_uri: Option<String>,

    /// `conversion_details`: `Hash<String, String>`, cardinality 0..1.
    ///
    /// Details related to conversion process that generated this model
    /// from an original, if relevant, as a list of name/value pairs.
    pub conversion_details: Option<HashMap<String, String>>,

    /// `other_details`: `Hash<String, String>`, cardinality 0..1.
    ///
    /// Additional non-language-sensitive resource meta-data, as a list of
    /// name/value pairs.
    pub other_details: Option<HashMap<String, String>>,

    /// `details`: `Hash<String, RESOURCE_DESCRIPTION_ITEM>`, cardinality
    /// 0..1.
    ///
    /// Details of all parts of resource description that are natural
    /// language-dependent, keyed by language code.
    pub details: Option<HashMap<String, ResourceDescriptionItem>>,
}

// PORT NOTE: `#[derive(PartialEq)]` is intentionally omitted. `Weak<T>`
// implements `PartialEq` (by pointer identity of the underlying allocation),
// so a derive would compile, but pointer-identity equality on a
// backreference field is not a meaningful notion of value-equality for this
// class and would be surprising to a caller comparing two
// `ResourceDescription`s for content equality. Add a manual `PartialEq` that
// excludes `parent_resource` if value comparison is needed later.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 resource §RESOURCE_DESCRIPTION — docs/research/spec-cache/BASE-1.2.0/uml_classes/resource_description.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master02-resource_package.adoc §Class Descriptions / resource_description.adoc §RESOURCE_DESCRIPTION Class
//   confidence: medium
//   todos: 2
//   note: parent_resource modelled as Weak<AuthoredResource> per the reverse-pointer rule; the spec's bare `{default = }` annotation on that attribute is not otherwise actionable yet. No invariants published for this class.
// ─────────────────────────────────────────────
