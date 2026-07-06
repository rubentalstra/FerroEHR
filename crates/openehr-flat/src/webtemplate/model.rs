//! The Better `web-template` JSON model (format version `"2.3"`).
//!
//! Field names and JSON shape match Better's `builder/model/*.kt` exactly — this
//! is an interop contract, so `#[serde(rename = ...)]` mirrors Better's
//! `@JsonProperty`/`@JsonPropertyOrder` and the `skip_serializing_if` guards
//! mirror Better's `@JsonInclude(NON_NULL|NON_EMPTY)`.
//!
//! A [`WebTemplateNode`] doubles as the mutable tree the builder shapes: fields
//! marked `#[serde(skip)]` are build-time scratch (the full dedup id chain, the
//! polymorphic alternate id, the cardinality RM path) and never serialized.

use indexmap::IndexMap;
use serde::Serialize;

/// A single template rendered in the Better web format.
///
/// `@JsonInclude(NON_EMPTY)`, order `templateId, semVer, version, defaultLanguage,
/// languages, tree, otherDetails`. `version` is the **format** version (`"2.3"`),
/// not the template version.
#[derive(Debug, Clone, Serialize)]
pub struct WebTemplate {
    #[serde(rename = "templateId")]
    pub template_id: String,
    #[serde(rename = "semVer", skip_serializing_if = "Option::is_none")]
    pub sem_ver: Option<String>,
    pub version: String,
    #[serde(rename = "defaultLanguage")]
    pub default_language: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    pub tree: WebTemplateNode,
    #[serde(rename = "otherDetails", skip_serializing_if = "IndexMap::is_empty")]
    pub other_details: IndexMap<String, String>,
}

/// One node of the web-template tree.
///
/// `@JsonInclude(NON_NULL)`; collection members are `NON_EMPTY`. `id` is the
/// **local** json-id segment (not the full path); `aqlPath` is the archetype
/// path root→node. `max == -1` means unbounded.
#[derive(Debug, Clone, Serialize)]
pub struct WebTemplateNode {
    /// The local json-id segment (Better `jsonId`, serialized as `id`).
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "localizedName", skip_serializing_if = "Option::is_none")]
    pub localized_name: Option<String>,
    #[serde(rename = "rmType")]
    pub rm_type: String,
    #[serde(rename = "nodeId", skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// Occurrences lower bound (Better `occurences.min`, `@JsonUnwrapped`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    /// Occurrences upper bound; `-1` = unbounded (Better `getJsonMax()`).
    pub max: i32,
    #[serde(rename = "inContext", skip_serializing_if = "Option::is_none")]
    pub in_context: Option<bool>,
    #[serde(rename = "aqlPath")]
    pub aql_path: String,
    #[serde(rename = "localizedNames", skip_serializing_if = "IndexMap::is_empty")]
    pub localized_names: IndexMap<String, String>,
    #[serde(
        rename = "localizedDescriptions",
        skip_serializing_if = "IndexMap::is_empty"
    )]
    pub localized_descriptions: IndexMap<String, String>,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub annotations: IndexMap<String, String>,
    #[serde(rename = "termBindings", skip_serializing_if = "IndexMap::is_empty")]
    pub term_bindings: IndexMap<String, WebTemplateBindingCodedValue>,
    #[serde(rename = "proportionTypes", skip_serializing_if = "Vec::is_empty")]
    pub proportion_types: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<WebTemplateInput>,
    #[serde(rename = "dependsOn", skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cardinalities: Vec<WebTemplateCardinality>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<WebTemplateNode>,

    /// AOM 1.4 `C_ATTRIBUTE.existence` constraints on this node's mandatory RM
    /// attributes — captured for the validation walk, **not** part of the Better
    /// web-template JSON (F-07-04), so `#[serde(skip)]`.
    #[serde(skip)]
    pub existence: Vec<WebTemplateExistence>,

    // ── build-time scratch (never serialized) ────────────────────────────────
    /// Full `parent/segment` id chain, used to scope dedup and cardinality ids.
    #[serde(skip)]
    pub full_id: String,
    /// Polymorphic alternate json-id (`value`/`value2`) — Better `alternativeJsonId`.
    #[serde(skip)]
    pub alt_json_id: Option<String>,
    /// The RM name-constraint code, when the node is name-constrained.
    #[serde(skip)]
    pub name_code: Option<String>,
}

impl WebTemplateNode {
    /// A fresh node with the given rm type / aql path; all other fields empty.
    pub(crate) fn new(rm_type: String, aql_path: String) -> Self {
        Self {
            id: String::new(),
            name: None,
            localized_name: None,
            rm_type,
            node_id: None,
            min: None,
            max: -1,
            in_context: None,
            aql_path,
            localized_names: IndexMap::new(),
            localized_descriptions: IndexMap::new(),
            annotations: IndexMap::new(),
            term_bindings: IndexMap::new(),
            proportion_types: Vec::new(),
            inputs: Vec::new(),
            depends_on: None,
            cardinalities: Vec::new(),
            children: Vec::new(),
            existence: Vec::new(),
            full_id: String::new(),
            alt_json_id: None,
            name_code: None,
        }
    }

    /// Whether the node carries an input (a leaf value node).
    pub(crate) fn has_input(&self) -> bool {
        !self.inputs.is_empty()
    }

