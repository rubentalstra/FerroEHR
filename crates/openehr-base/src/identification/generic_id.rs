//! `GENERIC_ID` — generic identifier type for otherwise-unknown identifier
//! formats.
//!
//! openEHR class: `GENERIC_ID`, package `base.base_types.identification`.
//! Inherits: `OBJECT_ID`.
//!
//! Generic identifier type for identifiers whose format is otherwise
//! unknown to openEHR. Includes an attribute for naming the identification
//! scheme (which may well be local).
use super::object_id::{ObjectId, ObjectIdApi, ObjectIdData};

/// Canonical `_type` discriminator string for this class in serialized
/// form.
///
/// TODO(port): see the `TODO(port)` on `archetype_id::TYPE_NAME` — the same
/// "no wire path currently emits `_type`" gap applies here (`GenericId` is
/// likewise reached only through the untagged `ObjectId::GenericId`
/// variant).
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
#[serde(rename = "GENERIC_ID")]
pub struct GenericId {
    /// Embedded `OBJECT_ID` state (the single `value` attribute).
    #[serde(flatten)]
    pub object_id: ObjectIdData,

    /// `scheme`: name of the scheme to which this identifier conforms.
    /// Ideally this name will be recognisable globally but realistically it
    /// may be a local ad hoc scheme whose name is not controlled or
    /// standardised in any way.
    pub scheme: String,
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
//   todos: 1
//   note: only OBJECT_ID descendant in this package to add a new attribute (scheme) rather than layering functions over the inherited value alone; no invariant table given in the spec for this class. P4 addendum: no wire path currently emits this class's _type (see archetype_id.rs); flagged for openehr-serde's P17 manual dispatch.
// ─────────────────────────────────────────────
