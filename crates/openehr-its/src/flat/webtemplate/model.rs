//! The Web Template JSON model.
//!
//! The wire shape is the Web Template metadata document of `ITS-REST
//! simplified_formats master04-basic_concepts.adoc` §"Web Template Metadata"
//! (`templateId`/`semVer`/`version`/`defaultLanguage`/`languages`/`tree`, each
//! node carrying `id`/`name`/`localizedName`/`rmType`/`nodeId`/`min`/`max`/
//! `aqlPath`/`inputs`/`children`/`inContext`/`termBindings`/`annotations`/…).
//! It is served as `application/openehr.wt+json`, so the `#[serde(rename = ...)]`
//! names, field order, and `skip_serializing_if` guards are a fixed wire contract
//! and must not change.
//!
//! A [`WebTemplateNode`] doubles as the mutable tree the builder shapes: fields
//! marked `#[serde(skip)]` are build-time scratch (the full dedup id chain, the
//! polymorphic alternate id, the cardinality RM path) and captured constraints
//! for validation — never serialized.
//!
//! Relationship to the vendored ITS-REST schema: the normative
//! `schemas/web_template/{WebTemplate,Tree,Child,Input,…}.yaml` describe a
//! *subset* of the metadata document. The model carries additive fields the
//! master04 example and the ITS-REST schema do not list (`cardinalities`,
//! `proportionTypes`, `dependsOn` resolution on nodes; `listOpen`, `terminology`
//! on inputs; `ordinal`/`scale`/`termBindings` on coded values; `otherDetails`
//! on the root) — no openEHR spec governs these fields; they are our own
//! design/extension, consumed by validation and by interop consumers, and
//! schema-legal because those schemas set no `additionalProperties: false`. The
//! one place the schema is *stricter* than a naive render is its `Tree.required`
//! list — satisfied by [`serialize_root`].

use indexmap::IndexMap;
use serde::{Serialize, Serializer};

/// A single Web Template metadata document (`ITS-REST simplified_formats
/// master04 §"Web Template Metadata"`).
///
/// Member order `templateId, semVer, version, defaultLanguage, languages, tree,
/// otherDetails`; empty members are omitted. `version` is the **format** version
/// string, not the template version (`otherDetails` is an additive field — no
/// openEHR spec governs it; our own design/extension).
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
    #[serde(serialize_with = "serialize_root")]
    pub tree: WebTemplateNode,
    #[serde(rename = "otherDetails", skip_serializing_if = "IndexMap::is_empty")]
    pub other_details: IndexMap<String, String>,
}

/// Serialize the **root** node so it satisfies the normative ITS-REST
/// `Tree` schema (`schemas/web_template/Tree.yaml`), whose `required` set —
/// `id, name, localizedName, rmType, nodeId, min, max, localizedNames,
/// localizedDescriptions, aqlPath, children` — is stricter than the `Child`
/// schema a nested [`WebTemplateNode`] serializes against.
///
/// A well-formed OPT root already carries every required member, so this
/// reproduces the normal node serialization byte-for-byte; it only *fills in*
/// members a sparse root would otherwise omit (empty rubric maps, an unbounded
/// `min`, an empty `nodeId`, no children), so a strict JSON-Schema validator
/// accepts the `WebTemplate`. Children keep the looser `Child`-schema shape
/// (`schemas/web_template/Child.yaml`). Missing scalars default to spec-valid
/// placeholders
/// (`name`/`localizedName` ← the node `id`, `nodeId` ← `""`, `min` ← `0`).
fn serialize_root<S: Serializer>(node: &WebTemplateNode, s: S) -> Result<S::Ok, S::Error> {
    // Serialize via the derived (Child-shaped) impl, then ensure the
    // Tree-required members are present. Appended keys are schema-legal (JSON
    // object member order is not significant); nothing already emitted is
    // touched, so the well-formed case is unchanged.
    use serde_json::json;
    let mut value = serde_json::to_value(node).map_err(serde::ser::Error::custom)?;
    if let serde_json::Value::Object(map) = &mut value {
        map.entry("name").or_insert_with(|| json!(node.id));
        map.entry("localizedName")
            .or_insert_with(|| json!(node.name.clone().unwrap_or_else(|| node.id.clone())));
        map.entry("nodeId").or_insert_with(|| json!(""));
        map.entry("min").or_insert_with(|| json!(0));
        map.entry("localizedNames").or_insert_with(|| json!({}));
        map.entry("localizedDescriptions")
            .or_insert_with(|| json!({}));
        map.entry("children").or_insert_with(|| json!([]));
    }
    value.serialize(s)
}

