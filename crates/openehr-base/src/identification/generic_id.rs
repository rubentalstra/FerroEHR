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
/// form. See the `TODO(port)` on `hier_object_id::TYPE_NAME` for why this
/// is a `const` rather than a `#[serde(rename = ...)]` in this pass.
pub const TYPE_NAME: &str = "GENERIC_ID";

/// `GENERIC_ID` embeds `ObjectIdData` (its inherited `value: String`
/// attribute, ADR-001 §3) and adds one attribute of its own: `scheme`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenericId {
    /// Embedded `OBJECT_ID` state (the single `value` attribute).
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
//   todos: 0
//   note: only OBJECT_ID descendant in this package to add a new attribute (scheme) rather than layering functions over the inherited value alone; no invariant table given in the spec for this class.
// ─────────────────────────────────────────────
