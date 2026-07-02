//! `GENERIC_ID` — generic identifier type for otherwise-unknown identifier
//! formats.
//!
//! openEHR class: `GENERIC_ID`, package `base.base_types.identification`.
//! Inherits: `OBJECT_ID`.
//!
//! Generic identifier type for identifiers whose format is otherwise
//! unknown to openEHR. Includes an attribute for naming the identification
//! scheme (which may well be local).
use openehr_foundation::serde_support::{TypeName, TypeTag};

use super::object_id::{ObjectId, ObjectIdApi, ObjectIdData};

/// Canonical `_type` discriminator string for this class in serialized
/// form. P4/ADR-002 update: single-sources the string carried by the
/// struct's own self-tagging `type_tag` field below (via the [`TypeName`]
/// impl) — see `archetype_id::TYPE_NAME`.
pub const TYPE_NAME: &str = "GENERIC_ID";

/// `GENERIC_ID` embeds `ObjectIdData` (its inherited `value: String`
/// attribute, ADR-001 §3) and adds one attribute of its own: `scheme`.
///
/// `#[serde(flatten)]` on the embedded `object_id` field folds
/// `ObjectIdData`'s single `value` attribute directly into this struct's
/// JSON object, alongside the sibling `scheme` field.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct GenericId {
    /// Canonical `_type` discriminator (`"GENERIC_ID"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `OBJECT_ID` state (the single `value` attribute).
    #[serde(flatten)]
    pub object_id: ObjectIdData,

    /// `scheme`: name of the scheme to which this identifier conforms.
    /// Ideally this name will be recognisable globally but realistically it
    /// may be a local ad hoc scheme whose name is not controlled or
    /// standardised in any way.
    pub scheme: String,
}

impl TypeName for GenericId {
    const NAME: &'static str = TYPE_NAME;
}

impl GenericId {
    /// `value`: the value of the id.
    pub fn value(&self) -> &str {
        &self.object_id.value
    }
}

impl ObjectIdApi for GenericId {
    fn value(&self) -> &str {
        &self.object_id.value
    }
}

impl From<GenericId> for ObjectId {
    fn from(value: GenericId) -> Self {
        ObjectId::GenericId(value)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §GENERIC_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/generic_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / generic_id.adoc §GENERIC_ID Class
//   confidence: high
//   todos: 0
//   note: only OBJECT_ID descendant in this package to add a new attribute (scheme) rather than layering functions over the inherited value alone; no invariant table given in the spec for this class. P4/ADR-002: self-tags via TypeTag<Self> first field (NAME single-sourced from TYPE_NAME); inert struct-level #[serde(rename)] deleted; the earlier "no wire path emits _type" TODO is resolved by the self-tag.
// ─────────────────────────────────────────────