/// The `defining_code` a node whose runtime `name` is constrained as a
/// `DV_CODED_TEXT` must carry — the constrained terminology + code the
/// composition builder stamps so the instance name is a coded name, not a
/// plain `DV_TEXT` (RM common `master03-archetyped_package.adoc` §"The
/// `LOCATABLE` class" — a `LOCATABLE.name` is `DV_TEXT` *or* `DV_CODED_TEXT`;
/// AOM 1.4 `master04-constraint_model_package.adoc` — a `C_ATTRIBUTE` on `name`
/// constrains the whole coded name). The display value is the node's
/// [`WebTemplateNode::name`].
#[derive(Debug, Clone)]
pub struct CodedName {
    /// The `defining_code` terminology id (e.g. `local`, `openehr`).
    pub terminology: String,
    /// The `defining_code` code string (an `atNNNN` / openEHR code).
    pub code: String,
    /// Whether the constraint is display/rubric-**incoherent**: the template
    /// fixes a `name/value` that equals NO listed code's archetype rubric, so
    /// any conforming instance pairs a display value with a code whose rubric
    /// says something else. Spec-legal on our reading (a name constraint may
    /// rename; RM common master03 §LOCATABLE), but the reference
    /// implementation enforces value ≡ local-code rubric and rejects every
    /// form of such a node (verified empirically) — the example generator
    /// omits OPTIONAL incoherent nodes so a shared payload exists.
    pub incoherent: bool,
}

