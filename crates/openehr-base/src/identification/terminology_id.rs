//! `TERMINOLOGY_ID` — identifier for terminologies.
//!
//! openEHR class: `TERMINOLOGY_ID`, package
//! `base.base_types.identification`.
//! Inherits: `OBJECT_ID`.
//!
//! Identifier for terminologies such as accessed via a terminology query
//! service. The value attribute identifies the Terminology in the
//! terminology service, e.g. `SNOMED-CT`. A terminology is assumed to be in
//! a particular language, which must be explicitly specified.
//!
//! Lexical form: `name [ '(' version ')' ]`.
use super::object_id::{ObjectId, ObjectIdApi, ObjectIdData};

/// Canonical `_type` discriminator string for this class in serialized
/// form. See the `TODO(port)` on `hier_object_id::TYPE_NAME` for why this
/// is a `const` rather than a `#[serde(rename = ...)]` in this pass.
pub const TYPE_NAME: &str = "TERMINOLOGY_ID";

/// `TERMINOLOGY_ID` declares no attribute of its own beyond the inherited
/// `value: String` from `OBJECT_ID`; its two functions (`name`,
/// `version_id`) parse substrings of that one attribute, so it embeds
/// `ObjectIdData` verbatim (ADR-001 §3).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminologyId {
    /// Embedded `OBJECT_ID` state (the single `value` attribute), in the
    /// lexical form `name [ '(' version ')' ]`.
    pub object_id: ObjectIdData,
}

impl TerminologyId {
    /// `value`: the raw `name [ '(' version ')' ]` string.
    pub fn value(&self) -> &str {
        &self.object_id.value
    }

    /// `name(): String`.
    ///
    /// Return the terminology id (which includes the version in some
    /// cases). Distinct names correspond to distinct (i.e. non-compatible)
    /// terminologies. Thus the names `ICD10AM` and `ICD10` refer to
    /// distinct terminologies. The part of `value` before the first `(`, if
    /// any, or else the whole string.
    pub fn name(&self) -> String {
        self.value().split_once('(').map_or_else(
            || self.value().to_string(),
            |(name, _rest)| name.to_string(),
        )
    }

    /// `version_id(): String`.
    ///
    /// Version of this terminology, if versioning supported, else the
    /// empty string. The part of `value` inside the parentheses, if
    /// present.
    pub fn version_id(&self) -> String {
        match self.value().split_once('(') {
            Some((_name, rest)) => rest.strip_suffix(')').unwrap_or(rest).to_string(),
            None => String::new(),
        }
    }
}

impl ObjectIdApi for TerminologyId {
    fn value(&self) -> &str {
        &self.object_id.value
    }
}

impl From<TerminologyId> for ObjectId {
    fn from(value: TerminologyId) -> Self {
        ObjectId::TerminologyId(value)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §TERMINOLOGY_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/terminology_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / terminology_id.adoc §TERMINOLOGY_ID Class
//   confidence: high
//   todos: 0
//   note: name()/version_id() implemented as string-splitting against the terminology_id EBNF grammar in the Syntaxes section; no invariant table given in the spec beyond the lexical grammar itself.
// ─────────────────────────────────────────────
