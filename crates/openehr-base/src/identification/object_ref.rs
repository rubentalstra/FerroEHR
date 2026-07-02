//! `OBJECT_REF` — reference to another object.
//!
//! openEHR class: `OBJECT_REF`, package `base.base_types.identification`.
//!
//! Class describing a reference to another object, which may exist locally
//! or be maintained outside the current namespace, e.g. in another
//! service. Services are usually external, e.g. available in a LAN
//! (including on the same host) or the internet via Corba, SOAP, or some
//! other distributed protocol. However, in small systems they may be part
//! of the same executable as the data containing the Id.
use super::object_id::ObjectId;

/// Canonical `_type` discriminator string for this class in serialized
/// form. See the `TODO(port)` on `hier_object_id::TYPE_NAME` for why this
/// is a `const` rather than a `#[serde(rename = ...)]` in this pass.
pub const TYPE_NAME: &str = "OBJECT_REF";

/// `OBJECT_REF` — a namespace, a type name, and the `OBJECT_ID` of the
/// referenced object.
///
/// `PARTY_REF` and `LOCATABLE_REF` both inherit `OBJECT_REF`; per ADR-001
/// §3 they embed `ObjectRef` by composition rather than a Rust trait-based
/// inheritance simulation. `LOCATABLE_REF` additionally redefines the `id`
/// field's type (`OBJECT_ID` narrowed to `UID_BASED_ID`) — see
/// `locatable_ref.rs` and ADR-001 §6.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectRef {
    /// `namespace`: namespace to which this identifier belongs in the
    /// local system context (and possibly in any other openEHR compliant
    /// environment) e.g. `terminology`, `demographic`. These names are not
    /// yet standardised.
    ///
    /// Legal values, per the class description: `"local"`, `"unknown"`, or
    /// a string matching the standard regex
    /// `[a-zA-Z][a-zA-Z0-9_.:\/&?=+-]*` (the first two values are just
    /// special cases already matched by the regex).
    ///
    /// TODO(port): the `namespace` legal-value constraint is not yet
    /// enforced by a constructor/`Validate` impl; awaits the RM invariant
    /// framework (`.claude/rules/rm-transcription.md` "Invariants").
    pub namespace: String,

    /// `type`: name of the class (concrete or abstract) of object to which
    /// this identifier type refers, e.g. `PARTY`, `PERSON`, `GUIDELINE`
    /// etc. These class names are from the relevant reference model. The
    /// type name `ANY` can be used to indicate that any type is accepted
    /// (e.g. if the type is unknown).
    ///
    /// PORT NOTE: named `r#type` because `type` is a Rust reserved keyword;
    /// the field is still serialized/documented under the spec's own name
    /// `type`.
    pub r#type: String,

    /// `id`: globally unique id of an object, regardless of where it is
    /// stored.
    pub id: ObjectId,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §OBJECT_REF — docs/research/spec-cache/BASE-1.2.0/uml_classes/object_ref.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / object_ref.adoc §OBJECT_REF Class
//   confidence: high
//   todos: 1
//   note: namespace legal-value regex constraint recorded in doc comment, not yet enforced; type field named r#type since `type` is a Rust keyword.
// ─────────────────────────────────────────────