    /// The first input's suffix, if any (Better `getInput().suffix`).
    pub(crate) fn first_input_type(&self) -> Option<WebTemplateInputType> {
        self.inputs.first().map(|i| i.input_type)
    }
}

/// The web-template `type` enum. Serializes as the SCREAMING constant name
/// (`"TEXT"`, `"CODED_TEXT"`, `"DATETIME"`, …), matching Jackson's default enum
/// rendering in `WebTemplateInputType.kt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebTemplateInputType {
    Text,
    CodedText,
    Date,
    Time,
    Datetime,
    Boolean,
    Integer,
    Decimal,
    Duration,
    Quantity,
    Count,
    Proportion,
}

/// A leaf input descriptor. `@JsonInclude(NON_EMPTY)`, order `suffix, type`.
#[derive(Debug, Clone, Serialize)]
pub struct WebTemplateInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(rename = "type")]
    pub input_type: WebTemplateInputType,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub list: Vec<WebTemplateCodedValue>,
    #[serde(rename = "listOpen", skip_serializing_if = "Option::is_none")]
    pub list_open: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<WebTemplateValidation>,
    #[serde(rename = "defaultValue", skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminology: Option<String>,
}

impl WebTemplateInput {
    pub(crate) fn new(input_type: WebTemplateInputType, suffix: Option<&str>) -> Self {
        Self {
            suffix: suffix.map(str::to_owned),
            input_type,
            list: Vec::new(),
            list_open: None,
            validation: None,
            default_value: None,
            terminology: None,
        }
    }
}

/// A coded option. `@JsonInclude(NON_NULL)`, order
/// `value, label, localizedLabels, localizedDescriptions, termBindings, validation,
/// ordinal|scale`. Better splits these into `WebTemplateCodedValue` /
/// `WebTemplateOrdinalCodedValue` / `WebTemplateScaleCodedValue`; we fold
/// `ordinal`/`scale` as optional fields (identical JSON, omitted when absent).
#[derive(Debug, Clone, Serialize)]
pub struct WebTemplateCodedValue {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "localizedLabels", skip_serializing_if = "IndexMap::is_empty")]
    pub localized_labels: IndexMap<String, String>,
    #[serde(
        rename = "localizedDescriptions",
        skip_serializing_if = "IndexMap::is_empty"
    )]
    pub localized_descriptions: IndexMap<String, String>,
    #[serde(rename = "termBindings", skip_serializing_if = "IndexMap::is_empty")]
    pub term_bindings: IndexMap<String, WebTemplateBindingCodedValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<WebTemplateValidation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
}

impl WebTemplateCodedValue {
    pub(crate) fn new(value: impl Into<String>, label: Option<String>) -> Self {
        Self {
            value: value.into(),
            label,
            localized_labels: IndexMap::new(),
            localized_descriptions: IndexMap::new(),
            term_bindings: IndexMap::new(),
            validation: None,
            ordinal: None,
            scale: None,
        }
    }
}

/// A terminology binding (`{value, terminologyId}`; Better forces `label` null).
#[derive(Debug, Clone, Serialize)]
pub struct WebTemplateBindingCodedValue {
    pub value: String,
    #[serde(rename = "terminologyId")]
    pub terminology_id: String,
}

/// `@JsonInclude(NON_NULL)`: `pattern`, `range`, `precision`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct WebTemplateValidation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<WebTemplateRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<WebTemplateRange>,
}

impl WebTemplateValidation {
    pub(crate) fn is_empty(&self) -> bool {
        self.pattern.is_none() && self.range.is_none() && self.precision.is_none()
    }
}

/// A validation range: `minOp, min, maxOp, max`. Covers Better's
/// `WebTemplateValidationIntegerRange` / `WebTemplateDecimalRange` /
/// `WebTemplateTemporalRange` — `min`/`max` are numbers or ISO strings, so they
/// are held as [`serde_json::Value`].
#[derive(Debug, Clone, Default, Serialize)]
pub struct WebTemplateRange {
    #[serde(rename = "minOp", skip_serializing_if = "Option::is_none")]
    pub min_op: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<serde_json::Value>,
    #[serde(rename = "maxOp", skip_serializing_if = "Option::is_none")]
    pub max_op: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<serde_json::Value>,
}

/// An AOM 1.4 `C_ATTRIBUTE.existence` constraint on a single RM attribute
/// (whether the attribute *field* is present at all — distinct from `cardinality`
/// = container membership and `occurrences` = per-object-block count; AOM 1.4
/// `master04-constraint_model_package.adoc` §existence). Captured for the
/// validation walk only; not part of the Better web-template JSON.
///
/// `path` is the absolute archetype path of the constrained attribute
/// (`{node aqlPath}/{rm_attribute_name}`); `min`/`max` are the existence bounds
/// (`max == -1` unbounded). A mandatory attribute has `min >= 1`.
#[derive(Debug, Clone)]
pub struct WebTemplateExistence {
    pub min: i32,
    pub max: i32,
    pub path: String,
}

/// A container cardinality: `{min, max, ids}`. `path` is build-time scratch used
/// to resolve `ids` from the child json-ids (never serialized).
#[derive(Debug, Clone, Serialize)]
pub struct WebTemplateCardinality {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    pub max: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
    #[serde(skip)]
    pub path: String,
}