/// One node of the Web Template tree (`ITS-REST simplified_formats master04
/// §"Web Template Metadata"`, the `tree`/`children` shape).
///
/// Null members are omitted, as are empty collection members. `id` is the
/// **local** json-id segment (not the full path; master04 §"Node ID Generation
/// Rules"); `aqlPath` is the archetype path root→node. `max == -1` means
/// unbounded.
#[derive(Debug, Clone, Serialize)]
pub struct WebTemplateNode {
    /// The local json-id segment, serialized as `id` (master04 §"Node ID
    /// Generation Rules").
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "localizedName", skip_serializing_if = "Option::is_none")]
    pub localized_name: Option<String>,
    #[serde(rename = "rmType")]
    pub rm_type: String,
    #[serde(rename = "nodeId", skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// Occurrences lower bound (master04 §"Web Template Metadata": `min`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    /// Occurrences upper bound; `-1` = unbounded (master04 §"Web Template
    /// Metadata": `max`, where `max = -1` marks an unbounded occurrence).
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
    /// attributes — captured for the validation walk, **not** part of the Web
    /// Template metadata document, so `#[serde(skip)]`.
    #[serde(skip)]
    pub existence: Vec<WebTemplateExistence>,

    /// **Every** constraining `C_MULTIPLE_ATTRIBUTE.cardinality` on this node
    /// (AOM 1.4 `master04-constraint_model_package.adoc` §cardinality) — a
    /// superset of the serialized `cardinalities`, which keeps only the intervals
    /// surfaced in the metadata document. The validation walk enforces this set
    /// so `0..1`/`1..1`/`1..*` container bounds are checked too. `#[serde(skip)]`
    /// — validation-only.
    #[serde(skip)]
    pub card_all: Vec<WebTemplateCardinality>,

    /// RM-type narrowings of wrapper constraints the compactor hoisted away
    /// (`ITEM_*`/`HISTORY`/single `EVENT`): an instance node matched at `path`
    /// must conform to one of the recorded types (AOM 1.4 type conformance —
    /// "Class not allowed"). `#[serde(skip)]` — validation-only.
    #[serde(skip)]
    pub slots: Vec<WebTemplateSlot>,

    /// `C_INTEGER.list`/`C_REAL.list` constraints on a leaf's numeric datum
    /// (`magnitude`, `value`, …), keyed by RM attribute name — the `inputs`
    /// document does not carry these lists, so they are validation-only.
    #[serde(skip)]
    pub numeric_lists: Vec<(String, Vec<f64>)>,

    /// `C_INTEGER.range`/`C_REAL.range` constraints on a leaf's numeric datum
    /// the `inputs` builders do not otherwise carry (e.g. `DV_MULTIMEDIA.size` —
    /// RM `data_types` §`DV_MULTIMEDIA`, `size: Integer`; AOM 1.4
    /// `master04-constraint_model_package.adoc` §`C_INTEGER`), keyed by RM
    /// attribute name. Validation-only; `#[serde(skip)]`.
    #[serde(skip)]
    pub numeric_ranges: Vec<(String, WebTemplateRange)>,

    /// `C_DURATION.range` (ISO-8601 duration bounds) on a `DV_DURATION` leaf's
    /// `value` — the `inputs` document encodes only the allowed-field split into
    /// per-field inputs, so the range is validation-only.
    #[serde(skip)]
    pub duration_range: Option<WebTemplateRange>,

    /// `C_TIME.timezone_validity` / `C_DATE_TIME.timezone_validity`
    /// (`VALIDITY_KIND`; OPT 1.4 XSD encodes `1001` = mandatory, `1002` =
    /// optional, `1003` = disallowed) on a temporal leaf's `value` — governs
    /// whether the instance's timezone designator must be present, may be
    /// present, or must be absent (AOM 1.4
    /// `AM/docs/UML/classes/org.openehr.am.aom14.c_time.adoc`/`…c_date_time.adoc`).
    /// Validation-only; `C_DATE` has no timezone. `#[serde(skip)]`.
    #[serde(skip)]
    pub tz_validity: Option<i32>,

    /// `C_QUANTITY.property` openEHR `property`-group code (e.g. `"122"` =
    /// Length) on a `DV_QUANTITY` leaf whose constraint carries a `property` but
    /// no enumerated `C_QUANTITY_ITEM` unit list. The instance's `units` must
    /// then belong to that property's unit set — resolved against the openEHR
    /// `PropertyUnitData.xml` property↔unit table (`openehr_term::bundle`).
    /// Validation-only; `#[serde(skip)]`.
    #[serde(skip)]
    pub quantity_property: Option<String>,

    /// True when this leaf's coded constraint (a `DV_CODED_TEXT` `defining_code`
    /// or a bare `CODE_PHRASE`) is a `C_CODE_PHRASE` that **explicitly** names
    /// the archetype-`local` terminology with a non-empty closed `code_list`.
    /// The builder strips the implicit/default `local` from
    /// [`WebTemplateInput::terminology`] (so it never reaches the wt+json
    /// document), losing the distinction between "explicitly local" and "no
    /// terminology named"; this flag preserves the explicit-local signal so the
    /// validator can reject a foreign-terminology instance code
    /// (`AM/docs/UML/classes/org.openehr.am.aom14.c_coded_text.adoc`
    /// §C_CODED_TEXT: the `code_list` is scoped to the named terminology).
    /// Validation-only; `#[serde(skip)]`.
    #[serde(skip)]
    pub coded_terminology_local: bool,

    /// `C_CODE_PHRASE` code lists on coded RM attributes the `inputs` mapping
    /// does not model (e.g. `DV_MULTIMEDIA.media_type`). `defining_code` is
    /// excluded (already covered by the coded-text `inputs`).
    #[serde(skip)]
    pub code_lists: Vec<WebTemplateCodeList>,

    /// Closed-archetype constraints: per
    /// constrained attribute that carries archetype-node-identified alternatives
    /// and/or open `ARCHETYPE_SLOT`s, the admissible child identities. The walk
    /// rejects an instance child under such an attribute whose `archetype_node_id`
    /// matches neither a fixed sibling alternative nor an open slot. Captured at
    /// **build time** from the OPT (before compaction, so no alternative is lost)
    /// and re-homed on the parent by absolute path when a wrapper is hoisted.
    /// Validation-only; `#[serde(skip)]`.
    #[serde(skip)]
    pub closed_attributes: Vec<WebTemplateClosedAttribute>,

    /// Structural stubs for the RM-mandatory structural attributes of an
    /// ENTRY-family node (`data`, `description`, `protocol`, `state`) that the
    /// template **does** constrain with a node-identified structural child
    /// (`ITEM_TREE[at0017]`, a `HISTORY`, …) which does not survive as a
    /// web-template tree child because it carries no leaf content and the
    /// compactor drops it. Recorded so the FLAT/TDD composition builder can
    /// synthesise the empty attribute with the *constrained* node id/type/name
    /// rather than a blind `at0001` placeholder — the constrained attribute must
    /// be filled by a conforming value (AOM 1.4
    /// `AM/docs/AOM1.4/master04-constraint_model_package.adoc` §`Valid_value`).
    /// Captured at build time from the OPT constraint object. Validation-only;
    /// `#[serde(skip)]`.
    #[serde(skip)]
    pub structural_stubs: Vec<WebTemplateStructuralStub>,

    // ── build-time scratch (never serialized) ────────────────────────────────
    /// Full `parent/segment` id chain, used to scope dedup and cardinality ids.
    #[serde(skip)]
    pub full_id: String,
    /// Polymorphic alternate json-id (`value`/`value2`) for a choice ELEMENT's
    /// alternative `DV_*` children. No openEHR spec governs choice-ELEMENT ids —
    /// our own design/extension; consumed by the FLAT converters and the
    /// validation walk.
    #[serde(skip)]
    pub alt_json_id: Option<String>,
    /// The RM name-constraint code, when the node is name-constrained.
    #[serde(skip)]
    pub name_code: Option<String>,
    /// When the node's runtime `name` is constrained as a `DV_CODED_TEXT`, the
    /// `(terminology, code)` the composition builder stamps as the coded name's
    /// `defining_code` (the display value is [`Self::name`]). `None` for a plain
    /// `DV_TEXT` name. Build-time only (`#[serde(skip)]`); see [`CodedName`].
    #[serde(skip)]
    pub name_coded: Option<CodedName>,

    /// Pre-parsed archetype-conformance walk plan: the constraint
    /// paths and sibling groups this node's validation walk needs, parsed ONCE at
    /// build time ([`crate::flat::webtemplate::build_web_template`] calls `prepare_walk`) instead of
    /// re-parsing every constraint path on every instance-node visit. A hand-built
    /// node with no plan is handled by the walk building the plan on the fly.
    /// Validation-only (`#[serde(skip)]`) — no openEHR spec governs the
    /// WebTemplate model (our own design/extension).
    #[serde(skip)]
    pub(crate) walk: Option<Box<crate::flat::validation::NodeWalk>>,
}

