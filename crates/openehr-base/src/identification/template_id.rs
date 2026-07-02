//! `TEMPLATE_ID` — identifier for templates.
//!
//! openEHR class: `TEMPLATE_ID`, package `base.base_types.identification`.
//! Inherits: `OBJECT_ID`.
//!
//! Identifier for templates. Lexical form to be determined (the spec
//! explicitly leaves this open at BASE 1.2.0).
use super::object_id::{ObjectId, ObjectIdApi, ObjectIdData};

/// Canonical `_type` discriminator string for this class in serialized
/// form. See the `TODO(port)` on `hier_object_id::TYPE_NAME` for why this
/// is a `const` rather than a `#[serde(rename = ...)]` in this pass.
pub const TYPE_NAME: &str = "TEMPLATE_ID";

/// `TEMPLATE_ID` declares no attribute or function of its own beyond those
/// inherited from `OBJECT_ID`, so it embeds `ObjectIdData` verbatim
/// (ADR-001 §3).
///
/// PORT NOTE: the spec's own description says "Lexical form to be
/// determined" — unlike every other `OBJECT_ID` descendant in this
/// package, `TEMPLATE_ID` has no EBNF production in the Syntaxes section
/// and no parsing functions of its own. Transcribed as a bare wrapper over
/// `value: String` with no derived accessors beyond the raw value, since
/// the spec genuinely does not define more.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplateId {
    /// Embedded `OBJECT_ID` state (the single `value` attribute); lexical
    /// form left open by the specification.
    pub object_id: ObjectIdData,
}

impl TemplateId {
    /// `value`: the value of the id; lexical form left open by the
    /// specification.
    pub fn value(&self) -> &str {
        &self.object_id.value
    }
}

impl ObjectIdApi for TemplateId {
    fn value(&self) -> &str {
        &self.object_id.value
    }
}

impl From<TemplateId> for ObjectId {
    fn from(value: TemplateId) -> Self {
        ObjectId::TemplateId(value)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §TEMPLATE_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/template_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / template_id.adoc §TEMPLATE_ID Class
//   confidence: high
//   todos: 0
//   note: spec itself states lexical form is undetermined at this release; no parsing functions to transcribe beyond the bare value attribute.
// ─────────────────────────────────────────────
