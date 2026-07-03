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
use openehr_foundation::primitive_types::string::OpenEhrString;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_foundation::terminology_types::terminology_code::TerminologyCode;
use serde::de::Error as _;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
#[derive(Debug, Clone)]
pub struct ResourceDescription {
    /// Canonical `_type` discriminator (`"RESOURCE_DESCRIPTION"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    pub type_tag: TypeTag<Self>,

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
    /// PORT NOTE (published-spec defect, no action possible): the class
    /// table renders this attribute's signature with a trailing
    /// `{default = }` whose right-hand side is empty — verbatim
    /// `{default{nbsp}={nbsp}}` in the published AsciiDoc
    /// (docs/research/spec-cache/BASE-1.2.0/uml_classes/resource_description.adoc)
    /// — i.e. the UML extraction emitted the default-value slot without any
    /// default expression. There is therefore no default value to encode; a
    /// freshly built `ResourceDescription` starts with an unattached
    /// back-reference (`Weak::new()`), which is also the only meaningful
    /// "default" for a non-owning parent pointer. Revisit only if a later
    /// BASE release publishes an actual expression.
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
// P4: that manual impl is now needed (the canonical-JSON round-trip harness
// compares fixture instances for content equality), so it is provided below,
// excluding `parent_resource` exactly as anticipated.

impl PartialEq for ResourceDescription {
    fn eq(&self, other: &Self) -> bool {
        self.original_author == other.original_author
            && self.original_namespace == other.original_namespace
            && self.original_publisher == other.original_publisher
            && self.other_contributors == other.other_contributors
            && self.lifecycle_state == other.lifecycle_state
            && self.custodian_namespace == other.custodian_namespace
            && self.custodian_organisation == other.custodian_organisation
            && self.copyright == other.copyright
            && self.licence == other.licence
            && self.ip_acknowledgements == other.ip_acknowledgements
            && self.references == other.references
            && self.resource_package_uri == other.resource_package_uri
            && self.conversion_details == other.conversion_details
            && self.other_details == other.other_details
            && self.details == other.details
    }
}

impl TypeName for ResourceDescription {
    const NAME: &'static str = "RESOURCE_DESCRIPTION";
}

impl Serialize for ResourceDescription {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 5;
        field_count += usize::from(self.other_contributors.is_some());
        field_count += usize::from(self.resource_package_uri.is_some());
        field_count += usize::from(self.other_details.is_some());

        let mut state = serializer.serialize_struct("RESOURCE_DESCRIPTION", field_count)?;
        state.serialize_field("_type", "RESOURCE_DESCRIPTION")?;
        state.serialize_field("original_author", &self.original_author)?;
        if let Some(other_contributors) = &self.other_contributors {
            state.serialize_field("other_contributors", other_contributors)?;
        }
        state.serialize_field("lifecycle_state", &self.lifecycle_state.code_string)?;
        if let Some(resource_package_uri) = &self.resource_package_uri {
            state.serialize_field("resource_package_uri", resource_package_uri)?;
        }
        if let Some(other_details) = &self.other_details {
            state.serialize_field("other_details", other_details)?;
        }
        state.serialize_field("parent_resource", &HashMap::<String, String>::new())?;

        let mut details: Vec<(&String, &ResourceDescriptionItem)> = self
            .details
            .as_ref()
            .map(|items| items.iter().collect())
            .unwrap_or_default();
        details.sort_by_key(|(left, _)| *left);
        let detail_values: Vec<&ResourceDescriptionItem> =
            details.into_iter().map(|(_, item)| item).collect();
        state.serialize_field("details", &detail_values)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ResourceDescription {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(rename = "_type")]
            type_name: Option<String>,
            original_author: HashMap<String, String>,
            #[serde(default)]
            other_contributors: Option<Vec<String>>,
            lifecycle_state: String,
            #[serde(default)]
            resource_package_uri: Option<String>,
            #[serde(default)]
            other_details: Option<HashMap<String, String>>,
            #[serde(default)]
            parent_resource: HashMap<String, String>,
            #[serde(default)]
            details: Vec<ResourceDescriptionItem>,
        }

        let wire = Wire::deserialize(deserializer)?;
        if wire
            .type_name
            .as_deref()
            .is_some_and(|name| name != "RESOURCE_DESCRIPTION")
        {
            return Err(D::Error::custom("expected _type \"RESOURCE_DESCRIPTION\""));
        }

        let details = if wire.details.is_empty() {
            None
        } else {
            let mut items = HashMap::new();
            for item in wire.details {
                items.insert(item.language.code_string.0.clone(), item);
            }
            Some(items)
        };

        let _parent_resource = wire.parent_resource;
        Ok(ResourceDescription {
            type_tag: TypeTag::new(),
            original_author: wire.original_author,
            original_namespace: None,
            original_publisher: None,
            other_contributors: wire.other_contributors,
            lifecycle_state: TerminologyCode {
                terminology_id: OpenEhrString("openehr".to_string()),
                terminology_version: None,
                code_string: OpenEhrString(wire.lifecycle_state),
                uri: None,
            },
            parent_resource: Weak::new(),
            custodian_namespace: None,
            custodian_organisation: None,
            copyright: None,
            licence: None,
            ip_acknowledgements: None,
            references: None,
            resource_package_uri: wire.resource_package_uri,
            conversion_details: None,
            other_details: wire.other_details,
            details,
        })
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 resource §RESOURCE_DESCRIPTION — docs/research/spec-cache/BASE-1.2.0/uml_classes/resource_description.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master02-resource_package.adoc §Class Descriptions / resource_description.adoc §RESOURCE_DESCRIPTION Class
//   confidence: medium
//   todos: 0
//   note: parent_resource modelled as Weak<AuthoredResource> per the reverse-pointer rule; the spec's bare `{default = }` annotation is a published-table defect (empty default expression) recorded as a PORT NOTE — Weak::new() is the de facto default. No invariants published for this class. P4: custom serde maps the in-memory BASE 1.2.0 shape to the pinned ITS-JSON object shape (`lifecycle_state` string, `details` array, placeholder `parent_resource` object).
// ─────────────────────────────────────────────