impl WebTemplateNode {
    /// A fresh node with the given rm type / aql path; all other fields empty.
    #[must_use]
    pub fn new(rm_type: String, aql_path: String) -> Self {
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
            card_all: Vec::new(),
            slots: Vec::new(),
            numeric_lists: Vec::new(),
            numeric_ranges: Vec::new(),
            duration_range: None,
            tz_validity: None,
            quantity_property: None,
            coded_terminology_local: false,
            code_lists: Vec::new(),
            closed_attributes: Vec::new(),
            structural_stubs: Vec::new(),
            full_id: String::new(),
            alt_json_id: None,
            name_code: None,
            name_coded: None,
            walk: None,
        }
    }

    /// Whether the node carries an input (a leaf value node).
    pub(crate) fn has_input(&self) -> bool {
        !self.inputs.is_empty()
    }

    /// The first input's type, if any.
    pub(crate) fn first_input_type(&self) -> Option<WebTemplateInputType> {
        self.inputs.first().map(|i| i.input_type)
    }
}

/// The Web Template input `type` enum. Serializes as the SCREAMING constant name
/// (`"TEXT"`, `"CODED_TEXT"`, `"DATETIME"`, …), the `inputs[].type` values of the
/// master04 §"Web Template Metadata" example.
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

/// A leaf input descriptor (master04 §"Web Template Metadata": `inputs[]`).
/// Member order `suffix, type`; empty members omitted. `listOpen`, `terminology`,
/// and `defaultValue` are additive fields the master04 example does not list —
/// no openEHR spec governs them; our own design/extension (`listOpen` also drives
/// the master04 §"Open Value-Sets and the `|other` Suffix" behaviour).
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
    #[must_use]
    pub fn new(input_type: WebTemplateInputType, suffix: Option<&str>) -> Self {
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

/// A coded option (master04 §"Web Template Metadata": an `inputs[].list[]`
/// entry). Null members omitted, member order `value, label, localizedLabels,
/// localizedDescriptions, termBindings, validation, ordinal|scale`. `ordinal`
/// (for `DV_ORDINAL`) and `scale` (for `DV_SCALE`) are optional fields, omitted
/// when absent — no openEHR spec governs them here; our own design/extension.
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
    pub fn new(value: impl Into<String>, label: Option<String>) -> Self {
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

/// A terminology binding (master04 §"Web Template Metadata": a `termBindings`
/// entry, `{value, terminologyId}`).
#[derive(Debug, Clone, Serialize)]
pub struct WebTemplateBindingCodedValue {
    pub value: String,
    #[serde(rename = "terminologyId")]
    pub terminology_id: String,
}

/// A leaf input's `validation` (master04 §"Web Template Metadata":
/// `inputs[].validation`): `pattern`, `range`, `precision`; null members omitted.
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

/// A validation range (master04 §"Web Template Metadata": the `range`/`precision`
/// object): `minOp, min, maxOp, max`. Serves integer, decimal, and temporal
/// ranges alike — `min`/`max` are numbers or ISO strings, so they are held as
/// [`serde_json::Value`].
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
/// validation walk only; not part of the Web Template metadata document.
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

/// A hoisted-wrapper type narrowing (validation-only, never serialized): the
/// instance node(s) matched at the absolute archetype `path` must conform to
/// `rm_type` (or to one of the types when several same-path alternatives were
/// hoisted — the walk groups by path).
#[derive(Debug, Clone)]
pub struct WebTemplateSlot {
    /// Absolute archetype path of the hoisted wrapper constraint.
    pub path: String,
    /// The wrapper's constrained RM type (`ITEM_LIST`, `POINT_EVENT`, …; an
    /// abstract type such as `ITEM_STRUCTURE` admits every concrete subtype).
    pub rm_type: String,
}

/// A `C_CODE_PHRASE` code-list constraint on a coded RM attribute of a leaf
/// (validation-only, never serialized): `attr` is the RM attribute name (e.g.
/// `media_type`), `terminology` the constrained terminology id (`None` =
/// `local`), `codes` the allowed `code_string`s.
#[derive(Debug, Clone)]
pub struct WebTemplateCodeList {
    pub attr: String,
    pub terminology: Option<String>,
    pub codes: Vec<String>,
}

/// A structural stub for an RM-mandatory structural attribute of an ENTRY-family
/// node (validation/synthesis-only, never serialized). The template constrains
/// the attribute `attr` (e.g. `description`) with a node-identified structural
/// child of RM type `rm_type` (`ITEM_TREE`, `HISTORY`, …) and archetype node id
/// `node_id` (`at0017`), whose rubric `name` comes from the archetype
/// `term_definitions`. When the attribute carries no leaf content, the compacted
/// web-template drops the wrapper, so this record lets the composition builder
/// synthesise the empty attribute with the *constrained* identity — a value the
/// closed-archetype walk admits (AOM 1.4
/// `AM/docs/AOM1.4/master04-constraint_model_package.adoc` §`Valid_value`).
#[derive(Debug, Clone)]
pub struct WebTemplateStructuralStub {
    /// The constrained RM attribute name (`data`, `description`, `protocol`,
    /// `state`).
    pub attr: String,
    /// The constrained structural RM type (`ITEM_TREE`, `HISTORY`, …).
    pub rm_type: String,
    /// The constraint's archetype node id (`atNNNN`, or an archetype id at a root).
    pub node_id: String,
    /// The rubric text for `node_id` from the archetype `term_definitions`, when
    /// present.
    pub name: Option<String>,
}

/// A closed-archetype constraint on one attribute (validation-only, never
/// serialized). Under the constrained attribute at
/// absolute archetype `path`, an instance child bearing an `archetype_node_id`
/// is admissible iff it matches one of `allowed_ids` (a fixed at-code /
/// archetype-id sibling alternative) **or** an open `ARCHETYPE_SLOT` in `slots`.
/// Any other archetyped child is an "unexpected node" (closed-world rejection).
#[derive(Debug, Clone)]
pub struct WebTemplateClosedAttribute {
    /// Absolute archetype path of the constrained attribute (`{node aqlPath}/{attr}`).
    pub path: String,
    /// Fixed sibling identities (constraint `node_id`s at at-code level, or the
    /// `archetype_id` of a `C_ARCHETYPE_ROOT` / `ARCHETYPE_INTERNAL_REF`).
    pub allowed_ids: Vec<String>,
    /// Open `ARCHETYPE_SLOT`s under this attribute.
    pub slots: Vec<WebTemplateArchetypeSlot>,
}

/// An open `ARCHETYPE_SLOT` constraint (AOM 1.4 `ARCHETYPE_SLOT`; validation-only,
/// never serialized). A slot-filling instance object must conform to `rm_type`,
/// match at least one `includes` archetype-id regex, and match no `excludes`
/// regex (a blanket `.*` exclude is ignored when `includes` is non-empty — the
/// ADL 1.4 closed-slot idiom; AOM 1.4 has no `is_closed`). `min`/`max` bound the
/// permitted number of fillers (`max == -1` unbounded).
#[derive(Debug, Clone)]
pub struct WebTemplateArchetypeSlot {
    pub rm_type: String,
    pub min: i32,
    pub max: i32,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
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
